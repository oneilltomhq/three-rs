//! Port of `examples/jsm/tsl/utils/RNoise.js`: texture-free analytic R² noise,
//! the per-pixel random numbers `SSRNode`'s stochastic mode and
//! `RecurrentDenoiseNode` draw. Gated against three's dump of
//! `tools/dump-pages/specular_helpers.html` in `tests/nodes_display_wgsl.rs`
//! (`docs/nodes.md` §85).

use super::{block, float, floor, fract, int, mod_, to_var, vec2_join, vec4, vec4_join};
use crate::nodes::node::{NodeRef, Type};

/// The plastic number, `P³ = P + 1`, whose powers drive the R² sequence.
const P: f64 = 1.324_717_957_244_746;

/// `bindAnalyticNoise( resolution, seed )` — returns the sampler `( uv,
/// sampleIndex ) => vec4`, three's `Fn( ( [ uvCoord, sampleIndex ] ) => … )`.
///
/// `resolution` is the `vec2` uniform holding the target size in pixels, and
/// `seed` is added to the index and to the coordinate hash so each pass gets
/// an independent R² phase (three's default is `0`). Index 0 samples
/// continuous screen pixels; other indices tile-shift by an R² sequence into a
/// 32×32 period. The four components are independent R² dimensions.
///
/// The returned closure is a plain `Fn` (no layout), so each call inlines its
/// body where it is used, as three's does.
pub fn bind_analytic_noise(
    resolution: NodeRef,
    seed: i32,
) -> impl Fn(NodeRef, NodeRef) -> NodeRef + Clone + 'static {
    move |uv_coord: NodeRef, sample_index: NodeRef| {
        analytic_noise(&resolution, seed, uv_coord, sample_index)
    }
}

/// The body of [`bind_analytic_noise`]'s returned `Fn`.
#[inline(never)]
fn analytic_noise(
    resolution: &NodeRef,
    seed: i32,
    uv_coord: NodeRef,
    sample_index: NodeRef,
) -> NodeRef {
    let index = sample_index.to(Type::I32).add(int(seed as i64));
    // `vec4()` with no arguments is `( 0, 0, 0, 1 )`.
    let noise = to_var(None, vec4(0.0, 0.0, 0.0, 1.0));
    let tile_size = float(32.0);

    let screen_pixel = floor(uv_coord.mul(resolution.clone()));
    let offset = floor(
        fract(vec2_join(vec![
            index.to(Type::F32).mul(0.7548776662),
            index.to(Type::F32).mul(0.5698402910),
        ]))
        .mul(tile_size.clone()),
    );
    let coords = mod_(screen_pixel.add(offset), tile_size);

    block(vec![noise.clone(), noise.assign(r4(coords, seed))], noise)
}

/// `r4( coords )` — four R² dimensions hashed from the sample coordinates.
fn r4(coords: NodeRef, seed: i32) -> NodeRef {
    let t = coords
        .x()
        .mul(1.0 / P)
        .add(coords.y().mul(1.0 / P.powi(2)))
        .add(float(seed));
    vec4_join(vec![
        fract(t.mul(P).mul(1.0 / P)),
        fract(t.mul(P * 2.0).mul(1.0 / P.powi(2))),
        // Not 1 / P³: three's magic constant, which it says gives better noise.
        fract(t.mul(P * 3.0).mul(0.4198754210)),
        fract(t.mul(P * 4.0).mul(1.0 / P.powi(3))),
    ])
}
