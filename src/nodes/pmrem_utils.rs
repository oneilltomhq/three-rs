//! Port of `three.js/src/nodes/pmrem/PMREMUtils.js` — the shader half of
//! PMREM.
//!
//! Three lays the prefiltered environment out as *tiles inside one 2-D
//! texture* rather than as a cube mip chain, so every read is hand-addressed:
//! [`get_face`] picks the cube face, [`get_uv`] projects the direction onto it,
//! and `bilinear_cube_uv` turns the pair into a texel of the atlas. The
//! arithmetic is unforgiving and silent — a wrong tile offset gives a
//! plausible, wrong reflection — so this file follows the JS statement for
//! statement and the generated WGSL is diffed against Three's own dump
//! (`examples/dump_wgsl`, the `pmrem_*` sections).
//!
//! The constants below must match `PMREMGenerator`'s; see
//! [`crate::renderer::pmrem`].

use std::rc::Rc;

use crate::nodes::node::{FnDef, Lazy, Type};
use crate::nodes::tsl::*;
use crate::nodes::NodeRef;
use crate::textures::Texture;

/// `GOLDEN_ANGLE` — the spiral step of the Gaussian blur kernel.
pub const GOLDEN_ANGLE: f64 = 2.399963229728653;

/// `LOD_MIN` as the shader sees it: below this mip the tiles stop shrinking
/// and move sideways into the extra-LOD columns instead.
const CUBE_UV_MIN_MIP_LEVEL: f64 = 4.0;
/// The width of one of those columns' faces.
const CUBE_UV_MIN_TILE_SIZE: f64 = 16.0;

/// The five `( roughness, mip )` knots of the piecewise `roughnessToMip`
/// curve. Kept as separate constants rather than folded, because three emits
/// them unfolded: `( ( 1.0 - roughness ) * ( -1.0 - -2.0 ) ) / ( 1.0 - 0.8 )`.
const CUBE_UV_R0: f64 = 1.0;
const CUBE_UV_M0: f64 = -2.0;
const CUBE_UV_R1: f64 = 0.8;
const CUBE_UV_M1: f64 = -1.0;
const CUBE_UV_R4: f64 = 0.4;
const CUBE_UV_M4: f64 = 2.0;
const CUBE_UV_R5: f64 = 0.305;
const CUBE_UV_M5: f64 = 3.0;
const CUBE_UV_R6: f64 = 0.21;
const CUBE_UV_M6: f64 = 4.0;

/// `roughnessToMip()` on the CPU — the same five branches, for the tests that
/// gate the curve against numbers dumped from three.js.
pub fn roughness_to_mip_value(roughness: f64) -> f64 {
    if roughness >= CUBE_UV_R1 {
        (CUBE_UV_R0 - roughness) * (CUBE_UV_M1 - CUBE_UV_M0) / (CUBE_UV_R0 - CUBE_UV_R1)
            + CUBE_UV_M0
    } else if roughness >= CUBE_UV_R4 {
        (CUBE_UV_R1 - roughness) * (CUBE_UV_M4 - CUBE_UV_M1) / (CUBE_UV_R1 - CUBE_UV_R4)
            + CUBE_UV_M1
    } else if roughness >= CUBE_UV_R5 {
        (CUBE_UV_R4 - roughness) * (CUBE_UV_M5 - CUBE_UV_M4) / (CUBE_UV_R4 - CUBE_UV_R5)
            + CUBE_UV_M4
    } else if roughness >= CUBE_UV_R6 {
        (CUBE_UV_R5 - roughness) * (CUBE_UV_M6 - CUBE_UV_M5) / (CUBE_UV_R5 - CUBE_UV_R6)
            + CUBE_UV_M5
    } else {
        -2.0 * (1.16 * roughness).log2()
    }
}

/// `getFace( direction )` — the 0–5 face index of a direction, in PMREM's own
/// face order (`+x +y +z -x -y -z`).
///
/// Emitted as a real `fn`: the JS has `.setLayout( { name: 'getFace', … } )`.
pub fn get_face(direction: NodeRef) -> NodeRef {
    thread_local! { static CELL: Lazy<Rc<FnDef>> = const { Lazy::new() }; }
    let def = CELL.with(|c| {
        c.get(|| {
            shader_fn(
                Some("getFace"),
                vec![("direction", Type::Vec3)],
                Type::F32,
                |args| {
                    let d = args[0].clone();
                    let abs_direction = to_var(None, abs(d.clone()));
                    let face = to_var(None, float(-1.0));

                    // `select( cond, a, b )` is `ConditionalNode`, which the
                    // builder lowers to an `if`/`else` over a result var —
                    // which is where the dump's four extra `nodeVar`s in this
                    // function come from.
                    let pick = |cond: NodeRef, a: f64, b: f64| cond.select(float(a), float(b));

                    block(
                        vec![
                            abs_direction.clone(),
                            face.clone(),
                            if_else(
                                abs_direction.x().greater_than(abs_direction.z()),
                                vec![if_else(
                                    abs_direction.x().greater_than(abs_direction.y()),
                                    vec![face.assign(pick(
                                        d.x().greater_than(float(0.0)),
                                        0.0,
                                        3.0,
                                    ))],
                                    vec![face.assign(pick(
                                        d.y().greater_than(float(0.0)),
                                        1.0,
                                        4.0,
                                    ))],
                                )],
                                vec![if_else(
                                    abs_direction.z().greater_than(abs_direction.y()),
                                    vec![face.assign(pick(
                                        d.z().greater_than(float(0.0)),
                                        2.0,
                                        5.0,
                                    ))],
                                    vec![face.assign(pick(
                                        d.y().greater_than(float(0.0)),
                                        1.0,
                                        4.0,
                                    ))],
                                )],
                            ),
                        ],
                        face,
                    )
                },
            )
        })
    });
    call(&def, vec![direction])
}

/// `getUV( direction, face )` — the direction's position on its face, in
/// `[ 0, 1 ]`. RH coordinate system, PMREM's face-indexing convention.
pub fn get_uv(direction: NodeRef, face: NodeRef) -> NodeRef {
    thread_local! { static CELL: Lazy<Rc<FnDef>> = const { Lazy::new() }; }
    let def = CELL.with(|c| {
        c.get(|| {
            shader_fn(
                Some("getUV"),
                vec![("direction", Type::Vec3), ("face", Type::F32)],
                Type::Vec2,
                |args| {
                    let (d, face) = (args[0].clone(), args[1].clone());
                    // `vec2().toVar()` — TSL's no-argument `vec2()` is the
                    // zero vector, which the dump declares in full.
                    let uv = to_var(None, vec2(0.0, 0.0));

                    let arm = |a: NodeRef, b: NodeRef, denom: NodeRef| {
                        vec![uv.assign(vec2_join(vec![a, b]).div(abs(denom)))]
                    };

                    // `ElseIf` is `Else( () => If( … ) )`, so the chain is
                    // right-nested and is built from the inside out.
                    let neg_y = if_else(
                        face.equal(float(4.0)),
                        arm(d.x().negate(), d.z(), d.y()),
                        arm(d.x(), d.y(), d.z()),
                    );
                    let neg_x = if_else(
                        face.equal(float(3.0)),
                        arm(d.z().negate(), d.y(), d.x()),
                        vec![neg_y],
                    );
                    let pos_z = if_else(
                        face.equal(float(2.0)),
                        arm(d.x().negate(), d.y(), d.z()),
                        vec![neg_x],
                    );
                    let pos_y = if_else(
                        face.equal(float(1.0)),
                        arm(d.x().negate(), d.z().negate(), d.y()),
                        vec![pos_z],
                    );
                    let pos_x = if_else(
                        face.equal(float(0.0)),
                        arm(d.z(), d.y(), d.x()),
                        vec![pos_y],
                    );

                    block(
                        vec![uv.clone(), pos_x],
                        // `mul( 0.5, uv.add( 1.0 ) )` — the scalar first, so
                        // the dump reads `( vec2<f32>( 0.5 ) * ( … ) )`.
                        float(0.5).mul(uv.add(float(1.0))),
                    )
                },
            )
        })
    });
    call(&def, vec![direction, face])
}

/// `roughnessToMip( roughness )` — the piecewise curve that maps a material
/// roughness onto a cubeUV mip. Four linear segments between the knots and a
/// `-2 * log2( 1.16 * roughness )` tail below `0.21`.
pub fn roughness_to_mip(roughness: NodeRef) -> NodeRef {
    thread_local! { static CELL: Lazy<Rc<FnDef>> = const { Lazy::new() }; }
    let def = CELL.with(|c| {
        c.get(|| {
            shader_fn(
                Some("roughnessToMip"),
                vec![("roughness", Type::F32)],
                Type::F32,
                |args| {
                    let roughness = args[0].clone();
                    let mip = to_var(None, float(0.0));

                    // `( r_hi - roughness ) * ( m_lo - m_hi ) / ( r_hi - r_lo )
                    // + m_hi`, spelt in that order because three does.
                    let segment = |r_hi: f64, r_lo: f64, m_hi: f64, m_lo: f64| {
                        vec![mip.assign(
                            float(r_hi)
                                .sub(roughness.clone())
                                .mul(float(m_lo).sub(float(m_hi)))
                                .div(float(r_hi).sub(float(r_lo)))
                                .add(float(m_hi)),
                        )]
                    };

                    // 1.16 = 1.79^0.25.
                    let tail =
                        vec![mip.assign(float(-2.0).mul(log2(float(1.16).mul(roughness.clone()))))];

                    let level3 = if_else(
                        roughness.greater_than_equal(float(CUBE_UV_R6)),
                        segment(CUBE_UV_R5, CUBE_UV_R6, CUBE_UV_M5, CUBE_UV_M6),
                        tail,
                    );
                    let level2 = if_else(
                        roughness.greater_than_equal(float(CUBE_UV_R5)),
                        segment(CUBE_UV_R4, CUBE_UV_R5, CUBE_UV_M4, CUBE_UV_M5),
                        vec![level3],
                    );
                    let level1 = if_else(
                        roughness.greater_than_equal(float(CUBE_UV_R4)),
                        segment(CUBE_UV_R1, CUBE_UV_R4, CUBE_UV_M1, CUBE_UV_M4),
                        vec![level2],
                    );
                    let level0 = if_else(
                        roughness.greater_than_equal(float(CUBE_UV_R1)),
                        segment(CUBE_UV_R0, CUBE_UV_R1, CUBE_UV_M0, CUBE_UV_M1),
                        vec![level1],
                    );

                    block(vec![mip.clone(), level0], mip)
                },
            )
        })
    });
    call(&def, vec![roughness])
}

/// The cubeUV atlas' three shape numbers, as the shader reads them. In the
/// generator's own materials they are baked constants (`float( 1 / width )`);
/// in `PMREMNode` they are uniforms, so each is a node rather than a number.
#[derive(Clone)]
pub struct CubeUvSize {
    /// `CUBEUV_TEXEL_WIDTH` — `1 / atlasWidth`.
    pub texel_width: NodeRef,
    /// `CUBEUV_TEXEL_HEIGHT` — `1 / atlasHeight`.
    pub texel_height: NodeRef,
    /// `CUBEUV_MAX_MIP` — `log2( cubeSize )`.
    pub max_mip: NodeRef,
}

/// `bilinearCubeUV( envMap, direction, mipInt, … )` — one tap of the atlas.
///
/// Inlined, as the JS `Fn()` has no layout. The name is historical: r186 reads
/// the atlas with `textureSampleGrad` and a *nearest* sampler and does no
/// manual bilinear filtering at all — the "bilinear" is the GPU's, and the
/// zero gradients only turn anisotropy off.
fn bilinear_cube_uv(
    env_map: &Texture,
    direction: NodeRef,
    mip_int_immutable: NodeRef,
    size: &CubeUvSize,
) -> NodeRef {
    let mip_int = to_var(None, mip_int_immutable);
    let face = to_var(None, get_face(direction.clone()));
    let filter_int = to_var(
        None,
        max(
            float(CUBE_UV_MIN_MIP_LEVEL).sub(mip_int.clone()),
            float(0.0),
        ),
    );
    let clamp_mip = mip_int.assign(max(mip_int.clone(), float(CUBE_UV_MIN_MIP_LEVEL)));
    let face_size = to_var(None, exp2(mip_int.clone()));
    // "UVs overshoot the face by one texel" — the `- 2` / `+ 1` is the border
    // the lod planes' geometry bakes into the directions.
    let uv = to_var(
        None,
        get_uv(direction, face.clone())
            .mul(face_size.sub(float(2.0)))
            .add(float(1.0)),
    );

    // The tap is a texture node, so the builder gives it a var of its own:
    // that is what puts `nodeVarN = textureSampleGrad( … )` on its own line.
    let sample = texture_grad(env_map, uv.clone());

    block(
        vec![
            mip_int,
            face.clone(),
            filter_int.clone(),
            clamp_mip,
            face_size.clone(),
            uv.clone(),
            // The bottom half of the atlas holds faces 3..5.
            if_then(
                face.greater_than(float(2.0)),
                vec![
                    uv.y().add_assign(face_size.clone()),
                    face.sub_assign(float(3.0)),
                ],
            ),
            uv.x().add_assign(face.mul(face_size.clone())),
            // The extra-LOD columns sit to the right of the main pyramid,
            // `3 * 16` texels apart.
            uv.x()
                .add_assign(filter_int.mul(float(3.0).mul(float(CUBE_UV_MIN_TILE_SIZE)))),
            uv.y()
                .add_assign(float(4.0).mul(exp2(size.max_mip.clone()).sub(face_size))),
            uv.x().mul_assign(size.texel_width.clone()),
            uv.y().mul_assign(size.texel_height.clone()),
        ],
        sample,
    )
}

/// `textureCubeUV( envMap, sampleDir, roughness, … )` — the read a material
/// makes. Two taps and a `mix` when the mip is fractional, one when it is not.
pub fn texture_cube_uv(
    env_map: &Texture,
    sample_dir: NodeRef,
    roughness: NodeRef,
    size: &CubeUvSize,
) -> NodeRef {
    let mip = roughness_to_mip(roughness).clamp(float(CUBE_UV_M0), size.max_mip.clone());
    let mip_f = fract(mip.clone());
    let mip_int = floor(mip);
    let color0 = to_var(
        None,
        bilinear_cube_uv(env_map, sample_dir.clone(), mip_int.clone(), size),
    );
    let color1 = to_var(
        None,
        bilinear_cube_uv(env_map, sample_dir, mip_int.add(float(1.0)), size),
    );

    block(
        vec![
            color0.clone(),
            if_then(
                mip_f.not_equal(float(0.0)),
                vec![
                    color1.clone(),
                    color0.assign(mix(color0.clone(), color1, mip_f)),
                ],
            ),
        ],
        color0,
    )
}

/// `ggxConvolution( { roughness, mipInt, envMap, N, GGX_SAMPLES, … } )` — the
/// prefilter kernel, GGX VNDF importance sampling (Heitz 2018).
///
/// This is the fragment of `PMREM_ggx`, the material the 20 blur passes share.
pub fn ggx_convolution(
    roughness: NodeRef,
    mip_int: NodeRef,
    env_map: &Texture,
    n_immutable: NodeRef,
    ggx_samples: usize,
    size: &CubeUvSize,
) -> NodeRef {
    let n = to_var(None, n_immutable);
    let prefiltered_color = to_var(None, vec3(0.0, 0.0, 0.0));
    let total_weight = to_var(None, float(0.0));

    // For very low roughness, sample the environment directly.
    let direct =
        prefiltered_color.assign(bilinear_cube_uv(env_map, n.clone(), mip_int.clone(), size));

    // Tangent-space basis for VNDF sampling.
    let up = abs(n.z())
        .less_than(float(0.999))
        .select(vec3(0.0, 0.0, 1.0), vec3(1.0, 0.0, 0.0));
    let tangent = to_var(None, cross(up, n.clone()).normalize());
    let bitangent = to_var(None, cross(n.clone(), tangent.clone()));

    let loop_body = {
        let (n, tangent, bitangent, prefiltered_color, total_weight, mip_int) = (
            n.clone(),
            tangent.clone(),
            bitangent.clone(),
            prefiltered_color.clone(),
            total_weight.clone(),
            mip_int.clone(),
        );
        let (env_map, size, roughness) = (env_map.clone(), size.clone(), roughness.clone());
        move |i: &NodeRef| {
            let xi = hammersley(i.clone(), ggx_samples);
            // For PMREM, V = N, so in tangent space V is always ( 0, 0, 1 ).
            // `to_var` where three writes none: the call's result is read
            // three times (`.x`, `.y`, `.z`), and three's builder turns any
            // node used more than once into a var — which is how the dump gets
            // its `nodeVar16 = normalize( vec3<f32>( … ) );`. The port applies
            // that rule to expression nodes but not to an inlined `Fn` body,
            // so the var is asked for here instead. Without it the whole VNDF
            // body is re-emitted per component.
            let h_tangent = to_var(
                None,
                importance_sample_ggx_vndf(xi, vec3(0.0, 0.0, 1.0), roughness.clone()),
            );

            let h = tangent
                .mul(h_tangent.x())
                .add(bitangent.mul(h_tangent.y()))
                .add(n.mul(h_tangent.z()))
                .normalize();
            let l = h
                .mul(dot(n.clone(), h.clone()).mul(float(2.0)))
                .sub(n.clone())
                .normalize();
            let n_dot_l = to_var(None, max(dot(n.clone(), l.clone()), float(0.0)));

            vec![
                n_dot_l.clone(),
                if_then(
                    n_dot_l.greater_than(float(0.0)),
                    vec![
                        // The sample is a `vec4` and the accumulator a `vec3`:
                        // three's `addAssign` widens the sum and swizzles the
                        // store, so the dump reads
                        // `( vec4<f32>( color, 1.0 ) + … ).xyz`.
                        prefiltered_color.add_assign(
                            bilinear_cube_uv(&env_map, l, mip_int.clone(), &size)
                                .mul(n_dot_l.clone()),
                        ),
                        total_weight.add_assign(n_dot_l),
                    ],
                ),
            ]
        }
    };

    let importance = vec![
        tangent,
        bitangent,
        // `Loop( { start: uint( 0 ), end: GGX_SAMPLES } )` still emits an
        // `i32` index and an unsuffixed bound — `for ( var i : i32 = 0; i <
        // 256; i ++ )` — which is why the body casts with `u32( i )`.
        loop_n("i", int(ggx_samples as i64), loop_body),
        if_then(
            total_weight.greater_than(float(0.0)),
            vec![prefiltered_color.assign(prefiltered_color.div(total_weight.clone()))],
        ),
    ];

    block(
        vec![
            n,
            prefiltered_color.clone(),
            total_weight,
            if_else(roughness.less_than(float(0.001)), vec![direct], importance),
        ],
        vec4_join(vec![prefiltered_color, float(1.0)]),
    )
}

/// `radicalInverse_VdC( bits )` — the Van der Corput radical inverse: a bit
/// reversal and a scale by `2^-32`.
fn radical_inverse_vdc(bits_immutable: NodeRef) -> NodeRef {
    let bits = to_var(None, bits_immutable.to(Type::U32));
    let swap = |shift: u32, mask: u32| {
        bits.assign(
            bits.bit_and(uint(mask))
                .shift_left(uint(shift))
                .bit_or(bits.bit_and(uint(!mask)).shift_right(uint(shift))),
        )
    };
    block(
        vec![
            bits.clone(),
            bits.assign(bits.shift_left(uint(16)).bit_or(bits.shift_right(uint(16)))),
            swap(1, 0x5555_5555),
            swap(2, 0x3333_3333),
            swap(4, 0x0F0F_0F0F),
            swap(8, 0x00FF_00FF),
        ],
        // 1 / 0x100000000.
        bits.to(Type::F32).mul(float(2.328_306_436_538_696_3e-10)),
    )
}

/// `hammersley( i, N )` — the quasi-random pair the GGX loop samples with.
fn hammersley(i: NodeRef, n: usize) -> NodeRef {
    vec2_join(vec![
        i.to(Type::F32).div(float(n as f64)),
        radical_inverse_vdc(i),
    ])
}

/// `importanceSampleGGX_VNDF( Xi, V, roughness )` — "Sampling the GGX
/// Distribution of Visible Normals", Heitz 2018,
/// <https://jcgt.org/published/0007/04/01/>.
fn importance_sample_ggx_vndf(xi: NodeRef, v: NodeRef, roughness: NodeRef) -> NodeRef {
    let alpha = to_const(None, roughness.mul(roughness.clone()));

    // Section 4.1: orthonormal basis.
    let t1 = to_const(None, vec3(1.0, 0.0, 0.0));
    let t2 = to_const(None, cross(v.clone(), t1.clone()));

    // Section 4.2: parameterisation of projected area.
    let r = to_const(None, sqrt(xi.x()));
    // `mul( 2.0, 3.14159265359 )` — three's literal, not `PI`: it is one digit
    // short of `f64::consts::PI` and the difference is visible in the WGSL.
    #[allow(clippy::approx_constant)]
    let pi = float(3.141_592_653_59);
    let phi = to_const(None, float(2.0).mul(pi).mul(xi.y()));
    let t1s = to_const(None, r.mul(phi.cos()));
    let t2s = to_var(None, r.mul(phi.sin()));
    let s = to_const(None, float(0.5).mul(v.z().add(float(1.0))));
    let reproject = t2s.assign(
        s.one_minus()
            .mul(sqrt(t1s.mul(t1s.clone()).one_minus()))
            .add(s.mul(t2s.clone())),
    );

    // Section 4.3: reprojection onto the hemisphere.
    let nh = t1
        .mul(t1s.clone())
        .add(t2.mul(t2s.clone()))
        .add(v.mul(sqrt(max(
            float(0.0),
            t1s.mul(t1s.clone()).add(t2s.mul(t2s.clone())).one_minus(),
        ))));

    block(
        vec![alpha.clone(), t1, t2, r, phi, t1s, t2s, s, reproject],
        // Section 3.4: transform back to the ellipsoid configuration.
        vec3_join(vec![
            alpha.mul(nh.x()),
            alpha.mul(nh.y()),
            max(float(0.0), nh.z()),
        ])
        .normalize(),
    )
}

/// `sphericalGaussianBlur( { SAMPLES, sigma, outputDirection, mipInt, envMap,
/// … } )` — the scene-blur kernel `fromScene( scene, sigma > 0 )` uses.
///
/// Not reached by `fromCubemap`, and therefore not exercised by
/// `webgpu_pmrem_cubemap`; it is here because `PMREMGenerator._init()` builds
/// the blur material unconditionally, and because the shape is the one
/// [`ggx_convolution`] has.
pub fn spherical_gaussian_blur(
    samples: usize,
    sigma: NodeRef,
    output_direction: NodeRef,
    mip_int: NodeRef,
    env_map: &Texture,
    size: &CubeUvSize,
) -> NodeRef {
    let color = to_var(None, vec3(0.0, 0.0, 0.0));

    let direct = color.assign(bilinear_cube_uv(
        env_map,
        output_direction.clone(),
        mip_int.clone(),
        size,
    ));

    let up = abs(output_direction.z())
        .less_than(float(0.999))
        .select(vec3(0.0, 0.0, 1.0), vec3(1.0, 0.0, 0.0));
    let tangent = to_var(None, cross(up, output_direction.clone()).normalize());
    let bitangent = to_var(None, cross(output_direction.clone(), tangent.clone()));

    // Truncate the kernel at three standard deviations or at the antipode.
    let theta_max = min_of(sigma.mul(float(3.0)), float(std::f64::consts::PI));
    let truncation = to_var(
        None,
        exp(theta_max
            .mul(theta_max.clone())
            .mul(float(-0.5))
            .div(sigma.mul(sigma.clone())))
        .one_minus(),
    );
    let accum_weight = to_var(None, float(0.0));

    let loop_body = {
        let (color, accum_weight, truncation, tangent, bitangent, sigma) = (
            color.clone(),
            accum_weight.clone(),
            truncation.clone(),
            tangent.clone(),
            bitangent.clone(),
            sigma.clone(),
        );
        let (env_map, size, mip_int, output_direction) = (
            env_map.clone(),
            size.clone(),
            mip_int.clone(),
            output_direction.clone(),
        );
        move |i: &NodeRef| {
            // Stratified inverse-CDF sampling of the Gaussian, placed on a
            // golden-angle spiral.
            let stratum = i.to(Type::F32).add(float(0.5)).div(float(samples as f64));
            let theta = to_var(
                None,
                sigma.mul(sqrt(
                    log(stratum.mul(truncation.clone()).one_minus()).mul(float(-2.0)),
                )),
            );
            let phi = to_var(None, i.to(Type::F32).mul(float(GOLDEN_ANGLE)));

            let offset = tangent.mul(phi.cos()).add(bitangent.mul(phi.sin()));
            let sample_direction = output_direction
                .mul(theta.cos())
                .add(offset.mul(theta.sin()));

            // Correct the planar sample density to solid angle.
            let weight = to_var(None, theta.sin().div(theta.clone()));

            vec![
                theta,
                phi,
                weight.clone(),
                color.add_assign(
                    bilinear_cube_uv(&env_map, sample_direction, mip_int.clone(), &size)
                        .mul(weight.clone()),
                ),
                accum_weight.add_assign(weight),
            ]
        }
    };

    block(
        vec![
            color.clone(),
            if_else(
                sigma.equal(float(0.0)),
                vec![direct],
                vec![
                    tangent,
                    bitangent,
                    truncation,
                    accum_weight.clone(),
                    loop_n("i", int(samples as i64), loop_body),
                    color.assign(color.div(accum_weight)),
                ],
            ),
        ],
        vec4_join(vec![color, float(1.0)]),
    )
}
