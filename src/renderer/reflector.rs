//! `ReflectorBaseNode.updateBefore()` (three.js/src/nodes/utils/ReflectorNode.js),
//! which is a nested `renderer.render()` and so lives with the renderer.
//!
//! Three calls it from `_renderObjectDirect()`, immediately before the object
//! whose material carries the reflector is drawn, once per `render()`
//! (`NodeUpdateType.RENDER`). The nested render's passes are submitted before
//! the outer pass is, so the outer pass samples a finished reflection. The
//! port records a pass after it has built every item, so it runs the updates
//! in a walk over the items in draw order, *before* the pass: the same renders
//! in the same order, each submitted first. What the order changes is which
//! `textureNode.value` a draw binds — three binds whatever the value is when
//! it records the draw, and a later reflector's nested render can move it — so
//! each item keeps the values as they stood right after its own update
//! (`Renderable::texture_overrides`). See `docs/nodes.md` §55.

use std::collections::HashSet;

use crate::cameras::{PerspectiveCamera, RenderCamera};
use crate::core::Node;
use crate::math::{CoordinateSystem, Matrix4, Plane, Vector3, Vector4};
use crate::nodes::reflector_node::{self, Reflector};
use crate::nodes::{BindingDesc, TextureSource};
use crate::objects::Scene;
use crate::textures::TextureType;

use super::{RenderTarget, RenderTargetOptions, Renderable, Renderer};

impl Renderer {
    /// The `updateBefore()` half of `_renderObjectDirect()` for every item that
    /// samples a reflector, in list order.
    pub(super) fn update_reflectors(
        &mut self,
        scene: &Scene,
        camera: &dyn RenderCamera,
        items: &mut [Renderable],
    ) {
        // The program the pass will look up, so the lookup here is the same
        // cache entry: its output width is the scene pass's colour format's.
        let output_components = self.scene_pass_color_format().components();
        // ... and its sample count, `builder.renderer.currentSamples`.
        let sample_count = self.current_samples().max(1);

        // `nodeUpdateBeforeMap.renderId !== this.renderId`: once per render.
        let mut updated: HashSet<usize> = HashSet::new();

        for item in items.iter_mut() {
            let Some(object) = item.object.clone() else {
                continue;
            };

            let program = self.node_builder_state(item, output_components, sample_count);
            let mut reflectors: Vec<Reflector> = Vec::new();
            for desc in program.groups.iter().flatten() {
                if let BindingDesc::Texture {
                    source: TextureSource::Texture2D(texture),
                    ..
                } = desc
                {
                    if let Some(reflector) = reflector_node::lookup(texture.id()) {
                        if !reflectors.iter().any(|r| r.key() == reflector.key()) {
                            reflectors.push(reflector);
                        }
                    }
                }
            }
            if reflectors.is_empty() {
                continue;
            }

            for reflector in &reflectors {
                // A target added from the material's own `Fn()`, which three
                // runs as the material is built (`add_target_on_setup`).
                let add_target_to = reflector.borrow_mut().add_target_to.take();
                if let Some(parent) = add_target_to.and_then(|parent| parent.upgrade()) {
                    let target = reflector.borrow().target.clone();
                    parent.add(&target);
                }
                // `ReflectorBaseNode.setup()`: `this._updateResolution(
                // _defaultRT, builder.renderer )`. The default target is what
                // the node samples until the reflector has rendered, so it
                // needs its allocation.
                let default_render_target = reflector.borrow().default_render_target.clone();
                self.update_resolution(reflector, &default_render_target);
                self.prepare_render_target(&default_render_target);

                if updated.insert(reflector.key()) {
                    self.reflector_update_before(reflector, scene, camera, &object);
                }
            }

            item.texture_overrides = reflectors
                .iter()
                .map(|reflector| (reflector.default_texture_id(), reflector.value()))
                .collect();
        }
    }

    /// The colour format the next scene pass draws into: the bound target's,
    /// the internal framebuffer target's, or the canvas'.
    fn scene_pass_color_format(&mut self) -> wgpu::TextureFormat {
        match &self.render_target {
            Some(render_target) => render_target.texture().format(),
            None if self.needs_frame_buffer_target() => {
                self.frame_buffer_target().texture().format()
            }
            None => super::CANVAS_FORMAT,
        }
    }

    /// `ReflectorBaseNode._updateResolution( renderTarget, renderer )`.
    fn update_resolution(&self, reflector: &Reflector, render_target: &RenderTarget) {
        let resolution = reflector.borrow().resolution_scale;
        let (width, height) = self.drawing_buffer_size();
        render_target.set_size(
            (width as f64 * resolution).round() as u32,
            (height as f64 * resolution).round() as u32,
        );
    }

    /// `ReflectorBaseNode.updateBefore( frame )`.
    ///
    /// `_inReflector` is set on entry and cleared on the way out, whichever
    /// way [`render_reflection`](Self::render_reflection) leaves.
    fn reflector_update_before(
        &mut self,
        reflector: &Reflector,
        scene: &Scene,
        camera: &dyn RenderCamera,
        object: &Node,
    ) {
        if !reflector.borrow().bounces && self.in_reflector {
            return;
        }

        self.in_reflector = true;
        self.render_reflection(reflector, scene, camera, object);
        self.in_reflector = false;
    }

    /// The body of `ReflectorBaseNode.updateBefore()` between setting and
    /// clearing `_inReflector`.
    fn render_reflection(
        &mut self,
        reflector: &Reflector,
        scene: &Scene,
        camera: &dyn RenderCamera,
        object: &Node,
    ) {
        let target = reflector.borrow().target.clone();

        // `getVirtualCamera( camera )` / `getRenderTarget( virtualCamera )`.
        // The virtual camera is taken out of the map for the length of the
        // render, so a nested update of this same reflector cannot alias it.
        let camera_id = camera_id(camera);
        let mut virtual_camera = reflector
            .borrow_mut()
            .virtual_cameras
            .remove(&camera_id)
            .unwrap_or_else(|| clone_camera(camera));
        let virtual_camera_id = virtual_camera.node.borrow().id;
        let render_target = {
            let mut base = reflector.borrow_mut();
            let samples = base.samples;
            base.render_targets
                .entry(virtual_camera_id)
                .or_insert_with(|| {
                    RenderTarget::new_with_options(
                        1,
                        1,
                        RenderTargetOptions {
                            texture_type: TextureType::HalfFloat,
                            samples,
                            ..RenderTargetOptions::default()
                        },
                    )
                    .expect("three-rs: HalfFloatType is a colour type")
                })
                .clone()
        };

        self.update_resolution(reflector, &render_target);

        //

        let target_world = target.borrow().matrix_world;
        let camera_world = camera.matrix_world();

        let mut reflector_world_position = Vector3::ZERO;
        reflector_world_position.set_from_matrix_position(&target_world);
        let mut camera_world_position = Vector3::ZERO;
        camera_world_position.set_from_matrix_position(&camera_world);

        let mut rotation_matrix = Matrix4::identity();
        rotation_matrix.extract_rotation(&target_world);

        let mut normal = Vector3::new(0.0, 0.0, 1.0);
        normal.apply_matrix4(&rotation_matrix);

        let mut view = reflector_world_position;
        view.sub(&camera_world_position);

        // Avoid rendering when reflector is facing away unless forcing an update
        let is_facing_away = view.dot(&normal) > 0.0;

        let mut needs_clear = false;

        if is_facing_away && !reflector.borrow().force_update {
            if !reflector.borrow().has_output {
                reflector
                    .borrow_mut()
                    .virtual_cameras
                    .insert(camera_id, virtual_camera);
                return;
            }

            needs_clear = true;
        }

        view.reflect(&normal).negate();
        view.add(&reflector_world_position);

        rotation_matrix.extract_rotation(&camera_world);

        let mut look_at_position = Vector3::new(0.0, 0.0, -1.0);
        look_at_position.apply_matrix4(&rotation_matrix);
        look_at_position.add(&camera_world_position);

        let mut look_target = reflector_world_position;
        look_target.sub(&look_at_position);
        look_target.reflect(&normal).negate();
        look_target.add(&reflector_world_position);

        //

        virtual_camera.coordinate_system = camera.coordinate_system();
        {
            let mut node = virtual_camera.node.borrow_mut();
            node.position = view;
            let mut up = Vector3::new(0.0, 1.0, 0.0);
            up.apply_matrix4(&rotation_matrix);
            up.reflect(&normal);
            node.up = up;
        }
        virtual_camera.look_at(&look_target);

        virtual_camera.near = camera.near();
        virtual_camera.far = camera.far();

        virtual_camera.update_matrix_world();
        virtual_camera.projection_matrix = camera.projection_matrix();

        // Now update projection matrix with new clip plane, implementing code from: http://www.terathon.com/code/oblique.html
        // Paper explaining this technique: http://www.terathon.com/lengyel/Lengyel-Oblique.pdf
        let mut reflector_plane = Plane::default();
        reflector_plane.set_from_normal_and_coplanar_point(&normal, &reflector_world_position);
        reflector_plane.apply_matrix4(&virtual_camera.matrix_world_inverse, None);

        let mut clip_plane = Vector4::new(
            reflector_plane.normal.x,
            reflector_plane.normal.y,
            reflector_plane.normal.z,
            reflector_plane.constant,
        );

        let projection_matrix = &mut virtual_camera.projection_matrix;
        let e = projection_matrix.elements;

        let q = Vector4::new(
            (js_sign(clip_plane.x) + e[8]) / e[0],
            (js_sign(clip_plane.y) + e[9]) / e[5],
            -1.0,
            (1.0 + e[10]) / e[14],
        );

        // Calculate the scaled plane vector
        clip_plane.multiply_scalar(1.0 / clip_plane.dot(&q));

        let clip_bias = 0.0;

        // Replacing the third row of the projection matrix. The renderer's
        // coordinate system is the backend's, always WebGPU here.
        projection_matrix.elements[2] = clip_plane.x;
        projection_matrix.elements[6] = clip_plane.y;
        projection_matrix.elements[10] = clip_plane.z - clip_bias;
        projection_matrix.elements[14] = clip_plane.w;
        debug_assert_eq!(camera.coordinate_system(), CoordinateSystem::WebGpu);

        //

        reflector.borrow_mut().value = render_target.texture();

        set_material_visible(object, false);

        // `currentRenderTarget`, `currentMRT` and `currentAutoClear`, put back
        // when the scope ends.
        {
            let mut renderer = self.save_state();

            renderer.set_mrt(None);
            renderer.set_render_target(Some(render_target));
            renderer.auto_clear = true;

            if needs_clear {
                renderer.clear(true, true);

                reflector.borrow_mut().has_output = false;
            } else {
                renderer.render_nested(scene, &mut virtual_camera);

                reflector.borrow_mut().has_output = true;
            }
        }

        set_material_visible(object, true);

        let mut base = reflector.borrow_mut();
        base.force_update = false;
        base.virtual_cameras.insert(camera_id, virtual_camera);
    }
}

/// `Math.sign`, which is 0 at 0 where `f64::signum` is 1.
fn js_sign(x: f64) -> f64 {
    if x > 0.0 {
        1.0
    } else if x < 0.0 {
        -1.0
    } else {
        x
    }
}

/// The key `virtualCameras` is a `WeakMap` on: the camera's object id.
fn camera_id(camera: &dyn RenderCamera) -> u32 {
    camera.id()
}

/// `camera.clone()`, as far as the virtual camera reads it: everything that
/// `updateBefore()` does not overwrite on every call is the layers and the
/// projection matrix's inverse.
fn clone_camera(camera: &dyn RenderCamera) -> PerspectiveCamera {
    let mut virtual_camera = PerspectiveCamera::new(50.0, 1.0, camera.near(), camera.far());
    virtual_camera.node.borrow_mut().layers = camera.layers();
    virtual_camera.projection_matrix = camera.projection_matrix();
    virtual_camera.projection_matrix_inverse = camera.projection_matrix_inverse();
    virtual_camera
}

/// `material.visible = …` on the object whose material carries the
/// reflector — `frame.material`.
fn set_material_visible(object: &Node, visible: bool) {
    if let Some(material) = object.borrow_mut().payload.material_mut() {
        material.visible = visible;
    }
}
