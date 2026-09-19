//! Prints the WGSL the node system generates for every material in rungs 1–4,
//! so it can be diffed against three.js' own dumped output.

#[allow(dead_code)]
#[path = "webgpu_morphtargets.rs"]
mod morphtargets;

use three_rs::lights::LightKind;
use three_rs::materials::phong::{LightDesc, ShadowMap};
use three_rs::materials::{setup, MeshBasicNodeMaterial, SetupContext, Side};
use three_rs::math::Color;
use three_rs::nodes::pmrem_node::PmremEnvironment;
use three_rs::nodes::tsl::*;
use three_rs::nodes::NodeBuilder;
use three_rs::textures::{CubeTexture, DepthTexture, Image, Texture};

#[path = "webgpu_tsl_galaxy.rs"]
#[allow(dead_code)] // the example's own `main()` is unused here
mod webgpu_tsl_galaxy;

#[path = "webgpu_mesh_batch.rs"]
#[allow(dead_code)]
mod webgpu_mesh_batch;

#[path = "webgpu_compute_points.rs"]
#[allow(dead_code)]
mod webgpu_compute_points;

fn show(label: &str, material: &MeshBasicNodeMaterial, ctx: SetupContext) {
    show_fog(label, material, ctx, None)
}

fn show_fog(
    label: &str,
    material: &MeshBasicNodeMaterial,
    ctx: SetupContext,
    fog: Option<&three_rs::nodes::tsl::FogNode>,
) {
    let flow = setup(material, &ctx, fog);
    let program = NodeBuilder::new().build(&flow);
    println!("########## {label} — vertex");
    println!("{}", program.vertex_wgsl);
    println!("########## {label} — fragment");
    println!("{}", program.fragment_wgsl);
    // Names and types only: an `AttributeSlot`'s source carries an `Rc`, and a
    // `BindingDesc::Buffer`'s `id` is a heap address, so printing either
    // verbatim would make two runs of this tool differ.
    let attributes: Vec<(&str, three_rs::nodes::Type)> = program
        .attributes
        .iter()
        .map(|slot| (slot.name.as_str(), slot.ty))
        .collect();
    println!("########## {label} — attributes {attributes:?}");
    for (i, g) in program.groups.iter().enumerate() {
        println!("  group {i}:");
        for (b, d) in g.iter().enumerate() {
            println!("    {b}: {}", strip_ids(&format!("{d:?}")));
        }
    }
    // The grouped vertex buffer layouts, printed only when something is
    // instanced — for a plain geometry material they are one buffer per
    // attribute and the attribute list above already says it.
    let buffers = program.vertex_buffers();
    if buffers.iter().any(|desc| desc.instanced) {
        println!("  vertex buffers:");
        for (slot, desc) in buffers.iter().enumerate() {
            let kind = match &desc.source {
                three_rs::nodes::builder::VertexBufferSource::Geometry(name) => name.to_string(),
                three_rs::nodes::builder::VertexBufferSource::Instance(buffer) => {
                    format!("{:?} x{}", buffer.source, buffer.count)
                }
            };
            println!(
                "    slot {slot}: stride {} step {} {kind} attributes {:?}",
                desc.array_stride,
                if desc.instanced { "instance" } else { "vertex" },
                desc.attributes
            );
        }
    }
}

/// `id: 94139…, ` out of a `BindingDesc` debug line.
fn strip_ids(line: &str) -> String {
    let mut out = String::with_capacity(line.len());
    let mut rest = line;
    while let Some(at) = rest.find("id: ") {
        out.push_str(&rest[..at]);
        let after = &rest[at + 4..];
        let end = after.find(", ").map(|i| i + 2).unwrap_or(after.len());
        rest = &after[end..];
    }
    out.push_str(rest);
    out
}

/// A compute kernel: one module, one entry point, and the dispatch the
/// renderer will ask for. `dispatch` and `workgroup_size` are printed because
/// three.js' dump records them beside the WGSL and a kernel with the right
/// source and the wrong dispatch is the failure the graded image cannot see.
fn show_compute(label: &str, flow: &three_rs::nodes::ComputeFlow) {
    let program = NodeBuilder::new().build_compute(flow);
    println!("########## {label} — compute");
    println!("{}", program.wgsl);
    println!(
        "########## {label} — workgroup_size {:?} dispatch {:?}",
        program.workgroup_size, program.dispatch
    );
    for (i, g) in program.groups.iter().enumerate() {
        println!("  group {i}:");
        for (b, d) in g.iter().enumerate() {
            println!("    {b}: {}", strip_ids(&format!("{d:?}")));
        }
    }
}

use three_rs::addons::lines::LineGeometry;
use three_rs::nodes::materialx::{mx_fractal_noise_float, mx_fractal_noise_vec3};

fn main() {
    // rung 1: the scene override material and the depth-texture quad.
    show(
        "basic",
        &MeshBasicNodeMaterial::new(),
        SetupContext::default(),
    );

    // The `lines` branch: `new LineBasicNodeMaterial( { color: 0xffffff } )`,
    // the d33 treemap's tile outlines. Three's own dump of it from that page is
    // in `docs/lines/LineBasicNodeMaterial_27.{vert,frag}.wgsl`; it is the
    // `basic` program above statement for statement, which is why there is no
    // `MaterialKind::Line`. What makes it a line is the topology the *object*
    // picks, and that is not in the WGSL at all.
    show(
        "line_basic",
        &MeshBasicNodeMaterial::line(Color::from_hex(0xffffff)),
        SetupContext::default(),
    );

    let depth = DepthTexture::new();
    let mut quad = MeshBasicNodeMaterial::new();
    quad.color_node = Some(depth_texture(&depth));
    quad.vertex_node = Some(three_rs::materials::quad_vertex_node());
    show("depth_texture_quad", &quad, SetupContext::default());

    // rung 2: the instanced mesh.
    let mut inst = MeshBasicNodeMaterial::new();
    inst.color_node = Some(mix(
        normal_world(),
        three_rs::materials::instanced_range(
            Color::new(0.0, 1.0, 0.0),
            Color::new(0.0, 0.0, 1.0),
            1000,
        )
        .xyz(),
        osc_sine(time().mul(float(0.1))),
    ));
    show(
        "instance_mesh",
        &inst,
        SetupContext {
            instance_count: Some(1000),
            instanced: true,
            instance_color: None,
            lights: Vec::new(),
            morph: None,
            skin: None,
            batch: None,
            line_segments: None,
            mrt: None,
        },
    );

    // rung 3: the env-mapped sphere and the skybox background.
    let cube = CubeTexture::new(vec![
        Image {
            width: 1,
            height: 1,
            data: vec![0; 4],
        };
        6
    ]);
    let mut env = MeshBasicNodeMaterial::new();
    env.env_map = Some(cube.clone());
    show("basic_envmap", &env, SetupContext::default());

    let mut bg = MeshBasicNodeMaterial::new();
    bg.color_node = Some(three_rs::materials::background_color_node(&cube));
    bg.vertex_node = Some(three_rs::materials::background_vertex_node());
    bg.side = Side::Back;
    bg.depth_test = false;
    bg.depth_write = false;
    show("background_cube", &bg, SetupContext::default());

    // the output pass.
    let framebuffer = Texture::render_target(800, 500, wgpu::TextureFormat::Rgba16Float);
    let mut out = MeshBasicNodeMaterial::new();
    out.fragment_node = Some(three_rs::materials::output_fragment_node(
        &framebuffer,
        three_rs::ToneMapping::None,
    ));
    show("output_color_transform", &out, SetupContext::default());

    // rung 8: the same pass with Reinhard tone mapping and exposure.
    let mut out_reinhard = MeshBasicNodeMaterial::new();
    out_reinhard.fragment_node = Some(three_rs::materials::output_fragment_node(
        &framebuffer,
        three_rs::materials::ToneMapping::Reinhard,
    ));
    show(
        "output_color_transform_reinhard",
        &out_reinhard,
        SetupContext::default(),
    );

    // rung 4: the textured box and the hue/saturation quad.
    let uv_texture = Texture::new(1024, 1024, Some(vec![0; 4]));
    let mut box_material = MeshBasicNodeMaterial::new();
    box_material.color_node = Some(texture(&uv_texture));
    show("rtt_box", &box_material, SetupContext::default());

    let rt = Texture::render_target(800, 500, wgpu::TextureFormat::Rgba8Unorm);
    let mouse = uniform_value(three_rs::nodes::Type::Vec2, vec![0.0, 0.0]);
    let mut fx = MeshBasicNodeMaterial::new();
    fx.color_node = Some(hue(
        saturation(texture(&rt).rgb(), mouse.x().one_minus()),
        mouse.y(),
    ));
    fx.vertex_node = Some(three_rs::materials::quad_vertex_node());
    show("rtt_fx_quad", &fx, SetupContext::default());

    // rung 9: the masking RenderPipeline quad.
    let rt = |_n: &str| Texture::render_target(800, 500, wgpu::TextureFormat::Rgba16Float);
    let base = to_var(None, texture_uv(&rt("base"), uv()));
    let mask1 = to_var(None, texture_uv(&rt("mask1"), uv()));
    let mask2 = to_var(None, texture_uv(&rt("mask2"), uv()));
    let texture1 = Texture::new(758, 600, Some(vec![0; 4]));
    let texture2 = Texture::new(4096, 2048, Some(vec![0; 4]));
    let mut compose = base;
    compose = mask1.a().mix(compose, texture(&texture1));
    compose = mask2.a().mix(compose, texture(&texture2));
    let mut masking = MeshBasicNodeMaterial::new();
    masking.fragment_node = Some(three_rs::materials::render_output(
        compose,
        three_rs::ToneMapping::None,
    ));
    masking.vertex_node = Some(three_rs::materials::quad_vertex_node());
    show("masking_quad", &masking, SetupContext::default());

    // rung webgpu_postprocessing_radial_blur: the RenderPipeline quad.
    let pass_rt = Texture::render_target(800, 500, wgpu::TextureFormat::Rgba16Float);
    let options = three_rs::nodes::display::RadialBlurOptions {
        weight: uniform_value(three_rs::nodes::Type::F32, vec![0.9]),
        decay: uniform_value(three_rs::nodes::Type::F32, vec![0.95]),
        exposure: uniform_value(three_rs::nodes::Type::F32, vec![5.0]),
        count: uniform_value(three_rs::nodes::Type::F32, vec![32.0]),
        ..Default::default()
    };
    let mut radial = MeshBasicNodeMaterial::new();
    radial.fragment_node = Some(three_rs::materials::render_output(
        three_rs::nodes::display::radial_blur(&pass_rt, &options),
        three_rs::ToneMapping::Neutral,
    ));
    radial.vertex_node = Some(three_rs::materials::quad_vertex_node());
    show("radial_blur_quad", &radial, SetupContext::default());

    // …and its scene: one flat-shaded instanced `MeshStandardMaterial` with
    // `setColorAt()` colours, under a hemisphere light and a point light.
    let mut radial_scene = MeshBasicNodeMaterial::standard(Color::new(1.0, 1.0, 1.0), 1.0, 0.0);
    radial_scene.flat_shading = true;
    show(
        "radial_blur_scene",
        &radial_scene,
        SetupContext {
            instance_count: Some(100),
            instanced: true,
            instance_color: Some(100),
            lights: vec![
                LightDesc {
                    index: 0,
                    kind: LightKind::Hemisphere,
                    shadow_map: None,
                },
                LightDesc {
                    index: 1,
                    kind: LightKind::Point,
                    shadow_map: None,
                },
            ],
            ..SetupContext::default()
        },
    );

    // rung webgpu_postprocessing_ssaa: the accumulation quad. `texture( rt )`
    // — with its `mat3x3` uv matrix — times the sample weight, unpremultiplied,
    // on a material whose `premultipliedAlpha` premultiplies it again.
    let sample_rt = Texture::render_target(800, 500, wgpu::TextureFormat::Rgba16Float);
    let (sample_weight, _cell) = uniform_settable(three_rs::nodes::Type::F32, vec![0.125]);
    let mut ssaa = MeshBasicNodeMaterial::new();
    ssaa.name = "SSAA";
    ssaa.fragment_node = Some(unpremultiply_alpha(texture(&sample_rt).mul(sample_weight)));
    ssaa.transparent = true;
    ssaa.depth_test = false;
    ssaa.depth_write = false;
    ssaa.premultiplied_alpha = true;
    ssaa.blending = three_rs::materials::Blending::Additive;
    ssaa.vertex_node = Some(three_rs::materials::quad_vertex_node());
    show("ssaa_quad", &ssaa, SetupContext::default());

    // …its `RenderPipeline` quad, which is `renderOutput` over the pass
    // texture with no tone mapping at all.
    // Built from a real `PassNode`, so this is the port's own graph and not a
    // hand-rolled stand-in: `renderOutput( passNode.getTextureNode() )`.
    let accumulator = three_rs::PassNode::new();
    let mut ssaa_output = MeshBasicNodeMaterial::new();
    ssaa_output.name = "RenderPipeline";
    ssaa_output.fragment_node = Some(three_rs::materials::render_output(
        accumulator.node(),
        three_rs::ToneMapping::None,
    ));
    ssaa_output.vertex_node = Some(three_rs::materials::quad_vertex_node());
    show(
        "ssaa_render_pipeline_quad",
        &ssaa_output,
        SetupContext::default(),
    );

    // …and its scene: one instanced `MeshStandardMaterial` with `setColorAt()`
    // colours, under three point lights and an ambient one. Smooth-shaded,
    // unlike radial_blur's.
    let ssaa_scene = MeshBasicNodeMaterial::standard(Color::new(1.0, 1.0, 1.0), 1.0, 0.0);
    show(
        "ssaa_scene",
        &ssaa_scene,
        SetupContext {
            instance_count: Some(120),
            instanced: true,
            instance_color: Some(120),
            lights: (0..3)
                .map(|index| LightDesc {
                    index,
                    kind: LightKind::Point,
                    shadow_map: None,
                })
                .chain(std::iter::once(LightDesc {
                    index: 3,
                    kind: LightKind::Ambient,
                    shadow_map: None,
                }))
                .collect(),
            ..SetupContext::default()
        },
    );

    // rung webgpu_postprocessing_bloom_selective (part A): the MRT scene
    // material. One `MeshBasicNodeMaterial` per sphere, each with
    // `mrtNode = mrt( { bloomIntensity: uniform( 0 or 1 ) } )` over the pass's
    // `mrt( { output, bloomIntensity: float( 0 ) } )`. The fragment stage's
    // `OutputType` struct and its two `output.mN` lines are the whole of what
    // part A adds; diffed against `m00`/`m01` of the scout's dump.
    let mut bloom_scene = MeshBasicNodeMaterial::new();
    bloom_scene.color = Color::new(0.25, 0.5, 0.75);
    bloom_scene.mrt_node = Some(three_rs::nodes::mrt(vec![(
        "bloomIntensity",
        uniform_value(three_rs::nodes::Type::F32, vec![1.0]),
    )]));
    show(
        "bloom_selective_scene",
        &bloom_scene,
        SetupContext {
            mrt: Some(three_rs::materials::MrtContext {
                node: three_rs::nodes::mrt(vec![
                    ("output", output_property()),
                    ("bloomIntensity", float(0.0)),
                ]),
                attachments: vec!["output".to_string(), "bloomIntensity".to_string()],
            }),
            ..SetupContext::default()
        },
    );

    // rung 5: the three teapots and the light spheres, against
    // `target/dumps/webgpu_lights_phong/`.
    let fog = fog(Color::from_hex(0xFF00FF), range_fog_factor(12.0, 30.0));
    let four = SetupContext {
        lights: (0..4)
            .map(|index| LightDesc {
                index,
                kind: LightKind::Point,
                shadow_map: None,
            })
            .collect(),
        ..SetupContext::default()
    };

    let normal_map_texture = Texture::new(512, 512, Some(vec![0; 4]));
    let alpha_texture = Texture::new(512, 512, Some(vec![0; 4]));

    let grey = Color::from_hex(0x555555);

    let mut left = MeshBasicNodeMaterial::phong(grey);
    left.lights_node = Some(vec![0]);
    left.specular_node = Some(texture(&alpha_texture));
    show_fog("phong_left", &left, four.clone(), Some(&fog));

    let mut centre = MeshBasicNodeMaterial::phong(grey);
    centre.normal_node = Some(normal_map(texture(&normal_map_texture)));
    centre.shininess = 80.0;
    show_fog("phong_centre", &centre, four.clone(), Some(&fog));

    let mut right = MeshBasicNodeMaterial::phong(grey);
    right.lights_node = Some(vec![1]);
    right.specular_node = Some(mix(
        Color::from_hex(0x0000FF),
        Color::from_hex(0xFF0000),
        checker(uv().mul(5.0)),
    ));
    right.shininess = 90.0;
    show_fog("phong_right", &right, four.clone(), Some(&fog));

    // rung 8: the four physical materials, against
    // `handoff/scouts/rung8/MeshStandardMaterial_*`.
    let bulb_lights = |shadow: Option<ShadowMap>| SetupContext {
        lights: vec![
            LightDesc {
                index: 0,
                kind: LightKind::Point,
                shadow_map: shadow,
            },
            LightDesc {
                index: 1,
                kind: LightKind::Hemisphere,
                shadow_map: None,
            },
        ],
        ..SetupContext::default()
    };
    let two_lights = bulb_lights(None);

    let mut bulb = MeshBasicNodeMaterial::standard(Color::from_hex(0x000000), 1.0, 0.0);
    bulb.emissive = Color::from_hex(0xffffee);
    bulb.emissive_intensity = 1.0;
    show("standard_bulb", &bulb, two_lights.clone());

    let brick_for_shadow = Texture::new(512, 512, Some(vec![0; 4]));
    let hardwood = Texture::new(1024, 1024, Some(vec![0; 4]));
    let hardwood_bump = Texture::new(1024, 1024, Some(vec![0; 4]));
    let hardwood_roughness = Texture::new(1024, 1024, Some(vec![0; 4]));
    let mut floor = MeshBasicNodeMaterial::standard(Color::new(1.0, 1.0, 1.0), 0.8, 0.2);
    floor.map = Some(hardwood.clone());
    floor.bump_map = Some(hardwood_bump.clone());
    floor.roughness_map = Some(hardwood_roughness.clone());
    show("standard_floor", &floor, two_lights.clone());
    // The same floor material with the bulb's shadow wired in: the one
    // `receiveShadow` mesh of the scene. Target: `MeshStandardMaterial_18`.
    show(
        "standard_floor_shadow",
        &floor,
        bulb_lights(Some(ShadowMap::Cube(
            three_rs::textures::CubeDepthTexture::new(512),
        ))),
    );
    // `ShadowBaseNode._getShadowMaterial()` for a casting material with a map.
    // Target: `ShadowMaterial_24`.
    let mut shadow = MeshBasicNodeMaterial::new();
    shadow.color_node = Some(vec4_join(vec![
        vec3(0.0, 0.0, 0.0),
        float(1.0).mul(texture(&brick_for_shadow).a()),
    ]));
    shadow.side = Side::Back;
    show("shadow_material", &shadow, SetupContext::default());

    let brick = Texture::new(512, 512, Some(vec![0; 4]));
    let brick_bump = Texture::new(512, 512, Some(vec![0; 4]));
    let mut cube_material = MeshBasicNodeMaterial::standard(Color::new(1.0, 1.0, 1.0), 0.7, 0.2);
    cube_material.map = Some(brick.clone());
    cube_material.bump_map = Some(brick_bump.clone());
    show("standard_cube", &cube_material, two_lights.clone());

    let earth = Texture::new(2048, 1024, Some(vec![0; 4]));
    let earth_specular = Texture::new(2048, 1024, Some(vec![0; 4]));
    let mut ball = MeshBasicNodeMaterial::standard(Color::new(1.0, 1.0, 1.0), 0.5, 1.0);
    ball.map = Some(earth.clone());
    ball.metalness_map = Some(earth_specular.clone());
    show("standard_ball", &ball, two_lights);

    let mut sphere = MeshBasicNodeMaterial::phong(Color::new(1.0, 1.0, 1.0));
    sphere.lights = false;
    sphere.color_node = Some(Color::from_hex(0x0040ff).into());
    show_fog("phong_light_sphere", &sphere, four.clone(), Some(&fog));

    // The instanced-attribute path: the same material as rung 2's, with an
    // instance count whose matrices (2000 * 64 = 128000 bytes) and whose
    // `range()` (2000 * 16 = 32000 bytes) straddle the 64 KiB uniform buffer
    // limit, so the matrix becomes four instanced `vec4` attributes and the
    // range stays a uniform buffer. Nothing in the ladder reaches this yet;
    // rung 13 (20000 sprites) and sdf-text's glyph quads do.
    const MANY: usize = 2000;
    let mut many = MeshBasicNodeMaterial::new();
    many.color_node = Some(mix(
        normal_world(),
        three_rs::materials::instanced_range(
            Color::new(0.0, 1.0, 0.0),
            Color::new(0.0, 0.0, 1.0),
            MANY,
        )
        .xyz(),
        osc_sine(time().mul(float(0.1))),
    ));
    show(
        "instance_mesh_2000",
        &many,
        SetupContext {
            instance_count: Some(MANY),
            instanced: true,
            ..SetupContext::default()
        },
    );

    // rung 6: the morphing box, against
    // `handoff/scouts/rung6/MeshPhongNodeMaterial.{vert,frag}-r186.wgsl`.
    let geometry = std::rc::Rc::new(morphtargets::create_geometry());
    let mut morph = MeshBasicNodeMaterial::phong(Color::from_hex(0xff0000));
    morph.flat_shading = true;
    show(
        "morphtargets",
        &morph,
        SetupContext {
            lights: vec![
                LightDesc {
                    index: 0,
                    kind: LightKind::Ambient,
                    shadow_map: None,
                },
                LightDesc {
                    index: 1,
                    kind: LightKind::Point,
                    shadow_map: None,
                },
            ],
            morph: three_rs::nodes::morph::get_entry(&geometry),
            ..SetupContext::default()
        },
    );

    // rung 13: the galaxy's `SpriteNodeMaterial`, against
    // `handoff/scouts/rung13/{vertex,fragment}-r186.wgsl`.
    show(
        "tsl_galaxy_sprite",
        &webgpu_tsl_galaxy::galaxy_material(),
        SetupContext {
            instance_count: Some(webgpu_tsl_galaxy::COUNT),
            instanced: true,
            ..SetupContext::default()
        },
    );

    // rung 7: `webgpu_shadowmap`, against
    // `handoff/scouts/rung7/m0*-r186.wgsl`. Light order is the scene order:
    // ambient, spot, directional.
    let shadow_fog =
        three_rs::nodes::tsl::fog(Color::from_hex(0x222244), range_fog_factor(50.0, 100.0));

    let mut background = MeshBasicNodeMaterial::new();
    background.color_node = Some(three_rs::materials::background_node_color_node(
        Color::from_hex(0x222244).into(),
    ));
    background.vertex_node = Some(three_rs::materials::background_vertex_node());
    background.side = Side::Back;
    background.depth_test = false;
    background.depth_write = false;
    background.fog = false;
    show("shadowmap_background", &background, SetupContext::default());

    let spot_map = DepthTexture::new();
    let dir_map = DepthTexture::new();
    let lit = |shadows: bool| SetupContext {
        lights: vec![
            LightDesc {
                index: 0,
                kind: LightKind::Ambient,
                shadow_map: None,
            },
            LightDesc {
                index: 1,
                kind: LightKind::Spot,
                shadow_map: shadows.then(|| ShadowMap::Planar(spot_map.clone())),
            },
            LightDesc {
                index: 2,
                kind: LightKind::Directional,
                shadow_map: shadows.then(|| ShadowMap::Planar(dir_map.clone())),
            },
        ],
        ..SetupContext::default()
    };

    // The pillars and the torus knot share one `MeshPhongNodeMaterial`; only
    // the knot's clone carries the mask, and only the ground receives shadows.
    let mut pillars = MeshBasicNodeMaterial::phong(Color::from_hex(0x999999));
    pillars.shininess = 0.0;
    pillars.specular = Color::from_hex(0x222222);
    show_fog(
        "shadowmap_phong_pillars",
        &pillars,
        lit(false),
        Some(&shadow_fog),
    );

    let mut knot = pillars.clone();
    knot.transparent = true;
    knot.mask_node = Some(
        mx_fractal_noise_float(position_local().mul(0.1), 3, 2.0, 0.5, 1.0)
            .x()
            .greater_than(0.0),
    );
    show_fog("shadowmap_phong_knot", &knot, lit(true), Some(&shadow_fog));

    let ground_position = || {
        let pos = to_var(None, position_world());
        let sum = to_var(
            None,
            pos.clone().xz().add(
                mx_fractal_noise_vec3(position_world().mul(2.0), 3, 2.0, 0.5, 1.0)
                    .saturate()
                    .xz(),
            ),
        );
        let statements = vec![
            pos.clone().x().assign(sum.clone().element_node(int(0))),
            pos.clone().z().assign(sum.element_node(int(1))),
        ];
        (pos, statements)
    };

    let mut ground = MeshBasicNodeMaterial::phong(Color::from_hex(0x999999));
    ground.shininess = 0.0;
    ground.specular = Color::from_hex(0x111111);
    let (pos, statements) = ground_position();
    ground.received_shadow_position_node = Some(block(statements, pos));
    let (_pos, statements) = ground_position();
    ground.color_node = Some(block(
        statements,
        mx_fractal_noise_vec3(position_world().mul(2.0), 3, 2.0, 0.5, 1.0)
            .saturate()
            .zzz()
            .mul(0.2)
            .add(0.5),
    ));
    show_fog(
        "shadowmap_phong_ground",
        &ground,
        lit(true),
        Some(&shadow_fog),
    );

    // The three `ShadowMaterial` programs, one per source material.
    show(
        "shadowmap_shadow_pillars",
        &three_rs::materials::shadow_material(&pillars),
        SetupContext::default(),
    );
    show(
        "shadowmap_shadow_ground",
        &three_rs::materials::shadow_material(&ground),
        SetupContext::default(),
    );
    show(
        "shadowmap_shadow_knot",
        &three_rs::materials::shadow_material(&knot),
        SetupContext::default(),
    );

    // rung `webgpu_materials`: the TSL breadth example, against
    // `scouts/scouts/webgpu_materials/dump/m*.wgsl`.
    // The uv-grid texture every textured material in the page shares (one
    // `TextureLoader.load`, so one `Texture` and one texture matrix uniform).
    let uv_texture = Texture::new(1024, 1024, Some(vec![0; 4]));
    let opacity_texture = Texture::new(512, 512, Some(vec![0; 4]));

    let colour = |label: &str, node: three_rs::nodes::NodeRef| {
        let mut m = MeshBasicNodeMaterial::new();
        m.color_node = Some(node);
        show(label, &m, SetupContext::default());
    };

    colour("materials_position_local", position_local());
    colour("materials_position_world", position_world());
    colour("materials_normal_local", normal_local());
    colour("materials_normal_world", normal_world());
    colour("materials_normal_view", normal_view());
    colour("materials_texture", texture(&uv_texture));
    colour(
        "materials_camera_projection",
        camera_projection_matrix().mul(position_local()),
    );

    let mut opacity = MeshBasicNodeMaterial::new();
    opacity.color_node = Some(Color::from_hex(0x0099ff).into());
    opacity.opacity_node = Some(texture(&uv_texture));
    opacity.transparent = true;
    show("materials_opacity", &opacity, SetupContext::default());

    let mut alpha_test = MeshBasicNodeMaterial::new();
    alpha_test.color_node = Some(texture(&uv_texture));
    alpha_test.opacity_node = Some(texture(&opacity_texture));
    alpha_test.alpha_test_node = Some(float(0.5));
    show("materials_alpha_test", &alpha_test, SetupContext::default());

    let (_grid_geometry, grid_material) = three_rs::helpers::GridHelper::parts(
        1000.0,
        40,
        Color::from_hex(0x303030),
        Color::from_hex(0x303030),
    );
    show("materials_grid", &grid_material, SetupContext::default());

    // Materials 28 and 29 of the page: `Fn( ( input ) => vec3( 0.299, 0.587,
    // 0.114 ).dot( input.color.xyz ) )` called with the texture, and the same
    // body with the texture captured and no inputs at all. Neither `Fn()` has
    // a layout, so both inline, and three emits byte-identical WGSL for the
    // two — which is why they share one pipeline. The port gets that for free:
    // an inlined `FnDef` contributes nothing to the graph but its body.
    let desaturate = inline_fn(1, three_rs::nodes::Type::F32, |args| {
        vec3(0.299, 0.587, 0.114).dot(args[0].clone().xyz())
    });
    colour(
        "materials_desaturate_fn",
        call(&desaturate, vec![texture(&uv_texture)]),
    );
    colour(
        "materials_desaturate_captured",
        vec3(0.299, 0.587, 0.114).dot(texture(&uv_texture).xyz()),
    );

    colour(
        "materials_triplanar",
        triplanar_texture(&uv_texture, None, None, float(0.01)),
    );
    colour(
        "materials_screen_uv",
        texture_uv(&uv_texture, screen_uv().flip_y()),
    );

    // Materials 30 and 31 of the page: hand-written WGSL through `wgslFn`.
    // The source text is copied through verbatim, including the example
    // file's own indentation and the whitespace a JS template literal leaves
    // after the closing brace, so these two string literals are written with
    // the same tabs three's are.
    let desaturate_wgsl = wgsl_fn(
        "
					fn desaturate( color:vec3<f32> ) -> vec3<f32> {

						let lum = vec3<f32>( 0.299, 0.587, 0.114 );

						return vec3<f32>( dot( lum, color ) );

					}
				",
        vec![],
    );
    let some_wgsl = wgsl_fn(
        "
					fn someFn( color:vec3<f32> ) -> vec3<f32> {

						return desaturate( color );

					}
				",
        vec![desaturate_wgsl],
    );
    colour(
        "materials_wgsl_include",
        call_wgsl(&some_wgsl, vec![("color", texture(&uv_texture).xyz())]),
    );

    let get_sample = wgsl_fn(
        "
					fn getWGSLTextureSample( tex: texture_2d<f32>, tex_sampler: sampler, uv:vec2<f32> ) -> vec4<f32> {

						return textureSample( tex, tex_sampler, uv ) * vec4<f32>( 0.0, 1.0, 0.0, 1.0 );

					}
				",
        vec![],
    );
    // One `texture( uvTexture )` node bound to both the texture and the
    // sampler parameter, which is how three's example writes it.
    let texture_node = texture(&uv_texture);
    colour(
        "materials_wgsl_texture",
        call_wgsl(
            &get_sample,
            vec![
                ("tex", texture_node.clone()),
                ("tex_sampler", texture_node),
                ("uv", uv()),
            ],
        ),
    );

    // Material 34 of the page: a `Loop()` used as a `colorNode`. `LoopNode`
    // is a *statement* node — its `generate()` writes the `for` and returns an
    // empty snippet — so `vec4( colorNode )` in `setupDiffuseColor()` casts
    // nothing and three emits `DiffuseColor = vec4<f32>(  );`. The loop still
    // runs, its result is still discarded, and the teapot is opaque black.
    // The port reproduces this on purpose: see docs/nodes.md §8.
    const LOOP_COUNT: usize = 10;
    let i = loop_index();
    let out = to_var(None, vec4(0.0, 0.0, 0.0, 1.0));
    let scale_i = osc_sine(time())
        .mul(0.09)
        .mul(i.to(three_rs::nodes::Type::F32));
    // `scaleI.negate()` is read twice (the right and bottom taps). `Node::Neg`
    // is not one of the kinds `needs_var` promotes, so the temp three's usage
    // counter gives it has to be asked for here.
    let scale_i_neg = to_var(None, scale_i.clone().negate());
    let tap = |offset: three_rs::nodes::NodeRef| {
        out.assign(out.add(texture_uv(&uv_texture, uv().add(offset))))
    };
    colour(
        "materials_loop",
        loop_statement(
            LOOP_COUNT,
            i.clone(),
            vec![
                tap(vec2_join(vec![scale_i.clone(), float(0.0)])),
                tap(vec2_join(vec![scale_i_neg.clone(), float(0.0)])),
                tap(vec2_join(vec![float(0.0), scale_i])),
                tap(vec2_join(vec![float(0.0), scale_i_neg])),
            ],
        ),
    );

    let mut normal = MeshBasicNodeMaterial::normal();
    normal.opacity = 0.5;
    normal.transparent = true;
    show(
        "materials_normal_material",
        &normal,
        SetupContext::default(),
    );

    // The output pass, this time with ACES filmic tone mapping.
    let mut aces = MeshBasicNodeMaterial::new();
    aces.fragment_node = Some(three_rs::materials::output_fragment_node(
        &framebuffer,
        three_rs::ToneMapping::AcesFilmic,
    ));
    show(
        "shadowmap_output_color_transform",
        &aces,
        SetupContext::default(),
    );

    // rung 10: the scene's `backgroundNode`, against
    // `handoff/scouts/rung10/m0{0,1}_*_Background.material-r186.wgsl`.
    let mut background = MeshBasicNodeMaterial::new();
    background.name = "Background.material";
    background.vertex_node = Some(three_rs::materials::background_vertex_node());
    background.side = Side::Back;
    background.color_node = Some(three_rs::materials::background_node_color_node(
        screen_uv()
            .y()
            .mix(Color::from_hex(0x66bbff), Color::from_hex(0x4466ff)),
    ));
    show("skinning_background", &background, SetupContext::default());

    // rung 10: Michelle's skinned body, against
    // `handoff/scouts/rung10/m03_vertex_Ch03_Body-r186.wgsl`.
    let diffuse = Texture::new(512, 512, Some(vec![0; 4]));
    let glossiness = Texture::new(512, 512, Some(vec![0; 4]));
    let specular_map = Texture::new(512, 512, Some(vec![0; 4]));
    let normal_tex = Texture::new(512, 512, Some(vec![0; 4]));
    let mut body = MeshBasicNodeMaterial::physical(Color::new(1.0, 1.0, 1.0), 1.0, 0.5);
    body.side = Side::Double;
    body.map = Some(diffuse);
    body.metalness_map = Some(glossiness.clone());
    body.roughness_map = Some(glossiness);
    body.specular_color_map = Some(specular_map);
    body.normal_map = Some(normal_tex);
    body.normal_scale = three_rs::math::Vector2::new(1.0, -1.0);
    body.ior = 1.45;
    show(
        "skinning_body",
        &body,
        SetupContext {
            skin: Some(three_rs::nodes::skinning::SkinEntry { bones: 65 }),
            lights: vec![
                LightDesc {
                    index: 0,
                    kind: LightKind::Point,
                    shadow_map: None,
                },
                LightDesc {
                    index: 1,
                    kind: LightKind::Ambient,
                    shadow_map: None,
                },
            ],
            ..SetupContext::default()
        },
    );

    // rung 11: `webgpu_mesh_batch`'s `MeshBasicNodeMaterial` on a
    // `BatchedMesh`, against `handoff/scouts/rung11/{vertex,fragment}-r186.wgsl`.
    // The three data textures only have to exist for `batch()` to bind them,
    // so one geometry and one coloured instance is enough to build the graph.
    let batched = three_rs::objects::BatchedMesh::new(
        webgpu_mesh_batch::COUNT,
        3 * 512,
        3 * 1024,
        webgpu_mesh_batch::batch_material(),
    );
    let batch_entry = {
        let mut object = batched.borrow_mut();
        let mesh = object.payload.batched_mesh_mut().unwrap();
        let geometry_id = mesh.add_geometry(&three_rs::box_geometry(2.0, 2.0, 2.0, 1, 1, 1));
        let instance_id = mesh.add_instance(geometry_id);
        mesh.set_color_at(instance_id, &three_rs::Color::new(1.0, 1.0, 1.0));
        mesh.batch_entry()
    };
    show(
        "mesh_batch",
        &webgpu_mesh_batch::batch_material(),
        SetupContext {
            batch: Some(batch_entry),
            ..SetupContext::default()
        },
    );

    // Rung 12: the two compute kernels and the points material that reads what
    // they wrote. The kernels are the only programs in the tree with no
    // material behind them, so they go through `build_compute` instead of
    // `show`.
    let particles = webgpu_compute_points::particles();
    show_compute(
        "compute_points_precompute_velocity",
        particles.update.on_init.as_ref().unwrap(),
    );
    show_compute("compute_points_update_particles", &particles.update);
    show(
        "compute_points_material",
        &webgpu_compute_points::material(),
        SetupContext::default(),
    );
    // The screen and inverse-matrix uniforms, and the `If( … ).ElseIf( … )`
    // arm, on their own. This is the slice of `Line2NodeMaterial.mvpLine()`
    // that needs no geometry: the trimmed segment's `If`/`ElseIf`, the
    // screen-space offset scaled by `materialLineWidth`, the divide by
    // `viewport.w / screenDPR`, and the round trip back through
    // `modelWorldMatrixInverse * cameraWorldMatrix *
    // cameraProjectionMatrixInverse`. Three's own text for it is
    // `scouts/scouts/webgpu_lines_fat/dump/m00_vertex_vertex.wgsl`.
    //
    // It is here so the five uniforms are visible in this dump — every one of
    // them is a new `UniformSource`, and a uniform that reaches the wrong
    // group, or the wrong `f32` count, is a silent wrong frame.
    let mut screen = MeshBasicNodeMaterial::new();
    let start = to_var(
        Some("start"),
        model_view_matrix().mul(vec4(0.0, 0.0, 0.0, 1.0)),
    );
    let end = to_var(
        Some("end"),
        model_view_matrix().mul(vec4(1.0, 0.0, 0.0, 1.0)),
    );
    let clip_start = to_var(None, camera_projection_matrix().mul(start.clone()));
    let clip_end = to_var(None, camera_projection_matrix().mul(end.clone()));

    let direction = to_var(
        None,
        clip_end
            .xyz()
            .div(clip_end.w())
            .xy()
            .sub(clip_start.xyz().div(clip_start.w()).xy()),
    );
    let aspect = to_var(None, viewport().z().div(viewport().w()));
    let offset = to_var(None, vec2_join(vec![direction.y(), direction.x().negate()]));

    // `If( position.y.lessThan( 0.0 ), … ).ElseIf( position.y.greaterThan( 1.0
    // ), … )` — a nested `if`/`else`, exactly as `StackNode.ElseIf()` builds it.
    let trim = if_else_if(
        position_geometry().y().less_than(float(0.0)),
        vec![offset.sub_assign(direction.clone())],
        position_geometry().y().greater_than(float(1.0)),
        vec![offset.add_assign(direction.clone())],
    );

    let scaled = offset
        .mul(material_line_width())
        .div(viewport().w().div(screen_dpr()));
    let clip = to_var(
        None,
        position_geometry()
            .y()
            .less_than(float(0.5))
            .select(clip_start.clone(), clip_end.clone()),
    );
    let shifted = clip.add(vec4_join(vec![
        scaled.x().mul(clip.w()),
        scaled.y().mul(clip.w()),
        float(0.0),
        float(0.0),
    ]));
    let back = to_var(
        None,
        model_world_matrix_inverse()
            .mul(camera_world_matrix())
            .mul(camera_projection_matrix_inverse())
            .mul(shifted),
    );

    // The vars are emitted as statements ahead of the `If`, which is the order
    // `Line2NodeMaterial` builds them in; leave them to be pulled in by their
    // first *use* and each branch re-emits its own copy.
    screen.position_node = Some(block(
        vec![
            start,
            end,
            clip_start,
            clip_end,
            direction,
            aspect,
            offset.clone(),
            trim,
            clip.clone(),
        ],
        back.xyz().div(back.w()),
    ));
    show("screen_uniforms", &screen, SetupContext::default());

    // `webgpu_lines_fat`: the real `Line2NodeMaterial`, over a two-segment
    // `LineGeometry`. Three's own text for both stages is
    // `scouts/scouts/webgpu_lines_fat/dump/m00_vertex_vertex.wgsl` and
    // `m01_fragment_fragment.wgsl`; the divergences are in `docs/nodes.md` §8.
    //
    // The vertex-buffer block below is the other half of the gate: four
    // buffers, strides 12 / 24 / 8 / 24, two of them `stepMode: instance` with
    // two views each. Two vertex buffers where three has one would draw a
    // garbage ribbon and raise nothing.
    let mut fat = LineGeometry::new();
    fat.set_positions(&[0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 1.0, 1.0, 0.0]);
    fat.set_colors(&[1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0]);
    let mut line2 = MeshBasicNodeMaterial::line2(Color::from_hex(0xffffff));
    line2.linewidth = 5.0;
    line2.vertex_colors = true;
    show(
        "line2",
        &line2,
        SetupContext {
            line_segments: Some(fat.as_segments().attributes()),
            ..SetupContext::default()
        },
    );

    // The same material with `alphaToCoverage`, which is the other branch of
    // `alphaLine()`: `smoothstep( 1 - dlen, 1 + dlen, len2 ).oneMinus()` in
    // place of the `discard`. `webgpu_lines_fat` does not take it — the
    // example's GUI does — but it is one flag away and a silent divergence if
    // it rots.
    let mut coverage = line2.clone();
    coverage.alpha_to_coverage = true;
    show(
        "line2_alpha_to_coverage",
        &coverage,
        SetupContext {
            line_segments: Some(fat.as_segments().attributes()),
            ..SetupContext::default()
        },
    );

    // `webgpu_pmrem_cubemap`: the two `PMREMGenerator` materials. Three's own
    // dump of them is `m00`/`m01` (`PMREM_cubemap`) and `m02`/`m03`
    // (`PMREM_ggx`) in the scout's `dump-pmrem_cubemap/`. The numbers baked
    // into the GGX shader are the ones a 256² source cube produces: a 768×1024
    // atlas and `lodMax = 8`.
    let hdr_cube = CubeTexture::new(
        (0..6)
            .map(|_| Image::rgba16float(4, 4, &[0u16; 4 * 4 * 4]))
            .collect(),
    );
    show(
        "pmrem_cubemap",
        &three_rs::renderer::pmrem::cubemap_material(&hdr_cube),
        SetupContext::default(),
    );
    let (ggx, _ggx_uniforms) = three_rs::renderer::pmrem::ggx_material(8, 768.0, 1024.0);
    show("pmrem_ggx", &ggx, SetupContext::default());

    // `webgpu_pmrem_test`: the third `PMREMGenerator` material, the one that
    // is the whole delta between the two examples. Three's dump of it is
    // `m01`/`m02` in the scout's `dump-pmrem_test/`. The source is 1024×512,
    // as `spot1Lux.hdr` decodes; the shader does not depend on the size, but
    // the material is built from a texture so the binding has one.
    let equirect = Texture::data_rgba16float(4, 2, &[0u16; 4 * 2 * 4]);
    show(
        "pmrem_equirect",
        &three_rs::renderer::pmrem::equirect_material(&equirect),
        SetupContext::default(),
    );

    // `scene.backgroundNode = pmremTexture( map, normalWorldGeometry,
    // uniform( 0.5 ) )` — three's `m05_fragment_fragment_Background.material`.
    // The PMREM has not been generated here, so the three cubeUV uniforms are
    // still zero; they are uniforms, so the WGSL does not depend on their
    // values, which is the whole reason `PMREMNode` holds them as uniforms
    // rather than baking them the way `_getGGXShader` does.
    let environment = PmremEnvironment::new(&hdr_cube);
    let (level, _level_cell) = uniform_settable(three_rs::nodes::Type::F32, vec![0.5]);
    let mut background = MeshBasicNodeMaterial::new();
    background.name = "Background.material";
    background.vertex_node = Some(three_rs::materials::background_vertex_node());
    background.side = Side::Back;
    background.depth_test = false;
    background.depth_write = false;
    background.color_node = Some(three_rs::materials::background_node_color_node(
        environment.sample(normal_world_geometry(), level),
    ));
    show("pmrem_background", &background, SetupContext::default());

    // The read side on a lit material: `MeshPhysicalNodeMaterial` with
    // `envMap`, which is `m06`/`m07`. The example has no lights, so the only
    // lighting is `EnvironmentNode`'s two samples.
    let mut sphere = MeshBasicNodeMaterial::physical(Color::new(1.0, 1.0, 1.0), 0.2, 0.6);
    sphere.pmrem_env = Some(environment.handle());
    show("pmrem_physical", &sphere, SetupContext::default());

    // `webgpu_pmrem_test`'s background: `scene.background = radianceMap`,
    // which is `Background.update()`'s node branch with
    // `NodeManager.getBackgroundNode()`'s `pmremTexture( background )` inside
    // it and `backgroundRotation` / `backgroundBlurriness` supplied by the
    // node context. Three's dump of it is `m05`/`m06` in `dump-pmrem_test/`.
    // Same atlas read as `pmrem_background` above, a different uv and level.
    let mut pmrem_background = MeshBasicNodeMaterial::new();
    pmrem_background.name = "Background.material";
    pmrem_background.vertex_node = Some(three_rs::materials::background_vertex_node());
    pmrem_background.side = Side::Back;
    pmrem_background.depth_test = false;
    pmrem_background.depth_write = false;
    pmrem_background.color_node = Some(three_rs::materials::background_pmrem_color_node(
        &environment.handle(),
    ));
    show(
        "pmrem_test_background",
        &pmrem_background,
        SetupContext::default(),
    );

    // And the lit material with a light in the graph: the example's
    // intensity-zero `DirectionalLight` is why `m08` carries a directional
    // block at all. The colour is the row-0 white metal.
    let mut lit = MeshBasicNodeMaterial::physical(Color::new(1.0, 1.0, 1.0), 0.0, 1.0);
    lit.pmrem_env = Some(environment.handle());
    show(
        "pmrem_test_physical",
        &lit,
        SetupContext {
            lights: vec![LightDesc {
                index: 0,
                kind: LightKind::Directional,
                shadow_map: None,
            }],
            ..SetupContext::default()
        },
    );
}
