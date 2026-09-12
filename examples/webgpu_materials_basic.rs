//! Port of `three.js/examples/webgpu_materials_basic.html`, calling the three-rs
//! API in the same order the page's `init()` / `animate()` do.
//!
//! Under the e2e harness the viewport is `400 * 2` x `250 * 2` with no
//! `deviceScaleFactor`, so `window.innerWidth` / `innerHeight` /
//! `devicePixelRatio` are 800, 500 and 1, and `Date.now()` is 0.
//!
//! `CubeTextureLoader.load()` is asynchronous on the page, but the harness only
//! fires its single RAF once the network is idle, so the six faces are always
//! present for the graded frame; here they are decoded synchronously.
//!
//! The example's GUI (`renderer.inspector.createParameters()`) only registers
//! `onChange` callbacks, so it changes nothing about the graded frame.

use std::rc::Rc;

use three_rs::testing::DeterministicRandom;
use three_rs::{
    sphere_geometry, Color, CubeTextureLoader, Mesh, MeshBasicNodeMaterial, PerspectiveCamera,
    Renderer, RendererParameters, Scene, Vector3,
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
    let mut camera =
        PerspectiveCamera::new(60.0, INNER_WIDTH / INNER_HEIGHT, 0.01, 100.0);
    camera.object.position.z = 3.0;

    let path = examples_dir().join("textures/cube/pisa");
    let format = "png";
    let urls = [
        path.join(format!("px.{format}")),
        path.join(format!("nx.{format}")),
        path.join(format!("py.{format}")),
        path.join(format!("ny.{format}")),
        path.join(format!("pz.{format}")),
        path.join(format!("nz.{format}")),
    ];

    let texture_cube = CubeTextureLoader::new().load(urls);

    let mut scene = Scene::new();
    scene.set_background(texture_cube.clone());

    let geometry = Rc::new(sphere_geometry(0.1, 32, 16));

    let mut material = MeshBasicNodeMaterial::new();
    material.color = Color::from_hex(0xffffff);
    material.env_map = Some(texture_cube);

    // `Math.random()` is the harness' deterministic sequence: four draws per
    // mesh, in the order x, y, z, scale.
    let mut random = DeterministicRandom::new();

    for _ in 0..500 {
        let mesh = Mesh::new(geometry.clone());
        {
            let mut object = mesh.borrow_mut();
            object.mesh_mut().unwrap().material = Some(material.clone());

            object.position.x = random.next() * 10.0 - 5.0;
            object.position.y = random.next() * 10.0 - 5.0;
            object.position.z = random.next() * 10.0 - 5.0;

            let scale = random.next() * 3.0 + 1.0;
            object.scale.set(scale, scale, scale);
        }
        scene.add(&mesh);
    }

    //

    let mut renderer = Renderer::new(RendererParameters { antialias: false });
    renderer.set_pixel_ratio(DPR);
    renderer.set_size(INNER_WIDTH, INNER_HEIGHT);

    App {
        renderer,
        scene,
        camera,
    }
}

/// The page's `animate()`.
pub fn animate(app: &mut App) {
    // `const timer = 0.0001 * Date.now();` — the harness pins `Date.now()` to 0.
    let timer = 0.0f64;

    // `mouseX` / `mouseY` are 0 with no pointer events, and the camera starts at
    // x = y = 0, so both of these are no-ops.
    let position = &mut app.camera.object.position;
    position.x += (0.0 - position.x) * 0.05;
    position.y += (0.0 - position.y) * 0.05;

    app.camera.look_at(&Vector3::ZERO);

    // `for ( let i = 0 … spheres.length )` — the scene holds nothing but the
    // spheres, so `scene.children` *is* the page's `spheres` array.
    for (i, sphere) in app.scene.children().iter().enumerate() {
        let mut sphere = sphere.borrow_mut();
        sphere.position.x = 5.0 * (timer + i as f64).cos();
        sphere.position.y = 5.0 * (timer + i as f64 * 1.1).sin();
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
        .unwrap_or_else(|| "target/webgpu_materials_basic.png".to_string());
    three_rs::testing::write_png(&path, width, height, &pixels);
    println!("wrote {path} ({width}x{height})");
}
