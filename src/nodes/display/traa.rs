//! Port of `three.js/examples/jsm/tsl/display/TRAANode.js` — temporal
//! reprojection anti-aliasing — and of the four helpers in
//! `examples/jsm/tsl/utils/TAAUtils.js` it is built from.
//!
//! Every frame the camera is jittered by a sub-pixel Halton offset, the scene
//! is rendered once, and a resolve quad blends the new frame into a history
//! texture reprojected along the scene's `velocity` attachment. Over a few
//! frames the jitter averages into an anti-aliased image.
//!
//! Three's node is a `TempNode` with `updateBeforeType = FRAME`; so is this
//! one ([`TraaState`] implements [`NodeUpdate`] and is registered as the
//! updater of the resolve texture, the way `passTexture( this, … )` makes it
//! one in three). The jitter is the part that cannot live in the node itself:
//! three installs it as `OnBeforeRenderPipeline` / `OnAfterRenderPipeline`
//! callbacks from `setup()`, guarded by `renderPipelineState.viewOffsetOwner`
//! so a pipeline with two TRAA nodes jitters once. The port has no
//! `setup()`-time access to the pipeline, so [`TraaNode::attach`] does the
//! same with [`RenderPipeline::on_before_render`] and the same guard.
//!
//! **Where the previous frame lives.** Two kinds of "previous":
//!
//! - the velocity attachment's previous matrices belong to the global
//!   `velocity` node (#163), which TRAA only tells to ignore the jitter
//!   ([`Renderer::set_velocity_projection_matrix`]);
//! - TRAA's own: the camera's previous world matrix and previous inverse
//!   projection, which roll over at the top of [`TraaState::update_before`]
//!   — once per frame, before the new values are written — exactly where
//!   three copies `_cameraWorldMatrix` into `_previousCameraWorldMatrix`.
//!   The history colour and depth are copied at the bottom of the same call,
//!   after the resolve has read them.
//!
//! **Order within a frame.** Three's `TRAANode.setup()` builds the depth,
//! velocity and beauty nodes before itself, so the scene pass is earlier in
//! the frame's update-before list and has rendered by the time TRAA's
//! `updateBefore()` runs (three's own command dump: scene pass, then the
//! history restart copy, the resolve, and the two history copies). The port
//! asks for the pass explicitly at the top of `update_before()`; the frame
//! guard makes the pass's own later reach a no-op.
//!
//! Not ported: an orthographic camera (`viewZToOrthographicDepth`), a
//! logarithmic or reversed depth buffer, a beauty node that is an `RTTNode`
//! rather than a pass attachment, and a `velocity` other than the global one
//! (`builder.context.velocity`).

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use crate::cameras::{PerspectiveCamera, RenderCamera};
use crate::materials::MeshBasicNodeMaterial;
use crate::math::{Color, Matrix4};
use crate::nodes::frame::register_texture_update;
use crate::nodes::node::{SettableValue, StructLayout, StructMember, TextureSource, Type};
use crate::nodes::tsl::{
    all, block, call, depth_texture_load, depth_texture_sample, float, get_view_position, if_then,
    int, ivec2, length, luminance, max, mix, shader_fn, struct_get, struct_new, struct_type,
    texture_load, texture_load_offset, texture_sample, texture_size, texture_uv, to_const, to_var,
    uniform_settable, uv, vec2, vec4_join, view_z_to_perspective_depth,
};
use crate::nodes::{NodeRef, NodeUpdate, NodeUpdateType};
use crate::objects::QuadMesh;
use crate::renderer::{RenderPipeline, RenderTarget, RenderTargetOptions, Renderer};
use crate::textures::{DepthTexture, Texture, TextureFilter, TextureType};

/// `TRAANode.depthThreshold`: how far the current depth may sit in front of
/// the reprojected one before the history counts as disoccluded.
const DEPTH_THRESHOLD: f64 = 0.0005;
/// `TRAANode.edgeDepthDiff`: a 3×3 depth range wider than this is an edge,
/// where disocclusion is not trusted.
const EDGE_DEPTH_DIFF: f64 = 0.001;
/// `TRAANode.maxVelocityLength`, in pixels: the motion at which the current
/// frame's weight saturates.
const MAX_VELOCITY_LENGTH: f64 = 128.0;
/// `_haltonOffsets.length`.
const JITTER_COUNT: usize = 32;

/// `computeHaltonOffsets( 32 )` — `[ halton( i + 1, 2 ), halton( i + 1, 3 ) ]`.
fn halton_offsets() -> [(f64, f64); JITTER_COUNT] {
    std::array::from_fn(|i| (halton(i as u32 + 1, 2), halton(i as u32 + 1, 3)))
}

/// `TAAUtils.js`' `halton( index, base )`, in the same operation order so the
/// doubles are the same bits.
fn halton(mut index: u32, base: u32) -> f64 {
    let mut fraction = 1.0;
    let mut result = 0.0;
    while index > 0 {
        fraction /= base as f64;
        result += fraction * (index % base) as f64;
        index /= base;
    }
    result
}

fn mat4_values(m: &Matrix4) -> Vec<f64> {
    m.elements.to_vec()
}

/// `traa( beautyNode, depthNode, velocityNode, camera )`.
///
/// As with the other display nodes, the inputs are the textures three's
/// `convertToTexture()` / `getTextureNode()` would hand over: the scene pass's
/// `output` attachment, its depth texture and its `velocity` attachment. The
/// pass must have an MRT with `velocity` on it, and the camera must be the
/// pass's.
pub fn traa(
    beauty: &Texture,
    depth: &DepthTexture,
    velocity: &Texture,
    camera: Rc<RefCell<PerspectiveCamera>>,
) -> TraaNode {
    TraaNode::new(beauty, depth, velocity, camera)
}

/// `TRAANode` — see the module docs.
pub struct TraaNode(Rc<TraaState>);

/// The node's state, shared with the renderer's update-before registry.
pub(crate) struct TraaState {
    /// `this.beautyNode`'s texture, whose pass renders the scene.
    beauty: Texture,
    /// `this.depthNode.value`, the pass's depth attachment.
    depth: DepthTexture,
    /// `this.camera`.
    camera: Rc<RefCell<PerspectiveCamera>>,
    /// `this._historyRenderTarget`, with its `DepthTexture`. Never drawn
    /// into: both attachments are filled by copies.
    history: RenderTarget,
    /// `this._resolveRenderTarget`.
    resolve: RenderTarget,
    /// `_quadMesh` with `this._resolveMaterial`, rebuilt when
    /// `useSubpixelCorrection` changes.
    quad: RefCell<QuadMesh>,
    /// `this.useSubpixelCorrection`.
    use_subpixel_correction: Cell<bool>,
    /// `this.velocityNode`'s texture, kept for a rebuild.
    velocity: Texture,
    /// The node halves of the four uniforms, kept for a rebuild.
    uniform_nodes: UniformNodes,
    /// `this._textureNode` — `passTexture( this, resolve.texture )`.
    node: NodeRef,
    /// `this._jitterIndex`.
    jitter_index: Cell<usize>,
    /// `this._cameraNearFar`.
    near_far: SettableValue,
    /// `this._cameraWorldMatrix`. A uniform in three that the resolve shader
    /// never reads; here only the value that rolls into
    /// [`previous_world`](Self::previous_world).
    world: Cell<Matrix4>,
    /// `this._cameraWorldMatrixInverse`.
    world_inverse: SettableValue,
    /// `this._cameraProjectionMatrixInverse` — like [`world`](Self::world),
    /// read only to roll over.
    projection_inverse: Cell<Matrix4>,
    /// `this._previousCameraWorldMatrix`.
    previous_world: SettableValue,
    /// `this._previousCameraProjectionMatrixInverse`.
    previous_projection_inverse: SettableValue,
}

impl TraaNode {
    /// `new TRAANode( beautyNode, depthNode, velocityNode, camera )`, with
    /// the material `setup()` gives it.
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
        // depthTexture: new DepthTexture() } )`.
        let history = RenderTarget::new_with_options(1, 1, options())
            .expect("three-rs: the TRAA history target is a colour type");
        history.set_depth_texture(DepthTexture::new());
        history.set_copy_destination();
        let resolve = RenderTarget::new_with_options(1, 1, options())
            .expect("three-rs: the TRAA resolve target is a colour type");
        // The history depth is written by a copy out of the pass's depth.
        depth.set_copyable();
        history
            .depth_texture()
            .expect("three-rs: set just above")
            .set_copyable();

        let (near_far, near_far_value) = uniform_settable(Type::Vec2, vec![0.0, 0.0]);
        let identity = mat4_values(&Matrix4::identity());
        let (world_inverse, world_inverse_value) = uniform_settable(Type::Mat4, identity.clone());
        let (previous_world, previous_world_value) = uniform_settable(Type::Mat4, identity.clone());
        let (previous_projection_inverse, previous_projection_inverse_value) =
            uniform_settable(Type::Mat4, identity);

        let uniform_nodes = UniformNodes {
            near_far,
            world_inverse,
            previous_world,
            previous_projection_inverse,
        };
        let material = resolve_material(beauty, depth, velocity, &history, &uniform_nodes, true);

        let node = to_var(None, texture_uv(&resolve.texture(), uv()));

        let state = Rc::new(TraaState {
            beauty: beauty.clone(),
            depth: depth.clone(),
            camera,
            history,
            resolve,
            quad: RefCell::new(QuadMesh::new(material)),
            use_subpixel_correction: Cell::new(true),
            velocity: velocity.clone(),
            uniform_nodes,
            node,
            jitter_index: Cell::new(0),
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

    /// `traaNode.getTextureNode()` — the resolved frame, for the graph
    /// downstream (usually the pipeline's `outputNode`).
    pub fn node(&self) -> NodeRef {
        self.0.node.clone()
    }

    /// The `TRAA.resolve` quad material, for `examples/dump_wgsl.rs` and the
    /// dump gate.
    #[doc(hidden)]
    pub fn quad_material(&self) -> MeshBasicNodeMaterial {
        self.0.quad.borrow().material.clone()
    }

    /// `traaNode.useSubpixelCorrection = value` — whether the minimum
    /// weight of the current frame rises with how sub-pixel the velocity
    /// is. On by default; `webgpu_postprocessing_ao` turns it off. Three
    /// reads the flag in `setup()`, so the resolve shader is rebuilt here.
    pub fn set_use_subpixel_correction(&self, on: bool) {
        let state = &self.0;
        if state.use_subpixel_correction.replace(on) == on {
            return;
        }
        state.quad.borrow_mut().material = resolve_material(
            &state.beauty,
            &state.depth,
            &state.velocity,
            &state.history,
            &state.uniform_nodes,
            on,
        );
    }

    /// The `OnBeforeRenderPipeline` / `OnAfterRenderPipeline` half of
    /// `TRAANode.setup()`: jitter the camera before the pipeline renders,
    /// clear the jitter after. Call once, on the pipeline whose output this
    /// node feeds.
    ///
    /// Three's `viewOffsetOwner` guard is kept: when the pipeline already has
    /// an owner (a second TRAA node), nothing is installed and that node's
    /// jitter serves both.
    pub fn attach(&self, pipeline: &mut RenderPipeline) {
        if !pipeline.claim_view_offset() {
            return;
        }
        let before = self.0.clone();
        pipeline.on_before_render(Box::new(move |renderer: &mut Renderer| {
            let (width, height) = renderer.drawing_buffer_size();
            before.set_view_offset(renderer, width, height);
        }));
        let after = self.0.clone();
        pipeline.on_after_render(Box::new(move |renderer: &mut Renderer| {
            after.clear_view_offset(renderer);
        }));
    }
}

impl TraaState {
    /// `TRAANode.setViewOffset( width, height )`.
    fn set_view_offset(&self, renderer: &mut Renderer, width: u32, height: u32) {
        let mut camera = self.camera.borrow_mut();
        // Save the original, unjittered projection matrix for the velocity
        // pass.
        camera.update_projection_matrix();
        renderer.set_velocity_projection_matrix(Some(camera.projection_matrix));

        let (jitter_x, jitter_y) = halton_offsets()[self.jitter_index.get()];
        let (width, height) = (width as f64, height as f64);
        camera.set_view_offset(width, height, jitter_x - 0.5, jitter_y - 0.5, width, height);
    }

    /// `TRAANode.clearViewOffset()`.
    fn clear_view_offset(&self, renderer: &mut Renderer) {
        self.camera.borrow_mut().clear_view_offset();
        renderer.set_velocity_projection_matrix(None);
        self.jitter_index
            .set((self.jitter_index.get() + 1) % JITTER_COUNT);
    }

    /// `TRAANode.setSize( width, height )`.
    fn set_size(&self, width: u32, height: u32) {
        self.history.set_size(width, height);
        self.resolve.set_size(width, height);
    }
}

impl NodeUpdate for TraaState {
    /// `this.updateBeforeType = NodeUpdateType.FRAME`.
    fn update_before_type(&self) -> NodeUpdateType {
        NodeUpdateType::Frame
    }

    /// `TRAANode.updateBefore( frame )`.
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

        // Keep the TRAA in sync with the dimensions of the beauty node.
        let (width, height) = self.beauty.size();

        // `_rendererState = RendererUtils.resetRendererState( renderer, … )`.
        let previous_target = renderer.render_target();
        let previous_mrt = renderer.mrt();
        let previous_auto_clear = renderer.auto_clear;
        let previous_clear_color = renderer.clear_color();
        let previous_clear_alpha = renderer.clear_alpha();
        renderer.set_mrt(None);
        renderer.set_clear_color(Color::new(0.0, 0.0, 0.0), 1.0);
        renderer.auto_clear = true;

        let needs_restart = self.history.size() != (width, height);
        self.set_size(width, height);

        // Every time the dimensions change the history needs fresh data.
        if needs_restart {
            // Make sure the targets are allocated after the resize, which
            // dropped their GPU textures.
            renderer.init_render_target(&self.history);
            renderer.init_render_target(&self.resolve);
            // Seed the history with the beauty buffer, or the frames after a
            // resize fade in from the black the history was cleared with.
            renderer.copy_render_texture(&self.beauty, &self.history.texture());
        }

        // Resolve.
        renderer.set_render_target(Some(self.resolve.clone()));
        renderer.render_quad(&self.quad.borrow());
        renderer.set_render_target(None);

        // Update the history.
        renderer.copy_render_texture(&self.resolve.texture(), &self.history.texture());

        // Copy the current depth to the previous depth buffer — only when the
        // history matches the drawing buffer, and so the scene's depth.
        if self.history.size() == renderer.drawing_buffer_size() {
            let history_depth = self
                .history
                .depth_texture()
                .expect("three-rs: the history target carries a DepthTexture");
            renderer.copy_depth_texture(&self.depth, &history_depth);
        }

        // `RendererUtils.restoreRendererState( renderer, _rendererState )`.
        renderer.set_render_target(previous_target);
        renderer.set_mrt(previous_mrt);
        renderer.set_clear_color(previous_clear_color, previous_clear_alpha);
        renderer.auto_clear = previous_auto_clear;
        true
    }
}

/// The node halves of `this._cameraNearFar`, `this._cameraWorldMatrixInverse`,
/// `this._previousCameraWorldMatrix` and
/// `this._previousCameraProjectionMatrixInverse`.
struct UniformNodes {
    near_far: NodeRef,
    world_inverse: NodeRef,
    previous_world: NodeRef,
    previous_projection_inverse: NodeRef,
}

/// `this._resolveMaterial` with `setup()`'s `colorNode`.
fn resolve_material(
    beauty: &Texture,
    depth: &DepthTexture,
    velocity: &Texture,
    history: &RenderTarget,
    uniforms: &UniformNodes,
    use_subpixel_correction: bool,
) -> MeshBasicNodeMaterial {
    let fragment = resolve_node(&ResolveInputs {
        beauty,
        depth,
        velocity,
        history: &history.texture(),
        // `this._previousDepthNode` starts as `texture( new DepthTexture(
        // 1, 1 ) )` and is pointed at the history depth after the first
        // copy. Before that copy the history depth is a fresh,
        // zero-initialised texture, which reads the same as the 1×1 dummy,
        // so the port points at it from the start.
        previous_depth: &history
            .depth_texture()
            .expect("three-rs: the history target carries a DepthTexture"),
        near_far: uniforms.near_far.clone(),
        world_inverse: uniforms.world_inverse.clone(),
        previous_world: uniforms.previous_world.clone(),
        previous_projection_inverse: uniforms.previous_projection_inverse.clone(),
        use_subpixel_correction,
    });
    let mut material = MeshBasicNodeMaterial::new();
    material.name = "TRAA.resolve";
    material.color_node = Some(fragment);
    material
}

/// What `resolve()` closes over.
struct ResolveInputs<'a> {
    beauty: &'a Texture,
    depth: &'a DepthTexture,
    velocity: &'a Texture,
    history: &'a Texture,
    previous_depth: &'a DepthTexture,
    near_far: NodeRef,
    world_inverse: NodeRef,
    previous_world: NodeRef,
    previous_projection_inverse: NodeRef,
    use_subpixel_correction: bool,
}

/// `TRAANode.setup()`'s `resolve` `Fn()`, the resolve material's
/// `colorNode`.
fn resolve_node(inputs: &ResolveInputs) -> NodeRef {
    let uv_node = uv();
    // Assumes all the buffers share the same size.
    let texture_size = texture_size(TextureSource::Texture2D(inputs.beauty.clone()), int(0));
    let position_texel = uv_node.mul(texture_size.clone());

    // Sample the closest and farthest depths in the current buffer.
    let layout = current_depth_struct();
    let current_depth = sample_current_depth(inputs.depth, &position_texel, &layout);
    let closest_depth = struct_get(&current_depth, &layout, "closestDepth");
    let closest_position_texel = struct_get(&current_depth, &layout, "closestPositionTexel");
    let farthest_depth = struct_get(&current_depth, &layout, "farthestDepth");

    // Convert the NDC offset to a UV offset.
    let offset_uv = texture_load(inputs.velocity, closest_position_texel)
        .xy()
        .mul(vec2(0.5, -0.5));

    // Sample the previous depth.
    let history_uv = uv_node.sub(offset_uv.clone());
    let previous_depth = sample_previous_depth(
        inputs.previous_depth,
        &history_uv,
        &inputs.previous_projection_inverse,
        &inputs.previous_world,
        &inputs.world_inverse,
        &inputs.near_far,
    );

    // The history is valid when the UV is in range and there is no
    // disocclusion, except on edges.
    let is_valid_uv =
        all(history_uv.greater_than_equal(0.0)).and(all(history_uv.less_than_equal(1.0)));
    let is_edge = farthest_depth
        .sub(closest_depth.clone())
        .greater_than(EDGE_DEPTH_DIFF);
    let is_disocclusion = closest_depth
        .sub(previous_depth)
        .greater_than(DEPTH_THRESHOLD);
    let has_valid_history = is_valid_uv.and(is_edge.or(is_disocclusion.not()));

    // Sample the current and previous colours.
    let current_color = texture_uv(inputs.beauty, uv_node.clone());
    let history_color = texture_sample(inputs.history, history_uv.clone());

    // Increase the weight towards the current frame under motion.
    let motion_factor = length(uv_node.sub(history_uv).mul(texture_size.clone()))
        .div(MAX_VELOCITY_LENGTH)
        .saturate();
    // A minimum weight.
    let current_weight = to_var(None, float(0.05));
    let mut statements = vec![current_weight.clone()];
    if inputs.use_subpixel_correction {
        // `useSubpixelCorrection`: raise the minimum weight towards the
        // current frame the more sub-pixel the velocity is.
        statements.push(
            current_weight
                .add_assign(call(&subpixel_correction(), vec![offset_uv, texture_size]).mul(0.25)),
        );
    }
    statements.push(current_weight.assign(has_valid_history.select(
        current_weight.add(motion_factor.clone()).saturate(),
        float(1.0),
    )));

    // Neighbourhood clipping, by variance. A reasonable gamma range is
    // [0.75, 2].
    let variance_gamma = mix(0.5, 1.0, motion_factor.one_minus().pow2());
    let clipped_history_color = variance_clipping(
        inputs.beauty,
        &position_texel,
        &current_color,
        &history_color,
        variance_gamma,
    );

    // Flicker reduction, by luminance weighting.
    block(
        statements,
        call(
            &flicker_reduction(),
            vec![current_color, clipped_history_color, current_weight],
        ),
    )
}

/// `struct( { closestDepth: 'float', closestPositionTexel: 'vec2',
/// farthestDepth: 'float' } )` — `currentDepthStruct`. Three names an
/// anonymous struct after its order of creation, and on the TRAA page it is
/// the first: `StructType0`.
fn current_depth_struct() -> Rc<StructLayout> {
    struct_type(
        "StructType0",
        vec![
            StructMember::new("closestDepth", Type::F32),
            StructMember::new("closestPositionTexel", Type::Vec2),
            StructMember::new("farthestDepth", Type::F32),
        ],
    )
}

/// `TAAUtils.sampleCurrentDepth( depthNode, positionTexel, cameraNearFar )`:
/// the closest and farthest depth of the 3×3 neighbourhood, and where the
/// closest one is. Three unrolls the two JS loops, x outer.
fn sample_current_depth(
    depth: &DepthTexture,
    position_texel: &NodeRef,
    layout: &Rc<StructLayout>,
) -> NodeRef {
    let closest_depth = to_var(None, float(2.0));
    let closest_position_texel = to_var(None, vec2(0.0, 0.0));
    let farthest_depth = to_var(None, float(-1.0));
    let mut statements = vec![
        closest_depth.clone(),
        closest_position_texel.clone(),
        farthest_depth.clone(),
    ];
    for x in -1..=1 {
        for y in -1..=1 {
            let neighbor = to_var(None, position_texel.add(vec2(x as f64, y as f64)));
            let depth = to_var(None, depth_texture_load(depth, neighbor.clone()));
            statements.push(neighbor.clone());
            statements.push(depth.clone());
            statements.push(if_then(
                depth.less_than(closest_depth.clone()),
                vec![
                    closest_depth.assign(depth.clone()),
                    closest_position_texel.assign(neighbor),
                ],
            ));
            statements.push(if_then(
                depth.greater_than(farthest_depth.clone()),
                vec![farthest_depth.assign(depth)],
            ));
        }
    }
    block(
        statements,
        struct_new(
            layout,
            vec![closest_depth, closest_position_texel, farthest_depth],
        ),
    )
}

/// `TAAUtils.samplePreviousDepth()` for a perspective camera: the history
/// depth at `uv`, reconstructed to a world position with the previous
/// camera, and projected back to a perspective depth with the current one.
fn sample_previous_depth(
    previous_depth: &DepthTexture,
    uv: &NodeRef,
    previous_projection_inverse: &NodeRef,
    previous_world: &NodeRef,
    world_inverse: &NodeRef,
    near_far: &NodeRef,
) -> NodeRef {
    let depth = depth_texture_sample(previous_depth, uv.clone());
    let position_view = get_view_position(uv.clone(), depth, previous_projection_inverse.clone());
    let position_world = previous_world
        .mul(vec4_join(vec![position_view, float(1.0)]))
        .xyz();
    let view_z = world_inverse
        .mul(vec4_join(vec![position_world, float(1.0)]))
        .z();
    view_z_to_perspective_depth(view_z, near_far.x(), near_far.y())
}

/// The `varianceClipping` `Fn()`: the mean and standard deviation of the 3×3
/// neighbourhood (taps clamped at zero so a NaN cannot spread), scaled by
/// `gamma`, and the history clipped to that box.
fn variance_clipping(
    beauty: &Texture,
    position_texel: &NodeRef,
    current_color: &NodeRef,
    history_color: &NodeRef,
    gamma: NodeRef,
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
    let moment1 = to_var(None, current_color.clone());
    let moment2 = to_var(None, current_color.pow2());
    let mut statements = vec![moment1.clone(), moment2.clone()];
    for (x, y) in OFFSETS {
        let neighbor =
            texture_load_offset(beauty, position_texel.clone(), ivec2(int(x), int(y))).max(0.0);
        statements.push(moment1.add_assign(neighbor.clone()));
        statements.push(moment2.add_assign(neighbor.pow2()));
    }

    let n = float((OFFSETS.len() + 1) as f64);
    let mean = moment1.div(n.clone());
    let variance = moment2.div(n).sub(mean.pow2()).max(0.0).sqrt().mul(gamma);
    let min_color = mean.sub(variance.clone());
    let max_color = mean.add(variance);

    block(
        statements,
        call(
            &clip_aabb(),
            vec![
                mean.clamp(min_color.clone(), max_color.clone()),
                history_color.clone(),
                min_color,
                max_color,
            ],
        ),
    )
}

/// The `subpixelCorrection` `Fn()`, with its layout: how sub-pixel the
/// velocity is, in [0, 1].
fn subpixel_correction() -> Rc<crate::nodes::node::FnDef> {
    shader_fn(
        Some("subpixelCorrection"),
        vec![("velocityUV", Type::Vec2), ("textureSize", Type::IVec2)],
        Type::F32,
        |args| {
            let velocity_texel = args[0].mul(args[1].to(Type::Vec2));
            let phase = velocity_texel.fract().abs();
            let weight = max(phase.clone(), phase.one_minus());
            weight.x().mul(weight.y()).one_minus().div(0.75)
        },
    )
}

/// `TAAUtils.clipAABB( currentColor, historyColor, minColor, maxColor )`.
fn clip_aabb() -> Rc<crate::nodes::node::FnDef> {
    shader_fn(
        Some("clipAABB"),
        vec![
            ("currentColor", Type::Vec4),
            ("historyColor", Type::Vec4),
            ("minColor", Type::Vec4),
            ("maxColor", Type::Vec4),
        ],
        Type::Vec4,
        |args| {
            let (current_color, history_color, min_color, max_color) =
                (&args[0], &args[1], &args[2], &args[3]);
            let p_clip = to_const(None, max_color.rgb().add(min_color.rgb()).mul(0.5));
            let e_clip = to_const(
                None,
                max_color.rgb().sub(min_color.rgb()).mul(0.5).add(1e-7),
            );
            let v_clip = to_const(
                None,
                history_color.sub(vec4_join(vec![p_clip.clone(), current_color.w()])),
            );
            let v_unit = to_const(None, v_clip.xyz().div(e_clip));
            let abs_unit = to_const(None, v_unit.abs());
            let max_unit = to_const(None, max(max(abs_unit.x(), abs_unit.y()), abs_unit.z()));
            max_unit.greater_than(1.0).select(
                vec4_join(vec![p_clip, current_color.w()]).add(v_clip.div(max_unit.clone())),
                history_color.clone(),
            )
        },
    )
}

/// `TAAUtils.flickerReduction( currentColor, historyColor, currentWeight )`:
/// blend in a tone-compressed space, each side weighted down by its
/// luminance.
fn flicker_reduction() -> Rc<crate::nodes::node::FnDef> {
    shader_fn(
        Some("flickerReduction"),
        vec![
            ("currentColor", Type::Vec4),
            ("historyColor", Type::Vec4),
            ("currentWeight", Type::F32),
        ],
        Type::Vec4,
        |args| {
            let (current_color, history_color, current_weight) = (&args[0], &args[1], &args[2]);
            let compress = |color: &NodeRef| {
                to_const(
                    None,
                    color.mul(float(1.0).div(max(max(color.x(), color.y()), color.z()).add(1.0))),
                )
            };
            let compressed_current = compress(current_color);
            let compressed_history = compress(history_color);

            let luminance_current = to_const(None, luminance(compressed_current.rgb()));
            let luminance_history = to_const(None, luminance(compressed_history.rgb()));

            let weight_current = to_const(None, current_weight.div(luminance_current.add(1.0)));
            let weight_history = to_const(
                None,
                current_weight.one_minus().div(luminance_history.add(1.0)),
            );

            current_color
                .mul(weight_current.clone())
                .add(history_color.mul(weight_history.clone()))
                .div(max(weight_current.add(weight_history), 0.00001))
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `computeHaltonOffsets( 32 )`'s first entries, as JS computes them.
    #[test]
    fn halton_offsets_match_three() {
        let offsets = halton_offsets();
        assert_eq!(offsets[0], (0.5, 1.0 / 3.0));
        assert_eq!(offsets[1], (0.25, 2.0 / 3.0));
        assert_eq!(offsets[2], (0.75, 1.0 / 9.0));
        assert_eq!(offsets[31].0, 1.0 / 64.0);
    }
}
