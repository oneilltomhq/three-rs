//! Port of `three.js/examples/webgpu_loader_gltf_iridescence.html`, calling the
//! three-rs API in the same order the page's `init()` does.
//!
//! Under the e2e harness the viewport is `400 * 2` x `250 * 2` with no
//! `deviceScaleFactor`, so `window.innerWidth` / `innerHeight` /
//! `devicePixelRatio` are 800, 500 and 1. The page draws from `Math.random()`
//! not at all.
//!
//! # What this page is
//!
//! `IridescenceLamp.glb` is the Khronos sample asset for
//! `KHR_materials_iridescence`, and this page is the only graded example that
//! reaches `evalIridescence`. The lamp is three meshes sharing one base colour
//! and one occlusion/roughness/metalness map:
//!
//! * `lamp` — no extension at all, a plain glTF PBR material;
//! * `lamp_iridescence` — `iridescenceFactor` 1, IOR 1.8, thickness
//!   `[ 485, 515 ]` nm through the green channel of
//!   `IridescenceLamp_Iridescence.png`;
//! * `lamp_transmission` — the same at IOR 1.67 and `[ 395, 405 ]` nm, plus
//!   `KHR_materials_transmission` (factor 1) and `KHR_materials_volume`
//!   (thickness 0.005), so it is drawn in the transmission pass over the
//!   copied opaque frame.
//!
//! Four things are easy to lose:
//!
//! * **The scene has no lights.** Every lit pixel comes from
//!   `scene.environment`, so `PhysicalLightingModel.direct()` is never called
//!   and the iridescent branch of `BRDF_GGX` never reaches a shader. All the
//!   iridescence on screen arrives through the iridescent F0 in
//!   `computeMultiscattering` — `docs/nodes.md` §95.
//! * **The environment is a Radiance `.hdr`,** not an UltraHDR JPEG: the
//!   page takes [`HdrLoader`] straight to a PMREM and to a cube background.
//! * **`backgroundBlurriness` is 0,** so the skybox is the sharp cube
//!   `CubeMapNode` converts the equirect into, not a PMREM read.
//! * **The camera turns.** `controls.autoRotate` is on, and with `deltaTime`
//!   null every `controls.update()` turns the camera by
//!   `2π/3600 * autoRotateSpeed` (−0.05°) about the target. Under three's
//!   deterministic `requestAnimationFrame` the page calls `update()` three
//!   times before the screenshot: once in `init()` after `autoRotate` is set,
//!   once in the `render()` `init()` ends with, and once in the single
//!   animation-loop frame. The constructor's own `update()` runs before
//!   `autoRotate` is set and turns nothing.
//!
//! [`HdrLoader`]: three_rs::loaders::HdrLoader

use three_rs::addons::controls::OrbitControls;
use three_rs::loaders::{GltfLoader, HdrLoader};
use three_rs::materials::ToneMapping;
use three_rs::nodes::pmrem_node::PmremEnvironment;
use three_rs::objects::Background;
use three_rs::renderer::cube_render_target;
use three_rs::{PerspectiveCamera, Renderer, RendererParameters, Scene};

pub const INNER_WIDTH: f64 = 800.0;
pub const INNER_HEIGHT: f64 = 500.0;
/// `renderer.setPixelRatio( window.devicePixelRatio )`.
pub const DPR: f64 = 1.0;

fn examples_dir() -> std::path::PathBuf {
    three_rs::testing::three_js_dir().join("examples")
}

pub struct App {
    pub renderer: Renderer,
    pub scene: Scene,
    pub camera: PerspectiveCamera,
    pub environment: PmremEnvironment,
    /// The page's `controls`.
    pub controls: OrbitControls,
}

pub fn init() -> App {
    // `new THREE.WebGPURenderer( { antialias: true } )` and the tone mapping.
    let mut parameters = RendererParameters::default();
    parameters.antialias = true;
    let mut renderer = Renderer::new(parameters).unwrap();
    renderer.set_pixel_ratio(DPR);
    renderer.set_size(INNER_WIDTH, INNER_HEIGHT);
    renderer.tone_mapping = ToneMapping::AcesFilmic;

    let mut scene = Scene::new();

    // `new THREE.PerspectiveCamera( 50, window.innerWidth / window.innerHeight,
    // 0.05, 20 )`, `camera.position.set( 0.35, 0.05, 0.35 )`.
    let mut camera = PerspectiveCamera::new(50.0, INNER_WIDTH / INNER_HEIGHT, 0.05, 20.0);
    camera.node.borrow_mut().position.set(0.35, 0.05, 0.35);

    // `controls = new OrbitControls( camera, renderer.domElement )`, then
    // `autoRotate`, its speed, the target and one `update()` — the first of
    // the page's three turns.
    let mut controls = OrbitControls::new(&mut camera);
    // The canvas the example renders at, standing in for the element's
    // `clientWidth` / `clientHeight`.
    controls.set_element_size(INNER_WIDTH, INNER_HEIGHT);
    controls.auto_rotate = true;
    controls.auto_rotate_speed = -0.5;
    controls.target.set(0.0, 0.2, 0.0);
    controls.update(&mut camera, None);

    // `new HDRLoader().setPath( 'textures/equirectangular/' ).loadAsync(
    // 'venice_sunset_1k.hdr' )` and `new GLTFLoader().setPath( 'models/gltf/'
    // ).loadAsync( 'IridescenceLamp.glb' )` — synchronous here, so the
    // `await Promise.all( … )` body follows inline.
    let texture = HdrLoader::new()
        .load(examples_dir().join("textures/equirectangular/venice_sunset_1k.hdr"))
        .unwrap();
    let gltf = GltfLoader::load(examples_dir().join("models/gltf/IridescenceLamp.glb"))
        .expect("IridescenceLamp.glb");

    // `texture.mapping = THREE.EquirectangularReflectionMapping; scene.background
    // = texture`: with `backgroundBlurriness` 0, `CubeMapNode.updateBefore()`
    // converts the 1024×512 map into a 512² cube once and the skybox samples
    // that at level 0.
    let background = cube_render_target::from_equirectangular_texture(&mut renderer, &texture)
        .expect("the equirectangular background converts to a cube");
    scene.background = Some(Background::CubeTexture(background));

    // `scene.environment = texture`: the same map, PMREM-filtered.
    let mut environment = PmremEnvironment::from_equirectangular(&texture);
    environment.update(&mut renderer).unwrap();
    scene.environment = Some(environment.handle());

    // `scene.add( gltf.scene )`.
    scene.add(&gltf.scene);

    // `render()`: its `controls.update()` is the second turn. The frame it
    // draws is overwritten by the animation loop's before anything reads the
    // canvas, so only the turn is kept.
    controls.update(&mut camera, None);

    App {
        renderer,
        scene,
        camera,
        environment,
        controls,
    }
}

/// The page's `render()`: `controls.update()` (one more auto-rotation turn)
/// and one `renderer.render( scene, camera )`.
pub fn animate(app: &mut App) {
    app.controls.update(&mut app.camera, None);

    app.environment.update(&mut app.renderer).unwrap();
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
        .unwrap_or_else(|| "target/webgpu_loader_gltf_iridescence.png".to_string());
    three_rs::testing::write_png(&path, width, height, &pixels);
    println!("wrote {path} ({width}x{height})");
}
