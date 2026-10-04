//! Port of `three.js/examples/webgpu_upscaling_fsr1.html`, calling the
//! three-rs API in the same order the page's `init()` / `animate()` do.
//!
//! Littlest Tokyo, drawn by a scene pass at half resolution and upscaled to
//! the canvas by [`Fsr1Node`]: EASU's edge-adaptive Lanczos upsampling, then
//! RCAS sharpening.
//!
//! **The rung is ignored.** three.js fails its own reference screenshot for
//! this page on this machine (703 of 100000 pixels off, twice, against a
//! 0.1% limit), so the port has nothing to be graded against. What the port
//! checks instead is the EASU and RCAS shaders, against three's dump in
//! `tests/nodes_display_wgsl.rs`, and what they do to a frame in
//! `tests/fsr1_frames.rs`.
//!
//! The page loads the model asynchronously, and its callback adds it to
//! whatever `scene` names when it runs: by then `init()` has replaced the
//! first `Scene` with the one that has the background and environment, so
//! that is the one drawn with the model in it. The port loads it
//! synchronously into that scene. The GUI is not ported; its defaults are
//! (`upscaleMethod: 'FSR1'`, `resolutionScale: 0.5`).

use std::cell::RefCell;
use std::rc::Rc;

use three_rs::addons::controls::OrbitControls;
use three_rs::animation::AnimationMixer;
use three_rs::loaders::GltfLoader;
use three_rs::nodes::display::Fsr1Node;
use three_rs::nodes::pmrem_node::PmremEnvironment;
use three_rs::nodes::tsl::float;
use three_rs::objects::Background;
use three_rs::{
    pass, Color, PassNode, PerspectiveCamera, RenderPipeline, Renderer, RendererParameters,
    RoomEnvironment, Scene, Timer,
};

pub const INNER_WIDTH: f64 = 800.0;
pub const INNER_HEIGHT: f64 = 500.0;
/// `renderer.setPixelRatio( window.devicePixelRatio )`.
pub const DPR: f64 = 1.0;

/// The page's `params`, the GUI's starting values.
pub struct Params {
    /// `'FSR1'`; the GUI's other choice, `'Bilinear'`, outputs `scenePass`.
    pub upscale_method: &'static str,
    pub resolution_scale: f64,
}

pub const PARAMS: Params = Params {
    upscale_method: "FSR1",
    resolution_scale: 0.5,
};

fn examples_dir() -> std::path::PathBuf {
    three_rs::testing::three_js_dir().join("examples")
}

pub struct App {
    pub renderer: Renderer,
    /// Shared with `scene_pass`, which renders it from `updateBefore()`.
    pub scene: Rc<RefCell<Scene>>,
    /// Shared with `scene_pass`.
    pub camera: Rc<RefCell<PerspectiveCamera>>,
    pub controls: OrbitControls,
    pub mixer: AnimationMixer,
    pub timer: Timer,
    /// Kept alive for the environment's handle on the scene.
    pub environment: PmremEnvironment,
    pub scene_pass: PassNode,
    pub fsr1_node: Fsr1Node,
    pub render_pipeline: RenderPipeline,
}

pub fn init() -> App {
    let mut camera = PerspectiveCamera::new(25.0, INNER_WIDTH / INNER_HEIGHT, 0.1, 100.0);
    camera.node.borrow_mut().position.set(-0.5, 0.0, 12.0);

    let timer = Timer::new();

    // renderer

    let mut parameters = RendererParameters::default();
    parameters.antialias = true;
    let mut renderer = Renderer::new(parameters).unwrap();
    renderer.set_pixel_ratio(DPR);
    renderer.set_size(INNER_WIDTH, INNER_HEIGHT);

    let mut scene = Scene::new();
    scene.background = Some(Background::Color(Color::from_hex(0xbfe3dd)));
    let mut room = RoomEnvironment::new();
    let environment = PmremEnvironment::from_scene(&mut renderer, &mut room, 0.04).unwrap();
    scene.environment = Some(environment.handle());

    // model: `loader.load( 'models/gltf/LittlestTokyo.glb', … )`'s callback,
    // run against the scene `init()` has just made.
    let gltf = GltfLoader::load(examples_dir().join("models/gltf/LittlestTokyo.glb"))
        .expect("three-rs: LittlestTokyo.glb loads");
    gltf.scene.borrow_mut().scale.set(0.01, 0.01, 0.01);
    scene.add(&gltf.scene);

    let mut mixer = AnimationMixer::new(Box::new(gltf.scene_resolver()));
    let action = mixer.clip_action(&gltf.animations[0], None, None);
    mixer.play(action);

    // controls

    let mut controls = OrbitControls::new(&mut camera);
    // The canvas the example renders at, standing in for the element's
    // `clientWidth` / `clientHeight`.
    controls.set_element_size(INNER_WIDTH, INNER_HEIGHT);
    controls.enable_damping = true;
    controls.target.set(-0.5, 0.0, 0.0);

    // render pipeline

    let scene = Rc::new(RefCell::new(scene));
    let camera = Rc::new(RefCell::new(camera));

    let render_pipeline = RenderPipeline::new();

    // `toInspector( … )` returns its node unchanged.
    let scene_pass = pass(scene.clone(), camera.clone());
    scene_pass.set_resolution_scale(PARAMS.resolution_scale);

    // FSR 1: `fsr1( scenePass )`, whose `convertToTexture()` passes the
    // pass's texture node through.
    let fsr1_node = Fsr1Node::new(
        &scene_pass.texture(),
        float(Fsr1Node::DEFAULT_SHARPNESS),
        false,
    );

    let mut app = App {
        renderer,
        scene,
        camera,
        controls,
        mixer,
        timer,
        environment,
        scene_pass,
        fsr1_node,
        render_pipeline,
    };
    update_pipeline(&mut app);
    app
}

/// The page's `updatePipeline()`, with `params.upscaleMethod` as the GUI
/// leaves it.
pub fn update_pipeline(app: &mut App) {
    app.render_pipeline.output_node = Some(if PARAMS.upscale_method == "FSR1" {
        app.fsr1_node.node()
    } else {
        app.scene_pass.node()
    });
}

/// The page's `animate()`. `timer.getDelta()` is 0 on the frame `main()`
/// writes, so the animation does not move.
pub fn animate(app: &mut App) {
    app.controls.update(&mut app.camera.borrow_mut(), None);

    app.timer.update();

    let delta = app.timer.get_delta();

    app.mixer.update(delta);

    app.render_pipeline.render(&mut app.renderer);
}

/// The page's `onWindowResize()`.
pub fn resize(app: &mut App, width: f64, height: f64) {
    let mut camera = app.camera.borrow_mut();
    camera.aspect = width / height;
    camera.update_projection_matrix();
    app.renderer.set_size(width, height);
    app.controls.set_element_size(width, height);
}

pub fn controls(app: &mut App) -> Option<&mut OrbitControls> {
    Some(&mut app.controls)
}

/// The controls and the camera at once, for a host delivering pointer events.
pub fn controls_and_camera(
    app: &mut App,
) -> Option<(&mut OrbitControls, std::cell::RefMut<'_, PerspectiveCamera>)> {
    Some((&mut app.controls, app.camera.borrow_mut()))
}

fn main() {
    // Pin both clocks to zero, as three.js' `test/e2e/deterministic-injection.js`
    // does to the page, so the frame this writes does not depend on how long
    // `init()` took.
    three_rs::testing::pin_time(Some(0.0));
    let mut app = init();
    println!("adapter: {:?}", app.renderer.adapter_info());
    animate(&mut app);

    let (width, height, pixels) = app.renderer.read_canvas_pixels().unwrap();
    let path = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "target/webgpu_upscaling_fsr1.png".to_string());
    three_rs::testing::write_png(&path, width, height, &pixels);
    println!("wrote {path} ({width}x{height})");
}
