//! Port of `three.js/examples/jsm/tsl/display/SSAONode.js` — screen space
//! ambient occlusion with a Vogel disk and a depth-aware blur.
//!
//! Every frame the node draws one full-screen quad into its AO target: for
//! each pixel it unprojects the scene depth to a view position, takes
//! `samples` points of a Vogel disk of `radius` in the view-aligned plane
//! through it (rotated per pixel by interleaved gradient noise), projects
//! each to the screen and reads one depth there. A sample counts as an
//! occluder by how far above the tangent plane it sits (normalised
//! obscurance, minus `bias`), faded with distance. Then, with the blur on,
//! [`depth_aware_blur`] runs horizontally (AO into the blur target) and
//! vertically (back into the AO target), so the texture downstream always
//! reads the AO target.
//!
//! Three's node is a `Node` with `updateBeforeType = FRAME`; so is this one
//! (its state implements [`NodeUpdate`] and is registered as the updater of
//! the AO texture, the way `passTexture( this, … )` makes it one in three).
//! The `samples` uniform is the loop's end, as in three: changing it does
//! not rebuild anything.
//!
//! Three swaps one blur material's `_blurInput.value` between the two blur
//! draws; the port has two blur materials, one per input texture, each with
//! its own direction uniform, and both build the same WGSL. Not ported: a
//! logarithmic depth buffer (the port has none), and the `RedFormat`
//! targets, which are `rgba8unorm` read through `.x` as in
//! [`GtaoNode`](super::GtaoNode).

use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};

use crate::cameras::PerspectiveCamera;
use crate::materials::MeshBasicNodeMaterial;
use crate::math::Color;
use crate::nodes::frame::register_texture_update;
use crate::nodes::node::{LiveValue, SettableValue, Type, UniformGroup, UniformSource};
use crate::nodes::tsl::{
    block, depth_aware_blur, discard, dot, float, get_screen_position_from_clip,
    get_view_position, if_then, int, interleaved_gradient_noise, loop_options, max,
    pass_depth_texture_uv, pi2, screen_coordinate, texture_uv, texture_with_uv, to_var, uniform,
    uniform_settable, uv, vec4_join, vogel_disk_sample,
};
use crate::nodes::{NodeRef, NodeUpdate, NodeUpdateType};
use crate::objects::QuadMesh;
use crate::renderer::{RenderTarget, RenderTargetOptions, Renderer};
use crate::textures::{DepthTexture, Texture, TextureFilter, TextureType};

use super::SampleFn;

/// `ssao( depthNode, normalNode, camera )`.
///
/// `depth` is the (pre-)pass's depth attachment, `normal` the sampler of the
/// view normal — three's `normalNode.sample` (the AO page's is `( uv ) =>
/// unpackRGBToNormal( prePass.getTextureNode().sample( uv ) )`) — and
/// `camera` the pass's.
pub fn ssao(
    depth: &DepthTexture,
    normal: SampleFn,
    camera: &Rc<RefCell<PerspectiveCamera>>,
) -> SsaoNode {
    SsaoNode::new(depth, normal, camera)
}

/// `SSAONode` — see the module docs. Keep it alive for as long as the graph
/// reads [`node`](Self::node): the renderer reaches it through its texture,
/// weakly.
pub struct SsaoNode {
    /// `this.radius` — the view-space radius of the sample disk, and the
    /// distance scale of the falloff and of the blur's depth rejection.
    /// `0.5` by default.
    pub radius: SettableValue,
    /// `this.intensity` — the occlusion's multiplier, `1` by default.
    pub intensity: SettableValue,
    /// `this.bias` — the cosine below which a sample does not occlude,
    /// `0.025` by default.
    pub bias: SettableValue,
    /// `this.samples` — the number of disk samples, the loop's end. `16` by
    /// default.
    pub samples: SettableValue,
    /// `this.blurSharpness` — how strongly a depth difference rejects a
    /// neighbour in the blur, `2` by default.
    pub blur_sharpness: SettableValue,
    state: Rc<SsaoState>,
}

/// The node's state, shared with the renderer's update-before registry.
struct SsaoState {
    /// `this.depthNode.value`.
    depth: DepthTexture,
    /// `this._aoRenderTarget`.
    ao_target: RenderTarget,
    /// `this._blurRenderTarget`.
    blur_target: RenderTarget,
    /// `_quadMesh` with `this._aoMaterial`.
    ao_quad: QuadMesh,
    /// `_quadMesh` with `this._blurMaterial` reading the AO target.
    blur_horizontal_quad: QuadMesh,
    /// `_quadMesh` with `this._blurMaterial` reading the blur target.
    blur_vertical_quad: QuadMesh,
    /// `this._blurDirection` for the horizontal draw.
    blur_direction_horizontal: SettableValue,
    /// `this._blurDirection` for the vertical draw.
    blur_direction_vertical: SettableValue,
    /// `this._textureNode` — `passTexture( this, aoRenderTarget.texture )`.
    node: NodeRef,
    /// `this.resolutionScale`.
    resolution_scale: Cell<f64>,
    /// `this.blurEnabled`.
    blur_enabled: Cell<bool>,
}

/// The node halves of the uniforms the AO `Fn()` reads.
struct Uniforms {
    radius: NodeRef,
    intensity: NodeRef,
    bias: NodeRef,
    samples: NodeRef,
    projection: NodeRef,
    projection_inverse: NodeRef,
}

/// `new RenderTarget( 1, 1, { depthBuffer: false, format: RedFormat } )`.
/// The port has no `RedFormat`; the quads write their float into an
/// `rgba8unorm` and the texture node reads `.x`.
fn red_target() -> RenderTarget {
    RenderTarget::new_with_options(
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
    .expect("three-rs: the SSAO targets are a colour type")
}

/// `uniform( camera.<matrix> )` — bound by reference, read at draw time.
fn camera_matrix(
    camera: &Rc<RefCell<PerspectiveCamera>>,
    read: fn(&PerspectiveCamera) -> Vec<f64>,
) -> NodeRef {
    let camera: Weak<RefCell<PerspectiveCamera>> = Rc::downgrade(camera);
    uniform(
        UniformSource::Live(LiveValue::new(move || {
            let camera = camera
                .upgrade()
                .expect("three-rs: an SSAONode outlived its camera");
            let value = read(&camera.borrow());
            value
        })),
        Type::Mat4,
        UniformGroup::Object,
        None,
    )
}

impl SsaoNode {
    /// `new SSAONode( depthNode, normalNode, camera )`, with the materials
    /// `setup()` gives it. See [`ssao`].
    pub fn new(
        depth: &DepthTexture,
        normal: SampleFn,
        camera: &Rc<RefCell<PerspectiveCamera>>,
    ) -> Self {
        let ao_target = red_target();
        let blur_target = red_target();

        let (radius, radius_value) = uniform_settable(Type::F32, vec![0.5]);
        let (intensity, intensity_value) = uniform_settable(Type::F32, vec![1.0]);
        let (bias, bias_value) = uniform_settable(Type::F32, vec![0.025]);
        let (samples, samples_value) = uniform_settable(Type::F32, vec![16.0]);
        let (blur_sharpness, blur_sharpness_value) = uniform_settable(Type::F32, vec![2.0]);
        let (direction_h, direction_h_value) = uniform_settable(Type::Vec2, vec![0.0, 0.0]);
        let (direction_v, direction_v_value) = uniform_settable(Type::Vec2, vec![0.0, 0.0]);
        let uniforms = Uniforms {
            radius: radius.clone(),
            intensity,
            bias,
            samples,
            projection: camera_matrix(camera, |c| c.projection_matrix.elements.to_vec()),
            projection_inverse: camera_matrix(camera, |c| {
                c.projection_matrix_inverse.elements.to_vec()
            }),
        };

        let mut ao_material = MeshBasicNodeMaterial::new();
        ao_material.name = "SSAO.AO";
        ao_material.fragment_node = Some(ao_node(depth, &normal, &uniforms));

        let blur_material = |input: &Texture, direction: NodeRef| {
            let mut material = MeshBasicNodeMaterial::new();
            material.name = "SSAO.Blur";
            let input = input.clone();
            material.fragment_node = Some(depth_aware_blur(
                move |coord| texture_with_uv(&input, coord),
                depth,
                direction,
                camera,
                blur_sharpness.clone(),
                radius.clone(),
            ));
            material
        };
        let blur_horizontal = blur_material(&ao_target.texture(), direction_h);
        let blur_vertical = blur_material(&blur_target.texture(), direction_v);

        let node = to_var(None, texture_uv(&ao_target.texture(), uv()));
        let state = Rc::new(SsaoState {
            depth: depth.clone(),
            ao_target,
            blur_target,
            ao_quad: QuadMesh::new(ao_material),
            blur_horizontal_quad: QuadMesh::new(blur_horizontal),
            blur_vertical_quad: QuadMesh::new(blur_vertical),
            blur_direction_horizontal: direction_h_value,
            blur_direction_vertical: direction_v_value,
            node,
            resolution_scale: Cell::new(0.5),
            blur_enabled: Cell::new(true),
        });
        register_texture_update(state.ao_target.texture().id(), &state);
        Self {
            radius: radius_value,
            intensity: intensity_value,
            bias: bias_value,
            samples: samples_value,
            blur_sharpness: blur_sharpness_value,
            state,
        }
    }

    /// `ssaoNode.getTextureNode()` — the occlusion, in `.x`, at `uv()`. It
    /// is the AO target with or without the blur.
    pub fn node(&self) -> NodeRef {
        self.state.node.clone()
    }

    /// `ssaoNode.getTextureNode().sample( uvNode )` — the occlusion at an
    /// explicit uv (a material the scene pass draws reads it at `screenUV`).
    pub fn sample(&self, coord: NodeRef) -> NodeRef {
        texture_uv(&self.state.ao_target.texture(), coord)
    }

    /// The AO target's texture, which [`node`](Self::node) reads.
    pub fn texture(&self) -> Texture {
        self.state.ao_target.texture()
    }

    /// `ssaoNode.resolutionScale = value` — the targets' size as a fraction
    /// of the drawing buffer's, `0.5` by default.
    pub fn set_resolution_scale(&self, scale: f64) {
        self.state.resolution_scale.set(scale);
    }

    /// `ssaoNode.blurEnabled = value` — run the two blur passes after the
    /// AO pass. On by default.
    pub fn set_blur_enabled(&self, on: bool) {
        self.state.blur_enabled.set(on);
    }

    /// The `SSAO.AO` quad material, for the dump gate.
    #[doc(hidden)]
    pub fn quad_material(&self) -> MeshBasicNodeMaterial {
        self.state.ao_quad.material.clone()
    }

    /// The horizontal `SSAO.Blur` quad material, for the dump gate.
    #[doc(hidden)]
    pub fn blur_quad_material(&self) -> MeshBasicNodeMaterial {
        self.state.blur_horizontal_quad.material.clone()
    }
}

/// `SSAONode.setup()`'s `ao` `Fn()`, which has no layout and so inlines.
#[inline(never)]
fn ao_node(depth: &DepthTexture, normal: &SampleFn, u: &Uniforms) -> NodeRef {
    let uv_node = uv();
    // `sampleDepth( uv )` — `this.depthNode.sample( uv ).r`; no logarithmic
    // depth buffer.
    let sample_depth = |coord: NodeRef| pass_depth_texture_uv(depth, coord);

    let depth_value = to_var(None, sample_depth(uv_node.clone()));
    let discard_far = if_then(depth_value.greater_than_equal(1.0), vec![discard()]);
    let view_position = to_var(
        None,
        get_view_position(
            uv_node.clone(),
            depth_value.clone(),
            u.projection_inverse.clone(),
        ),
    );
    let view_normal = to_var(None, normal(uv_node).rgb().normalize());
    let phi = to_var(
        None,
        interleaved_gradient_noise(screen_coordinate()).mul(pi2()),
    );
    let samples = u.samples.clone();
    let clip_position = to_var(
        None,
        u.projection
            .mul(vec4_join(vec![view_position.clone(), float(1.0)])),
    );
    let occlusion = to_var(None, float(0.0));

    let taps = loop_options("i", Type::I32, int(0), samples.clone(), "<", |i| {
        let offset = vogel_disk_sample(i.clone(), samples.clone(), phi.clone()).mul(u.radius.clone());
        let clip_offset = u
            .projection
            .mul(vec4_join(vec![offset, float(0.0), float(0.0)]));
        let sample_uv = get_screen_position_from_clip(clip_position.add(clip_offset));
        let sample_view_position = get_view_position(
            sample_uv.clone(),
            sample_depth(sample_uv),
            u.projection_inverse.clone(),
        );
        let v = to_var(None, sample_view_position.sub(view_position.clone()));
        let dist = to_var(None, v.length());
        let cos_angle = dot(v.clone(), view_normal.clone()).div(max(dist.clone(), float(0.0001)));
        let falloff = u.radius.div(u.radius.add(dist.clone()));
        vec![
            v,
            dist,
            occlusion.add_assign(max(cos_angle.sub(u.bias.clone()), 0.0).mul(falloff)),
        ]
    });

    block(
        vec![
            depth_value,
            discard_far,
            view_position,
            view_normal,
            phi,
            clip_position,
            occlusion.clone(),
            taps,
        ],
        occlusion
            .div(samples)
            .mul(u.intensity.clone())
            .one_minus()
            .clamp(0.0, 1.0),
    )
}

impl NodeUpdate for SsaoState {
    /// `this.updateBeforeType = NodeUpdateType.FRAME`.
    fn update_before_type(&self) -> NodeUpdateType {
        NodeUpdateType::Frame
    }

    /// `SSAONode.updateBefore( frame )`.
    fn update_before(&self, renderer: &mut Renderer) -> bool {
        // The pre-pass first, with the AO context lifted — see
        // `GtaoState::update_before`. The normal is a sampler here, so the
        // pass is found through the depth attachment it owns.
        if let Some(pass) = crate::nodes::frame::texture_update(self.depth.id()) {
            let outer_ao = renderer.context_ao.take();
            renderer.update_before_node(&pass);
            renderer.context_ao = outer_ao;
        }

        // `_rendererState = RendererUtils.resetRendererState( renderer, … )`.
        let previous_target = renderer.render_target();
        let previous_mrt = renderer.mrt();
        let previous_auto_clear = renderer.auto_clear;
        let previous_clear_color = renderer.clear_color();
        let previous_clear_alpha = renderer.clear_alpha();
        renderer.set_mrt(None);
        renderer.auto_clear = true;

        // `setSize( renderer.getDrawingBufferSize() )`.
        let (width, height) = renderer.drawing_buffer_size();
        let scale = self.resolution_scale.get();
        let width = ((scale * f64::from(width)).round() as u32).max(1);
        let height = ((scale * f64::from(height)).round() as u32).max(1);
        for target in [&self.ao_target, &self.blur_target] {
            if target.size() != (width, height) {
                target.set_size(width, height);
            }
        }

        renderer.set_clear_color(Color::new(1.0, 1.0, 1.0), 1.0);

        // Ambient occlusion.
        renderer.set_render_target(Some(self.ao_target.clone()));
        renderer.render_quad(&self.ao_quad);

        // Separable, depth-aware blur: horizontal (AO -> blur) then vertical
        // (blur -> AO).
        if self.blur_enabled.get() {
            self.blur_direction_horizontal
                .set(vec![1.0 / f64::from(width), 0.0]);
            renderer.set_render_target(Some(self.blur_target.clone()));
            renderer.render_quad(&self.blur_horizontal_quad);

            self.blur_direction_vertical
                .set(vec![0.0, 1.0 / f64::from(height)]);
            renderer.set_render_target(Some(self.ao_target.clone()));
            renderer.render_quad(&self.blur_vertical_quad);
        }

        // `RendererUtils.restoreRendererState( renderer, _rendererState )`.
        renderer.set_render_target(previous_target);
        renderer.set_mrt(previous_mrt);
        renderer.set_clear_color(previous_clear_color, previous_clear_alpha);
        renderer.auto_clear = previous_auto_clear;
        true
    }
}
