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

use three_rs::addons::controls::OrbitControls;
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
    /// The page's `controls`.
    pub controls: OrbitControls,
}

fn examples_dir() -> std::path::PathBuf {
    three_rs::testing::three_js_dir().join("examples")
}

pub fn init() -> App {
    let mut camera = PerspectiveCamera::new(70.0, INNER_WIDTH / INNER_HEIGHT, 0.1, 100.0);
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

    // `const controls = new OrbitControls( camera, renderer.domElement );`
    // The page never calls `controls.update()` itself, but the constructor's
    // own `update()` aims the camera at the default `( 0, 0, 0 )` target —
    // which is where `animate()`'s `camera.lookAt( scene.position )` aims it
    // anyway, so the graded frame is unchanged. With no pointer events the
    // distance limits never bite.
    let mut controls = OrbitControls::new(&mut camera);
    // The canvas the example renders at, standing in for the element's
    // `clientWidth` / `clientHeight`.
    controls.set_element_size(INNER_WIDTH, INNER_HEIGHT);
    controls.min_distance = 1.5;
    controls.max_distance = 6.0;

    App {
        renderer,
        scene,
        camera,
        controls,
    }
}

/// The page's `animate()`.
pub fn animate(app: &mut App) {
    // The three `backgroundRotation` toggles and `syncMaterial` are all
    // `false`, so the whole body of `animate()` before the `lookAt` is skipped.
    app.camera.look_at(&Vector3::ZERO);
    app.renderer.render(&mut app.scene, &mut app.camera);
}

/// The page's `onWindowResize()`.
///
/// The rung harness never calls this — the graded frame is always
/// 800 x 500 — but the viewer and the browser shell do, so the example
/// owns its own reaction to a resized canvas instead of the host
/// guessing at one.
pub fn resize(app: &mut App, width: f64, height: f64) {
    app.camera.aspect = width / height;
    app.camera.update_projection_matrix();

    app.renderer.set_size(width, height);
}

/// The example's controls, for a host that has a pointer. `None` when the
/// page creates none — the signature is the same for every example so the
/// viewer and the browser shell can drive any of them through one call.
pub fn controls(app: &mut App) -> Option<&mut OrbitControls> {
    Some(&mut app.controls)
}

/// The controls and the camera at once, which every one of the controls'
/// event handlers needs: the JS holds the camera as `this.object` and Rust
/// cannot, so `pointer_move` and the rest take it as an argument.
///
/// They are two fields of the same `App`, so borrowing both is sound — but
/// only this module can say so; a host holding `&mut App` and calling
/// [`controls`] and then reaching for the camera cannot. Hence the pair.
pub fn controls_and_camera(app: &mut App) -> Option<(&mut OrbitControls, &mut PerspectiveCamera)> {
    Some((&mut app.controls, &mut app.camera))
}

fn main() {
    // Pin both clocks to zero, as three.js' `test/e2e/deterministic-injection.js`
    // does to the page, so that the frame this writes is the frame the rung
    // grades no matter how long `init()` took.
    three_rs::testing::pin_time(Some(0.0));
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
