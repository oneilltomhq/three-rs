//! Port of `three.js/src/nodes/lighting/PointShadowNode.js` (the WebGPU branch)
//! plus the `PostProcessingUtils` filter it calls, and of
//! `three.js/src/lights/PointLightShadow.js`' defaults.

use crate::math::Vector3;
use crate::nodes::node::NodeRef;
use crate::nodes::tsl::{
    abs, cube_depth_texture_compare, cross, float, frag_coord, if_node, interleaved_gradient_noise,
    light_shadow_matrix, max, mix, normal_world, position_world, shadow_bias, shadow_camera_far,
    shadow_camera_near, shadow_intensity, shadow_map_size, shadow_normal_bias,
    shadow_position_world, shadow_radius, to_var, vec3, vec4_join, vogel_disk_sample,
};
use crate::textures::CubeDepthTexture;

/// `PointLightShadow` — `LightShadow`'s defaults with the point light's
/// `PerspectiveCamera( 90, 1, 0.5, 500 )`.
#[derive(Clone, Debug)]
pub struct PointLightShadow {
    pub map_size: u32,
    pub bias: f64,
    pub normal_bias: f64,
    pub radius: f64,
    pub intensity: f64,
    pub camera_near: f64,
    pub camera_far: f64,
}

impl Default for PointLightShadow {
    fn default() -> Self {
        Self {
            map_size: 512,
            bias: 0.0,
            normal_bias: 0.0,
            radius: 1.0,
            intensity: 1.0,
            camera_near: 0.5,
            camera_far: 500.0,
        }
    }
}

/// The six cube faces as `renderShadow()` walks them, in the WebGPU
/// coordinate system (the ±Y directions are swapped to match the sampling
/// convention). Face order: +X, −X, +Y, −Y, +Z, −Z.
pub const CUBE_DIRECTIONS: [Vector3; 6] = [
    Vector3 { x: 1.0, y: 0.0, z: 0.0 },
    Vector3 { x: -1.0, y: 0.0, z: 0.0 },
    Vector3 { x: 0.0, y: -1.0, z: 0.0 },
    Vector3 { x: 0.0, y: 1.0, z: 0.0 },
    Vector3 { x: 0.0, y: 0.0, z: 1.0 },
    Vector3 { x: 0.0, y: 0.0, z: -1.0 },
];

pub const CUBE_UPS: [Vector3; 6] = [
    Vector3 { x: 0.0, y: -1.0, z: 0.0 },
    Vector3 { x: 0.0, y: -1.0, z: 0.0 },
    Vector3 { x: 0.0, y: 0.0, z: -1.0 },
    Vector3 { x: 0.0, y: 0.0, z: 1.0 },
    Vector3 { x: 0.0, y: -1.0, z: 0.0 },
    Vector3 { x: 0.0, y: -1.0, z: 0.0 },
];

/// `PointShadowFilter` — percentage-closer filtering with five Vogel-disk taps
/// in the tangent frame of `bd3D`, rotated per pixel by interleaved gradient
/// noise.
fn point_shadow_filter(
    index: usize,
    depth_texture: &CubeDepthTexture,
    bd3d: NodeRef,
    dp: NodeRef,
) -> NodeRef {
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

    let phi = interleaved_gradient_noise(frag_coord().xy()).mul(6.283_185_307_18);

    let mut sum: Option<NodeRef> = None;
    for i in 0..5 {
        let s = vogel_disk_sample(i, 5, phi.clone());
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
    sum.unwrap().mul(1.0 / 5.0)
}

/// `pointShadowFilter` — the whole `PointShadowNode` shader side: the shadow
/// position is the vector from the light to the fragment, so the cube face is
/// picked by its largest component and the depth compared against the
/// perspective depth of that distance.
pub fn point_shadow(index: usize, depth_texture: &CubeDepthTexture) -> NodeRef {
    // `ShadowNode.setupShadowPosition()` then `setupShadow()`:
    // `shadowMatrix * ( shadowPositionWorld + normalWorld * normalBias )`,
    // and `PointShadowNode.setupShadowCoord()` leaves it unprojected.
    let assign_position = shadow_position_world().assign(position_world());
    // `shadowCoord.xyz.toConst()`: the port has no `toConst`, so the same
    // single-evaluation guarantee comes from a var. It is needed rather than
    // optional — `Node::Swizzle` is not one of the kinds the builder promotes
    // on usage count, so the matrix multiply would otherwise be emitted twice.
    let shadow_position = to_var(
        None,
        light_shadow_matrix(index)
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
    let filtered = point_shadow_filter(index, depth_texture, bd3d, dp.clone());

    let node = if_node(
        vec![assign_position, shadow_position_abs],
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
