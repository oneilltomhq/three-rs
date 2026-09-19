//! Port of `three.js/examples/jsm/tsl/display/ChromaticAberrationNode.js`.

use crate::nodes::node::Type;
use crate::nodes::tsl::{call, float, length, shader_fn, texture_uv, to_var, uv, vec4_join};
use crate::nodes::NodeRef;
use crate::textures::Texture;

/// `chromaticAberration( node, strength, center, scale )`.
///
/// The addon takes a *texture node* and calls `convertToTexture()` on whatever
/// it is given, which is why the page's `renderOutput( pass( … ) )` becomes an
/// [`RttNode`](crate::renderer::RttNode) draw of its own. This port takes the
/// texture that RTT produced, for the same reason
/// [`radial_blur`](super::radial_blur) does: the only thing the node does with
/// the texture node is `sample( uv )`, and an `RTTNode`'s `uvNode` is the raw
/// `uv()` varying.
///
/// Three declares the body with `setLayout( { name: 'ChromaticAberrationShader',
/// … } )`, so the four values arrive as WGSL function parameters rather than as
/// uniforms read inside the body — the texture is the only thing the body
/// closes over.
pub fn chromatic_aberration(
    map: &Texture,
    strength: NodeRef,
    center: NodeRef,
    scale: NodeRef,
) -> NodeRef {
    let map = map.clone();
    let def = shader_fn(
        Some("ChromaticAberrationShader"),
        vec![
            ("uv", Type::Vec2),
            ("strength", Type::F32),
            ("center", Type::Vec2),
            ("scale", Type::F32),
        ],
        Type::Vec4,
        move |args| {
            let (uv, strength, center, scale) = (
                args[0].clone(),
                args[1].clone(),
                args[2].clone(),
                args[3].clone(),
            );

            // `const offset = uv.sub( center ); const distance = offset.length();`
            // — `offset` is read eight times, so three's usage count promotes
            // it; `distance` is read once and stays inline.
            let offset = to_var(None, uv.sub(center.clone()));
            let distance = length(offset.clone());

            // The red channel is scaled outward, blue inward, green not at all.
            let step = scale.mul(float(0.02)).mul(strength.clone());
            let red_scale = float(1.0).add(step.clone());
            let green_scale = float(1.0);
            let blue_scale = float(1.0).sub(step);

            // `const aberrationStrength = strength.mul( distance );`
            let aberration_strength = to_var(None, strength.mul(distance));
            let radial = offset.mul(aberration_strength);

            let channel_uv = |channel_scale: NodeRef, chromatic: f64| {
                center
                    .add(offset.mul(channel_scale))
                    .add(radial.mul(float(chromatic)))
            };

            let r = to_var(None, texture_uv(&map, channel_uv(red_scale, 0.01)));
            let g = to_var(None, texture_uv(&map, channel_uv(green_scale, 0.0)));
            let b = to_var(None, texture_uv(&map, channel_uv(blue_scale, -0.01)));
            let a = to_var(None, texture_uv(&map, uv));

            vec4_join(vec![r.x(), g.y(), b.z(), a.w()])
        },
    );

    call(&def, vec![uv(), strength, center, scale])
}
