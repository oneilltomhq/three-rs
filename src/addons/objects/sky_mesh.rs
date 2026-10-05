//! Port of `three.js/examples/jsm/objects/SkyMesh.js` — the Preetham
//! analytic daylight model as a node material on a unit box seen from inside,
//! with a sun disc and a procedural cloud layer.
//!
//! Upstream's `SkyMesh extends Mesh`; the port's [`Mesh`] is a constructor
//! returning a scene-graph [`ObjectRef`], so [`SkyMesh`] holds that node next to
//! the uniforms, which upstream keeps as fields of the mesh itself. Each
//! uniform is a [`SettableValue`]: `sky.turbidity.value = 10` is
//! `sky.turbidity.set( vec![ 10.0 ] )`.
//!
//! The shader is upstream's two `Fn()`s node for node: `vertexNode` computes
//! the sun direction, intensity and the two extinction coefficients into four
//! varyings and pins the box to the far plane (`position.z = position.w`);
//! `colorNode` does the in-scattering, the sun disc and, under an `If`, the
//! clouds — three inline `Fn()`s, `gradient`, `noise` and `fbm`, which the
//! port writes as Rust functions that build the same nodes at each call site,
//! as an un-laid-out `Fn()` is inlined.

use crate::core::ObjectRef;
use crate::geometries::box_geometry;
use crate::materials::{MeshBasicNodeMaterial, Side};
use crate::nodes::node::SettableValue;
use crate::nodes::tsl::{
    block, camera_position, dot, exp, float, floor, fract, if_then, loop_n, max, min_of, mix,
    model_view_projection, position_world, smoothstep, time, to_const, to_var, to_var_intent,
    uniform_settable, varying_property, vec2, vec3, vec3s, vec4_join,
};
use crate::nodes::{NodeRef, Type};
use crate::objects::Mesh;

use std::rc::Rc;

/// `new SkyMesh()` — a skydome for scene backgrounds, after [A Practical
/// Analytic Model for Daylight](https://www.researchgate.net/publication/220720443_A_Practical_Analytic_Model_for_Daylight)
/// (Preetham et al.).
///
/// Scale [`SkyMesh::mesh`] up to enclose the scene (`sky.scale.setScalar(
/// 10000 )`) and add it; the vertex stage writes every fragment at the far
/// plane, so the box never occludes anything.
///
/// It can be useful to hide the sun disc when generating an environment map
/// to avoid artifacts: set [`SkyMesh::show_sun_disc`] to `0` before rendering
/// the map and back to `1` before rendering the sky box.
pub struct SkyMesh {
    /// The `Mesh` itself: a `BoxGeometry( 1, 1, 1 )` with the sky material.
    pub mesh: ObjectRef,
    /// `sky.turbidity` — default `2`.
    pub turbidity: SettableValue,
    /// `sky.rayleigh` — default `1`.
    pub rayleigh: SettableValue,
    /// `sky.mieCoefficient` — default `0.005`.
    pub mie_coefficient: SettableValue,
    /// `sky.mieDirectionalG` — default `0.8`.
    pub mie_directional_g: SettableValue,
    /// `sky.sunPosition` — a `vec3`, default `( 0, 0, 0 )`. Only its
    /// direction and its `y` reach the shader.
    pub sun_position: SettableValue,
    /// `sky.cloudScale` — default `0.0002`.
    pub cloud_scale: SettableValue,
    /// `sky.cloudSpeed` — default `0.00002`.
    pub cloud_speed: SettableValue,
    /// `sky.cloudCoverage` — default `0.4`. At `0` the cloud branch is
    /// skipped.
    pub cloud_coverage: SettableValue,
    /// `sky.cloudDensity` — default `0.4`.
    pub cloud_density: SettableValue,
    /// `sky.cloudElevation` — default `0.5`.
    pub cloud_elevation: SettableValue,
    /// `sky.showSunDisc` — whether to render the solar disc, `1` (default)
    /// or `0`.
    ///
    /// Upstream declares it `uniform( 1 )`, a float, but a `UniformNode`
    /// takes its type from its value when the material is first built, and
    /// `webgpu_sky` assigns `true` before that: three's dump has it as a
    /// `bool` (`nodeUniformN : u32`, read as `bool( … )` and multiplied in as
    /// `f32( … )`). The port's uniform types are fixed at construction, so it
    /// is a `bool` here — the type the one page that sets it gives it. Any
    /// non-zero value is `true`.
    pub show_sun_disc: SettableValue,
}

/// The uniforms as nodes, for the two `Fn()` bodies.
struct Uniforms {
    turbidity: NodeRef,
    rayleigh: NodeRef,
    mie_coefficient: NodeRef,
    mie_directional_g: NodeRef,
    sun_position: NodeRef,
    cloud_scale: NodeRef,
    cloud_speed: NodeRef,
    cloud_coverage: NodeRef,
    cloud_density: NodeRef,
    cloud_elevation: NodeRef,
    show_sun_disc: NodeRef,
}

/// The four `varyingProperty()`s. Upstream leaves them unnamed, so three
/// numbers them `nodeVaryingN`; the port's `varyingProperty` takes a name,
/// and these are upstream's JS identifiers.
struct Varyings {
    sun_direction: NodeRef,
    sun_e: NodeRef,
    beta_r: NodeRef,
    beta_m: NodeRef,
}

impl Varyings {
    fn new() -> Self {
        Self {
            sun_direction: varying_property("vSunDirection", Type::Vec3, false),
            sun_e: varying_property("vSunE", Type::F32, false),
            beta_r: varying_property("vBetaR", Type::Vec3, false),
            beta_m: varying_property("vBetaM", Type::Vec3, false),
        }
    }
}

impl SkyMesh {
    /// `new SkyMesh()`.
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        let (turbidity, turbidity_cell) = uniform_settable(Type::F32, vec![2.0]);
        let (rayleigh, rayleigh_cell) = uniform_settable(Type::F32, vec![1.0]);
        let (mie_coefficient, mie_coefficient_cell) = uniform_settable(Type::F32, vec![0.005]);
        let (mie_directional_g, mie_directional_g_cell) = uniform_settable(Type::F32, vec![0.8]);
        let (sun_position, sun_position_cell) = uniform_settable(Type::Vec3, vec![0.0, 0.0, 0.0]);
        let (cloud_scale, cloud_scale_cell) = uniform_settable(Type::F32, vec![0.0002]);
        let (cloud_speed, cloud_speed_cell) = uniform_settable(Type::F32, vec![0.00002]);
        let (cloud_coverage, cloud_coverage_cell) = uniform_settable(Type::F32, vec![0.4]);
        let (cloud_density, cloud_density_cell) = uniform_settable(Type::F32, vec![0.4]);
        let (cloud_elevation, cloud_elevation_cell) = uniform_settable(Type::F32, vec![0.5]);
        let (show_sun_disc, show_sun_disc_cell) = uniform_settable(Type::Bool, vec![1.0]);

        let uniforms = Uniforms {
            turbidity,
            rayleigh,
            mie_coefficient,
            mie_directional_g,
            sun_position,
            cloud_scale,
            cloud_speed,
            cloud_coverage,
            cloud_density,
            cloud_elevation,
            show_sun_disc,
        };

        let mesh = Mesh::new(
            Rc::new(box_geometry(1.0, 1.0, 1.0, 1, 1, 1)),
            Self::material(&uniforms),
        );

        Self {
            mesh,
            turbidity: turbidity_cell,
            rayleigh: rayleigh_cell,
            mie_coefficient: mie_coefficient_cell,
            mie_directional_g: mie_directional_g_cell,
            sun_position: sun_position_cell,
            cloud_scale: cloud_scale_cell,
            cloud_speed: cloud_speed_cell,
            cloud_coverage: cloud_coverage_cell,
            cloud_density: cloud_density_cell,
            cloud_elevation: cloud_elevation_cell,
            show_sun_disc: show_sun_disc_cell,
        }
    }

    /// The `NodeMaterial` the constructor builds. A bare `NodeMaterial` is
    /// unlit with no `colorNode` of its own, which the port's
    /// [`MeshBasicNodeMaterial`] is with its defaults.
    fn material(uniforms: &Uniforms) -> MeshBasicNodeMaterial {
        let varyings = Varyings::new();

        let mut material = MeshBasicNodeMaterial::new();
        material.side = Side::Back;
        material.depth_write = false;
        material.fog = false;

        material.vertex_node = Some(vertex_node(uniforms, &varyings));
        material.color_node = Some(color_node(uniforms, &varyings));
        material
    }
}

/// `vertexNode`.
fn vertex_node(u: &Uniforms, v: &Varyings) -> NodeRef {
    // constants for atmospheric scattering
    let e = float(std::f64::consts::E);

    // wavelength of used primaries, according to preetham
    // this pre-calculation replaces older TotalRayleigh(vec3 lambda) function:
    // (8.0 * pow(pi, 3.0) * pow(pow(n, 2.0) - 1.0, 2.0) * (6.0 + 3.0 * pn)) /
    // (3.0 * N * pow(lambda, vec3(4.0)) * (6.0 - 7.0 * pn))
    let total_rayleigh = vec3(
        5.804542996261093E-6,
        1.3562911419845635E-5,
        3.0265902468824876E-5,
    );

    // mie stuff
    // K coefficient for the primaries
    // MieConst = pi * pow( ( 2.0 * pi ) / lambda, vec3( v - 2.0 ) ) * K
    let mie_const = vec3(
        1.8399918514433978E14,
        2.7798023919660528E14,
        4.0790479543861094E14,
    );

    // earth shadow hack
    // cutoffAngle = pi / 1.95;
    let cutoff_angle = float(1.6110731556870734);
    let steepness = float(1.5);
    let ee = float(1000.0);

    // varying sun position
    // Read twice, so three emits it as a `let` (§8, "Usage-promoted temps").
    let sun_direction = to_const(None, u.sun_position.normalize());
    let assign_sun_direction = v.sun_direction.assign(sun_direction.clone());

    // varying sun intensity
    let angle = sun_direction.y();
    let zenith_angle_cos = angle.clamp(-1.0, 1.0);
    let sun_intensity = ee.mul(max(
        0.0,
        float(1.0).sub(
            e.pow(
                cutoff_angle
                    .sub(zenith_angle_cos.acos())
                    .div(steepness)
                    .negate(),
            ),
        ),
    ));
    let assign_sun_e = v.sun_e.assign(sun_intensity);

    // sun fade
    let sunfade = float(1.0).sub(
        float(1.0)
            .sub(exp(u.sun_position.y().div(450000.0)))
            .clamp(0.0, 1.0),
    );

    // varying vBetaR
    let rayleigh_coefficient = u.rayleigh.sub(float(1.0).mul(float(1.0).sub(sunfade)));

    // extinction (absorption + out scattering)
    // rayleigh coefficients
    let assign_beta_r = v.beta_r.assign(total_rayleigh.mul(rayleigh_coefficient));

    // varying vBetaM
    let c = float(0.2).mul(u.turbidity.clone()).mul(10E-18);
    let total_mie = float(0.434).mul(c).mul(mie_const);
    let assign_beta_m = v.beta_m.assign(total_mie.mul(u.mie_coefficient.clone()));

    // position
    let position = model_view_projection();
    let set_far = position.z().assign(position.w()); // set z to camera.far

    block(
        vec![
            assign_sun_direction,
            assign_sun_e,
            assign_beta_r,
            assign_beta_m,
            set_far,
        ],
        position,
    )
}

/// The inline `gradient( [ i ] )` Fn: the gradient at a lattice corner, from
/// a sinless hash so every GPU produces the same clouds.
fn gradient(i: NodeRef) -> NodeRef {
    let p = to_var(
        None,
        fract(i.swizzle("xyx").mul(vec3(0.1031, 0.1030, 0.0973))),
    );
    let add = p.add_assign(dot(p.clone(), p.swizzle("yzx").add(33.33)));

    block(
        vec![p.clone(), add],
        fract(p.swizzle("xx").add(p.swizzle("yz")).mul(p.swizzle("zy")))
            .mul(2.0)
            .sub(1.0),
    )
}

/// The inline `noise( [ p ] )` Fn: 2-D gradient noise — isotropic lobes like
/// Perlin at value-noise cost — scaled to about `[ -1, 1 ]`.
fn noise(p: NodeRef) -> NodeRef {
    // `i`, `f` and `u` are each read more than once, so three emits them as
    // `let`s (§8, "Usage-promoted temps").
    let i = to_const(None, floor(p.clone()));
    let f = to_const(None, fract(p));
    // quintic fade
    let u = to_const(
        None,
        f.mul(f.clone())
            .mul(f.clone())
            .mul(f.mul(f.mul(6.0).sub(15.0)).add(10.0)),
    );

    let a = dot(gradient(i.clone()), f.clone());
    let b = dot(gradient(i.add(vec2(1.0, 0.0))), f.sub(vec2(1.0, 0.0)));
    let c = dot(gradient(i.add(vec2(0.0, 1.0))), f.sub(vec2(0.0, 1.0)));
    let d = dot(gradient(i.add(vec2(1.0, 1.0))), f.sub(vec2(1.0, 1.0)));

    mix(mix(a, b, u.x()), mix(c, d, u.x()), u.y()).mul(1.6)
}

/// The inline `fbm( [ position, drift ] )` Fn: four octaves of [`noise`],
/// with a per-octave drift that makes the clouds billow instead of scrolling
/// as a rigid stamp.
fn fbm(position: NodeRef, drift: NodeRef) -> NodeRef {
    let p = to_var(None, position);
    let result = to_var(None, float(0.0));
    let amplitude = to_var(None, float(1.0));

    let lp = loop_n("i", 4.into(), |_| {
        vec![
            result.add_assign(amplitude.mul(noise(p.clone()))),
            amplitude.mul_assign(0.5),
            // `p.mulAssign( 2.0 ).addAssign( drift )` — two statements.
            p.mul_assign(2.0),
            p.add_assign(drift),
        ]
    });

    block(vec![p, result.clone(), amplitude, lp], result)
}

/// `colorNode`.
fn color_node(u: &Uniforms, v: &Varyings) -> NodeRef {
    // constants for atmospheric scattering
    let pi = float(std::f64::consts::PI);

    // optical length at zenith for molecules
    let rayleigh_zenith_length = float(8.4E3);
    let mie_zenith_length = float(1.25E3);
    // 66 arc seconds -> degrees, and the cosine of that
    let sun_angular_diameter_cos = float(0.9999566769464484);

    // 3.0 / ( 16.0 * pi )
    let three_over_sixteenpi = float(0.05968310365946075);
    // 1.0 / ( 4.0 * pi )
    let one_over_fourpi = float(0.07957747154594767);

    // Every `to_const` below is a temp three's dump has as a `let` because it
    // is read more than once (§8, "Usage-promoted temps"); the JS has none.
    let direction = to_const(None, position_world().sub(camera_position()).normalize());

    // optical length
    // cutoff angle at 90 to avoid singularity in next formula.
    let zenith_angle = to_const(None, max(0.0, direction.y()).acos());
    let inverse = to_const(
        None,
        float(1.0).div(
            zenith_angle.cos().add(
                float(0.15).mul(
                    float(93.885)
                        .sub(zenith_angle.mul(180.0).div(pi))
                        .pow(-1.253),
                ),
            ),
        ),
    );
    let s_r = rayleigh_zenith_length.mul(inverse.clone());
    let s_m = mie_zenith_length.mul(inverse);

    // combined extinction factor
    let fex = to_const(None, exp(v.beta_r.mul(s_r).add(v.beta_m.mul(s_m)).negate()));

    // in scattering
    let cos_theta = to_const(None, dot(direction.clone(), v.sun_direction.clone()));

    // betaRTheta
    let c = cos_theta.mul(0.5).add(0.5);
    let r_phase = three_over_sixteenpi.mul(float(1.0).add(c.pow(2.0)));
    let beta_r_theta = to_const(None, v.beta_r.mul(r_phase));

    // betaMTheta
    let g2 = to_const(None, u.mie_directional_g.pow(2.0));
    let inv = float(1.0).div(
        float(1.0)
            .sub(
                float(2.0)
                    .mul(u.mie_directional_g.clone())
                    .mul(cos_theta.clone()),
            )
            .add(g2.clone())
            .pow(1.5),
    );
    let m_phase = one_over_fourpi.mul(float(1.0).sub(g2)).mul(inv);
    let beta_m_theta = to_const(None, v.beta_m.mul(m_phase));

    // Upstream writes this product out twice, as two nodes, so three
    // evaluates it twice; so does the port.
    let scattering = || {
        v.sun_e.mul(
            beta_r_theta
                .add(beta_m_theta.clone())
                .div(v.beta_r.add(v.beta_m.clone())),
        )
    };
    // `pow()` is an intent var, and `Lin.mulAssign()` makes it a real one.
    let lin = to_var_intent(
        scattering()
            .mul(float(1.0).sub(fex.clone()))
            .pow(vec3s(1.5)),
    );
    let lin_mul = lin.mul_assign(mix(
        vec3s(1.0),
        scattering().mul(fex.clone()).pow(vec3s(1.0 / 2.0)),
        float(1.0).sub(v.sun_direction.y()).pow(5.0).clamp(0.0, 1.0),
    ));

    // nightsky
    let l0 = to_const(None, vec3s(0.1).mul(fex.clone()));

    // composition + solar disc
    let sundisc = cos_theta
        .sub(sun_angular_diameter_cos)
        .mul(50000.0)
        .clamp(0.0, 1.0)
        .mul(u.show_sun_disc.clone());
    let sundisc_color = to_const(
        None,
        min_of(v.sun_e.mul(fex.clone()), 80.0)
            .mul(760.0)
            .mul(sundisc),
    );

    let tex_color = to_var(
        None,
        lin.add(l0.clone())
            .mul(0.04)
            .add(sundisc_color.clone())
            .add(vec3(0.0, 0.0003, 0.00075)),
    );

    // Clouds
    let clouds = {
        // Project to cloud plane (higher elevation = clouds appear lower/closer)
        let elevation = mix(1.0, 0.1, u.cloud_elevation.clone());
        let cloud_uv = to_var(None, direction.xz().div(direction.y().mul(elevation)));
        let scale = cloud_uv.mul_assign(u.cloud_scale.clone());
        let drift = cloud_uv.add_assign(time().mul(u.cloud_speed.clone()));

        // Cloud density field
        let evolve = time().mul(u.cloud_speed.clone()).mul(300.0);
        let cloud_noise = to_var(
            None,
            fbm(cloud_uv.mul(1000.0), evolve)
                .mul(0.7)
                .add(0.5)
                .clamp(0.0, 1.0),
        );

        // Large-scale coverage variation: clear gaps next to dense banks
        // `noise()` reads its argument twice; here, unlike inside `fbm`, it is
        // not a var, so three emits it as a `let`.
        let region = noise(to_const(None, cloud_uv.mul(300.0)))
            .mul(0.37)
            .add(0.5);
        let cov = u
            .cloud_coverage
            .add(region.sub(0.5).mul(0.6))
            .clamp(0.0, 1.0);

        // Carve clouds where noise rises above the coverage level
        let threshold = to_var(None, float(1.0).sub(cov));
        let cloud_mask = to_var(
            None,
            smoothstep(threshold.clone(), threshold.add(0.3), cloud_noise.clone()),
        );

        // Fade clouds near horizon (adjusted by elevation)
        let horizon_fade = to_const(
            None,
            smoothstep(
                0.0,
                float(0.03).add(float(0.06).mul(u.cloud_elevation.clone())),
                direction.y(),
            ),
        );
        let fade = cloud_mask.mul_assign(horizon_fade.clone());

        // Cloud lighting from the sky's own radiance
        let day_factor = smoothstep(-0.08, 0.3, v.sun_direction.y());
        // 0.22 ~ albedo/pi, 0.04 = exposure; the aerial composite adds the
        // eye-leg extinction
        let sun_color = to_var(None, v.sun_e.mul(fex.clone()).mul(0.22).mul(0.04));
        let sky_ambient = lin.mul(0.04).add(vec3(0.0, 0.0003, 0.00075));

        // Beer-powder self-shadow from the sampled density
        let depth = to_var(None, max(0.0, cloud_noise.sub(threshold.clone())));
        let beer = to_var(None, exp(depth.mul(-4.0)));
        let powder = float(1.0).sub(beer.mul(beer.clone())); // beer*beer == exp(-8*depth)
                                                             // 2.6 = 1/0.385, normalizes beer*powder peak to 1
        let shade = mix(0.45, 1.0, beer.mul(powder).mul(2.6).clamp(0.0, 1.0));

        // Henyey-Greenstein forward lobe ( g = 0.7 ): silver lining on rims
        // toward the sun. 0.51=1-g^2, 1.49=1+g^2, 1.4=2g
        let silver = float(0.51)
            .div(float(1.49).sub(cos_theta.mul(1.4)).pow(1.5))
            .clamp(0.0, 3.0);
        let edge = cloud_mask.mul(float(1.0).sub(cloud_mask.clone())).mul(4.0);

        let cloud_color = to_var(None, sky_ambient.add(sun_color.mul(shade)));
        let rim = cloud_color.add_assign(sun_color.mul(silver).mul(edge).mul(0.6));
        let day = cloud_color.mul_assign(day_factor.max(0.03));

        // Cloud opacity via Beer's law: density sets how solid the clouds get
        let alpha = to_var(
            None,
            float(1.0)
                .sub(exp(depth.mul(u.cloud_density.clone()).mul(-12.0)))
                .mul(horizon_fade),
        );

        // Occlude the sun disc/glow behind opaque cloud
        let occlude = tex_color.sub_assign(l0.mul(0.04).add(sundisc_color).mul(alpha.clone()));

        // Composite through the atmosphere so distant clouds dissolve into haze
        let cloud_aerial = mix(tex_color.clone(), cloud_color.clone(), fex);
        let composite = tex_color.assign(mix(tex_color.clone(), cloud_aerial, alpha.clone()));

        if_then(
            direction
                .y()
                .greater_than(0.0)
                .and(u.cloud_coverage.greater_than(0.0)),
            vec![
                cloud_uv,
                scale,
                drift,
                cloud_noise,
                threshold,
                cloud_mask,
                fade,
                sun_color,
                depth,
                beer,
                cloud_color,
                rim,
                day,
                alpha,
                occlude,
                composite,
            ],
        )
    };

    block(
        vec![lin_mul, tex_color.clone(), clouds],
        vec4_join(vec![tex_color, float(1.0)]),
    )
}
