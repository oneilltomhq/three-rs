//! `KHR_materials_transmission` / `_volume` — the transmission half of
//! `three.js/src/nodes/functions/PhysicalLightingModel.js`.
//!
//! Three's transmissive surface does not refract the scene it is in front of;
//! it *re-reads the frame so far*. The renderer draws every opaque object,
//! copies the resolved colour attachment into a mipped texture, and then draws
//! the transmissive objects with that texture bound. Each fragment walks a
//! refracted ray through the volume, projects its exit point back onto the
//! screen and samples the copy there, at a mip chosen from the roughness and
//! the IOR. The three `setLayout` helpers below become real WGSL functions; the
//! rest — `getTransmissionSample`, `textureBicubicLevel` and
//! `getIBLVolumeRefraction` itself — are plain `Fn()`s and inline into `main`,
//! which is why the generated glass shader is 300 lines of `nodeVarN`.
//!
//! Dispersion (`KHR_materials_dispersion`) is the one branch left out: the
//! barn lamp's glass sets none, and the `Loop` over three IORs would be dead
//! code no graded pixel could check. See `docs/nodes.md` §26.

use std::rc::Rc;

use crate::nodes::node::{FnDef, Lazy, TextureSource};
use crate::nodes::tsl::*;
use crate::nodes::{NodeRef, Type};
use crate::textures::Texture;

/// The renderer's copy of the opaque frame — `viewportOpaqueMipTexture()`.
///
/// It is the renderer that owns the texture, so it reaches
/// `MeshPhysicalNodeMaterial.setup()` through
/// [`SetupContext`](crate::materials::SetupContext) rather than off the
/// material, and like [`PmremHandle`](crate::materials::environment::PmremHandle)
/// it hashes on the texture's id alone: which texture is read changes the
/// program, its size does not.
#[derive(Clone, Debug)]
pub struct OpaqueFrame {
    pub texture: Texture,
}

impl std::hash::Hash for OpaqueFrame {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.texture.id().hash(state);
    }
}

/// `getVolumeTransmissionRay( n, v, thickness, ior, modelMatrix )` — the
/// refracted ray through the slab, scaled by the model matrix' rotation-free
/// scale because the thickness is authored in local space.
fn get_volume_transmission_ray(
    n: NodeRef,
    v: NodeRef,
    thickness_value: NodeRef,
    ior_value: NodeRef,
    model_matrix: NodeRef,
) -> NodeRef {
    thread_local! { static CELL: Lazy<Rc<FnDef>> = const { Lazy::new() }; }
    let def = CELL.with(|c| {
        c.get(|| {
            shader_fn(
                Some("getVolumeTransmissionRay"),
                vec![
                    ("n", Type::Vec3),
                    ("v", Type::Vec3),
                    ("thickness", Type::F32),
                    ("ior", Type::F32),
                    ("modelMatrix", Type::Mat4),
                ],
                Type::Vec3,
                |args| {
                    let (n, v, thickness, ior, model_matrix) = (
                        args[0].clone(),
                        args[1].clone(),
                        args[2].clone(),
                        args[3].clone(),
                        args[4].clone(),
                    );
                    let refraction_vector = refract(v.negate(), n.normalize(), float(1.0).div(ior));
                    let model_scale = vec3_join(vec![
                        length(model_matrix.clone().element(0).xyz()),
                        length(model_matrix.clone().element(1).xyz()),
                        length(model_matrix.element(2).xyz()),
                    ]);
                    refraction_vector
                        .normalize()
                        .mul(thickness.mul(model_scale))
                },
            )
        })
    });
    call(&def, vec![n, v, thickness_value, ior_value, model_matrix])
}

/// `volumeAttenuation( transmissionDistance, attenuationColor,
/// attenuationDistance )` — Beer's law, with an early return so that an
/// attenuation distance of exactly 0 leaves the colour untouched.
fn volume_attenuation(
    transmission_distance: NodeRef,
    attenuation_color_value: NodeRef,
    attenuation_distance_value: NodeRef,
) -> NodeRef {
    thread_local! { static CELL: Lazy<Rc<FnDef>> = const { Lazy::new() }; }
    let def = CELL.with(|c| {
        c.get(|| {
            shader_fn(
                Some("volumeAttenuation"),
                vec![
                    ("transmissionDistance", Type::F32),
                    ("attenuationColor", Type::Vec3),
                    ("attenuationDistance", Type::F32),
                ],
                Type::Vec3,
                |args| {
                    let (distance, color, attenuation) =
                        (args[0].clone(), args[1].clone(), args[2].clone());
                    let coefficient = log(color).negate().div(attenuation.clone());
                    let transmittance = exp(coefficient.negate().mul(distance));
                    block(
                        vec![if_then(
                            attenuation.not_equal(float(0.0)),
                            vec![return_statement(transmittance)],
                        )],
                        // "Attenuation distance is +inf, i.e. the transmitted
                        // colour is not attenuated at all."
                        vec3(1.0, 1.0, 1.0),
                    )
                },
            )
        })
    });
    call(
        &def,
        vec![
            transmission_distance,
            attenuation_color_value,
            attenuation_distance_value,
        ],
    )
}

/// `applyIorToRoughness( roughness, ior )` — an IOR of 1 refracts nothing and
/// 1.5 refracts the default amount.
fn apply_ior_to_roughness(roughness_value: NodeRef, ior_value: NodeRef) -> NodeRef {
    thread_local! { static CELL: Lazy<Rc<FnDef>> = const { Lazy::new() }; }
    let def = CELL.with(|c| {
        c.get(|| {
            shader_fn(
                Some("applyIorToRoughness"),
                vec![("roughness", Type::F32), ("ior", Type::F32)],
                Type::F32,
                |args| {
                    args[0]
                        .clone()
                        .mul(args[1].clone().mul(2.0).sub(2.0).clamp(0.0, 1.0))
                },
            )
        })
    });
    call(&def, vec![roughness_value, ior_value])
}

/// `cameraViewport` — for a single camera three folds it to
/// `vec4( 0, 0, screenSize.x, screenSize.y ).toConst( 'cameraViewport' )`,
/// which is where the `let cameraViewport = …` line in the dump comes from.
fn camera_viewport() -> NodeRef {
    to_const(
        Some("cameraViewport"),
        vec4_join(vec![
            float(0.0),
            float(0.0),
            viewport_size().x(),
            viewport_size().y(),
        ]),
    )
}

// ---------------------------------------------------------------------------
// textureBicubicLevel — `src/nodes/accessors/TextureBicubic.js`
// ---------------------------------------------------------------------------

/// `1 / 6`, three's `bC`.
const BC: f64 = 1.0 / 6.0;

fn w0(a: NodeRef) -> NodeRef {
    float(BC).mul(
        a.clone()
            .mul(a.clone().mul(a.negate().add(3.0)).sub(3.0))
            .add(1.0),
    )
}

fn w1(a: NodeRef) -> NodeRef {
    float(BC).mul(
        a.clone()
            .mul(a.clone().mul(float(3.0).mul(a).sub(6.0)))
            .add(4.0),
    )
}

fn w2(a: NodeRef) -> NodeRef {
    float(BC).mul(
        a.clone()
            .mul(a.clone().mul(float(-3.0).mul(a).add(3.0)).add(3.0))
            .add(1.0),
    )
}

fn w3(a: NodeRef) -> NodeRef {
    float(BC).mul(a.pow(float(3.0)))
}

fn g0(a: NodeRef) -> NodeRef {
    w0(a.clone()).add(w1(a))
}

fn g1(a: NodeRef) -> NodeRef {
    w2(a.clone()).add(w3(a))
}

fn h0(a: NodeRef) -> NodeRef {
    float(-1.0).add(w1(a.clone()).div(w0(a.clone()).add(w1(a))))
}

fn h1(a: NodeRef) -> NodeRef {
    float(1.0).add(w3(a.clone()).div(w2(a.clone()).add(w3(a))))
}

/// `bicubic( textureNode, texelSize, lod )` — four `textureSampleLevel`s at the
/// offsets the two weight pairs give, blended by the same weights.
fn bicubic(map: &Texture, uv: NodeRef, texel_size: NodeRef, lod: NodeRef) -> NodeRef {
    let uv_scaled = uv.mul(texel_size.clone().zw()).add(0.5);

    let iuv = floor(uv_scaled.clone());
    let fuv = fract(uv_scaled);

    let g0x = g0(fuv.clone().x());
    let g1x = g1(fuv.clone().x());
    let h0x = h0(fuv.clone().x());
    let h1x = h1(fuv.clone().x());
    let h0y = h0(fuv.clone().y());
    let h1y = h1(fuv.clone().y());

    let point = |x: NodeRef, y: NodeRef| {
        vec2_join(vec![iuv.clone().x().add(x), iuv.clone().y().add(y)])
            .sub(0.5)
            .mul(texel_size.clone().xy())
    };

    let p0 = point(h0x.clone(), h0y.clone());
    let p1 = point(h1x.clone(), h0y);
    let p2 = point(h0x, h1y.clone());
    let p3 = point(h1x, h1y);

    let a = g0(fuv.clone().y()).mul(
        g0x.clone()
            .mul(texture_level(map, p0, lod.clone()))
            .add(g1x.clone().mul(texture_level(map, p1, lod.clone()))),
    );
    let b = g1(fuv.y()).mul(
        g0x.mul(texture_level(map, p2, lod.clone()))
            .add(g1x.mul(texture_level(map, p3, lod))),
    );

    a.add(b)
}

/// `textureBicubicLevel( textureNode, lodNode )` — the two bracketing mips,
/// each bicubically filtered, mixed by the fractional LOD.
fn texture_bicubic_level(map: &Texture, uv: NodeRef, lod: NodeRef) -> NodeRef {
    let size = |level: NodeRef| {
        texture_size(TextureSource::Texture2D(map.clone()), level.to(Type::I32)).to(Type::Vec2)
    };
    let f_lod_size = size(lod.clone());
    let c_lod_size = size(lod.clone().add(1.0));
    let f_lod_size_inv = float(1.0).div(f_lod_size.clone());
    let c_lod_size_inv = float(1.0).div(c_lod_size.clone());

    let f_sample = bicubic(
        map,
        uv.clone(),
        vec4_join(vec![f_lod_size_inv, f_lod_size]),
        floor(lod.clone()),
    );
    let c_sample = bicubic(
        map,
        uv,
        vec4_join(vec![c_lod_size_inv, c_lod_size]),
        ceil(lod.clone()),
    );

    mix(f_sample, c_sample, fract(lod))
}

/// `getTransmissionSample( fragCoord, roughness, ior )` — the screen-space read
/// of the opaque frame, at a mip chosen from `log2( viewport width )` times the
/// IOR-scaled roughness.
fn get_transmission_sample(
    map: &Texture,
    frag_coord: NodeRef,
    roughness_value: NodeRef,
    ior_value: NodeRef,
) -> NodeRef {
    let viewport = camera_viewport();
    let uv = frag_coord
        .mul(viewport.clone().zw())
        .add(viewport.clone().xy())
        .div(viewport_size());
    let lod = log2(viewport.z()).mul(apply_ior_to_roughness(roughness_value, ior_value));
    texture_bicubic_level(map, uv, lod)
}

/// `getIBLVolumeRefraction( … )` without the dispersion loop: one refracted
/// ray, one screen-space read, Beer's law over the path length and the
/// surface's own Fresnel taken off the top.
///
/// `environment_brdf` is passed in rather than imported so that
/// [`crate::materials::physical`] keeps the single definition of the DFG split
/// sum; the two would otherwise be a cycle.
#[allow(clippy::too_many_arguments)]
pub fn ibl_volume_refraction(
    map: &Texture,
    n: NodeRef,
    v: NodeRef,
    position: NodeRef,
    environment_brdf: impl Fn(NodeRef, NodeRef, NodeRef, NodeRef) -> NodeRef,
) -> NodeRef {
    let transmission_ray = get_volume_transmission_ray(
        n.clone(),
        v.clone(),
        thickness(),
        ior(),
        model_world_matrix(),
    );
    let refracted_ray_exit = position.add(transmission_ray.clone());

    // Project the refracted vector onto the framebuffer, mapping to NDC.
    let ndc_pos = camera_projection_matrix()
        .mul(camera_view_matrix().mul(vec4_join(vec![refracted_ray_exit, float(1.0)])));
    let refraction_coords = to_var(None, ndc_pos.clone().xy().div(ndc_pos.w()));
    let mut pre = vec![
        refraction_coords.assign(refraction_coords.clone().add(1.0)),
        refraction_coords.assign(refraction_coords.clone().div(2.0)),
        // `// webgpu` — the Y flip three does only on this backend.
        refraction_coords.assign(vec2_join(vec![
            refraction_coords.clone().x(),
            refraction_coords.clone().y().one_minus(),
        ])),
    ];

    let transmitted_light = get_transmission_sample(map, refraction_coords, roughness(), ior());
    let transmittance = diffuse_contribution().mul(volume_attenuation(
        length(transmission_ray),
        attenuation_color(),
        attenuation_distance(),
    ));

    let attenuated_color = transmittance
        .clone()
        .rgb()
        .mul(transmitted_light.clone().rgb());
    let dot_nv = dot(n, v).clamp(0.0, 1.0);

    let f = environment_brdf(
        dot_nv,
        specular_color_blended(),
        specular_f90(),
        roughness(),
    );

    // As less light is transmitted, the opacity should rise.
    let transmittance_factor = transmittance
        .clone()
        .x()
        .add(transmittance.clone().y())
        .add(transmittance.z())
        .div(3.0);

    let value = vec4_join(vec![
        f.one_minus().mul(attenuated_color),
        transmitted_light
            .w()
            .one_minus()
            .mul(transmittance_factor)
            .one_minus(),
    ]);

    // The three `refractionCoords` rewrites are statements, so they ride ahead
    // of the expression that reads the var.
    pre.push(value.clone());
    let result = pre.pop().expect("three-rs: the value was just pushed");
    block(pre, result)
}

/// `n` and `v` as `PhysicalLightingModel.start()` builds them: the world normal
/// and the world-space direction from the fragment to the camera.
pub fn world_view_vector() -> NodeRef {
    camera_position().sub(position_world()).normalize()
}
