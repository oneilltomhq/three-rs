//! Port of `three.js/examples/jsm/tsl/display/TAAUNode.js` — temporal
//! anti-aliased upsampling.
//!
//! The scene is rendered at a reduced resolution (the page uses
//! `setResolutionScale( 0.5 )`) and resolved to the drawing buffer's size.
//! Each output pixel reconstructs the current frame from the 3×3 input
//! texels around its closest jittered input sample — a Gaussian
//! approximation of a Blackman-Harris window, `exp( d² · -2.29 )` — and blends
//! that into a history reprojected along the scene's `velocity` attachment,
//! clipped to the same nine taps' variance. A "lock" term keeps thin, high
//! contrast features from being clipped away. The helpers it shares with
//! `TRAANode` (`examples/jsm/tsl/utils/TAAUtils.js`) live in `taa_utils.rs`.
//!
//! Like [`TraaNode`](super::TraaNode), the node is a `TempNode` with
//! `updateBeforeType = FRAME`: [`TaauState`] implements [`NodeUpdate`] and is
//! registered as the updater of the resolve texture. The jitter is installed
//! on the pipeline by [`TaauNode::attach`].
//!
//! **What three really renders.** Three's graph has quirks the port keeps,
//! because the page's pixels depend on them:
//!
//! - The history target has two attachments, colour and lock, and both the
//!   resolve and the seed material write `outputStruct( colorOutput,
//!   lockOutput )`. The resolve target has only one, so the resolve's lock
//!   output is dropped, and only the seed ever writes the lock attachment —
//!   with zero. `lockNode.r` therefore reads 0 and the lock is the gated
//!   thin-feature term alone. The port draws the same two-output fragment
//!   stage into the same one-attachment target.
//! - Three registers the `OnBeforeRenderPipeline` / `OnAfterRenderPipeline`
//!   callbacks while the pipeline's output is first built, which happens
//!   inside the first `renderPipeline.render()`, after that frame's "before"
//!   callbacks have already run. So the first frame is rendered unjittered
//!   (the jitter uniform still `( 0, 0 )`) while its "after" callback does
//!   run and advances the jitter index to 1. [`TaauNode::attach`] installs
//!   its hooks up front, so its "before" hook skips its first call to render
//!   the same first frame. The e2e harness grades exactly that frame.
//! - The previous-depth texture is a separate, input-sized render target's
//!   depth, unwritten (zero) on the first frame, when the previous camera
//!   matrices are still the identity.
//!
//! **Order within a frame.** As for TRAA, the port asks for the scene pass at
//! the top of `update_before()`; three's `setup()` builds the pass's nodes
//! before itself, so the pass has rendered by then.
//!
//! Not ported: an orthographic camera, a logarithmic or reversed depth
//! buffer, a beauty node that is an `RTTNode` rather than a pass attachment,
//! and a `velocity` other than the global one.

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use crate::cameras::{PerspectiveCamera, RenderCamera};
use crate::materials::MeshBasicNodeMaterial;
use crate::math::Matrix4;
use crate::nodes::frame::register_texture_update;
use crate::nodes::node::{SettableValue, TextureSource, Type};
use crate::nodes::tsl::{
    all, block, call, exp, float, int, ivec2, length, luminance, max, mix, output_struct, property,
    smoothstep, struct_get, texture_load, texture_sample, texture_size, texture_uv, to_const,
    to_var, uniform_settable, uv, vec2, vec4,
};
use crate::nodes::{NodeRef, NodeUpdate, NodeUpdateType};
use crate::objects::QuadMesh;
use crate::renderer::{RenderPipeline, RenderTarget, RenderTargetOptions, Renderer};
use crate::textures::{DepthTexture, Texture, TextureFilter, TextureType};

use super::taa_utils::{
    clip_aabb, current_depth_struct, flicker_reduction, halton_offsets, mat4_values,
    sample_current_depth, sample_previous_depth, JITTER_COUNT,
};

/// `TAAUNode.depthThreshold`: how far the current depth may sit in front of
/// the reprojected one before the history counts as disoccluded.
const DEPTH_THRESHOLD: f64 = 0.0005;
/// `TAAUNode.edgeDepthDiff`: a 3×3 depth range wider than this is an edge,
/// where disocclusion is not trusted.
const EDGE_DEPTH_DIFF: f64 = 0.001;
/// `TAAUNode.maxVelocityLength`, in input pixels: the motion at which the
/// current frame's weight saturates.
const MAX_VELOCITY_LENGTH: f64 = 128.0;
/// `TAAUNode.currentFrameWeight`: the current frame's minimum weight, lower
/// than TRAA's 0.05 because one input frame covers less of the output grid.
const CURRENT_FRAME_WEIGHT: f64 = 0.025;
/// The reconstruction's 3×3 taps around the closest input texel, in three's
/// order.
const TAP_OFFSETS: [(i64, i64); 9] = [
    (-1, -1),
    (0, -1),
    (1, -1),
    (-1, 0),
    (0, 0),
    (1, 0),
    (-1, 1),
    (0, 1),
    (1, 1),
];

/// `taau( beautyNode, depthNode, velocityNode, camera )`.
///
/// As with [`traa`](super::traa), the inputs are the textures three's
/// `convertToTexture()` hands over unchanged for a pass attachment: the
/// scene pass's `output` attachment, its depth texture and its `velocity`
/// attachment. The pass must have an MRT with `velocity` on it, the camera
/// must be the pass's, and the pass is normally rendered at a reduced
/// resolution scale — the node's output is always the drawing buffer's size.
pub fn taau(
    beauty: &Texture,
    depth: &DepthTexture,
    velocity: &Texture,
    camera: Rc<RefCell<PerspectiveCamera>>,
) -> TaauNode {
    TaauNode::new(beauty, depth, velocity, camera)
}

/// `TAAUNode` — see the module docs.
pub struct TaauNode(Rc<TaauState>);

/// The node's state, shared with the renderer's update-before registry and
/// the pipeline hooks.
pub(crate) struct TaauState {
    /// `this.beautyNode`'s texture, whose pass renders the scene.
    beauty: Texture,
    /// `this.depthNode.value`, the pass's depth attachment.
    depth: DepthTexture,
    /// `this.camera`.
    camera: Rc<RefCell<PerspectiveCamera>>,
    /// `this._historyRenderTarget`: output-sized, two attachments
    /// (`TAAUNode.history.color`, `TAAUNode.history.lock`).
    history: RenderTarget,
    /// `this._resolveRenderTarget`: output-sized, one attachment.
    resolve: RenderTarget,
    /// `this._previousDepthRenderTarget`, whose `DepthTexture`
    /// (`TAAUNode.previousDepth`) receives a copy of the scene's depth.
    previous_depth: RenderTarget,
    /// `_quadMesh` with `this._seedMaterial`.
    seed_quad: QuadMesh,
    /// `_quadMesh` with `this._resolveMaterial`.
    resolve_quad: QuadMesh,
    /// `this._textureNode` — `passTexture( this, resolve.texture )`.
    node: NodeRef,
    /// `this._jitterIndex`.
    jitter_index: Cell<usize>,
    /// `this._jitterOffset`, in input pixels.
    jitter_offset: SettableValue,
    /// Whether the "before" hook has yet to see its first frame — see the
    /// module docs.
    first_frame: Cell<bool>,
    /// `this._cameraNearFar`.
    near_far: SettableValue,
    /// `this._cameraWorldMatrix`, read only to roll into
    /// [`previous_world`](Self::previous_world).
    world: Cell<Matrix4>,
    /// `this._cameraWorldMatrixInverse`.
    world_inverse: SettableValue,
    /// `this._cameraProjectionMatrixInverse`, read only to roll over.
    projection_inverse: Cell<Matrix4>,
    /// `this._previousCameraWorldMatrix`.
    previous_world: SettableValue,
    /// `this._previousCameraProjectionMatrixInverse`.
    previous_projection_inverse: SettableValue,
}

impl TaauNode {
    /// `new TAAUNode( beautyNode, depthNode, velocityNode, camera )`, with
    /// the two materials `setup()` gives it.
    pub fn new(
        beauty: &Texture,
        depth: &DepthTexture,
        velocity: &Texture,
        camera: Rc<RefCell<PerspectiveCamera>>,
    ) -> Self {
        let options = || RenderTargetOptions {
            texture_type: TextureType::HalfFloat,
            samples: 0,
            depth_buffer: false,
            min_filter: TextureFilter::Linear,
            mag_filter: TextureFilter::Linear,
        };
        // `new RenderTarget( 1, 1, { depthBuffer: false, type: HalfFloatType,
        // count: 2 } )`.
        let history = RenderTarget::new_with_options(1, 1, options())
            .expect("three-rs: the TAAU history target is a colour type");
        history.set_count(2);
        history.set_texture_name(0, "TAAUNode.history.color");
        history.set_texture_name(1, "TAAUNode.history.lock");
        history.set_copy_destination();
        let resolve = RenderTarget::new_with_options(1, 1, options())
            .expect("three-rs: the TAAU resolve target is a colour type");
        // `new RenderTarget( 1, 1, { depthBuffer: false, depthTexture: new
        // DepthTexture() } )` — the default `UnsignedByteType` colour, which
        // nothing reads; the port's nearest is the half-float one.
        let previous_depth = RenderTarget::new_with_options(1, 1, options())
            .expect("three-rs: the TAAU previous-depth target is a colour type");
        previous_depth.set_depth_texture(DepthTexture::new());
        // The previous depth is written by a copy out of the pass's depth.
        depth.set_copyable();
        let previous_depth_texture = previous_depth
            .depth_texture()
            .expect("three-rs: set just above");
        previous_depth_texture.set_copyable();

        let (jitter, jitter_value) = uniform_settable(Type::Vec2, vec![0.0, 0.0]);
        let (near_far, near_far_value) = uniform_settable(Type::Vec2, vec![0.0, 0.0]);
        let identity = mat4_values(&Matrix4::identity());
        let (world_inverse, world_inverse_value) = uniform_settable(Type::Mat4, identity.clone());
        let (previous_world, previous_world_value) = uniform_settable(Type::Mat4, identity.clone());
        let (previous_projection_inverse, previous_projection_inverse_value) =
            uniform_settable(Type::Mat4, identity);

        let textures = history.textures();
        let (color_output, lock_output) = output_properties();
        let resolve_material = resolve_material(&ResolveInputs {
            beauty,
            depth,
            velocity,
            history: &textures[0],
            lock: &textures[1],
            previous_depth: &previous_depth_texture,
            jitter,
            near_far,
            world_inverse,
            previous_world,
            previous_projection_inverse,
            color_output: color_output.clone(),
            lock_output: lock_output.clone(),
        });
        let seed_material = seed_material(beauty, color_output, lock_output);

        let node = to_var(None, texture_uv(&resolve.texture(), uv()));

        let state = Rc::new(TaauState {
            beauty: beauty.clone(),
            depth: depth.clone(),
            camera,
            history,
            resolve,
            previous_depth,
            seed_quad: QuadMesh::new(seed_material),
            resolve_quad: QuadMesh::new(resolve_material),
            node,
            jitter_index: Cell::new(0),
            jitter_offset: jitter_value,
            first_frame: Cell::new(true),
            near_far: near_far_value,
            world: Cell::new(Matrix4::identity()),
            world_inverse: world_inverse_value,
            projection_inverse: Cell::new(Matrix4::identity()),
            previous_world: previous_world_value,
            previous_projection_inverse: previous_projection_inverse_value,
        });
        register_texture_update(state.resolve.texture().id(), &state);
        Self(state)
    }

    /// `taauNode.getTextureNode()` — the resolved, output-sized frame, for
    /// the graph downstream.
    pub fn node(&self) -> NodeRef {
        self.0.node.clone()
    }

    /// `this._resolveRenderTarget.texture` — what [`node`](Self::node)
    /// samples, for an effect that wants the texture itself, as the page's
    /// `sharpen( taauNode.getTextureNode() )` does.
    pub fn texture(&self) -> Texture {
        self.0.resolve.texture()
    }

    /// The `TAAU.resolve` quad material, for `examples/dump_wgsl.rs` and the
    /// dump gate.
    #[doc(hidden)]
    pub fn quad_material(&self) -> MeshBasicNodeMaterial {
        self.0.resolve_quad.material.clone()
    }

    /// The `TAAU.seed` quad material, for the dump gate.
    #[doc(hidden)]
    pub fn seed_material(&self) -> MeshBasicNodeMaterial {
        self.0.seed_quad.material.clone()
    }

    /// The `OnBeforeRenderPipeline` / `OnAfterRenderPipeline` half of
    /// `TAAUNode.setup()`: jitter the camera by a sub-pixel of the *input*
    /// resolution before the pipeline renders, clear the jitter after. Call
    /// once, on the pipeline whose output this node feeds.
    ///
    /// The "before" hook skips its first call, the frame three renders
    /// unjittered because its callbacks are only registered during that
    /// frame (see the module docs); the "after" hook runs from the first
    /// frame on. Three's `viewOffsetOwner` guard is kept: when the pipeline
    /// already has an owner, nothing is installed.
    pub fn attach(&self, pipeline: &mut RenderPipeline) {
        if !pipeline.claim_view_offset() {
            return;
        }
        let before = self.0.clone();
        pipeline.on_before_render(Box::new(move |renderer: &mut Renderer| {
            if before.first_frame.replace(false) {
                return;
            }
            // `beautyRenderTarget.texture.width / height`.
            let (width, height) = before.beauty.size();
            before.set_view_offset(renderer, width, height);
        }));
        let after = self.0.clone();
        pipeline.on_after_render(Box::new(move |renderer: &mut Renderer| {
            after.clear_view_offset(renderer);
        }));
    }
}

impl TaauState {
    /// `TAAUNode.setViewOffset( inputWidth, inputHeight )`.
    fn set_view_offset(&self, renderer: &mut Renderer, width: u32, height: u32) {
        let mut camera = self.camera.borrow_mut();
        // Save the original, unjittered projection matrix for the velocity
        // pass.
        camera.update_projection_matrix();
        renderer.set_velocity_projection_matrix(Some(camera.projection_matrix));

        let (halton_x, halton_y) = halton_offsets()[self.jitter_index.get()];
        let (jitter_x, jitter_y) = (halton_x - 0.5, halton_y - 0.5);
        self.jitter_offset.set(vec![jitter_x, jitter_y]);

        let (width, height) = (width as f64, height as f64);
        camera.set_view_offset(width, height, jitter_x, jitter_y, width, height);
    }

    /// `TAAUNode.clearViewOffset()`.
    fn clear_view_offset(&self, renderer: &mut Renderer) {
        self.camera.borrow_mut().clear_view_offset();
        renderer.set_velocity_projection_matrix(None);
        self.jitter_index
            .set((self.jitter_index.get() + 1) % JITTER_COUNT);
    }

    /// `TAAUNode.setSize( outputWidth, outputHeight )`.
    fn set_size(&self, width: u32, height: u32) {
        self.history.set_size(width, height);
        self.resolve.set_size(width, height);
    }

    /// `this.depthNode.value.image`'s size, once the pass has allocated it.
    fn depth_size(&self) -> Option<(u32, u32)> {
        let inner = self.depth.inner().borrow();
        inner.gpu.as_ref().map(|gpu| (gpu.width(), gpu.height()))
    }
}

impl NodeUpdate for TaauState {
    /// `this.updateBeforeType = NodeUpdateType.FRAME`.
    fn update_before_type(&self) -> NodeUpdateType {
        NodeUpdateType::Frame
    }

    /// `TAAUNode.updateBefore( frame )`.
    fn update_before(&self, renderer: &mut Renderer) -> bool {
        // The scene pass first — see the module docs.
        if let Some(pass) = crate::nodes::frame::texture_update(self.beauty.id()) {
            renderer.update_before_node(&pass);
        }

        // Store the previous frame's matrices before updating the current
        // ones.
        self.previous_world.set(mat4_values(&self.world.get()));
        self.previous_projection_inverse
            .set(mat4_values(&self.projection_inverse.get()));

        {
            let camera = self.camera.borrow();
            self.near_far.set(vec![camera.near, camera.far]);
            self.world.set(RenderCamera::matrix_world(&*camera));
            self.world_inverse
                .set(mat4_values(&camera.matrix_world_inverse));
            self.projection_inverse
                .set(camera.projection_matrix_inverse);
        }

        // The output dimensions are the drawing buffer's.
        let (width, height) = renderer.drawing_buffer_size();

        // `_rendererState = RendererUtils.resetRendererState( renderer, … )`.
        let mut renderer = renderer.reset_state();

        let needs_restart = self.history.size() != (width, height);
        self.set_size(width, height);

        // Every time the dimensions change the history needs fresh data.
        if needs_restart {
            // Make sure the targets are allocated after the resize, which
            // dropped their GPU textures.
            renderer.init_render_target(&self.history);
            renderer.init_render_target(&self.resolve);
            // Three's backend creates the never-written previous depth when
            // the resolve first binds it; the port's binding wants it to
            // exist already. Either way it reads zero until the first copy.
            if self
                .previous_depth
                .depth_texture()
                .is_some_and(|depth| depth.inner().borrow().gpu.is_none())
            {
                renderer.init_render_target(&self.previous_depth);
            }

            // Seed the history with a bilinear upscale of the current beauty
            // buffer, or the first frames after a resize fade in from black.
            renderer.set_render_target(Some(self.history.clone()));
            renderer.render_quad(&self.seed_quad);
            renderer.set_render_target(None);
        }

        // Resolve.
        renderer.set_render_target(Some(self.resolve.clone()));
        renderer.render_quad(&self.resolve_quad);
        renderer.set_render_target(None);

        // Update the history.
        renderer.copy_render_texture(&self.resolve.texture(), &self.history.texture());

        // Copy the current scene depth into the previous-depth texture, kept
        // at the source's size.
        if let Some((source_width, source_height)) = self.depth_size() {
            if self.previous_depth.size() != (source_width, source_height) {
                self.previous_depth.set_size(source_width, source_height);
                renderer.init_render_target(&self.previous_depth);
            }
            let previous_depth = self
                .previous_depth
                .depth_texture()
                .expect("three-rs: the previous-depth target carries a DepthTexture");
            renderer.copy_depth_texture(&self.depth, &previous_depth);
        }

        // `RendererUtils.restoreRendererState( renderer, _rendererState )`.
        drop(renderer);
        true
    }
}

/// `property( 'vec4' )` twice — `colorOutput` and `lockOutput`, which both
/// materials assign and their shared `outputStruct( colorOutput, lockOutput )`
/// writes to `@location( 0 )` and `@location( 1 )`.
fn output_properties() -> (NodeRef, NodeRef) {
    (
        property("TAAUColorOutput", Type::Vec4),
        property("TAAULockOutput", Type::Vec4),
    )
}

/// `this._seedMaterial`: the beauty buffer sampled bilinearly at output UVs,
/// and a zero lock.
fn seed_material(
    beauty: &Texture,
    color_output: NodeRef,
    lock_output: NodeRef,
) -> MeshBasicNodeMaterial {
    let fragment = block(
        vec![
            color_output.assign(texture_uv(beauty, uv())),
            lock_output.assign(float(0.0)),
        ],
        // `return vec4( 0 )` — three's placeholder colour.
        vec4(0.0, 0.0, 0.0, 0.0),
    );
    let mut material = MeshBasicNodeMaterial::new();
    material.name = "TAAU.seed";
    material.color_node = Some(fragment);
    material.output_node = Some(output_struct(vec![color_output, lock_output]));
    material
}

/// What `setup()`'s `resolve` `Fn()` closes over.
struct ResolveInputs<'a> {
    beauty: &'a Texture,
    depth: &'a DepthTexture,
    velocity: &'a Texture,
    /// `historyNode` — `texture( history.textures[ 0 ] )`.
    history: &'a Texture,
    /// `lockNode` — `texture( history.textures[ 1 ] )`.
    lock: &'a Texture,
    previous_depth: &'a DepthTexture,
    jitter: NodeRef,
    near_far: NodeRef,
    world_inverse: NodeRef,
    previous_world: NodeRef,
    previous_projection_inverse: NodeRef,
    color_output: NodeRef,
    lock_output: NodeRef,
}

/// `this._resolveMaterial`.
fn resolve_material(inputs: &ResolveInputs) -> MeshBasicNodeMaterial {
    let mut material = MeshBasicNodeMaterial::new();
    material.name = "TAAU.resolve";
    material.color_node = Some(resolve_node(inputs));
    material.output_node = Some(output_struct(vec![
        inputs.color_output.clone(),
        inputs.lock_output.clone(),
    ]));
    material
}

/// `vec2( 0.5 ).add( this._jitterOffset )` — where in its texel an input
/// sample was rendered. Three builds a fresh node at each use, so each is
/// spelled out in place.
fn sample_center(jitter: &NodeRef) -> NodeRef {
    vec2(0.5, 0.5).add(jitter.clone())
}

/// The 9-tap reconstruction's accumulators: `sumColor`, `sumWeight`,
/// `moment1`, `moment2`, and the statements that fill them.
struct Reconstruction {
    statements: Vec<NodeRef>,
    sum_color: NodeRef,
    sum_weight: NodeRef,
    moment1: NodeRef,
    moment2: NodeRef,
}

/// The 9-tap Blackman-Harris (Gaussian approximation) reconstruction of the
/// current frame, gathering the moments for the variance clip on the way.
#[inline(never)]
fn reconstruct(
    beauty: &Texture,
    p_in: &NodeRef,
    closest_tap: &NodeRef,
    jitter: &NodeRef,
) -> Reconstruction {
    let sum_color = to_var(None, vec4(0.0, 0.0, 0.0, 0.0));
    let sum_weight = to_var(None, float(0.0));
    let moment1 = to_var(None, vec4(0.0, 0.0, 0.0, 0.0));
    let moment2 = to_var(None, vec4(0.0, 0.0, 0.0, 0.0));
    let mut statements = vec![
        sum_color.clone(),
        sum_weight.clone(),
        moment1.clone(),
        moment2.clone(),
    ];
    for (x, y) in TAP_OFFSETS {
        let tap = closest_tap.add(ivec2(int(x), int(y)));
        let tap_center = tap.to(Type::Vec2).add(sample_center(jitter));
        let delta = p_in.sub(tap_center);
        let w = exp(delta.dot(delta.clone()).mul(-2.29));
        // `max()` keeps a NaN from spreading.
        let c = texture_load(beauty, tap).max(0.0);
        statements.push(sum_color.add_assign(c.mul(w.clone())));
        statements.push(sum_weight.add_assign(w));
        statements.push(moment1.add_assign(c.clone()));
        statements.push(moment2.add_assign(c.pow2()));
    }
    Reconstruction {
        statements,
        sum_color,
        sum_weight,
        moment1,
        moment2,
    }
}

/// `setup()`'s `resolve` `Fn()`, the resolve material's `colorNode`. It
/// writes the two output properties and returns three's placeholder
/// `vec4( 0 )`.
#[inline(never)]
fn resolve_node(inputs: &ResolveInputs) -> NodeRef {
    let uv_node = uv();
    // `vec2( this.beautyNode.size() )`. Three's builder declares it once, as
    // `let`, because three expressions read it; the port's has to be told.
    let input_size_f = to_const(
        None,
        texture_size(TextureSource::Texture2D(inputs.beauty.clone()), int(0)).to(Type::Vec2),
    );

    // The output pixel's centre in input-pixel coordinates. The input
    // sample at texel ( m, n ) was rendered at ( m + 0.5 + jitter ), so the
    // closest tap is the rounded difference.
    let p_in = uv_node.mul(input_size_f.clone());
    let closest_tap_f = p_in.sub(sample_center(&inputs.jitter)).round();
    let closest_tap = closest_tap_f.to(Type::IVec2);

    // Depth dilation around the closest input tap.
    let layout = current_depth_struct();
    let current_depth = sample_current_depth(inputs.depth, &closest_tap_f, &layout);
    let closest_depth = struct_get(&current_depth, &layout, "closestDepth");
    let closest_position_texel = struct_get(&current_depth, &layout, "closestPositionTexel");
    let farthest_depth = struct_get(&current_depth, &layout, "farthestDepth");

    // Reproject with the velocity at the dilated depth tap.
    let offset_uv = texture_load(inputs.velocity, closest_position_texel)
        .xy()
        .mul(vec2(0.5, -0.5));
    let history_uv = uv_node.sub(offset_uv);
    let previous_depth = sample_previous_depth(
        inputs.previous_depth,
        &history_uv,
        &inputs.previous_projection_inverse,
        &inputs.previous_world,
        &inputs.world_inverse,
        &inputs.near_far,
    );

    // History validity.
    let is_valid_uv =
        all(history_uv.greater_than_equal(0.0)).and(all(history_uv.less_than_equal(1.0)));
    let is_edge = farthest_depth
        .sub(closest_depth.clone())
        .greater_than(EDGE_DEPTH_DIFF);
    let is_disocclusion = closest_depth
        .sub(previous_depth.clone())
        .greater_than(DEPTH_THRESHOLD);
    let has_valid_history = is_valid_uv.and(is_edge.or(is_disocclusion.not()));

    let taps = reconstruct(inputs.beauty, &p_in, &closest_tap, &inputs.jitter);
    let mut statements = taps.statements;
    // After the four accumulators, where three's `let` lands.
    statements.insert(4, input_size_f.clone());
    let current_color = taps.sum_color.div(taps.sum_weight.max(1e-5));

    // Variance clipping with the moments just gathered.
    let n = float(TAP_OFFSETS.len() as f64);
    let mean = taps.moment1.div(n.clone());
    let motion_factor = length(uv_node.sub(history_uv.clone()).mul(input_size_f))
        .div(MAX_VELOCITY_LENGTH)
        .saturate();
    let variance_gamma = mix(0.5, 1.0, motion_factor.one_minus().pow2());
    let variance = taps
        .moment2
        .div(n)
        .sub(mean.pow2())
        .max(0.0)
        .sqrt()
        .mul(variance_gamma);
    let min_color = mean.sub(variance.clone());
    let max_color = mean.add(variance);

    let history_color = texture_sample(inputs.history, history_uv);
    let clipped_history_color = call(
        &clip_aabb(),
        vec![
            mean.clamp(min_color.clone(), max_color.clone()),
            history_color.clone(),
            min_color,
            max_color,
        ],
    );

    // The lock: thin, high-contrast features keep their history, gated by a
    // two-sided depth change.
    let current_luma = luminance(current_color.rgb());
    let mean_luma = to_const(None, luminance(mean.rgb()));
    statements.push(mean_luma.clone());
    let thin_feature = smoothstep(
        0.0,
        0.2,
        current_luma.sub(mean_luma.clone()).abs().div(mean_luma),
    );
    let is_depth_changed = closest_depth
        .sub(previous_depth)
        .abs()
        .greater_than(DEPTH_THRESHOLD);
    let can_lock = is_valid_uv.and(is_depth_changed.not());
    let gated_thin_feature = can_lock.select(thin_feature, float(0.0));
    let decay = is_disocclusion.select(float(0.0), float(0.5));
    let lock_history = texture_sample(inputs.lock, uv_node);
    let lock = max(gated_thin_feature, lock_history.x().mul(decay)).saturate();
    let locked_history_color = mix(clipped_history_color, history_color, lock.clone());

    // The current frame's weight: a low baseline, biased towards the current
    // frame under motion, and all of it without a valid history.
    let current_weight = to_var(None, float(CURRENT_FRAME_WEIGHT));
    statements.push(current_weight.clone());
    statements.push(current_weight.assign(
        has_valid_history.select(current_weight.add(motion_factor).saturate(), float(1.0)),
    ));

    let output = call(
        &flicker_reduction(),
        vec![current_color, locked_history_color, current_weight],
    );
    statements.push(inputs.color_output.assign(output));
    statements.push(inputs.lock_output.assign(lock));

    // `return vec4( 0 )` — three's placeholder colour.
    block(statements, vec4(0.0, 0.0, 0.0, 0.0))
}
