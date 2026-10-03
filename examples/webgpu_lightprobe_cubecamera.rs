//! Port of `three.js/examples/webgpu_lightprobe_cubecamera.html`, calling the
//! three-rs API in the same order the page's `init()` / `render()` do.
//!
//! Under the e2e harness the viewport is `400 * 2` x `250 * 2` with no
//! `deviceScaleFactor`, so `window.innerWidth` / `innerHeight` /
//! `devicePixelRatio` are 800, 500 and 1.
//!
//! The page draws from `Math.random()` not at all, so the harness' seeded
//! sequence is never touched.
//!
//! `CubeTextureLoader.load()` is asynchronous on the page, and so are
//! `renderer.init()` and `fromCubeRenderTarget()`, but the harness only fires
//! once the network is idle and the page has rendered, so the probe and its
//! helper are in place for the graded frame; here it all runs inline.
//!
//! # The light probe
//!
//! Where `webgpu_lightprobe` projects the pisa PNGs on the CPU, this page
//! renders the scene — nothing but its background — into a 256² RGBA8
//! `CubeRenderTarget` with a [`CubeCamera`] at the origin, reads the six faces
//! back (`readRenderTargetPixelsAsync()`), and projects those. The helper, a
//! sphere of radius 5 at the origin, draws the probe's irradiance over π; it is
//! the only thing in the scene the probe lights.
//!
//! # Not in the browser
//!
//! The readback blocks, as every readback in the port does. The web shell's
//! `init()` is synchronous, and a browser cannot block on `mapAsync()`, so
//! the readback returns an error there and the page stops before its first
//! frame. `tools/web_gate.skip` lists it until the shell can await something
//! between `init()` and the graded frame.

use three_rs::addons::controls::OrbitControls;
use three_rs::addons::helpers::LightProbeHelper;
use three_rs::addons::lights::LightProbeGenerator;
use three_rs::math::SphericalHarmonics3;
use three_rs::textures::TextureType;
use three_rs::{
    CubeCamera, CubeRenderTarget, CubeTextureLoader, LightProbe, PerspectiveCamera, Renderer,
    RendererParameters, Scene,
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
    /// The page's `cubeCamera`.
    pub cube_camera: CubeCamera,
    /// The page's `lightProbe`.
    pub light_probe: three_rs::core::Node,
    /// The `LightProbeHelper` the loader callback adds.
    pub helper: LightProbeHelper,
}

fn examples_dir() -> std::path::PathBuf {
    three_rs::testing::three_js_dir().join("examples")
}

pub fn init() -> App {
    // renderer
    let mut parameters = RendererParameters::default();
    parameters.antialias = true;
    let mut renderer = Renderer::new(parameters).unwrap();
    renderer.set_pixel_ratio(DPR);
    renderer.set_size(INNER_WIDTH, INNER_HEIGHT);

    // scene
    let mut scene = Scene::new();

    // camera
    let mut camera = PerspectiveCamera::new(40.0, INNER_WIDTH / INNER_HEIGHT, 1.0, 1000.0);
    camera.node.borrow_mut().position.set(0.0, 0.0, 30.0);

    // `new THREE.CubeRenderTarget( 256 )`
    let cube_render_target = CubeRenderTarget::new(256, TextureType::UnsignedByte).unwrap();

    let mut cube_camera = CubeCamera::new(1.0, 1000.0, cube_render_target);

    // controls
    let mut controls = OrbitControls::new(&mut camera);
    controls.set_element_size(INNER_WIDTH, INNER_HEIGHT);
    controls.min_distance = 10.0;
    controls.max_distance = 50.0;
    controls.enable_pan = false;

    // probe
    let light_probe = LightProbe::new(SphericalHarmonics3::default(), 1.0);
    scene.add(&light_probe);

    // envmap
    let cube_texture = CubeTextureLoader::new()
        .set_path(examples_dir().join("textures/cube/pisa/"))
        .load(["px.png", "nx.png", "py.png", "ny.png", "pz.png", "nz.png"])
        .unwrap();

    scene.set_background(cube_texture);

    cube_camera.update(&mut renderer, &mut scene);

    let probe = LightProbeGenerator::from_cube_render_target(
        &mut renderer,
        &cube_camera.render_target.texture,
    )
    .unwrap();

    LightProbe::copy(&light_probe, &probe);

    let helper = LightProbeHelper::new(&light_probe, 5.0);
    scene.add(&helper.node);

    App {
        renderer,
        scene,
        camera,
        controls,
        cube_camera,
        light_probe,
        helper,
    }
}

/// The page's `render()`.
pub fn animate(app: &mut App) {
    // `LightProbeHelper.onBeforeRender()`'s transform half.
    app.helper.update();
    app.renderer.render(&mut app.scene, &mut app.camera);
}

/// The page's `onWindowResize()`.
pub fn resize(app: &mut App, width: f64, height: f64) {
    app.renderer.set_size(width, height);

    app.camera.aspect = width / height;
    app.camera.update_projection_matrix();
}

/// The example's controls, for a host that has a pointer.
pub fn controls(app: &mut App) -> Option<&mut OrbitControls> {
    Some(&mut app.controls)
}

/// The controls and the camera at once, which every one of the controls'
/// event handlers needs.
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
        .unwrap_or_else(|| "target/webgpu_lightprobe_cubecamera.png".to_string());
    three_rs::testing::write_png(&path, width, height, &pixels);
    println!("wrote {path} ({width}x{height})");
}
