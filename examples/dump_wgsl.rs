//! Prints the WGSL the node system generates for every material in rungs 1–4,
//! so it can be diffed against three.js' own dumped output.

use three_rs::materials::{setup, LightKind, MeshBasicNodeMaterial, SetupContext, Side};
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
            light_count: 0,
            ..SetupContext::default()
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
    out.fragment_node = Some(three_rs::materials::output_fragment_node(&framebuffer));
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
    masking.fragment_node = Some(three_rs::materials::render_output(compose));
    masking.vertex_node = Some(three_rs::materials::quad_vertex_node());
    show("masking_quad", &masking, SetupContext::default());

    // rung 5: the three teapots and the light spheres, against
    // `target/dumps/webgpu_lights_phong/`.
    let fog = fog(Color::from_hex(0xFF00FF), range_fog_factor(12.0, 30.0));
    let four = SetupContext {
        light_count: 4,
        ..SetupContext::default()
    };

    let normal_map_texture = Texture::new(512, 512, Some(vec![0; 4]));
    let alpha_texture = Texture::new(512, 512, Some(vec![0; 4]));

    let grey = Color::from_hex(0x555555);

    let mut left = MeshBasicNodeMaterial::phong(grey);
    left.lights_node = Some(vec![0]);
    left.specular_node = Some(texture(&alpha_texture));
    show_fog("phong_left", &left, four, Some(&fog));

    let mut centre = MeshBasicNodeMaterial::phong(grey);
    centre.normal_node = Some(normal_map(texture(&normal_map_texture)));
    centre.shininess = 80.0;
    show_fog("phong_centre", &centre, four, Some(&fog));

    let mut right = MeshBasicNodeMaterial::phong(grey);
    right.lights_node = Some(vec![1]);
    right.specular_node = Some(mix(
        Color::from_hex(0x0000FF),
        Color::from_hex(0xFF0000),
        checker(uv().mul(5.0)),
    ));
    right.shininess = 90.0;
    show_fog("phong_right", &right, four, Some(&fog));

    // rung 8: the four physical materials, against
    // `handoff/scouts/rung8/MeshStandardMaterial_*`.
    let two_lights = SetupContext {
        light_count: 2,
        light_kinds: {
            let mut kinds = [LightKind::Point; three_rs::materials::MAX_LIGHTS];
            kinds[1] = LightKind::Hemisphere;
            kinds
        },
        ..SetupContext::default()
    };

    let mut bulb = MeshBasicNodeMaterial::standard(Color::from_hex(0x000000), 1.0, 0.0);
    bulb.emissive = Color::from_hex(0xffffee);
    bulb.emissive_intensity = 1.0;
    show("standard_bulb", &bulb, two_lights);

    let hardwood = Texture::new(1024, 1024, Some(vec![0; 4]));
    let hardwood_bump = Texture::new(1024, 1024, Some(vec![0; 4]));
    let hardwood_roughness = Texture::new(1024, 1024, Some(vec![0; 4]));
    let mut floor = MeshBasicNodeMaterial::standard(Color::new(1.0, 1.0, 1.0), 0.8, 0.2);
    floor.map = Some(hardwood.clone());
    floor.bump_map = Some(hardwood_bump.clone());
    floor.roughness_map = Some(hardwood_roughness.clone());
    show("standard_floor", &floor, two_lights);

    let brick = Texture::new(512, 512, Some(vec![0; 4]));
    let brick_bump = Texture::new(512, 512, Some(vec![0; 4]));
    let mut cube_material = MeshBasicNodeMaterial::standard(Color::new(1.0, 1.0, 1.0), 0.7, 0.2);
    cube_material.map = Some(brick.clone());
    cube_material.bump_map = Some(brick_bump.clone());
    show("standard_cube", &cube_material, two_lights);

    let earth = Texture::new(2048, 1024, Some(vec![0; 4]));
    let earth_specular = Texture::new(2048, 1024, Some(vec![0; 4]));
    let mut ball = MeshBasicNodeMaterial::standard(Color::new(1.0, 1.0, 1.0), 0.5, 1.0);
    ball.map = Some(earth.clone());
    ball.metalness_map = Some(earth_specular.clone());
    show("standard_ball", &ball, two_lights);

    let mut sphere = MeshBasicNodeMaterial::phong(Color::new(1.0, 1.0, 1.0));
    sphere.lights = false;
    sphere.color_node = Some(Color::from_hex(0x0040ff).into());
    show_fog("phong_light_sphere", &sphere, four, Some(&fog));
}
