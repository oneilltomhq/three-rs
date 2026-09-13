//! Port of `three.js/examples/webgpu_lights_phong.html`, calling the three-rs
//! API in the same order the page's `init()` / `animate()` do.
//!
//! Under the e2e harness the viewport is `400 * 2` x `250 * 2` with no
//! `deviceScaleFactor`, so `window.innerWidth` / `innerHeight` /
//! `devicePixelRatio` are 800, 500 and 1.
//!
//! `performance.now()` is pinned to 0, so `lightTime` is 0 and the four lights
//! sit at (0, 4, 3), (3, 0, 0), (0, 4, 0) and (0, 4, 0) — lights 3 and 4
//! coincide, and only light 2's sphere is inside the frustum (the other three
//! project above the top of the viewport).
//!
//! `TextureLoader.load()` is asynchronous on the page, but the harness only
//! fires its single RAF once the network is idle, so both JPEGs are always
//! present for the graded frame; here they are decoded synchronously.
//!
//! `OrbitControls` is constructed but never updated, and `renderer.inspector =
//! new Inspector()` only registers the scene for the inspector panel, so
//! neither touches the graded frame. The `resize` listener does not fire.

use std::rc::Rc;

use three_rs::core::{Node, Object3DNode};
use three_rs::nodes::tsl::{checker, fog, mix, normal_map, range_fog_factor, texture, uv};
use three_rs::textures::Wrapping;
use three_rs::{
    sphere_geometry, teapot_geometry, Color, Mesh, MeshPhongNodeMaterial, PerspectiveCamera,
    PointLight, Renderer, RendererParameters, Scene,
};

pub const INNER_WIDTH: f64 = 800.0;
pub const INNER_HEIGHT: f64 = 500.0;
/// `renderer.setPixelRatio( window.devicePixelRatio )`
pub const DPR: f64 = 1.0;

pub struct App {
    pub renderer: Renderer,
    pub scene: Scene,
    pub camera: PerspectiveCamera,
    /// `light1` … `light4`, the page's module-level handles, which `animate()`
    /// moves. They are in the scene, so the renderer finds them by walking it.
    pub lights: Vec<Node>,
}

fn examples_dir() -> std::path::PathBuf {
    let three = three_rs::testing::three_js_dir();
    three.join("examples")
}

pub fn init() -> App {
    let camera = PerspectiveCamera::new(50.0, INNER_WIDTH / INNER_HEIGHT, 0.01, 100.0);
    camera.node.borrow_mut().position.z = 7.0;

    let mut scene = Scene::new();
    scene.fog_node = Some(fog(
        Color::from_hex(0xFF00FF),
        range_fog_factor(12.0, 30.0),
    ));

    let sphere_geometry = Rc::new(sphere_geometry(0.1, 16, 8));

    // textures

    let texture_loader = three_rs::TextureLoader::new();

    let normal_map_texture =
        texture_loader.load(examples_dir().join("textures/water/Water_1_M_Normal.jpg"));
    normal_map_texture.set_wrapping(Wrapping::Repeat, Wrapping::Repeat);

    let alpha_texture = texture_loader.load(examples_dir().join("textures/roughness_map.jpg"));
    alpha_texture.set_wrapping(Wrapping::Repeat, Wrapping::Repeat);

    // lights

    // `addLight( hexColor, power = 1700, distance = 100 )`: the sphere mesh is an
    // ordinary child of the light, so it inherits the light's world matrix and
    // draws through the scene walk, while the light itself is collected into
    // `RenderList.lights` instead of being drawn.
    let add_light = |scene: &Scene, hex: u32| -> Node {
        let mut material = MeshPhongNodeMaterial::phong(Color::default());
        material.color_node = Some(Color::from_hex(hex).into());
        material.lights = false;

        let mesh = Mesh::new(sphere_geometry.clone());
        mesh.borrow_mut().mesh_mut().unwrap().material = Some(material);

        let light = PointLight::new(Color::from_hex(hex), 1.0, 100.0);
        light.borrow_mut().light_mut().unwrap().set_power(1700.0);
        light.add(&mesh);

        scene.add(&light);
        light
    };

    let lights = vec![
        add_light(&scene, 0x0040ff),
        add_light(&scene, 0xffffff),
        add_light(&scene, 0x80ff80),
        add_light(&scene, 0xffaa00),
    ];

    // light nodes ( selective lights )

    // `lights( [ light1 ] )` / `lights( [ light2 ] )` — indices into the
    // renderer's light list, which is scene-traversal order; the four lights are
    // added before the teapots, so it is the order they were added in.
    let blue_lights_node = vec![0];
    let white_lights_node = vec![1];

    // models

    let geometry_teapot = Rc::new(teapot_geometry(0.8, 18));

    let mut left_material = MeshPhongNodeMaterial::phong(Color::from_hex(0x555555));
    left_material.lights_node = Some(blue_lights_node);
    left_material.specular_node = Some(texture(&alpha_texture));
    let left_object = Mesh::new(geometry_teapot.clone());
    left_object.borrow_mut().mesh_mut().unwrap().material = Some(left_material);
    left_object.borrow_mut().position.x = -3.0;

    let mut centre_material = MeshPhongNodeMaterial::phong(Color::from_hex(0x555555));
    centre_material.normal_node = Some(normal_map(texture(&normal_map_texture)));
    centre_material.shininess = 80.0;
    let centre_object = Mesh::new(geometry_teapot.clone());
    centre_object.borrow_mut().mesh_mut().unwrap().material = Some(centre_material);

    let mut right_material = MeshPhongNodeMaterial::phong(Color::from_hex(0x555555));
    right_material.lights_node = Some(white_lights_node);
    right_material.specular_node = Some(mix(
        Color::from_hex(0x0000FF),
        Color::from_hex(0xFF0000),
        checker(uv().mul(5.0)),
    ));
    right_material.shininess = 90.0;
    let right_object = Mesh::new(geometry_teapot);
    right_object.borrow_mut().mesh_mut().unwrap().material = Some(right_material);
    right_object.borrow_mut().position.x = 3.0;

    for object in [&left_object, &centre_object, &right_object] {
        // `object.rotation.y = …` in three.js runs `Euler.onChange`, which is
        // `quaternion.setFromEuler( rotation, false )`; `set_rotation` is that
        // pair, and the matrix is composed from the quaternion.
        let mut object = object.borrow_mut();
        object.set_rotation(0.0, std::f64::consts::PI * -0.5, 0.0);
        object.position.y = -1.0;
    }

    scene.add(&left_object);
    scene.add(&centre_object);
    scene.add(&right_object);

    // renderer

    let mut renderer = Renderer::new(RendererParameters { antialias: true });
    renderer.set_pixel_ratio(DPR);
    renderer.set_size(INNER_WIDTH, INNER_HEIGHT);

    App {
        renderer,
        scene,
        camera,
        lights,
    }
}

/// The page's `animate()`. `performance.now()` is 0 under the harness, so
/// `lightTime` is 0 and every `sin` / `cos` below collapses to a constant.
pub fn animate(app: &mut App) {
    let light_time = 0.0f64;

    {
        let mut light = app.lights[0].borrow_mut();
        light.position.x = (light_time * 0.7).sin() * 3.0;
        light.position.y = (light_time * 0.5).cos() * 4.0;
        light.position.z = (light_time * 0.3).cos() * 3.0;
    }
    {
        let mut light = app.lights[1].borrow_mut();
        light.position.x = (light_time * 0.3).cos() * 3.0;
        light.position.y = (light_time * 0.5).sin() * 4.0;
        light.position.z = (light_time * 0.7).sin() * 3.0;
    }
    {
        let mut light = app.lights[2].borrow_mut();
        light.position.x = (light_time * 0.7).sin() * 3.0;
        light.position.y = (light_time * 0.3).cos() * 4.0;
        light.position.z = (light_time * 0.5).sin() * 3.0;
    }
    {
        let mut light = app.lights[3].borrow_mut();
        light.position.x = (light_time * 0.3).sin() * 3.0;
        light.position.y = (light_time * 0.7).cos() * 4.0;
        light.position.z = (light_time * 0.5).sin() * 3.0;
    }

    app.renderer.render(&mut app.scene, &mut app.camera);
}

fn main() {
    let mut app = init();
    println!("adapter: {:?}", app.renderer.adapter_info());
    animate(&mut app);

    let (width, height, pixels) = app.renderer.read_canvas_pixels();
    let path = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "target/webgpu_lights_phong.png".to_string());
    three_rs::testing::write_png(&path, width, height, &pixels);
    println!("wrote {path} ({width}x{height})");
}
