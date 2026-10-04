//! Port of `three.js/examples/jsm/tsl/display/TemporalReprojectNode.js` —
//! temporal reprojection for denoising screen-space effects (SSGI, SSR).
//!
//! Every frame a resolve quad reprojects a history texture along the scene's
//! `velocity` attachment with a geometrically weighted 4-tap bilinear fetch
//! (each tap trusted by how well the previous frame's depth and normal agree
//! with the current surface), clips it to the YCoCg variance box of the
//! current 3×3 neighbourhood, and writes the clipped history with
//! `1 / frameCount` in alpha — the blend weight a downstream accumulating pass
//! (`RecurrentDenoiseNode` on `webgpu_postprocessing_ssr_denoise`) gives the
//! current frame. The `Specular` mode also reprojects the reflection's
//! parallax hit point (the beauty's alpha carries the SSR ray length) and
//! blends the two histories.
//!
//! Unlike [`TraaNode`](super::TraaNode) the node does not jitter the camera.
//! It still claims the pipeline's view offset the way three's `setup()` does
//! (`renderPipelineState.viewOffsetOwner`): before the pipeline renders it
//! refreshes the camera's projection matrix and hands it to the velocity
//! attachment, after it clears it. [`TemporalReprojectNode::attach`] installs
//! that, as `TraaNode::attach` does.
//!
//! Three's node is a `Node` with `updateBeforeType = FRAME`; so is this one
//! (its state implements [`NodeUpdate`] and is registered as the updater of
//! the resolve texture, the way `passTexture( this, … )` makes it one in
//! three). As in `traa.rs`, `update_before()` asks for the scene pass first,
//! so the beauty, normal and depth it reads are this frame's.
//!
//! **The history binding.** Three keeps one `texture()` node for the history
//! and swaps its `value` between the internal history target and an external
//! texture (`setHistoryTexture()`, used with `accumulate: false`), going back
//! to the internal one for the frame after a resize. A texture node here binds
//! its texture when the graph is built, so the port builds the resolve quad
//! once per history source and picks the quad where three would swap the
//! value (`_syncHistoryTextureBinding`).
//!
//! **Previous depth and normal.** The previous depth is the history target's
//! `DepthTexture` from the start (three starts from a 1×1 placeholder and
//! points at it after the first copy; both read as an unusable depth that
//! first frame), and the previous normal is a target of the normal's format,
//! reallocated on every restart as three re-clones `normalNode.value`. Both
//! are filled by copies at the bottom of `update_before()`, when the normal
//! attachment matches the drawing buffer — three's guard is that the depth
//! has an image, which on its pages implies the same.
//!
//! Not ported: an orthographic camera, a logarithmic or reversed depth
//! buffer, a beauty node other than a texture (three's `convertToTexture()`
//! of an arbitrary node — pass [`convert_to_texture`](super::convert_to_texture)'s
//! texture), and a `RecurrentDenoiseNode` as the history source (pass its
//! texture).

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use crate::cameras::{PerspectiveCamera, RenderCamera};
use crate::materials::MeshBasicNodeMaterial;
use crate::math::Matrix4;
use crate::nodes::frame::register_texture_update;
use crate::nodes::node::{FnDef, SettableValue, StructLayout, StructMember, TextureSource, Type};
use crate::nodes::tsl::{
    abs, block, call, depth_texture_load, depth_texture_load_transformed, discard, dot, dpdx, dpdy,
    epsilon, exp, float, floor, fwidth, get_view_position, if_then, int, ivec2, length, luminance,
    max, mix, screen_coordinate, shader_fn, smoothstep, sqrt, struct_get, struct_new, struct_type,
    texture_load, texture_load_transformed, texture_size, texture_uv, to_const, to_var,
    to_var_intent, uniform_settable, unpack_rgb_to_normal, uv, vec2, vec3, vec3_join, vec4_join,
    ENV_RAY_LENGTH, ENV_RAY_LENGTH_THRESHOLD,
};
use crate::nodes::{NodeRef, NodeUpdate, NodeUpdateType};
use crate::objects::QuadMesh;
use crate::renderer::{RenderPipeline, RenderTarget, RenderTargetOptions, Renderer};
use crate::textures::{DepthTexture, Texture, TextureFilter, TextureType};

/// `VARIANCE_CLIP_LUMA_SCALE`: how hard bright samples are compressed before
/// they enter the variance box.
const VARIANCE_CLIP_LUMA_SCALE: f64 = 10.0;
/// `DEFAULT_MAX_VELOCITY_LENGTH` — `this.maxVelocityLength`, in pixels: the
/// motion at which the motion factor saturates. A plain number in three too,
/// baked into the shader when it is built.
const MAX_VELOCITY_LENGTH: f64 = 128.0;
/// `VARIANCE_GAMMA_MIN`.
const VARIANCE_GAMMA_MIN: f64 = 0.5;
/// `VARIANCE_GAMMA_MAX`.
const VARIANCE_GAMMA_MAX: f64 = 1.0;

fn mat4_values(m: &Matrix4) -> Vec<f64> {
    m.elements.to_vec()
}

/// `TemporalReprojectMode` — what the beauty input holds.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TemporalReprojectMode {
    /// `'diffuse'`: SSGI or scene colour. Surface-velocity reprojection only.
    #[default]
    Diffuse,
    /// `'specular'`: SSR reflections, with the ray length in the beauty's
    /// alpha. Adds the parallax hit-point reprojection.
    Specular,
}

/// `TemporalReprojectNodeOptions`, the constructor's `options`.
#[derive(Clone, Copy, Debug, Default)]
pub struct TemporalReprojectOptions {
    /// `mode`, `'diffuse'` by default.
    pub mode: TemporalReprojectMode,
    /// `hitPointReprojection`: the starting value of
    /// [`TemporalReprojectNode::hit_point_reprojection`]. `None` is three's
    /// default, `mode === 'specular'`.
    pub hit_point_reprojection: Option<bool>,
    /// `accumulate`, `false` by default. When `true` the resolve is copied
    /// into the node's own history each frame (a classic temporal resolve);
    /// when `false` the history comes from
    /// [`TemporalReprojectNode::set_history_texture`] (a denoiser's output).
    pub accumulate: bool,
}

/// `temporalReproject( beautyNode, depthNode, normalNode, velocityNode,
/// camera, options )`.
///
/// The inputs are the textures three's `convertToTexture()` / the pass's
/// `getTextureNode()` would hand over: the beauty (on
/// `webgpu_postprocessing_ssr_denoise` the SSR output, with the ray length in
/// alpha), the scene pass's depth texture, its packed-normal attachment
/// (`packNormalToRGB( normalView )` in `rgb`) and its `velocity` attachment.
/// The camera must be the pass's.
pub fn temporal_reproject(
    beauty: &Texture,
    depth: &DepthTexture,
    normal: &Texture,
    velocity: &Texture,
    camera: Rc<RefCell<PerspectiveCamera>>,
    options: TemporalReprojectOptions,
) -> TemporalReprojectNode {
    TemporalReprojectNode::new(beauty, depth, normal, velocity, camera, options)
}

/// `TemporalReprojectNode` — see the module docs.
pub struct TemporalReprojectNode(Rc<TemporalReprojectState>);

/// One `uniform()` of three's: the graph's node and the value behind it.
struct Uniform {
    node: NodeRef,
    value: SettableValue,
}

impl Uniform {
    fn new(ty: Type, values: Vec<f64>) -> Self {
        let (node, value) = uniform_settable(ty, values);
        Self { node, value }
    }

    fn mat4(m: &Matrix4) -> Self {
        Self::new(Type::Mat4, mat4_values(m))
    }
}

/// `bindTemporalCameraUniforms( camera )`: the current and previous camera
/// matrices.
struct CameraUniforms {
    world: Uniform,
    view: Uniform,
    projection: Uniform,
    projection_inverse: Uniform,
    world_position: Uniform,
    previous_world: Uniform,
    previous_view: Uniform,
    previous_projection: Uniform,
    previous_projection_inverse: Uniform,
}

impl CameraUniforms {
    /// Every uniform starts as a copy of the camera, current and previous.
    fn new(camera: &PerspectiveCamera) -> Self {
        let world = RenderCamera::matrix_world(camera);
        let position = camera.node.borrow().position;
        Self {
            world: Uniform::mat4(&world),
            view: Uniform::mat4(&camera.matrix_world_inverse),
            projection: Uniform::mat4(&camera.projection_matrix),
            projection_inverse: Uniform::mat4(&camera.projection_matrix_inverse),
            world_position: Uniform::new(Type::Vec3, vec![position.x, position.y, position.z]),
            previous_world: Uniform::mat4(&world),
            previous_view: Uniform::mat4(&camera.matrix_world_inverse),
            previous_projection: Uniform::mat4(&camera.projection_matrix),
            previous_projection_inverse: Uniform::mat4(&camera.projection_matrix_inverse),
        }
    }

    /// `updateFromCamera( cam )`: roll the current values into the previous
    /// ones, then read the camera. `worldPosition` is `cam.position`, as in
    /// three.
    fn update_from_camera(&self, camera: &PerspectiveCamera) {
        self.previous_world.value.set(self.world.value.get());
        self.previous_view.value.set(self.view.value.get());
        self.previous_projection
            .value
            .set(self.projection.value.get());
        self.previous_projection_inverse
            .value
            .set(self.projection_inverse.value.get());

        self.world
            .value
            .set(mat4_values(&RenderCamera::matrix_world(camera)));
        self.view
            .value
            .set(mat4_values(&camera.matrix_world_inverse));
        self.projection
            .value
            .set(mat4_values(&camera.projection_matrix));
        self.projection_inverse
            .value
            .set(mat4_values(&camera.projection_matrix_inverse));
        let position = camera.node.borrow().position;
        self.world_position
            .value
            .set(vec![position.x, position.y, position.z]);
    }
}

/// The node's state, shared with the renderer's update-before registry.
pub(crate) struct TemporalReprojectState {
    /// `this.beautyNode`'s texture.
    beauty: Texture,
    /// `this.depthNode.value`.
    depth: DepthTexture,
    /// `this.normalNode.value`.
    normal: Texture,
    /// `this.velocityNode.value`, kept to build the external-history quad.
    velocity: Texture,
    /// `this.camera`.
    camera: Rc<RefCell<PerspectiveCamera>>,
    /// `this.mode`.
    mode: TemporalReprojectMode,
    /// `this.accumulate`.
    accumulate: bool,
    /// `this._historyRenderTarget`, with its `DepthTexture` (the previous
    /// depth, filled by a copy).
    history: RenderTarget,
    /// `this._resolveRenderTarget`.
    resolve: RenderTarget,
    /// `this._previousNormalTexture`, as a target so it can be allocated and
    /// copied into.
    previous_normal: RenderTarget,
    /// `_quadMesh` with `this._seedMaterial`.
    seed_quad: QuadMesh,
    /// `_quadMesh` with `this._resolveMaterial`, reading the internal
    /// history.
    internal_quad: QuadMesh,
    /// The same resolve reading `this._externalHistoryTexture`, built by
    /// `set_history_texture()`.
    external_quad: RefCell<Option<QuadMesh>>,
    /// `this._resolution`.
    resolution: Uniform,
    /// `this._cameraUniforms`.
    camera_uniforms: CameraUniforms,
    /// `this.maxFrames`.
    max_frames: Uniform,
    /// `this.hitPointReprojection`.
    hit_point_reprojection: Uniform,
    /// `this.clampIntensity`.
    clamp_intensity: Uniform,
    /// `this.flickerSuppression`.
    flicker_suppression: Uniform,
    /// `this._textureNode` — `passTexture( this, resolve.texture )`.
    node: NodeRef,
    /// Whether `attach()` installed the view-offset hooks.
    attached: Cell<bool>,
}

impl TemporalReprojectNode {
    /// `new TemporalReprojectNode( beautyNode, depthNode, normalNode,
    /// velocityNode, camera, options )`, with the materials `setup()` gives
    /// it.
    pub fn new(
        beauty: &Texture,
        depth: &DepthTexture,
        normal: &Texture,
        velocity: &Texture,
        camera: Rc<RefCell<PerspectiveCamera>>,
        options: TemporalReprojectOptions,
    ) -> Self {
        let mode = options.mode;
        let hit_point_reprojection = options
            .hit_point_reprojection
            .unwrap_or(mode == TemporalReprojectMode::Specular);

        let half_float = || RenderTargetOptions {
            texture_type: TextureType::HalfFloat,
            samples: 0,
            depth_buffer: false,
            min_filter: TextureFilter::Linear,
            mag_filter: TextureFilter::Linear,
        };
        // `new RenderTarget( 1, 1, { depthBuffer: false, type: HalfFloatType,
        // depthTexture: new DepthTexture() } )`.
        let history = RenderTarget::new_with_options(1, 1, half_float())
            .expect("three-rs: the history target is a colour type");
        history.set_depth_texture(DepthTexture::new());
        history.set_copy_destination();
        let resolve = RenderTarget::new_with_options(1, 1, half_float())
            .expect("three-rs: the resolve target is a colour type");
        // `normalNode.value.clone()`: the format is re-read from the normal
        // at every restart, when the attachment has its final one.
        let previous_normal = RenderTarget::new_with_options(
            1,
            1,
            RenderTargetOptions {
                texture_type: TextureType::UnsignedByte,
                ..half_float()
            },
        )
        .expect("three-rs: the previous-normal target is a colour type");
        previous_normal.texture().set_format(normal.format());
        previous_normal.set_copy_destination();
        depth.set_copyable();
        history
            .depth_texture()
            .expect("three-rs: set just above")
            .set_copyable();

        let camera_uniforms = CameraUniforms::new(&camera.borrow());
        let resolution = Uniform::new(Type::Vec2, vec![0.0, 0.0]);
        let max_frames = Uniform::new(Type::F32, vec![32.0]);
        let hit_point_reprojection = Uniform::new(
            Type::Bool,
            vec![if hit_point_reprojection { 1.0 } else { 0.0 }],
        );
        let clamp_intensity = Uniform::new(Type::F32, vec![1.0]);
        let flicker_suppression = Uniform::new(Type::F32, vec![1.0]);

        let seed_quad = QuadMesh::new(seed_material(beauty, &resolution.node));
        let node = to_var(None, texture_uv(&resolve.texture(), uv()));

        let mut state = TemporalReprojectState {
            beauty: beauty.clone(),
            depth: depth.clone(),
            normal: normal.clone(),
            velocity: velocity.clone(),
            camera,
            mode,
            accumulate: options.accumulate,
            history,
            resolve,
            previous_normal,
            seed_quad,
            internal_quad: QuadMesh::new(MeshBasicNodeMaterial::new()),
            external_quad: RefCell::new(None),
            resolution,
            camera_uniforms,
            max_frames,
            hit_point_reprojection,
            clamp_intensity,
            flicker_suppression,
            node,
            attached: Cell::new(false),
        };
        let history_texture = state.history.texture();
        state.internal_quad = QuadMesh::new(state.resolve_material(&history_texture));

        let state = Rc::new(state);
        register_texture_update(state.resolve.texture().id(), &state);
        Self(state)
    }

    /// `temporalReprojectNode.getTextureNode()` — the resolved history, for
    /// the graph downstream.
    pub fn node(&self) -> NodeRef {
        self.0.node.clone()
    }

    /// `this._resolveRenderTarget.texture` — what [`node`](Self::node)
    /// samples, for a downstream pass that reads the texture itself.
    pub fn texture(&self) -> Texture {
        self.0.resolve.texture()
    }

    /// `temporalReprojectNode.maxFrames` — the most frames the history may
    /// stand for; the output alpha never drops below `1 / maxFrames`. 32 by
    /// default.
    pub fn max_frames(&self) -> &SettableValue {
        &self.0.max_frames.value
    }

    /// `temporalReprojectNode.hitPointReprojection` — whether `Specular` mode
    /// trusts the parallax hit-point history (`1.0`) or only the surface one
    /// (`0.0`). A `bool` uniform; ignored in `Diffuse` mode.
    pub fn hit_point_reprojection(&self) -> &SettableValue {
        &self.0.hit_point_reprojection.value
    }

    /// `temporalReprojectNode.clampIntensity` — how far the history is pulled
    /// towards its variance-clipped colour. 1 by default.
    pub fn clamp_intensity(&self) -> &SettableValue {
        &self.0.clamp_intensity.value
    }

    /// `temporalReprojectNode.flickerSuppression` — how hard bright samples
    /// are compressed before variance clipping. 1 by default.
    pub fn flicker_suppression(&self) -> &SettableValue {
        &self.0.flicker_suppression.value
    }

    /// `temporalReprojectNode.setHistoryTexture( source )` — the history to
    /// reproject when the node does not `accumulate` its own: on
    /// `webgpu_postprocessing_ssr_denoise`, the denoiser's output. `None`
    /// goes back to the internal history. Three stores the texture whatever
    /// `accumulate` is and binds it only when `accumulate` is `false`; so
    /// does this.
    pub fn set_history_texture(&self, texture: Option<&Texture>) {
        let quad = texture.map(|texture| QuadMesh::new(self.0.resolve_material(texture)));
        *self.0.external_quad.borrow_mut() = quad;
    }

    /// The `OnBeforeRenderPipeline` / `OnAfterRenderPipeline` half of
    /// `setup()`: before the pipeline renders, refresh the camera's
    /// projection matrix and give it to the velocity attachment
    /// (`setViewOffset()`); after, clear it (`clearViewOffset()`). The node
    /// does not jitter. Call once, on the pipeline whose output this node
    /// feeds; when the pipeline already has a view-offset owner (a TRAA node,
    /// say) nothing is installed, as in three.
    pub fn attach(&self, pipeline: &mut RenderPipeline) {
        if !pipeline.claim_view_offset() {
            return;
        }
        self.0.attached.set(true);
        let before = self.0.clone();
        pipeline.on_before_render(Box::new(move |renderer: &mut Renderer| {
            before.set_view_offset(renderer);
        }));
        pipeline.on_after_render(Box::new(move |renderer: &mut Renderer| {
            renderer.set_velocity_projection_matrix(None);
        }));
    }

    /// `temporalReprojectNode.setSize( width, height )`: the history and
    /// resolve targets and the resolution uniform. `update_before()` calls
    /// it every frame with the drawing-buffer size.
    pub fn set_size(&self, width: u32, height: u32) {
        self.0.set_size(width, height);
    }

    /// `temporalReprojectNode.dispose()`: release the GPU textures of the
    /// history, resolve and previous-normal targets. The crate has no
    /// `dispose()` on a target, so this drops the textures and returns the
    /// targets to their 1×1 starting size; a node rendered again afterwards
    /// starts a fresh history, as three's would after its targets were
    /// recreated.
    pub fn dispose(&self) {
        let state = &self.0;
        for target in [&state.history, &state.resolve, &state.previous_normal] {
            target.set_size(1, 1);
            target.texture().clear_gpu();
        }
        if let Some(depth) = state.history.depth_texture() {
            depth.inner().borrow_mut().gpu = None;
        }
    }

    /// The `TemporalReproject.seed` quad material, for the dump gate.
    #[doc(hidden)]
    pub fn seed_material(&self) -> MeshBasicNodeMaterial {
        self.0.seed_quad.material.clone()
    }

    /// The `TemporalReproject.resolve` quad material bound to the history
    /// three would bind, for the dump gate.
    #[doc(hidden)]
    pub fn quad_material(&self) -> MeshBasicNodeMaterial {
        let external = self.0.external_quad.borrow();
        match (&*external, self.0.accumulate) {
            (Some(quad), false) => quad.material.clone(),
            _ => self.0.internal_quad.material.clone(),
        }
    }
}

impl TemporalReprojectState {
    /// `setViewOffset()`: three only keeps the velocity attachment's
    /// projection unjittered — there is no jitter of its own to apply.
    fn set_view_offset(&self, renderer: &mut Renderer) {
        let mut camera = self.camera.borrow_mut();
        camera.update_projection_matrix();
        renderer.set_velocity_projection_matrix(Some(camera.projection_matrix));
    }

    /// `setSize( width, height )`.
    fn set_size(&self, width: u32, height: u32) {
        self.history.set_size(width, height);
        self.resolve.set_size(width, height);
        self.resolution.value.set(vec![width as f64, height as f64]);
    }

    /// `this._resolveMaterial` with `_buildResolve()`'s `fragmentNode`,
    /// reading `history`.
    fn resolve_material(&self, history: &Texture) -> MeshBasicNodeMaterial {
        let cam = &self.camera_uniforms;
        let fragment = resolve_node(&ResolveInputs {
            beauty: &self.beauty,
            depth: &self.depth,
            normal: &self.normal,
            velocity: &self.velocity,
            history,
            previous_depth: &self
                .history
                .depth_texture()
                .expect("three-rs: the history target carries a DepthTexture"),
            previous_normal: &self.previous_normal.texture(),
            resolution: self.resolution.node.clone(),
            view: cam.view.node.clone(),
            projection_inverse: cam.projection_inverse.node.clone(),
            world: cam.world.node.clone(),
            world_position: cam.world_position.node.clone(),
            previous_world: cam.previous_world.node.clone(),
            previous_view: cam.previous_view.node.clone(),
            previous_projection: cam.previous_projection.node.clone(),
            previous_projection_inverse: cam.previous_projection_inverse.node.clone(),
            max_frames: self.max_frames.node.clone(),
            hit_point_reprojection: self.hit_point_reprojection.node.clone(),
            clamp_intensity: self.clamp_intensity.node.clone(),
            flicker_suppression: self.flicker_suppression.node.clone(),
            specular: self.mode == TemporalReprojectMode::Specular,
        });
        let mut material = MeshBasicNodeMaterial::new();
        material.name = "TemporalReproject.resolve";
        material.fragment_node = Some(fragment);
        material
    }
}

impl NodeUpdate for TemporalReprojectState {
    /// `this.updateBeforeType = NodeUpdateType.FRAME`.
    fn update_before_type(&self) -> NodeUpdateType {
        NodeUpdateType::Frame
    }

    /// `TemporalReprojectNode.updateBefore( frame )`.
    fn update_before(&self, renderer: &mut Renderer) -> bool {
        // The scene pass first — see the module docs.
        if let Some(pass) = crate::nodes::frame::texture_update(self.beauty.id()) {
            renderer.update_before_node(&pass);
        }

        self.camera_uniforms
            .update_from_camera(&self.camera.borrow());

        let (width, height) = renderer.drawing_buffer_size();

        // `_rendererState = RendererUtils.resetRendererState( renderer, … )`.
        let mut renderer = renderer.reset_state();

        let needs_restart = self.history.size() != (width, height);
        self.set_size(width, height);

        let mut history_swapped_for_restart = false;
        if needs_restart {
            renderer.init_render_target(&self.history);
            renderer.init_render_target(&self.resolve);

            // `this._previousNormalTexture = this.normalNode.value.clone()`.
            self.previous_normal.set_size(width, height);
            let previous_normal = self.previous_normal.texture();
            if previous_normal.format() != self.normal.format() {
                previous_normal.set_format(self.normal.format());
                previous_normal.clear_gpu();
            }
            renderer.init_render_target(&self.previous_normal);

            // External history (a denoiser's feedback) is stale at the old
            // resolution — use the freshly seeded internal history for this
            // frame instead.
            history_swapped_for_restart = !self.accumulate && self.external_quad.borrow().is_some();

            renderer.set_render_target(Some(self.history.clone()));
            renderer.render_quad(&self.seed_quad);
            renderer.set_render_target(None);
        }

        // `_syncHistoryTextureBinding()`'s choice, with the restart swap.
        renderer.set_render_target(Some(self.resolve.clone()));
        {
            let external = self.external_quad.borrow();
            let quad = match (&*external, self.accumulate || history_swapped_for_restart) {
                (Some(quad), false) => quad,
                _ => &self.internal_quad,
            };
            renderer.render_quad(quad);
        }
        renderer.set_render_target(None);

        if !history_swapped_for_restart && self.accumulate {
            renderer.copy_render_texture(&self.resolve.texture(), &self.history.texture());
        }

        // The current depth and normal become the previous ones.
        if self.normal.has_gpu() && self.normal.size() == (width, height) {
            let history_depth = self
                .history
                .depth_texture()
                .expect("three-rs: the history target carries a DepthTexture");
            renderer.copy_depth_texture(&self.depth, &history_depth);
            renderer.copy_render_texture(&self.normal, &self.previous_normal.texture());
        }

        // `RendererUtils.restoreRendererState( renderer, _rendererState )`.
        drop(renderer);
        true
    }
}

/// `this._seedMaterial` with `_buildSeed()`'s `fragmentNode`: the beauty at
/// the resolve texel, clamped at zero.
fn seed_material(beauty: &Texture, resolution: &NodeRef) -> MeshBasicNodeMaterial {
    let screen_texel = floor(screen_coordinate().sub(0.5)).to(Type::IVec2);
    let beauty_size = texture_size(TextureSource::Texture2D(beauty.clone()), int(0));
    let beauty_texel = call(
        &beauty_texel_from_screen(),
        vec![screen_texel, beauty_size, resolution.clone()],
    );
    let mut material = MeshBasicNodeMaterial::new();
    material.name = "TemporalReproject.seed";
    material.fragment_node = Some(texture_load(beauty, beauty_texel).max(0.0));
    material
}

/// What `_buildResolve()`'s `resolve` closes over.
struct ResolveInputs<'a> {
    beauty: &'a Texture,
    depth: &'a DepthTexture,
    normal: &'a Texture,
    velocity: &'a Texture,
    history: &'a Texture,
    previous_depth: &'a DepthTexture,
    previous_normal: &'a Texture,
    resolution: NodeRef,
    view: NodeRef,
    projection_inverse: NodeRef,
    world: NodeRef,
    world_position: NodeRef,
    previous_world: NodeRef,
    previous_view: NodeRef,
    previous_projection: NodeRef,
    previous_projection_inverse: NodeRef,
    max_frames: NodeRef,
    hit_point_reprojection: NodeRef,
    clamp_intensity: NodeRef,
    flicker_suppression: NodeRef,
    specular: bool,
}

/// `_buildResolve()`'s `resolve` `Fn()`.
#[inline(never)]
fn resolve_node(inputs: &ResolveInputs) -> NodeRef {
    let uv_node = uv();
    let res = &inputs.resolution;
    let fs = &inputs.flicker_suppression;
    let mut statements = Vec::new();

    let screen_texel = to_const(None, floor(screen_coordinate().sub(0.5)).to(Type::IVec2));
    statements.push(screen_texel.clone());
    let depth = to_var(None, depth_texture_load(inputs.depth, screen_texel.clone()));
    statements.push(depth.clone());
    statements.push(if_then(depth.greater_than_equal(1.0), vec![discard()]));

    let beauty_size = texture_size(TextureSource::Texture2D(inputs.beauty.clone()), int(0));
    let beauty_texel = call(
        &beauty_texel_from_screen(),
        vec![screen_texel.clone(), beauty_size, res.clone()],
    );

    let input_color = to_var(
        None,
        texture_load(inputs.beauty, beauty_texel.clone()).max(0.0),
    );
    statements.push(input_color.clone());
    let view_normal = to_var(
        None,
        unpack_rgb_to_normal(texture_load(inputs.normal, screen_texel.clone()).rgb()),
    );
    statements.push(view_normal.clone());

    // Shared 3×3 beauty fetch: feeds both the variance-clip box and the SSR
    // ray-length statistics. Specular mode assigns into it, which makes three
    // declare it as a variable here; diffuse mode only reads it, so it is
    // generated where first read.
    let neighborhood_layout = neighborhood_struct();
    let mut neighborhood = collect_neighborhood(
        inputs.beauty,
        &beauty_texel,
        &input_color,
        fs,
        &neighborhood_layout,
    );
    if inputs.specular {
        neighborhood = to_var_intent(neighborhood);
        statements.push(neighborhood.clone());
    }
    let world_normal = to_var(
        None,
        view_normal.transform_normal_by_inverse_view_matrix(inputs.view.clone()),
    );
    statements.push(world_normal.clone());

    let view_position = to_var(
        None,
        get_view_position(uv_node.clone(), depth, inputs.projection_inverse.clone()),
    );
    statements.push(view_position.clone());
    let world_position = to_var(
        None,
        inputs
            .world
            .mul(vec4_join(vec![view_position, float(1.0)]))
            .xyz(),
    );
    statements.push(world_position.clone());

    let tap_layout = bilinear_tap_struct();
    let history_layout = history_result_struct();
    let history_ctx = HistoryContext {
        history: inputs.history,
        previous_depth: inputs.previous_depth,
        previous_normal: inputs.previous_normal,
        resolution: res.clone(),
        previous_projection_inverse: inputs.previous_projection_inverse.clone(),
        previous_world: inputs.previous_world.clone(),
        previous_view: inputs.previous_view.clone(),
        world_position: world_position.clone(),
        world_normal: world_normal.clone(),
        tap_layout: &tap_layout,
        history_layout: &history_layout,
    };
    let sample_history =
        |reproj_uv: &NodeRef| sample_history_4tap(&history_ctx, reproj_uv, &input_color.rgb());

    // Surface-velocity reprojection — the base history for both modes.
    let velocity_off = to_var(
        None,
        call(
            &velocity_to_uv_offset(),
            vec![texture_load(inputs.velocity, screen_texel).xy()],
        ),
    );
    statements.push(velocity_off.clone());
    let motion_factor = length(velocity_off.mul(res.clone()))
        .div(float(MAX_VELOCITY_LENGTH))
        .saturate();

    let history_uv = to_var(None, uv_node.sub(velocity_off));
    statements.push(history_uv.clone());
    let surf = sample_history(&history_uv);

    let history_color = to_var(None, struct_get(&surf, &history_layout, "color"));
    statements.push(history_color.clone());
    let total_confidence = to_var(None, float(1.0));
    statements.push(total_confidence.clone());
    let history_trust = to_var(None, float(0.0));
    statements.push(history_trust.clone());

    if inputs.specular {
        statements.extend(resolve_specular_history(&SpecularInputs {
            neighborhood: &neighborhood,
            neighborhood_layout: &neighborhood_layout,
            history_layout: &history_layout,
            history_uv: &history_uv,
            surf: &surf,
            world_position: &world_position,
            world_normal: &world_normal,
            camera_world_position: &inputs.world_position,
            previous_view: &inputs.previous_view,
            previous_projection: &inputs.previous_projection,
            hit_point_reprojection: &inputs.hit_point_reprojection,
            motion_factor: &motion_factor,
            input_color: &input_color,
            history_color: &history_color,
            total_confidence: &total_confidence,
            history_trust: &history_trust,
            sample_history: &sample_history,
        }));
    }

    let a = history_color.a().max(epsilon());

    // Universal stretch guard: reduce confidence where a small area is
    // projected over a large one.
    let stretch_confidence = reprojection_stretch_confidence(&history_uv, res);
    statements.push(total_confidence.mul_assign(stretch_confidence.pow(2.0)));

    let variance_gamma = mix(
        float(VARIANCE_GAMMA_MIN),
        float(VARIANCE_GAMMA_MAX),
        motion_factor.one_minus().pow2(),
    );
    let clipped_rgb = to_var(
        None,
        apply_variance_clipping(
            &history_color,
            &struct_get(&neighborhood, &neighborhood_layout, "mean"),
            &struct_get(&neighborhood, &neighborhood_layout, "stdColor"),
            variance_gamma,
            fs,
        ),
    );
    statements.push(clipped_rgb.clone());

    let clamp_intensity = inputs
        .clamp_intensity
        .mul(max(motion_factor.mul(10.0).min(1.0), 0.25))
        .mul(
            float(1.0).add(
                stretch_confidence
                    .one_minus()
                    .add(history_trust.one_minus())
                    .clamp(0.0, 1.0),
            ),
        );
    statements.push(history_color.rgb().assign(mix(
        history_color.rgb(),
        clipped_rgb.clone(),
        clamp_intensity.clone(),
    )));

    // Three's `originalHistoryColor = vec3( historyColor.rgb )` is the same
    // node as `historyColor.rgb`, so it reads the colour after the
    // assignment above; the port reads it there too.
    statements.push(
        total_confidence.mul_assign(exp(length(history_color.rgb().sub(clipped_rgb))
            .mul(clamp_intensity)
            .mul(30.0)
            .negate())),
    );
    statements.push(total_confidence.mul_assign(mix(
        float(1.0),
        history_trust.mul(0.05).add(0.95),
        motion_factor.mul(100.0).clamp(0.0, 1.0),
    )));

    statements.push(if_then(
        total_confidence.less_than(epsilon()),
        vec![history_color.assign(vec4_join(vec![input_color.rgb(), float(1.0)]))],
    ));

    let current_frame_count = to_var(
        None,
        float(1.0)
            .div(a)
            .mul(total_confidence)
            .add(1.0)
            .min(inputs.max_frames.clone()),
    );
    statements.push(current_frame_count.clone());

    if inputs.specular {
        // A black current sample means no reflection was found this frame (a
        // miss, not dark): the next accumulating pass will count it, so take
        // one frame back.
        statements.push(if_then(
            length(input_color.rgb()).less_than(epsilon()),
            vec![current_frame_count.assign(current_frame_count.sub(1.0).max(1.0))],
        ));
    }

    block(
        statements,
        vec4_join(vec![
            history_color.rgb(),
            float(1.0).div(current_frame_count),
        ]),
    )
}

/// What `resolveSpecularHistory()` closes over.
struct SpecularInputs<'a> {
    neighborhood: &'a NodeRef,
    neighborhood_layout: &'a Rc<StructLayout>,
    history_layout: &'a Rc<StructLayout>,
    history_uv: &'a NodeRef,
    surf: &'a NodeRef,
    world_position: &'a NodeRef,
    world_normal: &'a NodeRef,
    camera_world_position: &'a NodeRef,
    previous_view: &'a NodeRef,
    previous_projection: &'a NodeRef,
    hit_point_reprojection: &'a NodeRef,
    motion_factor: &'a NodeRef,
    input_color: &'a NodeRef,
    history_color: &'a NodeRef,
    total_confidence: &'a NodeRef,
    history_trust: &'a NodeRef,
    sample_history: &'a dyn Fn(&NodeRef) -> NodeRef,
}

/// `uv.x >= 0 && uv.x <= 1 && uv.y >= 0 && uv.y <= 1`, chained as three
/// writes it.
fn uv_in_range(uv: &NodeRef) -> NodeRef {
    uv.x()
        .greater_than_equal(0.0)
        .and(uv.x().less_than_equal(1.0))
        .and(uv.y().greater_than_equal(0.0))
        .and(uv.y().less_than_equal(1.0))
}

/// `resolveSpecularHistory()` with the three assignments `_buildResolve()`
/// makes from its result: the parallax hit-point history blended over the
/// surface one. Returns the statements, in three's order.
#[inline(never)]
fn resolve_specular_history(inputs: &SpecularInputs) -> Vec<NodeRef> {
    let layout = inputs.history_layout;
    let nb_layout = inputs.neighborhood_layout;
    let mut statements = Vec::new();

    let surf_valid = uv_in_range(inputs.history_uv);

    let history_uv_hit = to_var(
        None,
        reproject_hit_point(
            inputs.world_position,
            &struct_get(inputs.neighborhood, nb_layout, "rayLength"),
            inputs.camera_world_position,
            inputs.previous_view,
            inputs.previous_projection,
        ),
    );
    statements.push(history_uv_hit.clone());

    let hit_valid = uv_in_range(&history_uv_hit).and(inputs.hit_point_reprojection.clone());

    let hit = (inputs.sample_history)(&history_uv_hit);

    let hc_hit = struct_get(&hit, layout, "color").rgb().max(0.0);
    let hc_surf = struct_get(inputs.surf, layout, "color").rgb().max(0.0);

    let conf_hit = hit_valid.select(struct_get(&hit, layout, "tapConfidence"), float(0.0));
    let conf_surf = surf_valid.select(struct_get(inputs.surf, layout, "tapConfidence"), float(0.0));
    let min_conf_hit = struct_get(&hit, layout, "minConfidence");

    let reflection_edge_factor = struct_get(inputs.neighborhood, nb_layout, "stdDevRayLength");
    statements.push(
        reflection_edge_factor.assign(
            reflection_edge_factor
                .mul(inputs.motion_factor.mul(100.0).min(1.0))
                .mul(3.5)
                .min(1.0)
                .one_minus(),
        ),
    );

    let curvature_factor = length(fwidth(inputs.world_normal.clone()))
        .mul(50.0)
        .clamp(0.0, 1.0);

    let env_probability = struct_get(inputs.neighborhood, nb_layout, "envProbability");

    let w_hit_raw = to_const(
        None,
        min_conf_hit
            .mul(reflection_edge_factor)
            .mul(curvature_factor.one_minus())
            .mul(conf_hit.clone()),
    );
    statements.push(w_hit_raw.clone());

    let w_hit = w_hit_raw.mul(env_probability.pow2().one_minus());
    let w_surf = w_hit.one_minus().mul(conf_surf.clone());
    let w_sum = max(w_hit.add(w_surf.clone()), epsilon());

    let color = to_var(
        None,
        vec4_join(vec![
            hc_hit
                .mul(w_hit.clone())
                .add(hc_surf.mul(w_surf.clone()))
                .div(w_sum.clone()),
            struct_get(inputs.surf, layout, "color").a(),
        ]),
    );
    statements.push(color.clone());
    let confidence = conf_hit.mul(w_hit).add(conf_surf.mul(w_surf)).div(w_sum);

    // A near-black blend means neither tap was usable — fall back to the
    // current frame.
    statements.push(if_then(
        length(color.rgb()).less_than(epsilon()),
        vec![color.assign(vec4_join(vec![inputs.input_color.rgb(), float(1.0)]))],
    ));

    statements.push(inputs.history_color.assign(color));
    statements.push(inputs.total_confidence.assign(confidence));
    statements.push(inputs.history_trust.assign(w_hit_raw));
    statements
}

/// `struct( { mean: 'vec3', stdColor: 'vec3', rayLength: 'float',
/// envProbability: 'float', stdDevRayLength: 'float' } )` —
/// `neighborhoodStruct`, the third struct the module creates.
fn neighborhood_struct() -> Rc<StructLayout> {
    struct_type(
        "StructType2",
        vec![
            StructMember::new("mean", Type::Vec3),
            StructMember::new("stdColor", Type::Vec3),
            StructMember::new("rayLength", Type::F32),
            StructMember::new("envProbability", Type::F32),
            StructMember::new("stdDevRayLength", Type::F32),
        ],
    )
}

/// `bilinearTapStruct` — `{ color: 'vec4', weight: 'float', confidence:
/// 'float' }`.
fn bilinear_tap_struct() -> Rc<StructLayout> {
    struct_type(
        "StructType0",
        vec![
            StructMember::new("color", Type::Vec4),
            StructMember::new("weight", Type::F32),
            StructMember::new("confidence", Type::F32),
        ],
    )
}

/// `historyResultStruct` — `{ color: 'vec4', tapConfidence: 'float',
/// minConfidence: 'float' }`.
fn history_result_struct() -> Rc<StructLayout> {
    struct_type(
        "StructType1",
        vec![
            StructMember::new("color", Type::Vec4),
            StructMember::new("tapConfidence", Type::F32),
            StructMember::new("minConfidence", Type::F32),
        ],
    )
}

/// `beautyTexelFromScreen( screenTexel, beautySize, resolveSize )`, with its
/// layout: the beauty texel under a resolve texel when the two resolutions
/// differ.
fn beauty_texel_from_screen() -> Rc<FnDef> {
    shader_fn(
        Some("beautyTexelFromScreen"),
        vec![
            ("screenTexel", Type::IVec2),
            ("beautySize", Type::Vec2),
            ("resolveSize", Type::Vec2),
        ],
        Type::IVec2,
        |args| {
            floor(
                args[0]
                    .to(Type::Vec2)
                    .mul(args[1].clone())
                    .div(args[2].clone()),
            )
            .to(Type::IVec2)
        },
    )
}

/// `projectWorldToUV( worldPos, previousViewMatrix, previousProjectionMatrix
/// )`, with its layout: a world position in the previous frame's UVs, or
/// `(-1, -1)` behind the camera plane.
fn project_world_to_uv() -> Rc<FnDef> {
    shader_fn(
        Some("projectWorldToUV"),
        vec![
            ("worldPos", Type::Vec3),
            ("previousViewMatrix", Type::Mat4),
            ("previousProjectionMatrix", Type::Mat4),
        ],
        Type::Vec2,
        |args| {
            let result_uv = to_var(None, vec2(-1.0, -1.0));
            let view_space = args[1].mul(vec4_join(vec![args[0].clone(), float(1.0)]));
            let clip_space = to_var(None, args[2].mul(view_space));
            let clip_w = to_var(None, clip_space.w());
            let ndc = clip_space.xyz().div(clip_w.clone());
            block(
                vec![
                    result_uv.clone(),
                    clip_space,
                    clip_w.clone(),
                    if_then(
                        abs(clip_w).greater_than(float(1e-5)),
                        vec![
                            result_uv.assign(ndc.xy().mul(0.5).add(0.5)),
                            result_uv.y().assign(result_uv.y().one_minus()),
                        ],
                    ),
                ],
                result_uv,
            )
        },
    )
}

/// `rgbToYCoCg( c )`.
fn rgb_to_ycocg(c: &NodeRef) -> NodeRef {
    vec3_join(vec![
        dot(c.clone(), vec3(0.25, 0.5, 0.25)),
        dot(c.clone(), vec3(0.5, 0.0, -0.5)),
        dot(c.clone(), vec3(-0.25, 0.5, -0.25)),
    ])
}

/// `ycocgToRGB( c )`.
fn ycocg_to_rgb(c: &NodeRef) -> NodeRef {
    vec3_join(vec![
        c.x().add(c.y()).sub(c.z()),
        c.x().add(c.z()),
        c.x().sub(c.y()).sub(c.z()),
    ])
}

/// `dampenForVarianceClip( rgb, flickerSuppression )`: inverse-luminance
/// compression, so bright samples do not inflate the variance box.
fn dampen_for_variance_clip(rgb: &NodeRef, flicker_suppression: &NodeRef) -> NodeRef {
    let scale = luminance(rgb.clone())
        .mul(flicker_suppression.clone())
        .mul(VARIANCE_CLIP_LUMA_SCALE)
        .add(1.0);
    rgb.div(scale)
}

/// `clipToAABB( history, boxMin, boxMax )`, with its layout: the history
/// pulled towards the box centre until it is inside.
fn clip_to_aabb() -> Rc<FnDef> {
    shader_fn(
        Some("clipToAABB"),
        vec![
            ("history", Type::Vec3),
            ("boxMin", Type::Vec3),
            ("boxMax", Type::Vec3),
        ],
        Type::Vec3,
        |args| {
            let (history, box_min, box_max) = (&args[0], &args[1], &args[2]);
            let p_clip = to_const(None, box_max.add(box_min.clone()).mul(0.5));
            let e_clip = box_max.sub(box_min.clone()).mul(0.5).add(1e-7);
            let v_clip = to_const(None, history.sub(p_clip.clone()));
            let abs_unit = to_const(None, v_clip.div(e_clip).abs());
            let max_unit = to_const(None, max(max(abs_unit.x(), abs_unit.y()), abs_unit.z()));
            max_unit
                .greater_than(1.0)
                .select(p_clip.add(v_clip.div(max_unit.clone())), history.clone())
        },
    )
}

/// `collectNeighborhood( beautyTexture, beautyTexel, inputColor,
/// flickerSuppression )`: one pass over the 3×3 beauty neighbourhood for the
/// YCoCg colour moments and, by Welford, the SSR ray-length statistics of the
/// screen-space hits.
#[inline(never)]
fn collect_neighborhood(
    beauty: &Texture,
    beauty_texel: &NodeRef,
    input_color: &NodeRef,
    flicker_suppression: &NodeRef,
    layout: &Rc<StructLayout>,
) -> NodeRef {
    const OFFSETS: [(i64, i64); 8] = [
        (-1, -1),
        (-1, 1),
        (1, -1),
        (1, 1),
        (1, 0),
        (0, -1),
        (0, 1),
        (-1, 0),
    ];

    // Colour moments (YCoCg) — the centre reuses the fetched input colour.
    let center = rgb_to_ycocg(&dampen_for_variance_clip(
        &input_color.rgb(),
        flicker_suppression,
    ));
    let moment1 = to_var(None, center.clone());
    let moment2 = to_var(None, center.pow2());

    // Ray-length statistics (Welford) over screen-space hits only.
    let ray_length_sum = to_var(None, float(0.0));
    let ray_length_count = to_var(None, float(0.0));
    let mean_ray_length = to_var(None, float(0.0));
    let m2_ray_length = to_var(None, float(0.0));
    let mut statements = vec![
        moment1.clone(),
        moment2.clone(),
        ray_length_sum.clone(),
        ray_length_count.clone(),
        mean_ray_length.clone(),
        m2_ray_length.clone(),
    ];

    let accumulate_ray_length = |alpha: NodeRef| {
        let delta = to_var(None, alpha.sub(mean_ray_length.clone()));
        if_then(
            alpha.less_than(ENV_RAY_LENGTH_THRESHOLD),
            vec![
                ray_length_sum.add_assign(alpha.clone()),
                ray_length_count.add_assign(1.0),
                delta.clone(),
                mean_ray_length.add_assign(delta.div(ray_length_count.clone())),
                m2_ray_length.add_assign(delta.mul(alpha.sub(mean_ray_length.clone()))),
            ],
        )
    };

    statements.push(accumulate_ray_length(input_color.a()));

    for (x, y) in OFFSETS {
        let neighbor = to_var(
            None,
            texture_load(beauty, beauty_texel.add(ivec2(int(x), int(y)))).max(0.0),
        );
        statements.push(neighbor.clone());
        let c = rgb_to_ycocg(&dampen_for_variance_clip(
            &neighbor.rgb(),
            flicker_suppression,
        ));
        statements.push(moment1.add_assign(c.clone()));
        statements.push(moment2.add_assign(c.pow2()));
        statements.push(accumulate_ray_length(neighbor.a()));
    }

    let n = float((OFFSETS.len() + 1) as f64);
    let mean = moment1.div(n.clone());
    let std_color = moment2.div(n).sub(mean.pow2()).max(0.0).sqrt();

    // Continuous environment probability: the fraction of the neighbourhood
    // that missed in screen space and fell back to the environment.
    let env_probability = ray_length_count.div(float(9.0)).one_minus();
    let ray_length = ray_length_count.less_than(0.5).select(
        float(ENV_RAY_LENGTH),
        ray_length_sum.div(max(ray_length_count.clone(), float(1e-4))),
    );
    let std_dev_ray_length = sqrt(m2_ray_length.div(max(ray_length_count, float(1.0)))).max(1e-3);

    block(
        statements,
        struct_new(
            layout,
            vec![
                mean,
                std_color,
                ray_length,
                env_probability,
                std_dev_ray_length,
            ],
        ),
    )
}

/// `applyVarianceClipping( historyColor, mean, stdColor, gamma,
/// flickerSuppression )`: the history, compressed like the neighbourhood,
/// clipped to the YCoCg box widened by `gamma`, and decompressed.
#[inline(never)]
fn apply_variance_clipping(
    history_color: &NodeRef,
    mean: &NodeRef,
    std_color: &NodeRef,
    gamma: NodeRef,
    flicker_suppression: &NodeRef,
) -> NodeRef {
    let stddev = std_color.mul(gamma);
    let box_min = mean.sub(stddev.clone());
    let box_max = mean.add(stddev);

    let history_rgb = to_var(None, history_color.rgb());
    let history_scale = luminance(history_rgb.clone())
        .mul(flicker_suppression.clone())
        .mul(VARIANCE_CLIP_LUMA_SCALE)
        .add(1.0);
    let clipped = call(
        &clip_to_aabb(),
        vec![
            rgb_to_ycocg(&history_rgb.div(history_scale.clone())),
            box_min,
            box_max,
        ],
    );
    block(vec![history_rgb], ycocg_to_rgb(&clipped).mul(history_scale))
}

/// The shared inputs of every `sampleBilinearTap()` — three's `tapCtx`.
struct HistoryContext<'a> {
    history: &'a Texture,
    previous_depth: &'a DepthTexture,
    previous_normal: &'a Texture,
    resolution: NodeRef,
    previous_projection_inverse: NodeRef,
    previous_world: NodeRef,
    previous_view: NodeRef,
    world_position: NodeRef,
    world_normal: NodeRef,
    tap_layout: &'a Rc<StructLayout>,
    history_layout: &'a Rc<StructLayout>,
}

/// `sampleBilinearTap( … )`: one history texel, trusted by how close its
/// reconstructed world position lies to the current surface's plane and how
/// well its normal agrees.
#[inline(never)]
fn sample_bilinear_tap(
    ctx: &HistoryContext,
    tap_coord: &NodeRef,
    bilinear_weight: NodeRef,
) -> NodeRef {
    let color = texture_load_transformed(ctx.history, tap_coord.clone()).max(0.0);
    let reproj_depth = depth_texture_load_transformed(ctx.previous_depth, tap_coord.clone());
    let reproj_view_pos = get_view_position(
        tap_coord
            .to(Type::Vec2)
            .add(0.5)
            .div(ctx.resolution.clone()),
        reproj_depth,
        ctx.previous_projection_inverse.clone(),
    );
    let reproj_world_pos = ctx
        .previous_world
        .mul(vec4_join(vec![reproj_view_pos.clone(), float(1.0)]))
        .xyz();
    let reproj_world_norm = unpack_rgb_to_normal(
        texture_load_transformed(ctx.previous_normal, tap_coord.clone()).rgb(),
    )
    .transform_normal_by_inverse_view_matrix(ctx.previous_view.clone());

    let plane_diff = to_var(
        None,
        abs(dot(
            reproj_world_pos.sub(ctx.world_position.clone()),
            ctx.world_normal.clone(),
        )),
    );
    let divide = plane_diff.div_assign(abs(reproj_view_pos.z()));
    let normal_confidence =
        smoothstep(0.95, 0.999, reproj_world_norm.dot(ctx.world_normal.clone()));
    let confidence = smoothstep(0.0, 0.01, plane_diff.clone())
        .one_minus()
        .mul(normal_confidence);
    let weight = bilinear_weight.mul(confidence.clone());

    block(
        vec![plane_diff, divide],
        struct_new(
            ctx.tap_layout,
            vec![color.mul(weight.clone()), weight, confidence],
        ),
    )
}

/// `sampleHistory4Tap( … )`: the geometrically weighted bilinear history
/// sample at `reprojUV`, falling back to the current colour when no tap is
/// trusted.
#[inline(never)]
fn sample_history_4tap(ctx: &HistoryContext, reproj_uv: &NodeRef, input_rgb: &NodeRef) -> NodeRef {
    let reproj_pixel_coord = to_var(None, reproj_uv.mul(ctx.resolution.clone()).sub(0.5));
    let reproj_icoord = to_const(None, floor(reproj_pixel_coord.clone()).to(Type::IVec2));
    let f_coord = reproj_pixel_coord.fract();

    let fx = f_coord.x();
    let fy = f_coord.y();
    let f00 = float(1.0).sub(fx.clone()).mul(float(1.0).sub(fy.clone()));
    let f10 = fx.mul(float(1.0).sub(fy.clone()));
    let f01 = float(1.0).sub(fx.clone()).mul(fy.clone());
    let f11 = fx.mul(fy);

    let tap = |x: i64, y: i64, weight: NodeRef| {
        sample_bilinear_tap(ctx, &reproj_icoord.add(ivec2(int(x), int(y))), weight)
    };
    let tap00 = tap(0, 0, f00);
    let tap10 = tap(1, 0, f10);
    let tap01 = tap(0, 1, f01);
    let tap11 = tap(1, 1, f11);
    let get = |tap: &NodeRef, name: &str| struct_get(tap, ctx.tap_layout, name);

    let color_sum = get(&tap00, "color")
        .add(get(&tap10, "color"))
        .add(get(&tap01, "color"))
        .add(get(&tap11, "color"));
    let weight_sum = get(&tap00, "weight")
        .add(get(&tap10, "weight"))
        .add(get(&tap01, "weight"))
        .add(get(&tap11, "weight"));
    let max_conf = max(
        max(get(&tap00, "confidence"), get(&tap10, "confidence")),
        max(get(&tap01, "confidence"), get(&tap11, "confidence")),
    );
    let min_conf = get(&tap00, "confidence")
        .min(get(&tap10, "confidence"))
        .min(get(&tap01, "confidence").min(get(&tap11, "confidence")));

    block(
        vec![reproj_pixel_coord],
        struct_new(
            ctx.history_layout,
            vec![
                weight_sum.greater_than(0.01).select(
                    color_sum.div(weight_sum.clone()),
                    vec4_join(vec![input_rgb.clone(), float(1.0)]),
                ),
                max_conf,
                min_conf,
            ],
        ),
    )
}

/// `reprojectionStretchConfidence( historyUV, resolution )`: the minimum
/// singular value of the reprojection Jacobian, clamped to [0, 1] — below 1
/// where history is magnified (undersampled).
#[inline(never)]
fn reprojection_stretch_confidence(history_uv: &NodeRef, resolution: &NodeRef) -> NodeRef {
    let jx = to_var(None, dpdx(history_uv.clone()).mul(resolution.clone()));
    let jy = to_var(None, dpdy(history_uv.clone()).mul(resolution.clone()));

    let det = jx.x().mul(jy.y()).sub(jx.y().mul(jy.x()));
    let fro2 = dot(jx.clone(), jx.clone()).add(dot(jy.clone(), jy.clone()));
    let disc = fro2
        .mul(fro2.clone())
        .mul(0.25)
        .sub(det.mul(det.clone()))
        .max(0.0)
        .sqrt();
    let sig_min = fro2.mul(0.5).sub(disc).max(0.0).sqrt();

    block(vec![jx, jy], sig_min.saturate())
}

/// `reprojectHitPoint( rayOrig, rayLength, cameraWorldPosition,
/// previousViewMatrix, previousProjectionMatrix )`: the reflection's
/// parallax hit point in the previous frame's UVs.
#[inline(never)]
fn reproject_hit_point(
    ray_orig: &NodeRef,
    ray_length: &NodeRef,
    camera_world_position: &NodeRef,
    previous_view: &NodeRef,
    previous_projection: &NodeRef,
) -> NodeRef {
    let camera_ray = to_var(
        None,
        ray_orig.sub(camera_world_position.clone()).normalize(),
    );
    let parallax_hit_point = ray_orig.add(camera_ray.mul(ray_length.clone()));
    block(
        vec![camera_ray],
        call(
            &project_world_to_uv(),
            vec![
                parallax_hit_point,
                previous_view.clone(),
                previous_projection.clone(),
            ],
        ),
    )
}

/// `velocityToUVOffset( velocity )`, with its layout: the velocity
/// attachment's NDC offset as a UV offset.
fn velocity_to_uv_offset() -> Rc<FnDef> {
    shader_fn(
        Some("velocityToUVOffset"),
        vec![("velocity", Type::Vec2)],
        Type::Vec2,
        |args| args[0].mul(vec2(0.5, -0.5)),
    )
}
