//! Port of `three.js/examples/jsm/tsl/display/RecurrentDenoiseNode.js` — the
//! edge-aware spatial denoiser, with an optional Karis-style temporal blend,
//! that `webgpu_postprocessing_ssr_denoise` runs over its temporally
//! reprojected SSR.
//!
//! Every pixel takes eight taps on a golden-angle Vogel disk, laid out in
//! view space on the surface's tangent plane (`'diffuse'`) or across the
//! specular lobe's dominant direction (`'specular'`), rotated per pixel by
//! the analytic R² noise of [`bind_analytic_noise`] and projected back to
//! screen space. Each tap is weighted by luma, plane-distance, lobe-normal,
//! albedo (when a diffuse G-buffer is bound), roughness (`'specular'`) and
//! alpha edge-stopping terms, where the raw input's alpha is either an SSR
//! ray length or an AO factor ([`DenoiseAlphaSource`]). The radius shrinks
//! with the input's accumulated frame count (`1 / alpha`), and a running
//! polar bias skews later taps toward the directions that weighed most. With
//! `accumulate` the spatially denoised colour is blended with the denoised
//! raw input by the frame weight, and that weight is written to alpha for
//! the next temporal pass.
//!
//! Three's node is a `Node` with `updateBeforeType = FRAME` that draws its
//! one quad into a half-float target sized to the drawing buffer, and hands
//! the graph `passTexture( this, target.texture )`. The port's state
//! implements [`NodeUpdate`] and is registered as the updater of that
//! texture, as [`SharpenNode`](super::SharpenNode)'s is. `updateBefore()`
//! also asks for the input's own pass first (`frame.updateBeforeNode(
//! this.textureNode.passNode )`): the port looks the input texture up in the
//! frame's texture-update registry, as [`SsrNode`](super::SsrNode) does.
//!
//! `alphaSource` is read in `setup()`, so it is compiled into the shader;
//! [`RecurrentDenoiseNode::set_alpha_source`] rebuilds the quad material.
//!
//! Differences from three:
//!
//! - `raw = null`: three's `setup()` always builds `getNeighborhoodStats`,
//!   which calls `texture( null )` and throws. The port reads the input
//!   texture in its place, which is what the source's own comment says the
//!   fallback is ("sampleRaw falls back to the filtered texture"); the
//!   temporal blend is still three's `mix()` branch for a missing raw.
//! - `material.contextNode = context( builder.getSharedContext() )` is not
//!   ported: the quad material is built in the constructor, in a
//!   `NodeBuilder` pass of its own, as [`RttNode`](super::RttNode)'s is.
//! - The camera is a [`PerspectiveCamera`]: three copies `fov` only from
//!   one, and the port's display nodes take that type.
//! - `dispose()` consumes the node: the target's GPU texture goes with its
//!   last handle, and the registry only holds the node weakly.

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use crate::cameras::PerspectiveCamera;
use crate::materials::MeshBasicNodeMaterial;
use crate::math::{Color, Matrix4};
use crate::nodes::frame::register_texture_update;
use crate::nodes::node::{FnDef, Lazy, SettableValue, Type};
use crate::nodes::tsl::{
    abs, bind_analytic_noise, block, boolean, call, cross, discard, dot, exp, float,
    get_screen_position, get_view_position, if_else, if_then, int, log, loop_range, luminance,
    mat2_join, max, mix, pass_depth_texture_uv, pi, property, reflect, shader_fn, smoothstep, sqrt,
    texture_uv, to_const, to_var, transform_normal_by_inverse_view_matrix, uniform_settable,
    unpack_rgb_to_normal, uv, vec2, vec2_join, vec3, vec3_join, vec4_join,
    ENV_RAY_LENGTH_THRESHOLD,
};
use crate::nodes::{NodeRef, NodeUpdate, NodeUpdateType};
use crate::objects::QuadMesh;
use crate::renderer::{RenderTarget, RenderTargetOptions, Renderer};
use crate::textures::{DepthTexture, Texture, TextureFilter, TextureType};

use super::SampleFn;

/// `KERNEL_SAMPLES` — the Vogel disk's tap count.
const KERNEL_SAMPLES: i64 = 8;
/// `NOISE_ROTATION_SEED` — the analytic noise's R² phase.
const NOISE_ROTATION_SEED: i32 = 83;
/// `WORLD_RADIUS_SCALE`.
const WORLD_RADIUS_SCALE: f64 = 0.1;
/// `AO_EDGE_STOPPING_BIAS`.
const AO_EDGE_STOPPING_BIAS: f64 = 0.05;
/// `AGGRESSIVITY_RADIUS_MIN`.
const AGGRESSIVITY_RADIUS_MIN: f64 = 0.001;
/// `DIFFUSE_CHROMA_WEIGHT`.
const DIFFUSE_CHROMA_WEIGHT: f64 = 2.0;
/// `FLICKER_COV_GATE_MIN` — below this neighbourhood luma coefficient of
/// variation a region counts as flicker-free.
const FLICKER_COV_GATE_MIN: f64 = 0.1;
/// `FLICKER_COV_GATE_MAX` — above this it counts as noisy.
const FLICKER_COV_GATE_MAX: f64 = 2.0;
/// `EXP_WEIGHT_SCALE`.
const EXP_WEIGHT_SCALE: f64 = 4.0;
/// `NORMAL_ENCODING_ERROR` — one 8-bit step and a half, the smallest lobe
/// half-angle.
const NORMAL_ENCODING_ERROR: f64 = 1.5 / 255.0;
/// `EPSILON` — `MathNode.js`' `float( 1e-6 )`.
const EPSILON: f64 = 1e-6;

/// `DenoiseMode` — the kernel `setup()` builds.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum DenoiseMode {
    /// `'diffuse'` (SSGI): taps on the surface's tangent plane, the radius
    /// scaled by the raw input's AO.
    #[default]
    Diffuse,
    /// `'specular'` (SSR): taps across the specular lobe's dominant
    /// direction, the radius scaled by ray length and roughness, with a
    /// roughness edge-stopping term.
    Specular,
}

/// `DenoiseAlphaSource` — what the raw input's alpha holds, which picks the
/// alpha edge-stopping term.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum DenoiseAlphaSource {
    /// `'raylength'`: an SSR ray length (above
    /// [`ENV_RAY_LENGTH_THRESHOLD`] an environment miss), compared as a
    /// hit-distance factor. Three's default.
    #[default]
    RayLength,
    /// `'ao'`: an ambient-occlusion factor.
    Ao,
    /// `'none'`: no alpha edge stopping.
    None,
}

/// `RecurrentDenoiseNodeOptions`.
///
/// The G-buffer sources are samplers, `node.sample( uv )` for any uv, the
/// way three calls `.sample()` on the nodes it is given. Each returns what
/// three's node would: `normal` the packed view normal (`rgb`, unpacked by
/// the denoiser), `metal_roughness` a `vec2( metalness, roughness )`, and
/// `diffuse` the albedo (`rgb`).
#[derive(Clone)]
pub struct RecurrentDenoiseOptions {
    /// `depth` — the scene pass's depth texture, for view-space edge
    /// stopping. `None` is three's `float( 0.5 )` everywhere.
    pub depth: Option<DepthTexture>,
    /// `normal` — the packed view-space normal. `None` reads `vec3( 0, 0, 1
    /// )` before unpacking, as three does.
    pub normal: Option<SampleFn>,
    /// `metalRoughness` — `vec2( metalness, roughness )`. `None` is `vec2(
    /// 0, 1 )`.
    pub metal_roughness: Option<SampleFn>,
    /// `diffuse` — the albedo, for chromatic edge stopping (a term only a
    /// bound diffuse adds). `None` is `vec3( 0 )`.
    pub diffuse: Option<SampleFn>,
    /// `raw` — the unfiltered input (raw SSR or SSGI) whose alpha carries
    /// the [`DenoiseAlphaSource`], sampled for the luma and alpha terms and
    /// blended in by the temporal step. See the module docs for `None`.
    pub raw: Option<Texture>,
    /// `mode`. [`DenoiseMode::Diffuse`] by default.
    pub mode: DenoiseMode,
    /// `accumulate` — blend the denoised colour with the denoised raw input
    /// by the frame weight and write that weight to alpha; `false` filters
    /// spatially and passes the input's alpha through. `true` by default.
    pub accumulate: bool,
}

impl Default for RecurrentDenoiseOptions {
    fn default() -> Self {
        Self {
            depth: None,
            normal: None,
            metal_roughness: None,
            diffuse: None,
            raw: None,
            mode: DenoiseMode::Diffuse,
            accumulate: true,
        }
    }
}

/// `recurrentDenoise( inputTexture, camera, options )`.
///
/// `input` is the texture three's `toTextureNode( inputTexture )` wraps —
/// the temporally filtered colour, typically a node's output texture (a
/// [`TraaNode`](super::TraaNode)'s, or another `RecurrentDenoiseNode`'s
/// [`texture`](RecurrentDenoiseNode::texture)). Whatever fills it is drawn
/// first each frame.
pub fn recurrent_denoise(
    input: &Texture,
    camera: Rc<RefCell<PerspectiveCamera>>,
    options: RecurrentDenoiseOptions,
) -> RecurrentDenoiseNode {
    RecurrentDenoiseNode::new(input, camera, options)
}

/// `RecurrentDenoiseNode` — see the module docs. Keep it alive for as long
/// as the graph reads [`node`](Self::node): the renderer reaches it through
/// its texture, weakly.
pub struct RecurrentDenoiseNode(Rc<RecurrentDenoiseState>);

/// A uniform node and the handle that writes it.
type Uniform = (NodeRef, SettableValue);

/// Every uniform the shader reads.
struct Uniforms {
    luma_phi: Uniform,
    depth_phi: Uniform,
    normal_phi: Uniform,
    radius: Uniform,
    alpha_phi: Uniform,
    roughness_phi: Uniform,
    diffuse_phi: Uniform,
    adapt: Uniform,
    smooth_disocclusions: Uniform,
    strength: Uniform,
    max_frames: Uniform,
    flicker_suppression: Uniform,
    adaptive_trust: Uniform,
    /// `this._noiseIndex` — the frame id.
    noise_index: Uniform,
    /// `this._resolution`.
    resolution: Uniform,
    /// `this._fovY`, in radians.
    fov_y: Uniform,
    /// `this._cameraProjectionMatrixInverse`.
    projection_inverse: Uniform,
    /// `this._cameraProjectionMatrix`.
    projection: Uniform,
    /// `this._viewMatrix` — the camera's `matrixWorldInverse`.
    view: Uniform,
}

/// The node's state, shared with the renderer's update-before registry.
pub(crate) struct RecurrentDenoiseState {
    /// `this.textureNode`'s texture.
    input: Texture,
    /// `this.camera`.
    camera: Rc<RefCell<PerspectiveCamera>>,
    /// The options, kept to rebuild the material.
    options: RecurrentDenoiseOptions,
    /// `this.alphaSource`.
    alpha_source: Cell<DenoiseAlphaSource>,
    uniforms: Uniforms,
    /// `this._renderTarget`.
    target: RenderTarget,
    /// `_quadMesh` with `this._material`.
    quad: RefCell<QuadMesh>,
    /// `this._textureNode` — `passTexture( this, target.texture )`.
    node: NodeRef,
}

fn mat4_values(m: &Matrix4) -> Vec<f64> {
    m.elements.to_vec()
}

impl RecurrentDenoiseNode {
    /// `new RecurrentDenoiseNode( inputTexture, camera, options )`, with the
    /// material `setup()` gives it for the default `alphaSource`
    /// ([`DenoiseAlphaSource::RayLength`]).
    pub fn new(
        input: &Texture,
        camera: Rc<RefCell<PerspectiveCamera>>,
        options: RecurrentDenoiseOptions,
    ) -> Self {
        // `new RenderTarget( 1, 1, { depthBuffer: false, type: HalfFloatType } )`.
        let target = RenderTarget::new_with_options(
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
        .expect("three-rs: the denoise target is a HalfFloat colour type");

        let f32_uniform = |v: f64| uniform_settable(Type::F32, vec![v]);
        let (fov, projection, projection_inverse, view) = {
            let camera = camera.borrow();
            (
                camera.fov.to_radians(),
                mat4_values(&camera.projection_matrix),
                mat4_values(&camera.projection_matrix_inverse),
                mat4_values(&camera.matrix_world_inverse),
            )
        };
        let uniforms = Uniforms {
            luma_phi: f32_uniform(5.0),
            depth_phi: f32_uniform(5.0),
            normal_phi: f32_uniform(5.0),
            radius: f32_uniform(5.0),
            alpha_phi: f32_uniform(1.0),
            roughness_phi: f32_uniform(100.0),
            diffuse_phi: f32_uniform(100.0),
            adapt: f32_uniform(0.5),
            smooth_disocclusions: uniform_settable(Type::Bool, vec![1.0]),
            strength: f32_uniform(0.25),
            max_frames: f32_uniform(32.0),
            flicker_suppression: f32_uniform(1.0),
            adaptive_trust: f32_uniform(0.0),
            noise_index: f32_uniform(0.0),
            resolution: uniform_settable(Type::Vec2, vec![0.0, 0.0]),
            fov_y: f32_uniform(fov),
            projection_inverse: uniform_settable(Type::Mat4, projection_inverse),
            projection: uniform_settable(Type::Mat4, projection),
            view: uniform_settable(Type::Mat4, view),
        };

        let alpha_source = DenoiseAlphaSource::default();
        let material = denoise_material(input, &options, alpha_source, &uniforms);
        let node = to_var(None, texture_uv(&target.texture(), uv()));

        let state = Rc::new(RecurrentDenoiseState {
            input: input.clone(),
            camera,
            options,
            alpha_source: Cell::new(alpha_source),
            uniforms,
            target,
            quad: RefCell::new(QuadMesh::new(material)),
            node,
        });
        register_texture_update(state.target.texture().id(), &state);
        Self(state)
    }

    /// `denoiseNode.getTextureNode()` — the denoised frame.
    pub fn node(&self) -> NodeRef {
        self.0.node.clone()
    }

    /// `this._renderTarget.texture` — what a following pass (a second
    /// denoiser, a temporal reprojection) takes as its input.
    pub fn texture(&self) -> Texture {
        self.0.target.texture()
    }

    /// `denoiseNode.getRenderTarget()` — the output target, for temporal
    /// feedback loops (`SSRNode.setHistory()` reads it).
    pub fn get_render_target(&self) -> &RenderTarget {
        &self.0.target
    }

    /// [`get_render_target`](Self::get_render_target), in the crate's
    /// accessor spelling.
    pub fn render_target(&self) -> &RenderTarget {
        &self.0.target
    }

    /// `RecurrentDenoiseNode.setSize( width, height )`. `updateBefore()`
    /// sizes the target to the drawing buffer every frame, so a caller
    /// rarely needs this.
    pub fn set_size(&self, width: u32, height: u32) {
        self.0.set_size(width, height);
    }

    /// `denoiseNode.mode`.
    pub fn mode(&self) -> DenoiseMode {
        self.0.options.mode
    }

    /// `denoiseNode.accumulate`.
    pub fn accumulate(&self) -> bool {
        self.0.options.accumulate
    }

    /// `denoiseNode.alphaSource`. [`DenoiseAlphaSource::RayLength`] by
    /// default.
    pub fn alpha_source(&self) -> DenoiseAlphaSource {
        self.0.alpha_source.get()
    }

    /// `denoiseNode.alphaSource = value`. Three reads it in `setup()`, so it
    /// is compiled into the shader: a change rebuilds the quad material, and
    /// the same value is a no-op.
    pub fn set_alpha_source(&self, value: DenoiseAlphaSource) {
        if value == self.0.alpha_source.get() {
            return;
        }
        self.0.alpha_source.set(value);
        let material = denoise_material(&self.0.input, &self.0.options, value, &self.0.uniforms);
        self.0.quad.borrow_mut().material = material;
    }

    /// `denoiseNode.lumaPhi.value` — luma edge stopping. 5 by default.
    pub fn luma_phi(&self) -> &SettableValue {
        &self.0.uniforms.luma_phi.1
    }

    /// `denoiseNode.depthPhi.value` — plane-distance edge stopping. 5 by
    /// default.
    pub fn depth_phi(&self) -> &SettableValue {
        &self.0.uniforms.depth_phi.1
    }

    /// `denoiseNode.normalPhi.value` — lobe-normal edge stopping; the shader
    /// reads `1 - normalPhi`. 5 by default.
    pub fn normal_phi(&self) -> &SettableValue {
        &self.0.uniforms.normal_phi.1
    }

    /// `denoiseNode.radius.value` — the kernel radius, in tenths of a world
    /// unit before the per-pixel scaling. 5 by default.
    pub fn radius(&self) -> &SettableValue {
        &self.0.uniforms.radius.1
    }

    /// `denoiseNode.alphaPhi.value` — alpha (ray length or AO) edge
    /// stopping. 1 by default.
    pub fn alpha_phi(&self) -> &SettableValue {
        &self.0.uniforms.alpha_phi.1
    }

    /// `denoiseNode.roughnessPhi.value` — roughness edge stopping, in
    /// `'specular'` mode. 100 by default.
    pub fn roughness_phi(&self) -> &SettableValue {
        &self.0.uniforms.roughness_phi.1
    }

    /// `denoiseNode.diffusePhi.value` — albedo edge stopping, scaled by
    /// metalness, when a diffuse G-buffer is bound. 100 by default.
    pub fn diffuse_phi(&self) -> &SettableValue {
        &self.0.uniforms.diffuse_phi.1
    }

    /// `denoiseNode.adapt.value` — how strongly each tap's weight shrinks
    /// the radius and skews later taps. 0.5 by default.
    pub fn adapt(&self) -> &SettableValue {
        &self.0.uniforms.adapt.1
    }

    /// `denoiseNode.smoothDisocclusions.value` — denoise the input's alpha
    /// (its accumulation speed) too, for smoother disocclusions. A bool,
    /// `true` (1) by default.
    pub fn smooth_disocclusions(&self) -> &SettableValue {
        &self.0.uniforms.smooth_disocclusions.1
    }

    /// `denoiseNode.strength.value` — how quickly the radius closes as
    /// frames accumulate. 0.25 by default.
    pub fn strength(&self) -> &SettableValue {
        &self.0.uniforms.strength.1
    }

    /// `denoiseNode.maxFrames.value`. 32 by default; three declares it and
    /// no shader reads it.
    pub fn max_frames(&self) -> &SettableValue {
        &self.0.uniforms.max_frames.1
    }

    /// `denoiseNode.flickerSuppression.value` — the temporal blend's
    /// inverse-luminance weighting. 1 by default.
    pub fn flicker_suppression(&self) -> &SettableValue {
        &self.0.uniforms.flicker_suppression.1
    }

    /// `denoiseNode.adaptiveTrust.value` — how far a noisy neighbourhood
    /// pulls the temporal blend toward history. 0 (off) by default.
    pub fn adaptive_trust(&self) -> &SettableValue {
        &self.0.uniforms.adaptive_trust.1
    }

    /// The `RecurrentDenoise` quad material, for `examples/dump_wgsl.rs` and
    /// the dump gate.
    #[doc(hidden)]
    pub fn quad_material(&self) -> MeshBasicNodeMaterial {
        self.0.quad.borrow().material.clone()
    }

    /// `denoiseNode.dispose()` — drops the node, its target and its
    /// material. See the module docs.
    pub fn dispose(self) {}
}

impl RecurrentDenoiseState {
    /// `RecurrentDenoiseNode.setSize( width, height )`.
    fn set_size(&self, width: u32, height: u32) {
        self.target.set_size(width, height);
        self.uniforms
            .resolution
            .1
            .set(vec![width as f64, height as f64]);
    }
}

impl NodeUpdate for RecurrentDenoiseState {
    /// `this.updateBeforeType = NodeUpdateType.FRAME`.
    fn update_before_type(&self) -> NodeUpdateType {
        NodeUpdateType::Frame
    }

    /// `RecurrentDenoiseNode.updateBefore( frame )`.
    fn update_before(&self, renderer: &mut Renderer) -> bool {
        let (width, height) = renderer.drawing_buffer_size();
        let needs_restart = self.target.size() != (width, height);
        self.set_size(width, height);

        {
            let camera = self.camera.borrow();
            let u = &self.uniforms;
            u.projection.1.set(mat4_values(&camera.projection_matrix));
            u.projection_inverse
                .1
                .set(mat4_values(&camera.projection_matrix_inverse));
            u.view.1.set(mat4_values(&camera.matrix_world_inverse));
            u.fov_y.1.set(vec![camera.fov.to_radians()]);
        }
        self.uniforms
            .noise_index
            .1
            .set(vec![renderer.node_frame().frame_id as f64]);

        // The input's pass first: the quad is drawn here, not through the
        // pipeline's output graph, so nothing else schedules it.
        if let Some(pass) = crate::nodes::frame::texture_update(self.input.id()) {
            renderer.update_before_node(&pass);
        }

        // `_rendererState = RendererUtils.resetRendererState( renderer, … )`.
        let previous_target = renderer.render_target();
        let previous_mrt = renderer.mrt();
        let previous_auto_clear = renderer.auto_clear;
        let previous_clear_color = renderer.clear_color();
        let previous_clear_alpha = renderer.clear_alpha();
        renderer.set_mrt(None);
        renderer.set_clear_color(Color::new(0.0, 0.0, 0.0), 1.0);
        renderer.auto_clear = true;

        if needs_restart {
            renderer.init_render_target(&self.target);
            renderer.set_render_target(Some(self.target.clone()));
            renderer.clear(true, true);
            renderer.set_render_target(None);
        }

        renderer.set_render_target(Some(self.target.clone()));
        renderer.render_quad(&self.quad.borrow());
        renderer.set_render_target(None);

        // `RendererUtils.restoreRendererState( renderer, _rendererState )`.
        renderer.set_render_target(previous_target);
        renderer.set_mrt(previous_mrt);
        renderer.set_clear_color(previous_clear_color, previous_clear_alpha);
        renderer.auto_clear = previous_auto_clear;
        true
    }
}

// ---------------------------------------------------------------------------
// module-level `Fn`s
// ---------------------------------------------------------------------------

/// `vogelDisk( i, radius )` — golden-angle Vogel disk offset. Has a layout.
#[inline(never)]
fn vogel_disk() -> Rc<FnDef> {
    thread_local! { static CELL: Lazy<Rc<FnDef>> = const { Lazy::new() }; }
    CELL.with(|c| {
        c.get(|| {
            shader_fn(
                Some("vogelDisk"),
                vec![("i", Type::F32), ("radius", Type::F32)],
                Type::Vec2,
                |args| {
                    let (i, radius) = (&args[0], &args[1]);
                    let sample_count = KERNEL_SAMPLES as f64;
                    let theta = i.add(0.5).mul(2.399827721492203);
                    let r = radius.mul(sqrt(i.add(0.5).div(sample_count)));
                    vec2_join(vec![theta.cos(), theta.sin()]).mul(r)
                },
            )
        })
    })
}

/// `diffuseColorDistance( a, b, compressLuma )` — chromatic distance between
/// two linear albedos in YCoCg. Has a layout.
#[inline(never)]
fn diffuse_color_distance() -> Rc<FnDef> {
    thread_local! { static CELL: Lazy<Rc<FnDef>> = const { Lazy::new() }; }
    CELL.with(|c| {
        c.get(|| {
            shader_fn(
                Some("diffuseColorDistance"),
                vec![
                    ("a", Type::Vec3),
                    ("b", Type::Vec3),
                    ("compressLuma", Type::F32),
                ],
                Type::F32,
                |args| {
                    let (a, b, compress_luma) = (&args[0], &args[1], &args[2]);
                    let to_ycocg = |c: &NodeRef| {
                        vec3_join(vec![
                            dot(c.clone(), vec3(0.25, 0.5, 0.25)),
                            c.x().sub(c.z()),
                            c.y().sub(c.x().add(c.z()).mul(0.5)),
                        ])
                    };
                    let ya = to_ycocg(a);
                    let yb = to_ycocg(b);
                    let compress =
                        |l: NodeRef| mix(l.clone(), log(l.add(1.0)), compress_luma.clone());
                    let d_luma = abs(compress(ya.x()).sub(compress(yb.x())));
                    let d_chroma = vec2_join(vec![ya.y().sub(yb.y()), ya.z().sub(yb.z())]).length();
                    d_luma.add(d_chroma.mul(DIFFUSE_CHROMA_WEIGHT))
                },
            )
        })
    })
}

/// `_temporalWeight( x, strength )` — `1 / x^strength`. Has a layout
/// (`temporalWeight`).
#[inline(never)]
fn temporal_weight() -> Rc<FnDef> {
    thread_local! { static CELL: Lazy<Rc<FnDef>> = const { Lazy::new() }; }
    CELL.with(|c| {
        c.get(|| {
            shader_fn(
                Some("temporalWeight"),
                vec![("x", Type::F32), ("strength", Type::F32)],
                Type::F32,
                |args| float(1.0).div(args[0].pow(args[1].clone())),
            )
        })
    })
}

/// `getTemporalVarianceFactor( frameNum, strength )` — history confidence in
/// [0.05, 1]. Has a layout.
#[inline(never)]
fn get_temporal_variance_factor() -> Rc<FnDef> {
    thread_local! { static CELL: Lazy<Rc<FnDef>> = const { Lazy::new() }; }
    CELL.with(|c| {
        c.get(|| {
            shader_fn(
                Some("getTemporalVarianceFactor"),
                vec![("frameNum", Type::F32), ("strength", Type::F32)],
                Type::F32,
                |args| call(&temporal_weight(), vec![args[0].clone(), args[1].clone()]).max(0.05),
            )
        })
    })
}

/// `computeFrustumSize( viewZ, tanHalfFovY )` — the frustum's world height
/// at `viewZ` (REBLUR). Has a layout.
#[inline(never)]
fn compute_frustum_size() -> Rc<FnDef> {
    thread_local! { static CELL: Lazy<Rc<FnDef>> = const { Lazy::new() }; }
    CELL.with(|c| {
        c.get(|| {
            shader_fn(
                Some("computeFrustumSize"),
                vec![("viewZ", Type::F32), ("tanHalfFovY", Type::F32)],
                Type::F32,
                |args| float(2.0).mul(args[0].clone()).mul(args[1].clone()),
            )
        })
    })
}

/// `computeHitDistFactor( worldRayLength, viewZ, tanHalfFovY )` — a ray
/// length as a fraction of the frustum height, in [0, 1] (REBLUR). Has a
/// layout.
#[inline(never)]
fn compute_hit_dist_factor() -> Rc<FnDef> {
    thread_local! { static CELL: Lazy<Rc<FnDef>> = const { Lazy::new() }; }
    CELL.with(|c| {
        c.get(|| {
            shader_fn(
                Some("computeHitDistFactor"),
                vec![
                    ("worldRayLength", Type::F32),
                    ("viewZ", Type::F32),
                    ("tanHalfFovY", Type::F32),
                ],
                Type::F32,
                |args| {
                    let frustum_size = call(
                        &compute_frustum_size(),
                        vec![args[1].clone(), args[2].clone()],
                    );
                    args[0].div(frustum_size.max(1e-6)).clamp(0.0, 1.0)
                },
            )
        })
    })
}

/// `mapAo( aoVal )` — `aoVal^0.1`. A plain `Fn`, so it inlines.
fn map_ao(ao: &NodeRef) -> NodeRef {
    ao.pow(0.1)
}

/// `getSpecularDominantDirection( N, V, roughness )` — smooth surfaces lean
/// toward the reflection, rough ones toward the normal. Has a layout.
#[inline(never)]
fn get_specular_dominant_direction() -> Rc<FnDef> {
    thread_local! { static CELL: Lazy<Rc<FnDef>> = const { Lazy::new() }; }
    CELL.with(|c| {
        c.get(|| {
            shader_fn(
                Some("getSpecularDominantDirection"),
                vec![
                    ("N", Type::Vec3),
                    ("V", Type::Vec3),
                    ("roughness", Type::F32),
                ],
                Type::Vec3,
                |args| {
                    let (n, v, roughness) = (&args[0], &args[1], &args[2]);
                    mix(
                        n.clone(),
                        reflect(v.negate(), n.clone()),
                        roughness.one_minus(),
                    )
                    .normalize()
                },
            )
        })
    })
}

/// `specularLobeTanHalfAngle( roughness, percent )` — the GGX lobe's
/// half-angle tangent enclosing `percent` of its volume. Has a layout.
#[inline(never)]
fn specular_lobe_tan_half_angle() -> Rc<FnDef> {
    thread_local! { static CELL: Lazy<Rc<FnDef>> = const { Lazy::new() }; }
    CELL.with(|c| {
        c.get(|| {
            shader_fn(
                Some("specularLobeTanHalfAngle"),
                vec![("roughness", Type::F32), ("percent", Type::F32)],
                Type::F32,
                |args| {
                    let (roughness, percent) = (&args[0], &args[1]);
                    let alpha = roughness.mul(roughness.clone());
                    alpha.mul(sqrt(percent.div(float(1.0).sub(percent.clone()).max(1e-6))))
                },
            )
        })
    })
}

/// `lobeNormalFalloff( roughness, aggressivity, invNormalPhi )` — the
/// per-pixel Gaussian falloff of the normal weight, `2·4 / halfAngle²`. Has
/// a layout.
#[inline(never)]
fn lobe_normal_falloff() -> Rc<FnDef> {
    thread_local! { static CELL: Lazy<Rc<FnDef>> = const { Lazy::new() }; }
    CELL.with(|c| {
        c.get(|| {
            shader_fn(
                Some("lobeNormalFalloff"),
                vec![
                    ("roughness", Type::F32),
                    ("aggressivity", Type::F32),
                    ("invNormalPhi", Type::F32),
                ],
                Type::F32,
                |args| {
                    let (roughness, aggressivity, inv_normal_phi) = (&args[0], &args[1], &args[2]);
                    let percent = mix(inv_normal_phi.pow2(), float(0.0), aggressivity.sqrt())
                        .clamp(0.1, 0.99);
                    let tan_half_angle = call(
                        &specular_lobe_tan_half_angle(),
                        vec![roughness.clone(), percent],
                    );
                    let lobe_half_angle = max(tan_half_angle.atan(), float(NORMAL_ENCODING_ERROR));
                    let inv_half_angle = float(1.0).div(lobe_half_angle);
                    inv_half_angle
                        .mul(inv_half_angle.clone())
                        .mul(2.0 * EXP_WEIGHT_SCALE)
                },
            )
        })
    })
}

/// `lobeNormalWeight( viewNormal, nNormalV, lobeFalloff )` — the normal
/// edge-stopping weight, `exp( falloff · (cosθ − 1) )`. Has a layout.
#[inline(never)]
fn lobe_normal_weight() -> Rc<FnDef> {
    thread_local! { static CELL: Lazy<Rc<FnDef>> = const { Lazy::new() }; }
    CELL.with(|c| {
        c.get(|| {
            shader_fn(
                Some("lobeNormalWeight"),
                vec![
                    ("viewNormal", Type::Vec3),
                    ("nNormalV", Type::Vec3),
                    ("lobeFalloff", Type::F32),
                ],
                Type::F32,
                |args| {
                    let cos_a = dot(args[0].clone(), args[1].clone());
                    exp(cos_a.sub(1.0).mul(args[2].clone()))
                },
            )
        })
    })
}

/// `planeDistance( position, nPosition, normal )` — the view-space distance
/// of a tap from the centre's plane. Has a layout.
#[inline(never)]
fn plane_distance() -> Rc<FnDef> {
    thread_local! { static CELL: Lazy<Rc<FnDef>> = const { Lazy::new() }; }
    CELL.with(|c| {
        c.get(|| {
            shader_fn(
                Some("planeDistance"),
                vec![
                    ("position", Type::Vec3),
                    ("nPosition", Type::Vec3),
                    ("normal", Type::Vec3),
                ],
                Type::F32,
                |args| abs(dot(args[0].sub(args[1].clone()), args[2].clone())),
            )
        })
    })
}

/// `karisTemporalBlend( denoisedRgb, denoisedRaw, a, flickerSuppression,
/// adaptiveTrust, nbhdMeanLuma, nbhdStddevLuma )` — the inverse-luminance
/// temporal blend with optional adaptive trust. Has a layout.
#[inline(never)]
fn karis_temporal_blend() -> Rc<FnDef> {
    thread_local! { static CELL: Lazy<Rc<FnDef>> = const { Lazy::new() }; }
    CELL.with(|c| {
        c.get(|| {
            shader_fn(
                Some("karisTemporalBlend"),
                vec![
                    ("denoisedRgb", Type::Vec3),
                    ("denoisedRaw", Type::Vec3),
                    ("a", Type::F32),
                    ("flickerSuppression", Type::F32),
                    ("adaptiveTrust", Type::F32),
                    ("nbhdMeanLuma", Type::F32),
                    ("nbhdStddevLuma", Type::F32),
                ],
                Type::Vec3,
                karis_temporal_blend_body,
            )
        })
    })
}

/// [`karis_temporal_blend`]'s body.
#[inline(never)]
fn karis_temporal_blend_body(args: &[NodeRef]) -> NodeRef {
    let [denoised_rgb, denoised_raw, a, flicker_suppression, adaptive_trust, mean, stddev] = args
    else {
        unreachable!("karisTemporalBlend takes seven arguments")
    };
    let local_cov = stddev.div(mean.max(1e-4));
    let trust_suppress = local_cov
        .mul(adaptive_trust.clone())
        .mul(a.one_minus())
        .clamp(0.0, 0.9);
    let a_trust = a.mul(trust_suppress.one_minus());

    // In flicker-free neighbourhoods, back off the inverse-luminance
    // weighting so valid bright highlights keep their energy.
    let noisy = smoothstep(FLICKER_COV_GATE_MIN, FLICKER_COV_GATE_MAX, local_cov);
    let eff_flicker = flicker_suppression.mul(mix(adaptive_trust.one_minus(), float(1.0), noisy));

    let w_hist = float(1.0).sub(a_trust.clone()).div(
        luminance(denoised_rgb.clone())
            .mul(eff_flicker.clone())
            .mul(10.0)
            .add(1.0),
    );
    let w_raw = a_trust.div(
        luminance(denoised_raw.clone())
            .mul(eff_flicker)
            .mul(10.0)
            .add(1.0),
    );
    denoised_rgb
        .mul(w_hist.clone())
        .add(denoised_raw.mul(w_raw.clone()))
        .div(w_hist.add(w_raw).max(EPSILON))
}

// ---------------------------------------------------------------------------
// `setup()`
// ---------------------------------------------------------------------------

/// What `setup()`'s `Fn`s close over.
struct DenoiseInputs<'a> {
    input: &'a Texture,
    options: &'a RecurrentDenoiseOptions,
    alpha_source: DenoiseAlphaSource,
    u: &'a Uniforms,
}

impl DenoiseInputs<'_> {
    /// `this.rawNode`'s texture — the input when there is none (see the
    /// module docs).
    fn raw_texture(&self) -> &Texture {
        self.options.raw.as_ref().unwrap_or(self.input)
    }

    /// `sampleTexture( uv )` — `texture( this.textureNode, uv ).max( 0 )`.
    fn sample_texture(&self, coord: NodeRef) -> NodeRef {
        texture_uv(self.input, coord).max(0.0)
    }

    /// `sampleRaw( uv )` — `this.rawNode.sample( uv ).max( 0 )`.
    fn sample_raw(&self, coord: NodeRef) -> NodeRef {
        texture_uv(self.raw_texture(), coord).max(0.0)
    }

    /// `sampleDepth( uv )` — `this.depthNode.sample( uv ).x`, or `float( 0.5
    /// )`.
    fn sample_depth(&self, coord: NodeRef) -> NodeRef {
        match &self.options.depth {
            Some(depth) => pass_depth_texture_uv(depth, coord),
            None => float(0.5),
        }
    }

    /// `sampleNormal( uv )` — `unpackRGBToNormal( this.normalNode.sample( uv
    /// ).rgb )`, or the unpacked `vec3( 0, 0, 1 )`.
    fn sample_normal(&self, coord: NodeRef) -> NodeRef {
        let packed = match &self.options.normal {
            Some(normal) => normal(coord).rgb(),
            None => vec3(0.0, 0.0, 1.0),
        };
        unpack_rgb_to_normal(packed)
    }

    /// `sampleRoughnessMetalness( uv )` — `this.roughnessMetalnessNode.sample(
    /// uv ).rg`, or `vec2( 0, 1 )`.
    fn sample_roughness_metalness(&self, coord: NodeRef) -> NodeRef {
        match &self.options.metal_roughness {
            Some(metal_roughness) => metal_roughness(coord),
            None => vec2(0.0, 1.0),
        }
    }

    /// `sampleDiffuse( uv )` — `this.diffuseNode.sample( uv ).rgb`, or `vec3(
    /// 0 )`.
    fn sample_diffuse(&self, coord: NodeRef) -> NodeRef {
        match &self.options.diffuse {
            Some(diffuse) => diffuse(coord).rgb(),
            None => vec3(0.0, 0.0, 0.0),
        }
    }
}

/// `this._material` with `setup()`'s `fragmentNode`.
fn denoise_material(
    input: &Texture,
    options: &RecurrentDenoiseOptions,
    alpha_source: DenoiseAlphaSource,
    uniforms: &Uniforms,
) -> MeshBasicNodeMaterial {
    let inputs = DenoiseInputs {
        input,
        options,
        alpha_source,
        u: uniforms,
    };
    let mut material = MeshBasicNodeMaterial::new();
    material.name = "RecurrentDenoise";
    material.fragment_node = Some(denoise_fn(&inputs, uv()));
    material
}

/// `setup()`'s `noiseRotationMatrix` `Fn()`, which has no layout: a 2D
/// rotation by `r · 2π`.
fn noise_rotation_matrix(r: NodeRef) -> NodeRef {
    let angle = r.mul(2.0).mul(pi());
    mat2_join(vec![
        angle.cos(),
        angle.sin().negate(),
        angle.sin(),
        angle.cos(),
    ])
}

/// `setup()`'s `getNeighborhoodStats( uvCoord, centerSample )` `Fn()`, with
/// its layout: over the centre and its four axis neighbours in the raw
/// input, the inverse-length-weighted mean ray length (environment misses
/// count as 0.25 and set the flag) and the running luma mean and deviation
/// for adaptive trust. It closes over the raw texture, the resolution,
/// `adaptiveTrust` and `alphaSource`, so each material gets its own.
#[inline(never)]
fn get_neighborhood_stats(inputs: &DenoiseInputs) -> Rc<FnDef> {
    let raw = inputs.raw_texture().clone();
    let resolution = inputs.u.resolution.0.clone();
    let adaptive_trust = inputs.u.adaptive_trust.0.clone();
    let ray_length = inputs.alpha_source == DenoiseAlphaSource::RayLength;
    shader_fn(
        Some("getNeighborhoodStats"),
        vec![("uvCoord", Type::Vec2), ("centerSample", Type::Vec4)],
        Type::Vec4,
        move |args| {
            neighborhood_stats_body(
                &raw,
                &resolution,
                &adaptive_trust,
                ray_length,
                &args[0],
                &args[1],
            )
        },
    )
}

/// [`get_neighborhood_stats`]'s body.
#[inline(never)]
fn neighborhood_stats_body(
    raw: &Texture,
    resolution: &NodeRef,
    adaptive_trust: &NodeRef,
    ray_length: bool,
    uv_coord: &NodeRef,
    center_sample: &NodeRef,
) -> NodeRef {
    let rl_sum = to_var(None, float(0.0));
    let rl_sum_w = to_var(None, float(0.0));
    let mean_luma = to_var(None, float(0.0));
    let m2_luma = to_var(None, float(0.0));
    let luma_count = to_var(None, float(0.0));
    let has_env_ray = to_var(None, boolean(false));
    // Three's `toVar()`s are declared where they are first built: the ray
    // length sums only when they are read.
    let mut statements = Vec::new();
    if ray_length {
        statements.extend([rl_sum.clone(), rl_sum_w.clone()]);
    }
    statements.extend([
        mean_luma.clone(),
        m2_luma.clone(),
        luma_count.clone(),
        has_env_ray.clone(),
    ]);

    // A 4-tap cross (the pre-sampled centre and its four axis neighbours).
    let taps: [(f64, f64); 5] = [(0.0, 0.0), (-1.0, 0.0), (1.0, 0.0), (0.0, -1.0), (0.0, 1.0)];
    for (index, (dx, dy)) in taps.into_iter().enumerate() {
        let neighbor = if index == 0 {
            center_sample.clone()
        } else {
            let tap = to_const(
                None,
                texture_uv(raw, uv_coord.add(vec2(dx, dy).div(resolution.clone()))).max(0.0),
            );
            statements.push(tap.clone());
            tap
        };

        if ray_length {
            let sample_rl = to_var(None, neighbor.w());
            let w = float(1.0).div(sample_rl.add(0.001));
            statements.extend([
                sample_rl.clone(),
                if_then(
                    sample_rl.greater_than(ENV_RAY_LENGTH_THRESHOLD),
                    vec![sample_rl.assign(0.25), has_env_ray.assign(boolean(true))],
                ),
                rl_sum.add_assign(sample_rl.mul(w.clone())),
                rl_sum_w.add_assign(w),
            ]);
        }

        let n_luma = luminance(neighbor.rgb());
        let delta = to_const(None, n_luma.sub(mean_luma.clone()));
        statements.push(if_then(
            adaptive_trust.greater_than(0.0),
            vec![
                luma_count.add_assign(1.0),
                delta.clone(),
                mean_luma.add_assign(delta.div(luma_count.clone())),
                m2_luma.add_assign(delta.mul(n_luma.sub(mean_luma.clone()))),
            ],
        ));
    }

    let avg_ray_length = if ray_length {
        rl_sum.div(rl_sum_w)
    } else {
        float(1.0)
    };
    let stddev_luma = sqrt(m2_luma.div(luma_count.max(1.0)));
    block(
        statements,
        vec4_join(vec![
            avg_ray_length,
            mean_luma,
            stddev_luma,
            has_env_ray.to_float(),
        ]),
    )
}

/// `setup()`'s `denoiseFn( uvCoord )` `Fn()`, which has no layout and so
/// inlines into `main()`: discard the background, denoise the rest into
/// the `result` property.
#[inline(never)]
fn denoise_fn(inputs: &DenoiseInputs, uv_coord: NodeRef) -> NodeRef {
    let result = property("denoiseResult", Type::Vec4);
    let depth = to_const(None, inputs.sample_depth(uv_coord.clone()));
    let run_denoise = run_denoise(inputs, &uv_coord, &depth, &result);
    block(
        vec![
            depth.clone(),
            if_else(depth.greater_than_equal(1.0), vec![discard()], run_denoise),
        ],
        result,
    )
}

/// The per-pixel values `runDenoise` computes ahead of the kernel loop and
/// reads inside it.
struct Center {
    view_normal: NodeRef,
    world_normal: NodeRef,
    texel: NodeRef,
    view_position: NodeRef,
    roughness: NodeRef,
    metalness: NodeRef,
    raw: NodeRef,
    frame_num: NodeRef,
    aggressivity: NodeRef,
    has_env_ray: NodeRef,
    nbhd_mean_luma: NodeRef,
    nbhd_stddev_luma: NodeRef,
    tan_half_fov_y: Option<NodeRef>,
    hit_dist_factor: NodeRef,
    denoised: NodeRef,
    total_weight: NodeRef,
    denoised_frame: NodeRef,
    total_frame_weight: NodeRef,
    denoised_raw: NodeRef,
    total_weight_raw: NodeRef,
    mapped_avg_ao: NodeRef,
    tangent: NodeRef,
    bitangent: NodeRef,
    center_diffuse: NodeRef,
    radius_shrink: NodeRef,
    polar_bias: NodeRef,
    lobe_falloff: NodeRef,
}

/// `runDenoise`, the `Else` branch: the statements of the per-pixel set-up,
/// the kernel loop and the resolve.
#[inline(never)]
fn run_denoise(
    inputs: &DenoiseInputs,
    uv_coord: &NodeRef,
    depth: &NodeRef,
    result: &NodeRef,
) -> Vec<NodeRef> {
    let mut statements = Vec::new();
    let center = denoise_center(inputs, uv_coord, depth, &mut statements);
    statements.push(kernel_loop(inputs, uv_coord, &center));
    statements.extend(resolve(inputs, &center, result));
    statements
}

/// `runDenoise` up to the kernel loop.
#[inline(never)]
fn denoise_center(
    inputs: &DenoiseInputs,
    uv_coord: &NodeRef,
    depth: &NodeRef,
    statements: &mut Vec<NodeRef>,
) -> Center {
    let u = inputs.u;
    let specular = inputs.options.mode == DenoiseMode::Specular;
    let ray_length = inputs.alpha_source == DenoiseAlphaSource::RayLength;
    let ao = inputs.alpha_source == DenoiseAlphaSource::Ao;

    let view_normal = to_const(None, inputs.sample_normal(uv_coord.clone()));
    let world_normal = to_const(
        None,
        transform_normal_by_inverse_view_matrix(view_normal.clone(), u.view.0.clone()),
    );
    let texel = to_const(None, inputs.sample_texture(uv_coord.clone()).max(0.0));
    let view_position = to_const(
        None,
        get_view_position(
            uv_coord.clone(),
            depth.clone(),
            u.projection_inverse.0.clone(),
        ),
    );
    let roughness_metalness = to_const(None, inputs.sample_roughness_metalness(uv_coord.clone()));
    let roughness = roughness_metalness.y();
    let metalness = roughness_metalness.x();

    let frame_num = float(1.0).div(texel.w());
    let variance_factor = call(
        &get_temporal_variance_factor(),
        vec![frame_num.clone(), u.strength.0.one_minus()],
    );
    let aggressivity = variance_factor.one_minus();

    let raw = to_const(None, inputs.sample_raw(uv_coord.clone()));

    let view_z = abs(view_position.z());
    let rl = to_var(None, float(1.0));
    let nbhd_mean_luma = to_var(None, float(0.0));
    let nbhd_stddev_luma = to_var(None, float(0.0));
    let has_env_ray = to_var(None, boolean(false));
    statements.extend([
        view_normal.clone(),
        world_normal.clone(),
        texel.clone(),
        view_position.clone(),
        roughness_metalness.clone(),
        raw.clone(),
        rl.clone(),
        nbhd_mean_luma.clone(),
        nbhd_stddev_luma.clone(),
        has_env_ray.clone(),
    ]);

    let stats_fn = get_neighborhood_stats(inputs);
    let stats = call(&stats_fn, vec![uv_coord.clone(), raw.clone()]);
    if ray_length {
        statements.extend([
            rl.assign(stats.x()),
            nbhd_mean_luma.assign(stats.y()),
            nbhd_stddev_luma.assign(stats.z()),
            has_env_ray.assign(stats.w().greater_than(0.5)),
        ]);
    } else {
        statements.push(if_then(
            u.adaptive_trust.0.greater_than(0.0),
            vec![
                nbhd_mean_luma.assign(stats.y()),
                nbhd_stddev_luma.assign(stats.z()),
            ],
        ));
    }

    let tan_half_fov_y = ray_length.then(|| to_const(None, u.fov_y.0.mul(0.5).tan()));
    let hit_dist_factor = match &tan_half_fov_y {
        Some(tan_half_fov_y) => {
            let factor = to_const(
                None,
                call(
                    &compute_hit_dist_factor(),
                    vec![rl.clone(), view_z, tan_half_fov_y.clone()],
                ),
            );
            statements.extend([tan_half_fov_y.clone(), factor.clone()]);
            factor
        }
        None => float(1.0),
    };

    let denoised = to_var(None, texel.rgb());
    let total_weight = to_var(None, float(1.0));
    let denoised_frame = to_var(None, frame_num.clone());
    let total_frame_weight = to_var(None, float(1.0));
    let denoised_raw = to_var(None, raw.rgb());
    let total_weight_raw = to_var(None, float(1.0));
    statements.extend([
        denoised.clone(),
        total_weight.clone(),
        denoised_frame.clone(),
        total_frame_weight.clone(),
        denoised_raw.clone(),
        total_weight_raw.clone(),
        if_then(
            raw.rgb().length().less_than(0.0001),
            vec![
                denoised_raw.assign(vec3(0.0, 0.0, 0.0)),
                total_weight_raw.assign(0.0),
            ],
        ),
    ]);

    let avg_ao = if ao {
        let avg_ao = to_const(None, raw.w());
        statements.push(avg_ao.clone());
        avg_ao
    } else {
        float(1.0)
    };
    let mapped_avg_ao = if ao { map_ao(&avg_ao) } else { float(0.0) };

    let world_radius = to_var(None, u.radius.0.mul(WORLD_RADIUS_SCALE));
    statements.push(world_radius.clone());
    if specular {
        statements.extend([
            world_radius.mul_assign(rl.mul(view_position.z().abs())),
            world_radius.mul_assign(roughness.sqrt().max(0.01)),
        ]);
    } else {
        statements.push(world_radius.mul_assign(avg_ao.pow(2.0).mul(view_position.z().abs())));
    }
    statements.push(world_radius.mul_assign(mix(
        float(1.0),
        float(AGGRESSIVITY_RADIUS_MIN),
        aggressivity.clone(),
    )));

    let (tangent, bitangent) = tangent_frame(
        &view_normal,
        &view_position,
        &roughness,
        specular,
        statements,
    );
    statements.extend([
        tangent.mul_assign(world_radius.clone()),
        bitangent.mul_assign(world_radius),
    ]);

    let center_diffuse = to_const(None, inputs.sample_diffuse(uv_coord.clone()));
    let radius_shrink = to_var(None, float(1.0));
    // Directional analogue of radiusShrink: an accumulated tangent-space
    // shift that skews later taps toward directions that weighed most.
    let polar_bias = to_var(None, vec2(0.0, 0.0));
    // Lobe geometry depends only on per-pixel terms, so its falloff is
    // computed once here.
    let lobe_falloff = to_const(
        None,
        call(
            &lobe_normal_falloff(),
            vec![
                roughness.clone(),
                aggressivity.clone(),
                u.normal_phi.0.one_minus(),
            ],
        ),
    );
    statements.extend([
        center_diffuse.clone(),
        radius_shrink.clone(),
        polar_bias.clone(),
        lobe_falloff.clone(),
    ]);

    Center {
        view_normal,
        world_normal,
        texel,
        view_position,
        roughness,
        metalness,
        raw,
        frame_num,
        aggressivity,
        has_env_ray,
        nbhd_mean_luma,
        nbhd_stddev_luma,
        tan_half_fov_y,
        hit_dist_factor,
        denoised,
        total_weight,
        denoised_frame,
        total_frame_weight,
        denoised_raw,
        total_weight_raw,
        mapped_avg_ao,
        tangent,
        bitangent,
        center_diffuse,
        radius_shrink,
        polar_bias,
        lobe_falloff,
    }
}

/// The kernel's `T` and `B` vars: across the specular lobe's reflection
/// (skewed by roughness at grazing angles) or on the tangent plane.
#[inline(never)]
fn tangent_frame(
    view_normal: &NodeRef,
    view_position: &NodeRef,
    roughness: &NodeRef,
    specular: bool,
    statements: &mut Vec<NodeRef>,
) -> (NodeRef, NodeRef) {
    let t = to_var(None, vec3(0.0, 0.0, 0.0));
    let b = to_var(None, vec3(0.0, 0.0, 0.0));
    statements.extend([t.clone(), b.clone()]);

    if specular {
        let v = view_position.normalize().negate();
        let d = call(
            &get_specular_dominant_direction(),
            vec![view_normal.clone(), v, roughness.clone()],
        );
        let r = reflect(d.negate(), view_normal.clone());
        let tv = cross(view_normal.clone(), r.clone()).normalize();
        let bv = cross(r, tv.clone());
        let view_angle = abs(view_normal.z())
            .acos()
            .div(float(std::f64::consts::FRAC_PI_2))
            .clamp(0.0, 1.0);
        let skew_factor = mix(float(1.0), roughness.clone(), view_angle);
        statements.extend([t.assign(tv.mul(skew_factor)), b.assign(bv)]);
    } else {
        let up = vec3(0.0, 0.0, 1.0);
        let tv = to_var(None, cross(up, view_normal.clone()).normalize());
        statements.extend([
            tv.clone(),
            if_then(
                tv.length().less_than(EPSILON),
                vec![tv.assign(cross(vec3(0.0, 1.0, 0.0), view_normal.clone()).normalize())],
            ),
            t.assign(tv.clone()),
            b.assign(cross(view_normal.clone(), tv).normalize()),
        ]);
    }
    (t, b)
}

/// The eight-tap `Loop()`.
#[inline(never)]
fn kernel_loop(inputs: &DenoiseInputs, uv_coord: &NodeRef, c: &Center) -> NodeRef {
    let u = inputs.u;
    let sample_analytic_noise = bind_analytic_noise(u.resolution.0.clone(), NOISE_ROTATION_SEED);
    let noise_texel = sample_analytic_noise(uv_coord.clone(), u.noise_index.0.clone());
    let rotation_matrix = noise_rotation_matrix(noise_texel.x());
    let depth_weight_scale = u
        .depth_phi
        .0
        .mul(500.0)
        .mul(c.view_normal.z().abs())
        .div(c.view_position.z().abs());

    loop_range("i", int(0), int(KERNEL_SAMPLES), |i| {
        let mut body = Vec::new();
        let base_offset = to_var(None, call(&vogel_disk(), vec![i.to(Type::F32), float(1.0)]));
        let sample_dir = to_const(None, base_offset.normalize());

        // Blend the tap direction toward the polar bias, then restore the
        // Vogel radius and shrink.
        let skewed_dir = mix(
            sample_dir.clone(),
            c.polar_bias.max(EPSILON).normalize(),
            u.adapt.0.mul(c.aggressivity.clone()).mul(
                dot(c.polar_bias.clone(), c.polar_bias.clone())
                    .greater_than(0.001)
                    .select(float(1.0), float(0.0)),
            ),
        );
        let offset = to_var(
            None,
            rotation_matrix.mul(skewed_dir.mul(base_offset.length().mul(c.radius_shrink.clone()))),
        );

        // Exact per-sample view-space projection.
        let sample_view_pos = c
            .view_position
            .add(c.bitangent.mul(offset.x()).add(c.tangent.mul(offset.y())));
        let sample_uv = to_var(
            None,
            get_screen_position(sample_view_pos, u.projection.0.clone()),
        );
        body.extend([
            base_offset.clone(),
            sample_dir.clone(),
            offset.clone(),
            sample_uv.clone(),
            sample_uv.assign(
                sample_uv
                    .abs()
                    .one_minus()
                    .abs()
                    .one_minus()
                    .clamp(0.0, 1.0),
            ),
        ]);

        let tap = kernel_tap(inputs, c, &sample_uv, &mut body);
        let w = to_var(
            None,
            exp(tap
                .kernel_diff
                .mul(c.aggressivity.clone())
                .add(tap.dist_to_plane.mul(depth_weight_scale.clone()))
                .negate())
            .mul(tap.normal_w),
        );
        let neighbor_color = tap.neighbor_color;
        let raw_neighbor_color = tap.raw_neighbor_color;
        let neighbor_a_weight = neighbor_color
            .w()
            .greater_than(c.texel.w())
            .select(w.mul(0.33), float(0.0));
        body.extend([
            w.clone(),
            // Feedback to shrink the radius by the weight.
            c.radius_shrink
                .assign(mix(c.radius_shrink.clone(), w.clone(), u.adapt.0.clone())),
            // Polar feedback: skew later taps toward high-weight directions.
            c.polar_bias
                .assign(mix(c.polar_bias.clone(), sample_dir.mul(w.sub(0.5)), 0.5)),
            // Weigh by inverse luminance for the first five frames, against
            // fireflies in freshly disoccluded regions.
            w.mul_assign(mix(
                float(1.0).div(luminance(raw_neighbor_color.rgb()).pow(2.0).add(0.01)),
                float(1.0),
                c.frame_num.div(5.0).min(1.0),
            )),
            c.denoised_raw
                .add_assign(raw_neighbor_color.rgb().mul(w.clone())),
            c.total_weight_raw.add_assign(w.clone()),
            c.denoised.add_assign(neighbor_color.rgb().mul(w.clone())),
            c.total_weight.add_assign(w),
            // Denoise the alpha (accumulation speed) too, for smoother
            // disocclusion transitions.
            if_then(
                u.smooth_disocclusions.0.clone(),
                vec![
                    c.denoised_frame.add_assign(
                        float(1.0)
                            .div(neighbor_color.w())
                            .mul(neighbor_a_weight.clone()),
                    ),
                    c.total_frame_weight.add_assign(neighbor_a_weight),
                ],
            ),
        ]);
        body
    })
}

/// One tap's samples and edge-stopping terms.
struct Tap {
    neighbor_color: NodeRef,
    raw_neighbor_color: NodeRef,
    kernel_diff: NodeRef,
    dist_to_plane: NodeRef,
    normal_w: NodeRef,
}

/// The loop body from the tap's samples to its edge-stopping terms.
#[inline(never)]
fn kernel_tap(
    inputs: &DenoiseInputs,
    c: &Center,
    sample_uv: &NodeRef,
    body: &mut Vec<NodeRef>,
) -> Tap {
    let u = inputs.u;
    let neighbor_color = to_const(None, inputs.sample_texture(sample_uv.clone()).max(0.0));
    let raw_neighbor_color = to_var(None, inputs.sample_raw(sample_uv.clone()).max(0.0));
    let n_depth = inputs.sample_depth(sample_uv.clone());
    let n_view_position = to_const(
        None,
        get_view_position(sample_uv.clone(), n_depth, u.projection_inverse.0.clone()),
    );
    let n_view_z = to_const(None, abs(n_view_position.z()));
    let kernel_diff = to_var(None, float(0.0));
    body.extend([
        neighbor_color.clone(),
        raw_neighbor_color.clone(),
        n_view_position.clone(),
        n_view_z.clone(),
        kernel_diff.clone(),
        // Luma edge stopping.
        kernel_diff.add_assign(
            luminance(raw_neighbor_color.rgb())
                .sub(luminance(c.raw.rgb()))
                .abs()
                .mul(u.luma_phi.0.clone())
                .mul(10.0),
        ),
    ]);

    // Albedo edge stopping (only with a diffuse G-buffer).
    if inputs.options.diffuse.is_some() {
        body.push(
            kernel_diff.add_assign(
                call(
                    &diffuse_color_distance(),
                    vec![
                        c.center_diffuse.clone(),
                        inputs.sample_diffuse(sample_uv.clone()),
                        float(0.0),
                    ],
                )
                .mul(u.diffuse_phi.0.clone())
                .mul(c.metalness.clone()),
            ),
        );
    }

    match inputs.alpha_source {
        DenoiseAlphaSource::Ao => {
            let neighbor_mapped_ao = map_ao(&raw_neighbor_color.w());
            // Multiplied by aggressivity too, since an early aoW is noise.
            let ao_w = c
                .mapped_avg_ao
                .div(
                    c.mapped_avg_ao
                        .add(neighbor_mapped_ao)
                        .add(AO_EDGE_STOPPING_BIAS),
                )
                .mul(u.alpha_phi.0.clone())
                .mul(c.aggressivity.clone());
            body.push(kernel_diff.add_assign(ao_w));
        }
        DenoiseAlphaSource::RayLength => {
            let tan_half_fov_y = c
                .tan_half_fov_y
                .clone()
                .expect("three-rs: ray-length stopping hoists tanHalfFovY");
            let neighbor_hit_dist_factor = call(
                &compute_hit_dist_factor(),
                vec![raw_neighbor_color.w(), n_view_z, tan_half_fov_y],
            );
            let hdf_diff = c.hit_dist_factor.sub(neighbor_hit_dist_factor).abs();
            let ray_length_factor = hdf_diff
                .mul(u.alpha_phi.0.clone())
                .div(c.view_position.z().abs());
            // An environment ray is accepted when the neighbourhood has one.
            body.push(
                kernel_diff.add_assign(
                    raw_neighbor_color
                        .w()
                        .greater_than(ENV_RAY_LENGTH_THRESHOLD)
                        .and(c.has_env_ray.clone())
                        .select(float(1.0), ray_length_factor),
                ),
            );
        }
        DenoiseAlphaSource::None => {}
    }

    // Roughness edge stopping.
    if inputs.options.mode == DenoiseMode::Specular {
        body.push(
            kernel_diff.add_assign(
                abs(c
                    .roughness
                    .sub(inputs.sample_roughness_metalness(sample_uv.clone()).y()))
                .mul(u.roughness_phi.0.clone()),
            ),
        );
    }

    let n_view_normal = inputs.sample_normal(sample_uv.clone());
    let n_world_normal = transform_normal_by_inverse_view_matrix(n_view_normal, u.view.0.clone());
    let dist_to_plane = call(
        &plane_distance(),
        vec![
            c.view_position.clone(),
            n_view_position,
            c.view_normal.clone(),
        ],
    );
    let normal_w = call(
        &lobe_normal_weight(),
        vec![
            c.world_normal.clone(),
            n_world_normal,
            c.lobe_falloff.clone(),
        ],
    );
    Tap {
        neighbor_color,
        raw_neighbor_color,
        kernel_diff,
        dist_to_plane,
        normal_w,
    }
}

/// After the loop: normalise, then the temporal blend into `result`.
#[inline(never)]
fn resolve(inputs: &DenoiseInputs, c: &Center, result: &NodeRef) -> Vec<NodeRef> {
    let u = inputs.u;
    let mut statements = vec![
        c.denoised.div_assign(c.total_weight.max(EPSILON)),
        c.denoised.assign(c.denoised.max(EPSILON)),
        c.denoised_raw.div_assign(c.total_weight_raw.max(EPSILON)),
    ];

    if inputs.options.accumulate {
        let computed_frame = c.denoised_frame.div(c.total_frame_weight.max(EPSILON));
        let a = to_const(None, float(1.0).div(computed_frame.max(EPSILON)));
        statements.push(a.clone());
        let rgb = if inputs.options.raw.is_some() {
            call(
                &karis_temporal_blend(),
                vec![
                    c.denoised.clone(),
                    c.denoised_raw.clone(),
                    a.clone(),
                    u.flicker_suppression.0.clone(),
                    u.adaptive_trust.0.clone(),
                    c.nbhd_mean_luma.clone(),
                    c.nbhd_stddev_luma.clone(),
                ],
            )
        } else {
            mix(c.denoised.clone(), c.denoised_raw.clone(), a.clone())
        };
        statements.push(result.assign(vec4_join(vec![rgb, a])));
    } else {
        statements.push(result.assign(vec4_join(vec![c.denoised.clone(), c.texel.w()])));
    }
    statements
}
