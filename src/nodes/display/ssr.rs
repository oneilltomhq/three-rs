//! Port of `three.js/examples/jsm/tsl/display/SSRNode.js` — screen space
//! reflections — in its first-generation form: one mirror ray per pixel,
//! marched through the depth buffer in screen space, and roughness faked by
//! a blurred mip chain.
//!
//! Three's node is a `Node` with `updateBeforeType = FRAME`, and so is this
//! one ([`SsrState`] implements [`NodeUpdate`] and is registered as the
//! updater of both its textures). Every frame `updateBefore()` draws three
//! quads' worth of passes:
//!
//! 1. `SSRNode.SSR` into the half-float SSR target: the reflection colour,
//!    premultiplied by metalness, attenuation and a Fresnel term, with the
//!    hit's world-space distance in alpha;
//! 2. `SSRNode.Copy` into mip 0 of the blur target, unblurred;
//! 3. `SSRNode.Blur` into mips 1–4, a [`box_blur`](super::box_blur) of the
//!    SSR target whose tap spacing is the mip index.
//!
//! [`SsrNode::node`] then reads the blur target at a level picked by the
//! surface's roughness (`r² · 4`), which is `ssrPass` on the page.
//!
//! **Order within a frame.** Three's `setup()` builds the colour, depth and
//! normal nodes before itself, so the scene pass is earlier in the frame's
//! update-before list. The port asks for the pass explicitly at the top of
//! `update_before()`, as [`TraaNode`](super::TraaNode) does; the frame guard
//! makes the pass's own later reach a no-op.
//!
//! Not ported, all of them options `webgpu_postprocessing_ssr` leaves at
//! their defaults: `stochastic` (the GGX-sampled second-generation path,
//! with its analytic noise, environment fallback and importance-sampled
//! environment), `reflectNonMetals`, `binaryRefine`, `screenEdgeFadeBlack`,
//! `setHistory()` (multi-bounce), `diffuseNode`, `resolutionScale` other
//! than 1, an orthographic camera and a logarithmic depth buffer.

use std::cell::RefCell;
use std::rc::Rc;

use crate::cameras::{PerspectiveCamera, RenderCamera};
use crate::materials::MeshBasicNodeMaterial;
use crate::math::{Color, Matrix4};
use crate::nodes::frame::register_texture_update;
use crate::nodes::node::{SettableValue, Type};
use crate::nodes::tsl::{
    block, boolean, break_loop, continue_loop, cross, discard, distance, dot, float,
    get_screen_position, get_view_position, if_then, int, length, loop_range, luminance, max,
    pass_depth_texture_uv, perspective_depth_to_view_z, reflect, texture, texture_level,
    texture_sample, texture_uv, to_const, to_var, uniform_settable, uv, vec2, vec2_join, vec4,
    vec4_join,
};
use crate::nodes::{NodeRef, NodeUpdate, NodeUpdateType};
use crate::objects::QuadMesh;
use crate::renderer::{RenderTarget, RenderTargetOptions, Renderer};
use crate::textures::{DepthTexture, MinFilter, Texture, TextureFilter, TextureType};

use super::box_blur::{box_blur_with, BoxBlurOptions};

/// `blurRenderTarget.texture.mipmaps.push( {}, {}, {}, {}, {} )`: the blur
/// chain's levels, the first an unblurred copy.
const BLUR_MIPS: u32 = 5;

/// A normal source: `normalNode.sample( uv )` for any uv, the way three's
/// `sample( ( uv ) => unpackRGBToNormal( … ) )` node answers both its own
/// default-uv read and the march's `normalNode.sample( uvS )`.
pub type SampleFn = Rc<dyn Fn(NodeRef) -> NodeRef>;

/// `SSRNodeOptions`, the part of it the port reads.
#[derive(Clone)]
#[non_exhaustive]
pub struct SsrOptions {
    /// `options.metalnessNode` — per-pixel metalness. A pixel at or below
    /// zero is discarded (`reflectNonMetals = false`).
    pub metalness: NodeRef,
    /// `options.roughnessNode` — per-pixel roughness, which picks the blur
    /// level. `None` is three's `null`: no blur passes, and
    /// [`SsrNode::node`] is the raw SSR target.
    pub roughness: Option<NodeRef>,
}

impl SsrOptions {
    /// `{ metalnessNode, roughnessNode }`.
    pub fn new(metalness: NodeRef, roughness: Option<NodeRef>) -> Self {
        Self {
            metalness,
            roughness,
        }
    }
}

/// `ssr( colorNode, depthNode, normalNode, options )`.
///
/// As with the other display nodes, `color` and `depth` are the textures
/// three's pass texture nodes wrap: the scene pass's `output` attachment and
/// its depth texture. `normal` is the view-space normal at a uv. The camera
/// must be the pass's (three infers it from `colorNode.passNode`).
pub fn ssr(
    color: &Texture,
    depth: &DepthTexture,
    normal: SampleFn,
    options: SsrOptions,
    camera: Rc<RefCell<PerspectiveCamera>>,
) -> SsrNode {
    SsrNode::new(color, depth, normal, options, camera)
}

/// `SSRNode` — see the module docs.
pub struct SsrNode(Rc<SsrState>);

/// The uniforms `SSRNode` exposes as properties.
struct Params {
    max_distance: SettableValue,
    thickness: SettableValue,
    intensity: SettableValue,
    max_luminance: SettableValue,
    quality: SettableValue,
}

/// The camera uniforms `updateBefore()` (and, in three, the uniforms'
/// object references) keep current.
struct CameraUniforms {
    projection: SettableValue,
    projection_inverse: SettableValue,
    world: SettableValue,
    near: SettableValue,
    far: SettableValue,
    resolution: SettableValue,
}

/// The node's state, shared with the renderer's update-before registry.
pub(crate) struct SsrState {
    /// `this.colorNode`'s texture, whose pass renders the scene.
    color: Texture,
    /// `this.camera`.
    camera: Rc<RefCell<PerspectiveCamera>>,
    /// `this._ssrRenderTarget`.
    ssr_target: RenderTarget,
    /// `this._blurRenderTarget`, with [`BLUR_MIPS`] levels.
    blur_target: RenderTarget,
    /// `_quadMesh` with `this._ssrMaterial`.
    ssr_quad: QuadMesh,
    /// `_quadMesh` with `this._copyMaterial`.
    copy_quad: QuadMesh,
    /// `_quadMesh` with `this._blurMaterial`. Rebuilt by
    /// [`SsrNode::set_blur_quality`], as three's `_buildBlurMaterial()` does.
    blur_quad: RefCell<QuadMesh>,
    /// `this._blurQuality`.
    blur_quality: std::cell::Cell<u32>,
    /// `this._blurSpread`.
    blur_spread: (NodeRef, SettableValue),
    /// `this.roughnessNode !== null` — whether the blur passes run.
    blurred: bool,
    params: Params,
    camera_uniforms: CameraUniforms,
    /// `this.getTextureNode()`.
    node: NodeRef,
}

fn mat4_values(m: &Matrix4) -> Vec<f64> {
    m.elements.to_vec()
}

impl SsrNode {
    /// `new SSRNode( colorNode, depthNode, normalNode, options )`, with the
    /// materials `setup()` gives it.
    pub fn new(
        color: &Texture,
        depth: &DepthTexture,
        normal: SampleFn,
        options: SsrOptions,
        camera: Rc<RefCell<PerspectiveCamera>>,
    ) -> Self {
        // `new RenderTarget( 1, 1, { depthBuffer: false, type: HalfFloatType } )`.
        let ssr_target = RenderTarget::new_with_options(
            1,
            1,
            RenderTargetOptions {
                texture_type: TextureType::HalfFloat,
                samples: 0,
                depth_buffer: false,
                min_filter: TextureFilter::Linear,
                mag_filter: TextureFilter::Linear,
            },
        )
        .expect("three-rs: the SSR target is a colour type");
        // `{ …, minFilter: LinearMipmapLinearFilter, magFilter: LinearFilter }`
        // and five `mipmaps` entries: the levels exist, and are filled by the
        // blur passes rather than generated.
        let blur_target = RenderTarget::new_with_options(
            1,
            1,
            RenderTargetOptions {
                texture_type: TextureType::HalfFloat,
                samples: 0,
                depth_buffer: false,
                min_filter: TextureFilter::Linear,
                mag_filter: TextureFilter::Linear,
            },
        )
        .expect("three-rs: the SSR blur target is a colour type");
        blur_target
            .texture()
            .set_min_filter(MinFilter::LinearMipmapLinear);
        blur_target.set_mip_level_count(BLUR_MIPS);

        let (max_distance, max_distance_value) = uniform_settable(Type::F32, vec![1.0]);
        let (thickness, thickness_value) = uniform_settable(Type::F32, vec![0.1]);
        let (intensity, intensity_value) = uniform_settable(Type::F32, vec![1.0]);
        let (max_luminance, max_luminance_value) = uniform_settable(Type::F32, vec![10.0]);
        let (quality, quality_value) = uniform_settable(Type::F32, vec![0.5]);

        let identity = mat4_values(&Matrix4::identity());
        let (projection, projection_value) = uniform_settable(Type::Mat4, identity.clone());
        let (projection_inverse, projection_inverse_value) =
            uniform_settable(Type::Mat4, identity.clone());
        let (world, world_value) = uniform_settable(Type::Mat4, identity);
        let (near, near_value) = uniform_settable(Type::F32, vec![camera.borrow().near]);
        let (far, far_value) = uniform_settable(Type::F32, vec![camera.borrow().far]);
        let (resolution, resolution_value) = uniform_settable(Type::Vec2, vec![0.0, 0.0]);

        let fragment = ssr_fragment(&SsrInputs {
            color,
            depth,
            normal: &normal,
            metalness: options.metalness.clone(),
            max_distance,
            thickness,
            intensity,
            max_luminance,
            quality,
            projection,
            projection_inverse,
            world,
            near,
            far,
            resolution,
        });
        let mut ssr_material = MeshBasicNodeMaterial::new();
        ssr_material.name = "SSRNode.SSR";
        ssr_material.fragment_node = Some(fragment);

        let ssr_texture = ssr_target.texture();

        // `this._copyMaterial.fragmentNode = texture( this._ssrRenderTarget.texture )`.
        let mut copy_material = MeshBasicNodeMaterial::new();
        copy_material.name = "SSRNode.Copy";
        copy_material.fragment_node = Some(texture(&ssr_texture));

        let blur_spread = uniform_settable(Type::F32, vec![1.0]);
        // `this._blurQuality = 2`.
        let blur_quality = 2;
        let blur_material = blur_material(&ssr_texture, blur_quality, blur_spread.0.clone());

        // `getTextureNode()`: the blur chain at a roughness-picked level, or
        // the SSR target when there is no roughness.
        let blur_texture = blur_target.texture();
        let node = match &options.roughness {
            Some(r) => {
                let mips = (BLUR_MIPS - 1) as f64;
                let lod = r.mul(r.clone()).mul(mips).clamp(0.0, mips);
                texture_level(&blur_texture, uv(), lod)
            }
            None => texture_uv(&ssr_texture, uv()),
        };

        let state = Rc::new(SsrState {
            color: color.clone(),
            camera,
            ssr_target,
            blur_target,
            ssr_quad: QuadMesh::new(ssr_material),
            copy_quad: QuadMesh::new(copy_material),
            blur_quad: RefCell::new(QuadMesh::new(blur_material)),
            blur_quality: std::cell::Cell::new(blur_quality),
            blur_spread,
            blurred: options.roughness.is_some(),
            params: Params {
                max_distance: max_distance_value,
                thickness: thickness_value,
                intensity: intensity_value,
                max_luminance: max_luminance_value,
                quality: quality_value,
            },
            camera_uniforms: CameraUniforms {
                projection: projection_value,
                projection_inverse: projection_inverse_value,
                world: world_value,
                near: near_value,
                far: far_value,
                resolution: resolution_value,
            },
            node,
        });
        // `passTexture( this, … )` for both textures: whichever one a
        // material samples, this node fills it first.
        register_texture_update(state.ssr_target.texture().id(), &state);
        register_texture_update(state.blur_target.texture().id(), &state);
        Self(state)
    }

    /// `ssrNode.getTextureNode()` — what the page calls `ssrPass`.
    pub fn node(&self) -> NodeRef {
        self.0.node.clone()
    }

    /// `ssrNode.maxDistance.value` — how far a reflection may reach, in
    /// world units. 1 by default.
    pub fn max_distance(&self) -> &SettableValue {
        &self.0.params.max_distance
    }

    /// `ssrNode.thickness.value` — how thick the depth buffer's surfaces are
    /// taken to be, in world units. 0.1 by default.
    pub fn thickness(&self) -> &SettableValue {
        &self.0.params.thickness
    }

    /// `ssrNode.intensity.value`. 1 by default.
    pub fn intensity(&self) -> &SettableValue {
        &self.0.params.intensity
    }

    /// `ssrNode.maxLuminance.value` — the luminance a reflection is clamped
    /// to, against fireflies. 10 by default.
    pub fn max_luminance(&self) -> &SettableValue {
        &self.0.params.max_luminance
    }

    /// `ssrNode.quality.value`, in [0, 1] — the fraction of the ray's
    /// screen-space pixels that are tested. 0.5 by default.
    pub fn quality(&self) -> &SettableValue {
        &self.0.params.quality
    }

    /// `ssrNode.blurQuality` — the box blur's half-width in taps.
    pub fn blur_quality(&self) -> u32 {
        self.0.blur_quality.get()
    }

    /// `ssrNode.blurQuality = value`: the size is baked into the blur's node
    /// tree, so a change rebuilds the material (`_buildBlurMaterial()`), and
    /// the same value is a no-op.
    pub fn set_blur_quality(&self, value: u32) {
        if value == self.0.blur_quality.get() {
            return;
        }
        self.0.blur_quality.set(value);
        let material = blur_material(
            &self.0.ssr_target.texture(),
            value,
            self.0.blur_spread.0.clone(),
        );
        self.0.blur_quad.borrow_mut().material = material;
    }

    /// The `SSRNode.SSR` quad material, for `examples/dump_wgsl.rs` and the
    /// dump gate.
    #[doc(hidden)]
    pub fn quad_material(&self) -> &MeshBasicNodeMaterial {
        &self.0.ssr_quad.material
    }

    /// The `SSRNode.Copy` quad material.
    #[doc(hidden)]
    pub fn copy_material(&self) -> &MeshBasicNodeMaterial {
        &self.0.copy_quad.material
    }

    /// The `SSRNode.Blur` quad material, as of the current blur quality.
    #[doc(hidden)]
    pub fn blur_material(&self) -> MeshBasicNodeMaterial {
        self.0.blur_quad.borrow().material.clone()
    }
}

/// `_buildBlurMaterial()`: `boxBlur( texture( ssrTarget.texture ), { size:
/// blurQuality, separation: _blurSpread } )`. The blur reads the texture
/// through `texture()`, so each tap is a `.sample()` with its own uv matrix.
fn blur_material(ssr_texture: &Texture, size: u32, spread: NodeRef) -> MeshBasicNodeMaterial {
    let tap_texture = ssr_texture.clone();
    let fragment = box_blur_with(
        ssr_texture,
        BoxBlurOptions {
            size: float(size as f64),
            separation: spread,
            premultiplied_alpha: false,
        },
        &move |coord| texture_sample(&tap_texture, coord),
    );
    let mut material = MeshBasicNodeMaterial::new();
    material.name = "SSRNode.Blur";
    material.fragment_node = Some(fragment);
    material
}

impl SsrState {
    /// `SSRNode.setSize( width, height )` with `resolutionScale = 1`.
    fn set_size(&self, width: u32, height: u32) {
        self.camera_uniforms
            .resolution
            .set(vec![width as f64, height as f64]);
        self.ssr_target.set_size(width, height);
        self.blur_target.set_size(width, height);
        // A full chain at this size is `floor(log2(max(w, h))) + 1` levels;
        // asking WebGPU for more than that fails texture creation, so a window
        // under 16 pixels on its long side gets fewer than [`BLUR_MIPS`].
        let full_chain = width.max(height).max(1).ilog2() + 1;
        self.blur_target
            .set_mip_level_count(BLUR_MIPS.min(full_chain));
    }
}

impl NodeUpdate for SsrState {
    /// `this.updateBeforeType = NodeUpdateType.FRAME`.
    fn update_before_type(&self) -> NodeUpdateType {
        NodeUpdateType::Frame
    }

    /// `SSRNode.updateBefore( frame )`.
    fn update_before(&self, renderer: &mut Renderer) -> bool {
        // The scene pass first — see the module docs.
        if let Some(pass) = crate::nodes::frame::texture_update(self.color.id()) {
            renderer.update_before_node(&pass);
        }

        {
            // `_cameraWorldMatrix.value.copy( camera.matrixWorld )`, and the
            // values three's object-referencing uniforms read at render time.
            let camera = self.camera.borrow();
            let u = &self.camera_uniforms;
            u.world
                .set(mat4_values(&RenderCamera::matrix_world(&*camera)));
            u.projection.set(mat4_values(&camera.projection_matrix));
            u.projection_inverse
                .set(mat4_values(&camera.projection_matrix_inverse));
            u.near.set(vec![camera.near]);
            u.far.set(vec![camera.far]);
        }

        // `_rendererState = RendererUtils.resetRendererState( renderer, … )`.
        let previous_target = renderer.render_target();
        let previous_level = renderer.active_mipmap_level();
        let previous_mrt = renderer.mrt();
        let previous_auto_clear = renderer.auto_clear;
        let previous_clear_color = renderer.clear_color();
        let previous_clear_alpha = renderer.clear_alpha();
        renderer.auto_clear = true;

        let (width, height) = renderer.drawing_buffer_size();
        self.set_size(width, height);

        // clear
        renderer.set_mrt(None);
        renderer.set_clear_color(Color::new(0.0, 0.0, 0.0), 0.0);

        // ssr
        renderer.set_render_target(Some(self.ssr_target.clone()));
        renderer.render_quad(&self.ssr_quad);

        // blur: mip 0 is the unblurred copy, every level after it spreads
        // the taps one texel further.
        if self.blurred {
            for i in 0..self.blur_target.mip_level_count() {
                self.blur_spread.1.set(vec![i as f64]);
                renderer.set_render_target_level(Some(self.blur_target.clone()), i);
                if i == 0 {
                    renderer.render_quad(&self.copy_quad);
                } else {
                    renderer.render_quad(&self.blur_quad.borrow());
                }
            }
        }

        // `RendererUtils.restoreRendererState( renderer, _rendererState )`.
        renderer.set_render_target_level(previous_target, previous_level);
        renderer.set_mrt(previous_mrt);
        renderer.set_clear_color(previous_clear_color, previous_clear_alpha);
        renderer.auto_clear = previous_auto_clear;
        true
    }
}

/// What the `ssr` `Fn()` closes over.
struct SsrInputs<'a> {
    color: &'a Texture,
    depth: &'a DepthTexture,
    normal: &'a SampleFn,
    metalness: NodeRef,
    max_distance: NodeRef,
    thickness: NodeRef,
    intensity: NodeRef,
    max_luminance: NodeRef,
    quality: NodeRef,
    projection: NodeRef,
    projection_inverse: NodeRef,
    world: NodeRef,
    near: NodeRef,
    far: NodeRef,
    resolution: NodeRef,
}

/// The `pointPlaneDistance` `Fn()`, inlined: `planeNormal` is normalised,
/// so the denominator is 1.
fn point_plane_distance(point: &NodeRef, plane_point: &NodeRef, plane_normal: &NodeRef) -> NodeRef {
    let n = plane_normal;
    let d = to_var(
        None,
        n.x()
            .mul(plane_point.x())
            .add(n.y().mul(plane_point.y()))
            .add(n.z().mul(plane_point.z()))
            .negate(),
    );
    block(
        vec![d.clone()],
        n.x()
            .mul(point.x())
            .add(n.y().mul(point.y()))
            .add(n.z().mul(point.z()))
            .add(d),
    )
}

/// The `pointToLineDistance` `Fn()`, inlined.
fn point_to_line_distance(point: &NodeRef, a: &NodeRef, b: &NodeRef) -> NodeRef {
    length(cross(point.sub(a.clone()), point.sub(b.clone()))).div(length(b.sub(a.clone())))
}

/// `SSRNode.setup()`'s `ssr` `Fn()` with `stochastic = false`,
/// `reflectNonMetals = false`, `binaryRefine = false`, no history and a
/// perspective camera: the `SSRNode.SSR` material's `fragmentNode`.
fn ssr_fragment(inputs: &SsrInputs) -> NodeRef {
    let SsrInputs {
        color,
        depth,
        normal,
        ..
    } = *inputs;
    let near = &inputs.near;
    let far = &inputs.far;
    let resolution = &inputs.resolution;
    let max_distance = &inputs.max_distance;
    let projection_inverse = &inputs.projection_inverse;

    let sample_depth = |coord: NodeRef| pass_depth_texture_uv(depth, coord);
    let mut statements = Vec::new();

    let uv_pos = to_var(None, uv());
    let depth_value = to_var(None, sample_depth(uv_pos.clone()));
    statements.push(uv_pos.clone());
    statements.push(depth_value.clone());
    // Skip background pixels (cleared far-plane depth).
    statements.push(if_then(
        depth_value.greater_than_equal(1.0),
        vec![discard()],
    ));

    let view_position = to_var(
        None,
        get_view_position(uv_pos.clone(), depth_value, projection_inverse.clone()),
    );
    let world_position = to_var(
        None,
        inputs
            .world
            .mul(vec4_join(vec![view_position.clone(), float(1.0)]))
            .xyz(),
    );
    let view_normal = to_var(None, normal(uv()).rgb().normalize());
    let view_incident_dir = to_var(None, view_position.normalize());
    statements.extend([
        view_position.clone(),
        world_position.clone(),
        view_normal.clone(),
        view_incident_dir.clone(),
    ]);

    // `float( this.metalnessNode )`.
    let metalness = to_const(None, inputs.metalness.clone());
    statements.push(if_then(metalness.less_than_equal(0.0), vec![discard()]));

    let view_reflect_dir = to_var(
        None,
        reflect(view_incident_dir.clone(), view_normal.clone()).normalize(),
    );
    let max_reflect_ray_len = to_var(
        None,
        max_distance.div(dot(view_incident_dir.negate(), view_normal.clone())),
    );
    let d1_view_position = to_var(
        None,
        view_position.add(view_reflect_dir.mul(max_reflect_ray_len.clone())),
    );
    statements.extend([
        view_reflect_dir.clone(),
        max_reflect_ray_len,
        d1_view_position.clone(),
    ]);
    // Clip the ray's far end to the near plane.
    statements.push(if_then(
        d1_view_position.z().greater_than(near.negate()),
        vec![d1_view_position.assign(
            view_position.add(
                view_reflect_dir.mul(
                    near.negate()
                        .sub(view_position.z())
                        .div(view_reflect_dir.z()),
                ),
            ),
        )],
    ));

    let d0 = to_var(None, uv_pos.mul(resolution.clone()).xy());
    let d1 = to_var(
        None,
        get_screen_position(d1_view_position.clone(), inputs.projection.clone())
            .mul(resolution.clone()),
    );
    let x_len = to_var(None, d1.x().sub(d0.x()));
    let y_len = to_var(None, d1.y().sub(d0.y()));
    let total_step = to_const(
        None,
        max(
            max(x_len.abs(), y_len.abs())
                .mul(inputs.quality.clamp(0.0, 1.0))
                .trunc()
                .to(Type::I32),
            int(1),
        ),
    );
    let total_step_f = total_step.to(Type::F32);
    let x_span = to_var(None, x_len.div(total_step_f.clone()));
    let y_span = to_var(None, y_len.div(total_step_f.clone()));
    let step_vec = to_var(None, vec2_join(vec![x_span.clone(), y_span.clone()]));
    let inv_resolution = to_var(None, vec2(1.0, 1.0).div(resolution.clone()));
    let uv_pixel_step_x = to_var(None, vec2_join(vec![inv_resolution.x(), float(0.0)]));
    let output = to_var(None, vec4(0.0, 0.0, 0.0, 0.0));
    let hit = to_var(None, float(0.0));
    let recip_vpz = to_const(None, float(1.0).div(view_position.z()));
    let recip_d1vpz = to_const(None, float(1.0).div(d1_view_position.z()));
    let found_hit = to_var(None, boolean(false));
    let hit_uv_s = to_var(None, vec2(0.0, 0.0));
    let hit_d = to_var(None, float(0.0));
    statements.extend([
        d0.clone(),
        d1,
        x_len,
        y_len,
        x_span,
        y_span,
        step_vec.clone(),
        inv_resolution.clone(),
        uv_pixel_step_x.clone(),
        output.clone(),
        hit.clone(),
        recip_vpz.clone(),
        recip_d1vpz.clone(),
        found_hit.clone(),
        hit_uv_s.clone(),
        hit_d.clone(),
    ]);

    let march = loop_range("i", int(1), total_step.clone(), |i| {
        let s = to_var(None, i.to(Type::F32).div(total_step_f.clone()));
        let xy = to_var(None, d0.add(step_vec.mul(s.mul(total_step_f.clone()))));
        let off_screen = xy
            .x()
            .less_than(0.0)
            .or(xy.x().greater_than(resolution.x()))
            .or(xy.y().less_than(0.0))
            .or(xy.y().greater_than(resolution.y()));
        let uv_s = to_var(None, xy.mul(inv_resolution.clone()));
        let d = to_var(None, sample_depth(uv_s.clone()));
        let v_z = to_var(
            None,
            perspective_depth_to_view_z(d.clone(), near.clone(), far.clone()),
        );
        let view_reflect_ray_z = to_var(
            None,
            float(1.0).div(recip_vpz.add(s.mul(recip_d1vpz.sub(recip_vpz.clone())))),
        );

        let v_p = to_var(
            None,
            get_view_position(uv_s.clone(), d.clone(), projection_inverse.clone()),
        );
        let away = to_var(
            None,
            point_to_line_distance(&v_p, &view_position, &d1_view_position),
        );
        let uv_neighbor = to_var(None, uv_s.add(uv_pixel_step_x.clone()));
        let v_p_neighbor = to_var(
            None,
            get_view_position(uv_neighbor.clone(), d.clone(), projection_inverse.clone()),
        );
        let min_thickness = to_var(None, v_p_neighbor.x().sub(v_p.x()).mul(3.0));
        let tk = to_var(None, max(min_thickness.clone(), inputs.thickness.clone()));

        let v_n = to_var(None, normal(uv_s.clone()).rgb().normalize());
        let plane_distance = to_var(
            None,
            point_plane_distance(&v_p, &view_position, &view_normal),
        );

        vec![
            s,
            xy.clone(),
            if_then(off_screen, vec![break_loop()]),
            uv_s.clone(),
            d.clone(),
            v_z.clone(),
            view_reflect_ray_z.clone(),
            if_then(
                view_reflect_ray_z.less_than_equal(v_z),
                vec![
                    v_p,
                    away.clone(),
                    uv_neighbor,
                    v_p_neighbor,
                    min_thickness,
                    tk.clone(),
                    if_then(
                        away.less_than_equal(tk),
                        vec![
                            v_n.clone(),
                            if_then(
                                dot(view_reflect_dir.clone(), v_n).greater_than_equal(0.0),
                                vec![continue_loop()],
                            ),
                            plane_distance.clone(),
                            if_then(
                                plane_distance.greater_than(max_distance.clone()),
                                vec![break_loop()],
                            ),
                            found_hit.assign(boolean(true)),
                            hit_uv_s.assign(uv_s),
                            hit_d.assign(d),
                            break_loop(),
                        ],
                    ),
                ],
            ),
        ]
    });
    statements.push(march);

    // The hit's colour, weighted.
    let v_p = to_var(
        None,
        get_view_position(hit_uv_s.clone(), hit_d.clone(), projection_inverse.clone()),
    );
    let distance_point_plane = to_var(
        None,
        point_plane_distance(&v_p, &view_position, &view_normal),
    );
    let hit_world_position = to_var(
        None,
        inputs
            .world
            .mul(vec4_join(vec![v_p.clone(), float(1.0)]))
            .xyz(),
    );
    // `.mul( specDominantFactor )`, which is `float( 1 )` off the stochastic
    // path.
    let world_distance = to_var(
        None,
        distance(world_position, hit_world_position.clone()).mul(1.0),
    );
    let reflect_color = to_var(None, texture_uv(color, hit_uv_s.clone()));
    let ratio = to_var(
        None,
        float(1.0).sub(distance_point_plane.div(max_distance.clone())),
    );
    let attenuation = to_var(None, ratio.mul(ratio.clone()));
    let fresnel_coe = to_var(
        None,
        dot(view_incident_dir, view_reflect_dir).add(1.0).div(2.0),
    );
    // `reflectColor.rgb.mul( finalSampleWeight )`, `finalSampleWeight =
    // vec3( metalness )`.
    let weighted_color = reflect_color
        .rgb()
        .mul(metalness.to(Type::Vec3))
        .mul(attenuation.mul(fresnel_coe.clone()));

    statements.push(if_then(
        found_hit,
        vec![
            v_p,
            distance_point_plane.clone(),
            if_then(
                distance_point_plane.less_than_equal(max_distance.clone()),
                vec![
                    hit_world_position,
                    world_distance.clone(),
                    reflect_color.clone(),
                    // `reprojectHitPointHistory()` with no history: the
                    // colour assigned back to itself.
                    reflect_color.rgb().assign(reflect_color.rgb()),
                    ratio,
                    attenuation,
                    fresnel_coe,
                    hit.assign(1.0),
                    output.assign(vec4_join(vec![weighted_color, world_distance])),
                ],
            ),
        ],
    ));

    // Screen-space ray missed: the environment fallback is stochastic-only.
    statements.push(if_then(hit.equal(0.0), vec![]));

    let lum = to_var(None, luminance(output.rgb()).max(1e-4));
    statements.push(lum.clone());
    statements.push(
        output
            .rgb()
            .mul_assign(inputs.max_luminance.div(lum).min(1.0)),
    );
    // Scale the reflection colour by the user-controlled intensity.
    statements.push(output.rgb().mul_assign(inputs.intensity.clone()));

    block(statements, output.max(0.0))
}
