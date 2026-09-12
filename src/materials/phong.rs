//! Port of `three.js/src/nodes/functions/BSDF/BRDF_Lambert.js`,
//! `BRDF_BlinnPhong.js`, `F_Schlick.js`, `D_BlinnPhong.js` and
//! `PhongLightingModel.js`, plus the `LightsNode` per-light loop body for a
//! `PointLight`.
//!
//! Every statement here is read off the dump of `webgpu_lights_phong`'s three
//! Phong programs (`docs/rung5-progress.md` §4 quotes the WGSL); the node graph
//! below is arranged to emit exactly that.

#![allow(dead_code)] // wired into `setup()` by the next step of rung 5

use crate::nodes::tsl::*;
use crate::nodes::NodeRef;

/// `1 / π` — three.js' `RECIPROCAL_PI`, which prints as
/// `0.3183098861837907` in the dumps.
pub const RECIPROCAL_PI: f64 = std::f64::consts::FRAC_1_PI;

/// The uniforms one `PointLight` contributes, in the order `LightsNode` lays
/// them into the **render** group: colour × intensity, cutoff distance, decay,
/// then (appended after every light's triple) the view-space position.
pub struct PointLightUniforms {
    /// `light.color * light.intensity`, linear.
    pub color: NodeRef,
    /// The light's world position through the camera view matrix.
    pub view_position: NodeRef,
    /// `light.distance`.
    pub cutoff_distance: NodeRef,
    /// `light.decay`.
    pub decay: NodeRef,
}

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
    let dot_vh = position_view_direction()
        .dot(half_dir)
        .clamp(0.0, 1.0);

    let f = f_schlick(specular_color(), float(1.0), dot_vh);
    // `G_BlinnPhong_Implicit()` is the constant 0.25.
    f.mul(0.25).mul(d_blinn_phong(shininess(), dot_nh))
}

/// One `PointLight`'s contribution: `LightNode.setup()`'s `lightDirection` /
/// `lightColor` pair fed through `PhongLightingModel.direct()`. The statements
/// are pushed in the order the dump prints them.
pub fn direct_point_light(light: &PointLightUniforms, out: &mut Vec<NodeRef>) {
    let l_vector = light.view_position.clone().sub(position_view());
    let light_direction = l_vector.clone().normalize();
    let attenuation = distance_attenuation(
        length(l_vector),
        light.cutoff_distance.clone(),
        light.decay.clone(),
    );
    let light_color = light.color.clone().mul(attenuation);

    // `irradiance = dotNL * lightColor`, clamped — `getLightingIrradiance`.
    let dot_nl = normal_view().dot(light_direction.clone()).clamp(0.0, 1.0);
    let irr = dot_nl.mul(light_color);

    // `PhongLightingModel.direct()`.
    out.push(direct_diffuse().assign(
        direct_diffuse().add(irr.clone().mul(brdf_lambert(diffuse_color().xyz()))),
    ));
    out.push(
        direct_specular().assign(
            direct_specular().add(irr.mul(brdf_blinn_phong(light_direction)).mul(1.0)),
        ),
    );
}
