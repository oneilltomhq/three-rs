//! Port of `three.js/examples/jsm/tsl/display/boxBlur.js`.

use crate::nodes::node::{TextureSource, Type};
use crate::nodes::tsl::{
    block, int, loop_options, max, premultiply_alpha, texture_size, texture_uv, to_var,
    unpremultiply_alpha, uv, vec2, vec2_join, vec4,
};
use crate::nodes::NodeRef;
use crate::textures::Texture;

/// `boxBlur( textureNode, options )`' options.
#[derive(Clone)]
pub struct BoxBlurOptions {
    /// `options.size` — the kernel's half-width in taps, `int( 1 )` by
    /// default.
    pub size: NodeRef,
    /// `options.separation` — the spacing between taps in texels, `int( 1 )`
    /// by default.
    pub separation: NodeRef,
    /// `options.premultipliedAlpha`.
    pub premultiplied_alpha: bool,
}

impl Default for BoxBlurOptions {
    fn default() -> Self {
        Self {
            size: int(1),
            separation: int(1),
            premultiplied_alpha: false,
        }
    }
}

/// `boxBlur( textureNode, options )` — the average of the `( 2 size + 1 )²`
/// taps round the fragment, `separation` texels apart.
pub fn box_blur(map: &Texture, options: BoxBlurOptions) -> NodeRef {
    let BoxBlurOptions {
        size,
        separation,
        premultiplied_alpha,
    } = options;
    let tap = |uv: NodeRef| {
        let sample = texture_uv(map, uv);
        if premultiplied_alpha {
            premultiply_alpha(sample)
        } else {
            sample
        }
    };

    let target_uv = uv();
    let result = to_var(None, vec4(0.0, 0.0, 0.0, 0.0));
    let sep = max(separation, 1.0);
    let count = to_var(None, int(0));
    let pixel_step = vec2(1.0, 1.0)
        .div(texture_size(TextureSource::Texture2D(map.clone()), int(0)).to(Type::Vec2));

    // `Loop( { start: size.negate(), end: size, name: 'i', condition: '<=' } )`
    // — the bounds are cast to the `int` index, as three's `LoopNode` does.
    let start = size.negate().to(Type::I32);
    let end = size.to(Type::I32);
    let outer = loop_options("i", Type::I32, start.clone(), end.clone(), "<=", |i| {
        vec![loop_options("j", Type::I32, start, end, "<=", |j| {
            let uvs = target_uv.clone().add(
                vec2_join(vec![i.to(Type::F32), j.to(Type::F32)])
                    .mul(pixel_step)
                    .mul(sep),
            );
            vec![result.add_assign(tap(uvs)), count.add_assign(int(1))]
        })]
    });

    let statements = vec![
        result.clone(),
        count.clone(),
        outer,
        result.div_assign(count.to(Type::F32)),
    ];

    let result = if premultiplied_alpha {
        unpremultiply_alpha(result)
    } else {
        result
    };
    block(statements, result)
}
