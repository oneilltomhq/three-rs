//! Port of `three.js/examples/jsm/tsl/display/Shape.js`.

use crate::nodes::tsl::{length, smoothstep};
use crate::nodes::NodeRef;

/// `circle( scale = 1, softness = 0.5, coord = uv() )` — a soft disc: 1 inside
/// `scale`, falling to 0 over the outer `softness` fraction of it, measured as
/// `length( coord - 0.5 ) * 2`, so `scale = 1` touches the edges of the unit
/// square and `1.42` (≈ √2) its corners.
///
/// The `Fn()` has no layout, so three inlines it; this is a plain function
/// for the same reason. Three's defaults are the arguments to pass for its
/// `circle()`: `float( 1 )`, `float( 0.5 )` and `uv()`.
///
/// Not core TSL's `shapeCircle()`
/// ([`shape_circle`](crate::nodes::tsl::shape_circle)), which is the points
/// material's round sprite.
pub fn circle(
    scale: impl Into<NodeRef>,
    softness: impl Into<NodeRef>,
    coord: impl Into<NodeRef>,
) -> NodeRef {
    let (scale, softness, coord) = (scale.into(), softness.into(), coord.into());

    // Centre the coordinates (-0.5 to 0.5), and measure from the centre: 0
    // there, ~0.707 * 2 at the corners.
    let centered = coord.sub(0.5);
    let dist = length(centered).mul(2.0);

    // The two edges, and the smoothstep across them.
    let outer = scale.clone();
    let inner = scale.clone().sub(softness.mul(scale));
    smoothstep(outer, inner, dist)
}
