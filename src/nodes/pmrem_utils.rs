//! Port of `three.js/src/nodes/pmrem/PMREMUtils.js` — the shader half of
//! PMREM.
//!
//! Since three.js 2f80402 a PMREM is a real cube texture with one mip level
//! per prefiltered roughness, so there is no atlas to address any more: every
//! read here is a `cubeTexture( envMap ).sample( dir ).level( lod )`, and the
//! only arithmetic left is which level a roughness lives at
//! ([`roughness_to_mip`]) and the three prefilter kernels
//! [`crate::renderer::pmrem`] renders with. The generated WGSL is diffed
//! against Three's own dump (`examples/dump_wgsl`, the `pmrem_*` sections).

use crate::nodes::node::Type;
use crate::nodes::tsl::*;
use crate::nodes::NodeRef;
use crate::textures::CubeTexture;

/// `GOLDEN_ANGLE` — the spiral step of the Gaussian blur kernel.
pub const GOLDEN_ANGLE: f64 = 2.399963229728653;

/// `roughnessToMip( roughness, maxLod )` — the mip level of a PMREM that was
/// prefiltered for `roughness`, the inverse of
/// [`lod_to_roughness`](crate::renderer::pmrem::lod_to_roughness):
/// `maxLod * r * ( 2 - r )` with `r` clamped to `[ 0, 1 ]` (Filament's
/// `perceptualRoughnessToLod()`).
pub fn roughness_to_mip(roughness: NodeRef, max_lod: NodeRef) -> NodeRef {
    let roughness = roughness.clamp(float(0.0), float(1.0));
    max_lod
        .mul(roughness.clone())
        .mul(float(2.0).sub(roughness))
}

/// The CPU twin of [`roughness_to_mip`], for tests.
pub fn roughness_to_mip_value(roughness: f64, max_lod: f64) -> f64 {
    let r = roughness.clamp(0.0, 1.0);
    max_lod * r * (2.0 - r)
}

/// `select( abs( N.z ).lessThan( 0.999 ), vec3( 0, 0, 1 ), vec3( 1, 0, 0 ) )`
/// and the tangent frame built on it — shared by the blur and the GGX kernel.
fn tangent_frame(n: &NodeRef) -> (NodeRef, NodeRef) {
    let up = abs(n.z())
        .less_than(float(0.999))
        .select(vec3(0.0, 0.0, 1.0), vec3(1.0, 0.0, 0.0));
    let tangent = to_var(None, cross(up, n.clone()).normalize());
    let bitangent = to_var(None, cross(n.clone(), tangent.clone()));
    (tangent, bitangent)
}

/// `sphericalGaussianBlur( { SAMPLES, sigma, direction, envMap } )` — a
/// Gaussian blur using stratified inverse-CDF samples on a golden-angle
/// spiral, every tap read at level 0 of the cube.
pub fn spherical_gaussian_blur(
    samples: usize,
    sigma: NodeRef,
    direction: NodeRef,
    env_map: &CubeTexture,
) -> NodeRef {
    let output_direction = to_var(None, direction);
    let (tangent, bitangent) = tangent_frame(&output_direction);

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

    let color = to_var(None, vec3(0.0, 0.0, 0.0));
    let accum_weight = to_var(None, float(0.0));

    let loop_body = {
        let (color, accum_weight, truncation, tangent, bitangent, sigma, output_direction) = (
            color.clone(),
            accum_weight.clone(),
            truncation.clone(),
            tangent.clone(),
            bitangent.clone(),
            sigma.clone(),
            output_direction.clone(),
        );
        let env_map = env_map.clone();
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
                    cube_texture_level(&env_map, sample_direction, float(0.0))
                        .xyz()
                        .mul(weight.clone()),
                ),
                accum_weight.add_assign(weight),
            ]
        }
    };

    block(
        vec![
            output_direction,
            tangent,
            bitangent,
            truncation,
            color.clone(),
            accum_weight.clone(),
            loop_n("i", int(samples as i64), loop_body),
        ],
        vec4_join(vec![color.div(accum_weight), float(1.0)]),
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

/// `hammersley( i, N )`.
fn hammersley(i: NodeRef, n: usize) -> NodeRef {
    vec2_join(vec![
        i.to(Type::F32).div(float(n as f64)),
        radical_inverse_vdc(i),
    ])
}

/// `ggxConvolution( { roughness, lodBias, envMap, direction, GGX_SAMPLES } )`
/// — GGX convolution by VNDF importance sampling with V = N, each sample read
/// at the source mip that matches its solid angle (filtered importance
/// sampling), `max( log2( alpha2 * invQ ) + lodBias, 0 )`.
///
/// The fragment of `PMREM_ggx`.
pub fn ggx_convolution(
    roughness: NodeRef,
    lod_bias: NodeRef,
    env_map: &CubeTexture,
    direction: NodeRef,
    ggx_samples: usize,
) -> NodeRef {
    let n = to_var(None, direction);
    let prefiltered_color = to_var(None, vec3(0.0, 0.0, 0.0));

    // For very low roughness, just sample the environment directly.
    let direct = prefiltered_color.assign(cube_texture_level(env_map, n.clone(), float(0.0)).xyz());

    let alpha = to_const(None, roughness.mul(roughness.clone()));
    let alpha2 = to_const(None, alpha.mul(alpha.clone()));

    // Tangent space basis for VNDF sampling.
    let (tangent, bitangent) = tangent_frame(&n);
    let total_weight = to_var(None, float(0.0));

    let loop_body = {
        let (n, tangent, bitangent, prefiltered_color, total_weight) = (
            n.clone(),
            tangent.clone(),
            bitangent.clone(),
            prefiltered_color.clone(),
            total_weight.clone(),
        );
        let (env_map, alpha, alpha2, lod_bias) = (
            env_map.clone(),
            alpha.clone(),
            alpha2.clone(),
            lod_bias.clone(),
        );
        move |i: &NodeRef| {
            let xi = to_const(None, hammersley(i.clone(), ggx_samples));

            // With V = N, sample the reflected direction directly.
            let inv_q = to_const(
                None,
                float(1.0).div(xi.x().one_minus().add(alpha2.mul(xi.x()))),
            );
            let n_dot_l = to_const(
                None,
                xi.x()
                    .one_minus()
                    .sub(alpha2.mul(xi.x()))
                    .mul(inv_q.clone()),
            );

            let phi = to_const(None, xi.y().mul(float(2.0 * std::f64::consts::PI)));
            let sin_theta = to_const(
                None,
                alpha
                    .mul(float(2.0))
                    .mul(sqrt(xi.x().mul(xi.x().one_minus())))
                    .mul(inv_q.clone()),
            );
            let l = to_const(
                None,
                n.mul(n_dot_l.clone()).add(
                    tangent
                        .mul(phi.cos())
                        .add(bitangent.mul(phi.sin()))
                        .mul(sin_theta.clone()),
                ),
            );

            // Match the source mip to the sample's solid angle; see lodBias.
            let d = alpha2.mul(inv_q.clone());
            let lod = max(log2(d).add(lod_bias), float(0.0));

            vec![
                xi,
                inv_q,
                n_dot_l.clone(),
                if_then(
                    n_dot_l.greater_than(float(0.0)),
                    vec![
                        phi,
                        sin_theta,
                        l.clone(),
                        // Weight by NdotL for the split-sum approximation.
                        prefiltered_color.add_assign(
                            cube_texture_level(&env_map, l, lod)
                                .xyz()
                                .mul(n_dot_l.clone()),
                        ),
                        total_weight.add_assign(n_dot_l),
                    ],
                ),
            ]
        }
    };

    let importance = vec![
        alpha,
        alpha2,
        tangent,
        bitangent,
        total_weight.clone(),
        // `Loop( { start: uint( 0 ), end: GGX_SAMPLES } )` still emits an
        // `i32` index and an unsuffixed bound — `for ( var i : i32 = 0; i <
        // 256; i ++ )` — which is why the body casts with `u32( i )`.
        loop_n("i", int(ggx_samples as i64), loop_body),
        prefiltered_color.div_assign(total_weight),
    ];

    block(
        vec![
            n,
            prefiltered_color.clone(),
            if_else(roughness.less_than(float(0.001)), vec![direct], importance),
        ],
        vec4_join(vec![prefiltered_color, float(1.0)]),
    )
}

/// `ggxIntegration( { roughness, sourceLod, sourceSize, envMap, direction } )`
/// — a GGX convolution that weights every texel of a small source mip: noise
/// free and, for the wide lobes of the rough levels, cheaper than importance
/// sampling. The fragment of `PMREM_integration`.
///
/// `source_size` is an `int` uniform in three "so the loops aren't
/// unrolled", and so here.
pub fn ggx_integration(
    roughness: NodeRef,
    source_lod: NodeRef,
    source_size: NodeRef,
    env_map: &CubeTexture,
    direction: NodeRef,
) -> NodeRef {
    let n = to_var(None, direction);

    let alpha = to_const(None, roughness.mul(roughness.clone()));
    let alpha2 = to_const(None, alpha.mul(alpha.clone()));

    let texel_size = to_const(None, float(2.0).div(source_size.to(Type::F32)));

    let prefiltered_color = to_var(None, vec3(0.0, 0.0, 0.0));
    let total_weight = to_var(None, float(0.0));

    // Pair opposite texels: only the one in N's hemisphere contributes.
    let face_loop = {
        let (n, alpha2, texel_size, prefiltered_color, total_weight, source_size, source_lod) = (
            n.clone(),
            alpha2.clone(),
            texel_size.clone(),
            prefiltered_color.clone(),
            total_weight.clone(),
            source_size.clone(),
            source_lod.clone(),
        );
        let env_map = env_map.clone();
        move |face: &NodeRef| {
            let face = face.clone();
            vec![loop_n("y", source_size.clone(), move |y: &NodeRef| {
                let y = y.clone();
                vec![loop_n("x", source_size, move |x: &NodeRef| {
                    let uv = to_const(
                        None,
                        vec2_join(vec![x.to(Type::F32), y.to(Type::F32)])
                            .add(float(0.5))
                            .mul(texel_size)
                            .sub(float(1.0)),
                    );
                    let texel_direction = to_var(
                        None,
                        // `face.equal( 0 )`: a JS number is a float, so the
                        // index is converted — `f32( face ) == 0.0`.
                        face.to(Type::F32).equal(float(0.0)).select(
                            vec3_join(vec![float(1.0), uv.clone()]),
                            face.to(Type::F32).equal(float(1.0)).select(
                                vec3_join(vec![uv.x(), float(1.0), uv.y()]),
                                vec3_join(vec![uv.clone(), float(1.0)]),
                            ),
                        ),
                    );

                    let inv_distance = to_const(
                        None,
                        inverse_sqrt(dot(uv.clone(), uv.clone()).add(float(1.0))),
                    );
                    let n_dot_l = to_var(None, dot(n, texel_direction.clone()));

                    // With V = N, NdotH squared is ( 1 + NdotL ) / 2. Common
                    // factors in the GGX distribution and texel solid angle
                    // cancel when normalized.
                    // A `let` where three writes no `toConst()`: `d` is read
                    // twice, and three's builder caches a twice-read temp as a
                    // `let` inside a loop body, where the port's makes a var.
                    let d = to_const(
                        None,
                        alpha2
                            .add(float(1.0))
                            .add(alpha2.sub(float(1.0)).mul(n_dot_l.clone())),
                    );
                    let weight = to_const(
                        None,
                        n_dot_l
                            .mul(inv_distance.clone())
                            .mul(inv_distance.clone())
                            .mul(inv_distance.clone())
                            .div(d.clone().mul(d.clone())),
                    );

                    vec![
                        uv,
                        texel_direction.clone(),
                        inv_distance.clone(),
                        n_dot_l.clone(),
                        texel_direction.mul_assign(
                            n_dot_l
                                .less_than(float(0.0))
                                .select(float(-1.0), float(1.0)),
                        ),
                        n_dot_l.assign(abs(n_dot_l.clone()).mul(inv_distance)),
                        d,
                        weight.clone(),
                        prefiltered_color.add_assign(
                            cube_texture_level(&env_map, texel_direction, source_lod)
                                .xyz()
                                .mul(weight.clone()),
                        ),
                        total_weight.add_assign(weight),
                    ]
                })]
            })]
        }
    };

    block(
        vec![
            n,
            alpha,
            alpha2,
            texel_size,
            prefiltered_color.clone(),
            total_weight.clone(),
            loop_n("face", int(3), face_loop),
        ],
        vec4_join(vec![prefiltered_color.div(total_weight), float(1.0)]),
    )
}
