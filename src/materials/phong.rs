//! Port of `three.js/src/nodes/functions/BSDF/BRDF_Lambert.js`,
//! `BRDF_BlinnPhong.js`, `F_Schlick.js`, `D_BlinnPhong.js` and
//! `PhongLightingModel.js`, plus the `LightsNode` per-light loop body for a
//! `PointLight`.
//!
//! Every statement here is read off the dump of `webgpu_lights_phong`'s three
//! Phong programs (`docs/rung5-progress.md` §4 quotes the WGSL); the node graph
//! below is arranged to emit exactly that.

#![allow(dead_code)] // wired into `setup()` by the next step of rung 5

use crate::lights::{point_shadow, LightKind};
use crate::nodes::tsl::*;
use crate::nodes::NodeRef;
use crate::textures::{CubeDepthTexture, DepthTexture};

/// `1 / π` — three.js' `RECIPROCAL_PI`, which prints as
/// `0.3183098861837907` in the dumps.
pub const RECIPROCAL_PI: f64 = std::f64::consts::FRAC_1_PI;

/// `getDistanceAttenuation( lightDistance, cutoffDistance, decayExponent )` —
/// the whole thing, including the `if ( cutoffDistance > 0.0 ) { … } else { … }`
/// over a shared temp that the dump shows. `cutoffDistance` is a uniform, so the
/// branch stays in the shader; `Node::Select` already lowers to exactly that
/// shape (`builder.rs`' `Node::Select` arm), so no new node variant is needed.
pub fn distance_attenuation(
    light_distance: NodeRef,
    cutoff_distance: NodeRef,
    decay: NodeRef,
) -> NodeRef {
    cutoff_distance.greater_than(0.0).select(
        distance_attenuation_with_cutoff(
            light_distance.clone(),
            cutoff_distance.clone(),
            decay.clone(),
        ),
        distance_attenuation_no_cutoff(light_distance, decay),
    )
}

/// The `cutoffDistance > 0` branch: inverse-square falloff times the smooth
/// window `clamp( 1 - ( d / cutoff )^4, 0, 1 )^2`.
pub fn distance_attenuation_with_cutoff(
    light_distance: NodeRef,
    cutoff_distance: NodeRef,
    decay: NodeRef,
) -> NodeRef {
    let t = light_distance.clone().div(cutoff_distance);
    let f = float(1.0)
        .sub(t.clone().mul(t.clone()).mul(t.clone()).mul(t))
        .clamp(0.0, 1.0);
    float(1.0)
        .div(max(light_distance.pow(decay), float(0.01)))
        .mul(f.clone().mul(f))
}

/// `getDistanceAttenuation` with `cutoffDistance == 0`.
pub fn distance_attenuation_no_cutoff(light_distance: NodeRef, decay: NodeRef) -> NodeRef {
    float(1.0).div(max(light_distance.pow(decay), float(0.01)))
}

/// `BRDF_Lambert( { diffuseColor } )` — `RECIPROCAL_PI * diffuseColor`.
pub fn brdf_lambert(diffuse: NodeRef) -> NodeRef {
    diffuse.mul(RECIPROCAL_PI)
}

/// `F_Schlick( { f0, f90, dotVH } )` with three.js' `exp2` approximation of
/// `pow( 1 - dotVH, 5 )`.
pub fn f_schlick(f0: NodeRef, f90: NodeRef, dot_vh: NodeRef) -> NodeRef {
    let fresnel = exp2(dot_vh.clone().mul(-5.55473).sub(6.98316).mul(dot_vh));
    f0.mul(float(1.0).sub(fresnel.clone()))
        .add(f90.mul(fresnel))
}

/// `D_BlinnPhong( { shininess, dotNH } )` —
/// `RECIPROCAL_PI * ( shininess * 0.5 + 1.0 ) * pow( dotNH, shininess )`.
pub fn d_blinn_phong(shininess_value: NodeRef, dot_nh: NodeRef) -> NodeRef {
    shininess_value
        .clone()
        .mul(0.5)
        .add(1.0)
        .mul(RECIPROCAL_PI)
        .mul(dot_nh.pow(shininess_value))
}

/// `BRDF_BlinnPhong( { lightDirection, specularColor, shininess } )`.
pub fn brdf_blinn_phong(light_direction: NodeRef) -> NodeRef {
    let half_dir = light_direction.add(position_view_direction()).normalize();
    let dot_nh = normal_view().dot(half_dir.clone()).clamp(0.0, 1.0);
    let dot_vh = position_view_direction().dot(half_dir).clamp(0.0, 1.0);

    let f = f_schlick(specular_color(), float(1.0), dot_vh);
    // `G_BlinnPhong_Implicit()` is the constant 0.25.
    f.mul(0.25).mul(d_blinn_phong(shininess(), dot_nh))
}

/// One light's rendered shadow map: `ShadowNode`'s 2-D depth texture for a
/// spot or directional light, `PointShadowNode`'s cube for a point light.
#[derive(Clone, Debug)]
pub enum ShadowMap {
    Planar(DepthTexture),
    Cube(CubeDepthTexture),
}

/// By identity, as a texture contributes its `uuid` to `Node.getCacheKey()`:
/// the map's kind and which map it is are what the program depends on.
impl std::hash::Hash for ShadowMap {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        std::mem::discriminant(self).hash(state);
        match self {
            ShadowMap::Planar(texture) => texture.id().hash(state),
            ShadowMap::Cube(texture) => texture.id().hash(state),
        }
    }
}

/// One entry of `LightsNode`'s light list, as the material setup sees it.
///
/// `Hash` is the light's share of the render object's dynamic cache key
/// (`RenderObject.getDynamicCacheKey()` → `lightsNode.getCacheKey()`).
#[derive(Clone, Debug, Hash)]
pub struct LightDesc {
    /// The light's index in the renderer's light list, which is what every
    /// `UniformSource::Light*` / `Shadow*` variant keys on.
    pub index: usize,
    pub kind: LightKind,
    /// `light.castShadow && object.receiveShadow && renderer.shadowMap.enabled`
    /// — the shadow map, read with `textureSampleCompare`.
    pub shadow_map: Option<ShadowMap>,
}

/// `ShadowBaseNode.setupShadowPosition()` (a statement, pushed here) followed
/// by the shadow node itself — `ShadowNode` or `PointShadowNode` by map.
pub fn shadow_node(
    index: usize,
    map: &ShadowMap,
    received_shadow_position: Option<&NodeRef>,
    out: &mut Vec<NodeRef>,
) -> NodeRef {
    let position = match received_shadow_position {
        Some(node) => node.clone(),
        None => position_world(),
    };
    out.push(shadow_position_world().assign(position));
    match map {
        ShadowMap::Planar(map) => shadow_factor(index, map),
        ShadowMap::Cube(map) => point_shadow(index, map),
    }
}

/// `LightsNode.setupLightsNode()`'s per-light half that every lighting model
/// shares: an ambient or hemisphere light adds straight to `irradiance` and
/// yields nothing; an analytic light yields `LightNode.setup()`'s
/// `( lightDirection, lightColor )` pair — shadow factor included — for the
/// model's `direct()`.
pub fn setup_light(
    light: &LightDesc,
    received_shadow_position: Option<&NodeRef>,
    out: &mut Vec<NodeRef>,
) -> Option<(NodeRef, NodeRef)> {
    let index = light.index;

    match light.kind {
        // `AmbientLightNode.setup()` — `irradiance += lightColor`.
        LightKind::Ambient => {
            out.push(irradiance().assign(irradiance().add(light_color_intensity(index))));
            return None;
        }
        // `HemisphereLightNode.setup()`: a sky/ground mix by the world
        // normal's hemisphere weight.
        LightKind::Hemisphere => {
            let light_direction = light_world_position(index).normalize();
            let dot_nl = normal_world().dot(light_direction);
            let hemi_diffuse_weight = dot_nl.mul(0.5).add(0.5);
            let value = mix(
                light_ground_color(index),
                light_color_intensity(index),
                hemi_diffuse_weight,
            );
            out.push(irradiance().assign(irradiance().add(value)));
            return None;
        }
        _ => {}
    }

    // `AnalyticLightNode.setupShadow()`'s `colorNode = colorNode.mul( shadow )`
    // — before the light's own attenuation.
    let mut color = light_color_intensity(index);
    if let Some(map) = &light.shadow_map {
        color = color.mul(shadow_node(index, map, received_shadow_position, out));
    }

    Some(match light.kind {
        LightKind::Ambient | LightKind::Hemisphere => {
            unreachable!("three-rs: the ambient and hemisphere lights are summed elsewhere")
        }
        LightKind::Point => {
            let l_vector = light_view_position(index).sub(position_view());
            let attenuation = distance_attenuation(
                length(l_vector.clone()),
                light_cutoff_distance(index),
                light_decay(index),
            );
            (l_vector.normalize(), color.mul(attenuation))
        }
        LightKind::Spot => {
            let l_vector = light_view_position(index).sub(position_view());
            let light_direction = l_vector.clone().normalize();
            let angle_cos = light_direction.clone().dot(light_target_direction(index));
            let spot_attenuation =
                smoothstep(light_cone_cos(index), light_penumbra_cos(index), angle_cos);
            let attenuation = distance_attenuation(
                length(l_vector),
                light_cutoff_distance(index),
                light_decay(index),
            );
            (
                light_direction,
                color.mul(spot_attenuation).mul(attenuation),
            )
        }
        LightKind::Directional => (light_target_direction(index), color),
    })
}

/// `ShadowNode.setupShadowCoord()` + `setupShadowFilter()` + the
/// `mix( 1, shadow, intensity )` of `setupShadow()`, for the light at `index`.
///
/// `shadowPositionWorld` must already have been assigned — that is
/// `ShadowBaseNode.setupShadowPosition()`, which the caller pushes because it is
/// a statement rather than an expression.
pub fn shadow_factor(index: usize, map: &DepthTexture) -> NodeRef {
    // `shadowPosition = shadowMatrix * vec4( shadowPositionWorld +
    // normalWorld * normalBias, 1 )`.
    let position = shadow_matrix(index).mul(vec4_join(vec![
        shadow_position_world().add(normal_world().mul(shadow_normal_bias(index))),
        float(1.0),
    ]));

    // `setupShadowCoord`: the perspective divide, then the Y flip WebGPU needs
    // and `bias` added to the depth. Both cameras take this branch —
    // `logarithmicDepthBuffer` is off.
    let divided = position.clone().xyz().div(position.w());
    let coord = vec3_join(vec![
        divided.clone().x(),
        float(1.0).sub(divided.clone().y()),
        divided.z().add(shadow_bias(index)),
    ]);

    // `setupShadowFilter`'s frustum test. Note there is no `z >= 0` term.
    let frustum_test = coord
        .clone()
        .x()
        .greater_than_equal(float(0.0))
        .and(coord.clone().x().less_than_equal(float(1.0)))
        .and(coord.clone().y().greater_than_equal(float(0.0)))
        .and(coord.clone().y().less_than_equal(float(1.0)))
        .and(coord.clone().z().less_than_equal(float(1.0)));

    let shadow = frustum_test.select(pcf_shadow(index, map, coord), float(1.0));

    mix(float(1.0), shadow, shadow_intensity(index))
}

/// `PCFShadowFilter` — five Vogel-disk taps rotated by interleaved gradient
/// noise, each one a hardware comparison sample (so 20 effective taps).
fn pcf_shadow(index: usize, map: &DepthTexture, coord: NodeRef) -> NodeRef {
    let texel_size = vec2(1.0, 1.0).div(shadow_map_size(index));
    let radius_scaled = shadow_radius(index).mul(texel_size.x());
    // `6.28318530718` mirrors three.js's `PCFShadowFilter` literal (an approximation
    // of `TAU`, not the exact constant); keeping the same literal keeps this
    // pixel-identical to three.js's output.
    #[allow(clippy::approx_constant)]
    let phi = interleaved_gradient_noise(frag_coord().xy()).mul(float(6.28318530718));

    let mut sum: Option<NodeRef> = None;
    for i in 0..5 {
        let offset = vogel_disk_sample(int(i), int(5), phi.clone()).mul(radius_scaled.clone());
        let tap = shadow_map_compare(map, coord.clone().xy().add(offset), coord.clone().z());
        sum = Some(match sum {
            Some(acc) => acc.add(tap),
            None => tap,
        });
    }
    sum.expect("three-rs: the five-tap loop always sets sum")
        .mul(float(1.0 / 5.0))
}

/// `AmbientLightNode.setup()` — `irradiance += lightColor`, no attenuation.
pub fn ambient_lights(indices: &[usize], out: &mut Vec<NodeRef>) {
    out.push(irradiance().assign(vec3(0.0, 0.0, 0.0)));
    for index in indices {
        out.push(irradiance().assign(irradiance().add(light_color_intensity(*index))));
    }
}

/// One direct light's contribution: its `lightDirection` / `lightColor` pair
/// (including the shadow factor when it casts one) fed through
/// `PhongLightingModel.direct()`.
pub fn direct_light(
    light: &LightDesc,
    received_shadow_position: Option<&NodeRef>,
    out: &mut Vec<NodeRef>,
) {
    let Some((light_direction, light_color)) = setup_light(light, received_shadow_position, out)
    else {
        return;
    };

    // `irradiance = dotNL * lightColor`, clamped — `getLightingIrradiance`.
    let dot_nl = normal_view().dot(light_direction.clone()).clamp(0.0, 1.0);
    let irr = dot_nl.mul(light_color);

    // `PhongLightingModel.direct()`.
    out.push(
        direct_diffuse()
            .assign(direct_diffuse().add(irr.clone().mul(brdf_lambert(diffuse_color().xyz())))),
    );
    out.push(
        direct_specular()
            .assign(direct_specular().add(irr.mul(brdf_blinn_phong(light_direction)).mul(1.0))),
    );
}
