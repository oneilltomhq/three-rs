//! Port of `three.js/examples/jsm/tsl/display/CRT.js` — the cathode-ray-tube
//! effects `webgpu_postprocessing_retro` stacks on its retro pass:
//! [`barrel_uv`], [`barrel_mask`], [`color_bleeding`], [`scanlines`] and
//! [`vignette`].
//!
//! Every one is a `Fn()` with no layout, which three inlines into the shader
//! that uses it, so each is a plain function here. Three's parameter defaults
//! are given in each function's documentation; the port takes every argument
//! explicitly, as the page passes them.

use super::rtt::{convert_to_texture, owned_by};
use super::shape::circle;
use crate::nodes::tsl::{dot, float, mix, screen_uv, time, vec2_join, vec3_join};
use crate::nodes::NodeRef;

/// `barrelUV( curvature = 0.1, coord = uv() )` — `coord` pushed outward from
/// the centre, a CRT screen's bulge, and scaled back so the corners stay where
/// they were:
///
/// `centered / ( 1 - r² · curvature ) · ( 1 - 2 · curvature ) · 0.5 + 0.5`,
/// with `centered = ( coord - 0.5 ) · 2` and `r² = dot( centered, centered )`.
///
/// The result is a uv, for [`replace_default_uv`](crate::nodes::tsl::replace_default_uv)
/// to hand to a pass, or for [`barrel_mask`] to test.
pub fn barrel_uv(curvature: impl Into<NodeRef>, coord: impl Into<NodeRef>) -> NodeRef {
    let (curvature, coord) = (curvature.into(), coord.into());

    // Centre the coordinates (-1 to 1), and the squared distance from the
    // centre.
    let centered = coord.sub(0.5).mul(2.0);
    let r2 = dot(centered.clone(), centered.clone());

    // The bulge, and the scale that compensates for it at the corners, where
    // r² = 2.
    let distortion = float(1.0).sub(r2.mul(curvature.clone()));
    let corner_distortion = float(1.0).sub(curvature.mul(2.0));

    centered
        .div(distortion)
        .mul(corner_distortion)
        .mul(0.5)
        .add(0.5)
}

/// `barrelMask( coord )` — 0 where a [`barrel_uv`] result falls outside the
/// unit square, 1 inside: `select( coord.x < 0 || coord.x > 1 || coord.y < 0
/// || coord.y > 1, 0, 1 )`.
pub fn barrel_mask(coord: impl Into<NodeRef>) -> NodeRef {
    let coord = coord.into();
    let out_of_bounds = coord
        .x()
        .less_than(0.0)
        .or(coord.x().greater_than(1.0))
        .or(coord.y().less_than(0.0))
        .or(coord.y().greater_than(1.0));
    out_of_bounds.select(float(0.0), float(1.0))
}

/// `colorBleeding( color, amount = 0.002 )` — an analogue signal's colour
/// trailing to the right: `color` is drawn to a texture
/// (`convertToTexture( color )`, an [`rtt`](super::rtt) quad of its own) and
/// read back at the fragment and at one, two and three `amount`s to its
/// left. Red takes the most of its neighbours (0.4, 0.2, 0.1), green less
/// (0.25, 0.1) and blue least (0.15); each channel is normalised by its
/// weights' sum and clamped to `[0, 1]`. A `vec3`.
///
/// The taps are at `screenUV`, so the texture is read where the fragment is,
/// whatever uv the quad that reads it was given.
pub fn color_bleeding(color: impl Into<NodeRef>, amount: impl Into<NodeRef>) -> NodeRef {
    let (color, amount) = (color.into(), amount.into());
    let input_texture = convert_to_texture(color);

    // The original colour.
    let original = input_texture.sample(screen_uv()).rgb();

    // Colours from the left, as a signal trails.
    let left = |k: f64| {
        let offset = if k == 1.0 {
            amount.clone()
        } else {
            amount.clone().mul(k)
        };
        input_texture
            .sample(screen_uv().sub(vec2_join(vec![offset, float(0.0)])))
            .rgb()
    };
    let (left1, left2, left3) = (left(1.0), left(2.0), left(3.0));

    // Red bleeds most, green medium, blue least.
    let bleed_r = original
        .x()
        .add(left1.x().mul(0.4))
        .add(left2.x().mul(0.2))
        .add(left3.x().mul(0.1));
    let bleed_g = original
        .y()
        .add(left1.y().mul(0.25))
        .add(left2.y().mul(0.1));
    let bleed_b = original.z().add(left1.z().mul(0.15));

    // Normalise and clamp.
    let r = bleed_r.div(1.7).clamp(0.0, 1.0);
    let g = bleed_g.div(1.35).clamp(0.0, 1.0);
    let b = bleed_b.div(1.15).clamp(0.0, 1.0);

    // The `rtt()` is the graph's: it is drawn for as long as the result is
    // read.
    owned_by(vec3_join(vec![r, g, b]), input_texture)
}

/// `scanlines( color, intensity = 0.3, count = 240, speed = 0, coord = uv() )`
/// — `color` darkened by a sine across `coord.y`, `count` periods over the
/// screen, scrolling down at `speed`:
///
/// `color · ( 1 - ( sin( ( coord.y - time · speed ) · count ) · 0.5 + 0.5 )
/// · intensity )`.
pub fn scanlines(
    color: impl Into<NodeRef>,
    intensity: impl Into<NodeRef>,
    count: impl Into<NodeRef>,
    speed: impl Into<NodeRef>,
    coord: impl Into<NodeRef>,
) -> NodeRef {
    let (color, intensity, count, speed, coord) = (
        color.into(),
        intensity.into(),
        count.into(),
        speed.into(),
        coord.into(),
    );

    // The scroll, as a CRT's vertical-sync roll.
    let animated_y = coord.y().sub(time().mul(speed));

    // The pattern, and how much of it darkens.
    let scanline = animated_y.mul(count).sin();
    let scanline_intensity = scanline.mul(0.5).add(0.5).mul(intensity);

    color.mul(float(1.0).sub(scanline_intensity))
}

/// `vignette( color, intensity = 0.4, smoothness = 0.5, coord = uv() )` —
/// `color` darkened towards the corners to `1 - intensity` of itself, by a
/// [`circle`] of scale 1.42 (≈ √2, so the falloff reaches the corners) and
/// softness `smoothness`.
pub fn vignette(
    color: impl Into<NodeRef>,
    intensity: impl Into<NodeRef>,
    smoothness: impl Into<NodeRef>,
    coord: impl Into<NodeRef>,
) -> NodeRef {
    let (color, intensity) = (color.into(), intensity.into());

    // The radial gradient.
    let mask = circle(float(1.42), smoothness, coord);

    // Centre 1, edges `1 - intensity`.
    let vignette_amount = mix(float(1.0).sub(intensity), float(1.0), mask);

    color.mul(vignette_amount)
}
