//! Prints the WGSL the node system generates for every material in rungs 1–4,
//! so it can be diffed against three.js' own dumped output.

use three_rs::materials::{setup, MeshBasicNodeMaterial, SetupContext, Side};
use three_rs::math::Color;
use three_rs::nodes::tsl::*;
use three_rs::nodes::NodeBuilder;
use three_rs::textures::{CubeTexture, DepthTexture, Image, Texture};

fn show(label: &str, material: &MeshBasicNodeMaterial, ctx: SetupContext) {
    let flow = setup(material, &ctx);
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
}
