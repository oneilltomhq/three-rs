//! The lighting and material batch of TSL (sweep 5): `materialAnisotropy`,
//! the anisotropic GGX terms from `BSDF/D_GGX_Anisotropic.js` and
//! `BSDF/V_GGX_SmithCorrelated_Anisotropic.js`, `BSDF/Schlick_to_F0.js`, the
//! `BSDF/LTC.js` rect-area functions and `lights()`. Each item's doc comment
//! names the JS it ports; the WGSL the shader-emitting ones produce is pinned
//! against three's own dump in `tests/nodes_tsl_batch.rs`.
//!
//! The BRDF and LTC functions are standalone here: the port's own lighting
//! calls none of them yet. `BRDF_GGX`'s anisotropic branch is not wired
//! (`src/materials/physical.rs` evaluates the isotropic lobe for every
//! material), and there is no `RectAreaLight` to feed `LTC_Evaluate`.

use std::rc::Rc;

use super::{
    block, call, float, if_then, inverse_sqrt, join, length, mat3_join, material_anisotropy_vector,
    max, shader_fn, texture, to_var, to_var_intent, vec2, vec2_join, vec3, vec3_join,
};
use crate::materials::MeshBasicNodeMaterial;
use crate::nodes::node::{FnDef, Lazy, NodeRef, Type};

/// `RECIPROCAL_PI` — `MathNode.js`: `float( 1 / Math.PI )`.
const RECIPROCAL_PI: f64 = std::f64::consts::FRAC_1_PI;

// ---------------------------------------------------------------------------
// materialAnisotropy (`accessors/MaterialNode.js`)
// ---------------------------------------------------------------------------

/// `materialAnisotropy` — `MaterialNode.ANISOTROPY`. With an `anisotropyMap`
/// the vector is the map's direction (`rg * 2 - 1`, normalised) rotated by
/// [`material_anisotropy_vector`] and scaled by the map's blue channel:
///
/// ```js
/// mat2( v.x, v.y, v.y.negate(), v.x ).mul( polar.rg.mul( 2.0 ).sub( vec2( 1.0 ) ).normalize().mul( polar.b ) )
/// ```
///
/// Without one it is the uniform itself. The physical material's
/// anisotropy setup reads its vector from here.
pub fn material_anisotropy(material: &MeshBasicNodeMaterial) -> NodeRef {
    match &material.anisotropy_map {
        Some(map) => {
            let polar = texture(map);
            let v = material_anisotropy_vector();
            let rotation = join(
                Type::Mat2,
                vec![v.clone().x(), v.clone().y(), v.clone().y().negate(), v.x()],
            );
            rotation.mul(
                polar
                    .clone()
                    .xy()
                    .mul(2.0)
                    .sub(vec2(1.0, 1.0))
                    .normalize()
                    .mul(polar.z()),
            )
        }
        None => material_anisotropy_vector(),
    }
}

// ---------------------------------------------------------------------------
// anisotropic GGX (`functions/BSDF/`)
// ---------------------------------------------------------------------------

/// `D_GGX_Anisotropic( { alphaT, alphaB, dotNH, dotTH, dotBH } )` — the
/// anisotropic GGX normal distribution, from Burley's "Physically-Based
/// Shading at Disney". A layout function, emitted as `fn D_GGX_Anisotropic`.
pub fn d_ggx_anisotropic(
    alpha_t: impl Into<NodeRef>,
    alpha_b: impl Into<NodeRef>,
    dot_nh: impl Into<NodeRef>,
    dot_th: impl Into<NodeRef>,
    dot_bh: impl Into<NodeRef>,
) -> NodeRef {
    thread_local! { static CELL: Lazy<Rc<FnDef>> = const { Lazy::new() }; }
    let def = CELL.with(|c| {
        c.get(|| {
            shader_fn(
                Some("D_GGX_Anisotropic"),
                vec![
                    ("alphaT", Type::F32),
                    ("alphaB", Type::F32),
                    ("dotNH", Type::F32),
                    ("dotTH", Type::F32),
                    ("dotBH", Type::F32),
                ],
                Type::F32,
                |args| {
                    let (alpha_t, alpha_b) = (args[0].clone(), args[1].clone());
                    let (dot_nh, dot_th, dot_bh) =
                        (args[2].clone(), args[3].clone(), args[4].clone());
                    let a2 = alpha_t.clone().mul(alpha_b.clone());
                    let v = vec3_join(vec![
                        alpha_b.mul(dot_th),
                        alpha_t.mul(dot_bh),
                        a2.clone().mul(dot_nh),
                    ]);
                    let v2 = v.clone().dot(v);
                    let w2 = a2.clone().div(v2);
                    float(RECIPROCAL_PI).mul(a2.mul(w2.pow2()))
                },
            )
        })
    });
    call(
        &def,
        vec![
            alpha_t.into(),
            alpha_b.into(),
            dot_nh.into(),
            dot_th.into(),
            dot_bh.into(),
        ],
    )
}

/// `V_GGX_SmithCorrelated_Anisotropic( { alphaT, alphaB, dotTV, dotBV, dotTL,
/// dotBL, dotNV, dotNL } )` — the height-correlated Smith visibility term for
/// the anisotropic lobe (Heitz 2014). A layout function, emitted as `fn
/// V_GGX_SmithCorrelated_Anisotropic`.
#[allow(clippy::too_many_arguments)]
pub fn v_ggx_smith_correlated_anisotropic(
    alpha_t: impl Into<NodeRef>,
    alpha_b: impl Into<NodeRef>,
    dot_tv: impl Into<NodeRef>,
    dot_bv: impl Into<NodeRef>,
    dot_tl: impl Into<NodeRef>,
    dot_bl: impl Into<NodeRef>,
    dot_nv: impl Into<NodeRef>,
    dot_nl: impl Into<NodeRef>,
) -> NodeRef {
    thread_local! { static CELL: Lazy<Rc<FnDef>> = const { Lazy::new() }; }
    let def = CELL.with(|c| {
        c.get(|| {
            shader_fn(
                Some("V_GGX_SmithCorrelated_Anisotropic"),
                vec![
                    ("alphaT", Type::F32),
                    ("alphaB", Type::F32),
                    ("dotTV", Type::F32),
                    ("dotBV", Type::F32),
                    ("dotTL", Type::F32),
                    ("dotBL", Type::F32),
                    ("dotNV", Type::F32),
                    ("dotNL", Type::F32),
                ],
                Type::F32,
                |args| {
                    let [alpha_t, alpha_b, dot_tv, dot_bv, dot_tl, dot_bl, dot_nv, dot_nl] =
                        std::array::from_fn(|i| args[i].clone());
                    let gv = dot_nl.clone().mul(length(vec3_join(vec![
                        alpha_t.clone().mul(dot_tv),
                        alpha_b.clone().mul(dot_bv),
                        dot_nv.clone(),
                    ])));
                    let gl = dot_nv.mul(length(vec3_join(vec![
                        alpha_t.mul(dot_tl),
                        alpha_b.mul(dot_bl),
                        dot_nl,
                    ])));
                    // `EPSILON` — `MathNode.js`: `float( 1e-6 )`.
                    float(0.5).div(max(gv.add(gl), float(1e-6)))
                },
            )
        })
    });
    call(
        &def,
        vec![
            alpha_t.into(),
            alpha_b.into(),
            dot_tv.into(),
            dot_bv.into(),
            dot_tl.into(),
            dot_bl.into(),
            dot_nv.into(),
            dot_nl.into(),
        ],
    )
}

/// `Schlick_to_F0( { f, f90, dotVH } )` — inverts Schlick's Fresnel: the `f0`
/// that would give reflectance `f` at `dotVH`, with `x⁵` clamped below 1 so
/// grazing angles stay finite. `PhysicalLightingModel.start()`'s iridescence
/// branch calls it twice, for the dielectric and metallic thin-film F0s
/// (`crate::materials::physical`, `docs/nodes.md` §95). A layout function,
/// emitted as `fn Schlick_to_F0`.
pub fn schlick_to_f0(
    f: impl Into<NodeRef>,
    f90: impl Into<NodeRef>,
    dot_vh: impl Into<NodeRef>,
) -> NodeRef {
    thread_local! { static CELL: Lazy<Rc<FnDef>> = const { Lazy::new() }; }
    let def = CELL.with(|c| {
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
    });
    call(&def, vec![f.into(), f90.into(), dot_vh.into()])
}

// ---------------------------------------------------------------------------
// LTC (`functions/BSDF/LTC.js`)
// ---------------------------------------------------------------------------

/// `LTC_Uv( { N, V, roughness } )` — where to read the 64×64 LTC lookup
/// tables: `( roughness, sqrt( 1 - saturate( N · V ) ) )` mapped onto texel
/// centres. A layout function, emitted as `fn LTC_Uv`.
pub fn ltc_uv(
    n: impl Into<NodeRef>,
    v: impl Into<NodeRef>,
    roughness: impl Into<NodeRef>,
) -> NodeRef {
    thread_local! { static CELL: Lazy<Rc<FnDef>> = const { Lazy::new() }; }
    let def = CELL.with(|c| {
        c.get(|| {
            shader_fn(
                Some("LTC_Uv"),
                vec![
                    ("N", Type::Vec3),
                    ("V", Type::Vec3),
                    ("roughness", Type::F32),
                ],
                Type::Vec2,
                |args| {
                    let (n, v, roughness) = (args[0].clone(), args[1].clone(), args[2].clone());
                    const LUT_SIZE: f64 = 64.0;
                    const LUT_SCALE: f64 = (LUT_SIZE - 1.0) / LUT_SIZE;
                    const LUT_BIAS: f64 = 0.5 / LUT_SIZE;
                    let dot_nv = n.dot(v).saturate();
                    // `uv.assign( … )` on the `vec2()` itself: three's intent
                    // var, declared with its first value.
                    let uv = to_var_intent(vec2_join(vec![roughness, dot_nv.one_minus().sqrt()]));
                    block(vec![uv.assign(uv.clone().mul(LUT_SCALE).add(LUT_BIAS))], uv)
                },
            )
        })
    });
    call(&def, vec![n.into(), v.into(), roughness.into()])
}

/// `LTC_ClippedSphereFormFactor( { f } )` — the form factor of a
/// horizon-clipped rectangle from its vector form factor (Hill, "Real-Time
/// Area Lighting: a Journey from Research to Production", p. 102). Private in
/// three too.
fn ltc_clipped_sphere_form_factor(f: NodeRef) -> NodeRef {
    thread_local! { static CELL: Lazy<Rc<FnDef>> = const { Lazy::new() }; }
    let def = CELL.with(|c| {
        c.get(|| {
            shader_fn(
                Some("LTC_ClippedSphereFormFactor"),
                vec![("f", Type::Vec3)],
                Type::F32,
                |args| {
                    let f = args[0].clone();
                    let l = length(f.clone());
                    max(
                        l.clone().mul(l.clone()).add(f.z()).div(l.add(1.0)),
                        float(0.0),
                    )
                },
            )
        })
    });
    call(&def, vec![f])
}

/// `LTC_EdgeVectorFormFactor( { v1, v2 } )` — one edge's contribution to the
/// vector form factor, with a rational fit to `θ / sin θ / 2π`. Private in
/// three too.
fn ltc_edge_vector_form_factor(v1: NodeRef, v2: NodeRef) -> NodeRef {
    thread_local! { static CELL: Lazy<Rc<FnDef>> = const { Lazy::new() }; }
    let def = CELL.with(|c| {
        c.get(|| {
            shader_fn(
                Some("LTC_EdgeVectorFormFactor"),
                vec![("v1", Type::Vec3), ("v2", Type::Vec3)],
                Type::Vec3,
                |args| {
                    let (v1, v2) = (args[0].clone(), args[1].clone());
                    let x = v1.clone().dot(v2.clone());
                    let y = to_var(None, x.clone().abs());
                    let a = to_var(
                        None,
                        y.clone()
                            .mul(0.0145206)
                            .add(0.4965155)
                            .mul(y.clone())
                            .add(0.8543985),
                    );
                    let b = to_var(None, y.clone().add(4.1616724).mul(y.clone()).add(3.417594));
                    // `v = a.div( b )`, built once per branch: three writes it
                    // out in each, where one shared node would be hoisted
                    // into a var of its own.
                    let v = || a.clone().div(b.clone());
                    let theta_sintheta = x.clone().greater_than(0.0).select(
                        v(),
                        inverse_sqrt(max(x.clone().mul(x).one_minus(), float(1e-7)))
                            .mul(0.5)
                            .sub(v()),
                    );
                    // Three's `toVar()`s are assigned where they are created;
                    // listing them first keeps both branches of the select
                    // after them.
                    block(vec![y, a, b], v1.cross(v2).mul(theta_sintheta))
                },
            )
        })
    });
    call(&def, vec![v1, v2])
}

/// The common shape of `LTC_Evaluate` and `LTC_Evaluate_Volume`: bail when `P`
/// is behind the quad `p0..p3` (counter-clockwise winding), otherwise project
/// each corner with `project`, sum the four edges' vector form factors and
/// return the clipped-sphere form factor of the sum, passed through `clip`
/// first.
fn ltc_quad(
    p: NodeRef,
    corners: [NodeRef; 4],
    project: impl Fn(NodeRef) -> NodeRef,
    prologue: Vec<NodeRef>,
    clip: impl FnOnce(NodeRef) -> NodeRef,
) -> NodeRef {
    let [p0, p1, p2, p3] = corners;
    let v1 = to_var(None, p1.clone().sub(p0.clone()));
    let v2 = to_var(None, p3.clone().sub(p0.clone()));
    let light_normal = v1.clone().cross(v2.clone());
    let result = to_var(None, vec3(0.0, 0.0, 0.0));

    let coords: [NodeRef; 4] = [&p0, &p1, &p2, &p3]
        .map(|corner| to_var(None, project(corner.clone().sub(p.clone())).normalize()));
    let factor = to_var(None, vec3(0.0, 0.0, 0.0));
    let mut body = prologue;
    body.extend(coords.iter().cloned());
    body.push(factor.clone());
    for i in 0..4 {
        body.push(factor.add_assign(ltc_edge_vector_form_factor(
            coords[i].clone(),
            coords[(i + 1) % 4].clone(),
        )));
    }
    body.push(result.assign(ltc_clipped_sphere_form_factor(clip(factor)).to(Type::Vec3)));

    block(
        vec![
            v1,
            v2,
            result.clone(),
            if_then(light_normal.dot(p.sub(p0)).greater_than_equal(0.0), body),
        ],
        result,
    )
}

/// `LTC_Evaluate( { N, V, P, mInv, p0, p1, p2, p3 } )` — the LTC integral of
/// the quad `p0..p3` (counter-clockwise) seen from `P` with normal `N` and
/// view direction `V`, through the inverse LTC matrix `mInv`; zero from
/// behind the quad. `RectAreaLightNode` feeds it in three. A layout function,
/// emitted as `fn LTC_Evaluate`.
#[allow(clippy::too_many_arguments)]
pub fn ltc_evaluate(
    n: impl Into<NodeRef>,
    v: impl Into<NodeRef>,
    p: impl Into<NodeRef>,
    m_inv: impl Into<NodeRef>,
    p0: impl Into<NodeRef>,
    p1: impl Into<NodeRef>,
    p2: impl Into<NodeRef>,
    p3: impl Into<NodeRef>,
) -> NodeRef {
    thread_local! { static CELL: Lazy<Rc<FnDef>> = const { Lazy::new() }; }
    let def = CELL.with(|c| {
        c.get(|| {
            shader_fn(
                Some("LTC_Evaluate"),
                vec![
                    ("N", Type::Vec3),
                    ("V", Type::Vec3),
                    ("P", Type::Vec3),
                    ("mInv", Type::Mat3),
                    ("p0", Type::Vec3),
                    ("p1", Type::Vec3),
                    ("p2", Type::Vec3),
                    ("p3", Type::Vec3),
                ],
                Type::Vec3,
                |args| {
                    let [n, v, p, m_inv, p0, p1, p2, p3] = std::array::from_fn(|i| args[i].clone());
                    // Orthonormal basis around N. T2 is negated from the
                    // paper, "possibly due to a different handedness".
                    let t1 = v.clone().sub(n.clone().mul(v.dot(n.clone()))).normalize();
                    let t2 = n.clone().cross(t1.clone()).negate();
                    let mat = to_var(None, m_inv.mul(mat3_join(vec![t1, t2, n]).transpose()));
                    ltc_quad(
                        p,
                        [p0, p1, p2, p3],
                        |offset| mat.clone().mul(offset),
                        vec![mat.clone()],
                        |factor| factor,
                    )
                },
            )
        })
    });
    call(
        &def,
        vec![
            n.into(),
            v.into(),
            p.into(),
            m_inv.into(),
            p0.into(),
            p1.into(),
            p2.into(),
            p3.into(),
        ],
    )
}

/// `LTC_Evaluate_Volume( { P, p0, p1, p2, p3 } )` — [`ltc_evaluate`] without
/// the LTC transform, for a point inside a participating medium: the corners
/// are projected as they are and the summed form factor is taken by absolute
/// value. A layout function, emitted as `fn LTC_Evaluate_Volume`.
pub fn ltc_evaluate_volume(
    p: impl Into<NodeRef>,
    p0: impl Into<NodeRef>,
    p1: impl Into<NodeRef>,
    p2: impl Into<NodeRef>,
    p3: impl Into<NodeRef>,
) -> NodeRef {
    thread_local! { static CELL: Lazy<Rc<FnDef>> = const { Lazy::new() }; }
    let def = CELL.with(|c| {
        c.get(|| {
            shader_fn(
                Some("LTC_Evaluate_Volume"),
                vec![
                    ("P", Type::Vec3),
                    ("p0", Type::Vec3),
                    ("p1", Type::Vec3),
                    ("p2", Type::Vec3),
                    ("p3", Type::Vec3),
                ],
                Type::Vec3,
                |args| {
                    let [p, p0, p1, p2, p3] = std::array::from_fn(|i| args[i].clone());
                    ltc_quad(
                        p,
                        [p0, p1, p2, p3],
                        |offset| offset,
                        Vec::new(),
                        |factor| factor.abs(),
                    )
                },
            )
        })
    });
    call(
        &def,
        vec![p.into(), p0.into(), p1.into(), p2.into(), p3.into()],
    )
}

// ---------------------------------------------------------------------------
// lights (`lighting/LightsNode.js`)
// ---------------------------------------------------------------------------

/// `lights( [ light1, light2 ] )` — the light set for
/// [`MeshBasicNodeMaterial::lights_node`], three's `material.lightsNode =
/// lights( [ … ] )`.
///
/// Three returns a `LightsNode` over the light objects. Here a light is owned
/// by the scene rather than shared through an `Rc`, so the set is indices into
/// the renderer's light list (scene-traversal order), and it is a plain
/// `Vec<usize>` the material holds rather than a node a graph can use.
///
/// ```
/// use three_rs::materials::MeshBasicNodeMaterial;
/// use three_rs::nodes::tsl::lights;
///
/// let mut material = MeshBasicNodeMaterial::new();
/// // Lit by the first and third lights of the scene only.
/// material.lights_node = Some(lights([0, 2]));
/// assert_eq!(material.lights_node, Some(vec![0, 2]));
/// ```
pub fn lights(lights: impl IntoIterator<Item = usize>) -> Vec<usize> {
    lights.into_iter().collect()
}
