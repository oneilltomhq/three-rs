//! Port of `three.js/examples/jsm/tsl/display/GTAONode.js` — ground truth
//! ambient occlusion.
//!
//! Every frame the node draws one full-screen quad into its own `RedFormat`
//! target: for each pixel it unprojects the scene depth to a view position,
//! then for each of a few directions (a "slice" through the hemisphere
//! around the normal) marches a handful of steps outwards along the screen
//! and tracks the highest horizon the depth buffer puts in the way on either
//! side. The closed-form integral of the visible arc, weighted by the
//! normal's projection into the slice, is the occlusion. A 5×5 magic-square
//! noise texture rotates the slices per pixel, and with temporal filtering
//! on, the rotation and the step offset also change per frame so a TRAA
//! after it averages the pattern out.
//!
//! Three's node is a `TempNode` with `updateBeforeType = FRAME`; so is this
//! one ([`GtaoState`] implements [`NodeUpdate`] and is registered as the
//! updater of the AO texture, the way `passTexture( this, … )` makes it one
//! in three). The sample count is baked into the shader so the loops unroll
//! — three rebuilds the material when `samples` changes, and so does
//! [`GtaoNode::set_samples`].
//!
//! Not ported: `normalNode = null` (`getNormalFromDepth`, which the page
//! never uses — it feeds the pre-pass's packed normals), a logarithmic depth
//! buffer, and the two unused `distanceExponent` / `distanceFallOff`
//! uniforms three still declares.

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use crate::cameras::PerspectiveCamera;
use crate::materials::MeshBasicNodeMaterial;
use crate::math::{Color, Matrix4};
use crate::nodes::frame::register_texture_update;
use crate::nodes::node::{SettableValue, TextureSource, Type};
use crate::nodes::tsl::{
    abs, block, cross, depth_texture_gather, discard, dot, float, get_screen_position_from_clip,
    get_view_position, if_then, int, interleaved_gradient_noise, loop_options, mat3_join, max, mix,
    pass_depth_texture_uv, pi, rand, screen_coordinate, texture_sample, texture_size, texture_uv,
    to_const, to_var, uniform_settable, unpack_rgb_to_normal, uv, vec2_join, vec3, vec3_join,
    vec4_join,
};
use crate::nodes::{NodeRef, NodeUpdate, NodeUpdateType};
use crate::objects::QuadMesh;
use crate::renderer::{RenderTarget, RenderTargetOptions, Renderer};
use crate::textures::{DepthTexture, MinFilter, Texture, TextureFilter, TextureType, Wrapping};

/// `_temporalRotations`, in degrees: the slice rotation per frame under
/// temporal filtering.
const TEMPORAL_ROTATIONS: [f64; 6] = [60.0, 300.0, 180.0, 240.0, 120.0, 0.0];
/// `_spatialOffsets`: the per-frame step offset under temporal filtering.
const SPATIAL_OFFSETS: [f64; 4] = [0.0, 0.5, 0.25, 0.75];

/// `ao( depthNode, normalNode, camera )`.
///
/// As with the other display nodes, the inputs are the textures three's
/// `getTextureNode()` would hand over: the pre-pass's depth attachment and
/// its `output` attachment, which the page's MRT fills with `packNormalToRGB(
/// normalView )`. The port unpacks it itself — the page's `normalNode` is
/// `sample( ( uv ) => unpackRGBToNormal( prePass.getTextureNode().sample( uv )
/// ) )`, so this is what three generates. The camera must be the pass's.
pub fn ao(
    depth: &DepthTexture,
    normal: &Texture,
    camera: Rc<RefCell<PerspectiveCamera>>,
) -> GtaoNode {
    GtaoNode::new(depth, normal, camera)
}

/// `GTAONode` — see the module docs.
pub struct GtaoNode {
    /// `this.radius` — the world-space radius the horizons are searched
    /// within, `0.25` by default.
    pub radius: SettableValue,
    /// `this.thickness` — how deep, in view units, a sample may sit behind
    /// the fragment and still occlude it, `1` by default.
    pub thickness: SettableValue,
    /// `this.scale` — the exponent the occlusion is raised to, `1` by
    /// default.
    pub scale: SettableValue,
    state: Rc<GtaoState>,
}

/// The node's state, shared with the renderer's update-before registry.
pub(crate) struct GtaoState {
    /// `this.depthNode.value`, the pass's depth attachment.
    depth: DepthTexture,
    /// `this.normalNode`'s texture, whose pass renders the scene.
    normal: Texture,
    /// `this.camera`.
    camera: Rc<RefCell<PerspectiveCamera>>,
    /// `this._aoRenderTarget`.
    target: RenderTarget,
    /// `_quadMesh` with `this._material`, rebuilt when `samples` changes.
    quad: RefCell<QuadMesh>,
    /// `this._textureNode` — `passTexture( this, aoRenderTarget.texture )`.
    node: NodeRef,
    /// `this.resolutionScale`.
    resolution_scale: Cell<f64>,
    /// `this.useTemporalFiltering`.
    use_temporal_filtering: Cell<bool>,
    /// `this.samples.value`, baked into the shader.
    samples: Cell<u32>,
    /// `this._currentSamples` — what the quad's shader was built for.
    current_samples: Cell<u32>,
    /// The uniforms the shader reads, kept for a rebuild.
    uniforms: Uniforms,
    /// `this._noiseTexture`.
    noise: Texture,
    /// The settable halves of the private uniforms.
    resolution: SettableValue,
    resolution_scale_uniform: SettableValue,
    temporal_direction: SettableValue,
    temporal_offset: SettableValue,
    projection: SettableValue,
    projection_inverse: SettableValue,
}

/// The node halves of every uniform the AO `Fn()` reads.
struct Uniforms {
    radius: NodeRef,
    thickness: NodeRef,
    scale: NodeRef,
    resolution: NodeRef,
    resolution_scale: NodeRef,
    temporal_direction: NodeRef,
    temporal_offset: NodeRef,
    projection: NodeRef,
    projection_inverse: NodeRef,
}

fn mat4_values(m: &Matrix4) -> Vec<f64> {
    m.elements.to_vec()
}

impl GtaoNode {
    /// `new GTAONode( depthNode, normalNode, camera )`, with the material
    /// `setup()` gives it.
    pub fn new(
        depth: &DepthTexture,
        normal: &Texture,
        camera: Rc<RefCell<PerspectiveCamera>>,
    ) -> Self {
        // `new RenderTarget( 1, 1, { depthBuffer: false, format: RedFormat } )`.
        // The port has no `RedFormat`; the quad writes its float into an
        // `rgba8unorm` and the texture node reads `.x`.
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
        .expect("three-rs: the GTAO target is a colour type");

        let (radius, radius_value) = uniform_settable(Type::F32, vec![0.25]);
        let (thickness, thickness_value) = uniform_settable(Type::F32, vec![1.0]);
        let (scale, scale_value) = uniform_settable(Type::F32, vec![1.0]);
        let (resolution, resolution_value) = uniform_settable(Type::Vec2, vec![0.0, 0.0]);
        let (resolution_scale, resolution_scale_value) = uniform_settable(Type::F32, vec![0.0]);
        let (temporal_direction, temporal_direction_value) = uniform_settable(Type::F32, vec![0.0]);
        let (temporal_offset, temporal_offset_value) = uniform_settable(Type::F32, vec![0.0]);
        let identity = mat4_values(&Matrix4::identity());
        let (projection, projection_value) = uniform_settable(Type::Mat4, identity.clone());
        let (projection_inverse, projection_inverse_value) = uniform_settable(Type::Mat4, identity);
        let uniforms = Uniforms {
            radius,
            thickness,
            scale,
            resolution,
            resolution_scale,
            temporal_direction,
            temporal_offset,
            projection,
            projection_inverse,
        };

        let noise = generate_magic_square_noise(5);
        let samples = 16;
        let material = material(depth, normal, &noise, &uniforms, samples);
        let node = to_var(None, texture_uv(&target.texture(), uv()));

        let state = Rc::new(GtaoState {
            depth: depth.clone(),
            normal: normal.clone(),
            camera,
            target,
            quad: RefCell::new(QuadMesh::new(material)),
            node,
            resolution_scale: Cell::new(1.0),
            use_temporal_filtering: Cell::new(false),
            samples: Cell::new(samples),
            current_samples: Cell::new(samples),
            uniforms,
            noise,
            resolution: resolution_value,
            resolution_scale_uniform: resolution_scale_value,
            temporal_direction: temporal_direction_value,
            temporal_offset: temporal_offset_value,
            projection: projection_value,
            projection_inverse: projection_inverse_value,
        });
        register_texture_update(state.target.texture().id(), &state);
        Self {
            radius: radius_value,
            thickness: thickness_value,
            scale: scale_value,
            state,
        }
    }

    /// `gtaoNode.getTextureNode()` — the occlusion, in `.x`, for the graph
    /// downstream (the page multiplies it into the scene pass's materials
    /// through [`crate::renderer::PassNode::set_context_ao`]).
    pub fn node(&self) -> NodeRef {
        self.state.node.clone()
    }

    /// `gtaoNode.getTextureNode().sample( uvNode )` — the occlusion at an
    /// explicit uv. A material drawn by the scene pass reads it at
    /// `screenUV`, since its own `uv()` is the mesh's.
    pub fn sample(&self, coord: NodeRef) -> NodeRef {
        texture_uv(&self.state.target.texture(), coord)
    }

    /// `gtaoNode.resolutionScale = value` — the AO target's size as a
    /// fraction of the drawing buffer's, `1` by default. Below `1` the centre
    /// depth is read with a `textureGather` so the half-resolution rounding
    /// does not band.
    pub fn set_resolution_scale(&self, scale: f64) {
        self.state.resolution_scale.set(scale);
    }

    /// `gtaoNode.useTemporalFiltering = value` — rotate the slices and
    /// offset the steps per frame, for a TRAA to average. Off by default.
    pub fn set_use_temporal_filtering(&self, on: bool) {
        self.state.use_temporal_filtering.set(on);
    }

    /// `gtaoNode.samples.value = n` — the sample budget, `16` by default:
    /// three slices of `ceil( n / 3 )` steps below 30, five above. Baked
    /// into the shader, which is rebuilt before the next frame.
    pub fn set_samples(&self, samples: u32) {
        self.state.samples.set(samples.max(1));
    }

    /// The `GTAO` quad material, for `examples/dump_wgsl.rs` and the dump
    /// gate.
    #[doc(hidden)]
    pub fn quad_material(&self) -> MeshBasicNodeMaterial {
        self.state.quad.borrow().material.clone()
    }
}

/// `this._material` with `setup()`'s `fragmentNode`: the AO `Fn()` for a
/// given sample count.
fn material(
    depth: &DepthTexture,
    normal: &Texture,
    noise: &Texture,
    uniforms: &Uniforms,
    samples: u32,
) -> MeshBasicNodeMaterial {
    let mut material = MeshBasicNodeMaterial::new();
    material.name = "GTAO";
    material.fragment_node = Some(ao_node(depth, normal, noise, uniforms, samples));
    material
}

/// `GTAONode.setup()`'s `this._ao` `Fn()`.
fn ao_node(
    depth: &DepthTexture,
    normal: &Texture,
    noise: &Texture,
    u: &Uniforms,
    samples: u32,
) -> NodeRef {
    let uv_node = uv();

    // `sampleDepth` — `this.depthNode.sample( uv ).r`; no logarithmic depth.
    let sample_depth = |coord: NodeRef| pass_depth_texture_uv(depth, coord);
    // `sampleCenterDepth`: sidestep the nearest-rounding during depth access
    // for the unjittered centre pixel to avoid banding.
    let sample_center_depth = |coord: NodeRef| {
        let g = depth_texture_gather(depth, coord);
        g.x().min(g.y()).min(g.z().min(g.w()))
    };

    let depth_value = to_const(
        None,
        u.resolution_scale.less_than(1.0).select(
            sample_center_depth(uv_node.clone()),
            sample_depth(uv_node.clone()),
        ),
    );
    let discard_far = if_then(depth_value.greater_than_equal(1.0), vec![discard()]);

    let view_position = to_const(
        None,
        get_view_position(
            uv_node.clone(),
            depth_value.clone(),
            u.projection_inverse.clone(),
        ),
    );
    // `sampleNormal` — `this.normalNode.sample( uv ).rgb.normalize()`, with
    // the page's `unpackRGBToNormal` inside the node.
    let view_normal = to_const(
        None,
        unpack_rgb_to_normal(texture_uv(normal, uv_node.clone()))
            .xyz()
            .normalize(),
    );

    let radius = u.radius.clone();
    let inv_radius = to_const(None, radius.reciprocal());
    let view_dir = to_const(None, view_position.xyz().negate().normalize());
    let clip_position = to_const(
        None,
        u.projection
            .mul(vec4_join(vec![view_position.clone(), float(1.0)])),
    );

    let noise_resolution = texture_size(TextureSource::Texture2D(noise.clone()), int(0));
    let noise_uv = vec2_join(vec![uv_node.x(), uv_node.y().one_minus()])
        .mul(u.resolution.div(noise_resolution));
    let noise_texel = texture_sample(noise, noise_uv);
    let random_vec = noise_texel.xyz().mul(2.0).sub(1.0);
    let tangent = vec3_join(vec![random_vec.xy(), float(0.0)]).normalize();
    let bitangent = vec3_join(vec![tangent.y().mul(-1.0), tangent.x(), float(0.0)]);
    let kernel_matrix = mat3_join(vec![tangent, bitangent, vec3(0.0, 0.0, 1.0)]);

    // The sample count is baked into the shader so loop unrolling works.
    let directions: u32 = if samples < 30 { 3 } else { 5 };
    let steps = samples.div_ceil(directions);
    let inv_steps = 1.0 / f64::from(steps);

    let ao = to_var(None, float(0.0));

    // Per-step phase jitter for spatio-temporal decorrelation.
    let noise_jitter_idx = u.temporal_direction.mul(0.02);
    let step_jitter =
        interleaved_gradient_noise(screen_coordinate().add(u.temporal_offset.clone()))
            .add(rand(uv_node.add(noise_jitter_idx).mul(2.0).sub(1.0)));

    // Each iteration analyses one vertical "slice" of the 3D space around
    // the fragment.
    let slices = loop_options(
        "i",
        Type::I32,
        int(0),
        int(i64::from(directions)),
        "<",
        |i| {
            let angle = to_const(
                None,
                i.to_float()
                    .div(f64::from(directions))
                    .mul(pi())
                    .add(u.temporal_direction.clone()),
            );
            let sample_dir = to_const(
                None,
                kernel_matrix.mul(vec3_join(vec![angle.cos(), angle.sin(), float(0.0)])),
            );
            let clip_dir_radius = to_const(
                None,
                u.projection
                    .mul(vec4_join(vec![sample_dir.clone(), float(0.0)]))
                    .mul(radius.clone()),
            );

            let slice_bitangent = to_const(
                None,
                cross(sample_dir.clone(), view_dir.clone()).normalize(),
            );
            let slice_tangent = to_const(None, cross(slice_bitangent.clone(), view_dir.clone()));

            // Project the view normal onto the slice plane; the unnormalised
            // length is the foreshortening weight at slice integration.
            let proj_n_raw = to_const(
                None,
                view_normal
                    .sub(slice_bitangent.mul(dot(view_normal.clone(), slice_bitangent.clone()))),
            );
            let proj_n_len = to_const(None, proj_n_raw.length());
            let proj_n = to_const(None, proj_n_raw.div(max(proj_n_len.clone(), float(0.0001))));

            // γ — the angle of `projN` within the slice plane, signed by the
            // tangent direction.
            let n_sin = to_const(None, dot(proj_n.clone(), slice_tangent.clone()));
            let n_cos = to_const(None, dot(proj_n.clone(), view_dir.clone()).clamp(0.0, 1.0));
            let sign_n_sin = n_sin
                .greater_than_equal(0.0)
                .select(float(1.0), float(-1.0));
            let angle_n = to_const(None, sign_n_sin.mul(n_cos.acos()));

            let tangent_to_normal_in_slice =
                to_const(None, cross(proj_n.clone(), slice_bitangent.clone()));
            let cos_horizon = to_const(
                None,
                dot(view_dir.clone(), tangent_to_normal_in_slice.clone()),
            );
            let cos_horizons = to_var(
                None,
                vec2_join(vec![cos_horizon.clone(), cos_horizon.negate()]),
            );

            // One side of the slice's line: march from `clipPosition` by
            // `offset`, find the horizon, fold it into `horizon`.
            let march = |offset: NodeRef, horizon: NodeRef| {
                let screen_position = to_const(None, get_screen_position_from_clip(offset));
                let sample_depth_value = to_const(None, sample_depth(screen_position.clone()));
                let scene_view_position = to_const(
                    None,
                    get_view_position(
                        screen_position,
                        sample_depth_value,
                        u.projection_inverse.clone(),
                    ),
                );
                let view_delta = to_const(None, scene_view_position.sub(view_position.clone()));
                let len = to_const(None, view_delta.length());
                // Manual normalise guards against a zero-length delta.
                let s_h =
                    dot(view_dir.clone(), view_delta.clone()).div(max(len.clone(), float(0.0001)));
                // Sphere falloff: `( dist / radius )²` fades the sample's
                // horizon contribution back toward the prior horizon as it
                // approaches the radius boundary.
                let dist_fac = len.mul(inv_radius.clone()).min(1.0);
                let dist_fac_sq = dist_fac.mul(dist_fac.clone());
                if_then(
                    abs(view_delta.z()).less_than(u.thickness.clone()),
                    vec![horizon.assign(mix(
                        max(horizon.clone(), s_h),
                        horizon.clone(),
                        dist_fac_sq,
                    ))],
                )
            };

            // For each slice, the inner loop ray-marches to find the horizons.
            let steps_loop =
                loop_options("j", Type::I32, int(0), int(i64::from(steps)), "<", |j| {
                    // Quadratic step distribution (`sampleDist = t²`) concentrates
                    // samples in the near field.
                    let t = to_const(
                        None,
                        j.to_float()
                            .add(1.0)
                            .add(step_jitter.clone())
                            .mul(inv_steps),
                    );
                    let sample_dist = t.mul(t.clone());
                    let clip_offset = to_const(None, clip_dir_radius.mul(sample_dist));
                    vec![
                        march(clip_position.add(clip_offset.clone()), cos_horizons.x()),
                        march(clip_position.sub(clip_offset), cos_horizons.y()),
                    ]
                });

            // Cosine-weighted inner integral, closed-form (Activision GTAO
            // paper, Eq. 7): `−cos( 2h − γ ) + cos( γ ) + 2h sin( γ )` per
            // horizon, ½ for the integral and ½ for averaging the two. The
            // `+sampleDir` samples (`cosHorizons.x`) live on the −T side of
            // the slice, so `hPos` reads from `.y`.
            let h_pos = to_const(None, cos_horizons.y().acos());
            let h_neg = to_const(None, cos_horizons.x().acos().negate());
            let term = |h: &NodeRef| {
                h.mul(2.0)
                    .sub(angle_n.clone())
                    .cos()
                    .negate()
                    .add(n_cos.clone())
                    .add(h.mul(2.0).mul(n_sin.clone()))
            };
            let a = term(&h_pos).add(term(&h_neg)).mul(0.25);

            vec![
                angle,
                sample_dir,
                clip_dir_radius,
                slice_bitangent,
                slice_tangent,
                proj_n_raw,
                proj_n_len.clone(),
                proj_n,
                n_sin,
                n_cos,
                angle_n,
                tangent_to_normal_in_slice,
                cos_horizon,
                cos_horizons,
                steps_loop,
                h_pos,
                h_neg,
                // `|projN|` is the foreshortening weight.
                ao.add_assign(proj_n_len.mul(a)),
            ]
        },
    );

    let statements = vec![
        depth_value,
        discard_far,
        view_position,
        view_normal,
        inv_radius,
        view_dir,
        clip_position,
        ao.clone(),
        slices,
        ao.assign(ao.div(f64::from(directions)).clamp(0.0, 1.0)),
        ao.assign(ao.pow(u.scale.clone())),
    ];
    block(statements, ao)
}

/// `generateMagicSquareNoise( size )`: a `size`×`size` (odd) texture whose
/// texels are unit vectors at the angles a magic square orders, so every
/// `size`×`size` tile of the screen sees each rotation once.
fn generate_magic_square_noise(size: u32) -> Texture {
    let noise_size = if size.is_multiple_of(2) {
        size + 1
    } else {
        size
    };
    let magic_square = generate_magic_square(noise_size);
    let count = magic_square.len();
    let mut data = Vec::with_capacity(count * 4);
    for &i_ang in &magic_square {
        let angle = 2.0 * std::f64::consts::PI * f64::from(i_ang) / count as f64;
        let (x, y) = (angle.cos(), angle.sin());
        data.push(((x * 0.5 + 0.5) * 255.0) as u8);
        data.push(((y * 0.5 + 0.5) * 255.0) as u8);
        data.push(127);
        data.push(255);
    }
    // `new DataTexture( data, size, size )`: nearest filters, no mipmaps,
    // `flipY = false`; then `RepeatWrapping` both ways.
    let texture = Texture::new(noise_size, noise_size, Some(data));
    texture.set_flip_y(false);
    texture.set_generate_mipmaps(false);
    texture.set_min_filter(MinFilter::Nearest);
    texture.set_mag_filter(TextureFilter::Nearest);
    texture.set_wrapping(Wrapping::Repeat, Wrapping::Repeat);
    texture
}

/// `generateMagicSquare( size )` — the Siamese method, with three's exact
/// wrap-around rules so the texels come out in the same order.
fn generate_magic_square(size: u32) -> Vec<u32> {
    let noise_size = if size.is_multiple_of(2) {
        size + 1
    } else {
        size
    } as i64;
    let square_size = noise_size * noise_size;
    let mut magic_square = vec![0u32; square_size as usize];
    let mut i = noise_size / 2;
    let mut j = noise_size - 1;
    let mut num = 1u32;
    while i64::from(num) <= square_size {
        if i == -1 && j == noise_size {
            j = noise_size - 2;
            i = 0;
        } else {
            if j == noise_size {
                j = 0;
            }
            if i < 0 {
                i = noise_size - 1;
            }
        }
        let index = (i * noise_size + j) as usize;
        if magic_square[index] != 0 {
            j -= 2;
            i += 1;
            continue;
        }
        magic_square[index] = num;
        num += 1;
        j += 1;
        i -= 1;
    }
    magic_square
}

impl NodeUpdate for GtaoState {
    /// `this.updateBeforeType = NodeUpdateType.FRAME`.
    fn update_before_type(&self) -> NodeUpdateType {
        NodeUpdateType::Frame
    }

    /// `GTAONode.updateBefore( frame )`.
    fn update_before(&self, renderer: &mut Renderer) -> bool {
        // The pre-pass first, as `setup()` building the depth and normal
        // nodes before itself orders it in three.
        //
        // The AO is asked for from inside a pass whose `builtinAOContext` is
        // this node, and a nested pass inherits the outer context (§64). The
        // pre-pass feeds this node, so it cannot also read it: its target is
        // not rendered, or even allocated, yet. Three's pre-pass would bind
        // the 1×1 placeholder and never read it; the port lifts the context
        // for the pre-pass's render instead.
        if let Some(pass) = crate::nodes::frame::texture_update(self.normal.id()) {
            let outer_ao = renderer.context_ao.take();
            renderer.update_before_node(&pass);
            renderer.context_ao = outer_ao;
        }

        // Update the temporal uniforms.
        if self.use_temporal_filtering.get() {
            let frame_id = renderer.frame_id() as usize;
            self.temporal_direction
                .set(vec![TEMPORAL_ROTATIONS[frame_id % 6] / 360.0]);
            self.temporal_offset
                .set(vec![SPATIAL_OFFSETS[frame_id % 4]]);
        } else {
            self.temporal_direction.set(vec![0.0]);
            self.temporal_offset.set(vec![1.0]);
        }

        {
            let camera = self.camera.borrow();
            self.projection.set(mat4_values(&camera.projection_matrix));
            self.projection_inverse
                .set(mat4_values(&camera.projection_matrix_inverse));
        }

        // Rebuild the material if the sample count has changed.
        if self.samples.get() != self.current_samples.get() {
            self.current_samples.set(self.samples.get());
            self.quad.borrow_mut().material = material(
                &self.depth,
                &self.normal,
                &self.noise,
                &self.uniforms,
                self.samples.get(),
            );
        }

        // `setSize( renderer.getDrawingBufferSize() )`.
        let (width, height) = renderer.drawing_buffer_size();
        let scale = self.resolution_scale.get();
        let width = (scale * f64::from(width)).round() as u32;
        let height = (scale * f64::from(height)).round() as u32;
        self.resolution_scale_uniform.set(vec![scale]);
        self.resolution
            .set(vec![f64::from(width), f64::from(height)]);
        if self.target.size() != (width, height) {
            self.target.set_size(width.max(1), height.max(1));
        }

        // `_rendererState = RendererUtils.resetRendererState( renderer, … )`.
        let previous_target = renderer.render_target();
        let previous_mrt = renderer.mrt();
        let previous_auto_clear = renderer.auto_clear;
        let previous_clear_color = renderer.clear_color();
        let previous_clear_alpha = renderer.clear_alpha();
        renderer.set_mrt(None);
        renderer.auto_clear = true;
        // Clear to white: no occlusion where the quad discards (the sky).
        renderer.set_clear_color(Color::new(1.0, 1.0, 1.0), 1.0);

        // AO.
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

#[cfg(test)]
mod tests {
    use super::*;

    /// Three's `generateMagicSquare( 5 )`, as its loop produces it.
    #[test]
    fn magic_square_of_five_matches_three() {
        let square = generate_magic_square(5);
        assert_eq!(
            square,
            vec![
                9, 3, 22, 16, 15, //
                2, 21, 20, 14, 8, //
                25, 19, 13, 7, 1, //
                18, 12, 6, 5, 24, //
                11, 10, 4, 23, 17,
            ]
        );
    }
}
