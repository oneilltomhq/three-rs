//! Port of `three.js/examples/webgpu_postprocessing_sobel.html`, calling the
//! three-rs API in the same order the page's `init()` / `animate()` do.
//!
//! Under the e2e harness the viewport is `400 * 2` x `250 * 2` with no
//! `deviceScaleFactor`, so `window.innerWidth` / `innerHeight` /
//! `devicePixelRatio` are 800, 500 and 1. Nothing in the page moves with time:
//! `controls.update()` with damping and no pointer input only aims the camera
//! at `( 0, 0.5, 0 )`.
//!
//! The page's output is `sobel( renderOutput( scenePass ) )`:
//! `SobelOperatorNode` calls `convertToTexture()` on its input, so the tone
//! mapped, sRGB-encoded scene is drawn into an `RTTNode` first and the
//! operator reads nine taps of that. `outputColorTransform = false` keeps the
//! pipeline's own quad from encoding it again.

use three_rs::addons::controls::OrbitControls;
use three_rs::loaders::GLTFLoader;
use three_rs::materials::{render_output, ToneMapping};
use three_rs::nodes::display::{convert_to_texture, sobel, RttNode, SobelOperatorNode};
use three_rs::nodes::pmrem_node::PmremEnvironment;
use three_rs::{
    Color, MeshStandardNodeMaterial, PassNode, PerspectiveCamera, RenderPipeline, Renderer,
    RendererParameters, RoomEnvironment, Scene,
};

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
    pub scene_pass: PassNode,
    /// The `RTTNode` `SobelOperatorNode.setup()` makes of `renderOutput(
    /// scenePass )`.
    pub sobel_input: RttNode,
    pub sobel: SobelOperatorNode,
    pub render_pipeline: RenderPipeline,
}

pub fn init() -> App {
    let mut scene = Scene::new();
    scene.set_background(Color::from_hex(0x000000));

    let mut camera = PerspectiveCamera::new(70.0, INNER_WIDTH / INNER_HEIGHT, 0.1, 100.0);
    camera.node.borrow_mut().position.set(0.0, 1.0, 3.0);

    // `loader.loadAsync( 'models/gltf/DragonAttenuation.glb' )`, then
    // `gltf.scene.children[ 1 ]` — the dragon, the only node the page keeps —
    // with a default `MeshStandardNodeMaterial` in place of its transmissive
    // one.
    let gltf = GLTFLoader::load(examples_dir().join("models/gltf/DragonAttenuation.glb"))
        .expect("DragonAttenuation.glb");
    let model = gltf.scene.children()[1].clone();
    model
        .borrow_mut()
        .mesh_mut()
        .expect("the dragon is a mesh")
        .material = Some(MeshStandardNodeMaterial::standard(
        Color::from_hex(0xffffff),
        1.0,
        0.0,
    ));
    scene.add(&model);

    let mut renderer = Renderer::new(RendererParameters { antialias: true }).unwrap();
    renderer.set_pixel_ratio(DPR);
    renderer.set_size(INNER_WIDTH, INNER_HEIGHT);
    renderer.tone_mapping = ToneMapping::Linear;

    // `scene.environment = pmremGenerator.fromScene( environment, 0.04 ).texture`.
    let mut room = RoomEnvironment::new();
    let environment = PmremEnvironment::from_scene(&mut renderer, &mut room, 0.04).unwrap();
    scene.environment = Some(environment.handle());

    let mut controls = OrbitControls::new(&mut camera);
    // The canvas the example renders at, standing in for the element's
    // `clientWidth` / `clientHeight`.
    controls.set_element_size(INNER_WIDTH, INNER_HEIGHT);
    controls.enable_damping = true;
    controls.enable_zoom = false;
    controls.target.set(0.0, 0.5, 0.0);
    controls.update(&mut camera, None);

    // postprocessing

    let mut render_pipeline = RenderPipeline::new();
    render_pipeline.output_color_transform = false;

    let scene_pass = PassNode::new();

    let sobel_input = convert_to_texture(render_output(scene_pass.node(), renderer.tone_mapping));
    let sobel = sobel(&sobel_input.texture());
    render_pipeline.output_node = Some(sobel.node());

    App {
        renderer,
        scene,
        camera,
        controls,
        scene_pass,
        sobel_input,
        sobel,
        render_pipeline,
    }
}

/// The page's `animate()` with `params.enabled` true: `controls.update()`,
/// then `renderPipeline.render()`, whose three passes the port fires in turn
/// (see `docs/postprocessing.md`).
pub fn animate(app: &mut App) {
    app.controls.update(&mut app.camera, None);

    app.scene_pass
        .render(&mut app.renderer, &mut app.scene, &mut app.camera);
    app.sobel_input.render(&mut app.renderer);
    // `SobelOperatorNode.updateBefore()`, after the RTT has its size.
    app.sobel.update();
    app.render_pipeline.render(&mut app.renderer);
}

/// The page's `onWindowResize()`.
pub fn resize(app: &mut App, width: f64, height: f64) {
    app.camera.aspect = width / height;
    app.camera.update_projection_matrix();
    app.renderer.set_size(width, height);
}

/// The example's controls, for a host that has a pointer.
pub fn controls(app: &mut App) -> Option<&mut OrbitControls> {
    Some(&mut app.controls)
}

/// The controls and the camera at once; see `webgpu_postprocessing_ca`.
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
        .unwrap_or_else(|| "target/webgpu_postprocessing_sobel.png".to_string());
    three_rs::testing::write_png(&path, width, height, &pixels);
    println!("wrote {path} ({width}x{height})");
}
