//! Port of `three.js/examples/jsm/tsl/display/hashBlur.js`.

use crate::nodes::node::Type;
use crate::nodes::tsl::{
    block, float, loop_options, premultiply_alpha, rand, texture_uv, to_const, to_var,
    unpremultiply_alpha, uv, vec2_join, vec4,
};
use crate::nodes::NodeRef;
use crate::textures::Texture;

/// `hashBlur( textureNode, bluramount, options )`' options.
#[derive(Clone)]
pub struct HashBlurOptions {
    /// `options.repeats` — the number of taps, `float( 45 )` by default.
    pub repeats: NodeRef,
    /// `options.premultipliedAlpha`.
    pub premultiplied_alpha: bool,
}

impl Default for HashBlurOptions {
    fn default() -> Self {
        Self {
            repeats: float(45.0),
            premultiplied_alpha: false,
        }
    }
}

/// `hashBlur( textureNode, bluramount = 0.1, options )` — `repeats` taps
/// scattered round a circle by a per-fragment hash, averaged.
///
/// Takes the texture three's `convertToTexture()` would have made, sampled at
/// `uv()`. [`hash_blur_with`] is the general form.
pub fn hash_blur(map: &Texture, blur_amount: NodeRef, options: HashBlurOptions) -> NodeRef {
    hash_blur_with(|coord| texture_uv(map, coord), uv(), blur_amount, options)
}

/// [`hash_blur`] over any texture node: `tap( uv )` is the only thing three
/// does with its `textureNode`, and `target_uv` is `textureNode.uvNode ||
/// uv()`. `webgpu_backdrop_area` hands it `viewportSharedTexture()`, whose tap
/// is a `textureLoad` at the screen uv rather than a plain `textureSample`.
pub fn hash_blur_with(
    tap: impl Fn(NodeRef) -> NodeRef,
    target_uv: NodeRef,
    blur_amount: NodeRef,
    options: HashBlurOptions,
) -> NodeRef {
    let HashBlurOptions {
        repeats,
        premultiplied_alpha,
    } = options;
    let tap = |uv: NodeRef| {
        let sample = tap(uv);
        if premultiplied_alpha {
            premultiply_alpha(sample)
        } else {
            sample
        }
    };

    let blurred_image = to_var(None, vec4(0.0, 0.0, 0.0, 0.0));

    let body = |i: &NodeRef| {
        let angle = to_const(
            None,
            i.div(repeats.clone()).mul(float(std::f64::consts::TAU)),
        );
        let q = vec2_join(vec![angle.cos(), angle.sin()]).mul(
            rand(vec2_join(vec![i.clone(), target_uv.x().add(target_uv.y())]))
                .add(blur_amount.clone()),
        );
        let uv2 = target_uv.clone().add(q.mul(blur_amount.clone()));
        vec![blurred_image.add_assign(tap(uv2))]
    };

    let statements = vec![
        blurred_image.clone(),
        loop_options("i", Type::F32, float(0.0), repeats.clone(), "<", body),
        blurred_image.div_assign(repeats),
    ];

    let result = if premultiplied_alpha {
        unpremultiply_alpha(blurred_image)
    } else {
        blurred_image
    };
    block(statements, result)
}
