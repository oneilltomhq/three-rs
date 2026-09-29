//! Port of `three.js/examples/webgpu_loader_gltf_compressed.html`, calling the
//! three-rs API in the same order the page's `init()` and its loader callback
//! do.
//!
//! Under the e2e harness the viewport is `400 * 2` x `250 * 2` with no
//! `deviceScaleFactor`, so `window.innerWidth` / `innerHeight` /
//! `devicePixelRatio` are 800, 500 and 1. The page draws from `Math.random()`
//! not at all and reads no clock.
//!
//! # What this page is
//!
//! `coffeemat.glb`: a mug and an Oreo-dunking arm, compressed every way glTF
//! allows at once. Its geometry is `EXT_meshopt_compression` bufferViews over
//! `KHR_mesh_quantization` accessors, and its five maps are
//! `KHR_texture_basisu` KTX 2.0 images that `KTX2Loader` transcodes, after
//! `detectSupport( renderer )`, to a block format the device samples
//! compressed. Every map carries a `KHR_texture_transform` (a tiny scale and
//! offset into a shared atlas).
//!
//! The only light is a `PointLight` of power 1300 lm parented to the camera,
//! the background a flat `0xEEEEEE`, and the output goes through
//! `ReinhardToneMapping`. There is no environment, so the indirect terms in
//! the fragment shader are all zero, as in three's dump.

use three_rs::addons::controls::OrbitControls;
use three_rs::loaders::{GltfLoader, Ktx2Loader};
use three_rs::materials::ToneMapping;
use three_rs::math::Color;
use three_rs::objects::Background;
use three_rs::{PerspectiveCamera, PointLight, Renderer, RendererParameters, Scene};

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
    /// The page's `controls`.
    pub controls: OrbitControls,
}

pub fn init() -> App {
    // `new THREE.PerspectiveCamera( 50, window.innerWidth /
    // window.innerHeight, 1, 20 )`, then `camera.position.set( 2, 2, 2 )`.
    let mut camera = PerspectiveCamera::new(50.0, INNER_WIDTH / INNER_HEIGHT, 1.0, 20.0);
    camera.node.borrow_mut().position.set(2.0, 2.0, 2.0);

    let mut scene = Scene::new();
    scene.background = Some(Background::Color(Color::from_hex(0xEEEEEE)));

    //lights

    // `new THREE.PointLight( 0xffffff )`: intensity 1, distance 0, then
    // `light.power = 1300`.
    let light = PointLight::new(Color::from_hex(0xffffff), 1.0, 0.0);
    light.borrow_mut().light_mut().unwrap().set_power(1300.0);
    camera.node.add(&light);
    scene.add(&camera.node);

    //renderer

    let mut renderer = Renderer::new(RendererParameters { antialias: true }).unwrap();
    renderer.set_pixel_ratio(DPR);
    renderer.set_size(INNER_WIDTH, INNER_HEIGHT);
    renderer.tone_mapping = ToneMapping::Reinhard;
    renderer.tone_mapping_exposure = 1.0;

    let mut controls = OrbitControls::new(&mut camera);
    // The canvas the example renders at, standing in for the element's
    // `clientWidth` / `clientHeight`.
    controls.set_element_size(INNER_WIDTH, INNER_HEIGHT);
    controls.min_distance = 3.0;
    controls.max_distance = 6.0;
    controls.update(&mut camera, None);

    // `await new KTX2Loader().detectSupport( renderer )`.
    let ktx2_loader = Ktx2Loader::new().detect_support(&renderer);

    // `loader.setKTX2Loader( ktx2Loader ); loader.setMeshoptDecoder(
    // MeshoptDecoder ); loader.load( 'models/gltf/coffeemat.glb', … )`. The
    // meshopt decoder is built into the port's loader; the load is
    // synchronous, so the callback's body follows inline.
    let gltf = GltfLoader::load_with_ktx2(
        examples_dir().join("models/gltf/coffeemat.glb"),
        &ktx2_loader,
    )
    .expect("coffeemat.glb");

    let gltf_scene = gltf.scene.clone();
    {
        let mut node = gltf_scene.borrow_mut();
        node.position.y = -0.8;
        node.scale.set(0.01, 0.01, 0.01);
    }

    scene.add(&gltf_scene);

    App {
        renderer,
        scene,
        camera,
        controls,
    }
}

/// The page's `animate()`.
pub fn animate(app: &mut App) {
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
    three_rs::testing::pin_time(Some(0.0));
    let mut app = init();
    println!("adapter: {:?}", app.renderer.adapter_info());
    animate(&mut app);

    let (width, height, pixels) = app.renderer.read_canvas_pixels().unwrap();
    let path = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "target/webgpu_loader_gltf_compressed.png".to_string());
    three_rs::testing::write_png(&path, width, height, &pixels);
    println!("wrote {path} ({width}x{height})");
}
