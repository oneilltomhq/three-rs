//! Port of `three.js/examples/jsm/tsl/display/SSRNode.js` — screen space
//! reflections, in both of three's forms:
//!
//! - `stochastic = false` (first generation): one mirror ray per pixel,
//!   marched through the depth buffer in screen space, with roughness faked
//!   by a blurred mip chain;
//! - `stochastic = true` (second generation): one GGX-sampled ray per pixel,
//!   jittered by analytic noise whose index advances every frame, weighted
//!   by the sampled lobe, and falling back to an equirectangular environment
//!   ([`ImportanceSampledEnvironment`]) where it misses. The result is noisy
//!   and expects a denoiser downstream; there is no blur chain.
//!
//! Three's node is a `Node` with `updateBeforeType = FRAME`, and so is this
//! one ([`SsrState`] implements [`NodeUpdate`] and is registered as the
//! updater of both its textures). Every frame `updateBefore()` draws:
//!
//! 1. `SSRNode.SSR` into the half-float SSR target: the reflection colour,
//!    weighted (by metalness, attenuation and a Fresnel term, or by the GGX
//!    sample's weight), with the hit's world-space distance in alpha;
//! 2. without `stochastic` and with a roughness node, `SSRNode.Copy` into
//!    mip 0 of the blur target, unblurred;
//! 3. and then `SSRNode.Blur` into mips 1–4, a [`box_blur`](super::box_blur)
//!    of the SSR target whose tap spacing is the mip index.
//!
//! [`SsrNode::node`] then reads the blur target at a level picked by the
//! surface's roughness (`r² · 4`), which is `ssrPass` on
//! `webgpu_postprocessing_ssr`, or the SSR target itself when there is no
//! blur.
//!
//! **Compile-time switches.** `binaryRefine`, `stepExponent`,
//! `reflectNonMetals`, `screenEdgeFadeBlack`, the environment and the
//! history are baked into the SSR material, and three's setters rebuild it
//! (`_buildSSRMaterial()`). The port builds the material in [`SsrNode::new`]
//! rather than in a lazy `setup()`, so [`SsrNode::set_history`] rebuilds too.
//!
//! **Order within a frame.** Three's `setup()` builds the colour, depth and
//! normal nodes before itself, so the scene pass is earlier in the frame's
//! update-before list. The port asks for the pass explicitly at the top of
//! `update_before()`, as [`TraaNode`](super::TraaNode) does; the frame guard
//! makes the pass's own later reach a no-op.
//!
//! Not ported: `resolutionScale` other than 1, an orthographic camera and a
//! logarithmic depth buffer.

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use crate::cameras::{PerspectiveCamera, RenderCamera};
use crate::materials::MeshBasicNodeMaterial;
use crate::math::{Color, Matrix4};
use crate::nodes::frame::register_texture_update;
use crate::nodes::node::{FnDef, Lazy, SettableValue, Type};
use crate::nodes::tsl::{
    bind_analytic_noise, block, boolean, break_loop, call, continue_loop, cross, discard, distance,
    dot, float, get_screen_position, get_specular_dominant_factor, get_view_position,
    ggx_reflection_sample, ggx_reflection_struct, if_else, if_then, int, length, loop_range,
    luminance, max, mix, pass_depth_texture_uv, perspective_depth_to_view_z, reflect, shader_fn,
    struct_get, texture, texture_level, texture_sample, texture_uv, to_const, to_var,
    uniform_settable, uv, vec2, vec2_join, vec3, vec4, vec4_join, ENV_RAY_LENGTH,
};
use crate::nodes::{NodeRef, NodeUpdate, NodeUpdateType};
use crate::objects::QuadMesh;
use crate::renderer::{RenderTarget, RenderTargetOptions, Renderer};
use crate::textures::{DepthTexture, MinFilter, Texture, TextureFilter, TextureType};

use super::box_blur::{box_blur_with, BoxBlurOptions};
use super::importance_sampled_environment::{EnvironmentLobe, ImportanceSampledEnvironment};

/// `blurRenderTarget.texture.mipmaps.push( {}, {}, {}, {}, {} )`: the blur
/// chain's levels, the first an unblurred copy.
const BLUR_MIPS: u32 = 5;

/// `MAX_STEPS`: the stochastic march's step count at `quality = 1`.
const MAX_STEPS: f64 = 64.0;

/// A normal source: `normalNode.sample( uv )` for any uv, the way three's
/// `sample( ( uv ) => unpackRGBToNormal( … ) )` node answers both its own
/// default-uv read and the march's `normalNode.sample( uvS )`.
pub type SampleFn = Rc<dyn Fn(NodeRef) -> NodeRef>;

/// `SSRNodeOptions`. [`SsrOptions::new`] is `{ metalnessNode, roughnessNode }`
/// with every other option at three's default; the `with_*` methods set the
/// rest.
#[derive(Clone)]
#[non_exhaustive]
pub struct SsrOptions {
    /// `options.metalnessNode` — per-pixel metalness. Without `stochastic`
    /// and `reflect_non_metals`, a pixel at or below zero is discarded.
    pub metalness: NodeRef,
    /// `options.roughnessNode` — per-pixel roughness, which picks the blur
    /// level (or, with `stochastic`, shapes the GGX lobe). `None` is three's
    /// `null`: no blur passes, and [`SsrNode::node`] is the raw SSR target.
    /// The stochastic path needs one.
    pub roughness: Option<NodeRef>,
    /// `options.stochastic` — GGX-sampled rays instead of one mirror ray and
    /// a blur chain. False by default.
    pub stochastic: bool,
    /// `options.reflectNonMetals` — without `stochastic`, also reflect
    /// surfaces whose metalness is zero. False by default.
    pub reflect_non_metals: bool,
    /// `options.environmentNode` — an equirectangular HDR texture with CPU
    /// pixel data, which [`SsrNode::set_env_map`] is called with. The
    /// stochastic path's fallback where a ray misses.
    pub environment: Option<Texture>,
    /// `options.envImportanceSampling` — estimate a miss with multiple
    /// importance sampling against the environment's luminance CDF rather
    /// than the BRDF-sampled ray alone. False by default.
    pub env_importance_sampling: bool,
    /// `options.diffuseNode` — the albedo the GGX sample's metal Fresnel is
    /// tinted by, read at the pixel's uv. `None` is white.
    pub diffuse: Option<SampleFn>,
    /// `options.binaryRefine` — bisect a hit between the last two march
    /// steps, eight times. False by default.
    pub binary_refine: bool,
}

impl SsrOptions {
    /// `{ metalnessNode, roughnessNode }`.
    pub fn new(metalness: NodeRef, roughness: Option<NodeRef>) -> Self {
        Self {
            metalness,
            roughness,
            stochastic: false,
            reflect_non_metals: false,
            environment: None,
            env_importance_sampling: false,
            diffuse: None,
            binary_refine: false,
        }
    }

    /// `{ stochastic }`.
    pub fn with_stochastic(mut self, stochastic: bool) -> Self {
        self.stochastic = stochastic;
        self
    }

    /// `{ reflectNonMetals }`.
    pub fn with_reflect_non_metals(mut self, reflect_non_metals: bool) -> Self {
        self.reflect_non_metals = reflect_non_metals;
        self
    }

    /// `{ environmentNode }` — an equirectangular HDR texture.
    pub fn with_environment(mut self, hdr: &Texture) -> Self {
        self.environment = Some(hdr.clone());
        self
    }

    /// `{ envImportanceSampling }`.
    pub fn with_env_importance_sampling(mut self, importance_sampling: bool) -> Self {
        self.env_importance_sampling = importance_sampling;
        self
    }

    /// `{ diffuseNode }` — the albedo at a uv.
    pub fn with_diffuse(mut self, diffuse: SampleFn) -> Self {
        self.diffuse = Some(diffuse);
        self
    }

    /// `{ binaryRefine }`.
    pub fn with_binary_refine(mut self, binary_refine: bool) -> Self {
        self.binary_refine = binary_refine;
        self
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
    mirror_bias: SettableValue,
    screen_edge_fade: SettableValue,
    environment_intensity: SettableValue,
}

/// The camera uniforms `updateBefore()` (and, in three, the uniforms'
/// object references) keep current, and the per-frame noise index.
struct CameraUniforms {
    projection: SettableValue,
    projection_inverse: SettableValue,
    world: SettableValue,
    /// `_cameraWorldPosition`. Kept current as three's is, though no part of
    /// the graph reads it.
    world_position: SettableValue,
    near: SettableValue,
    far: SettableValue,
    resolution: SettableValue,
    /// `_noiseIndex`.
    noise_index: SettableValue,
}

/// The uniform nodes the SSR material reads, kept so a rebuild reads the
/// same uniforms.
#[derive(Clone)]
struct UniformNodes {
    max_distance: NodeRef,
    thickness: NodeRef,
    intensity: NodeRef,
    max_luminance: NodeRef,
    quality: NodeRef,
    mirror_bias: NodeRef,
    screen_edge_fade: NodeRef,
    environment_intensity: NodeRef,
    projection: NodeRef,
    projection_inverse: NodeRef,
    world: NodeRef,
    near: NodeRef,
    far: NodeRef,
    resolution: NodeRef,
    noise_index: NodeRef,
}

/// The constructor's options and the compile-time properties: what
/// `_buildSSRMaterial()` reads.
struct Switches {
    stochastic: bool,
    env_importance_sampling: bool,
    reflect_non_metals: Cell<bool>,
    binary_refine: Cell<bool>,
    step_exponent: Cell<f64>,
    screen_edge_fade_black: Cell<bool>,
}

/// The node's state, shared with the renderer's update-before registry.
pub(crate) struct SsrState {
    /// `this.colorNode`'s texture, whose pass renders the scene.
    color: Texture,
    /// `this.depthNode`'s texture.
    depth: DepthTexture,
    /// `this.normalNode`.
    normal: SampleFn,
    /// `this.metalnessNode`.
    metalness: NodeRef,
    /// `this.roughnessNode`.
    roughness: Option<NodeRef>,
    /// `this.diffuseNode`.
    diffuse: Option<SampleFn>,
    /// `this.camera`.
    camera: Rc<RefCell<PerspectiveCamera>>,
    /// `this._ssrRenderTarget`.
    ssr_target: RenderTarget,
    /// `this._blurRenderTarget`, with [`BLUR_MIPS`] levels.
    blur_target: RenderTarget,
    /// `_quadMesh` with `this._ssrMaterial`, rebuilt by the compile-time
    /// setters.
    ssr_quad: RefCell<QuadMesh>,
    /// `_quadMesh` with `this._copyMaterial`.
    copy_quad: QuadMesh,
    /// `_quadMesh` with `this._blurMaterial`. Rebuilt by
    /// [`SsrNode::set_blur_quality`], as three's `_buildBlurMaterial()` does.
    blur_quad: RefCell<QuadMesh>,
    /// `this._blurQuality`.
    blur_quality: Cell<u32>,
    /// `this._blurSpread`.
    blur_spread: (NodeRef, SettableValue),
    /// `this.stochastic === false && this.roughnessNode !== null` — whether
    /// the blur passes run.
    blurred: bool,
    params: Params,
    camera_uniforms: CameraUniforms,
    uniforms: UniformNodes,
    switches: Switches,
    /// `this.historyTexture` and `this.velocityTexture`.
    history: RefCell<Option<(Texture, Texture)>>,
    /// `this._importanceEnvironment`.
    environment: RefCell<Option<ImportanceSampledEnvironment>>,
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
        let (mirror_bias, mirror_bias_value) = uniform_settable(Type::F32, vec![0.5]);
        let (screen_edge_fade, screen_edge_fade_value) = uniform_settable(Type::F32, vec![0.2]);
        let (environment_intensity, environment_intensity_value) =
            uniform_settable(Type::F32, vec![std::f64::consts::PI]);

        let identity = mat4_values(&Matrix4::identity());
        let (projection, projection_value) = uniform_settable(Type::Mat4, identity.clone());
        let (projection_inverse, projection_inverse_value) =
            uniform_settable(Type::Mat4, identity.clone());
        let (world, world_value) = uniform_settable(Type::Mat4, identity);
        let (_, world_position_value) = uniform_settable(Type::Vec3, vec![0.0, 0.0, 0.0]);
        let (near, near_value) = uniform_settable(Type::F32, vec![camera.borrow().near]);
        let (far, far_value) = uniform_settable(Type::F32, vec![camera.borrow().far]);
        let (resolution, resolution_value) = uniform_settable(Type::Vec2, vec![0.0, 0.0]);
        let (noise_index, noise_index_value) = uniform_settable(Type::F32, vec![0.0]);

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
        // the SSR target when there is no blur.
        let blurred = !options.stochastic && options.roughness.is_some();
        let blur_texture = blur_target.texture();
        let node = match &options.roughness {
            Some(r) if blurred => {
                let mips = (BLUR_MIPS - 1) as f64;
                let lod = r.mul(r.clone()).mul(mips).clamp(0.0, mips);
                texture_level(&blur_texture, uv(), lod)
            }
            _ => texture_uv(&ssr_texture, uv()),
        };

        let state = Rc::new(SsrState {
            color: color.clone(),
            depth: depth.clone(),
            normal,
            metalness: options.metalness,
            roughness: options.roughness,
            diffuse: options.diffuse,
            camera,
            ssr_target,
            blur_target,
            ssr_quad: RefCell::new(QuadMesh::new(MeshBasicNodeMaterial::new())),
            copy_quad: QuadMesh::new(copy_material),
            blur_quad: RefCell::new(QuadMesh::new(blur_material)),
            blur_quality: Cell::new(blur_quality),
            blur_spread,
            blurred,
            params: Params {
                max_distance: max_distance_value,
                thickness: thickness_value,
                intensity: intensity_value,
                max_luminance: max_luminance_value,
                quality: quality_value,
                mirror_bias: mirror_bias_value,
                screen_edge_fade: screen_edge_fade_value,
                environment_intensity: environment_intensity_value,
            },
            camera_uniforms: CameraUniforms {
                projection: projection_value,
                projection_inverse: projection_inverse_value,
                world: world_value,
                world_position: world_position_value,
                near: near_value,
                far: far_value,
                resolution: resolution_value,
                noise_index: noise_index_value,
            },
            uniforms: UniformNodes {
                max_distance,
                thickness,
                intensity,
                max_luminance,
                quality,
                mirror_bias,
                screen_edge_fade,
                environment_intensity,
                projection,
                projection_inverse,
                world,
                near,
                far,
                resolution,
                noise_index,
            },
            switches: Switches {
                stochastic: options.stochastic,
                env_importance_sampling: options.env_importance_sampling,
                reflect_non_metals: Cell::new(options.reflect_non_metals),
                binary_refine: Cell::new(options.binary_refine),
                step_exponent: Cell::new(2.0),
                screen_edge_fade_black: Cell::new(false),
            },
            history: RefCell::new(None),
            environment: RefCell::new(None),
            node,
        });
        // `if ( environmentNode !== null && environmentNode.isTexture )
        // this.setEnvMap( environmentNode )`, then `setup()`'s
        // `_buildSSRMaterial()`.
        if let Some(hdr) = &options.environment {
            state.update_env_map(Some(hdr));
        }
        state.rebuild();
        // `passTexture( this, … )` for both textures: whichever one a
        // material samples, this node fills it first.
        register_texture_update(state.ssr_target.texture().id(), &state);
        register_texture_update(state.blur_target.texture().id(), &state);
        Self(state)
    }

    /// `ssrNode.getTextureNode()` — what the page calls `ssrPass`: the blur
    /// chain at a roughness-picked level, or with `stochastic` (or without a
    /// roughness node) the raw SSR target.
    pub fn node(&self) -> NodeRef {
        self.0.node.clone()
    }

    /// `ssrNode.getRenderTarget()` — the SSR target, the reflection colour
    /// with the hit distance in alpha.
    pub fn render_target(&self) -> RenderTarget {
        self.0.ssr_target.clone()
    }

    /// `ssrNode.stochastic`.
    pub fn stochastic(&self) -> bool {
        self.0.switches.stochastic
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
    /// screen-space pixels that are tested, or with `stochastic` the fraction
    /// of 64 steps taken. 0.5 by default.
    pub fn quality(&self) -> &SettableValue {
        &self.0.params.quality
    }

    /// `ssrNode.mirrorBias.value`, in [0, 1] — with `stochastic`, how far the
    /// GGX samples are pulled toward the mirror direction. 0.5 by default.
    pub fn mirror_bias(&self) -> &SettableValue {
        &self.0.params.mirror_bias
    }

    /// `ssrNode.screenEdgeFade.value` — with `stochastic`, the width (in uv)
    /// of the band at the screen's edges where a hit fades out. 0.2 by
    /// default.
    pub fn screen_edge_fade(&self) -> &SettableValue {
        &self.0.params.screen_edge_fade
    }

    /// `ssrNode.environmentIntensity.value` — with `stochastic`, the scale on
    /// the environment fallback. π by default.
    pub fn environment_intensity(&self) -> &SettableValue {
        &self.0.params.environment_intensity
    }

    /// `ssrNode.envMapIntensity` — the environment's own intensity uniform,
    /// once [`set_env_map`](Self::set_env_map) has given it one.
    pub fn env_map_intensity(&self) -> Option<SettableValue> {
        self.0
            .environment
            .borrow()
            .as_ref()
            .map(|environment| environment.intensity().clone())
    }

    /// `ssrNode.setEnvMap( hdr )` — the equirectangular HDR environment
    /// (with CPU pixel data) the stochastic path falls back to where a ray
    /// misses, or `None` for none. Rebuilds the SSR material.
    pub fn set_env_map(&self, hdr: Option<&Texture>) {
        self.0.update_env_map(hdr);
        self.0.rebuild();
    }

    /// `ssrNode.setHistory( history, velocity )` — multi-bounce: the
    /// previous frame's (denoised) reflections, reprojected through the
    /// velocity texture and added at each hit. A node with a render target
    /// passes its target's texture. Three's material reads these when it is
    /// first built; the port's is already built, so this rebuilds it.
    pub fn set_history(&self, history: &Texture, velocity: &Texture) {
        *self.0.history.borrow_mut() = Some((history.clone(), velocity.clone()));
        self.0.rebuild();
    }

    /// `ssrNode.setHistory( null, null )` — no multi-bounce. Rebuilds the
    /// SSR material.
    pub fn clear_history(&self) {
        *self.0.history.borrow_mut() = None;
        self.0.rebuild();
    }

    /// `ssrNode.stepExponent` — with `stochastic`, the exponent of the
    /// march's `( i / steps ) ^ e` step spacing. 2 by default.
    pub fn step_exponent(&self) -> f64 {
        self.0.switches.step_exponent.get()
    }

    /// `ssrNode.stepExponent = value`: baked into the shader, so a change
    /// rebuilds the SSR material.
    pub fn set_step_exponent(&self, value: f64) {
        if value != self.0.switches.step_exponent.get() {
            self.0.switches.step_exponent.set(value);
            self.0.rebuild();
        }
    }

    /// `ssrNode.binaryRefine`.
    pub fn binary_refine(&self) -> bool {
        self.0.switches.binary_refine.get()
    }

    /// `ssrNode.binaryRefine = value`: baked into the shader, so a change
    /// rebuilds the SSR material.
    pub fn set_binary_refine(&self, value: bool) {
        if value != self.0.switches.binary_refine.get() {
            self.0.switches.binary_refine.set(value);
            self.0.rebuild();
        }
    }

    /// `ssrNode.reflectNonMetals`.
    pub fn reflect_non_metals(&self) -> bool {
        self.0.switches.reflect_non_metals.get()
    }

    /// `ssrNode.reflectNonMetals = value`: baked into the shader, so a
    /// change rebuilds the SSR material.
    pub fn set_reflect_non_metals(&self, value: bool) {
        if value != self.0.switches.reflect_non_metals.get() {
            self.0.switches.reflect_non_metals.set(value);
            self.0.rebuild();
        }
    }

    /// `ssrNode.screenEdgeFadeBlack` — with `stochastic`, whether hits and
    /// misses near the screen edge fade to black rather than toward the
    /// environment. False by default.
    pub fn screen_edge_fade_black(&self) -> bool {
        self.0.switches.screen_edge_fade_black.get()
    }

    /// `ssrNode.screenEdgeFadeBlack = value`: baked into the shader, so a
    /// change rebuilds the SSR material.
    pub fn set_screen_edge_fade_black(&self, value: bool) {
        if value != self.0.switches.screen_edge_fade_black.get() {
            self.0.switches.screen_edge_fade_black.set(value);
            self.0.rebuild();
        }
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

    /// The `SSRNode.SSR` quad material, as of the last rebuild, for
    /// `examples/dump_wgsl.rs` and the dump gate.
    #[doc(hidden)]
    pub fn quad_material(&self) -> MeshBasicNodeMaterial {
        self.0.ssr_quad.borrow().material.clone()
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

    /// `setEnvMap( hdr )` without the rebuild: drop the environment, or make
    /// one (`new ImportanceSampledEnvironment( envImportanceSampling )`) and
    /// `updateFrom( hdr )`.
    fn update_env_map(&self, hdr: Option<&Texture>) {
        let mut environment = self.environment.borrow_mut();
        match hdr {
            None => {
                if let Some(mut old) = environment.take() {
                    old.clear();
                }
            }
            Some(hdr) => environment
                .get_or_insert_with(|| {
                    ImportanceSampledEnvironment::new(self.switches.env_importance_sampling)
                })
                .update_from(hdr),
        }
    }

    /// `_buildSSRMaterial()`: the `ssr` `Fn()` as of the current switches,
    /// environment and history, into the SSR quad's material.
    fn rebuild(&self) {
        let environment = self.environment.borrow();
        let history = self.history.borrow().clone();
        let switches = &self.switches;
        let fragment = ssr_fragment(&SsrInputs {
            color: &self.color,
            depth: &self.depth,
            normal: &self.normal,
            metalness: self.metalness.clone(),
            roughness: self.roughness.clone(),
            diffuse: self.diffuse.as_ref(),
            u: &self.uniforms,
            stochastic: switches.stochastic,
            reflect_non_metals: switches.reflect_non_metals.get(),
            binary_refine: switches.binary_refine.get(),
            step_exponent: switches.step_exponent.get(),
            screen_edge_fade_black: switches.screen_edge_fade_black.get(),
            env_importance_sampling: switches.env_importance_sampling,
            environment: environment.as_ref(),
            history,
        });
        let mut material = MeshBasicNodeMaterial::new();
        material.name = "SSRNode.SSR";
        material.fragment_node = Some(fragment);
        self.ssr_quad.borrow_mut().material = material;
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
            // `_cameraWorldMatrix.value.copy( camera.matrixWorld )`,
            // `_cameraWorldPosition.value.copy( camera.position )`, and the
            // values three's object-referencing uniforms read at render time.
            let camera = self.camera.borrow();
            let u = &self.camera_uniforms;
            u.world
                .set(mat4_values(&RenderCamera::matrix_world(&*camera)));
            let position = camera.node.borrow().position;
            u.world_position
                .set(vec![position.x, position.y, position.z]);
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

        // `this._noiseIndex.value = ( this._noiseIndex.value + 1 ) % 0x7fffffff`.
        let noise_index = self.camera_uniforms.noise_index.get()[0];
        self.camera_uniforms
            .noise_index
            .set(vec![(noise_index + 1.0) % 2_147_483_647.0]);

        // clear
        renderer.set_mrt(None);
        renderer.set_clear_color(Color::new(0.0, 0.0, 0.0), 0.0);

        // ssr
        renderer.set_render_target(Some(self.ssr_target.clone()));
        renderer.render_quad(&self.ssr_quad.borrow());

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

/// What the `ssr` `Fn()` closes over: the node's inputs, uniforms and
/// compile-time switches as of a `_buildSSRMaterial()`.
struct SsrInputs<'a> {
    color: &'a Texture,
    depth: &'a DepthTexture,
    normal: &'a SampleFn,
    metalness: NodeRef,
    roughness: Option<NodeRef>,
    diffuse: Option<&'a SampleFn>,
    u: &'a UniformNodes,
    stochastic: bool,
    reflect_non_metals: bool,
    binary_refine: bool,
    step_exponent: f64,
    screen_edge_fade_black: bool,
    env_importance_sampling: bool,
    environment: Option<&'a ImportanceSampledEnvironment>,
    history: Option<(Texture, Texture)>,
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

/// `computeScreenBorderFactor( uvCoord, borderWidth )` — 1 inside the
/// screen, easing to 0 over `borderWidth` (in uv) at the nearest edge. A
/// `Fn` with a layout, so a real `fn` in the module.
fn compute_screen_border_factor(uv_coord: NodeRef, border_width: NodeRef) -> NodeRef {
    thread_local! { static CELL: Lazy<Rc<FnDef>> = const { Lazy::new() }; }
    let def = CELL.with(|cell| {
        cell.get(|| {
            shader_fn(
                Some("computeScreenBorderFactor"),
                vec![("uvCoord", Type::Vec2), ("borderWidth", Type::F32)],
                Type::F32,
                |args| {
                    let (uv_coord, border_width) = (&args[0], &args[1]);
                    let border = border_width.max(1e-4);
                    // Distance to the nearest screen edge — uniform falloff
                    // at corners.
                    let edge_dist = uv_coord
                        .x()
                        .min(float(1.0).sub(uv_coord.x()))
                        .min(uv_coord.y().min(float(1.0).sub(uv_coord.y())));
                    // Two smoothsteps for a softer ease-in-out than one ramp.
                    let t = edge_dist.smoothstep(float(0.0), border);
                    t.smoothstep(float(0.0), float(1.0)).pow(0.125)
                },
            )
        })
    });
    call(&def, vec![uv_coord, border_width])
}

/// The pixel being shaded: what the `ssr` `Fn()` reads before it marches.
struct Surface {
    uv_pos: NodeRef,
    view_position: NodeRef,
    world_position: NodeRef,
    view_normal: NodeRef,
    view_incident_dir: NodeRef,
    /// `float( this.metalnessNode )`.
    metalness: NodeRef,
    /// `float( this.roughnessNode )`, stochastic only.
    roughness: Option<NodeRef>,
    /// `sampleMarchNoise( uvNode, this._noiseIndex )`, stochastic only.
    noise: Option<NodeRef>,
    /// `this.screenEdgeFade.mul( glossiness )`, stochastic only.
    hit_border_width: Option<NodeRef>,
}

/// The ray to march and what weights its hit.
struct Reflection {
    view_reflect_dir: NodeRef,
    final_sample_weight: NodeRef,
    spec_dominant_factor: NodeRef,
    /// The stochastic path's `V` and `ggxSample` var, which the environment
    /// lookup reads.
    stochastic: Option<(NodeRef, NodeRef)>,
}

/// The march's state: the screen-space ray and the vars that carry a hit
/// out of the loop.
struct Ray {
    d0: NodeRef,
    d1_view_position: NodeRef,
    step_vec: NodeRef,
    inv_resolution: NodeRef,
    uv_pixel_step_x: NodeRef,
    total_step: NodeRef,
    total_step_f: NodeRef,
    ray_len: NodeRef,
    recip_vpz: NodeRef,
    recip_d1vpz: NodeRef,
    output: NodeRef,
    hit: NodeRef,
    found_hit: NodeRef,
    hit_s_lo: NodeRef,
    hit_s_hi: NodeRef,
    hit_uv_s: NodeRef,
    hit_d: NodeRef,
}

impl Ray {
    /// `reflectRayZAt( s )` for a perspective camera: the reflected ray's
    /// view-space z at `s`, linear in `1 / z`.
    fn reflect_ray_z_at(&self, s: &NodeRef) -> NodeRef {
        float(1.0).div(
            self.recip_vpz
                .add(s.mul(self.recip_d1vpz.sub(self.recip_vpz.clone()))),
        )
    }

    /// `screenPosAt( s )`: the ray's screen position (pixels) at `s`.
    fn screen_pos_at(&self, s: &NodeRef) -> NodeRef {
        self.d0
            .add(self.step_vec.mul(s.mul(self.total_step_f.clone())))
    }

    /// `sampleFraction( idx )`: the ray parameter of step `idx`, uniform
    /// without `stochastic`, and with it `( idx / steps ) ^ stepExponent`,
    /// jittered by the noise and floored to a texel a step. `idx` is read
    /// twice there, so three's builder gives it a `let`.
    fn sample_fraction(
        &self,
        inputs: &SsrInputs,
        noise: Option<&NodeRef>,
        idx: NodeRef,
    ) -> NodeRef {
        match noise {
            Some(noise) if inputs.stochastic => {
                let idx = to_const(None, idx);
                max(
                    idx.add(noise.z().sub(0.5))
                        .div(self.total_step_f.clone())
                        .pow(inputs.step_exponent),
                    idx.div(self.ray_len.clone()),
                )
            }
            _ => idx.div(self.total_step_f.clone()),
        }
    }
}

/// `SSRNode.setup()`'s `ssr` `Fn()` — the `SSRNode.SSR` material's
/// `fragmentNode` — for a perspective camera, as of the compile-time
/// switches in `inputs`.
#[inline(never)]
fn ssr_fragment(inputs: &SsrInputs) -> NodeRef {
    let mut statements = Vec::new();
    let surface = ssr_surface(inputs, &mut statements);
    let reflection = ssr_reflection(inputs, &surface, &mut statements);
    let ray = ssr_ray(inputs, &surface, &reflection, &mut statements);
    statements.push(ssr_march(inputs, &surface, &reflection, &ray));
    statements.push(ssr_shade_hit(inputs, &surface, &reflection, &ray));

    // Screen-space ray missed: environment fallback (MIS when CDF env is
    // set up), stochastic only.
    let output = ray.output.clone();
    let mut miss = Vec::new();
    if inputs.stochastic {
        let env = sample_env_reflection(inputs, &surface, &reflection);
        miss.push(output.assign(vec4_join(vec![
            env.mul(inputs.u.environment_intensity.clone()),
            float(ENV_RAY_LENGTH),
        ])));
        // Misses fade by the surface pixel uv (where the reflection is being
        // shaded).
        if inputs.screen_edge_fade_black {
            miss.push(output.rgb().mul_assign(compute_screen_border_factor(
                surface.uv_pos.clone(),
                inputs.u.screen_edge_fade.clone(),
            )));
        }
    }
    statements.push(if_then(ray.hit.equal(0.0), miss));

    let lum = to_var(None, luminance(output.rgb()).max(1e-4));
    statements.push(lum.clone());
    statements.push(
        output
            .rgb()
            .mul_assign(inputs.u.max_luminance.div(lum).min(1.0)),
    );
    // Scale the reflection colour by the user-controlled intensity.
    statements.push(output.rgb().mul_assign(inputs.u.intensity.clone()));

    block(statements, output.max(0.0))
}

/// The `ssr` `Fn()` up to the reflected ray: the pixel's depth (background
/// discarded), position, normal and incident direction, its metalness
/// (non-metals discarded without `stochastic` or `reflectNonMetals`), and on
/// the stochastic path the noise and the roughness-scaled edge-fade width.
#[inline(never)]
fn ssr_surface(inputs: &SsrInputs, statements: &mut Vec<NodeRef>) -> Surface {
    let u = inputs.u;
    let noise = inputs
        .stochastic
        .then(|| bind_analytic_noise(u.resolution.clone(), 47)(uv(), u.noise_index.clone()));

    let uv_pos = to_var(None, uv());
    let depth = to_var(None, pass_depth_texture_uv(inputs.depth, uv_pos.clone()));
    statements.push(uv_pos.clone());
    statements.push(depth.clone());
    // Skip background pixels (cleared far-plane depth); the target is
    // cleared each frame.
    statements.push(if_then(depth.greater_than_equal(1.0), vec![discard()]));

    let view_position = to_var(
        None,
        get_view_position(uv_pos.clone(), depth, u.projection_inverse.clone()),
    );
    let world_position = to_var(
        None,
        u.world
            .mul(vec4_join(vec![view_position.clone(), float(1.0)]))
            .xyz(),
    );
    let view_normal = to_var(None, (inputs.normal)(uv()).rgb().normalize());
    let view_incident_dir = to_var(None, view_position.normalize());
    statements.extend([
        view_position.clone(),
        world_position.clone(),
        view_normal.clone(),
        view_incident_dir.clone(),
    ]);

    // `float( this.metalnessNode )`: read twice (both a discard and a
    // weight, or both GGX samples) everywhere but the non-metal mirror path,
    // so three's builder gives it a `let` there.
    let metalness = if inputs.stochastic {
        to_const(None, inputs.metalness.clone())
    } else if inputs.reflect_non_metals {
        inputs.metalness.clone()
    } else {
        let metalness = to_const(None, inputs.metalness.clone());
        statements.push(if_then(metalness.less_than_equal(0.0), vec![discard()]));
        metalness
    };

    // `float( this.roughnessNode )`, read by both GGX samples, the specular
    // dominant factor and the edge fade, so three's builder gives it a
    // `let`. Only the stochastic path reads it.
    let roughness = inputs.stochastic.then(|| {
        to_const(
            None,
            inputs
                .roughness
                .clone()
                .expect("three-rs: SSR's stochastic path needs a roughness node"),
        )
    });
    let hit_border_width = roughness.as_ref().map(|roughness| {
        let glossiness = roughness.div(0.25).min(1.0).one_minus();
        u.screen_edge_fade.mul(glossiness)
    });

    Surface {
        uv_pos,
        view_position,
        world_position,
        view_normal,
        view_incident_dir,
        metalness,
        roughness,
        noise,
        hit_border_width,
    }
}

/// The reflected ray and its weight: the mirror direction and `vec3(
/// metalness )`, or on the stochastic path a GGX sample (re-drawn once if it
/// points into the surface), its weight and the specular dominant factor.
#[inline(never)]
fn ssr_reflection(
    inputs: &SsrInputs,
    surface: &Surface,
    statements: &mut Vec<NodeRef>,
) -> Reflection {
    let (Some(noise), Some(roughness)) = (&surface.noise, &surface.roughness) else {
        let view_reflect_dir = to_var(
            None,
            reflect(
                surface.view_incident_dir.clone(),
                surface.view_normal.clone(),
            )
            .normalize(),
        );
        statements.push(view_reflect_dir.clone());
        return Reflection {
            view_reflect_dir,
            final_sample_weight: surface.metalness.to(Type::Vec3),
            spec_dominant_factor: float(1.0),
            stochastic: None,
        };
    };
    let u = inputs.u;
    let n = &surface.view_normal;
    let metalness = &surface.metalness;
    let layout = ggx_reflection_struct();

    let v = to_var(None, surface.view_incident_dir.negate().normalize());
    let albedo = to_var(None, vec3(1.0, 1.0, 1.0));
    let xi = to_var(None, noise.clone());
    statements.extend([v.clone(), albedo.clone(), xi.clone()]);
    // Mirror bias: pull `Xi.y` toward the cap top to tighten the GGX lobe.
    statements.push(
        xi.y()
            .assign(mix(xi.y(), float(0.0), u.mirror_bias.mul(xi.w().sqrt()))),
    );
    statements.push(albedo.assign(match inputs.diffuse {
        Some(diffuse) => diffuse(surface.uv_pos.clone()).rgb(),
        None => vec3(1.0, 1.0, 1.0),
    }));
    let sample = |xi: NodeRef| {
        ggx_reflection_sample(
            n.clone(),
            v.clone(),
            roughness.clone(),
            metalness.clone(),
            albedo.clone(),
            xi,
        )
    };
    let ggx = to_var(None, sample(xi.clone()));
    statements.push(ggx.clone());
    // Sometimes the GGX sample faces away from the surface: re-sample.
    statements.push(if_then(
        dot(struct_get(&ggx, &layout, "reflectDir"), n.clone()).less_than(0.0),
        vec![ggx.assign(sample(to_const(None, xi.add(xi.mul(7.0)).fract())))],
    ));

    let view_reflect_dir = to_var(None, struct_get(&ggx, &layout, "reflectDir"));
    let final_sample_weight = to_var(None, struct_get(&ggx, &layout, "sampleWeight"));
    let spec_dominant_factor = to_var(
        None,
        get_specular_dominant_factor(struct_get(&ggx, &layout, "NdotV"), roughness.clone()),
    );
    statements.extend([
        view_reflect_dir.clone(),
        final_sample_weight.clone(),
        spec_dominant_factor.clone(),
    ]);
    Reflection {
        view_reflect_dir,
        final_sample_weight,
        spec_dominant_factor,
        stochastic: Some((v, ggx)),
    }
}

/// `sampleEnvReflection()`: the environment along the GGX-sampled ray
/// (BRDF-sampled, or with `envImportanceSampling` MIS against the
/// environment's luminance CDF), or black without an environment. Each call
/// builds its own nodes, as each of three's calls does.
#[inline(never)]
fn sample_env_reflection(
    inputs: &SsrInputs,
    surface: &Surface,
    reflection: &Reflection,
) -> NodeRef {
    let (Some(environment), Some((v, ggx))) = (inputs.environment, &reflection.stochastic) else {
        return vec3(0.0, 0.0, 0.0);
    };
    let u = inputs.u;
    let layout = ggx_reflection_struct();
    let lobe = EnvironmentLobe {
        camera_world_matrix: u.world.clone(),
        view_reflect_dir: reflection.view_reflect_dir.clone(),
        n: surface.view_normal.clone(),
        v: v.clone(),
        alpha: struct_get(ggx, &layout, "alpha"),
        f0: struct_get(ggx, &layout, "f0"),
    };
    let env_color = to_var(None, vec3(0.0, 0.0, 0.0));
    let value = if inputs.env_importance_sampling {
        let xi2 = bind_analytic_noise(u.resolution.clone(), 59)(
            surface.uv_pos.clone(),
            u.noise_index.clone(),
        );
        environment.sample_environment_mis(&lobe, xi2)
    } else {
        environment.sample_environment_brdf(&lobe)
    };
    block(vec![env_color.clone(), env_color.assign(value)], env_color)
}

/// The screen-space ray from the pixel to `maxDistance` along the reflected
/// direction (clipped to the near plane), its step count and spacing, and
/// the vars the march leaves its hit in.
#[inline(never)]
fn ssr_ray(
    inputs: &SsrInputs,
    surface: &Surface,
    reflection: &Reflection,
    statements: &mut Vec<NodeRef>,
) -> Ray {
    let u = inputs.u;
    let near = &u.near;
    let resolution = &u.resolution;
    let view_position = &surface.view_position;
    let view_reflect_dir = &reflection.view_reflect_dir;

    let max_reflect_ray_len = to_var(
        None,
        u.max_distance.div(dot(
            surface.view_incident_dir.negate(),
            surface.view_normal.clone(),
        )),
    );
    let d1_view_position = to_var(
        None,
        view_position.add(view_reflect_dir.mul(max_reflect_ray_len.clone())),
    );
    statements.extend([max_reflect_ray_len, d1_view_position.clone()]);
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

    let d0 = to_var(None, surface.uv_pos.mul(resolution.clone()).xy());
    let d1 = to_var(
        None,
        get_screen_position(d1_view_position.clone(), u.projection.clone()).mul(resolution.clone()),
    );
    let x_len = to_var(None, d1.x().sub(d0.x()));
    let y_len = to_var(None, d1.y().sub(d0.y()));
    // The dominant-axis ray length in texels, for the stochastic per-step
    // floor.
    let ray_len = to_var(None, max(x_len.abs(), y_len.abs()).max(1.0));
    // The mirror ray spends steps in proportion to its screen-space length;
    // the stochastic one a fixed, bounded count.
    let total_step = if inputs.stochastic {
        to_const(
            None,
            u.quality
                .clamp(0.0, 1.0)
                .mul(MAX_STEPS)
                .max(float(1.0))
                .to(Type::I32),
        )
    } else {
        to_const(
            None,
            max(
                max(x_len.abs(), y_len.abs())
                    .mul(u.quality.clamp(0.0, 1.0))
                    .trunc()
                    .to(Type::I32),
                int(1),
            ),
        )
    };
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
    let hit_s_lo = to_var(None, float(0.0));
    let hit_s_hi = to_var(None, float(0.0));
    let hit_uv_s = to_var(None, vec2(0.0, 0.0));
    let hit_d = to_var(None, float(0.0));
    statements.extend([d0.clone(), d1, x_len, y_len]);
    // Three's unread vars are not declared.
    if inputs.stochastic {
        statements.push(ray_len.clone());
    }
    statements.extend([
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
    ]);
    if inputs.binary_refine {
        statements.extend([hit_s_lo.clone(), hit_s_hi.clone()]);
    }
    statements.extend([hit_uv_s.clone(), hit_d.clone()]);

    Ray {
        d0,
        d1_view_position,
        step_vec,
        inv_resolution,
        uv_pixel_step_x,
        total_step,
        total_step_f,
        ray_len,
        recip_vpz,
        recip_d1vpz,
        output,
        hit,
        found_hit,
        hit_s_lo,
        hit_s_hi,
        hit_uv_s,
        hit_d,
    }
}

/// The march: `Loop( { start: 1, end: totalStep } )` from `d0` toward `d1`,
/// stopping at the first depth crossing within the surface thickness (and,
/// without `stochastic`, facing the ray and within `maxDistance`), which it
/// leaves in `foundHit`, `hitUvS`, `hitD` and, with `binaryRefine`, the
/// bracket `hitSLo`/`hitSHi`.
#[inline(never)]
fn ssr_march(inputs: &SsrInputs, surface: &Surface, reflection: &Reflection, ray: &Ray) -> NodeRef {
    let u = inputs.u;
    let resolution = &u.resolution;
    let projection_inverse = &u.projection_inverse;
    let noise = surface.noise.as_ref();
    let sample_depth = |coord: NodeRef| pass_depth_texture_uv(inputs.depth, coord);

    loop_range("i", int(1), ray.total_step.clone(), |i| {
        let s = to_var(None, ray.sample_fraction(inputs, noise, i.to(Type::F32)));
        let xy = to_var(None, ray.screen_pos_at(&s));
        let off_screen = xy
            .x()
            .less_than(0.0)
            .or(xy.x().greater_than(resolution.x()))
            .or(xy.y().less_than(0.0))
            .or(xy.y().greater_than(resolution.y()));
        let uv_s = to_var(None, xy.mul(ray.inv_resolution.clone()));
        let d = to_var(None, sample_depth(uv_s.clone()));
        let v_z = to_var(
            None,
            perspective_depth_to_view_z(d.clone(), u.near.clone(), u.far.clone()),
        );
        let view_reflect_ray_z = to_var(None, ray.reflect_ray_z_at(&s));

        let v_p = to_var(
            None,
            get_view_position(uv_s.clone(), d.clone(), projection_inverse.clone()),
        );
        let away = to_var(
            None,
            point_to_line_distance(&v_p, &surface.view_position, &ray.d1_view_position),
        );
        let uv_neighbor = to_var(None, uv_s.add(ray.uv_pixel_step_x.clone()));
        let v_p_neighbor = to_var(
            None,
            get_view_position(uv_neighbor.clone(), d.clone(), projection_inverse.clone()),
        );
        let min_thickness = to_var(None, v_p_neighbor.x().sub(v_p.x()).mul(3.0));
        let tk = to_var(None, max(min_thickness.clone(), u.thickness.clone()));

        let v_n = to_var(None, (inputs.normal)(uv_s.clone()).rgb().normalize());
        let mut found = vec![v_n.clone()];
        if !inputs.stochastic {
            // The reflected ray points the same way as the surface it
            // reached, so it cannot reflect off it: try the next step.
            let plane_distance = to_var(
                None,
                point_plane_distance(&v_p, &surface.view_position, &surface.view_normal),
            );
            found.extend([
                if_then(
                    dot(reflection.view_reflect_dir.clone(), v_n).greater_than_equal(0.0),
                    vec![continue_loop()],
                ),
                plane_distance.clone(),
                if_then(
                    plane_distance.greater_than(u.max_distance.clone()),
                    vec![break_loop()],
                ),
            ]);
        }
        found.extend([
            ray.found_hit.assign(boolean(true)),
            ray.hit_uv_s.assign(uv_s.clone()),
            ray.hit_d.assign(d.clone()),
        ]);
        if inputs.binary_refine {
            found.extend([
                ray.hit_s_lo
                    .assign(ray.sample_fraction(inputs, noise, i.to(Type::F32).sub(1.0))),
                ray.hit_s_hi.assign(s.clone()),
            ]);
        }
        found.push(break_loop());

        vec![
            s,
            xy.clone(),
            if_then(off_screen, vec![break_loop()]),
            uv_s,
            d,
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
                    if_then(away.less_than_equal(tk), found),
                ],
            ),
        ]
    })
}

/// `If( foundHit, … )`: with `binaryRefine`, eight bisections of the
/// bracketed crossing; then the hit's colour (plus the reprojected history,
/// faded at the screen edge on the stochastic path), weighted, with its
/// world-space distance in alpha.
#[inline(never)]
fn ssr_shade_hit(
    inputs: &SsrInputs,
    surface: &Surface,
    reflection: &Reflection,
    ray: &Ray,
) -> NodeRef {
    let u = inputs.u;
    let max_distance = &u.max_distance;
    let sample_depth = |coord: NodeRef| pass_depth_texture_uv(inputs.depth, coord);
    let hit_uv_s = &ray.hit_uv_s;
    let mut body = Vec::new();

    if inputs.binary_refine {
        let refine = loop_range("i", int(0), int(8), |_| {
            let s_mid = to_var(None, ray.hit_s_lo.add(ray.hit_s_hi.clone()).mul(0.5));
            let scene_z_mid = perspective_depth_to_view_z(
                sample_depth(ray.screen_pos_at(&s_mid).mul(ray.inv_resolution.clone())),
                u.near.clone(),
                u.far.clone(),
            );
            vec![
                s_mid.clone(),
                if_else(
                    ray.reflect_ray_z_at(&s_mid).less_than_equal(scene_z_mid),
                    vec![ray.hit_s_hi.assign(s_mid.clone())],
                    vec![ray.hit_s_lo.assign(s_mid)],
                ),
            ]
        });
        body.extend([
            refine,
            // Refinement moved the crossing: re-fetch uv and depth.
            hit_uv_s.assign(
                ray.screen_pos_at(&ray.hit_s_hi)
                    .mul(ray.inv_resolution.clone()),
            ),
            ray.hit_d.assign(sample_depth(hit_uv_s.clone())),
        ]);
    }

    let v_p = to_var(
        None,
        get_view_position(
            hit_uv_s.clone(),
            ray.hit_d.clone(),
            u.projection_inverse.clone(),
        ),
    );
    body.push(v_p.clone());
    // The mirror path's ratio² falloff re-grows past `maxDistance`, so its
    // over-range hits are dropped; the stochastic path bounds its reach by
    // the ray length, so every hit shades.
    let distance_point_plane = if inputs.stochastic {
        float(0.0)
    } else {
        let d = to_var(
            None,
            point_plane_distance(&v_p, &surface.view_position, &surface.view_normal),
        );
        body.push(d.clone());
        d
    };

    let hit_world_position = to_var(
        None,
        u.world.mul(vec4_join(vec![v_p.clone(), float(1.0)])).xyz(),
    );
    let world_distance = to_var(
        None,
        distance(surface.world_position.clone(), hit_world_position.clone())
            .mul(reflection.spec_dominant_factor.clone()),
    );
    let reflect_color = to_var(None, texture_uv(inputs.color, hit_uv_s.clone()));
    let mut within = vec![
        hit_world_position,
        world_distance.clone(),
        reflect_color.clone(),
        // Multi-bounce: the reprojected previous-frame reflection at the
        // hit, or with no history the colour assigned back to itself.
        reflect_color.rgb().assign(reproject_hit_point_history(
            inputs,
            hit_uv_s,
            reflect_color.rgb(),
        )),
    ];
    if inputs.stochastic {
        within.extend(apply_hit_edge_fade(
            inputs,
            surface,
            reflection,
            &reflect_color,
            hit_uv_s,
        ));
    }
    let mut weighted_color = reflect_color
        .rgb()
        .mul(reflection.final_sample_weight.clone());
    if !inputs.stochastic {
        // The mirror path is a plain reflection: upstream's squared distance
        // attenuation and grazing Fresnel.
        let ratio = to_var(
            None,
            float(1.0).sub(distance_point_plane.div(max_distance.clone())),
        );
        let attenuation = to_var(None, ratio.mul(ratio.clone()));
        let fresnel_coe = to_var(
            None,
            dot(
                surface.view_incident_dir.clone(),
                reflection.view_reflect_dir.clone(),
            )
            .add(1.0)
            .div(2.0),
        );
        weighted_color = weighted_color.mul(attenuation.mul(fresnel_coe.clone()));
        within.extend([ratio, attenuation, fresnel_coe]);
    }
    within.extend([
        ray.hit.assign(1.0),
        ray.output
            .assign(vec4_join(vec![weighted_color, world_distance])),
    ]);
    body.push(if_then(
        distance_point_plane.less_than_equal(max_distance.clone()),
        within,
    ));

    if_then(ray.found_hit.clone(), body)
}

/// `reprojectHitPointHistory( uvHit, color )`: `color` plus the history at
/// the hit, reprojected by the velocity there and damped by its alpha; just
/// `color` until both textures are set.
fn reproject_hit_point_history(inputs: &SsrInputs, uv_hit: &NodeRef, color: NodeRef) -> NodeRef {
    let Some((history, velocity)) = &inputs.history else {
        return color;
    };
    let velocity = texture_uv(velocity, uv_hit.clone()).xy();
    let history_bounce = to_var(None, texture_uv(history, uv_hit.sub(velocity)));
    let sample_decay = history_bounce.a().one_minus();
    color.add(history_bounce.rgb().mul(sample_decay))
}

/// `applyHitEdgeFade( reflectColor, uvS, hitBorderWidth )`: fade a hit
/// near the screen border to black (`screenEdgeFadeBlack`), or toward the
/// environment reflection.
fn apply_hit_edge_fade(
    inputs: &SsrInputs,
    surface: &Surface,
    reflection: &Reflection,
    reflect_color: &NodeRef,
    uv_s: &NodeRef,
) -> Vec<NodeRef> {
    let u = inputs.u;
    if inputs.screen_edge_fade_black {
        let factor = compute_screen_border_factor(uv_s.clone(), u.screen_edge_fade.clone());
        return vec![reflect_color.rgb().mul_assign(factor)];
    }
    let border_width = surface
        .hit_border_width
        .clone()
        .expect("three-rs: the stochastic path has a hit border width");
    let factor = to_const(
        None,
        compute_screen_border_factor(uv_s.clone(), border_width),
    );
    let env = sample_env_reflection(inputs, surface, reflection);
    vec![
        factor.clone(),
        if_then(
            factor.less_than(1.0),
            vec![reflect_color.rgb().assign(mix(
                env.mul(u.environment_intensity.clone()),
                reflect_color.rgb(),
                factor,
            ))],
        ),
    ]
}
