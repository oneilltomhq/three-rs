//! Port of `three.js/examples/jsm/tsl/display/TransitionNode.js`.

use crate::nodes::node::Type;
use crate::nodes::tsl::{block, float, if_else, int, mix, texture_uv, to_var, uv, vec4};
use crate::nodes::NodeRef;
use crate::textures::Texture;

/// `transition( nodeA, nodeB, mixTextureNode, mixRatio, threshold, useTexture )`.
///
/// With `use_texture == 1` the mix texture's red channel, shifted by
/// `mix_ratio` and sharpened by `threshold`, picks between A and B per
/// fragment; otherwise it is a plain cross-fade from B to A by `mix_ratio`.
///
/// All three inputs are textures, sampled at `uv()`. Three.js lets the page
/// swap `mixTextureNode.value` every frame; a texture is an identity in the
/// port's graph, so the mix texture is fixed when the node is built.
pub fn transition(
    a: &Texture,
    b: &Texture,
    mix_texture: &Texture,
    mix_ratio: NodeRef,
    threshold: NodeRef,
    use_texture: NodeRef,
) -> NodeRef {
    let texel_one = texture_uv(a, uv());
    let texel_two = texture_uv(b, uv());

    let color = to_var(None, vec4(0.0, 0.0, 0.0, 0.0));

    let transition_texel = texture_uv(mix_texture, uv());
    let r = mix_ratio
        .clone()
        .mul(threshold.clone().mul(2.0).add(1.0))
        .sub(threshold.clone());
    let mixf = transition_texel
        .x()
        .sub(r)
        .mul(float(1.0).div(threshold))
        .clamp(0.0, 1.0);

    // `useTextureNode.equal( int( 1 ) )`: the page's `uniform( 1 )` is a
    // float, and three's `OperatorNode` converts the `int` to it.
    let use_texture = use_texture.equal(int(1).to(Type::F32));

    block(
        vec![
            color.clone(),
            if_else(
                use_texture,
                vec![color.assign(mix(texel_one.clone(), texel_two.clone(), mixf))],
                vec![color.assign(mix(texel_two, texel_one, mix_ratio))],
            ),
        ],
        color,
    )
}
