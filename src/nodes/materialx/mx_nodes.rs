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
use crate::nodes::tsl::{call, vec2, vec4_join};

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
