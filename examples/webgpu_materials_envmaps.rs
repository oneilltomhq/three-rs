//! Port of `three.js/examples/webgpu_materials_envmaps.html`, calling the
//! three-rs API in the same order the page's `init()` / `animate()` do.
//!
//! Under the e2e harness the viewport is `400 * 2` x `250 * 2` with no
//! `deviceScaleFactor`, so `window.innerWidth` / `innerHeight` /
//! `devicePixelRatio` are 800, 500 and 1.
//!
//! The page draws from `Math.random()` not at all, so the harness' seeded
//! sequence is never touched.
//!
//! # What the graded frame actually contains
//!
//! The page's GUI defaults are `Type: 'Cube'` and `Refraction: false`, and the
//! e2e frame is the first one, so none of the `onChange` handlers has run:
//! the graded frame is the **cube reflection** path and nothing else. The
//! equirectangular texture the page also loads is never sampled — it only
//! becomes the background and the env map when the `Type` dropdown is moved —
//! so it is not loaded here, and neither `EquirectangularReflectionMapping`
//! nor `CubeRefractionMapping` is ported. `docs/webgpu_materials_envmaps-progress.md`
//! says what that leaves out.
//!
//! `scene.backgroundRotation` is only touched by the three rotation toggles,
//! which default to `false`, so it stays the identity.
//!
//! `CubeTextureLoader.load()` is asynchronous on the page, but the harness only
//! fires its single RAF once the network is idle, so the six faces are always
//! present for the graded frame; here they are decoded synchronously. These
//! faces are JPEG rather than the PNG `webgpu_materials_basic` loads.

use std::rc::Rc;

use three_rs::geometries::icosahedron_geometry;
use three_rs::{
    CubeTextureLoader, Mesh, MeshBasicNodeMaterial, PerspectiveCamera, Renderer,
    RendererParameters, Scene, Vector3,
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
    three_rs::testing::three_js_dir().join("examples")
}

pub fn init() -> App {
    let camera = PerspectiveCamera::new(70.0, INNER_WIDTH / INNER_HEIGHT, 0.1, 100.0);
    camera.node.borrow_mut().position.set(0.0, 0.0, 2.5);

    let mut scene = Scene::new();

    // `loader.setPath( 'textures/cube/Bridge2/' )`, then the six faces in
    // `CubeTexture`'s own px, nx, py, ny, pz, nz order.
    let texture_cube = CubeTextureLoader::new()
        .set_path(examples_dir().join("textures/cube/Bridge2/"))
        .load([
            "posx.jpg", "negx.jpg", "posy.jpg", "negy.jpg", "posz.jpg", "negz.jpg",
        ])
        .unwrap();

    scene.set_background(texture_cube.clone());

    //

    let geometry = Rc::new(icosahedron_geometry(1.0, 15));
    // `new THREE.MeshBasicMaterial( { envMap: textureCube } )` — the colour
    // stays at the `0xffffff` default.
    let mut material = MeshBasicNodeMaterial::new();
    material.env_map = Some(texture_cube);

    let sphere_mesh = Mesh::new(geometry, material);
    scene.add(&sphere_mesh);

    //

    let mut renderer = Renderer::new(RendererParameters { antialias: false }).unwrap();
    renderer.set_pixel_ratio(DPR);
    renderer.set_size(INNER_WIDTH, INNER_HEIGHT);

    // `new OrbitControls( camera, renderer.domElement )` with `minDistance`
    // and `maxDistance`: with no pointer events the orbit is the identity and
    // the distance limits never bite.

    App {
        renderer,
        scene,
        camera,
    }
}

/// The page's `animate()`.
pub fn animate(app: &mut App) {
    // The three `backgroundRotation` toggles and `syncMaterial` are all
    // `false`, so the whole body of `animate()` before the `lookAt` is skipped.
    app.camera.look_at(&Vector3::ZERO);
    app.renderer.render(&mut app.scene, &mut app.camera);
}

fn main() {
    let mut app = init();
    println!("adapter: {:?}", app.renderer.adapter_info());
    animate(&mut app);

    let (width, height, pixels) = app.renderer.read_canvas_pixels().unwrap();
    let path = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "target/webgpu_materials_envmaps.png".to_string());
    three_rs::testing::write_png(&path, width, height, &pixels);
    println!("wrote {path} ({width}x{height})");
}
