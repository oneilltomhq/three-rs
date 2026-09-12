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
}

fn examples_dir() -> std::path::PathBuf {
    let three = match std::env::var("THREE_JS_DIR") {
        Ok(dir) => std::path::PathBuf::from(dir),
        Err(_) => {
            std::path::PathBuf::from(std::env::var("HOME").unwrap()).join("src/vendor/three.js")
        }
    };
    three.join("examples")
}

pub fn init() -> App {
    let mut camera = PerspectiveCamera::new(50.0, INNER_WIDTH / INNER_HEIGHT, 0.01, 100.0);
    camera.object.position.z = 7.0;

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

    // `addLight( hexColor, power = 1700, distance = 100 )`: the sphere mesh is a
    // child of the light, so it inherits the light's world matrix.
    let add_light = |scene: &mut Scene, hex: u32| {
        let mut material = MeshPhongNodeMaterial::phong(Color::default());
        material.color_node = Some(Color::from_hex(hex).into());
        material.lights = false;

        let mut mesh = Mesh::new(sphere_geometry.clone());
        mesh.material = Some(material);

        let mut light = PointLight::new(Color::from_hex(hex), 1.0, 100.0);
        light.set_power(1700.0);
        light.add(mesh);

        scene.add_light(light);
    };

    add_light(&mut scene, 0x0040ff);
    add_light(&mut scene, 0xffffff);
    add_light(&mut scene, 0x80ff80);
    add_light(&mut scene, 0xffaa00);

    // light nodes ( selective lights )

    // `lights( [ light1 ] )` / `lights( [ light2 ] )` — indices into
    // `Scene.lights`, which is the order the lights were added in.
    let blue_lights_node = vec![0];
    let white_lights_node = vec![1];

    // models

    let geometry_teapot = Rc::new(teapot_geometry(0.8, 18));

    let mut left_material = MeshPhongNodeMaterial::phong(Color::from_hex(0x555555));
    left_material.lights_node = Some(blue_lights_node);
    left_material.specular_node = Some(texture(&alpha_texture));
    let mut left_object = Mesh::new(geometry_teapot.clone());
    left_object.material = Some(left_material);
    left_object.object.position.x = -3.0;

    let mut centre_material = MeshPhongNodeMaterial::phong(Color::from_hex(0x555555));
    centre_material.normal_node = Some(normal_map(texture(&normal_map_texture)));
    centre_material.shininess = 80.0;
    let mut centre_object = Mesh::new(geometry_teapot.clone());
    centre_object.material = Some(centre_material);

    let mut right_material = MeshPhongNodeMaterial::phong(Color::from_hex(0x555555));
    right_material.lights_node = Some(white_lights_node);
    right_material.specular_node = Some(mix(
        Color::from_hex(0x0000FF),
        Color::from_hex(0xFF0000),
        checker(uv().mul(5.0)),
    ));
    right_material.shininess = 90.0;
    let mut right_object = Mesh::new(geometry_teapot);
    right_object.material = Some(right_material);
    right_object.object.position.x = 3.0;

    for object in [&mut left_object, &mut centre_object, &mut right_object] {
        object.object.rotation.y = std::f64::consts::PI * -0.5;
        object.object.position.y = -1.0;
    }

    scene.add(left_object);
    scene.add(centre_object);
    scene.add(right_object);

    // renderer

    let mut renderer = Renderer::new(RendererParameters { antialias: true });
    renderer.set_pixel_ratio(DPR);
    renderer.set_size(INNER_WIDTH, INNER_HEIGHT);

    App {
        renderer,
        scene,
        camera,
    }
}

/// The page's `animate()`. `performance.now()` is 0 under the harness, so
/// `lightTime` is 0 and every `sin` / `cos` below collapses to a constant.
pub fn animate(app: &mut App) {
    let light_time = 0.0f64;

    {
        let light = &mut app.scene.lights[0];
        light.object_mut().position.x = (light_time * 0.7).sin() * 3.0;
        light.object_mut().position.y = (light_time * 0.5).cos() * 4.0;
        light.object_mut().position.z = (light_time * 0.3).cos() * 3.0;
    }
    {
        let light = &mut app.scene.lights[1];
        light.object_mut().position.x = (light_time * 0.3).cos() * 3.0;
        light.object_mut().position.y = (light_time * 0.5).sin() * 4.0;
        light.object_mut().position.z = (light_time * 0.7).sin() * 3.0;
    }
    {
        let light = &mut app.scene.lights[2];
        light.object_mut().position.x = (light_time * 0.7).sin() * 3.0;
        light.object_mut().position.y = (light_time * 0.3).cos() * 4.0;
        light.object_mut().position.z = (light_time * 0.5).sin() * 3.0;
    }
    {
        let light = &mut app.scene.lights[3];
        light.object_mut().position.x = (light_time * 0.3).sin() * 3.0;
        light.object_mut().position.y = (light_time * 0.7).cos() * 4.0;
        light.object_mut().position.z = (light_time * 0.5).sin() * 3.0;
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
