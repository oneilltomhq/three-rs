//! Port of `three.js/examples/jsm/tsl/display/Sepia.js`.

use crate::nodes::tsl::{dot, vec3, vec4_join};
use crate::nodes::NodeRef;

/// `sepia( color )` — glfx.js' sepia matrix applied to `color.rgb`, with
/// `color.a` kept:
///
/// `vec4( dot( c, ( 0.393, 0.769, 0.189 ) ), dot( c, ( 0.349, 0.686, 0.168 )
/// ), dot( c, ( 0.272, 0.534, 0.131 ) ), color.a )`.
///
/// A `Fn()` with no layout, so inlined.
pub fn sepia(color: impl Into<NodeRef>) -> NodeRef {
    let color = color.into();
    // `vec3( color )` of a `vec4` is its `.xyz`.
    let c = color.rgb();

    // https://github.com/evanw/glfx.js/blob/master/src/filters/adjust/sepia.js
    vec4_join(vec![
        dot(c.clone(), vec3(0.393, 0.769, 0.189)),
        dot(c.clone(), vec3(0.349, 0.686, 0.168)),
        dot(c, vec3(0.272, 0.534, 0.131)),
        color.a(),
    ])
}
