//! Port of `three.js/examples/jsm/tsl/display/radialBlur.js`.

use crate::nodes::node::Type;
use crate::nodes::tsl::{
    block, float, frag_coord, interleaved_gradient_noise, loop_n, mix, texture_uv, to_const,
    to_var, uv, vec2, vec4,
};
use crate::nodes::NodeRef;
use crate::textures::Texture;

/// `radialBlur( textureNode, options )`' options object. Every field is a node
/// so the example can hand it a `uniform()`; three.js' defaults are in
/// [`RadialBlurOptions::default`].
///
/// `count` and `exposure` are written `int( … )` in the page but reach the
/// shader as `f32` — `uniform( int( 32 ) )` keeps the value, not the type, and
/// the loop bound is `i32( object.nodeUniformN )`. So they are floats here too.
#[derive(Clone)]
pub struct RadialBlurOptions {
    /// `options.center` — the centre of the streaks, in screen uv.
    pub center: NodeRef,
    /// `options.weight` — the base weight of each sample.
    pub weight: NodeRef,
    /// `options.decay` — the factor the weight is multiplied by per step.
    pub decay: NodeRef,
    /// `options.count` — the number of samples.
    pub count: NodeRef,
    /// `options.exposure` — the blur's exposure control.
    pub exposure: NodeRef,
}

impl Default for RadialBlurOptions {
    fn default() -> Self {
        Self {
            center: vec2(0.5, 0.5),
            weight: float(0.9),
            decay: float(0.95),
            count: float(32.0),
            exposure: float(5.0),
        }
    }
}

/// `radialBlur( textureNode, options )`.
///
/// The addon takes a texture *node*; this port takes the texture itself,
/// because the only thing it does with the node is `textureNode.sample( uv )`
/// and `textureNode.uvNode || uv()` — and a `PassTextureNode`'s `uvNode` is
/// null, so the uv is always the raw `uv()` varying
/// (`docs/postprocessing.md`). `options.premultipliedAlpha` is not ported:
/// both of its branches are dead for every call site in three.js' own
/// examples.
pub fn radial_blur(map: &Texture, options: &RadialBlurOptions) -> NodeRef {
    let RadialBlurOptions {
        center,
        weight,
        decay,
        count,
        exposure,
    } = options.clone();

    // `const sampleUv = vec2( textureNode.uvNode || uv() )` — a `toVar`,
    // because the loop walks it outwards one `offset` at a time.
    let sample_uv = to_var(None, uv());
    // `const base = tap( sampleUv ).toConst()`.
    let base = to_const(None, texture_uv(map, sample_uv.clone()));
    // `const blur = vec4().toVar()` — TSL's no-argument `vec4()` is
    // `( 0, 0, 0, 1 )`, so the accumulator starts with an alpha of one.
    let blur = to_var(None, vec4(0.0, 0.0, 0.0, 1.0));
    // `const offset = center.sub( sampleUv ).div( count ).toConst()`.
    let offset = to_const(None, center.sub(sample_uv.clone()).div(count.clone()));
    // `const w = float( weight ).toVar()`.
    let w = to_var(None, weight);

    // `sampleUv.addAssign( offset.mul( noise ) )` — mitigate banding.
    let noise = interleaved_gradient_noise(frag_coord().xy());

    let loop_body = {
        let (sample_uv, blur, w, offset, decay) = (
            sample_uv.clone(),
            blur.clone(),
            w.clone(),
            offset.clone(),
            decay,
        );
        let map = map.clone();
        move |_i: &NodeRef| {
            vec![
                sample_uv.add_assign(offset),
                blur.add_assign(texture_uv(&map, sample_uv.clone()).mul(w.clone())),
                w.mul_assign(decay),
            ]
        }
    };

    // Three's `Fn()` body pushes each `toVar()` / `toConst()` onto the
    // builder's stack as it is created, so the five declarations are
    // statements in their own right and are emitted in creation order, ahead
    // of the loop that reads them.
    //
    // The result is wrapped in a var because three.js' usage count promotes it
    // to one: `renderOutput()` reads the effect's colour twice (`.rgb` and
    // `.a`), and the dump's `nodeVar5 = mix( … )` is that promotion. The port
    // asks for it, because `Node::Block` is not a kind the builder promotes.
    to_var(
        None,
        block(
            vec![
                sample_uv.clone(),
                base.clone(),
                blur.clone(),
                offset.clone(),
                w.clone(),
                sample_uv.add_assign(offset.mul(noise)),
                // `Loop( { start: int( 0 ), end: int( count ), type: 'int',
                // condition: '<' }, … )` — a node-bounded loop, so the bound is
                // the uniform cast to `i32`, not a literal.
                loop_n("i", count.to(Type::I32), loop_body),
                blur.assign(blur.div(count.clone())),
                blur.assign(blur.mul(exposure)),
            ],
            mix(blur, base.mul(float(2.0)), float(0.5)),
        ),
    )
}
