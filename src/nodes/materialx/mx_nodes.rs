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

use crate::nodes::node::{NodeRef, Type};
use crate::nodes::tsl::{call, int, vec2, vec3, vec4_join};

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
