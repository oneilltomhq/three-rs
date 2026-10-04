//! Port of `three.js/examples/jsm/tsl/utils/TAAUtils.js` — the helpers
//! `TRAANode` and `TAAUNode` share: the Halton jitter sequence, the 3×3
//! closest / farthest depth search, the reprojected history depth, AABB
//! clipping and flicker reduction.
//!
//! Three exports them from one module and both nodes import the same
//! functions, so the port keeps one copy too. `clipAABB` and
//! `flickerReduction` carry a `setLayout()` and become WGSL functions of
//! those names; the other two are inline `Fn()`s, expanded in place.

use std::rc::Rc;

use crate::math::Matrix4;
use crate::nodes::node::{FnDef, StructLayout, StructMember, Type};
use crate::nodes::tsl::{
    block, depth_texture_load, depth_texture_sample, float, get_view_position, if_then, luminance,
    max, shader_fn, struct_new, struct_type, to_const, to_var, vec2, vec4_join,
    view_z_to_perspective_depth,
};
use crate::nodes::NodeRef;
use crate::textures::DepthTexture;

/// `_haltonOffsets.length` — the jitter sequence both nodes cycle through.
pub(crate) const JITTER_COUNT: usize = 32;

/// `computeHaltonOffsets( 32 )` — `[ halton( i + 1, 2 ), halton( i + 1, 3 ) ]`.
pub(crate) fn halton_offsets() -> [(f64, f64); JITTER_COUNT] {
    std::array::from_fn(|i| (halton(i as u32 + 1, 2), halton(i as u32 + 1, 3)))
}

/// `TAAUtils.js`' `halton( index, base )`, in the same operation order so the
/// doubles are the same bits.
fn halton(mut index: u32, base: u32) -> f64 {
    let mut fraction = 1.0;
    let mut result = 0.0;
    while index > 0 {
        fraction /= base as f64;
        result += fraction * (index % base) as f64;
        index /= base;
    }
    result
}

/// A matrix's column-major elements, for a `mat4` uniform.
pub(crate) fn mat4_values(m: &Matrix4) -> Vec<f64> {
    m.elements.to_vec()
}

/// `struct( { closestDepth: 'float', closestPositionTexel: 'vec2',
/// farthestDepth: 'float' } )` — `currentDepthStruct`. Three names an
/// anonymous struct after its order of creation, and on both the TRAA and the
/// TAAU page it is the first: `StructType0`.
pub(crate) fn current_depth_struct() -> Rc<StructLayout> {
    struct_type(
        "StructType0",
        vec![
            StructMember::new("closestDepth", Type::F32),
            StructMember::new("closestPositionTexel", Type::Vec2),
            StructMember::new("farthestDepth", Type::F32),
        ],
    )
}

/// `TAAUtils.sampleCurrentDepth( depthNode, positionTexel, cameraNearFar )`:
/// the closest and farthest depth of the 3×3 neighbourhood, and where the
/// closest one is. Three unrolls the two JS loops, x outer.
pub(crate) fn sample_current_depth(
    depth: &DepthTexture,
    position_texel: &NodeRef,
    layout: &Rc<StructLayout>,
) -> NodeRef {
    let closest_depth = to_var(None, float(2.0));
    let closest_position_texel = to_var(None, vec2(0.0, 0.0));
    let farthest_depth = to_var(None, float(-1.0));
    let mut statements = vec![
        closest_depth.clone(),
        closest_position_texel.clone(),
        farthest_depth.clone(),
    ];
    for x in -1..=1 {
        for y in -1..=1 {
            let neighbor = to_var(None, position_texel.add(vec2(x as f64, y as f64)));
            let depth = to_var(None, depth_texture_load(depth, neighbor.clone()));
            statements.push(neighbor.clone());
            statements.push(depth.clone());
            statements.push(if_then(
                depth.less_than(closest_depth.clone()),
                vec![
                    closest_depth.assign(depth.clone()),
                    closest_position_texel.assign(neighbor),
                ],
            ));
            statements.push(if_then(
                depth.greater_than(farthest_depth.clone()),
                vec![farthest_depth.assign(depth)],
            ));
        }
    }
    block(
        statements,
        struct_new(
            layout,
            vec![closest_depth, closest_position_texel, farthest_depth],
        ),
    )
}

/// `TAAUtils.samplePreviousDepth()` for a perspective camera: the history
/// depth at `uv`, reconstructed to a world position with the previous
/// camera, and projected back to a perspective depth with the current one.
pub(crate) fn sample_previous_depth(
    previous_depth: &DepthTexture,
    uv: &NodeRef,
    previous_projection_inverse: &NodeRef,
    previous_world: &NodeRef,
    world_inverse: &NodeRef,
    near_far: &NodeRef,
) -> NodeRef {
    let depth = depth_texture_sample(previous_depth, uv.clone());
    let position_view = get_view_position(uv.clone(), depth, previous_projection_inverse.clone());
    let position_world = previous_world
        .mul(vec4_join(vec![position_view, float(1.0)]))
        .xyz();
    let view_z = world_inverse
        .mul(vec4_join(vec![position_world, float(1.0)]))
        .z();
    view_z_to_perspective_depth(view_z, near_far.x(), near_far.y())
}

/// `TAAUtils.clipAABB( currentColor, historyColor, minColor, maxColor )`.
pub(crate) fn clip_aabb() -> Rc<FnDef> {
    shader_fn(
        Some("clipAABB"),
        vec![
            ("currentColor", Type::Vec4),
            ("historyColor", Type::Vec4),
            ("minColor", Type::Vec4),
            ("maxColor", Type::Vec4),
        ],
        Type::Vec4,
        |args| {
            let (current_color, history_color, min_color, max_color) =
                (&args[0], &args[1], &args[2], &args[3]);
            let p_clip = to_const(None, max_color.rgb().add(min_color.rgb()).mul(0.5));
            let e_clip = to_const(
                None,
                max_color.rgb().sub(min_color.rgb()).mul(0.5).add(1e-7),
            );
            let v_clip = to_const(
                None,
                history_color.sub(vec4_join(vec![p_clip.clone(), current_color.w()])),
            );
            let v_unit = to_const(None, v_clip.xyz().div(e_clip));
            let abs_unit = to_const(None, v_unit.abs());
            let max_unit = to_const(None, max(max(abs_unit.x(), abs_unit.y()), abs_unit.z()));
            max_unit.greater_than(1.0).select(
                vec4_join(vec![p_clip, current_color.w()]).add(v_clip.div(max_unit.clone())),
                history_color.clone(),
            )
        },
    )
}

/// `TAAUtils.flickerReduction( currentColor, historyColor, currentWeight )`:
/// blend in a tone-compressed space, each side weighted down by its
/// luminance.
pub(crate) fn flicker_reduction() -> Rc<FnDef> {
    shader_fn(
        Some("flickerReduction"),
        vec![
            ("currentColor", Type::Vec4),
            ("historyColor", Type::Vec4),
            ("currentWeight", Type::F32),
        ],
        Type::Vec4,
        |args| {
            let (current_color, history_color, current_weight) = (&args[0], &args[1], &args[2]);
            let compress = |color: &NodeRef| {
                to_const(
                    None,
                    color.mul(float(1.0).div(max(max(color.x(), color.y()), color.z()).add(1.0))),
                )
            };
            let compressed_current = compress(current_color);
            let compressed_history = compress(history_color);

            let luminance_current = to_const(None, luminance(compressed_current.rgb()));
            let luminance_history = to_const(None, luminance(compressed_history.rgb()));

            let weight_current = to_const(None, current_weight.div(luminance_current.add(1.0)));
            let weight_history = to_const(
                None,
                current_weight.one_minus().div(luminance_history.add(1.0)),
            );

            current_color
                .mul(weight_current.clone())
                .add(history_color.mul(weight_history.clone()))
                .div(max(weight_current.add(weight_history), 0.00001))
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `computeHaltonOffsets( 32 )`'s first entries, as JS computes them.
    #[test]
    fn halton_offsets_match_three() {
        let offsets = halton_offsets();
        assert_eq!(offsets[0], (0.5, 1.0 / 3.0));
        assert_eq!(offsets[1], (0.25, 2.0 / 3.0));
        assert_eq!(offsets[2], (0.75, 1.0 / 9.0));
        assert_eq!(offsets[31].0, 1.0 / 64.0);
    }
}
