//! Port of `three.js/examples/jsm/tsl/display/SSGINode.js` — screen space
//! global illumination with a visibility bitmask, after SSRT3.
//!
//! Every frame the node draws one full-screen quad into a two-attachment
//! target: `SSGI.AO` (one channel) and `SSGI.GI` (RGB). For each pixel it
//! unprojects the scene depth to a view position and, for `sliceCount`
//! directions around it, marches `stepCount` steps along the screen to either
//! side. Each sample's front and back horizon angles mark a run of bits in a
//! 32-bit occlusion mask; the bits a sample sets for the first time are the
//! share of the hemisphere it occludes, and the sample's colour in the beauty
//! pass — weighted by that share and by the cosines at both ends — is the
//! light it bounces back. The set bits over all slices are the AO.
//!
//! Three's node is a `Node` with `updateBeforeType = FRAME`; so is this one
//! ([`SsgiState`] implements [`NodeUpdate`] and is registered as the updater
//! of both textures, the way `passTexture( this, … )` makes it theirs in
//! three). The two textures are written through `outputStruct( aoField,
//! giField )`, which this port carries as
//! [`output_struct`](crate::nodes::tsl::output_struct): the fragment stage
//! returns a struct with an `f32` and a `vec3<f32>` member.
//!
//! The GI attachment is `rg11b10ufloat`, as three's `RGBFormat` /
//! `UnsignedInt101111Type` is, and is only renderable with the device feature
//! of that name. Without it three logs an error and the effect fails; so does
//! the port. It logs the same error and leaves the format alone, so wgpu
//! rejects the attachment (`RENDER_ATTACHMENT` on a format the device cannot
//! render) as a validation error, which wgpu's default handler raises as a
//! panic. There is no fallback: the fragment stage writes a `vec3<f32>`, and
//! wgpu rejects a four-channel target such as `rgba16float` for it.
//!
//! Not ported:
//! - an arbitrary `normalNode`. [`ssgi`] takes the scene pass's packed
//!   normal texture and unpacks it with `* 2 - 1` itself, which is what the
//!   page's `sample( uv => unpackRGBToNormal( … ) )` does;
//! - `normalNode = null`, which rebuilds the normal from depth through
//!   `getNormalFromDepth`;
//! - a logarithmic depth buffer;
//! - the AO texture's name `SSGI.AO`: the render target names attachment 0
//!   `output`, whatever it is given;
//! - `setup()` returning the AO node as the node's own value. [`SsgiNode`]
//!   is not a node; [`SsgiNode::ao_node`] and [`SsgiNode::gi_node`] are the
//!   outputs;
//! - `dispose()`: the target and quad are freed when the last `Rc` drops;
//! - `contextNode = context( builder.getSharedContext() )` on the quad's
//!   material.
//!
//! r187's node has no `resolutionScale`; the target is always the drawing
//! buffer's size, here as in three.

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use crate::cameras::PerspectiveCamera;
use crate::materials::MeshBasicNodeMaterial;
use crate::math::{Color, Matrix4};
use crate::nodes::frame::register_texture_update;
use crate::nodes::node::{FnDef, Lazy, SettableValue, Type};
use crate::nodes::tsl::{
    abs, block, boolean, break_loop, call, ceil, cross, discard, dot, float, fract,
    get_view_position, half_pi, if_else, if_then, interleaved_gradient_noise, loop_options, max,
    output_struct, pass_depth_texture_uv, pi, property, rand, screen_coordinate, shader_fn, sign,
    sqrt, texture_uv, to_const, to_var, uint, uniform_settable, uv, vec2, vec2_join, vec3,
    vec3_join, vec4, vec4_join,
};
use crate::nodes::{NodeRef, NodeUpdate, NodeUpdateType};
use crate::objects::QuadMesh;
use crate::renderer::{RenderTarget, RenderTargetOptions, Renderer};
use crate::textures::{DepthTexture, Texture, TextureFilter, TextureType};

/// `_temporalRotations`, in degrees: the slice rotation per frame under
/// temporal filtering.
const TEMPORAL_ROTATIONS: [f64; 6] = [60.0, 300.0, 180.0, 240.0, 120.0, 0.0];
/// `_spatialOffsets`: the per-frame step offset under temporal filtering.
const SPATIAL_OFFSETS: [f64; 4] = [0.0, 0.5, 0.25, 0.75];

/// `MAX_RAY` — the occlusion bitmask's width.
const MAX_RAY: u32 = 32;

/// `ssgi( beautyNode, depthNode, normalNode, camera )`.
///
/// As with the other display nodes, the inputs are the textures three's
/// `getTextureNode()` would hand over: the scene pass's `output`, its depth
/// attachment, and the attachment its MRT fills with `packNormalToRGB(
/// normalView )`. The port unpacks the normals itself — the page's
/// `normalNode` is `sample( ( uv ) => unpackRGBToNormal( … .sample( uv ) ) )`,
/// so this is what three generates. The camera must be the pass's.
pub fn ssgi(
    beauty: &Texture,
    depth: &DepthTexture,
    normal: &Texture,
    camera: Rc<RefCell<PerspectiveCamera>>,
) -> SsgiNode {
    SsgiNode::new(beauty, depth, normal, camera)
}

/// `SSGINode` — see the module docs.
pub struct SsgiNode {
    /// `this.sliceCount` — how many directions around the pixel are
    /// marched, a `uint`, `1` by default.
    pub slice_count: SettableValue,
    /// `this.stepCount` — how many steps each side of a slice takes, a
    /// `uint`, `12` by default.
    pub step_count: SettableValue,
    /// `this.aoIntensity` — the exponent the visibility is raised to, `1` by
    /// default.
    pub ao_intensity: SettableValue,
    /// `this.giIntensity` — the indirect light's multiplier, `10` by
    /// default.
    pub gi_intensity: SettableValue,
    /// `this.radius` — how far the steps reach: in units of 1/16 of half the
    /// drawing buffer's width under screen-space sampling, in view units
    /// otherwise. `12` by default.
    pub radius: SettableValue,
    /// `this.useScreenSpaceSampling` — whether `radius` is in screen space
    /// (constant on screen whatever the depth), a `bool`, on by default.
    pub use_screen_space_sampling: SettableValue,
    /// `this.expFactor` — the exponent that spaces the steps out with
    /// distance, `2` by default.
    pub exp_factor: SettableValue,
    /// `this.thickness` — how deep, in view units, a sample's surface is
    /// assumed to be, `1` by default.
    pub thickness: SettableValue,
    /// `this.useLinearThickness` — scale `thickness` by the sample's linear
    /// depth (`100 · −z / far`), a `bool`, off by default.
    pub use_linear_thickness: SettableValue,
    /// `this.backfaceLighting` — how much light a sample's back face
    /// bounces, `0` by default.
    pub backface_lighting: SettableValue,
    state: Rc<SsgiState>,
}

/// The node's state, shared with the renderer's update-before registry.
pub(crate) struct SsgiState {
    /// `this.beautyNode.value`, whose pass renders the scene.
    beauty: Texture,
    /// `this._camera`.
    camera: Rc<RefCell<PerspectiveCamera>>,
    /// `this._ssgiRenderTarget`: `SSGI.AO` and `SSGI.GI`.
    target: RenderTarget,
    /// `_quadMesh` with `this._material`.
    quad: QuadMesh,
    /// `this.useTemporalFiltering`.
    use_temporal_filtering: Cell<bool>,
    /// Whether the GI attachment's format has been settled against the
    /// device's features.
    format_checked: Cell<bool>,
    /// The settable halves of the private uniforms.
    resolution: SettableValue,
    half_proj_scale: SettableValue,
    temporal_direction: SettableValue,
    temporal_offset: SettableValue,
    projection_inverse: SettableValue,
    camera_far: SettableValue,
}

/// The node halves of every uniform the `gi` `Fn()` reads.
struct Uniforms {
    slice_count: NodeRef,
    step_count: NodeRef,
    ao_intensity: NodeRef,
    gi_intensity: NodeRef,
    radius: NodeRef,
    use_screen_space_sampling: NodeRef,
    exp_factor: NodeRef,
    thickness: NodeRef,
    use_linear_thickness: NodeRef,
    backface_lighting: NodeRef,
    resolution: NodeRef,
    half_proj_scale: NodeRef,
    temporal_direction: NodeRef,
    temporal_offset: NodeRef,
    projection_inverse: NodeRef,
    camera_far: NodeRef,
}

fn mat4_values(m: &Matrix4) -> Vec<f64> {
    m.elements.to_vec()
}

impl SsgiNode {
    /// `new SSGINode( beautyNode, depthNode, normalNode, camera )`, with the
    /// material `setup()` gives it.
    pub fn new(
        beauty: &Texture,
        depth: &DepthTexture,
        normal: &Texture,
        camera: Rc<RefCell<PerspectiveCamera>>,
    ) -> Self {
        // `new RenderTarget( 1, 1, { depthBuffer: false, count: 2 } )`, then
        // `textures[ 0 ]` is `RedFormat` / `UnsignedByteType` and `textures[
        // 1 ]` `RGBFormat` / `UnsignedInt101111Type`.
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
        .expect("three-rs: the SSGI target is a colour type");
        target.set_count(2);
        target.set_texture_name(1, "SSGI.GI");
        let textures = target.textures();
        textures[0].set_format(wgpu::TextureFormat::R8Unorm);
        textures[1].set_format(wgpu::TextureFormat::Rg11b10Ufloat);

        let (slice_count, slice_count_value) = uniform_settable(Type::U32, vec![1.0]);
        let (step_count, step_count_value) = uniform_settable(Type::U32, vec![12.0]);
        let (ao_intensity, ao_intensity_value) = uniform_settable(Type::F32, vec![1.0]);
        let (gi_intensity, gi_intensity_value) = uniform_settable(Type::F32, vec![10.0]);
        let (radius, radius_value) = uniform_settable(Type::F32, vec![12.0]);
        let (use_screen_space_sampling, use_screen_space_sampling_value) =
            uniform_settable(Type::Bool, vec![1.0]);
        let (exp_factor, exp_factor_value) = uniform_settable(Type::F32, vec![2.0]);
        let (thickness, thickness_value) = uniform_settable(Type::F32, vec![1.0]);
        let (use_linear_thickness, use_linear_thickness_value) =
            uniform_settable(Type::Bool, vec![0.0]);
        let (backface_lighting, backface_lighting_value) = uniform_settable(Type::F32, vec![0.0]);
        let (resolution, resolution_value) = uniform_settable(Type::Vec2, vec![0.0, 0.0]);
        let (half_proj_scale, half_proj_scale_value) = uniform_settable(Type::F32, vec![1.0]);
        let (temporal_direction, temporal_direction_value) = uniform_settable(Type::F32, vec![0.0]);
        let (temporal_offset, temporal_offset_value) = uniform_settable(Type::F32, vec![0.0]);
        let (projection_inverse, projection_inverse_value) = uniform_settable(
            Type::Mat4,
            mat4_values(&camera.borrow().projection_matrix_inverse),
        );
        // `reference( 'far', 'float', camera )` — an object-group uniform in
        // three's dump, so a settable one here rather than `cameraFar`.
        let (camera_far, camera_far_value) = uniform_settable(Type::F32, vec![camera.borrow().far]);
        let uniforms = Uniforms {
            slice_count,
            step_count,
            ao_intensity,
            gi_intensity,
            radius,
            use_screen_space_sampling,
            exp_factor,
            thickness,
            use_linear_thickness,
            backface_lighting,
            resolution,
            half_proj_scale,
            temporal_direction,
            temporal_offset,
            projection_inverse,
            camera_far,
        };

        let mut material = MeshBasicNodeMaterial::new();
        material.name = "SSGI";
        let (gi, output) = gi_node(beauty, depth, normal, &uniforms);
        material.color_node = Some(gi);
        material.output_node = Some(output);

        let state = Rc::new(SsgiState {
            beauty: beauty.clone(),
            camera,
            target,
            quad: QuadMesh::new(material),
            use_temporal_filtering: Cell::new(true),
            format_checked: Cell::new(false),
            resolution: resolution_value,
            half_proj_scale: half_proj_scale_value,
            temporal_direction: temporal_direction_value,
            temporal_offset: temporal_offset_value,
            projection_inverse: projection_inverse_value,
            camera_far: camera_far_value,
        });
        for texture in state.target.textures() {
            register_texture_update(texture.id(), &state);
        }
        Self {
            slice_count: slice_count_value,
            step_count: step_count_value,
            ao_intensity: ao_intensity_value,
            gi_intensity: gi_intensity_value,
            radius: radius_value,
            use_screen_space_sampling: use_screen_space_sampling_value,
            exp_factor: exp_factor_value,
            thickness: thickness_value,
            use_linear_thickness: use_linear_thickness_value,
            backface_lighting: backface_lighting_value,
            state,
        }
    }

    /// `ssgiNode.getAONode()`'s texture, `SSGI.AO`: the visibility in `.x`,
    /// `1` where nothing occludes (and where the quad discards, the sky).
    pub fn ao_texture(&self) -> Texture {
        self.state.target.textures()[0].clone()
    }

    /// `ssgiNode.getGINode()`'s texture, `SSGI.GI`: the bounced light in
    /// `.rgb`.
    pub fn gi_texture(&self) -> Texture {
        self.state.target.textures()[1].clone()
    }

    /// `ssgiNode.getAONode()`, read at `uv()` as the scalar three's
    /// `RedFormat` texture node gives the composite.
    pub fn ao_node(&self) -> NodeRef {
        texture_uv(&self.ao_texture(), uv()).x()
    }

    /// `ssgiNode.getGINode().rgb`, read at `uv()`.
    pub fn gi_node(&self) -> NodeRef {
        texture_uv(&self.gi_texture(), uv()).xyz()
    }

    /// `ssgiNode.useTemporalFiltering = value` — rotate the slices and
    /// offset the steps per frame, for a TRAA to average. On by default, as
    /// in three.
    pub fn set_use_temporal_filtering(&self, on: bool) {
        self.state.use_temporal_filtering.set(on);
    }

    /// `ssgiNode.useTemporalFiltering`.
    pub fn use_temporal_filtering(&self) -> bool {
        self.state.use_temporal_filtering.get()
    }

    /// The `SSGI` quad material, for `examples/dump_wgsl.rs` and the dump
    /// gate.
    #[doc(hidden)]
    pub fn quad_material(&self) -> MeshBasicNodeMaterial {
        self.state.quad.material.clone()
    }
}

/// `spatialOffsets( position )` — `0.25 * ( ( position.y - position.x ) & 3
/// )`, with a layout, so a real `fn` in the module.
fn spatial_offsets(position: NodeRef) -> NodeRef {
    thread_local! { static CELL: Lazy<Rc<FnDef>> = const { Lazy::new() }; }
    let def = CELL.with(|cell| {
        cell.get(|| {
            shader_fn(
                Some("spatialOffsets"),
                vec![("position", Type::Vec2)],
                Type::F32,
                |args| {
                    let position = args[0].clone();
                    let bits = position
                        .y()
                        .sub(position.x())
                        .to(Type::I32)
                        .bit_and(crate::nodes::tsl::int(3));
                    float(0.25).mul(bits.to(Type::F32))
                },
            )
        })
    });
    call(&def, vec![position])
}

/// `GTAOFastAcos( value )` — a polynomial `acos` of both components, with a
/// layout.
fn gtao_fast_acos(value: NodeRef) -> NodeRef {
    thread_local! { static CELL: Lazy<Rc<FnDef>> = const { Lazy::new() }; }
    let def = CELL.with(|cell| {
        cell.get(|| {
            shader_fn(
                Some("GTAOFastAcos"),
                vec![("value", Type::Vec2)],
                Type::Vec2,
                |args| {
                    let value = args[0].clone();
                    let out_val = to_var(
                        None,
                        abs(value.clone()).mul(float(-0.156583)).add(half_pi()),
                    );
                    let scaled = out_val.mul_assign(sqrt(abs(value.clone()).one_minus()));
                    let x = value
                        .x()
                        .greater_than_equal(0.0)
                        .select(out_val.x(), pi().sub(out_val.x()));
                    let y = value
                        .y()
                        .greater_than_equal(0.0)
                        .select(out_val.y(), pi().sub(out_val.y()));
                    block(vec![out_val.clone(), scaled], vec2_join(vec![x, y]))
                },
            )
        })
    });
    call(&def, vec![value])
}

/// The per-pixel values `horizonSampling()` takes besides its direction.
struct Slice<'a> {
    step_radius: &'a NodeRef,
    radius_vs: &'a NodeRef,
    view_position: &'a NodeRef,
    slide_dir_texel_size: &'a NodeRef,
    initial_ray_step: &'a NodeRef,
    uv_node: &'a NodeRef,
    view_dir: &'a NodeRef,
    view_normal: &'a NodeRef,
    n: &'a NodeRef,
    global_occluded_bitfield: &'a NodeRef,
}

/// The textures the `gi` `Fn()` samples.
struct Inputs<'a> {
    beauty: &'a Texture,
    depth: &'a DepthTexture,
    normal: &'a Texture,
}

impl Inputs<'_> {
    /// `sampleDepth( uv )` — no logarithmic depth.
    fn depth(&self, coord: NodeRef) -> NodeRef {
        pass_depth_texture_uv(self.depth, coord)
    }

    /// `sampleNormal( uv )` — `normalNode.sample( uv ).rgb.normalize()`, with
    /// the page's `unpackRGBToNormal` inside the node.
    fn normal(&self, coord: NodeRef) -> NodeRef {
        texture_uv(self.normal, coord)
            .mul(2.0)
            .sub(1.0)
            .xyz()
            .normalize()
    }

    /// `sampleBeauty( uv )`.
    fn beauty(&self, coord: NodeRef) -> NodeRef {
        texture_uv(self.beauty, coord)
    }
}

/// `horizonSampling( directionIsRight, … )` — the inline `Fn()` that marches
/// one side of a slice. Returns its statements and the `color` it gathers.
#[inline(never)]
fn horizon_sampling(
    direction_is_right: bool,
    s: &Slice<'_>,
    inputs: &Inputs<'_>,
    u: &Uniforms,
) -> (Vec<NodeRef>, NodeRef) {
    let direction_is_right = boolean(direction_is_right);
    let step_count = to_const(None, u.step_count.clone());
    let exp_factor = to_const(None, u.exp_factor.clone());
    let thickness = to_const(None, u.thickness.clone());
    let backface_lighting = to_const(None, u.backface_lighting.clone());

    // Port note (three's): because of different uv conventions, uv-y has a
    // different sign.
    let uv_direction = direction_is_right.select(vec2(1.0, -1.0), vec2(-1.0, 1.0));
    let sampling_direction = direction_is_right.select(float(1.0), float(-1.0));
    let color = to_var(None, vec3(0.0, 0.0, 0.0));

    let steps = loop_options("i", Type::U32, uint(0), step_count.clone(), "<", |i| {
        march_step(
            i,
            &direction_is_right,
            &uv_direction,
            &sampling_direction,
            &color,
            &thickness,
            &exp_factor,
            &backface_lighting,
            s,
            inputs,
            u,
        )
    });

    (
        vec![
            exp_factor.clone(),
            thickness.clone(),
            backface_lighting.clone(),
            color.clone(),
            step_count,
            steps,
        ],
        color,
    )
}

/// One iteration of `horizonSampling()`'s step loop.
#[allow(clippy::too_many_arguments)]
#[inline(never)]
fn march_step(
    i: &NodeRef,
    direction_is_right: &NodeRef,
    uv_direction: &NodeRef,
    sampling_direction: &NodeRef,
    color: &NodeRef,
    thickness: &NodeRef,
    exp_factor: &NodeRef,
    backface_lighting: &NodeRef,
    s: &Slice<'_>,
    inputs: &Inputs<'_>,
    u: &Uniforms,
) -> Vec<NodeRef> {
    let i_float = i.to(Type::F32);
    let offset = to_const(
        None,
        abs(s
            .step_radius
            .mul(i_float.add(s.initial_ray_step.clone()))
            .div(s.radius_vs.clone()))
        .pow(exp_factor.clone())
        .mul(s.radius_vs.clone()),
    );
    let uv_offset = to_const(
        None,
        s.slide_dir_texel_size
            .mul(max(offset.clone(), i.to(Type::F32).add(1.0))),
    );
    let sample_uv = to_const(None, s.uv_node.add(uv_offset.mul(uv_direction.clone())));
    let out_of_bounds = if_then(
        sample_uv
            .x()
            .less_than_equal(0.0)
            .or(sample_uv.y().less_than_equal(0.0))
            .or(sample_uv.x().greater_than_equal(1.0))
            .or(sample_uv.y().greater_than_equal(1.0)),
        vec![break_loop()],
    );

    let sample_view_position = to_const(
        None,
        get_view_position(
            sample_uv.clone(),
            inputs.depth(sample_uv.clone()),
            u.projection_inverse.clone(),
        ),
    );
    let pixel_to_sample = to_const(
        None,
        sample_view_position
            .sub(s.view_position.clone())
            .normalize(),
    );
    let linear_thickness_multiplier = u.use_linear_thickness.select(
        sample_view_position
            .z()
            .negate()
            .div(u.camera_far.clone())
            .clamp(0.0, 1.0)
            .mul(100.0),
        float(1.0),
    );
    let pixel_to_sample_backface = sample_view_position
        .sub(
            linear_thickness_multiplier
                .mul(s.view_dir.clone())
                .mul(thickness.clone()),
        )
        .sub(s.view_position.clone())
        .normalize();

    let front_back_horizon = vec2_join(vec![
        dot(pixel_to_sample.clone(), s.view_dir.clone()),
        dot(pixel_to_sample_backface, s.view_dir.clone()),
    ]);
    let front_back_horizon = gtao_fast_acos(front_back_horizon.clamp(-1.0, 1.0));
    // Port note (three's): subtract half pi instead of adding it.
    let front_back_horizon = sampling_direction
        .mul(front_back_horizon.negate())
        .sub(s.n.sub(half_pi()))
        .div(pi())
        .clamp(0.0, 1.0);
    // Front/back get inverted depending on angle.
    let front_back_horizon =
        direction_is_right.select(front_back_horizon.swizzle("yx"), front_back_horizon.xy());
    let min_horizon = to_const(None, front_back_horizon.x());
    let max_horizon = to_const(None, front_back_horizon.y());
    let start_horizon_int = to_const(
        None,
        front_back_horizon
            .mul(float(f64::from(MAX_RAY)))
            .x()
            .to(Type::U32),
    );
    let angle_horizon_int = to_const(
        None,
        ceil(
            max_horizon
                .sub(min_horizon.clone())
                .mul(float(f64::from(MAX_RAY))),
        )
        .to(Type::U32),
    );
    let angle_horizon_bitfield = to_const(
        None,
        angle_horizon_int.greater_than(uint(0)).select(
            uint(0xFFFF_FFFF).shift_right(
                uint(32)
                    .sub(uint(MAX_RAY))
                    .add(uint(MAX_RAY).sub(angle_horizon_int.clone())),
            ),
            uint(0),
        ),
    );
    let current_occluded_bitfield = to_const(
        None,
        angle_horizon_bitfield
            .shift_left(start_horizon_int.clone())
            .bit_and(s.global_occluded_bitfield.bit_not()),
    );
    let global = s.global_occluded_bitfield;
    let merge = global.assign(global.bit_or(current_occluded_bitfield.clone()));
    let num_occluded_zones = to_const(None, current_occluded_bitfield.count_one_bits());

    let gather = gather_light(
        &num_occluded_zones,
        &pixel_to_sample,
        &sample_uv,
        color,
        backface_lighting,
        s,
        inputs,
    );

    vec![
        offset,
        uv_offset,
        sample_uv,
        out_of_bounds,
        sample_view_position,
        pixel_to_sample,
        min_horizon,
        max_horizon,
        start_horizon_int,
        angle_horizon_int,
        angle_horizon_bitfield,
        current_occluded_bitfield,
        merge,
        num_occluded_zones,
        gather,
    ]
}

/// The nested `If()`s that add a visible sample's light to `color`.
#[inline(never)]
fn gather_light(
    num_occluded_zones: &NodeRef,
    pixel_to_sample: &NodeRef,
    sample_uv: &NodeRef,
    color: &NodeRef,
    backface_lighting: &NodeRef,
    s: &Slice<'_>,
    inputs: &Inputs<'_>,
) -> NodeRef {
    // If a ray hit the sample, that sample is visible from the shading point.
    let light_color = to_var(None, inputs.beauty(sample_uv.clone()));
    let light_direction_vs = to_const(None, pixel_to_sample.normalize());
    let normal_dot_light_direction = to_const(
        None,
        dot(s.view_normal.clone(), light_direction_vs.clone()).clamp(0.0, 1.0),
    );
    let light_normal_vs = to_const(None, inputs.normal(sample_uv.clone()));
    let raw = dot(light_normal_vs.clone(), light_direction_vs.negate());
    let raw_const = to_const(None, raw.clone());
    let d = sign(raw_const.clone()).less_than(0.0).select(
        abs(raw_const.clone()).mul(backface_lighting.clone()),
        abs(raw_const.clone()),
    );
    let light_normal_dot_light_direction = backface_lighting
        .greater_than(0.0)
        .and(dot(light_normal_vs.clone(), s.view_dir.clone()).greater_than(0.0))
        .select(block(vec![raw_const], d), raw.clamp(0.0, 1.0));
    // `color.rgb.addAssign( … )` with a `vec4` light colour: three widens
    // `color` and narrows the sum back.
    let add = color.assign(
        vec4_join(vec![color.clone(), float(1.0)])
            .add(
                num_occluded_zones
                    .to(Type::F32)
                    .div(float(f64::from(MAX_RAY)))
                    .mul(light_color.clone())
                    .mul(normal_dot_light_direction.clone())
                    .mul(light_normal_dot_light_direction),
            )
            .xyz(),
    );
    // `luminance( lightColor )` of a `vec4`: the coefficients are widened
    // with a `1`, so alpha counts.
    let luminance = dot(
        light_color.clone(),
        vec4_join(vec![vec3(0.2126, 0.7152, 0.0722), float(1.0)]),
    );
    if_then(
        num_occluded_zones.to(Type::F32).greater_than(0.0),
        vec![
            light_color,
            // Continue if there is light at that location (intensity > 0).
            if_then(
                luminance.greater_than(0.001),
                vec![
                    light_direction_vs,
                    normal_dot_light_direction.clone(),
                    // Continue if light is facing the surface normal.
                    if_then(
                        normal_dot_light_direction.greater_than(0.001),
                        vec![light_normal_vs, add],
                    ),
                ],
            ),
        ],
    )
}

/// `setup()`'s `gi` `Fn()` and `outputStruct( aoField, giField )`: the
/// material's `colorNode` and `outputNode`.
#[inline(never)]
fn gi_node(
    beauty: &Texture,
    depth: &DepthTexture,
    normal: &Texture,
    u: &Uniforms,
) -> (NodeRef, NodeRef) {
    let inputs = Inputs {
        beauty,
        depth,
        normal,
    };
    let uv_node = uv();
    let ao_field = property("ssgiAO", Type::F32);
    let gi_field = property("ssgiGI", Type::Vec3);
    let output = output_struct(vec![ao_field.clone(), gi_field.clone()]);

    let depth_value = to_var(None, inputs.depth(uv_node.clone()));
    let discard_far = if_then(depth_value.greater_than_equal(1.0), vec![discard()]);
    let view_position = to_var(
        None,
        get_view_position(
            uv_node.clone(),
            depth_value.clone(),
            u.projection_inverse.clone(),
        ),
    );
    let view_normal = to_var(None, inputs.normal(uv_node.clone()));
    let view_dir = to_var(None, view_position.xyz().negate().normalize());

    let noise_offset = spatial_offsets(screen_coordinate());
    let noise_direction = interleaved_gradient_noise(screen_coordinate());
    // Three's port note: `noiseJitterIdx` for slightly better noise
    // convergence with TRAA (#31890).
    let noise_jitter_idx = u.temporal_direction.mul(0.02);
    let initial_ray_step = fract(noise_offset.add(u.temporal_offset.clone()))
        .add(rand(uv_node.add(noise_jitter_idx).mul(2.0).sub(1.0)));

    let ao = to_var(None, float(0.0));
    let color = to_var(None, vec3(0.0, 0.0, 0.0));
    let rotation_count = to_const(None, u.slice_count.clone());
    let step_count = to_const(None, u.step_count.clone());
    let ao_intensity = to_const(None, u.ao_intensity.clone());
    let gi_intensity = to_const(None, u.gi_intensity.clone());
    let radius = to_const(None, u.radius.clone());

    let step_radius = to_var(None, float(0.0));
    let pick_step_radius = if_else(
        u.use_screen_space_sampling.clone(),
        // SSRT3 divides `stepRadius` by `STEP_COUNT` twice; three fixes it.
        vec![step_radius.assign(radius.mul(u.resolution.x().div(2.0)).div(float(16.0)))],
        // View z is negative, so a negate is required.
        vec![step_radius.assign(max(
            radius
                .mul(u.half_proj_scale.clone())
                .div(view_position.z().negate()),
            step_count.to(Type::F32),
        ))],
    );
    let step_radius_div = step_radius.div_assign(step_count.to(Type::F32).add(1.0));
    let radius_vs = to_const(
        None,
        max(float(1.0), step_count.sub(uint(1)).to(Type::F32)).mul(step_radius.clone()),
    );

    let global_occluded_bitfield = to_var(None, uint(0));

    let slices = loop_options("i", Type::U32, uint(0), rotation_count.clone(), "<", |i| {
        let rotation_angle = to_const(
            None,
            i.to(Type::F32)
                .add(noise_direction.clone())
                .add(u.temporal_direction.clone())
                .mul(pi().div(rotation_count.to(Type::F32))),
        );
        let slice_dir = to_const(
            None,
            vec3_join(vec![
                vec2_join(vec![rotation_angle.cos(), rotation_angle.sin()]),
                float(0.0),
            ]),
        );
        let slide_dir_texel_size = to_const(
            None,
            slice_dir.xy().mul(float(1.0).div(u.resolution.clone())),
        );
        let plane_normal = to_const(None, cross(slice_dir.clone(), view_dir.clone()).normalize());
        let tangent = to_const(None, cross(view_dir.clone(), plane_normal.clone()));
        let projected_normal = to_const(
            None,
            view_normal.sub(plane_normal.mul(dot(view_normal.clone(), plane_normal.clone()))),
        );
        let projected_normal_normalized = to_const(None, projected_normal.normalize());
        let cos_n = to_const(
            None,
            dot(projected_normal_normalized.clone(), view_dir.clone()).clamp(-1.0, 1.0),
        );
        let n = to_const(
            None,
            sign(dot(projected_normal.clone(), tangent.clone()))
                .negate()
                .mul(cos_n.acos()),
        );

        let slice = Slice {
            step_radius: &step_radius,
            radius_vs: &radius_vs,
            view_position: &view_position,
            slide_dir_texel_size: &slide_dir_texel_size,
            initial_ray_step: &initial_ray_step,
            uv_node: &uv_node,
            view_dir: &view_dir,
            view_normal: &view_normal,
            n: &n,
            global_occluded_bitfield: &global_occluded_bitfield,
        };
        let (right, right_color) = horizon_sampling(true, &slice, &inputs, u);
        let (left, left_color) = horizon_sampling(false, &slice, &inputs, u);

        let mut statements = vec![
            rotation_angle,
            slice_dir,
            slide_dir_texel_size,
            plane_normal,
            tangent,
            projected_normal,
            projected_normal_normalized,
            cos_n,
            n,
            global_occluded_bitfield.clone(),
            global_occluded_bitfield.assign(uint(0)),
        ];
        statements.extend(right);
        statements.push(color.add_assign(right_color));
        statements.extend(left);
        statements.push(color.add_assign(left_color));
        statements.push(
            ao.add_assign(
                global_occluded_bitfield
                    .count_one_bits()
                    .to(Type::F32)
                    .div(float(f64::from(MAX_RAY))),
            ),
        );
        statements
    });

    // 7 represents an HDR luminance value.
    let max_luminance = to_const(None, float(7.0));
    let current_luminance = to_const(None, dot(color.clone(), vec3(0.2126, 0.7152, 0.0722)));
    let scale = current_luminance
        .greater_than(max_luminance.clone())
        .select(max_luminance.div(current_luminance.clone()), float(1.0));

    let statements = vec![
        depth_value,
        discard_far,
        view_position,
        view_normal,
        view_dir,
        ao.clone(),
        color.clone(),
        rotation_count.clone(),
        step_count,
        ao_intensity.clone(),
        gi_intensity.clone(),
        radius,
        step_radius.clone(),
        pick_step_radius,
        step_radius_div,
        radius_vs,
        slices,
        ao.div_assign(rotation_count.to(Type::F32)),
        ao.assign(
            ao.clamp(0.0, 1.0)
                .one_minus()
                .pow(ao_intensity)
                .clamp(0.0, 1.0),
        ),
        color.div_assign(rotation_count.to(Type::F32)),
        color.mul_assign(gi_intensity),
        max_luminance,
        current_luminance,
        color.mul_assign(scale),
        ao_field.assign(ao),
        gi_field.assign(color),
    ];
    (block(statements, vec4(0.0, 0.0, 0.0, 0.0)), output)
}

impl NodeUpdate for SsgiState {
    /// `this.updateBeforeType = NodeUpdateType.FRAME`.
    fn update_before_type(&self) -> NodeUpdateType {
        NodeUpdateType::Frame
    }

    /// `SSGINode.updateBefore( frame )`.
    fn update_before(&self, renderer: &mut Renderer) -> bool {
        // The scene pass first, as `setup()` building the beauty, depth and
        // normal nodes before itself orders it in three.
        if let Some(pass) = crate::nodes::frame::texture_update(self.beauty.id()) {
            renderer.update_before_node(&pass);
        }

        // `setup()`'s feature check, once. Like three it only logs: the GI
        // attachment stays `rg11b10ufloat`, and wgpu rejects it on the render
        // below. A wider format is no way out, since the fragment stage
        // writes a `vec3<f32>` to it and wgpu rejects four-channel targets
        // for a three-component output.
        if !self.format_checked.replace(true)
            && !renderer
                .features()
                .contains(wgpu::Features::RG11B10UFLOAT_RENDERABLE)
        {
            eprintln!(
                "THREE.SSGINode: The device does not support the \"rg11b10ufloat-renderable\" \
                 feature which is required for SSGI."
            );
        }

        // `this.setSize( renderer.getDrawingBufferSize() )`.
        let (width, height) = renderer.drawing_buffer_size();
        let (width, height) = (width.max(1), height.max(1));
        self.resolution
            .set(vec![f64::from(width), f64::from(height)]);
        if self.target.size() != (width, height) {
            self.target.set_size(width, height);
        }
        {
            let camera = self.camera.borrow();
            self.half_proj_scale.set(vec![
                f64::from(height) / ((camera.fov.to_radians() * 0.5).tan() * 2.0) * 0.5,
            ]);
            self.projection_inverse
                .set(mat4_values(&camera.projection_matrix_inverse));
            self.camera_far.set(vec![camera.far]);
        }

        if self.use_temporal_filtering.get() {
            let frame_id = renderer.node_frame().frame_id as usize;
            self.temporal_direction
                .set(vec![TEMPORAL_ROTATIONS[frame_id % 6] / 360.0]);
            self.temporal_offset
                .set(vec![SPATIAL_OFFSETS[frame_id % 4]]);
        } else {
            self.temporal_direction.set(vec![1.0]);
            self.temporal_offset.set(vec![1.0]);
        }

        // `_rendererState = RendererUtils.resetRendererState( renderer, … )`.
        let mut renderer = renderer.reset_state();
        renderer.set_clear_color(Color::new(1.0, 1.0, 1.0), 1.0);

        renderer.set_render_target(Some(self.target.clone()));
        renderer.render_quad(&self.quad);
        renderer.set_render_target(None);

        // `RendererUtils.restoreRendererState( renderer, _rendererState )`.
        drop(renderer);
        true
    }
}
