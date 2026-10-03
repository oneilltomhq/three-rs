//! Port of `three.js/examples/jsm/tsl/display/MotionBlur.js`.

use crate::nodes::node::Type;
use crate::nodes::tsl::{block, float, int, loop_options, texture_uv, to_var, uv};
use crate::nodes::NodeRef;
use crate::textures::Texture;

/// `motionBlur( inputNode, velocity, numSamples = int( 16 ) )` — the colour
/// at `uv()` plus `numSamples` more taps along the velocity vector, from half
/// a step behind the pixel to half a step ahead, averaged.
///
/// As with [`radial_blur`](super::radial_blur), the addon's `inputNode` is a
/// texture node whose only use is `inputNode.sample( uv )`, so this port takes
/// the texture. `velocity` is a node, read inside the loop, and is usually the
/// pass's `velocity` attachment scaled by a uniform.
///
/// The sum has `numSamples + 1` taps (the centre plus `i = 1 ..= numSamples`)
/// and is divided by `numSamples`, so a still pixel comes out brighter by
/// `( numSamples + 1 ) / numSamples`. That is three's arithmetic, kept.
pub fn motion_blur(input: &Texture, velocity: NodeRef, num_samples: i32) -> NodeRef {
    let uvs = uv();
    // `const colorResult = sampleColor( uvs ).toVar()`.
    let color_result = to_var(None, texture_uv(input, uvs.clone()));
    // `const fSamples = float( numSamples )`.
    let f_samples = float(num_samples as f64);

    let loop_body = {
        let (input, color_result, f_samples) =
            (input.clone(), color_result.clone(), f_samples.clone());
        move |i: &NodeRef| {
            // `const offset = velocity.mul( float( i ).div( fSamples.sub( 1 )
            // ).sub( 0.5 ) )`; `uvs.add( offset )` is a `vec4`, and
            // `sample()` reads its `.xy`.
            let offset = velocity.mul(
                i.to(Type::F32)
                    .div(f_samples.sub(float(1.0)))
                    .sub(float(0.5)),
            );
            vec![color_result.add_assign(texture_uv(&input, uvs.add(offset).xy()))]
        }
    };

    block(
        vec![
            color_result.clone(),
            // `Loop( { start: int( 1 ), end: numSamples, type: 'int',
            // condition: '<=' }, … )`.
            loop_options(
                "i",
                Type::I32,
                int(1),
                int(num_samples as i64),
                "<=",
                loop_body,
            ),
            color_result.div_assign(f_samples),
        ],
        color_result,
    )
}
