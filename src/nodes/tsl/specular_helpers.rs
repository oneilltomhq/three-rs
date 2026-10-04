//! Port of `examples/jsm/tsl/utils/SpecularHelpers.js`: the microfacet terms
//! the SSR-denoise stack shares — bounded-VNDF GGX sampling, the GTR
//! distribution, Smith geometry, Schlick Fresnel, the specular dominant factor
//! for parallax-corrected ray lengths, and the equirectangular direction / PDF
//! / MIS helpers environment importance sampling uses.
//!
//! Every function keeps three's `Fn` shape: the ones three gives a layout
//! (`SampleGGXVNDF`, `getSpecularDominantFactor`, `equirectUvToDir`,
//! `equirectDirPdf`, `misPowerHeuristic`) are real WGSL functions here too,
//! and the plain `Fn`s (`D_GTR`, `SmithG`, `GeometryTerm`, `GGXVNDFPdf`,
//! `F_Schlick`, `ggxReflectionSample`) are Rust functions that inline their
//! body, `toVar()`s included, at every call, as three's do. The WGSL is gated
//! against three's dump of `tools/dump-pages/specular_helpers.html` in
//! `tests/nodes_display_wgsl.rs` (`docs/nodes.md` §85).

use std::f64::consts::PI;
use std::rc::Rc;

use super::{
    block, call, cross, dot, equirect_uv, float, if_then, log, max, mix, pi, reflect, shader_fn,
    sqrt, struct_new, struct_type, to_var, vec3, vec3_join, StructLayout, StructMember,
};
use crate::nodes::node::{FnDef, Lazy, NodeRef, Type};

/// `ENV_RAY_LENGTH` — the sentinel ray length the SSR pass writes for an
/// environment miss (no screen-space hit), far above any real hit distance so
/// one magnitude test separates misses from hits.
pub const ENV_RAY_LENGTH: f64 = 1e4;

/// `ENV_RAY_LENGTH_THRESHOLD` — above this a ray length is an environment
/// miss ([`ENV_RAY_LENGTH`]), below it a real hit; an order of magnitude under
/// the sentinel, so fp16 storage and bilinear blending do not misclassify it.
pub const ENV_RAY_LENGTH_THRESHOLD: f64 = 1e3;

/// `SampleGGXVNDF( V, ax, ay, r1, r2 )` — the bounded-VNDF sampler (Eto &
/// Tokuyoshi 2023, spherical-cap form), with its layout: a real WGSL `fn`.
#[inline(never)]
fn sample_ggx_vndf(v: NodeRef, ax: NodeRef, ay: NodeRef, r1: NodeRef, r2: NodeRef) -> NodeRef {
    thread_local! { static CELL: Lazy<Rc<FnDef>> = const { Lazy::new() }; }
    let def = CELL.with(|cell| {
        cell.get(|| {
            shader_fn(
                Some("SampleGGXVNDF"),
                vec![
                    ("V", Type::Vec3),
                    ("ax", Type::F32),
                    ("ay", Type::F32),
                    ("r1", Type::F32),
                    ("r2", Type::F32),
                ],
                Type::Vec3,
                sample_ggx_vndf_body,
            )
        })
    });
    call(&def, vec![v, ax, ay, r1, r2])
}

/// The body of [`sample_ggx_vndf`].
#[inline(never)]
fn sample_ggx_vndf_body(args: &[NodeRef]) -> NodeRef {
    let (v, ax, ay, r1, r2) = (
        args[0].clone(),
        args[1].clone(),
        args[2].clone(),
        args[3].clone(),
        args[4].clone(),
    );
    // Warp the view direction to the hemisphere configuration.
    let wi_std = to_var(
        None,
        vec3_join(vec![ax.mul(v.x()), ay.mul(v.y()), v.z()]).normalize(),
    );
    // Isotropic bound on the spherical cap.
    let a = to_var(None, ax.min(ay.clone()));
    let s = to_var(None, float(1.0).add(v.xy().length()));
    let a2 = to_var(None, a.mul(a.clone()));
    let s2 = to_var(None, s.mul(s.clone()));
    let k = to_var(
        None,
        a2.one_minus()
            .mul(s2.clone())
            .div(s2.add(a2.mul(v.z()).mul(v.z()))),
    );
    let b = to_var(None, wi_std.z().mul(k.clone()));
    // Sample the bounded spherical cap.
    let phi = to_var(None, float(6.283185307179586).mul(r1));
    let z = to_var(
        None,
        r2.one_minus().mul(float(1.0).add(b.clone())).sub(b.clone()),
    );
    let sin_theta = to_var(
        None,
        sqrt(max(float(0.0), float(1.0).sub(z.mul(z.clone())))),
    );
    let c = to_var(
        None,
        vec3_join(vec![
            sin_theta.mul(phi.cos()),
            sin_theta.mul(phi.sin()),
            z.clone(),
        ]),
    );
    // Microfacet normal in the standard configuration, warped back.
    let wm_std = to_var(None, c.add(wi_std.clone()));
    let ne = to_var(
        None,
        vec3_join(vec![
            ax.mul(wm_std.x()),
            ay.mul(wm_std.y()),
            max(float(0.0), wm_std.z()),
        ])
        .normalize(),
    );
    block(
        vec![
            wi_std,
            a,
            s,
            a2,
            s2,
            k,
            b,
            phi,
            z,
            sin_theta,
            c,
            wm_std,
            ne.clone(),
        ],
        ne,
    )
}

/// `D_GTR( roughness, NoH, k )` — the generalized Trowbridge-Reitz
/// distribution, `α² / ( π · ( NoH² ( α² − 1 ) + 1 )^k )`; `k = 2` is GGX.
/// `roughness` is α, not α². A plain `Fn` in three, so it inlines.
pub fn d_gtr(roughness: NodeRef, no_h: NodeRef, k: NodeRef) -> NodeRef {
    let a2 = to_var(None, roughness.mul(roughness.clone()));
    let no_h2 = to_var(None, no_h.mul(no_h.clone()));
    let base = to_var(None, no_h2.mul(a2.sub(float(1.0))).add(float(1.0)));
    let d = to_var(None, a2.div(pi().mul(base.pow(k))));
    block(vec![a2, no_h2, base, d.clone()], d)
}

/// `SmithG( NDotX, alpha )` — Heitz's Smith G1, `2·NX / ( NX + √( α² + ( 1 −
/// α² ) NX² ) )`. `alpha` is not squared; the function squares it. A plain
/// `Fn`, so it inlines.
pub fn smith_g(n_dot_x: NodeRef, alpha: NodeRef) -> NodeRef {
    let a2 = to_var(None, alpha.mul(alpha.clone()));
    let n_dot_x2 = to_var(None, n_dot_x.mul(n_dot_x.clone()));
    let g = float(2.0)
        .mul(n_dot_x.clone())
        .div(n_dot_x.add(sqrt(a2.add(a2.one_minus().mul(n_dot_x2.clone())))));
    block(vec![a2, n_dot_x2], g)
}

/// `GeometryTerm( NoL, NoV, alphaG )` — `G1( N·V ) · G1( N·L )` with
/// [`smith_g`]; `alphaG` is not squared. A plain `Fn`, so it inlines.
pub fn geometry_term(no_l: NodeRef, no_v: NodeRef, alpha_g: NodeRef) -> NodeRef {
    let g1v = to_var(None, smith_g(no_v, alpha_g.clone()));
    let g1l = to_var(None, smith_g(no_l, alpha_g));
    let g = to_var(None, g1v.mul(g1l.clone()));
    block(vec![g1v, g1l, g.clone()], g)
}

/// `GGXVNDFPdf( NoH, NoV, roughness )` — the bounded-VNDF direction PDF that
/// matches [`sample_ggx_vndf`] (Eto & Tokuyoshi eq. 8). Not exported by three;
/// a plain `Fn`, so it inlines.
#[inline(never)]
fn ggx_vndf_pdf(no_h: NodeRef, no_v: NodeRef, roughness: NodeRef) -> NodeRef {
    let d = to_var(None, d_gtr(roughness.clone(), no_h, float(2.0)));
    let a2 = to_var(None, roughness.mul(roughness.clone()));
    let sin_v2 = to_var(
        None,
        max(float(0.0), float(1.0).sub(no_v.mul(no_v.clone()))),
    );
    let s = to_var(None, float(1.0).add(sqrt(sin_v2.clone())));
    let s2 = to_var(None, s.mul(s.clone()));
    let k = to_var(
        None,
        float(1.0)
            .sub(a2.clone())
            .mul(s2.clone())
            .div(s2.add(a2.mul(no_v.clone()).mul(no_v.clone()))),
    );
    let t = to_var(
        None,
        sqrt(a2.mul(sin_v2.clone()).add(no_v.mul(no_v.clone()))),
    );
    let pdf = to_var(
        None,
        d.div(max(float(1e-6), float(2.0).mul(k.mul(no_v).add(t.clone())))),
    );
    block(vec![d, a2, sin_v2, s, s2, k, t, pdf.clone()], pdf)
}

/// `F_Schlick( f0, theta )` — Schlick's Fresnel, `f0 + ( 1 − f0 ) ( 1 − θ )⁵`
/// with `theta` the cosine (`V·H`). Returns a `vec3`. A plain `Fn`, so it
/// inlines.
pub fn f_schlick(f0: NodeRef, theta: NodeRef) -> NodeRef {
    let one_minus = to_var(None, float(1.0).sub(theta));
    let one_minus2 = to_var(None, one_minus.mul(one_minus.clone()));
    let one_minus5 = to_var(
        None,
        one_minus2.mul(one_minus2.clone()).mul(one_minus.clone()),
    );
    let f = to_var(
        None,
        f0.add(vec3(1.0, 1.0, 1.0).sub(f0.clone()).mul(one_minus5.clone())),
    );
    block(vec![one_minus, one_minus2, one_minus5, f.clone()], f)
}

/// `getSpecularDominantFactor( NoV, roughness )` — the specular dominant
/// factor for parallax-corrected ray lengths, from NRD's REBLUR. Three gives
/// it a layout, so it is a real WGSL `fn`.
pub fn get_specular_dominant_factor(no_v: NodeRef, roughness: NodeRef) -> NodeRef {
    thread_local! { static CELL: Lazy<Rc<FnDef>> = const { Lazy::new() }; }
    let def = CELL.with(|cell| {
        cell.get(|| {
            shader_fn(
                Some("getSpecularDominantFactor"),
                vec![("NoV", Type::F32), ("roughness", Type::F32)],
                Type::F32,
                |args| {
                    let (no_v, roughness) = (args[0].clone(), args[1].clone());
                    let a =
                        float(0.298475).mul(log(float(39.4115).sub(float(39.0029).mul(roughness))));
                    let f = float(1.0)
                        .sub(no_v)
                        .pow(float(10.8649))
                        .mul(float(1.0).sub(a.clone()))
                        .add(a);
                    f.clamp(float(0.0), float(1.0))
                },
            )
        })
    });
    call(&def, vec![no_v, roughness])
}

/// `ggxReflectionStruct` — the value [`ggx_reflection_sample`] returns:
/// `reflectDir` (vec3, the view-space reflected ray), `sampleWeight` (vec3,
/// the chromatic Monte-Carlo weight), `pdf` (float, the VNDF direction pdf),
/// `NdotV`, `alpha` (the clamped GGX α) and `f0` (vec3). Read a member with
/// [`struct_get`](super::struct_get).
///
/// Three's `struct( { … } )` has no name, so the builder calls it
/// `StructType${ n }` by order of creation; on a page where it is the only
/// struct (and in `SSRNode`'s shader) that is `StructType0`.
pub fn ggx_reflection_struct() -> Rc<StructLayout> {
    thread_local! { static CELL: Lazy<Rc<StructLayout>> = const { Lazy::new() }; }
    CELL.with(|cell| {
        cell.get(|| {
            struct_type(
                "StructType0",
                vec![
                    StructMember::new("reflectDir", Type::Vec3),
                    StructMember::new("sampleWeight", Type::Vec3),
                    StructMember::new("pdf", Type::F32),
                    StructMember::new("NdotV", Type::F32),
                    StructMember::new("alpha", Type::F32),
                    StructMember::new("f0", Type::Vec3),
                ],
            )
        })
    })
}

/// `ggxReflectionSample( N, V, roughness, metalness, albedo, Xi )` —
/// importance-samples the GGX/VNDF specular lobe for one pixel: `N` and `V`
/// are the view-space normal and surface→camera direction, `roughness` the
/// perceptual roughness, `albedo` tints the metal Fresnel `f0`, and `Xi.xy`
/// are the random numbers. Returns a [`ggx_reflection_struct`] value; a plain
/// `Fn` in three, so its body inlines at the call.
#[inline(never)]
pub fn ggx_reflection_sample(
    n: NodeRef,
    v: NodeRef,
    roughness: NodeRef,
    metalness: NodeRef,
    albedo: NodeRef,
    xi: NodeRef,
) -> NodeRef {
    let mut statements = Vec::new();

    // GGX alpha (r², clamped away from a degenerate lobe).
    let a = to_var(None, roughness.mul(roughness.clone()).max(float(0.001)));
    let ax = to_var(None, a.clone());
    let ay = to_var(None, a.clone());
    statements.extend([a, ax.clone(), ay.clone()]);

    // TBN from the view-space normal.
    let up = vec3(0.0, 0.0, 1.0);
    let t0 = to_var(None, cross(up, n.clone()));
    let t = to_var(None, t0.normalize());
    let fallback = if_then(
        t.length().less_than(float(1e-3)),
        vec![t.assign(cross(vec3(0.0, 1.0, 0.0), n.clone()).normalize())],
    );
    let b = to_var(None, cross(n.clone(), t.clone()).normalize());
    statements.extend([t0, t.clone(), fallback, b.clone()]);

    // V in the local frame (N = +Z), and a VNDF sample there.
    let v_local = to_var(
        None,
        vec3_join(vec![
            dot(t.clone(), v.clone()),
            dot(b.clone(), v.clone()),
            dot(n.clone(), v.clone()),
        ]),
    );
    let h_local = to_var(
        None,
        sample_ggx_vndf(v_local.clone(), ax.clone(), ay, xi.x(), xi.y()),
    );
    let flip = if_then(
        h_local.z().less_than(float(0.0)),
        vec![h_local.assign(h_local.negate())],
    );
    statements.extend([v_local, h_local.clone(), flip]);

    // H back to view space, and the reflected ray.
    let h = to_var(
        None,
        t.mul(h_local.x())
            .add(b.mul(h_local.y()))
            .add(n.mul(h_local.z()))
            .normalize(),
    );
    let view_reflect_dir = to_var(None, reflect(v.negate(), h.clone()).normalize());
    let l = to_var(None, view_reflect_dir.clone());
    let hh = to_var(None, v.add(l.clone()).normalize());
    statements.extend([h, view_reflect_dir.clone(), l.clone(), hh.clone()]);

    let n_dot_v = to_var(None, max(float(0.0), dot(n.clone(), v.clone())));
    let n_dot_l = to_var(None, max(float(0.0), dot(n.clone(), l)));
    let n_dot_h = to_var(None, max(float(0.0), dot(n, hh.clone())));
    let v_dot_h = to_var(None, max(float(0.0), dot(v, hh)));
    let f0 = to_var(None, mix(vec3(0.04, 0.04, 0.04), albedo, metalness));
    let fresnel_weight = to_var(None, f_schlick(f0.clone(), v_dot_h.clone()));
    let pdf = to_var(
        None,
        ggx_vndf_pdf(n_dot_h.clone(), n_dot_v.clone(), ax.clone()),
    );
    statements.extend([
        n_dot_v.clone(),
        n_dot_l.clone(),
        n_dot_h,
        v_dot_h,
        f0.clone(),
        fresnel_weight.clone(),
        pdf.clone(),
    ]);

    // The importance weight with GGX D cancelled analytically.
    let a2 = to_var(None, ax.mul(ax.clone()));
    let sin_v2 = to_var(
        None,
        n_dot_v.mul(n_dot_v.clone()).one_minus().max(float(0.0)),
    );
    let s_b = to_var(None, float(1.0).add(sqrt(sin_v2.clone())));
    let s2_b = to_var(None, s_b.mul(s_b.clone()));
    let k_b = to_var(
        None,
        a2.one_minus()
            .mul(s2_b.clone())
            .div(s2_b.add(a2.mul(n_dot_v.clone()).mul(n_dot_v.clone()))),
    );
    let t_b = to_var(
        None,
        sqrt(a2.mul(sin_v2.clone()).add(n_dot_v.mul(n_dot_v.clone()))),
    );
    let glossy_weight = to_var(
        None,
        fresnel_weight
            .mul(geometry_term(n_dot_l, n_dot_v.clone(), ax.clone()))
            .mul(k_b.mul(n_dot_v.clone()).add(t_b.clone()))
            .div(float(2.0).mul(n_dot_v.clone()).max(float(1e-4))),
    );
    statements.extend([a2, sin_v2, s_b, s2_b, k_b, t_b, glossy_weight.clone()]);

    block(
        statements,
        struct_new(
            &ggx_reflection_struct(),
            vec![view_reflect_dir, glossy_weight, pdf, n_dot_v, ax, f0],
        ),
    )
}

/// `equirectUvToDir( uv )` — the direction an equirectangular UV maps to.
/// Three gives it a layout, so it is a real WGSL `fn`.
pub fn equirect_uv_to_dir(uv: NodeRef) -> NodeRef {
    thread_local! { static CELL: Lazy<Rc<FnDef>> = const { Lazy::new() }; }
    let def = CELL.with(|cell| {
        cell.get(|| {
            shader_fn(
                Some("equirectUvToDir"),
                vec![("uv", Type::Vec2)],
                Type::Vec3,
                |args| {
                    let uv = args[0].clone();
                    let phi = uv.x().mul(PI * 2.0).sub(PI);
                    let lat = uv.y().sub(0.5).mul(PI);
                    let cos_lat = lat.cos();
                    vec3_join(vec![
                        cos_lat.mul(phi.cos()),
                        lat.sin(),
                        cos_lat.mul(phi.sin()),
                    ])
                    .normalize()
                },
            )
        })
    });
    call(&def, vec![uv])
}

/// `equirectDirPdf( direction )` — the solid-angle PDF of a direction under
/// the equirectangular parameterization, `1 / ( 2π² sin θ )` (0 at the
/// poles). Three gives it a layout, so it is a real WGSL `fn`.
pub fn equirect_dir_pdf(direction: NodeRef) -> NodeRef {
    thread_local! { static CELL: Lazy<Rc<FnDef>> = const { Lazy::new() }; }
    let def = CELL.with(|cell| {
        cell.get(|| {
            shader_fn(
                Some("equirectDirPdf"),
                vec![("direction", Type::Vec3)],
                Type::F32,
                |args| {
                    let uv_dir = equirect_uv(args[0].clone());
                    let sin_theta = uv_dir.y().mul(PI).sin();
                    sin_theta.abs().less_than(float(1e-6)).select(
                        float(0.0),
                        float(1.0).div(float(2.0 * PI * PI).mul(sin_theta)),
                    )
                },
            )
        })
    });
    call(&def, vec![direction])
}

/// `misPowerHeuristic( pdfA, pdfB )` — Veach's power heuristic with β = 2,
/// `pdfA² / ( pdfA² + pdfB² )`. Three gives it a layout, so it is a real WGSL
/// `fn`.
pub fn mis_power_heuristic(pdf_a: NodeRef, pdf_b: NodeRef) -> NodeRef {
    thread_local! { static CELL: Lazy<Rc<FnDef>> = const { Lazy::new() }; }
    let def = CELL.with(|cell| {
        cell.get(|| {
            shader_fn(
                Some("misPowerHeuristic"),
                vec![("pdfA", Type::F32), ("pdfB", Type::F32)],
                Type::F32,
                |args| {
                    let (pdf_a, pdf_b) = (args[0].clone(), args[1].clone());
                    let pdf_a_sq = pdf_a.mul(pdf_a.clone());
                    let pdf_b_sq = pdf_b.mul(pdf_b.clone());
                    pdf_a_sq.div(pdf_a_sq.add(pdf_b_sq))
                },
            )
        })
    });
    call(&def, vec![pdf_a, pdf_b])
}
