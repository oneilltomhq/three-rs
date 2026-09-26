//! Port of `three.js/examples/jsm/tsl/display/DotScreenNode.js`.

use crate::nodes::tsl::{uv, vec2_join, vec3_join, vec4_join, viewport_size};
use crate::nodes::NodeRef;

/// `dotScreen( node, angle = 1.57, scale = 1 )` — a halftone dot pattern over
/// the average of `input`'s three channels.
///
/// `input` is a node, not a texture: the effect reads it once, at the
/// fragment being shaded, so three.js does not convert it.
///
/// `screenSize` is the port's `viewport_size()`: three's `screenSize` is the
/// drawing buffer's size and `viewportSize` the bound target's, and the two
/// are the same for every full-screen pass this node is drawn in.
pub fn dot_screen(input: NodeRef, angle: NodeRef, scale: NodeRef) -> NodeRef {
    let pattern = {
        let s = angle.sin();
        let c = angle.cos();
        let tex = uv().mul(viewport_size());
        let point = vec2_join(vec![
            c.clone().mul(tex.x()).sub(s.clone().mul(tex.y())),
            s.mul(tex.x()).add(c.mul(tex.y())),
        ])
        .mul(scale);
        point.x().sin().mul(point.y().sin()).mul(4.0)
    };

    let color = input;
    let average = color.x().add(color.y()).add(color.z()).div(3.0);
    vec4_join(vec![
        vec3_join(vec![average.mul(10.0).sub(5.0).add(pattern)]),
        color.a(),
    ])
}
