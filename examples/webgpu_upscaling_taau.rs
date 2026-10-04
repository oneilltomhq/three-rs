//! Port of `three.js/examples/webgpu_upscaling_taau.html`, calling the
//! three-rs API in the same order the page's `init()` / `animate()` do.
//!
//! The Littlest Tokyo model, lit by a `RoomEnvironment` PMREM, is drawn at
//! half the drawing buffer's resolution into an `output` + `velocity` MRT.
//! [`taau`] upsamples it back to full size, accumulating jittered frames into
//! a full-resolution history, and an RCAS [`SharpenNode`] sharpens the result.
//!
//! **The page has no grade.** three.js itself misses its own reference
//! screenshot on this machine (540 of 100000 pixels, limit 0.1%), so the
//! e2e rung is ignored and records the port's score beside three's. The
//! node is checked by the dump gates in `tests/nodes_display_wgsl.rs` and
//! over frames by `tests/taau_frames.rs`.
//!
//! Things the page does that are easy to lose:
//!
//! * **The first frame is unjittered.** three registers TAAU's pipeline
//!   callbacks while that frame builds, after the before callbacks have run,
//!   so the graded frame is the beauty pass upsampled from a history seeded
//!   with its bilinear resize; [`TaauNode::attach`] reproduces that.
//! * **`scene` is replaced after `renderer.init()`.** The model's load
//!   callback runs after that and adds it to the new scene, the one with the
//!   background and environment; the port builds that one scene.
//! * **`sharpen( taauNode.getTextureNode(), params.sharpness )`** passes the
//!   number 0.2, which becomes a constant in the RCAS shader (the GUI's
//!   `sharpenNode.sharpness.value = …` does nothing). The port builds the
//!   node straight over the TAAU texture, as `sharpen()` does for a texture
//!   node, with a `float( 0.2 )` constant.
//! * **The mixer is updated with `timer.getDelta()`**, 0 on the graded frame,
//!   which poses the model at the clip's first keyframe.
//! * **The GUI** (bilinear/TAAU switch, resolution scale, sharpening) is not
//!   ported; the page's initial state is.

use std::cell::RefCell;
use std::rc::Rc;

use three_rs::addons::controls::OrbitControls;
use three_rs::animation::AnimationMixer;
use three_rs::loaders::GltfLoader;
use three_rs::nodes::display::{taau, SharpenNode, TaauNode};
use three_rs::nodes::mrt;
use three_rs::nodes::pmrem_node::PmremEnvironment;
use three_rs::nodes::tsl::{float, output_property};
use three_rs::nodes::velocity::velocity;
use three_rs::Timer;
use three_rs::{
    pass, Color, PassNode, PerspectiveCamera, RenderPipeline, Renderer, RendererParameters,
    RoomEnvironment, Scene,
};

pub const INNER_WIDTH: f64 = 800.0;
pub const INNER_HEIGHT: f64 = 500.0;
/// `renderer.setPixelRatio( window.devicePixelRatio )`.
pub const DPR: f64 = 1.0;

/// The page's `params.resolutionScale`.
const RESOLUTION_SCALE: f64 = 0.5;
/// The page's `params.sharpness`.
const SHARPNESS: f64 = 0.2;

fn examples_dir() -> std::path::PathBuf {
    three_rs::testing::three_js_dir().join("examples")
}

pub struct App {
    pub renderer: Renderer,
    /// Shared with `scene_pass`, which renders it from `updateBefore()`.
    pub scene: Rc<RefCell<Scene>>,
    /// Shared with `scene_pass` and `taau_node`, which jitters it.
    pub camera: Rc<RefCell<PerspectiveCamera>>,
    /// The page's `controls`.
    pub controls: OrbitControls,
    /// The page's module-level `mixer`.
    pub mixer: AnimationMixer,
    /// The page's module-level `timer`.
    pub timer: Timer,
    /// `scene.environment`, PMREM-filtered.
    pub environment: PmremEnvironment,
    pub scene_pass: PassNode,
    pub taau_node: TaauNode,
    pub sharpen_node: SharpenNode,
    pub render_pipeline: RenderPipeline,
}

pub fn init() -> App {
    let mut camera = PerspectiveCamera::new(25.0, INNER_WIDTH / INNER_HEIGHT, 0.1, 100.0);
    camera.node.borrow_mut().position.set(-0.5, 0.0, 12.0);

    let timer = Timer::new();

    // model — `loader.load( 'models/gltf/LittlestTokyo.glb', … )`,
    // Draco-compressed. The loader here is synchronous; see the module docs
    // for which scene the model lands in.

    let gltf = GltfLoader::load(examples_dir().join("models/gltf/LittlestTokyo.glb"))
        .expect("three-rs: LittlestTokyo.glb loads");
    let model = gltf.scene.clone();
    model.borrow_mut().scale.set(0.01, 0.01, 0.01);

    let mut mixer = AnimationMixer::new(Box::new(gltf.scene_resolver()));
    let action = mixer.clip_action(&gltf.animations[0], None, None);
    mixer.play(action);

    // renderer

    let mut renderer = Renderer::new(RendererParameters::default()).unwrap();
    renderer.set_pixel_ratio(DPR);
    renderer.set_size(INNER_WIDTH, INNER_HEIGHT);

    let mut scene = Scene::new();
    scene.set_background(Color::from_hex(0xbfe3dd));
    let mut room = RoomEnvironment::new();
    let environment = PmremEnvironment::from_scene(&mut renderer, &mut room, 0.04).unwrap();
    scene.environment = Some(environment.handle());
    scene.add(&model);

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

    let mut render_pipeline = RenderPipeline::new();
    let scene_pass = pass(scene.clone(), camera.clone());
    scene_pass.set_resolution_scale(RESOLUTION_SCALE);
    scene_pass.set_mrt(mrt(vec![
        ("output", output_property()),
        ("velocity", velocity()),
    ]));

    // `getTextureNode( name )` for each of the three inputs: besides the
    // node, it is what adds the `velocity` attachment and links all three to
    // the pass. `toInspector( … )` returns its node unchanged.
    let _ = scene_pass.texture_node("output");
    let _ = scene_pass.texture_node("depth");
    let _ = scene_pass.texture_node("velocity");

    let taau_node = taau(
        &scene_pass.texture(),
        &scene_pass.depth_texture(),
        &scene_pass.texture_named("velocity"),
        camera.clone(),
    );
    taau_node.attach(&mut render_pipeline);
    let sharpen_node = SharpenNode::new(&taau_node.texture(), float(SHARPNESS), false);

    // `updatePipeline()` with `upscaleMethod: 'TAAU'` and `sharpening: true`.
    render_pipeline.output_node = Some(sharpen_node.node());

    App {
        renderer,
        scene,
        camera,
        controls,
        mixer,
        timer,
        environment,
        scene_pass,
        taau_node,
        sharpen_node,
        render_pipeline,
    }
}

/// The page's `animate()`.
pub fn animate(app: &mut App) {
    app.controls.update(&mut app.camera.borrow_mut(), None);

    app.timer.update();

    let delta = app.timer.get_delta();

    app.mixer.update(delta);

    app.render_pipeline.render(&mut app.renderer);
}

/// The page's `onWindowResize()`. The history restarts at the new size on
/// the next frame.
pub fn resize(app: &mut App, width: f64, height: f64) {
    let mut camera = app.camera.borrow_mut();
    camera.aspect = width / height;
    camera.update_projection_matrix();
    app.renderer.set_size(width, height);
    app.controls.set_element_size(width, height);
}

/// The example's controls, for a host that has a pointer.
pub fn controls(app: &mut App) -> Option<&mut OrbitControls> {
    Some(&mut app.controls)
}

/// The controls and the camera at once, which every one of the controls'
/// event handlers needs.
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
        .unwrap_or_else(|| "target/webgpu_upscaling_taau.png".to_string());
    three_rs::testing::write_png(&path, width, height, &pixels);
    println!("wrote {path} ({width}x{height})");
}
