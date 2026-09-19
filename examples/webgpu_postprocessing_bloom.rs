//! Port of `three.js/examples/webgpu_postprocessing_bloom.html`, calling the
//! three-rs API in the same order the page's top-level script does.
//!
//! The whole scene is one glTF file — `models/gltf/PrimaryIonDrive.glb`, 20
//! nodes, 6 primitives, 3 materials, `COLOR_0` vertex colours, an alpha-blended
//! material, emissive factors and a node-TRS clip — so this example is the gate
//! for `GLTFLoader`'s ordinary, non-skinned path. What the loader produces is
//! checked against three's own parse by `tests/gltf_primary_ion_drive.rs`
//! before a pixel is rendered.
//!
//! Under the e2e harness the viewport is `400 * 2` x `250 * 2` with no
//! `deviceScaleFactor`, so `window.innerWidth` / `innerHeight` /
//! `devicePixelRatio` are 800, 500 and 1, and `performance.now()` is pinned to
//! 0 — so `timer.getDelta()` is 0 and the mixer is at t = 0 on the graded
//! frame.
//!
//! Three details of the page that are easy to lose:
//!
//! * **`OrbitControls` is constructed and never updated.** Its constructor
//!   calls `update()`, which is `lookAt( 0, 0, 0 )`; `animate()` does not touch
//!   it. So the camera pose is static and spelled out here.
//! * **The point light is a child of the camera**, and the camera is a child of
//!   the scene, so `camera.matrixWorld` has to be current before the light is
//!   collected — which `scene.add( camera )` is enough for.
//! * **`clip.optimize()` is not a no-op.** Two of the four tracks drop from 879
//!   keyframes to 690, and it is the optimized clip the mixer plays.
//!
//! `Math.random()` is never drawn from: `grep -c Math.random` is 0 in both the
//! inspector and lil-gui, so there is no seeded sequence to keep in step.

use three_rs::animation::AnimationMixer;
use three_rs::loaders::GLTFLoader;
use three_rs::nodes::display::{bloom, BloomNode};
use three_rs::{
    AmbientLight, Color, PassNode, PerspectiveCamera, PointLight, RenderPipeline, Renderer,
    RendererParameters, Scene, ToneMapping, Vector3,
};

pub const INNER_WIDTH: f64 = 800.0;
pub const INNER_HEIGHT: f64 = 500.0;
/// `renderer.setPixelRatio( window.devicePixelRatio )`.
pub const DPR: f64 = 1.0;

pub struct App {
    pub renderer: Renderer,
    pub scene: Scene,
    pub camera: PerspectiveCamera,
    pub mixer: AnimationMixer,
    pub gltf_scene: three_rs::Node,
    pub scene_pass: PassNode,
    pub bloom_pass: BloomNode,
    pub render_pipeline: RenderPipeline,
}

pub fn init() -> App {
    let scene = Scene::new();

    let mut camera = PerspectiveCamera::new(40.0, INNER_WIDTH / INNER_HEIGHT, 1.0, 100.0);
    camera.node.borrow_mut().position.set(-5.0, 2.5, -3.5);
    scene.add(&camera.node);

    scene.add(&AmbientLight::new(Color::from_hex(0xcccccc), 1.0));

    // `camera.add( pointLight )` — at the camera's own origin, so the model is
    // lit from wherever the camera is.
    let point_light = PointLight::new(Color::from_hex(0xffffff), 100.0, 0.0);
    camera.node.add(&point_light);

    // `const gltf = await new GLTFLoader().loadAsync( … )` — the harness's
    // single RAF fires only once the load has resolved, so the load is
    // synchronous here and the frame below is the page's first.
    let three = three_rs::testing::three_js_dir();
    let gltf = GLTFLoader::load(three.join("examples/models/gltf/PrimaryIonDrive.glb"))
        .expect("PrimaryIonDrive.glb");

    let model = gltf.scene.clone();
    scene.add(&model);

    // `mixer.clipAction( gltf.animations[ 0 ].optimize() ).play()`. `optimize()`
    // drops 189 keyframes from each of the two `circle` tracks; playing the
    // unoptimized clip is a different pose at t = 0.
    let mut clip = gltf.animations[0].clone();
    clip.optimize();

    let mut mixer = AnimationMixer::new(Box::new(gltf.scene_resolver()));
    let action = mixer.clip_action(&clip, None, None);
    mixer.play(action);

    // `new OrbitControls( camera, renderer.domElement )`: the constructor calls
    // `update()`, and `animate()` never does, so this is the camera's pose for
    // the whole page.
    camera.look_at(&Vector3::new(0.0, 0.0, 0.0));

    // renderer

    let mut renderer = Renderer::new(RendererParameters { antialias: true }).unwrap();
    renderer.set_pixel_ratio(DPR);
    renderer.set_size(INNER_WIDTH, INNER_HEIGHT);
    // `renderer.toneMapping = ReinhardToneMapping`, exposure left at 1. It is
    // set before the `RenderPipeline` is built, so the tone mapper ends up
    // inside the output quad's own shader — see `docs/postprocessing.md`.
    renderer.tone_mapping = ToneMapping::Reinhard;

    // post processing

    let scene_pass = PassNode::new();
    // `toInspector( 'Color' )` and `toInspector( 'Bloom' )` are no-ops on the
    // rendered frame: they name the node for the inspector panel and return it
    // unchanged.
    let scene_pass_color = scene_pass.texture_node("output");

    // `bloom( scenePassColor )` — strength 1, radius 0, threshold 0.
    let bloom_pass = bloom(scene_pass_color);

    let mut render_pipeline = RenderPipeline::new();
    render_pipeline.output_node = Some(scene_pass.texture_node("output").add(bloom_pass.node()));

    App {
        renderer,
        scene,
        camera,
        mixer,
        gltf_scene: model,
        scene_pass,
        bloom_pass,
        render_pipeline,
    }
}

/// The page's `animate()`, run once by the harness's single RAF.
pub fn animate(app: &mut App) {
    // `timer.update(); mixer.update( timer.getDelta() )` — `performance.now()`
    // is pinned to 0, so the delta is 0 and the clip is at its first keyframe.
    app.mixer.update(0.0);

    // `PassNode.updateBefore()`, then `BloomNode.updateBefore()`'s twelve
    // quads, then the output quad — see `docs/postprocessing.md` for why the
    // port fires them explicitly.
    app.scene_pass
        .render(&mut app.renderer, &mut app.scene, &mut app.camera);
    app.bloom_pass.render(&mut app.renderer);
    app.render_pipeline.render(&mut app.renderer);
}

fn main() {
    let mut app = init();
    println!("adapter: {:?}", app.renderer.adapter_info());
    animate(&mut app);

    let (width, height, pixels) = app.renderer.read_canvas_pixels().unwrap();
    let path = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "target/webgpu_postprocessing_bloom.png".to_string());
    three_rs::testing::write_png(&path, width, height, &pixels);
    println!("wrote {path} ({width}x{height})");
}
