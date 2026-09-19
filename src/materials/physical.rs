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

/// `D_Charlie( { roughness, dotNH } )` — Estevez and Kulla 2017, "Production
/// Friendly Microfacet Sheen BRDF", by way of Filament. A real WGSL `fn`,
/// because three.js gives it a layout.
fn d_charlie() -> Rc<FnDef> {
    thread_local! { static CELL: crate::nodes::node::Lazy<Rc<FnDef>> = const { crate::nodes::node::Lazy::new() }; }
    CELL.with(|c| {
        c.get(|| {
            shader_fn(
                Some("D_Charlie"),
                vec![("roughness", Type::F32), ("dotNH", Type::F32)],
                Type::F32,
                |args| {
                    let (roughness_value, dot_nh) = (args[0].clone(), args[1].clone());
                    let alpha = roughness_value.clone().mul(roughness_value);
                    let inv_alpha = float(1.0).div(alpha);
                    let cos2h = dot_nh.clone().mul(dot_nh);
                    // `2^( -14/2 )`, so `sin2h^2 > 0` in fp16.
                    let sin2h = max(cos2h.one_minus(), float(0.0078125));
                    float(2.0)
                        .add(inv_alpha.clone())
                        .mul(sin2h.pow(inv_alpha.mul(0.5)))
                        .div(2.0 * std::f64::consts::PI)
                },
            )
        })
    })
}

/// `V_Neubelt( { dotNV, dotNL } )` — Neubelt and Pettineo 2013, "Crafting a
/// Next-gen Material Pipeline for The Order: 1886".
fn v_neubelt() -> Rc<FnDef> {
    thread_local! { static CELL: crate::nodes::node::Lazy<Rc<FnDef>> = const { crate::nodes::node::Lazy::new() }; }
    CELL.with(|c| {
        c.get(|| {
            shader_fn(
                Some("V_Neubelt"),
                vec![("dotNV", Type::F32), ("dotNL", Type::F32)],
                Type::F32,
                |args| {
                    let (dot_nv, dot_nl) = (args[0].clone(), args[1].clone());
                    float(1.0)
                        .div(
                            float(4.0)
                                .mul(dot_nl.clone().add(dot_nv.clone()).sub(dot_nl.mul(dot_nv))),
                        )
                        .saturate()
                },
            )
        })
    })
}

/// `BRDF_Sheen( { lightDirection } )` — `sheen * D_Charlie * V_Neubelt`.
///
/// No page on this ladder reaches it: `webgpu_loader_gltf_sheen` has no light
/// in the scene, so `PhysicalLightingModel.direct()` is never called and
/// neither `D_Charlie` nor `V_Neubelt` appears in three's own dump for it. It
/// is written from `BRDF_Sheen.js` all the same, because `direct()`'s sheen
/// branch is not optional in three and leaving it out would make the port's
/// lighting model quietly different for the first sheen page that does light
/// its model. `docs/nodes.md` §25.
pub fn brdf_sheen(light_direction: NodeRef) -> NodeRef {
    let half_dir = light_direction
        .clone()
        .add(position_view_direction())
        .normalize();

    let dot_nl = normal_view().dot(light_direction).saturate();
    let dot_nv = normal_view().dot(position_view_direction()).saturate();
    let dot_nh = normal_view().dot(half_dir).saturate();

    let d = call(&d_charlie(), vec![sheen_roughness(), dot_nh]);
    let v = call(&v_neubelt(), vec![dot_nv, dot_nl]);

    sheen().mul(d).mul(v)
}

/// `IBLSheenBRDF( { normal, viewDir, roughness } )` — the curve-fit of the
/// Charlie sheen BRDF integrated over the hemisphere. Three gives it no
/// layout, so it is inlined at each of its four uses rather than emitted as a
/// `fn`; `r2` and `rInv` are read twice each and so become temps, which is
/// what the dump shows.
fn ibl_sheen_brdf(normal: NodeRef, view_dir: NodeRef, roughness_value: NodeRef) -> NodeRef {
    let dot_nv = normal.dot(view_dir).saturate();
    let r2 = roughness_value.clone().mul(roughness_value.clone());
    let r_inv = roughness_value.clone().add(0.1).reciprocal();

    let a = float(-1.9362)
        .add(roughness_value.clone().mul(1.0678))
        .add(r2.clone().mul(0.4573))
        .sub(r_inv.clone().mul(0.8469));
    let b = float(-0.6014)
        .add(roughness_value.mul(0.5538))
        .sub(r2.mul(0.4670))
        .sub(r_inv.mul(0.1255));

    exp(a.mul(dot_nv).add(b)).saturate()
}

/// `IBLSheenBRDF( { normalView, positionViewDirection, sheenRoughness } )`,
/// the argument triple every indirect use passes.
fn sheen_albedo() -> NodeRef {
    ibl_sheen_brdf(normal_view(), position_view_direction(), sheen_roughness())
}

/// `sheen.r.max( sheen.g ).max( sheen.b ).mul( albedo ).oneMinus()` — the
/// energy the sheen lobe took, which the layer underneath does not get.
fn sheen_energy_comp(albedo: NodeRef) -> NodeRef {
    sheen()
        .x()
        .max(sheen().y())
        .max(sheen().z())
        .mul(albedo)
        .one_minus()
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

/// `new PhysicalLightingModel( clearcoat, sheen, … )` — the clearcoat /
/// iridescence / transmission / anisotropy flags are all off for this ladder's
/// materials; `sheen` is `MeshPhysicalNodeMaterial.useSheen`, which
/// `webgpu_loader_gltf_sheen`'s fabric turns on.
pub struct Physical {
    /// `this.dfg` — `toConst( 'dfg' )`.
    pub dfg: NodeRef,
    /// `this.multiScatteringCompensation`.
    pub multi_scattering_compensation: NodeRef,
    /// `this.sheen`. Every sheen branch below is gated on it, and with it
    /// false the emitted WGSL is what it was before sheen existed.
    pub sheen: bool,
}

impl Physical {
    /// `PhysicalLightingModel.start()`: the DFG lookup and the direct-light
    /// multi-scattering compensation, both `toConst` so they materialise where
    /// they are first used — plus, with sheen, the two `vec3().toVar()`
    /// accumulators, which three declares here and the dump shows here.
    pub fn start(sheen: bool, out: &mut Vec<NodeRef>) -> Self {
        if sheen {
            out.push(sheen_specular_direct().assign(vec3(0.0, 0.0, 0.0)));
            out.push(sheen_specular_indirect().assign(vec3(0.0, 0.0, 0.0)));
        }

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
            sheen,
        }
    }

    /// `PhysicalLightingModel.finish()` — the sheen lobe is added to the
    /// outgoing light after `setupLighting()` has summed the four accumulators.
    pub fn finish(&self, out: &mut Vec<NodeRef>) {
        if self.sheen {
            out.push(
                outgoing_light().assign(
                    outgoing_light()
                        .add(sheen_specular_direct())
                        .add(sheen_specular_indirect()),
                ),
            );
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
        // `irradiance` is `.toVar()` in three; without sheen nothing assigns
        // to it again, so the port leaves it inline there and every already
        // green example generates exactly the WGSL it did before.
        let irradiance = if self.sheen {
            to_var(None, dot_nl.mul(light_color))
        } else {
            dot_nl.mul(light_color)
        };

        if self.sheen {
            out.push(
                sheen_specular_direct().assign(
                    sheen_specular_direct()
                        .add(irradiance.clone().mul(brdf_sheen(light_direction.clone()))),
                ),
            );

            // The view and the light each see their own sheen albedo here;
            // the energy taken is the larger of the two.
            let albedo_v = sheen_albedo();
            let albedo_l =
                ibl_sheen_brdf(normal_view(), light_direction.clone(), sheen_roughness());

            out.push(
                irradiance.clone().assign(
                    irradiance
                        .clone()
                        .mul(sheen_energy_comp(albedo_v.max(albedo_l))),
                ),
            );
        }

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

        let diffuse = if self.sheen {
            let diffuse = to_var(None, diffuse);

            // Not a `toVar` in three: the node is simply read twice, and the
            // builder gives a shared node a temp of its own.
            let albedo = sheen_albedo();
            out.push(
                sheen_specular_indirect().assign(
                    sheen_specular_indirect().add(
                        irradiance()
                            .mul(sheen())
                            .mul(albedo.clone())
                            .mul(RECIPROCAL_PI),
                    ),
                ),
            );

            out.push(
                diffuse
                    .clone()
                    .assign(diffuse.clone().mul(sheen_energy_comp(albedo))),
            );
            diffuse
        } else {
            diffuse
        };

        out.push(indirect_diffuse().assign(indirect_diffuse().add(diffuse)));
    }

    /// `PhysicalLightingModel.indirectSpecular()`: dielectric and metallic
    /// multi-scattering computed separately and mixed by metalness. With no
    /// environment both `radiance` and `iblIrradiance` stay zero, so the whole
    /// block contributes nothing — three.js emits it regardless, and so do we,
    /// because the zero has to reach the pixel through the same arithmetic.
    pub fn indirect_specular(&self, has_environment: bool, out: &mut Vec<NodeRef>) {
        if self.sheen {
            out.push(
                sheen_specular_indirect().assign(
                    sheen_specular_indirect().add(
                        ibl_irradiance()
                            .mul(sheen())
                            .mul(sheen_albedo())
                            .mul(RECIPROCAL_PI),
                    ),
                ),
            );
        }

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
        // context, declared at their first use. With an environment that use
        // is `EnvironmentNode.setup()`, which runs as a lighting node before
        // `indirectSpecular` and so carries the zeros with it; with none,
        // nothing ever adds to them and they are declared here.
        if !has_environment {
            out.push(radiance().assign(vec3(0.0, 0.0, 0.0)));
            out.push(ibl_irradiance().assign(vec3(0.0, 0.0, 0.0)));
        }

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

        let (indirect_specular_value, indirect_diffuse_value) = if self.sheen {
            // Both are `.toVar()` in three, so that the sheen energy
            // compensation can multiply them in place.
            let specular = to_var(None, indirect_specular_value);
            let diffuse = to_var(None, indirect_diffuse_value);

            let comp = sheen_energy_comp(sheen_albedo());
            out.push(specular.clone().assign(specular.clone().mul(comp.clone())));
            out.push(diffuse.clone().assign(diffuse.clone().mul(comp)));

            (specular, diffuse)
        } else {
            (indirect_specular_value, indirect_diffuse_value)
        };

        out.push(indirect_specular().assign(indirect_specular().add(indirect_specular_value)));
        out.push(indirect_diffuse().assign(indirect_diffuse().add(indirect_diffuse_value)));
    }

    /// `PhysicalLightingModel.ambientOcclusion()`.
    ///
    /// `has_ao_node` says whether an `AONode` already ran as a lighting node
    /// and so already declared the `ambientOcclusion` var — three builds it as
    /// `float( 1 ).toVar( 'ambientOcclusion' )`, whose initialiser is emitted
    /// at its *first* read, and with an `aoMap` that read is `AONode`'s
    /// `mulAssign`, not this method.
    pub fn ambient_occlusion(&self, has_ao_node: bool, out: &mut Vec<NodeRef>) {
        if !has_ao_node {
            out.push(ambient_occlusion().assign(float(1.0)));
        }

        if self.sheen {
            out.push(
                sheen_specular_indirect()
                    .assign(sheen_specular_indirect().mul(ambient_occlusion())),
            );
        }

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
