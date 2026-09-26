//! Port of `three.js/examples/jsm/tsl/display/RGBShiftNode.js`.

use crate::nodes::tsl::{texture_uv, uv, vec2_join, vec4_join};
use crate::nodes::NodeRef;
use crate::textures::Texture;

/// `rgbShift( node, amount = 0.005, angle = 0 )` — red and blue sampled at
/// opposite offsets along `angle`, green and alpha in place.
///
/// Takes the texture three's `convertToTexture()` would have made; the uv is
/// the raw `uv()` a pass or `rtt()` texture node carries.
pub fn rgb_shift(map: &Texture, amount: NodeRef, angle: NodeRef) -> NodeRef {
    let uv_node = uv();
    let offset = vec2_join(vec![angle.cos(), angle.sin()]).mul(amount);
    let cr = texture_uv(map, uv_node.clone().add(offset.clone()));
    let cga = texture_uv(map, uv_node.clone());
    let cb = texture_uv(map, uv_node.sub(offset));
    vec4_join(vec![cr.x(), cga.y(), cb.z(), cga.a()])
}
