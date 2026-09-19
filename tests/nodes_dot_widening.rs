//! `dot( a, b )` builds both operands at the node's *input* type.
//!
//! `MathNode.generate()`'s generic branch is
//! `params.push( a.build( builder, inputType ) ); params.push( b.build(
//! builder, inputType ) )`, and `inputType` is the widest of the operands —
//! not the result type, which for `dot` is a scalar. So `luminance( vec4 )`,
//! which is `dot( color, vec3( 0.2126, 0.7152, 0.0722 ) )` with a **vec4**
//! colour, comes out as
//!
//! ```wgsl
//! dot( color, vec4<f32>( vec3<f32>( 0.2126, 0.7152, 0.0722 ), 1.0 ) )
//! ```
//!
//! — the alpha channel weighted 1.0, because `NodeBuilder.format()` pads a
//! `vec3` into a `vec4` with a one. `webgpu_postprocessing_difference` takes
//! `luminance()` of a frame *difference*, whose alpha is 0 at frame 0 against
//! a 1.0 current alpha, so a vec3-only reading changes the saturation of every
//! lit texel. Nothing shows in the picture as an error; it shows as the wrong
//! colour.
//!
//! No GPU: this reads the generated WGSL.

use three_rs::materials::{setup, MeshBasicNodeMaterial, SetupContext};
use three_rs::nodes::tsl::{luminance, texture, vec3};
use three_rs::nodes::NodeBuilder;
use three_rs::textures::Texture;

fn fragment(node: three_rs::nodes::NodeRef) -> String {
    let mut material = MeshBasicNodeMaterial::new();
    material.fragment_node = Some(node);
    let flow = setup(&material, &SetupContext::default(), None);
    NodeBuilder::new().build(&flow).fragment_wgsl
}

#[test]
fn luminance_of_a_vec4_weights_alpha_by_one() {
    let map = Texture::new(4, 4, Some(vec![0; 64]));
    let wgsl = fragment(luminance(texture(&map)));

    assert!(
        wgsl.contains("vec4<f32>( vec3<f32>( 0.2126, 0.7152, 0.0722 ), 1.0 )"),
        "the vec3 coefficients were not padded into the vec4 operand:\n{wgsl}"
    );
}

/// The vec3 case — every `luminance()` on the ladder before this one — is
/// untouched: both operands are already `vec3`, so `format()` is the identity.
#[test]
fn luminance_of_a_vec3_is_unchanged() {
    let wgsl = fragment(vec3(1.0, 1.0, 1.0).mul(luminance(vec3(0.25, 0.5, 0.75))));

    assert!(
        wgsl.contains("dot( vec3<f32>( 0.25, 0.5, 0.75 ), vec3<f32>( 0.2126, 0.7152, 0.0722 ) )"),
        "a vec3 dot vec3 grew a conversion:\n{wgsl}"
    );
}
