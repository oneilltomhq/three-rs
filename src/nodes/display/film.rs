//! Port of `three.js/examples/jsm/tsl/display/FilmNode.js`.

use crate::nodes::tsl::{fract, mix, rand, time, uv, vec4_join};
use crate::nodes::NodeRef;

/// `film( inputNode, intensityNode = null, uvNode = null )` — `FilmNode`:
/// film grain. Each fragment's colour is brightened by a noise of its uv and
/// the time, `rgb + rgb · clamp( rand( fract( uv + time ) ) + 0.1, 0, 1 )`.
/// With an intensity the result is `mix( rgb, grained, intensity )`, so 0
/// leaves the input as it is. The input's alpha is kept.
///
/// Three's `FilmNode` is a `Node` whose `setup()` returns that `Fn()`'s
/// call, with no layout, so the graph is inlined into the shader that reads
/// it; the port returns the graph. `uv` defaults to `uv()`.
pub fn film(
    input: impl Into<NodeRef>,
    intensity: Option<NodeRef>,
    uv_node: Option<NodeRef>,
) -> NodeRef {
    let input = input.into();
    let uv_node = uv_node.unwrap_or_else(uv);

    let base = input.rgb();
    let noise = rand(fract(uv_node.add(time())));

    let mut color = base
        .clone()
        .add(base.clone().mul(noise.add(0.1).clamp(0.0, 1.0)));

    if let Some(intensity) = intensity {
        color = mix(base, color, intensity);
    }

    vec4_join(vec![color, input.a()])
}
