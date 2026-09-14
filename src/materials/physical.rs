//! Port of `three.js/src/nodes/functions/PhysicalLightingModel.js` and the
//! BSDF functions it reaches: `BRDF_GGX` (`F_Schlick` × `V_GGX_SmithCorrelated`
//! × `D_GGX`), `BRDF_Lambert`, `computeMultiscattering` and `DFGLUT`.
//!
//! Every statement is read off the rung-8 dumps of `webgpu_lights_physical`
//! (`handoff/scouts/rung8/MeshStandardMaterial_1{7,8,9}`,`_20`); the node graph
//! here is arranged to emit that arithmetic in that order. `docs/nodes.md` §8
//! lists where the emitted text differs (temp names, a few hoisted zero
//! initialisers) without changing a value.

use std::rc::Rc;

use super::dfg_lut::dfg_lut;
use super::phong::{self, LightDesc};
use crate::nodes::node::{FnDef, Type};
use crate::nodes::tsl::*;
use crate::nodes::NodeRef;

/// `1 / π`.
pub const RECIPROCAL_PI: f64 = std::f64::consts::FRAC_1_PI;

// ---------------------------------------------------------------------------
// material functions
// ---------------------------------------------------------------------------

/// `getGeometryRoughness()` — `max3( max( abs( dFdx( normalViewGeometry ) ),
/// abs( dFdy( normalViewGeometry ) ) ) )`, with `dFdy`'s sign flip inside
/// `dpdy()`.
pub fn geometry_roughness() -> NodeRef {
    let d = max(
        abs(dpdx(normal_view_geometry())),
        abs(dpdy(normal_view_geometry())),
    );
    max(max(d.x(), d.y()), d.z())
}

/// `getRoughness( { roughness } )` — `min( max( roughness, 0.0525 ) +
/// geometryRoughness, 1.0 )`. The 0.0525 floor keeps the GGX highlight from
/// aliasing to a single pixel.
pub fn get_roughness(roughness: NodeRef) -> NodeRef {
    max(roughness, float(0.0525))
        .add(geometry_roughness())
        .min(float(1.0))
}

// ---------------------------------------------------------------------------
// BSDF
// ---------------------------------------------------------------------------

/// `V_GGX_SmithCorrelated( { alpha, dotNL, dotNV } )`, emitted as a real WGSL
/// `fn` because three.js gives it a layout.
fn v_ggx_smith_correlated() -> Rc<FnDef> {
    thread_local! { static CELL: crate::nodes::node::Lazy<Rc<FnDef>> = const { crate::nodes::node::Lazy::new() }; }
    CELL.with(|c| {
        c.get(|| {
            shader_fn(
                Some("V_GGX_SmithCorrelated"),
                vec![
                    ("alpha", Type::F32),
                    ("dotNL", Type::F32),
                    ("dotNV", Type::F32),
                ],
                Type::F32,
                |args| {
                    let (alpha, dot_nl, dot_nv) =
                        (args[0].clone(), args[1].clone(), args[2].clone());
                    let a2 = alpha.clone().mul(alpha);
                    let gv = dot_nl.clone().mul(
                        a2.clone()
                            .add(
                                float(1.0)
                                    .sub(a2.clone())
                                    .mul(dot_nv.clone().mul(dot_nv.clone())),
                            )
                            .sqrt(),
                    );
                    let gl = dot_nv.mul(
                        a2.clone()
                            .add(float(1.0).sub(a2).mul(dot_nl.clone().mul(dot_nl)))
                            .sqrt(),
                    );
                    // `EPSILON` is 1e-6.
                    float(0.5).div(max(gv.add(gl), float(0.000001)))
                },
            )
        })
    })
}

/// `D_GGX( { alpha, dotNH } )`.
fn d_ggx() -> Rc<FnDef> {
    thread_local! { static CELL: crate::nodes::node::Lazy<Rc<FnDef>> = const { crate::nodes::node::Lazy::new() }; }
    CELL.with(|c| {
        c.get(|| {
            shader_fn(
                Some("D_GGX"),
                vec![("alpha", Type::F32), ("dotNH", Type::F32)],
                Type::F32,
                |args| {
                    let (alpha, dot_nh) = (args[0].clone(), args[1].clone());
                    let a2 = alpha.clone().mul(alpha);
                    let denom =
                        float(1.0).sub(dot_nh.clone().mul(dot_nh).mul(float(1.0).sub(a2.clone())));
                    a2.div(denom.clone().mul(denom)).mul(RECIPROCAL_PI)
                },
            )
        })
    })
}

/// `BRDF_GGX( { lightDirection, f0, f90, roughness } )` — the isotropic,
/// non-iridescent path, which is all this rung's materials ask for.
pub fn brdf_ggx(light_direction: NodeRef, f0: NodeRef, f90: NodeRef) -> NodeRef {
    // `roughness.pow2()` — UE4's alpha.
    let alpha = roughness().mul(roughness());

    let half_dir = light_direction
        .clone()
        .add(position_view_direction())
        .normalize();

    let dot_nl = normal_view().dot(light_direction).clamp(0.0, 1.0);
    let dot_nv = normal_view().dot(position_view_direction()).clamp(0.0, 1.0);
    let dot_nh = normal_view().dot(half_dir.clone()).clamp(0.0, 1.0);
    let dot_vh = position_view_direction().dot(half_dir).clamp(0.0, 1.0);

    let f = phong::f_schlick(f0, f90, dot_vh);
    let v = call(
        &v_ggx_smith_correlated(),
        vec![alpha.clone(), dot_nl, dot_nv],
    );
    let d = call(&d_ggx(), vec![alpha, dot_nh]);

    f.mul(v).mul(d)
}

/// `DFGLUT( { roughness, dotNV } )` — the 16×16 RG16F table, sampled with an
/// explicit UV so no texture matrix is applied.
fn dfg_sample(roughness_value: NodeRef, dot_nv: NodeRef) -> NodeRef {
    let lut = dfg_lut();
    texture_uv(&lut, join(Type::Vec2, vec![roughness_value, dot_nv])).xy()
}

// ---------------------------------------------------------------------------
// PhysicalLightingModel
// ---------------------------------------------------------------------------

/// `new PhysicalLightingModel()` — the clearcoat / sheen / iridescence /
/// transmission / anisotropy flags are all off for this rung's materials, so
/// the model carries only what `start()` prepares.
pub struct Physical {
    /// `this.dfg` — `toConst( 'dfg' )`.
    pub dfg: NodeRef,
    /// `this.multiScatteringCompensation`.
    pub multi_scattering_compensation: NodeRef,
}

impl Physical {
    /// `PhysicalLightingModel.start()`: the DFG lookup and the direct-light
    /// multi-scattering compensation. Emits no statement of its own — both are
    /// `toConst`, so they materialise where they are first used.
    pub fn start() -> Self {
        let dot_nv = normal_view().dot(position_view_direction()).clamp(0.0, 1.0);
        let dfg = dfg_sample(roughness(), dot_nv);

        // `Ess` — the energy of the single-scattering lobe in a white furnace.
        let ess = dfg.x().add(dfg.y());
        let multi_scattering_compensation = specular_color_blended()
            .mul(ess.reciprocal().sub(1.0))
            .add(1.0);

        Self {
            dfg,
            multi_scattering_compensation,
        }
    }

    /// `computeMultiscattering( singleScatter, multiScatter, specularF90, f0 )`
    /// — Fdez-Agüera's approximation, pushed as the two `addAssign`s three.js
    /// emits.
    fn compute_multiscattering(
        &self,
        single_scatter: NodeRef,
        multi_scatter: NodeRef,
        f0: NodeRef,
        out: &mut Vec<NodeRef>,
    ) {
        let fab = self.dfg.clone();

        let fss_ess = f0.clone().mul(fab.x()).add(specular_f90().mul(fab.y()));

        let ess = fab.x().add(fab.y());
        let ems = ess.one_minus();

        // `Favg = Fr + ( 1 - Fr ) * 1/21`.
        let favg = f0.clone().add(f0.one_minus().mul(0.047619));
        let fms = fss_ess
            .clone()
            .mul(favg.clone())
            .div(ems.clone().mul(favg).one_minus());

        out.push(single_scatter.clone().assign(single_scatter.add(fss_ess)));
        out.push(
            multi_scatter
                .clone()
                .assign(multi_scatter.add(fms.mul(ems))),
        );
    }

    /// `PhysicalLightingModel.direct( { lightDirection, lightColor } )`.
    pub fn direct(&self, light_direction: NodeRef, light_color: NodeRef, out: &mut Vec<NodeRef>) {
        let dot_nl = normal_view().dot(light_direction.clone()).clamp(0.0, 1.0);
        let irradiance = dot_nl.mul(light_color);

        // glTF's `fresnel_mix`: light reflected by the specular interface is not
        // available to the diffuse layer.
        let half_dir = light_direction
            .clone()
            .add(position_view_direction())
            .normalize();
        let dot_vh = position_view_direction().dot(half_dir).clamp(0.0, 1.0);
        let f = phong::f_schlick(specular_color(), specular_f90(), dot_vh);

        let specular_brdf = brdf_ggx(light_direction, specular_color_blended(), float(1.0));

        out.push(
            direct_diffuse().assign(
                direct_diffuse().add(
                    irradiance
                        .clone()
                        .mul(brdf_lambert(diffuse_contribution()))
                        .mul(f.one_minus()),
                ),
            ),
        );
        out.push(
            direct_specular().assign(
                direct_specular().add(
                    irradiance
                        .mul(specular_brdf)
                        .mul(self.multi_scattering_compensation.clone()),
                ),
            ),
        );
    }

    /// `PhysicalLightingModel.indirectDiffuse()` — the irradiance that reaches
    /// the diffuse layer, minus the energy the specular lobe took.
    pub fn indirect_diffuse(&self, out: &mut Vec<NodeRef>) {
        out.push(single_scattering().assign(vec3(0.0, 0.0, 0.0)));
        out.push(multi_scattering().assign(vec3(0.0, 0.0, 0.0)));

        self.compute_multiscattering(
            single_scattering(),
            multi_scattering(),
            specular_color(),
            out,
        );

        let diffuse = irradiance()
            .mul(brdf_lambert(diffuse_contribution()))
            .mul(single_scattering().add(multi_scattering()).one_minus());

        out.push(indirect_diffuse().assign(indirect_diffuse().add(diffuse)));
    }

    /// `PhysicalLightingModel.indirectSpecular()`: dielectric and metallic
    /// multi-scattering computed separately and mixed by metalness. With no
    /// environment both `radiance` and `iblIrradiance` stay zero, so the whole
    /// block contributes nothing — three.js emits it regardless, and so do we,
    /// because the zero has to reach the pixel through the same arithmetic.
    pub fn indirect_specular(&self, out: &mut Vec<NodeRef>) {
        out.push(single_scattering_dielectric().assign(vec3(0.0, 0.0, 0.0)));
        out.push(multi_scattering_dielectric().assign(vec3(0.0, 0.0, 0.0)));
        out.push(single_scattering_metallic().assign(vec3(0.0, 0.0, 0.0)));
        out.push(multi_scattering_metallic().assign(vec3(0.0, 0.0, 0.0)));

        self.compute_multiscattering(
            single_scattering_dielectric(),
            multi_scattering_dielectric(),
            specular_color(),
            out,
        );
        self.compute_multiscattering(
            single_scattering_metallic(),
            multi_scattering_metallic(),
            diffuse_color().rgb(),
            out,
        );

        // `radiance` / `iblIrradiance` are `vec3().toVar()` on the lighting
        // context; with no environment node nothing ever adds to them.
        out.push(radiance().assign(vec3(0.0, 0.0, 0.0)));
        out.push(ibl_irradiance().assign(vec3(0.0, 0.0, 0.0)));

        let single_scattering_mixed = mix(
            single_scattering_dielectric(),
            single_scattering_metallic(),
            metalness(),
        );
        let multi_scattering_mixed = mix(
            multi_scattering_dielectric(),
            multi_scattering_metallic(),
            metalness(),
        );

        let cosine_weighted_irradiance = ibl_irradiance().mul(RECIPROCAL_PI);

        let indirect_specular_value = radiance()
            .mul(single_scattering_mixed)
            .add(multi_scattering_mixed.mul(cosine_weighted_irradiance.clone()));

        // Diffuse energy conservation uses the dielectric path.
        let total_scattering_dielectric =
            single_scattering_dielectric().add(multi_scattering_dielectric());
        let diffuse = diffuse_contribution().mul(total_scattering_dielectric.one_minus());
        let indirect_diffuse_value = diffuse.mul(cosine_weighted_irradiance);

        out.push(indirect_specular().assign(indirect_specular().add(indirect_specular_value)));
        out.push(indirect_diffuse().assign(indirect_diffuse().add(indirect_diffuse_value)));
    }

    /// `PhysicalLightingModel.ambientOcclusion()`.
    pub fn ambient_occlusion(&self, out: &mut Vec<NodeRef>) {
        out.push(ambient_occlusion().assign(float(1.0)));

        out.push(indirect_diffuse().assign(indirect_diffuse().mul(ambient_occlusion())));

        let dot_nv = normal_view().dot(position_view_direction()).clamp(0.0, 1.0);
        let ao_nv = dot_nv.add(ambient_occlusion());
        let ao_exp = roughness().mul(-16.0).one_minus().negate().exp2();
        let ao_node = ambient_occlusion()
            .sub(ao_nv.pow(ao_exp).one_minus())
            .clamp(0.0, 1.0);

        out.push(indirect_specular().assign(indirect_specular().mul(ao_node)));
    }
}

/// `BRDF_Lambert( { diffuseColor } )`.
pub fn brdf_lambert(diffuse: NodeRef) -> NodeRef {
    diffuse.mul(RECIPROCAL_PI)
}

/// One light of `LightsNode`'s list through `PhysicalLightingModel`: the
/// ambient and hemisphere lights add to `irradiance`, the analytic ones go
/// through `direct()` with `LightNode.setup()`'s direction/colour pair.
pub fn direct_light(
    model: &Physical,
    light: &LightDesc,
    received_shadow_position: Option<&NodeRef>,
    out: &mut Vec<NodeRef>,
) {
    if let Some((light_direction, light_color)) =
        phong::setup_light(light, received_shadow_position, out)
    {
        model.direct(light_direction, light_color, out);
    }
}
