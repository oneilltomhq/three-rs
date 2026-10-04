//! Port of `bayerDither` from `three.js/examples/jsm/tsl/math/Bayer.js`.
//!
//! It is a `tsl/math` addon, not a `tsl/display` one, but its only use is as
//! a display effect (`webgpu_postprocessing_retro` dithers before it
//! posterizes), so it lives with them.
//!
//! **Not ported:** `bayer16( uv )`, the same file's 16×16 Bayer-matrix
//! texture lookup, which decodes an embedded PNG; no ported page uses it.

use crate::nodes::tsl::{float, mod_float, screen_size, screen_uv, vec3_join};
use crate::nodes::NodeRef;

/// `bayerDither( color, steps = 32 )` — `color.rgb` plus a structured offset
/// in `[ -0.5, 0.5 ) / steps` from a 4×4 pattern over the screen's pixels,
/// to be quantised by [`posterize`](crate::nodes::tsl::posterize) with the
/// same `steps`:
///
/// `( mod( floor( x + 1 ) · floor( y + 1 ) · 17, 16 ) / 16 - 0.5 ) / steps`,
/// with `x` and `y` the pixel's coordinates `mod 4`.
///
/// Three's comment calls the pattern a "simplified Bayer matrix
/// approximation"; it is ported as written. A `vec3`.
pub fn bayer_dither(color: impl Into<NodeRef>, steps: impl Into<NodeRef>) -> NodeRef {
    let (color, steps) = (color.into(), steps.into());

    let screen_pos = screen_uv().mul(screen_size());
    let x = mod_float(screen_pos.x().floor(), float(4.0));
    let y = mod_float(screen_pos.y().floor(), float(4.0));

    // Simplified Bayer matrix approximation
    let bayer = mod_float(
        x.add(1.0).floor().mul(y.add(1.0).floor()).mul(17.0),
        float(16.0),
    )
    .div(16.0)
    .sub(0.5);

    // Apply dither offset before quantization
    let dither_offset = bayer.div(steps);

    vec3_join(vec![
        color.x().add(dither_offset.clone()),
        color.y().add(dither_offset.clone()),
        color.z().add(dither_offset),
    ])
}
