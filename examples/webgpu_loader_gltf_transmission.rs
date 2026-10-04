//! Port of `three.js/examples/webgpu_loader_gltf_transmission.html`, calling
//! the three-rs API in the same order the page's `init()` and its two loader
//! callbacks do.
//!
//! Under the e2e harness the viewport is `400 * 2` x `250 * 2` with no
//! `deviceScaleFactor`, so `window.innerWidth` / `innerHeight` /
//! `devicePixelRatio` are 800, 500 and 1.
//!
//! The page draws from `Math.random()` not at all. Two things move:
//!
//! * **The animation.** `glassCover rotation`, one LINEAR quaternion track on
//!   the `glassCover_animation` node, through an `AnimationMixer` on a
//!   non-skinned node, driven by a `Timer`. As in `webgpu_skinning`, the
//!   harness pins the clock, so the graded delta is the first one, which is 0:
//!   `mixer.update( 0 )` is the graded pose (the clip's first keyframe, the
//!   cover lifted and tilted).
//! * **The camera.** `controls.autoRotate` with `autoRotateSpeed = -0.75`, and
//!   the page calls `controls.update()` with no delta — once in `init()` and
//!   once per frame — so each call turns the camera by `2π / 60 / 60 * -0.75`
//!   (the frame-count branch of `_getAutoRotationAngle`), applied through
//!   `enableDamping`'s `dampingFactor`. That is per frame, not per second, so
//!   the pinned clock does not stop it; the graded frame is the camera after
//!   exactly those two updates, which is what [`init`] and [`animate`] do.
//!
//! # What this page is
//!
//! The direct test of the transmission path `webgpu_loader_gltf_anisotropy`
//! built over a copied opaque frame. Where the barn lamp has one small glass
//! cover, `IridescentDishWithOlives.glb` has two transmissive materials covering
//! most of the model and stacked over each other, both reading the one opaque
//! copy:
//!
//! * `glassDish` — `transmissionFactor` 1, `thicknessFactor` 0.01,
//!   `specularColorFactor` `[ 2, 2, 2 ]` with a `specularColorTexture`,
//!   roughness 0.07, metalness 0, plus an `occlusionTexture` and `COLOR_0`.
//! * `glassCover` — `transmissionFactor` 1, `ior` 1.5, `thicknessFactor` 0.1
//!   with a `thicknessTexture`, `specularColorFactor` `[ 3, 3, 3 ]`, a normal
//!   map at `scale` 2, and a `metallicRoughnessTexture`.
//! * `olives` — an ordinary opaque standard material, seen *through* both.
//! * `goldLeaf` — `alphaMode: MASK`, `alphaCutoff` 0.5, which reaches the
//!   shader as the `materialAlphaTest` uniform.
//!
//! Neither transmissive material is `doubleSided`, so `needsDoublePass()` is
//! false. Despite the asset's name there is no `KHR_materials_iridescence` in
//! it — the iridescent look is the `specularColorTexture` on the dish.
//!
//! Every mesh in the file is `KHR_draco_mesh_compression` (listed in
//! `extensionsRequired`), so this page is also the ladder's Draco page for a
//! glTF: the page's `DRACOLoader` is the loader's built-in decoder here.
//! `docs/nodes.md` §94.

use three_rs::addons::controls::OrbitControls;
use three_rs::animation::AnimationMixer;
use three_rs::loaders::{GltfLoader, UltraHdrLoader};
use three_rs::materials::ToneMapping;
use three_rs::nodes::pmrem_node::PmremEnvironment;
use three_rs::objects::Background;
use three_rs::Timer;
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
    /// The page's module-level `mixer`.
    pub mixer: AnimationMixer,
    /// The page's module-level `timer`.
    pub timer: Timer,
}

pub fn init() -> App {
    // `timer = new THREE.Timer(); timer.connect( document );`
    let timer = Timer::new();

    // `new THREE.PerspectiveCamera( 45, window.innerWidth / window.innerHeight,
    // 0.25, 20 )`, `camera.position.set( 0, 0.4, 0.7 )`.
    let mut camera = PerspectiveCamera::new(45.0, INNER_WIDTH / INNER_HEIGHT, 0.25, 20.0);
    camera.node.borrow_mut().position.set(0.0, 0.4, 0.7);

    let mut scene = Scene::new();

    // `new THREE.WebGPURenderer( { antialias: true } )` and the tone mapping
    // pair. The page builds the renderer after starting the loads, but the
    // PMREM conversion below needs it, so it is first here as in the other
    // loader examples.
    let mut parameters = RendererParameters::default();
    parameters.antialias = true;
    let mut renderer = Renderer::new(parameters).unwrap();
    renderer.set_pixel_ratio(DPR);
    renderer.set_size(INNER_WIDTH, INNER_HEIGHT);
    renderer.tone_mapping = ToneMapping::AcesFilmic;
    renderer.tone_mapping_exposure = 1.0;

    // `new UltraHDRLoader().setPath( 'textures/equirectangular/' ).load(
    // 'royal_esplanade_2k.hdr.jpg', … )`; the loader is synchronous here so the
    // callback body follows inline.
    let texture = UltraHdrLoader::new()
        .load(examples_dir().join("textures/equirectangular/royal_esplanade_2k.hdr.jpg"))
        .unwrap();

    // `texture.mapping = THREE.EquirectangularReflectionMapping`, then
    // `scene.background` / `scene.environment` off the one PMREM, with
    // `backgroundBlurriness = 0.35` making the skybox a cubeUV read of it
    // rather than the sharp cube.
    let mut environment = PmremEnvironment::from_equirectangular(&texture);
    environment.update(&mut renderer).unwrap();

    scene.background = Some(Background::Pmrem(environment.handle()));
    scene.background_blurriness = 0.35;
    scene.environment = Some(environment.handle());

    // `new GLTFLoader().setPath( 'models/gltf/' ).setDRACOLoader( … ).load(
    // 'IridescentDishWithOlives.glb', … )`, synchronous here; the loader
    // decodes `KHR_draco_mesh_compression` itself.
    let gltf = GltfLoader::load(examples_dir().join("models/gltf/IridescentDishWithOlives.glb"))
        .expect("IridescentDishWithOlives.glb");

    // `mixer = new THREE.AnimationMixer( gltf.scene ); mixer.clipAction(
    // gltf.animations[ 0 ] ).play();`
    let mut mixer = AnimationMixer::new(Box::new(gltf.scene_resolver()));
    let action = mixer.clip_action(&gltf.animations[0], None, None);
    mixer.play(action);

    scene.add(&gltf.scene);

    // `controls = new OrbitControls( camera, renderer.domElement )`. The
    // distance to the target is 0.762, inside `[ minDistance, maxDistance ]`,
    // so the clamps do nothing; `autoRotate` turns the camera on every
    // `update()` (see the module docs).
    let mut controls = OrbitControls::new(&mut camera);
    // The canvas the example renders at, standing in for the element's
    // `clientWidth` / `clientHeight`.
    controls.set_element_size(INNER_WIDTH, INNER_HEIGHT);
    controls.auto_rotate = true;
    controls.auto_rotate_speed = -0.75;
    controls.enable_damping = true;
    controls.min_distance = 0.5;
    controls.max_distance = 1.0;
    controls.target.set(0.0, 0.1, 0.0);
    // `controls.update();`
    controls.update(&mut camera, None);

    App {
        renderer,
        scene,
        camera,
        environment,
        controls,
        mixer,
        timer,
    }
}

/// The page's `render()`: `timer.update()`, `mixer.update( timer.getDelta() )`
/// (0 on the graded frame), `controls.update()`, one render.
pub fn animate(app: &mut App) {
    app.timer.update();
    let delta = app.timer.get_delta();
    app.mixer.update(delta);

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
        .unwrap_or_else(|| "target/webgpu_loader_gltf_transmission.png".to_string());
    three_rs::testing::write_png(&path, width, height, &pixels);
    println!("wrote {path} ({width}x{height})");
}
