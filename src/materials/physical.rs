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
use super::transmission;
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
/// `has_normal` is `builder.geometry.hasAttribute( 'normal' )`: with no normal
/// attribute the function returns `float( 0 )` outright, and the derivative
/// pair is never emitted. `webgpu_deferred`'s resolve quad is the first
/// geometry on the ladder that has none, and its dump reads
/// `min( ( max( nodeVar4.w, 0.0525 ) + 0.0 ), 1.0 )`.
pub fn geometry_roughness(has_normal: bool) -> NodeRef {
    if !has_normal {
        return float(0.0);
    }
    let d = max(
        abs(dpdx(normal_view_geometry())),
        abs(dpdy(normal_view_geometry())),
    );
    max(max(d.x(), d.y()), d.z())
}

/// `getRoughness( { roughness } )` — `min( max( roughness, 0.0525 ) +
/// geometryRoughness, 1.0 )`. The 0.0525 floor keeps the GGX highlight from
/// aliasing to a single pixel.
pub fn get_roughness(roughness: NodeRef, has_normal: bool) -> NodeRef {
    max(roughness, float(0.0525))
        .add(geometry_roughness(has_normal))
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

/// `BRDF_GGX( { lightDirection, f0, f90, roughness, f, USE_IRIDESCENCE } )` —
/// the isotropic path. `iridescence_fresnel` is `f`, present only when the
/// material is iridescent; three's `defined( USE_IRIDESCENCE )` branch then
/// blends the Schlick Fresnel towards it by `Iridescence`.
///
/// No page on this ladder reaches the blend: `webgpu_loader_gltf_iridescence`
/// has no light in the scene, so `direct()` never runs and the dump for it
/// carries no call at all. It is written from `BRDF_GGX.js` for the same
/// reason `BRDF_Sheen` is — `docs/nodes.md` §30.
pub fn brdf_ggx(
    light_direction: NodeRef,
    f0: NodeRef,
    f90: NodeRef,
    iridescence_fresnel: Option<NodeRef>,
) -> NodeRef {
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
    let f = match iridescence_fresnel {
        Some(fresnel) => mix(f, fresnel, iridescence()),
        None => f,
    };
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

/// `new PhysicalLightingModel( clearcoat, sheen, iridescence, anisotropy,
/// transmission, dispersion, retroreflection )`. Iridescence, dispersion and
/// retroreflection are still off for every material the ladder builds;
/// `sheen` is `MeshPhysicalNodeMaterial.useSheen`, which
/// `webgpu_loader_gltf_sheen`'s fabric turns on, and clearcoat and anisotropy
/// are the two the barn lamp turns on.
pub struct Physical {
    /// `this.dfg` — `toConst( 'dfg' )`.
    pub dfg: NodeRef,
    /// `this.multiScatteringCompensation`.
    pub multi_scattering_compensation: NodeRef,
    /// `this.sheen`. Every sheen branch below is gated on it, and with it
    /// false the emitted WGSL is what it was before sheen existed.
    pub sheen: bool,
    /// `this.clearcoat` — the flag that gives the model its three clearcoat
    /// accumulators and the extra lobe in `indirectSpecular()` / `finish()`.
    pub clearcoat: bool,
    /// `this.iridescenceFresnel` — `mix( dielectric, metallic, metalness )`,
    /// read only by `direct()`; `None` when the material is not iridescent.
    pub iridescence_fresnel: Option<NodeRef>,
    /// `this.iridescenceF0Dielectric` / `.iridescenceF0Metallic` — the two F0s
    /// `computeMultiscattering` blends towards by `Iridescence`.
    pub iridescence_f0_dielectric: Option<NodeRef>,
    pub iridescence_f0_metallic: Option<NodeRef>,
    /// `builder.context.backdrop` — `getIBLVolumeRefraction()`'s `vec4`, set
    /// by the transmission branch of `start()` and read once more where
    /// `LightsNode` blends it into `totalDiffuse`.
    pub backdrop: Option<NodeRef>,
}

impl Physical {
    /// `PhysicalLightingModel.start()`: the DFG lookup, the direct-light
    /// multi-scattering compensation and — with sheen or clearcoat — the
    /// lobes' accumulators, plus, with transmission, the screen-space
    /// backdrop. The first two are `toConst`, so they materialise where they
    /// are first used; the accumulators are `toVar` and so land here.
    pub fn start(
        sheen: bool,
        clearcoat: bool,
        iridescence: bool,
        opaque_frame: Option<&transmission::OpaqueFrame>,
        out: &mut Vec<NodeRef>,
    ) -> Self {
        if sheen {
            out.push(sheen_specular_direct().assign(vec3(0.0, 0.0, 0.0)));
            out.push(sheen_specular_indirect().assign(vec3(0.0, 0.0, 0.0)));
        }

        // The iridescence block. Nothing here is a `toVar` or a `toConst`, so
        // none of it is emitted until something reads it — with no light in
        // the scene that is `computeMultiscattering`, and `iridescenceFresnel`
        // is never emitted at all.
        let (iridescence_fresnel, iridescence_f0_dielectric, iridescence_f0_metallic) =
            if iridescence {
                let dot_nvi = normal_view().dot(position_view_direction()).clamp(0.0, 1.0);
                let eval = |base_f0: NodeRef| {
                    call(
                        &eval_iridescence(),
                        vec![
                            float(1.0),
                            iridescence_ior(),
                            dot_nvi.clone(),
                            iridescence_thickness(),
                            base_f0,
                        ],
                    )
                };
                let dielectric = eval(specular_color());
                let metallic = eval(diffuse_color().rgb());
                let to_f0 =
                    |f: NodeRef| call(&schlick_to_f0(), vec![f, float(1.0), dot_nvi.clone()]);
                (
                    Some(mix(dielectric.clone(), metallic.clone(), metalness())),
                    Some(to_f0(dielectric)),
                    Some(to_f0(metallic)),
                )
            } else {
                (None, None, None)
            };

        // The transmission branch runs before the DFG lookup, and its
        // `diffuseColor.a.mulAssign()` is what pulls the whole screen-space
        // read into the flow here rather than at `totalDiffuse`.
        let backdrop = opaque_frame.map(|frame| {
            let v = transmission::world_view_vector();
            // Three keeps the refraction result in a var (`nodeVar42`), which
            // both the `DiffuseColor.w` write below and `total_diffuse()` read;
            // without the var the whole bicubic expression is emitted twice.
            let backdrop = to_var(
                None,
                transmission::ibl_volume_refraction(
                    &frame.texture,
                    normal_world(),
                    v,
                    position_world(),
                    Self::environment_brdf,
                ),
            );
            out.push(diffuse_color().w().assign(diffuse_color().w().mul(mix(
                float(1.0),
                backdrop.clone().w(),
                transmission(),
            ))));
            backdrop
        });

        let dot_nv = normal_view().dot(position_view_direction()).clamp(0.0, 1.0);
        let dfg = dfg_sample(roughness(), dot_nv);

        // `Ess` — the energy of the single-scattering lobe in a white furnace.
        let ess = dfg.x().add(dfg.y());
        let multi_scattering_compensation = specular_color_blended()
            .mul(ess.reciprocal().sub(1.0))
            .add(1.0);

        if clearcoat {
            out.push(clearcoat_radiance().assign(vec3(0.0, 0.0, 0.0)));
            out.push(clearcoat_specular_direct().assign(vec3(0.0, 0.0, 0.0)));
            out.push(clearcoat_specular_indirect().assign(vec3(0.0, 0.0, 0.0)));
        }

        Self {
            dfg,
            multi_scattering_compensation,
            sheen,
            clearcoat,
            iridescence_fresnel,
            iridescence_f0_dielectric,
            iridescence_f0_metallic,
            backdrop,
        }
    }

    /// `LightsNode.setup()`'s backdrop blend: with a backdrop the diffuse total
    /// is `mix( vec4( totalDiffuse, 1 ), backdrop, backdropAlpha ).xyz`, where
    /// the alpha is `Transmission`.
    pub fn total_diffuse(&self, direct_plus_indirect: NodeRef) -> NodeRef {
        match &self.backdrop {
            Some(backdrop) => mix(
                vec4_join(vec![direct_plus_indirect, float(1.0)]),
                backdrop.clone(),
                transmission(),
            )
            .xyz(),
            None => direct_plus_indirect,
        }
    }

    /// `PhysicalLightingModel.finish()` — the sheen lobe is added to the
    /// outgoing light after `setupLighting()` has summed the four
    /// accumulators, and the clearcoat branch then attenuates that base lobe
    /// by the coat's Fresnel and adds the coat's own specular on top.
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

        if self.clearcoat {
            let dot_nvcc = clearcoat_normal_view()
                .dot(position_view_direction())
                .clamp(0.0, 1.0);
            let fcc = phong::f_schlick(vec3(0.04, 0.04, 0.04), float(1.0), dot_nvcc);
            let value = outgoing_light().mul(clearcoat().mul(fcc).one_minus()).add(
                clearcoat_specular_direct()
                    .add(clearcoat_specular_indirect())
                    .mul(clearcoat()),
            );
            out.push(outgoing_light().assign(value));
        }
    }

    /// `EnvironmentBRDF( { dotNV, specularColor, specularF90, roughness } )` —
    /// the split-sum approximation over the DFG table.
    pub(crate) fn environment_brdf(
        dot_nv: NodeRef,
        specular_color_value: NodeRef,
        specular_f90_value: NodeRef,
        roughness_value: NodeRef,
    ) -> NodeRef {
        let fab = dfg_sample(roughness_value, dot_nv);
        specular_color_value
            .mul(fab.x())
            .add(specular_f90_value.mul(fab.y()))
    }

    /// `computeMultiscattering( singleScatter, multiScatter, specularF90, f0 )`
    /// — Fdez-Agüera's approximation, pushed as the two `addAssign`s three.js
    /// emits.
    fn compute_multiscattering(
        &self,
        single_scatter: NodeRef,
        multi_scatter: NodeRef,
        f0: NodeRef,
        iridescence_f0: Option<NodeRef>,
        out: &mut Vec<NodeRef>,
    ) {
        let fab = self.dfg.clone();

        // `const Fr = iridescenceF0 ? iridescence.mix( f0, iridescenceF0 ) : f0`
        // — a fresh `mix` per call, so the dump shows it twice for the two
        // dielectric uses rather than once in a shared temp.
        let f0 = match iridescence_f0 {
            Some(iridescence_f0) => mix(f0, iridescence_f0, iridescence()),
            None => f0,
        };

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

        let specular_brdf = brdf_ggx(
            light_direction,
            specular_color_blended(),
            float(1.0),
            self.iridescence_fresnel.clone(),
        );

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
            self.iridescence_f0_dielectric.clone(),
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

        if self.clearcoat {
            let dot_nvcc = clearcoat_normal_view()
                .dot(position_view_direction())
                .clamp(0.0, 1.0);
            let clearcoat_env = Self::environment_brdf(
                dot_nvcc,
                vec3(0.04, 0.04, 0.04),
                float(1.0),
                clearcoat_roughness(),
            );
            out.push(clearcoat_specular_indirect().assign(
                clearcoat_specular_indirect().add(clearcoat_radiance().mul(clearcoat_env)),
            ));
        }

        out.push(single_scattering_dielectric().assign(vec3(0.0, 0.0, 0.0)));
        out.push(multi_scattering_dielectric().assign(vec3(0.0, 0.0, 0.0)));
        out.push(single_scattering_metallic().assign(vec3(0.0, 0.0, 0.0)));
        out.push(multi_scattering_metallic().assign(vec3(0.0, 0.0, 0.0)));

        self.compute_multiscattering(
            single_scattering_dielectric(),
            multi_scattering_dielectric(),
            specular_color(),
            self.iridescence_f0_dielectric.clone(),
            out,
        );
        self.compute_multiscattering(
            single_scattering_metallic(),
            multi_scattering_metallic(),
            diffuse_color().rgb(),
            self.iridescence_f0_metallic.clone(),
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

        if self.clearcoat {
            out.push(
                clearcoat_specular_indirect()
                    .assign(clearcoat_specular_indirect().mul(ambient_occlusion())),
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

// ---------------------------------------------------------------------------
// iridescence — `PhysicalLightingModel.js`' thin-film block
// ---------------------------------------------------------------------------

/// `XYZ_TO_REC709`, three's `mat3( … )` written out in the column-major order
/// WGSL's `mat3x3<f32>` constructor takes.
fn xyz_to_rec709() -> NodeRef {
    join(
        Type::Mat3,
        vec![
            float(3.2404542),
            float(-0.9692660),
            float(0.0556434),
            float(-1.5371385),
            float(1.8760108),
            float(-0.2040259),
            float(-0.4985314),
            float(0.0415560),
            float(1.0572252),
        ],
    )
}

/// `Fresnel0ToIor( fresnel0 )` — `( 1 + sqrt( F0 ) ) / ( 1 - sqrt( F0 ) )`,
/// the IOR a dielectric with that normal-incidence reflectance has. Three
/// notes that the `fresnel0 == 1` case is not handled; the single caller
/// clamps to 0.9999 first, which is why that clamp is not optional.
fn fresnel0_to_ior(fresnel0: NodeRef) -> NodeRef {
    let sqrt_f0 = sqrt(fresnel0);
    vec3(1.0, 1.0, 1.0)
        .add(sqrt_f0.clone())
        .div(vec3(1.0, 1.0, 1.0).sub(sqrt_f0))
}

/// `IorToFresnel0( transmittedIor, incidentIor )` — the inverse, unsquared:
/// the caller squares it, and three leaves the `pow2()` inside so the
/// difference-over-sum lands in a temp of its own.
fn ior_to_fresnel0(transmitted_ior: NodeRef, incident_ior: NodeRef) -> NodeRef {
    let r = transmitted_ior
        .clone()
        .sub(incident_ior.clone())
        .div(transmitted_ior.add(incident_ior));
    r.clone().mul(r)
}

/// `evalSensitivity( OPD, shift )` — Belcour and Barla's Fourier-space fit of
/// the XYZ colour matching functions, converted to linear sRGB. Three gives it
/// no layout, so it is inlined at its one use inside the `m` loop.
fn eval_sensitivity(opd: NodeRef, shift: NodeRef) -> NodeRef {
    // `2 * PI * 1e-9` — the OPD is in nanometres and the fit wants metres.
    let phase = opd.mul(2.0 * std::f64::consts::PI * 1.0e-9);
    let val = vec3(5.4856e-13, 4.4201e-13, 5.2481e-13);
    let pos = vec3(1.6810e+06, 1.7953e+06, 2.2084e+06);
    let var = vec3(4.3278e+09, 9.3046e+09, 6.6121e+09);

    // The fourth, wider Gaussian, which only the x channel carries. Its
    // amplitude is folded on the CPU, exactly as three writes it.
    let x = float(9.7470e-14 * (2.0 * std::f64::consts::PI * 4.5282e+09).sqrt())
        .mul(phase.clone().mul(2.2399e+06).add(shift.clone().x()).cos())
        .mul(exp(phase.clone().mul(phase.clone()).mul(-4.5282e+09)));

    let xyz = val
        .mul(sqrt(var.clone().mul(2.0 * std::f64::consts::PI)))
        .mul(pos.mul(phase.clone()).add(shift).cos())
        .mul(exp(phase.clone().mul(phase).negate().mul(var)));
    let xyz = join(
        Type::Vec3,
        vec![xyz.clone().x().add(x), xyz.clone().y(), xyz.z()],
    )
    .div(1.0685e-7);

    xyz_to_rec709().mul(xyz)
}

/// `evalIridescence( { outsideIOR, eta2, cosTheta1, thinFilmThickness, baseF0 } )`
/// — the airy-summed reflectance of a thin film over a base with the given F0.
/// Three gives it a layout, so it is a real WGSL `fn`, early `return` and `m`
/// loop and all. `docs/nodes.md` §30.
fn eval_iridescence() -> Rc<FnDef> {
    thread_local! { static CELL: crate::nodes::node::Lazy<Rc<FnDef>> = const { crate::nodes::node::Lazy::new() }; }
    CELL.with(|c| {
        c.get(|| {
            shader_fn(
                Some("evalIridescence"),
                vec![
                    ("outsideIOR", Type::F32),
                    ("eta2", Type::F32),
                    ("cosTheta1", Type::F32),
                    ("thinFilmThickness", Type::F32),
                    ("baseF0", Type::Vec3),
                ],
                Type::Vec3,
                |args| {
                    let (outside_ior, eta2, cos_theta1, thin_film_thickness, base_f0) = (
                        args[0].clone(),
                        args[1].clone(),
                        args[2].clone(),
                        args[3].clone(),
                        args[4].clone(),
                    );

                    // Force the film's IOR back to the outside medium's as the
                    // thickness vanishes, so a zero-thickness film is a no-op.
                    let iridescence_ior_value = mix(
                        outside_ior.clone(),
                        eta2,
                        smoothstep(0.0, 0.03, thin_film_thickness.clone()),
                    );

                    // Snell's law on the base layer.
                    let eta = outside_ior.clone().div(iridescence_ior_value.clone());
                    let sin_theta2_sq = eta
                        .clone()
                        .mul(eta)
                        .mul(cos_theta1.clone().mul(cos_theta1.clone()).one_minus());
                    let cos_theta2_sq = sin_theta2_sq.one_minus();

                    // Total internal reflection: everything comes back.
                    let tir = if_then(
                        cos_theta2_sq.clone().less_than(float(0.0)),
                        vec![return_statement(vec3(1.0, 1.0, 1.0))],
                    );

                    let cos_theta2 = sqrt(cos_theta2_sq);

                    // First interface (outside ↔ film).
                    let r0 = ior_to_fresnel0(iridescence_ior_value.clone(), outside_ior.clone());
                    let r12 = phong::f_schlick(r0, float(1.0), cos_theta1);
                    let t121 = r12.clone().one_minus();
                    let phi12 = iridescence_ior_value
                        .clone()
                        .less_than(outside_ior)
                        .select(float(std::f64::consts::PI), float(0.0));
                    let phi21 = float(std::f64::consts::PI).sub(phi12);

                    // Second interface (film ↔ base). The 0.9999 guard keeps
                    // `Fresnel0ToIor`'s denominator off zero.
                    let base_ior = fresnel0_to_ior(base_f0.clamp(0.0, 0.9999));
                    let r1 = ior_to_fresnel0(
                        base_ior.clone(),
                        iridescence_ior_value.clone().to(Type::Vec3),
                    );
                    let r23 = phong::f_schlick(r1, float(1.0), cos_theta2.clone());
                    let pi_or_zero = |c: NodeRef| {
                        c.less_than(iridescence_ior_value.clone())
                            .select(float(std::f64::consts::PI), float(0.0))
                    };
                    let phi23 = join(
                        Type::Vec3,
                        vec![
                            pi_or_zero(base_ior.clone().x()),
                            pi_or_zero(base_ior.clone().y()),
                            pi_or_zero(base_ior.z()),
                        ],
                    );

                    // Optical path difference and the total phase shift.
                    let opd = iridescence_ior_value
                        .mul(thin_film_thickness)
                        .mul(cos_theta2)
                        .mul(2.0);
                    let phi = phi21.to(Type::Vec3).add(phi23);

                    let r123 = r12.clone().mul(r23.clone()).clamp(1e-5, 0.9999);
                    let r123_sqrt = sqrt(r123.clone());
                    let rs = t121
                        .clone()
                        .mul(t121.clone())
                        .mul(r23)
                        .div(vec3(1.0, 1.0, 1.0).sub(r123));

                    // m = 0, the DC term.
                    let i = to_var(None, r12.add(rs.clone()));
                    // m > 0, the pairs of diracs.
                    let cm = to_var(None, rs.sub(t121));

                    let body = {
                        let (i, cm, opd, phi, r123_sqrt) =
                            (i.clone(), cm.clone(), opd, phi, r123_sqrt);
                        loop_range("m", int(1), int(2), true, move |m| {
                            let m = m.to(Type::F32);
                            let sm = eval_sensitivity(
                                m.clone().mul(opd.clone()),
                                m.to(Type::Vec3).mul(phi.clone()),
                            )
                            .mul(2.0);
                            vec![
                                cm.clone().assign(cm.clone().mul(r123_sqrt.clone())),
                                i.clone().assign(i.clone().add(cm.clone().mul(sm))),
                            ]
                        })
                    };

                    // Out-of-gamut colours can come out negative; clamp them.
                    block(vec![tir, body], max(i, vec3(0.0, 0.0, 0.0)))
                },
            )
        })
    })
}

/// `Schlick_to_F0( { f, f90, dotVH } )` — the F0 a Schlick Fresnel with this
/// value at this angle would have come from. A real `fn`: three gives it a
/// layout.
fn schlick_to_f0() -> Rc<FnDef> {
    thread_local! { static CELL: crate::nodes::node::Lazy<Rc<FnDef>> = const { crate::nodes::node::Lazy::new() }; }
    CELL.with(|c| {
        c.get(|| {
            shader_fn(
                Some("Schlick_to_F0"),
                vec![("f", Type::Vec3), ("f90", Type::F32), ("dotVH", Type::F32)],
                Type::Vec3,
                |args| {
                    let (f, f90, dot_vh) = (args[0].clone(), args[1].clone(), args[2].clone());
                    let x = dot_vh.one_minus().saturate();
                    let x2 = x.clone().mul(x.clone());
                    let x5 = x.mul(x2.clone()).mul(x2).clamp(0.0, 0.9999);
                    f.sub(f90.to(Type::Vec3).mul(x5.clone().to(Type::Vec3)))
                        .div(x5.one_minus().to(Type::Vec3))
                },
            )
        })
    })
}
