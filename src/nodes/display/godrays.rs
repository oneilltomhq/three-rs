//! Port of `three.js/examples/jsm/tsl/display/GodraysNode.js` — volumetric
//! light shafts ray-marched through a point light's shadow map.
//!
//! `godrays( depthNode, camera, light )` owns one render target at half the
//! drawing buffer. Per pixel, its quad reconstructs the world position from the
//! scene depth, clips the camera ray against the six planes of the shadow
//! camera's reach (the `far`-sized box around the light), and marches between
//! the two ends, sampling the cube shadow map at every step: a lit step adds
//! `density`-scaled in-scattering, fading with distance from the light. The
//! sum goes through `1 - exp( -illum )`, clamped to `maxDensity`, into the
//! red, green and blue channels; alpha carries the scene depth.
//!
//! Faithful quirks — the WGSL is three's, so they are kept:
//!
//! * `worldPosition = cameraMatrixWorld.mul( viewPosition )` is a `vec4`, and
//!   every `vec3` it meets is widened with a `1.0` w: `sdPlane( worldPosition,
//!   n, h )` is `dot( p, vec4( n, 1 ) ) + h`, i.e. the plane test is off by
//!   one unit, and the start/end distances are 4-D distances whose w terms
//!   cancel.
//! * The march count is `round( steps + ( steps / 8 + 2 ) · noise )`, with
//!   `raymarchSteps` a `uint` uniform that three's WGSL declares `f32`.
//!
//! Divergences that do not reach the WGSL:
//!
//! * The input is the pass's depth texture, not a node; the camera is the
//!   pass's. [`GodraysNode`] runs the pass's `updateBefore()` first, as
//!   [`TraaNode`](super::TraaNode) does, so the depth it marches is this
//!   frame's.
//! * The shadow map is [`LightShadow::point_depth_texture`](crate::lights::LightShadow::point_depth_texture),
//!   which the renderer draws the light's shadow into. Three reads
//!   `light.shadow.map.depthTexture` during `setup()`; `shadow.map` was
//!   assigned by `ShadowNode.setupShadow()` when the first material the light
//!   shines on was built, not by a shadow render.
//! * `setSize()` clamps each side to at least one texel. Three's
//!   `Math.round( resolutionScale * size )` has no clamp, so a scale that
//!   rounds to 0 asks for an empty target; `BilateralBlurNode` does clamp.
//!
//! Not ported: the `DirectionalLight` branch (a frustum from the shadow
//! camera's matrices and a 2-D shadow-map compare) — the constructor panics
//! on anything but a point light, where three throws for anything but a point
//! or directional one — a logarithmic depth buffer, `dispose()`, and the
//! shared `builder.getSharedContext()` the material is given, which the
//! port's per-material builds have no use for.

use std::cell::RefCell;
use std::rc::Rc;

use crate::cameras::{PerspectiveCamera, RenderCamera};
use crate::lights::LightKind;
use crate::materials::MeshBasicNodeMaterial;
use crate::math::{Color, Vector3};
use crate::nodes::frame::register_texture_update;
use crate::nodes::node::{LiveValue, SettableValue, Type, UniformGroup, UniformSource};
use crate::nodes::tsl::{
    block, boolean, cube_depth_texture_compare, distance, dot, exp, float, frag_coord,
    get_view_position, if_else, if_then, int, interleaved_gradient_noise, loop_n, max, mix,
    pass_depth_texture_uv, round, texture_uv, to_const, to_var, uniform, uniform_array_live,
    uniform_settable, uv, vec2_join, vec4, vec4_join, view_z_to_perspective_depth, UniformArray,
};
use crate::nodes::{NodeRef, NodeUpdate, NodeUpdateType};
use crate::objects::QuadMesh;
use crate::renderer::{RenderTarget, RenderTargetOptions, Renderer};
use crate::textures::{CubeDepthTexture, DepthTexture, Texture, TextureFilter, TextureType};

/// `_DIRECTIONS` — the six face normals of the shadow box, in three's order.
const DIRECTIONS: [[f64; 3]; 6] = [
    [1.0, 0.0, 0.0],
    [-1.0, 0.0, 0.0],
    [0.0, 1.0, 0.0],
    [0.0, -1.0, 0.0],
    [0.0, 0.0, 1.0],
    [0.0, 0.0, -1.0],
];

/// `godrays( depthNode, camera, light )` — see the module docs.
///
/// `depth` is the scene pass's depth attachment, `camera` the pass's camera
/// and `light` a shadow-casting `PointLight`.
pub fn godrays(
    depth: &DepthTexture,
    camera: Rc<RefCell<PerspectiveCamera>>,
    light: &crate::core::Node,
) -> GodraysNode {
    GodraysNode::new(depth, camera, light)
}

/// `GodraysNode` — a handle; the state is shared with the renderer's
/// update-before registry.
pub struct GodraysNode(Rc<GodraysState>);

/// What a [`GodraysNode`] shares with the renderer.
pub(crate) struct GodraysState {
    /// `this.depthNode.value`, whose pass renders the scene.
    depth: DepthTexture,
    /// `this._camera`.
    camera: Rc<RefCell<PerspectiveCamera>>,
    /// `this._light`.
    light: crate::core::Node,
    /// `this._godraysRenderTarget`.
    target: RenderTarget,
    /// `_quadMesh` with `this._material`.
    quad: QuadMesh,
    /// `this.resolutionScale`.
    resolution_scale: f64,
    /// `this._cameraMatrixWorld`.
    camera_matrix_world: SettableValue,
    /// `this._cameraProjectionMatrixInverse`.
    camera_projection_matrix_inverse: SettableValue,
    /// `this._cameraPosition`.
    camera_position: SettableValue,
    /// `this._fNormals.array`, padded to four floats an element.
    f_normals: Rc<RefCell<Vec<f64>>>,
    /// `this._fConstants.array`, padded likewise.
    f_constants: Rc<RefCell<Vec<f64>>>,
    /// `this._textureNode` — `passTexture( this, target.texture )`.
    node: NodeRef,
    /// `this.raymarchSteps`.
    raymarch_steps: SettableValue,
    /// `this.density`.
    density: SettableValue,
    /// `this.maxDensity`.
    max_density: SettableValue,
    /// `this.distanceAttenuation`.
    distance_attenuation: SettableValue,
}

/// The nodes the `godrays` `Fn` reads.
struct Inputs<'a> {
    depth: &'a DepthTexture,
    shadow_map: &'a CubeDepthTexture,
    camera_matrix_world: NodeRef,
    camera_projection_matrix_inverse: NodeRef,
    camera_position: NodeRef,
    shadow_camera_near: NodeRef,
    shadow_camera_far: NodeRef,
    light_position: NodeRef,
    f_normals: UniformArray,
    f_constants: UniformArray,
    raymarch_steps: NodeRef,
    density: NodeRef,
    max_density: NodeRef,
    distance_attenuation: NodeRef,
}

/// `reference( name, 'float', light.shadow.camera )`, read at draw time.
fn shadow_camera_reference(light: &crate::core::Node, far: bool) -> NodeRef {
    let light = light.downgrade();
    uniform(
        UniformSource::Live(LiveValue::new(move || {
            let light = light
                .upgrade()
                .expect("three-rs: a GodraysNode outlived its light");
            let object = light.borrow();
            let shadow = object
                .light()
                .and_then(|light| light.shadow.as_ref())
                .expect("three-rs: godrays() wants a shadow-casting light");
            vec![if far {
                shadow.camera.far()
            } else {
                shadow.camera.near()
            }]
        })),
        Type::F32,
        UniformGroup::Object,
        None,
    )
}

/// `lightPosition( light )` — the light's world position, a render-group
/// uniform updated per render.
fn light_position(light: &crate::core::Node) -> NodeRef {
    let light = light.downgrade();
    uniform(
        UniformSource::Live(LiveValue::new(move || {
            let light = light
                .upgrade()
                .expect("three-rs: a GodraysNode outlived its light");
            let e = light.borrow().matrix_world.elements;
            vec![e[12], e[13], e[14]]
        })),
        Type::Vec3,
        UniformGroup::Render,
        None,
    )
}

impl GodraysNode {
    /// `new GodraysNode( depthNode, camera, light )`, with the material
    /// `setup()` gives it.
    pub fn new(
        depth: &DepthTexture,
        camera: Rc<RefCell<PerspectiveCamera>>,
        light: &crate::core::Node,
    ) -> Self {
        let shadow_map = {
            let mut object = light.borrow_mut();
            let light = object
                .light_mut()
                .expect("three-rs: godrays() wants a light");
            assert!(
                light.kind == LightKind::Point,
                "three-rs: GodraysNode supports point lights only (the directional branch is not ported)"
            );
            light
                .shadow
                .as_mut()
                .expect("three-rs: godrays() wants a shadow-casting light")
                .point_depth_texture()
        };

        // `new RenderTarget( 1, 1, { depthBuffer: false } )`.
        let target = RenderTarget::new_with_options(
            1,
            1,
            RenderTargetOptions {
                texture_type: TextureType::UnsignedByte,
                samples: 0,
                depth_buffer: false,
                min_filter: TextureFilter::Linear,
                mag_filter: TextureFilter::Linear,
            },
        )
        .expect("three-rs: the godrays target is a colour type");

        let identity = crate::math::Matrix4::identity().elements.to_vec();
        let (camera_matrix_world, camera_matrix_world_value) =
            uniform_settable(Type::Mat4, identity.clone());
        let (camera_projection_matrix_inverse, camera_projection_matrix_inverse_value) =
            uniform_settable(Type::Mat4, identity);
        let (camera_position, camera_position_value) =
            uniform_settable(Type::Vec3, vec![0.0, 0.0, 0.0]);
        // `uniform( uint( 60 ) )`, declared `f32` by three's WGSL.
        let (raymarch_steps, raymarch_steps_value) = uniform_settable(Type::F32, vec![60.0]);
        let (density, density_value) = uniform_settable(Type::F32, vec![0.7]);
        let (max_density, max_density_value) = uniform_settable(Type::F32, vec![0.5]);
        let (distance_attenuation, distance_attenuation_value) =
            uniform_settable(Type::F32, vec![2.0]);

        let f_normals_values = Rc::new(RefCell::new(vec![0.0; 24]));
        let f_constants_values = Rc::new(RefCell::new(vec![0.0; 24]));
        let f_normals = {
            let values = f_normals_values.clone();
            uniform_array_live(6, move || values.borrow().clone())
        };
        let f_constants = {
            let values = f_constants_values.clone();
            uniform_array_live(6, move || values.borrow().clone())
        };

        let fragment = godrays_fragment(&Inputs {
            depth,
            shadow_map: &shadow_map,
            camera_matrix_world,
            camera_projection_matrix_inverse,
            camera_position,
            shadow_camera_near: shadow_camera_reference(light, false),
            shadow_camera_far: shadow_camera_reference(light, true),
            light_position: light_position(light),
            f_normals,
            f_constants,
            raymarch_steps,
            density,
            max_density,
            distance_attenuation,
        });

        let mut material = MeshBasicNodeMaterial::new();
        material.name = "Godrays";
        material.fragment_node = Some(fragment);

        let node = to_var(None, texture_uv(&target.texture(), uv()));
        let state = Rc::new(GodraysState {
            depth: depth.clone(),
            camera,
            light: light.clone(),
            target,
            quad: QuadMesh::new(material),
            resolution_scale: 0.5,
            camera_matrix_world: camera_matrix_world_value,
            camera_projection_matrix_inverse: camera_projection_matrix_inverse_value,
            camera_position: camera_position_value,
            f_normals: f_normals_values,
            f_constants: f_constants_values,
            node,
            raymarch_steps: raymarch_steps_value,
            density: density_value,
            max_density: max_density_value,
            distance_attenuation: distance_attenuation_value,
        });
        register_texture_update(state.target.texture().id(), &state);
        Self(state)
    }

    /// `godraysNode.getTextureNode()` — the rays, for the graph downstream.
    pub fn node(&self) -> NodeRef {
        self.0.node.clone()
    }

    /// `this._godraysRenderTarget.texture`.
    pub fn texture(&self) -> Texture {
        self.0.target.texture()
    }

    /// `godraysNode.raymarchSteps` — the base step count (60).
    pub fn raymarch_steps(&self) -> &SettableValue {
        &self.0.raymarch_steps
    }

    /// `godraysNode.density` (0.7).
    pub fn density(&self) -> &SettableValue {
        &self.0.density
    }

    /// `godraysNode.maxDensity` (0.5).
    pub fn max_density(&self) -> &SettableValue {
        &self.0.max_density
    }

    /// `godraysNode.distanceAttenuation` (2).
    pub fn distance_attenuation(&self) -> &SettableValue {
        &self.0.distance_attenuation
    }

    /// The quad material, for `examples/dump_wgsl.rs`.
    #[doc(hidden)]
    pub fn quad_material(&self) -> &MeshBasicNodeMaterial {
        &self.0.quad.material
    }

    /// `GodraysNode.setSize( width, height )`.
    pub fn set_size(&self, width: u32, height: u32) {
        self.0.set_size(width, height);
    }
}

impl GodraysState {
    /// `Math.round( resolutionScale * size )`, clamped to one texel — three
    /// does not clamp (see the module's divergences).
    fn set_size(&self, width: u32, height: u32) {
        let scale = |size: u32| ((self.resolution_scale * size as f64).round() as u32).max(1);
        self.target.set_size(scale(width), scale(height));
    }

    /// `GodraysNode._updateLightParams()`, point-light branch: plane `i` has
    /// normal `_DIRECTIONS[ i ]` and passes through `light.position +
    /// direction · shadow.camera.far` — the light's *local* position, as in
    /// three.
    fn update_light_params(&self) {
        let object = self.light.borrow();
        let position = object.position;
        let far = object
            .light()
            .and_then(|light| light.shadow.as_ref())
            .expect("three-rs: godrays() wants a shadow-casting light")
            .camera
            .far();
        let mut normals = self.f_normals.borrow_mut();
        let mut constants = self.f_constants.borrow_mut();
        for (i, d) in DIRECTIONS.iter().enumerate() {
            let normal = Vector3::new(d[0], d[1], d[2]);
            let mut point = position;
            point.add_scaled_vector(&normal, far);
            // `Plane.setFromNormalAndCoplanarPoint()`: `constant = - point ·
            // normal`.
            let constant = -point.dot(&normal);
            normals[i * 4..i * 4 + 4].copy_from_slice(&[d[0], d[1], d[2], 0.0]);
            constants[i * 4..i * 4 + 4].copy_from_slice(&[constant, 0.0, 0.0, 0.0]);
        }
    }
}

impl NodeUpdate for GodraysState {
    /// `this.updateBeforeType = NodeUpdateType.FRAME`.
    fn update_before_type(&self) -> NodeUpdateType {
        NodeUpdateType::Frame
    }

    /// `GodraysNode.updateBefore( frame )`.
    fn update_before(&self, renderer: &mut Renderer) -> bool {
        // The scene pass first — see the module docs.
        if let Some(pass) = crate::nodes::frame::texture_update(self.depth.id()) {
            renderer.update_before_node(&pass);
        }

        // `RendererUtils.resetRendererState( renderer, _rendererState )`.
        let previous_target = renderer.render_target();
        let previous_mrt = renderer.mrt();
        let previous_auto_clear = renderer.auto_clear;
        let previous_clear_color = renderer.clear_color();
        let previous_clear_alpha = renderer.clear_alpha();
        renderer.set_mrt(None);
        renderer.set_clear_color(Color::new(0.0, 0.0, 0.0), 1.0);
        renderer.auto_clear = true;

        let (width, height) = renderer.drawing_buffer_size();
        self.set_size(width, height);

        self.update_light_params();
        {
            // `uniform( camera.matrixWorld )` and `uniform(
            // camera.projectionMatrixInverse )` hold the camera's own
            // matrices; the port copies them here, before the one draw that
            // reads them.
            let camera = self.camera.borrow();
            let world = RenderCamera::matrix_world(&*camera);
            self.camera_matrix_world.set(world.elements.to_vec());
            self.camera_projection_matrix_inverse
                .set(camera.projection_matrix_inverse.elements.to_vec());
            // `_cameraPosition.value.setFromMatrixPosition( camera.matrixWorld
            // )`.
            let e = world.elements;
            self.camera_position.set(vec![e[12], e[13], e[14]]);
        }

        // `renderer.setClearColor( 0xffffff, 1 )`.
        renderer.set_clear_color(Color::new(1.0, 1.0, 1.0), 1.0);

        renderer.set_render_target(Some(self.target.clone()));
        renderer.render_quad(&self.quad);

        // `RendererUtils.restoreRendererState( renderer, _rendererState )`.
        renderer.set_render_target(previous_target);
        renderer.set_mrt(previous_mrt);
        renderer.set_clear_color(previous_clear_color, previous_clear_alpha);
        renderer.auto_clear = previous_auto_clear;
        true
    }
}

/// The `godrays` `Fn` of `GodraysNode.setup()`, point-light branch.
fn godrays_fragment(inputs: &Inputs<'_>) -> NodeRef {
    let uv_node = uv();
    let camera_position = inputs.camera_position.clone();
    let f_normals = &inputs.f_normals;
    let f_constants = &inputs.f_constants;

    // `sdPlane( p, n, h )`: `dot( p, n ) + h`.
    let sd_plane = |p: NodeRef, i: &NodeRef| {
        dot(p, f_normals.element_xyz(i.clone())).add(f_constants.element_x(i.clone()))
    };
    // `intersectRayPlane( rayOrigin, rayDirection, n, h )`.
    let intersect_ray_plane = |origin: NodeRef, direction: NodeRef, i: &NodeRef| {
        let denom = dot(f_normals.element_xyz(i.clone()), direction);
        sd_plane(origin, i).div(denom).negate()
    };

    let output = to_var(None, vec4(0.0, 0.0, 0.0, 1.0));
    let is_early_out = to_var(None, boolean(false));
    let depth = to_const(None, pass_depth_texture_uv(inputs.depth, uv_node.clone()));
    let view_position = to_const(
        None,
        get_view_position(
            uv_node,
            depth.clone(),
            inputs.camera_projection_matrix_inverse.clone(),
        ),
    );
    // A `vec4` — see the module docs.
    let world_position = to_var(
        None,
        inputs
            .camera_matrix_world
            .clone()
            .mul(view_position.clone()),
    );

    let in_box_dist = to_var(None, float(-10000.0));
    let in_box = loop_n("i", int(6), |i| {
        vec![in_box_dist.assign(max(
            in_box_dist.clone(),
            sd_plane(camera_position.clone(), i),
        ))]
    });
    let start_position = to_var(None, camera_position.clone());

    // The ray target is outside the shadow box: move it to the nearest point
    // on the box, to avoid marching through unlit space.
    let clip_end = loop_n("i", int(6), |i| {
        let direction = to_const(None, world_position.sub(camera_position.clone()));
        let t = intersect_ray_plane(camera_position.clone(), direction.clone(), i);
        vec![if_then(
            sd_plane(world_position.clone(), i).greater_than(float(0.0)),
            vec![
                direction.clone(),
                world_position.assign(camera_position.clone().add(t.mul(direction))),
            ],
        )]
    });

    // Otherwise find where the ray enters the shadow box (`startPosition`).
    let direction = to_const(None, world_position.sub(camera_position.clone()));
    let min_t = to_var(None, float(10000.0));
    let entry = loop_n("i", int(6), |i| {
        let t = intersect_ray_plane(camera_position.clone(), direction.clone(), i);
        vec![if_then(
            t.less_than(min_t.clone()).and(t.greater_than(float(0.0))),
            vec![min_t.assign(t)],
        )]
    });
    let end_in_box_dist = to_var(None, float(-10000.0));
    let end_in_box = loop_n("i", int(6), |i| {
        vec![end_in_box_dist.assign(max(
            end_in_box_dist.clone(),
            sd_plane(world_position.clone(), i),
        ))]
    });
    let min_t2 = to_var(None, float(10000.0));
    let exit = loop_n("i", int(6), |i| {
        let t = intersect_ray_plane(start_position.clone(), direction.clone(), i);
        vec![if_then(
            sd_plane(world_position.clone(), i).greater_than(float(0.0)),
            vec![if_then(
                t.less_than(min_t2.clone()).and(t.greater_than(float(0.0))),
                vec![min_t2.assign(t)],
            )],
        )]
    });
    let clip_exit = if_then(
        end_in_box_dist.greater_than_equal(float(0.0)),
        vec![
            min_t2.clone(),
            exit,
            if_then(
                min_t2.less_than(distance(world_position.clone(), start_position.clone())),
                vec![world_position.assign(
                    start_position
                        .clone()
                        .add(min_t2.clone().mul(direction.clone())),
                )],
            ),
        ],
    );
    let found_entry = if_else(
        min_t.equal(float(10000.0)),
        vec![is_early_out.assign(boolean(true))],
        vec![
            start_position.assign(
                camera_position
                    .clone()
                    .add(min_t.clone().add(float(0.001)).mul(direction.clone())),
            ),
            end_in_box_dist.clone(),
            end_in_box,
            clip_exit,
        ],
    );
    let clip = if_else(
        in_box_dist.less_than(float(0.0)),
        vec![clip_end],
        vec![direction.clone(), min_t.clone(), entry, found_entry],
    );

    // The march.
    let illum = to_var(None, float(0.0));
    let noise = to_const(None, interleaved_gradient_noise(frag_coord().xy()));
    let steps = inputs.raymarch_steps.clone();
    let samples_float = to_const(
        None,
        round(
            steps
                .clone()
                .add(steps.div(float(8.0)).add(float(2.0)).mul(noise.clone())),
        ),
    );
    let samples = to_const(None, samples_float.to(Type::U32));
    let march = loop_n("i", samples.clone(), |i| {
        let sample_pos = to_const(
            None,
            mix(
                start_position.clone(),
                world_position.clone(),
                i.to(Type::F32).div(samples_float.clone()),
            ),
        );
        // `inShadow( samplePos )`, point-light branch.
        let light_to_pos = to_const(None, sample_pos.sub(inputs.light_position.clone()));
        let shadow_position_abs = to_const(None, light_to_pos.abs());
        let view_z = shadow_position_abs
            .x()
            .max(shadow_position_abs.y())
            .max(shadow_position_abs.z())
            .negate();
        let shadow_depth = view_z_to_perspective_depth(
            view_z.clone(),
            inputs.shadow_camera_near.clone(),
            inputs.shadow_camera_far.clone(),
        );
        let result = cube_depth_texture_compare(inputs.shadow_map, light_to_pos, shadow_depth);
        let shadow_info = vec2_join(vec![result.one_minus().add(float(0.005)), view_z.negate()]);

        let shadow_amount = to_const(None, shadow_info.x().one_minus());
        vec![illum.add_assign(
            shadow_amount
                .mul(
                    distance(start_position.clone(), world_position.clone())
                        .mul(inputs.density.clone().div(float(100.0))),
                )
                .mul(
                    shadow_info
                        .y()
                        .div(inputs.shadow_camera_far.clone())
                        .one_minus()
                        .pow(inputs.distance_attenuation.clone()),
                ),
        )]
    });
    let shade = if_then(
        is_early_out.equal(boolean(false)),
        vec![
            illum.clone(),
            noise,
            samples_float.clone(),
            samples,
            march,
            illum.div_assign(samples_float),
            output.assign(vec4_join(vec![
                exp(illum.negate())
                    .one_minus()
                    .clamp(float(0.0), inputs.max_density.clone())
                    .to(Type::Vec3),
                depth.clone(),
            ])),
        ],
    );

    block(
        vec![
            output.clone(),
            is_early_out,
            depth,
            view_position,
            world_position,
            in_box_dist,
            in_box,
            start_position,
            clip,
            shade,
        ],
        output,
    )
}
