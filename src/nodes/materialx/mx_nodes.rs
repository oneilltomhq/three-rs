//! Port of `three.js/src/nodes/materialx/MaterialXNodes.js` — the public
//! MaterialX surface TSL re-exports: thin wrappers that give the
//! `MaterialXNoise.js` functions their defaults, the texture-coordinate
//! helpers, and the math ops `MaterialXLoader` maps MaterialX nodes onto.
//!
//! Rust has no default arguments, so every parameter three defaults is a
//! plain argument here and the default is named in the doc comment. Where
//! three's default is `uv()`, pass [`uv`](crate::nodes::tsl::uv).
//!
//! JS numbers become `float` constants in TSL, so a numeric argument here is
//! `impl Into<NodeRef>`: pass `1.0`, not `1`, which would be an `int`.

use crate::nodes::node::{Node, NodeRef, Type};
use crate::nodes::tsl::{
    call, dpdx, dpdy, float, int, length, mix, sign, smoothstep, step, time, vec2, vec2_join, vec3,
    vec3_join, vec4_join,
};

pub use super::mx_core::{mx_rotate2d, mx_rotate3d};

use super::mx_noise::{self, fractal_def, Fractal};

/// `texcoord.convert( 'vec2|vec3' )` — `ConvertNode` with an overload list
/// keeps a `vec2` or `vec3` as it is and converts anything else to the last
/// listed type whose length matches, else the first: a `float` or a `vec4`
/// becomes a `vec2`.
fn vec2_or_vec3(texcoord: NodeRef) -> NodeRef {
    match texcoord.ty() {
        Type::Vec2 | Type::Vec3 => texcoord,
        _ => texcoord.to(Type::Vec2),
    }
}

// ---------------------------------------------------------------------------
// noise
// ---------------------------------------------------------------------------

/// `mx_noise_float( texcoord = uv(), amplitude = 1, pivot = 0 )` — Perlin
/// noise, `amplitude`-scaled and `pivot`-offset. A `vec2` texcoord takes the
/// 2D chain, a `vec3` the 3D one.
pub fn mx_noise_float(
    texcoord: NodeRef,
    amplitude: impl Into<NodeRef>,
    pivot: impl Into<NodeRef>,
) -> NodeRef {
    mx_noise::mx_perlin_noise_float(vec2_or_vec3(texcoord))
        .mul(amplitude)
        .add(pivot)
}

/// `mx_noise_vec3( texcoord = uv(), amplitude = 1, pivot = 0 )`.
pub fn mx_noise_vec3(
    texcoord: NodeRef,
    amplitude: impl Into<NodeRef>,
    pivot: impl Into<NodeRef>,
) -> NodeRef {
    mx_noise::mx_perlin_noise_vec3(vec2_or_vec3(texcoord))
        .mul(amplitude)
        .add(pivot)
}

/// `mx_noise_vec4( texcoord = uv(), amplitude = 1, pivot = 0 )` — the `vec3`
/// noise, with a `float` noise at `texcoord + ( 19, 73 )` as its `w`.
pub fn mx_noise_vec4(
    texcoord: NodeRef,
    amplitude: impl Into<NodeRef>,
    pivot: impl Into<NodeRef>,
) -> NodeRef {
    let texcoord = vec2_or_vec3(texcoord);
    vec4_join(vec![
        mx_noise::mx_perlin_noise_vec3(texcoord.clone()),
        mx_noise::mx_perlin_noise_float(texcoord.add(vec2(19.0, 73.0))),
    ])
    .mul(amplitude)
    .add(pivot)
}

/// `mx_cell_noise_float( texcoord = uv() )`.
pub fn mx_cell_noise_float(texcoord: NodeRef) -> NodeRef {
    mx_noise::mx_cell_noise_float(vec2_or_vec3(texcoord))
}

/// `mx_cell_noise_vec3( texcoord = uv() )`.
pub fn mx_cell_noise_vec3(texcoord: NodeRef) -> NodeRef {
    mx_noise::mx_cell_noise_vec3(vec2_or_vec3(texcoord))
}

// ---------------------------------------------------------------------------
// fractal noise
// ---------------------------------------------------------------------------

fn fractal(
    kind: Fractal,
    position: NodeRef,
    octaves: impl Into<NodeRef>,
    lacunarity: impl Into<NodeRef>,
    diminish: impl Into<NodeRef>,
    amplitude: impl Into<NodeRef>,
) -> NodeRef {
    call(
        &fractal_def(kind),
        vec![
            position,
            octaves.into().to(Type::I32),
            lacunarity.into(),
            diminish.into(),
        ],
    )
    .mul(amplitude)
}

/// `mx_fractal_noise_float_2d( texcoord = uv(), octaves = 3, lacunarity = 2,
/// diminish = .5, amplitude = 1 )` — `amplitude` is always multiplied in,
/// even at `1.0`.
pub fn mx_fractal_noise_float_2d(
    texcoord: NodeRef,
    octaves: impl Into<NodeRef>,
    lacunarity: impl Into<NodeRef>,
    diminish: impl Into<NodeRef>,
    amplitude: impl Into<NodeRef>,
) -> NodeRef {
    fractal(
        Fractal::Float2d,
        texcoord,
        octaves,
        lacunarity,
        diminish,
        amplitude,
    )
}

/// `mx_fractal_noise_float( position = uv(), octaves = 3, lacunarity = 2,
/// diminish = .5, amplitude = 1 )`.
pub fn mx_fractal_noise_float(
    position: NodeRef,
    octaves: impl Into<NodeRef>,
    lacunarity: impl Into<NodeRef>,
    diminish: impl Into<NodeRef>,
    amplitude: impl Into<NodeRef>,
) -> NodeRef {
    fractal(
        Fractal::Float,
        position,
        octaves,
        lacunarity,
        diminish,
        amplitude,
    )
}

/// `mx_fractal_noise_vec2( position = uv(), octaves = 3, lacunarity = 2,
/// diminish = .5, amplitude = 1 )`.
pub fn mx_fractal_noise_vec2(
    position: NodeRef,
    octaves: impl Into<NodeRef>,
    lacunarity: impl Into<NodeRef>,
    diminish: impl Into<NodeRef>,
    amplitude: impl Into<NodeRef>,
) -> NodeRef {
    fractal(
        Fractal::Vec2,
        position,
        octaves,
        lacunarity,
        diminish,
        amplitude,
    )
}

/// `mx_fractal_noise_vec3( position = uv(), octaves = 3, lacunarity = 2,
/// diminish = .5, amplitude = 1 )`.
pub fn mx_fractal_noise_vec3(
    position: NodeRef,
    octaves: impl Into<NodeRef>,
    lacunarity: impl Into<NodeRef>,
    diminish: impl Into<NodeRef>,
    amplitude: impl Into<NodeRef>,
) -> NodeRef {
    fractal(
        Fractal::Vec3,
        position,
        octaves,
        lacunarity,
        diminish,
        amplitude,
    )
}

/// `mx_fractal_noise_vec4( position = uv(), octaves = 3, lacunarity = 2,
/// diminish = .5, amplitude = 1 )`.
pub fn mx_fractal_noise_vec4(
    position: NodeRef,
    octaves: impl Into<NodeRef>,
    lacunarity: impl Into<NodeRef>,
    diminish: impl Into<NodeRef>,
    amplitude: impl Into<NodeRef>,
) -> NodeRef {
    fractal(
        Fractal::Vec4,
        position,
        octaves,
        lacunarity,
        diminish,
        amplitude,
    )
}

// ---------------------------------------------------------------------------
// Worley noise
// ---------------------------------------------------------------------------
//
// `style` and `metric` are `int( … )`-converted by three, so pass them as
// Rust integers (`0`, not `0.0`): an integer is an `int` constant already,
// and anything else is converted with `i32( … )` as three's `int()` would.

/// `mx_worley_noise_float( texcoord = uv(), jitter = 1, style = 0 )` — always
/// the 3D body; a `vec2` texcoord is widened with `z = 0`.
pub fn mx_worley_noise_float(
    texcoord: NodeRef,
    jitter: impl Into<NodeRef>,
    style: impl Into<NodeRef>,
) -> NodeRef {
    mx_worley_noise_float_3d(vec2_or_vec3(texcoord), jitter, style)
}

/// `mx_worley_noise_float_2d( texcoord = uv(), jitter = 1, style = 0 )`.
pub fn mx_worley_noise_float_2d(
    texcoord: NodeRef,
    jitter: impl Into<NodeRef>,
    style: impl Into<NodeRef>,
) -> NodeRef {
    mx_noise::mx_worley_noise_float_2d(texcoord, jitter.into(), style.into())
}

/// `mx_worley_noise_float_3d( texcoord = uv(), jitter = 1, style = 0 )`.
pub fn mx_worley_noise_float_3d(
    texcoord: NodeRef,
    jitter: impl Into<NodeRef>,
    style: impl Into<NodeRef>,
) -> NodeRef {
    mx_noise::mx_worley_noise_float_3d(texcoord, jitter.into(), style.into())
}

/// `mx_worley_noise_vec2( texcoord = uv(), jitter = 1 )` — the two nearest
/// Euclidean distances (metric is fixed at `int( 1 )`).
pub fn mx_worley_noise_vec2(texcoord: NodeRef, jitter: impl Into<NodeRef>) -> NodeRef {
    mx_noise::mx_worley_noise_vec2(vec2_or_vec3(texcoord), jitter.into(), int(1))
}

/// `mx_worley_noise_vec3( texcoord = uv(), jitter = 1, metric = 1 )` — the
/// three nearest distances.
pub fn mx_worley_noise_vec3(
    texcoord: NodeRef,
    jitter: impl Into<NodeRef>,
    metric: impl Into<NodeRef>,
) -> NodeRef {
    mx_noise::mx_worley_noise_vec3(vec2_or_vec3(texcoord), jitter.into(), to_int(metric))
}

/// `mx_worley_noise_vec3_style( texcoord = uv(), jitter = 1, style = 0,
/// metric = 0 )`.
pub fn mx_worley_noise_vec3_style(
    texcoord: NodeRef,
    jitter: impl Into<NodeRef>,
    style: impl Into<NodeRef>,
    metric: impl Into<NodeRef>,
) -> NodeRef {
    mx_noise::mx_worley_noise_vec3_style(
        vec2_or_vec3(texcoord),
        jitter.into(),
        to_int(style),
        to_int(metric),
    )
}

/// three's `int( x )`: a no-op on an `int`, a conversion otherwise.
fn to_int(x: impl Into<NodeRef>) -> NodeRef {
    x.into().to(Type::I32)
}

// ---------------------------------------------------------------------------
// unified noise
// ---------------------------------------------------------------------------

/// The arguments of `mx_unifiednoise2d` / `mx_unifiednoise3d`. Three takes
/// them positionally with defaults; [`UnifiedNoise::new_2d`] /
/// [`UnifiedNoise::new_3d`] fill in the same defaults, and the fields can be
/// overridden before the call.
#[derive(Clone)]
pub struct UnifiedNoise {
    /// `0` Perlin, `1` cell, `2` Worley, `3` fractal.
    pub noise_type: NodeRef,
    /// Default `uv()`.
    pub texcoord: NodeRef,
    /// Default `vec2( 1, 1 )` / `vec3( 1, 1, 1 )`.
    pub freq: NodeRef,
    /// Default `vec2( 0, 0 )` / `vec3( 0, 0, 0 )`.
    pub offset: NodeRef,
    /// Default `1`.
    pub jitter: NodeRef,
    /// Default `0`.
    pub outmin: NodeRef,
    /// Default `1`.
    pub outmax: NodeRef,
    /// Default `false`; three converts it with `float()`.
    pub clampoutput: NodeRef,
    /// Default `1`.
    pub octaves: NodeRef,
    /// Default `2`.
    pub lacunarity: NodeRef,
    /// Default `.5`.
    pub diminish: NodeRef,
    /// Default `0`.
    pub style: NodeRef,
}

impl UnifiedNoise {
    fn with_defaults(
        noise_type: NodeRef,
        texcoord: NodeRef,
        freq: NodeRef,
        offset: NodeRef,
    ) -> Self {
        Self {
            noise_type,
            texcoord,
            freq,
            offset,
            jitter: 1.0.into(),
            outmin: 0.0.into(),
            outmax: 1.0.into(),
            clampoutput: false.into(),
            octaves: 1.into(),
            lacunarity: 2.0.into(),
            diminish: 0.5.into(),
            style: 0.into(),
        }
    }

    /// `mx_unifiednoise2d( noiseType, texcoord )`'s defaults.
    pub fn new_2d(noise_type: impl Into<NodeRef>, texcoord: NodeRef) -> Self {
        Self::with_defaults(noise_type.into(), texcoord, vec2(1.0, 1.0), vec2(0.0, 0.0))
    }

    /// `mx_unifiednoise3d( noiseType, texcoord )`'s defaults.
    pub fn new_3d(noise_type: impl Into<NodeRef>, texcoord: NodeRef) -> Self {
        Self::with_defaults(
            noise_type.into(),
            texcoord,
            vec3(1.0, 1.0, 1.0),
            vec3(0.0, 0.0, 0.0),
        )
    }

    fn args(self) -> Vec<NodeRef> {
        vec![
            self.noise_type,
            self.texcoord,
            self.freq,
            self.offset,
            self.jitter,
            self.outmin,
            self.outmax,
            self.clampoutput,
            self.octaves,
            self.lacunarity,
            self.diminish,
            self.style,
        ]
    }
}

/// `mx_unifiednoise2d( noiseType, texcoord = uv(), freq, offset, jitter,
/// outmin, outmax, clampoutput, octaves, lacunarity, diminish, style )`.
pub fn mx_unifiednoise2d(params: UnifiedNoise) -> NodeRef {
    mx_noise::mx_unifiednoise(2, params.args())
}

/// `mx_unifiednoise3d( noiseType, position = uv(), … )`.
pub fn mx_unifiednoise3d(params: UnifiedNoise) -> NodeRef {
    mx_noise::mx_unifiednoise(3, params.args())
}

// ---------------------------------------------------------------------------
// colour
// ---------------------------------------------------------------------------

/// `mx_hsvtorgb( hsv )`.
pub fn mx_hsvtorgb(hsv: NodeRef) -> NodeRef {
    super::mx_color::mx_hsvtorgb(hsv.to(Type::Vec3))
}

/// `mx_rgbtohsv( c )`.
pub fn mx_rgbtohsv(c: NodeRef) -> NodeRef {
    super::mx_color::mx_rgbtohsv(c.to(Type::Vec3))
}

/// `mx_srgb_texture_to_lin_rec709( color )`.
pub fn mx_srgb_texture_to_lin_rec709(color: NodeRef) -> NodeRef {
    super::mx_color::mx_srgb_texture_to_lin_rec709(color.to(Type::Vec3))
}

// ---------------------------------------------------------------------------
// texture-coordinate helpers
// ---------------------------------------------------------------------------

/// `vec2( x )` of a node that is already a `vec2`: three still wraps it in a
/// `ConvertNode`, which prints as `x` but is a node of its own, so `x` is
/// reached once however often the wrapper is read and never becomes a var.
/// `mx_place2d` reads `mx_rotate2d`'s input four times and three writes the
/// `( centered / scale )` out four times; `NodeRef::to()` would return `x`
/// itself and let the builder hoist it.
fn convert(x: NodeRef) -> NodeRef {
    let ty = x.ty();
    NodeRef::new(Node::Cast { node: x, ty })
}

/// `mix( a, b, t )` with a `bool` weight, which `MathNode` builds as
/// `f32( t )` — the component count of the weight, not of the output.
fn mix_bool(a: impl Into<NodeRef>, b: impl Into<NodeRef>, t: NodeRef) -> NodeRef {
    let ty = Type::vector_of(Type::F32, t.ty().components());
    mix(a, b, t.to(ty))
}

/// `mx_aastep( threshold, value )` — a step antialiased over one pixel's
/// screen-space footprint of `value`. Three's literal `0.70710678118654757`
/// is the same `f64` as `FRAC_1_SQRT_2`.
pub fn mx_aastep(threshold: impl Into<NodeRef>, value: impl Into<NodeRef>) -> NodeRef {
    let threshold = threshold.into().to(Type::F32);
    let value = value.into().to(Type::F32);
    let afwidth = length(vec2_join(vec![dpdx(&value), dpdy(&value)]))
        .mul(float(std::f64::consts::FRAC_1_SQRT_2));
    smoothstep(threshold.sub(&afwidth), threshold.add(&afwidth), value)
}

/// `mx_ramplr( valuel, valuer, texcoord = uv() )` — left-to-right ramp.
pub fn mx_ramplr(
    valuel: impl Into<NodeRef>,
    valuer: impl Into<NodeRef>,
    texcoord: NodeRef,
) -> NodeRef {
    mix(valuel, valuer, texcoord.x().clamp(float(0.0), float(1.0)))
}

/// `mx_ramptb( valueb, valuet, texcoord = uv() )` — bottom-to-top ramp.
pub fn mx_ramptb(
    valueb: impl Into<NodeRef>,
    valuet: impl Into<NodeRef>,
    texcoord: NodeRef,
) -> NodeRef {
    mix(valueb, valuet, texcoord.y().clamp(float(0.0), float(1.0)))
}

/// `mx_ramp4( valuetl, valuetr, valuebl, valuebr, texcoord = uv() )` —
/// bilinear between four corners.
pub fn mx_ramp4(
    valuetl: impl Into<NodeRef>,
    valuetr: impl Into<NodeRef>,
    valuebl: impl Into<NodeRef>,
    valuebr: impl Into<NodeRef>,
    texcoord: NodeRef,
) -> NodeRef {
    let u = texcoord.x().clamp(float(0.0), float(1.0));
    let v = texcoord.y().clamp(float(0.0), float(1.0));
    let top = mix(valuetl, valuetr, &u);
    let bottom = mix(valuebl, valuebr, &u);
    mix(bottom, top, v)
}

/// `mx_splitlr( valuel, valuer, center, texcoord = uv() )` — an antialiased
/// left/right split at `center`.
pub fn mx_splitlr(
    valuel: impl Into<NodeRef>,
    valuer: impl Into<NodeRef>,
    center: impl Into<NodeRef>,
    texcoord: NodeRef,
) -> NodeRef {
    mix(valuel, valuer, mx_aastep(center, texcoord.x()))
}

/// `mx_splittb( valueb, valuet, center, texcoord = uv() )`.
pub fn mx_splittb(
    valueb: impl Into<NodeRef>,
    valuet: impl Into<NodeRef>,
    center: impl Into<NodeRef>,
    texcoord: NodeRef,
) -> NodeRef {
    mix(valueb, valuet, mx_aastep(center, texcoord.y()))
}

/// `mx_transform_uv( uv_scale = 1, uv_offset = 0, uv_geo = uv() )`.
pub fn mx_transform_uv(
    uv_scale: impl Into<NodeRef>,
    uv_offset: impl Into<NodeRef>,
    uv_geo: NodeRef,
) -> NodeRef {
    uv_geo.mul(uv_scale).add(uv_offset)
}

/// `mx_place2d( texcoord, pivot = vec2( 0, 0 ), scale = vec2( 1, 1 ),
/// rotate = float( 0 ), offset = vec2( 0, 0 ), operationorder = int( 0 ) )`.
pub fn mx_place2d(
    texcoord: impl Into<NodeRef>,
    pivot: impl Into<NodeRef>,
    scale: impl Into<NodeRef>,
    rotate: impl Into<NodeRef>,
    offset: impl Into<NodeRef>,
    operationorder: Place2dOrder,
) -> NodeRef {
    let (pivot, scale, rotate, offset) = (pivot.into(), scale.into(), rotate.into(), offset.into());
    let centered = texcoord.into().sub(&pivot);
    let srt = || {
        mx_rotate2d(convert(centered.div(&scale)), rotate.clone())
            .sub(&offset)
            .add(&pivot)
    };
    let trs = || {
        mx_rotate2d(convert(centered.sub(&offset)), rotate.clone())
            .div(&scale)
            .add(&pivot)
    };
    match operationorder {
        Place2dOrder::Srt => srt(),
        Place2dOrder::Trs => trs(),
        Place2dOrder::Node(order) => {
            // `float( int( 0 ) )` of a constant is folded to `0.0` by TSL.
            let order = match &*order.0 {
                Node::Const { values, .. } if values.len() == 1 => float(values[0]),
                _ => order.to(Type::F32),
            };
            mix(srt(), trs(), step(float(0.5), order))
        }
    }
}

/// `mx_place2d`'s `operationorder`. Three branches on whether it is a JS
/// number (picked at build time) or a node (blended in the shader).
#[derive(Clone)]
pub enum Place2dOrder {
    /// The number `0`: scale, rotate, translate.
    Srt,
    /// Any other number: translate, rotate, scale.
    Trs,
    /// A node: `mix( srt, trs, step( 0.5, float( order ) ) )`. Three's default
    /// is `Node( int( 0 ) )`.
    Node(NodeRef),
}

/// `mx_heighttonormal( input, scale = 1, texcoord = uv() )` — a tangent-space
/// normal from the screen-space derivatives of a height, remapped to `[0, 1]`.
pub fn mx_heighttonormal(
    input: impl Into<NodeRef>,
    scale: impl Into<NodeRef>,
    texcoord: NodeRef,
) -> NodeRef {
    let sobel_scale = float(1.0 / 16.0);
    let height = input.into().to(Type::F32);
    let uv_node = convert(texcoord.to(Type::Vec2));
    let d_hds = vec2_join(vec![dpdx(&height), dpdy(&height)])
        .mul(scale.into().to(Type::F32))
        .mul(sobel_scale);
    let d_uds = vec2_join(vec![dpdx(uv_node.x()), dpdy(uv_node.x())]);
    let d_vds = vec2_join(vec![dpdx(uv_node.y()), dpdy(uv_node.y())]);
    let tangent = vec3_join(vec![d_uds.x(), d_vds.x(), d_hds.x()]);
    let bitangent = vec3_join(vec![d_uds.y(), d_vds.y(), d_hds.y()]);
    let n = tangent.cross(bitangent);
    let invalid = n.dot(&n).less_than(float(1e-12));
    let n = mix_bool(&n, vec3(0.0, 0.0, 1.0), invalid);
    let mirrored = n.z().less_than(float(0.0));
    let n = mix_bool(&n, n.mul(float(-1.0)), mirrored);
    n.normalize().mul(float(0.5)).add(float(0.5))
}

// ---------------------------------------------------------------------------
// math
// ---------------------------------------------------------------------------

/// `mx_safepower( in1, in2 = 1 )` — `pow( abs( in1 ), in2 ) * sign( in1 )`.
pub fn mx_safepower(in1: impl Into<NodeRef>, in2: impl Into<NodeRef>) -> NodeRef {
    let in1 = in1.into().to(Type::F32);
    in1.abs().pow(in2).mul(sign(&in1))
}

/// `mx_contrast( input, amount = 1, pivot = .5 )`.
pub fn mx_contrast(
    input: impl Into<NodeRef>,
    amount: impl Into<NodeRef>,
    pivot: impl Into<NodeRef>,
) -> NodeRef {
    let pivot = pivot.into();
    input
        .into()
        .to(Type::F32)
        .sub(&pivot)
        .mul(amount)
        .add(pivot)
}

/// `mx_smoothstep( in, low = 0, high = 1 )` — MaterialX's smoothstep, which
/// degrades to a step when `high <= low` instead of dividing by zero.
pub fn mx_smoothstep(
    input: impl Into<NodeRef>,
    low: impl Into<NodeRef>,
    high: impl Into<NodeRef>,
) -> NodeRef {
    let (input, low, high) = (input.into(), low.into(), high.into());
    let range = high.sub(&low);
    let safe_range = range.abs().max(float(1e-6));
    let t = input
        .sub(&low)
        .div(safe_range)
        .clamp(float(0.0), float(1.0));
    let hermite = t.mul(&t).mul(float(3.0).sub(float(2.0).mul(&t)));
    let fallback = step(&high, &input);
    let use_fallback = step(&high, &low);
    mix(hermite, fallback, use_fallback)
}

/// `mx_add( in1, in2 = float( 0 ) )`.
pub fn mx_add(in1: impl Into<NodeRef>, in2: impl Into<NodeRef>) -> NodeRef {
    in1.into().add(in2)
}

/// `mx_subtract( in1, in2 = float( 0 ) )`.
pub fn mx_subtract(in1: impl Into<NodeRef>, in2: impl Into<NodeRef>) -> NodeRef {
    in1.into().sub(in2)
}

/// `mx_multiply( in1, in2 = float( 1 ) )`.
pub fn mx_multiply(in1: impl Into<NodeRef>, in2: impl Into<NodeRef>) -> NodeRef {
    in1.into().mul(in2)
}

/// `mx_divide( in1, in2 = float( 1 ) )`.
pub fn mx_divide(in1: impl Into<NodeRef>, in2: impl Into<NodeRef>) -> NodeRef {
    in1.into().div(in2)
}

/// `mx_modulo( in1, in2 = float( 1 ) )` — `in1 - in2 * floor( in1 / in2 )`,
/// GLSL's `mod`, not WGSL's truncating `%`.
pub fn mx_modulo(in1: impl Into<NodeRef>, in2: impl Into<NodeRef>) -> NodeRef {
    let (in1, in2) = (in1.into(), in2.into());
    in1.sub(in2.mul(in1.div(&in2).floor()))
}

/// `mx_power( in1, in2 = float( 1 ) )`.
pub fn mx_power(in1: impl Into<NodeRef>, in2: impl Into<NodeRef>) -> NodeRef {
    in1.into().pow(in2)
}

/// `mx_atan2( in1 = float( 0 ), in2 = float( 1 ) )` — `atan( y, x )`.
pub fn mx_atan2(in1: impl Into<NodeRef>, in2: impl Into<NodeRef>) -> NodeRef {
    in1.into().atan2(in2)
}

/// `mx_timer()` — three's `time`.
pub fn mx_timer() -> NodeRef {
    time()
}

/// `mx_invert( in1, amount = float( 1 ) )` — `amount - in1`.
pub fn mx_invert(in1: impl Into<NodeRef>, amount: impl Into<NodeRef>) -> NodeRef {
    amount.into().sub(in1)
}

/// `mx_ifgreater( value1, value2, in1, in2 )` — `in1` where `value1 > value2`.
pub fn mx_ifgreater(
    value1: impl Into<NodeRef>,
    value2: impl Into<NodeRef>,
    in1: impl Into<NodeRef>,
    in2: impl Into<NodeRef>,
) -> NodeRef {
    mix_bool(in2, in1, value1.into().greater_than(value2))
}

/// `mx_ifgreatereq( value1, value2, in1, in2 )`.
pub fn mx_ifgreatereq(
    value1: impl Into<NodeRef>,
    value2: impl Into<NodeRef>,
    in1: impl Into<NodeRef>,
    in2: impl Into<NodeRef>,
) -> NodeRef {
    mix_bool(in2, in1, value1.into().greater_than_equal(value2))
}

/// `mx_ifequal( value1, value2, in1, in2 )`.
pub fn mx_ifequal(
    value1: impl Into<NodeRef>,
    value2: impl Into<NodeRef>,
    in1: impl Into<NodeRef>,
    in2: impl Into<NodeRef>,
) -> NodeRef {
    mix_bool(in2, in1, value1.into().equal(value2))
}

/// `mx_separate( in1, channelOrOut = null )`'s second argument.
#[derive(Clone, Copy, Debug)]
pub enum MxChannel<'a> {
    /// `null`: the input itself.
    Whole,
    /// A channel name — `'x'`/`'r'` … `'w'`/`'a'`, optionally `out`-prefixed
    /// (`'outy'`), any case. An unknown name returns the input.
    Name(&'a str),
    /// A component index.
    Index(usize),
}

/// `mx_separate( in1, channelOrOut )` — one component of `in1`.
pub fn mx_separate(in1: NodeRef, channel: MxChannel<'_>) -> NodeRef {
    let index = |c: &str| match c {
        "x" | "r" => Some(0),
        "y" | "g" => Some(1),
        "z" | "b" => Some(2),
        "w" | "a" => Some(3),
        _ => None,
    };
    match channel {
        MxChannel::Whole => in1,
        MxChannel::Index(i) => in1.element(i),
        MxChannel::Name(name) => {
            let c = name.strip_prefix("out").unwrap_or(name).to_lowercase();
            match index(&c) {
                Some(i) => in1.element(i),
                None => in1,
            }
        }
    }
}
