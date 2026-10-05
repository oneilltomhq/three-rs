//! Port of `three.js/examples/jsm/tsl/display/SSSNode.js` — screen-space
//! shadows.
//!
//! Every frame the node draws one full-screen quad into its own target: for
//! each pixel it unprojects the scene depth to a view position and marches
//! a short ray from it towards the main light, in screen space, one pixel
//! step at a time (scaled by `quality`). The first step whose depth-buffer
//! surface sits just in front of the ray — closer to the camera by less than
//! `thickness` — marks the pixel occluded, at `shadowIntensity`. The result
//! is `1 - occlusion`: white where lit, darker in the contact shadows that
//! a shadow map is too coarse to resolve. The pixels with nothing behind
//! them (depth 1) discard and keep the white clear.
//!
//! The page multiplies it into the light's colour in the scene pass with
//! `builtinShadowContext( sss, light )`, which the port spells
//! [`PassNode::set_context_shadow`](crate::renderer::PassNode::set_context_shadow).
//!
//! Three's node is a `Node` with `updateBeforeType = FRAME`; so is this one
//! ([`SssState`] implements [`NodeUpdate`] and is registered as the updater
//! of the SSS texture, the way `passTexture( this, … )` makes it one in
//! three).
//!
//! **The frame id is baked into the shader.** The constructor makes
//! `_frameId` a uniform, but `updateBefore()` then overwrites the property
//! with the plain number `frame.frameId` (or `0` without temporal
//! filtering), and the `Fn()` body that reads it runs when the quad
//! material is built, which is after that assignment. So three's
//! `rand( uv.add( this._frameId ) )` is `rand( uv + vec2( N ) )` for the
//! frame the material was last built on (`2.0` in the page's dump).
//!
//! Three rebuilds that material more often than the port does. `setup()`
//! runs again in every builder that sets up a material reading the SSS
//! texture (`PassTextureNode.setup()` puts the `SSSNode` in that builder's
//! node properties), and each run assigns a fresh `fragmentNode = sss()` and
//! sets `needsUpdate`, so the next `updateBefore()` rebuilds the quad with
//! the frame id of that frame. On the page that is the first frame (the
//! statue's and the ground's scene materials), again when the glTF arrives,
//! and again whenever the GUI's output switch makes a scene material set up
//! again. The port builds the quad material once, on the first
//! [`NodeUpdate::update_before`], and keeps it, so it bakes the first
//! frame's id. Only the phase of the `rand()` noise differs; turning
//! temporal filtering on or off later changes the `temporalOffset` uniform
//! in both, which is the same.
//!
//! Not ported, and absent from the API rather than ignored:
//!
//! * an `OrthographicCamera` — the node takes a [`PerspectiveCamera`], so
//!   the `orthographicDepthToViewZ` branch of `getViewZ` never applies;
//! * a logarithmic depth buffer (`logarithmicDepthToViewZ` →
//!   `viewZToPerspectiveDepth` in `sampleDepth`), which the port's renderer
//!   does not have;
//! * `this._material.contextNode = context( builder.getSharedContext() )`:
//!   the quad builds with an empty context. On the page `setup()` runs in
//!   the scene materials' builders, where the SSS read sits inside the
//!   `getShadow` hook, and `getSharedContext()` strips the material and
//!   the `getShadow` / `getAO` / `getGI` / `getUV` / `getOutput` hooks from
//!   that context; nothing left in it is read by the quad's fragment graph,
//!   so the result is still effectively empty.
//!
//! `RendererUtils.resetRendererState()`'s `setRenderObjectFunction( null )`
//! is not mirrored around the quad's render, as in `rtt.rs` and
//! `after_image.rs`: [`Renderer::render_quad`] never goes through the
//! render-object function.
//!
//! Divergences: three's target is `RedFormat`; the port has no one-channel
//! render target, so the quad writes its float into an `rgba8unorm` target
//! (so the `OutputStruct` member is a `vec4` splat of three's `f32`) and the
//! consumer reads `.x`. `getScreenPosition` (`PostProcessingUtils.js`) is a
//! private helper here.

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use crate::cameras::PerspectiveCamera;
use crate::core::ObjectRef;
use crate::lights::LightObject;
use crate::materials::MeshBasicNodeMaterial;
use crate::math::{Color, Matrix4};
use crate::nodes::frame::register_texture_update;
use crate::nodes::node::{SettableValue, Type, UniformGroup, UniformSource};
use crate::nodes::tsl::{
    abs, block, break_loop, discard, float, fract, get_view_position, if_then,
    interleaved_gradient_noise, loop_n, max, mix, pass_depth_texture_uv,
    perspective_depth_to_view_z, rand, screen_coordinate, texture_uv, to_const, to_var,
    transform_direction, uniform, uniform_settable, uv, vec2_join, vec4_join,
};
use crate::nodes::{NodeRef, NodeUpdate, NodeUpdateType};
use crate::objects::QuadMesh;
use crate::renderer::{RenderTarget, RenderTargetOptions, Renderer};
use crate::textures::{DepthTexture, TextureFilter, TextureType};

/// `_spatialOffsets`: the per-frame ray offset under temporal filtering.
const SPATIAL_OFFSETS: [f64; 4] = [0.0, 0.5, 0.25, 0.75];

/// `sss( depthNode, camera, mainLight )`.
///
/// `depth` is the texture three's `depthNode` samples — the page's
/// `prePass.getTextureNode( 'depth' )`. `camera` must be the pass's, and
/// `main_light` the light whose direction the rays march along (a
/// `DirectionalLight` in the page: the direction is its position minus its
/// target's).
pub fn sss(
    depth: &DepthTexture,
    camera: Rc<RefCell<PerspectiveCamera>>,
    main_light: &ObjectRef,
) -> SssNode {
    SssNode::new(depth, camera, main_light)
}

/// `SSSNode` — see the module docs.
pub struct SssNode {
    /// `this.maxDistance` — the longest shadow, in world units, `0.1` by
    /// default. Longer rays cost more steps.
    pub max_distance: SettableValue,
    /// `this.thickness` — how far in front of the ray, in view units, a
    /// surface may sit and still occlude it, `0.01` by default.
    pub thickness: SettableValue,
    /// `this.shadowIntensity` — the occlusion an occluded pixel gets, in
    /// `[0, 1]`, `1` by default.
    pub shadow_intensity: SettableValue,
    /// `this.quality` — the fraction of the ray's pixel length that is
    /// marched, clamped to `[0, 1]`, `0.5` by default.
    pub quality: SettableValue,
    state: Rc<SssState>,
}

/// The node's state, shared with the renderer's update-before registry.
pub(crate) struct SssState {
    /// `this.depthNode.value`, the pass's depth attachment.
    depth: DepthTexture,
    /// `this._camera`.
    camera: Rc<RefCell<PerspectiveCamera>>,
    /// `this._mainLight`.
    main_light: ObjectRef,
    /// `this._sssRenderTarget`.
    target: RenderTarget,
    /// `_quadMesh` with `this._material`, built on the first frame.
    quad: RefCell<Option<QuadMesh>>,
    /// `this._textureNode` — `passTexture( this, sssRenderTarget.texture )`.
    node: NodeRef,
    /// `this.resolutionScale`.
    resolution_scale: Cell<f64>,
    /// `this.useTemporalFiltering`.
    use_temporal_filtering: Cell<bool>,
    /// The uniforms the shader reads.
    uniforms: Uniforms,
    /// The settable halves of the private uniforms.
    camera_view_matrix: SettableValue,
    camera_projection_matrix: SettableValue,
    camera_projection_matrix_inverse: SettableValue,
    camera_near: SettableValue,
    camera_far: SettableValue,
    resolution: SettableValue,
    temporal_offset: SettableValue,
    light_position: SettableValue,
    light_target_position: SettableValue,
}

/// The node halves of every uniform the SSS `Fn()` reads.
struct Uniforms {
    max_distance: NodeRef,
    thickness: NodeRef,
    shadow_intensity: NodeRef,
    quality: NodeRef,
    camera_view_matrix: NodeRef,
    camera_projection_matrix: NodeRef,
    camera_projection_matrix_inverse: NodeRef,
    camera_near: NodeRef,
    camera_far: NodeRef,
    resolution: NodeRef,
    temporal_offset: NodeRef,
    /// `lightPosition( this._mainLight )`.
    light_position: NodeRef,
    /// `lightTargetPosition( this._mainLight )`.
    light_target_position: NodeRef,
}

fn mat4_values(m: &Matrix4) -> Vec<f64> {
    m.elements.to_vec()
}

/// A `vec3` uniform in the render group whose value the node writes:
/// `lightPosition( light )` / `lightTargetPosition( light )`, which three
/// keys on the light object and the port's renderer keys on the light's
/// index in the scene list, which a quad's render has none of.
fn render_vec3() -> (NodeRef, SettableValue) {
    let cell = SettableValue::new(vec![0.0, 0.0, 0.0]);
    let node = uniform(
        UniformSource::Settable(cell.clone()),
        Type::Vec3,
        UniformGroup::Render,
        None,
    );
    (node, cell)
}

impl SssNode {
    /// `new SSSNode( depthNode, camera, mainLight )`. The material `setup()`
    /// gives it is built on the first frame — see the module docs.
    pub fn new(
        depth: &DepthTexture,
        camera: Rc<RefCell<PerspectiveCamera>>,
        main_light: &ObjectRef,
    ) -> Self {
        // `new RenderTarget( 1, 1, { depthBuffer: false, format: RedFormat,
        // type: UnsignedByteType } )`, minus the `RedFormat`.
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
        .expect("three-rs: the SSS target is a colour type");

        let (max_distance, max_distance_value) = uniform_settable(Type::F32, vec![0.1]);
        let (thickness, thickness_value) = uniform_settable(Type::F32, vec![0.01]);
        let (shadow_intensity, shadow_intensity_value) = uniform_settable(Type::F32, vec![1.0]);
        let (quality, quality_value) = uniform_settable(Type::F32, vec![0.5]);
        let identity = mat4_values(&Matrix4::identity());
        let (camera_view_matrix, camera_view_matrix_value) =
            uniform_settable(Type::Mat4, identity.clone());
        let (camera_projection_matrix, camera_projection_matrix_value) =
            uniform_settable(Type::Mat4, identity.clone());
        let (camera_projection_matrix_inverse, camera_projection_matrix_inverse_value) =
            uniform_settable(Type::Mat4, identity);
        let (camera_near, camera_near_value) = uniform_settable(Type::F32, vec![0.0]);
        let (camera_far, camera_far_value) = uniform_settable(Type::F32, vec![0.0]);
        let (resolution, resolution_value) = uniform_settable(Type::Vec2, vec![0.0, 0.0]);
        let (temporal_offset, temporal_offset_value) = uniform_settable(Type::F32, vec![0.0]);
        let (light_position, light_position_value) = render_vec3();
        let (light_target_position, light_target_position_value) = render_vec3();

        let uniforms = Uniforms {
            max_distance,
            thickness,
            shadow_intensity,
            quality,
            camera_view_matrix,
            camera_projection_matrix,
            camera_projection_matrix_inverse,
            camera_near,
            camera_far,
            resolution,
            temporal_offset,
            light_position,
            light_target_position,
        };

        let node = to_var(None, texture_uv(&target.texture(), uv()));

        let state = Rc::new(SssState {
            depth: depth.clone(),
            camera,
            main_light: main_light.clone(),
            target,
            quad: RefCell::new(None),
            node,
            resolution_scale: Cell::new(1.0),
            use_temporal_filtering: Cell::new(false),
            uniforms,
            camera_view_matrix: camera_view_matrix_value,
            camera_projection_matrix: camera_projection_matrix_value,
            camera_projection_matrix_inverse: camera_projection_matrix_inverse_value,
            camera_near: camera_near_value,
            camera_far: camera_far_value,
            resolution: resolution_value,
            temporal_offset: temporal_offset_value,
            light_position: light_position_value,
            light_target_position: light_target_position_value,
        });
        register_texture_update(state.target.texture().id(), &state);
        Self {
            max_distance: max_distance_value,
            thickness: thickness_value,
            shadow_intensity: shadow_intensity_value,
            quality: quality_value,
            state,
        }
    }

    /// `sssNode.getTextureNode()` — the shadow term, in `.x`, at the quad's
    /// own uv: what the page's `SSS` view shows (`vec4( vec3( sssPass.r ),
    /// 1 )`).
    pub fn node(&self) -> NodeRef {
        self.state.node.clone()
    }

    /// `sssNode.getTextureNode().sample( uvNode )` — the shadow term at an
    /// explicit uv. A material drawn by the scene pass reads it at
    /// `screenUV`, since its own `uv()` is the mesh's; `.x` of it is the
    /// page's `sssSample`.
    pub fn sample(&self, coord: NodeRef) -> NodeRef {
        texture_uv(&self.state.target.texture(), coord)
    }

    /// `sssNode.resolutionScale = value` — the SSS target's size as a
    /// fraction of the drawing buffer's, `1` by default.
    pub fn set_resolution_scale(&self, scale: f64) {
        self.state.resolution_scale.set(scale);
    }

    /// `sssNode.useTemporalFiltering = value` — offset the rays' first step
    /// per frame, for a TRAA to average. Off by default. Turning it on or off
    /// later changes only the `temporalOffset` uniform, as in three. The
    /// frame id the shader's `rand()` adds is baked into the quad material;
    /// three re-bakes it whenever a consumer material sets up again and the
    /// port keeps the first frame's, which shifts only the noise's phase
    /// (see the module docs).
    pub fn set_use_temporal_filtering(&self, on: bool) {
        self.state.use_temporal_filtering.set(on);
    }

    /// The `SSS` quad material as it is built on a frame with id
    /// `frame_id` (`0` without temporal filtering). Its caller is
    /// `tests/display/materials.rs`, the display-quad list that the dump gate
    /// (`tests/nodes_display_wgsl.rs`) and `examples/dump_wgsl.rs` both
    /// include.
    #[doc(hidden)]
    pub fn quad_material(&self, frame_id: u64) -> MeshBasicNodeMaterial {
        material(&self.state.depth, &self.state.uniforms, frame_id)
    }
}

/// `this._material` with `setup()`'s `fragmentNode`, the SSS `Fn()` read on
/// the frame `frame_id`.
fn material(depth: &DepthTexture, uniforms: &Uniforms, frame_id: u64) -> MeshBasicNodeMaterial {
    let mut material = MeshBasicNodeMaterial::new();
    material.name = "SSS";
    material.fragment_node = Some(sss_node(depth, uniforms, frame_id));
    material
}

/// `getScreenPosition( viewPosition, projectionMatrix )` —
/// `PostProcessingUtils.js`: the screen uv a view-space position projects
/// to, y down.
fn get_screen_position(view_position: NodeRef, projection_matrix: NodeRef) -> NodeRef {
    let sample_clip_pos = projection_matrix.mul(vec4_join(vec![view_position, float(1.0)]));
    let sample_uv = to_var(
        None,
        sample_clip_pos
            .xy()
            .div(sample_clip_pos.w())
            .mul(0.5)
            .add(0.5),
    );
    vec2_join(vec![sample_uv.x(), sample_uv.y().one_minus()])
}

/// `SSSNode.setup()`'s `sss` `Fn()`.
fn sss_node(depth: &DepthTexture, u: &Uniforms, frame_id: u64) -> NodeRef {
    let uv_node = uv();

    // `sampleDepth` — `this.depthNode.sample( uv ).r`; no logarithmic depth.
    let sample_depth = |coord: NodeRef| pass_depth_texture_uv(depth, coord);
    // `getViewZ` — the perspective branch; see the module docs.
    let get_view_z = |depth: NodeRef| {
        perspective_depth_to_view_z(depth, u.camera_near.clone(), u.camera_far.clone())
    };

    let depth_value = to_var(None, sample_depth(uv_node.clone()));
    let discard_far = if_then(depth_value.greater_than_equal(1.0), vec![discard()]);

    // compute ray position and direction (in view-space)

    let ray_start_position = to_var(
        Some("rayStartPosition"),
        get_view_position(
            uv_node.clone(),
            depth_value.clone(),
            u.camera_projection_matrix_inverse.clone(),
        ),
    );
    let ray_direction = to_const(
        Some("rayDirection"),
        transform_direction(
            u.camera_view_matrix.clone(),
            u.light_position.sub(u.light_target_position.clone()),
        ),
    );
    let ray_end_position = to_const(
        Some("rayEndPosition"),
        ray_start_position.add(ray_direction.mul(u.max_distance.clone())),
    );

    // d0 and d1 are the start and maximum points of the ray in screen space
    let d0 = to_var(None, screen_coordinate());
    let d1 = to_var(
        None,
        get_screen_position(ray_end_position.clone(), u.camera_projection_matrix.clone())
            .mul(u.resolution.clone()),
    );

    // below variables are used to control the raymarching process

    // total length of the ray
    let total_len = to_var(None, d1.sub(d0.clone()).length());

    // offset in x and y direction
    let x_len = to_var(None, d1.x().sub(d0.x()));
    let y_len = to_var(None, d1.y().sub(d0.y()));

    // determine the larger delta: it decides how far to travel in x and y
    // each iteration and how many iterations cover the whole ray
    let total_step = to_const(
        None,
        max(abs(x_len.clone()), abs(y_len.clone()))
            .mul(u.quality.clamp(0.0, 1.0))
            .to_int(),
    );

    // step sizes in the x and y directions
    let x_span = to_var(None, x_len.div(total_step.to_float()));
    let y_span = to_var(None, y_len.div(total_step.to_float()));

    // compute noise based ray offset; `this._frameId` is the plain number
    // `updateBefore()` stored — see the module docs.
    let noise = interleaved_gradient_noise(screen_coordinate());
    let offset = to_const(
        Some("offset"),
        fract(noise.add(u.temporal_offset.clone())).add(rand(uv_node.add(float(frame_id as f64)))),
    );

    let occlusion = to_var(None, float(0.0));

    let march = loop_n("i", total_step.clone(), |i| {
        // advance on the ray by computing a new position in screen
        // coordinates
        // `float( i ).add( offset )` is built twice, as three writes it.
        let step = || i.to_float().add(offset.clone());
        let xy = to_var(
            None,
            vec2_join(vec![
                d0.x().add(x_span.mul(step())),
                d0.y().add(y_span.mul(step())),
            ]),
        );

        // stop processing if the new position lies outside of the screen
        let off_screen = if_then(
            xy.x()
                .less_than(0.0)
                .or(xy.x().greater_than(u.resolution.x()))
                .or(xy.y().less_than(0.0))
                .or(xy.y().greater_than(u.resolution.y())),
            vec![break_loop()],
        );

        // compute new uv, depth and viewZ for the next fragment
        let uv_node = xy.div(u.resolution.clone());
        let fragment_depth = to_const(None, sample_depth(uv_node));
        let fragment_view_z = to_const(Some("fragmentViewZ"), get_view_z(fragment_depth.clone()));

        let s = to_var(None, xy.sub(d0.clone()).length().div(total_len.clone()));
        let ray_position = mix(
            ray_start_position.clone(),
            ray_end_position.clone(),
            s.clone(),
        );

        // viewZ values are negative in three
        let depth_delta = to_const(None, ray_position.z().sub(fragment_view_z.clone()).negate());

        // check if the camera can't "see" the ray (ray depth must be larger
        // than the camera depth, so a positive depth delta)
        let occluded = if_then(
            depth_delta
                .greater_than(0.0)
                .and(depth_delta.less_than(u.thickness.clone())),
            vec![
                // mark as occluded
                occlusion.assign(u.shadow_intensity.clone()),
                break_loop(),
            ],
        );

        vec![
            xy,
            off_screen,
            fragment_depth,
            fragment_view_z,
            s,
            depth_delta,
            occluded,
        ]
    });

    let statements = vec![
        depth_value,
        discard_far,
        ray_start_position,
        ray_direction,
        ray_end_position,
        d0,
        d1,
        total_len,
        x_len,
        y_len,
        total_step,
        x_span,
        y_span,
        offset,
        occlusion.clone(),
        march,
    ];
    block(statements, occlusion.one_minus())
}

impl NodeUpdate for SssState {
    /// `this.updateBeforeType = NodeUpdateType.FRAME`.
    fn update_before_type(&self) -> NodeUpdateType {
        NodeUpdateType::Frame
    }

    /// `SSSNode.updateBefore( frame )`.
    fn update_before(&self, renderer: &mut Renderer) -> bool {
        // The pre-pass first, as `setup()` building the depth node before
        // itself orders it in three.
        //
        // The SSS is asked for from inside a pass whose
        // `builtinShadowContext` reads it, and a nested pass inherits the
        // outer context. The pre-pass feeds this node, so it cannot also
        // read it; the port lifts the context for the pre-pass's render. (Three's pre-pass would
        // bind the 1×1 placeholder on the first frame and last frame's SSS
        // after that, into an MRT whose `output` is the velocity, so nothing
        // it writes reads it either.)
        if let Some(pass) = crate::nodes::frame::texture_update(self.depth.id()) {
            let mut renderer = renderer.save_state();
            renderer.context_shadow = None;
            renderer.update_before_node(&pass);
        }

        // `setSize( renderer.getDrawingBufferSize() )`.
        let (width, height) = renderer.drawing_buffer_size();
        let scale = self.resolution_scale.get();
        let width = (scale * f64::from(width)).round() as u32;
        let height = (scale * f64::from(height)).round() as u32;
        self.resolution
            .set(vec![f64::from(width), f64::from(height)]);
        let (width, height) = (width.max(1), height.max(1));
        if self.target.size() != (width, height) {
            self.target.set_size(width, height);
        }

        // update temporal uniforms
        let frame_id = if self.use_temporal_filtering.get() {
            let frame_id = renderer.node_frame().frame_id;
            self.temporal_offset
                .set(vec![SPATIAL_OFFSETS[(frame_id % 4) as usize]]);
            frame_id
        } else {
            self.temporal_offset.set(vec![0.0]);
            0
        };

        // The uniforms three reads from the camera and the light objects
        // themselves.
        {
            let camera = self.camera.borrow();
            self.camera_view_matrix
                .set(mat4_values(&camera.matrix_world_inverse));
            self.camera_projection_matrix
                .set(mat4_values(&camera.projection_matrix));
            self.camera_projection_matrix_inverse
                .set(mat4_values(&camera.projection_matrix_inverse));
            self.camera_near.set(vec![camera.near]);
            self.camera_far.set(vec![camera.far]);
        }
        {
            let object = self.main_light.borrow();
            let position = LightObject::world_position(&object.matrix_world);
            self.light_position
                .set(vec![position.x, position.y, position.z]);
            let target = object
                .light()
                .map_or_else(Default::default, LightObject::target_world_position);
            self.light_target_position
                .set(vec![target.x, target.y, target.z]);
        }

        // Built once, on the first frame; three rebuilds it whenever a
        // consumer material sets up again, re-baking `this._frameId` — see
        // the module docs.
        let mut quad = self.quad.borrow_mut();
        let quad = quad
            .get_or_insert_with(|| QuadMesh::new(material(&self.depth, &self.uniforms, frame_id)));

        // `_rendererState = RendererUtils.resetRendererState( renderer, … )`.
        let mut renderer = renderer.reset_state();

        // clear: white, no shadow where the quad discards (the sky).
        renderer.set_clear_color(Color::new(1.0, 1.0, 1.0), 1.0);

        // sss
        renderer.set_render_target(Some(self.target.clone()));
        renderer.render_quad(quad);

        // `RendererUtils.restoreRendererState( renderer, _rendererState )`.
        drop(renderer);
        true
    }
}
