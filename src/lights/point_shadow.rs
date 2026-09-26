//! Port of `three.js/src/nodes/lighting/PointShadowNode.js` (the WebGPU branch)
//! plus the `PostProcessingUtils` filter it calls. `PointLightShadow`'s defaults
//! are `LightShadow::point()`.

use crate::math::Vector3;
use crate::nodes::node::NodeRef;
use crate::nodes::tsl::{
    abs, cross, cube_depth_texture_compare, float, frag_coord, if_node, int,
    interleaved_gradient_noise, max, mix, normal_world, shadow_bias, shadow_camera_far,
    shadow_camera_near, shadow_intensity, shadow_map_size, shadow_matrix, shadow_normal_bias,
    shadow_position_world, shadow_radius, to_var, vec3, vec4_join, vogel_disk_sample,
};
use crate::textures::CubeDepthTexture;

use super::shadow_filter::{ShadowFilter, ShadowFilterInputs, ShadowFilterMap};

/// The six cube faces as `renderShadow()` walks them, in the WebGPU
/// coordinate system (the ±Y directions are swapped to match the sampling
/// convention). Face order: +X, −X, +Y, −Y, +Z, −Z.
pub const CUBE_DIRECTIONS: [Vector3; 6] = [
    Vector3 {
        x: 1.0,
        y: 0.0,
        z: 0.0,
    },
    Vector3 {
        x: -1.0,
        y: 0.0,
        z: 0.0,
    },
    Vector3 {
        x: 0.0,
        y: -1.0,
        z: 0.0,
    },
    Vector3 {
        x: 0.0,
        y: 1.0,
        z: 0.0,
    },
    Vector3 {
        x: 0.0,
        y: 0.0,
        z: 1.0,
    },
    Vector3 {
        x: 0.0,
        y: 0.0,
        z: -1.0,
    },
];

pub const CUBE_UPS: [Vector3; 6] = [
    Vector3 {
        x: 0.0,
        y: -1.0,
        z: 0.0,
    },
    Vector3 {
        x: 0.0,
        y: -1.0,
        z: 0.0,
    },
    Vector3 {
        x: 0.0,
        y: 0.0,
        z: -1.0,
    },
    Vector3 {
        x: 0.0,
        y: 0.0,
        z: 1.0,
    },
    Vector3 {
        x: 0.0,
        y: -1.0,
        z: 0.0,
    },
    Vector3 {
        x: 0.0,
        y: -1.0,
        z: 0.0,
    },
];

/// `BasicPointShadowFilter` — one hardware comparison along `bd3D`:
/// `cubeTexture( depthTexture, bd3D ).compare( dp )`.
pub fn basic_point_shadow_filter(inputs: &ShadowFilterInputs) -> NodeRef {
    let (depth_texture, bd3d, dp) = point_inputs(inputs, "BasicPointShadowFilter");
    cube_depth_texture_compare(&depth_texture, bd3d, dp)
}

fn point_inputs(inputs: &ShadowFilterInputs, filter: &str) -> (CubeDepthTexture, NodeRef, NodeRef) {
    match (&inputs.map, &inputs.dp) {
        (ShadowFilterMap::Cube(map), Some(dp)) => {
            (map.clone(), inputs.shadow_coord.clone(), dp.clone())
        }
        (other, _) => {
            panic!("three-rs: {filter} reads a cube depth texture and a dp, got {other:?}")
        }
    }
}

/// `PointShadowFilter` — percentage-closer filtering with five Vogel-disk taps
/// in the tangent frame of `bd3D`, rotated per pixel by interleaved gradient
/// noise.
pub fn point_shadow_filter(inputs: &ShadowFilterInputs) -> NodeRef {
    let index = inputs.index;
    let (depth_texture, bd3d, dp) = point_inputs(inputs, "PointShadowFilter");
    let depth_texture = &depth_texture;
    let texel_size = shadow_radius(index).div(shadow_map_size(index).x());

    let abs_dir = abs(bd3d.clone());
    let tangent = cross(
        bd3d.clone(),
        abs_dir
            .clone()
            .x()
            .greater_than(abs_dir.z())
            .select(vec3(0.0, 1.0, 0.0), vec3(1.0, 0.0, 0.0)),
    )
    .normalize();
    let bitangent = cross(bd3d.clone(), tangent.clone());

    // `6.283_185_307_18` mirrors three.js's `PCFShadowFilter` literal (an approximation
    // of `TAU`, not the exact constant); keeping the same literal keeps this
    // pixel-identical to three.js's output.
    #[allow(clippy::approx_constant)]
    let phi = interleaved_gradient_noise(frag_coord().xy()).mul(6.283_185_307_18);

    let mut sum: Option<NodeRef> = None;
    for i in 0..5 {
        let s = vogel_disk_sample(int(i), int(5), phi.clone());
        let offset = tangent
            .clone()
            .mul(s.clone().x())
            .add(bitangent.clone().mul(s.y()))
            .mul(texel_size.clone());
        let tap = cube_depth_texture_compare(depth_texture, bd3d.clone().add(offset), dp.clone());
        sum = Some(match sum {
            None => tap,
            Some(acc) => acc.add(tap),
        });
    }
    sum.expect("three-rs: the five-tap loop always sets sum")
        .mul(1.0 / 5.0)
}

/// `pointShadowFilter` — the whole `PointShadowNode` shader side: the shadow
/// position is the vector from the light to the fragment, so the cube face is
/// picked by its largest component and the depth compared against the
/// perspective depth of that distance.
///
/// `shadowPositionWorld` must already have been assigned — that is
/// `ShadowBaseNode.setupShadowPosition()`, which the caller pushes because it
/// is a statement rather than an expression (same seam as `shadow_factor`).
pub fn point_shadow(index: usize, depth_texture: &CubeDepthTexture) -> NodeRef {
    point_shadow_filtered(index, depth_texture, &ShadowFilter::Pcf)
}

/// [`point_shadow`] through a given filter — `PointShadowNode.getShadowFilterFn(
/// type )` (`BasicPointShadowFilter` for `BasicShadowMap`, `PointShadowFilter`
/// otherwise, VSM included) or the light's own `shadow.filterNode`.
pub fn point_shadow_filtered(
    index: usize,
    depth_texture: &CubeDepthTexture,
    filter: &ShadowFilter,
) -> NodeRef {
    // `setupShadow()`: `shadowMatrix * ( shadowPositionWorld + normalWorld *
    // normalBias )`, and `PointShadowNode.setupShadowCoord()` leaves it
    // unprojected.
    // `shadowCoord.xyz.toConst()`: the port has no `toConst`, so the same
    // single-evaluation guarantee comes from a var. It is needed rather than
    // optional — `Node::Swizzle` is not one of the kinds the builder promotes
    // on usage count, so the matrix multiply would otherwise be emitted twice.
    let shadow_position = to_var(
        None,
        shadow_matrix(index)
            .mul(vec4_join(vec![
                shadow_position_world().add(normal_world().mul(shadow_normal_bias(index))),
                float(1.0),
            ]))
            .xyz(),
    );
    let shadow_position_abs = to_var(None, abs(shadow_position.clone()));

    let view_z = max(
        max(
            shadow_position_abs.clone().x(),
            shadow_position_abs.clone().y(),
        ),
        shadow_position_abs.clone().z(),
    );

    let near = shadow_camera_near(index);
    let far = shadow_camera_far(index);

    let result = to_var(None, float(1.0));

    // `viewZToPerspectiveDepth( viewZ.negate(), near, far )`. The negation is
    // read three times, and `Node::Neg` is not one of the kinds the builder
    // promotes on its own, so it is made a var here — which is also the var
    // three.js' `toVar()`-free flow happens to produce.
    let neg_view_z = to_var(None, view_z.clone().negate());
    let dp = to_var(
        None,
        near.clone()
            .add(neg_view_z.clone())
            .mul(far.clone())
            .div(far.clone().sub(near.clone()).mul(neg_view_z)),
    );
    let dp_bias = dp.clone().assign(dp.clone().add(shadow_bias(index)));

    let bd3d = shadow_position.normalize();
    let filtered = filter.apply(&ShadowFilterInputs {
        index,
        map: ShadowFilterMap::Cube(depth_texture.clone()),
        shadow_coord: bd3d,
        dp: Some(dp.clone()),
    });

    let node = if_node(
        vec![shadow_position_abs],
        result.clone(),
        view_z
            .clone()
            .sub(far)
            .less_than_equal(float(0.0))
            .and(view_z.sub(near).greater_than_equal(float(0.0))),
        vec![dp_bias, result.assign(filtered)],
    );

    mix(float(1.0), node, shadow_intensity(index))
}
