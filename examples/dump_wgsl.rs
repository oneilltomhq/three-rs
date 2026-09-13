//! Prints the WGSL the node system generates for every material in rungs 1–4,
//! so it can be diffed against three.js' own dumped output.

use three_rs::materials::{setup, MeshBasicNodeMaterial, SetupContext, Side};
use three_rs::math::Color;
use three_rs::nodes::tsl::*;
use three_rs::nodes::NodeBuilder;
use three_rs::textures::{CubeTexture, DepthTexture, Image, Texture};

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
    println!("########## {label} — attributes {:?}", program.attributes);
    for (i, g) in program.groups.iter().enumerate() {
        println!("  group {i}:");
        for (b, d) in g.iter().enumerate() {
            println!("    {b}: {d:?}");
        }
    }
}

use three_rs::lights::LightKind;
use three_rs::materials::phong::LightDesc;
use three_rs::nodes::materialx::{mx_fractal_noise_float, mx_fractal_noise_vec3};

fn main() {
    // rung 1: the scene override material and the depth-texture quad.
    show(
        "basic",
        &MeshBasicNodeMaterial::new(),
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
            lights: Vec::new(),
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
    out.fragment_node = Some(three_rs::materials::output_fragment_node(&framebuffer, three_rs::ToneMapping::None));
    show("output_color_transform", &out, SetupContext::default());

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
    masking.fragment_node = Some(three_rs::materials::render_output(compose, three_rs::ToneMapping::None));
    masking.vertex_node = Some(three_rs::materials::quad_vertex_node());
    show("masking_quad", &masking, SetupContext::default());

    // rung 5: the three teapots and the light spheres, against
    // `target/dumps/webgpu_lights_phong/`.
    let fog = fog(Color::from_hex(0xFF00FF), range_fog_factor(12.0, 30.0));
    let four = SetupContext {
        lights: (0..4).map(|index| LightDesc { index, kind: LightKind::Point, shadow_map: None }).collect(),
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

    let mut sphere = MeshBasicNodeMaterial::phong(Color::new(1.0, 1.0, 1.0));
    sphere.lights = false;
    sphere.color_node = Some(Color::from_hex(0x0040ff).into());
    show_fog("phong_light_sphere", &sphere, four.clone(), Some(&fog));
    // rung 7: `webgpu_shadowmap`, against
    // `handoff/scouts/rung7/m0*-r186.wgsl`. Light order is the scene order:
    // ambient, spot, directional.
    let shadow_fog = three_rs::nodes::tsl::fog(Color::from_hex(0x222244), range_fog_factor(50.0, 100.0));

    let mut background = MeshBasicNodeMaterial::new();
    background.color_node = Some(three_rs::materials::background_node_color_node(
        Color::from_hex(0x222244),
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
            LightDesc { index: 0, kind: LightKind::Ambient, shadow_map: None },
            LightDesc {
                index: 1,
                kind: LightKind::Spot,
                shadow_map: shadows.then(|| spot_map.clone()),
            },
            LightDesc {
                index: 2,
                kind: LightKind::Directional,
                shadow_map: shadows.then(|| dir_map.clone()),
            },
        ],
        ..SetupContext::default()
    };

    // The pillars and the torus knot share one `MeshPhongNodeMaterial`; only
    // the knot's clone carries the mask, and only the ground receives shadows.
    let mut pillars = MeshBasicNodeMaterial::phong(Color::from_hex(0x999999));
    pillars.shininess = 0.0;
    pillars.specular = Color::from_hex(0x222222);
    show_fog("shadowmap_phong_pillars", &pillars, lit(false), Some(&shadow_fog));

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
    show_fog("shadowmap_phong_ground", &ground, lit(true), Some(&shadow_fog));

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

    // The output pass, this time with ACES filmic tone mapping.
    let mut aces = MeshBasicNodeMaterial::new();
    aces.fragment_node = Some(three_rs::materials::output_fragment_node(
        &framebuffer,
        three_rs::ToneMapping::AcesFilmic,
    ));
    show("shadowmap_output_color_transform", &aces, SetupContext::default());
}
