//! Port of `three.js/examples/webgpu_loader_gltf_transmission.html`, calling
//! the three-rs API in the same order the page's `init()` does.
//!
//! **This example does not run yet.** `IridescentDishWithOlives.glb` is
//! Draco-compressed (`extensionsRequired: [ "KHR_draco_mesh_compression" ]`)
//! and nothing in the port decodes Draco, so `GLTFLoader::load` returns
//! `GltfError::UnsupportedRequiredExtension`. The file is here because every
//! other line of the page is already portable and the rung is one decoder away
//! from green; `docs/webgpu_loader_gltf_transmission-progress.md` and
//! `docs/nodes.md` §28 say exactly what is missing. It is deliberately NOT in
//! `tests/e2e/main.rs`.
//!
//! Under the e2e harness the viewport is `400 * 2` x `250 * 2` with no
//! `deviceScaleFactor`, so `window.innerWidth` / `innerHeight` /
//! `devicePixelRatio` are 800, 500 and 1.
//!
//! # What this page is
//!
//! The direct test of the transmission path that `webgpu_loader_gltf_anisotropy`
//! (`docs/nodes.md` §26) built over a copied opaque frame. Where the barn lamp
//! has one small glass cover, the dish has two transmissive materials covering
//! most of the frame and stacked over each other:
//!
//! * `glassDish` — `transmissionFactor` 1, `thicknessFactor` 0.01,
//!   `specularColorFactor` `[ 2, 2, 2 ]` with a `specularColorTexture`,
//!   roughness 0.07, metalness 0, plus an `occlusionTexture` and `COLOR_0`.
//! * `glassCover` — `transmissionFactor` 1, `ior` 1.5, `thicknessFactor` 0.1
//!   with a `thicknessTexture`, `specularColorFactor` `[ 3, 3, 3 ]`, a normal
//!   map at `scale` 2, and a `metallicRoughnessTexture`.
//! * `olives` — an ordinary opaque standard material, seen *through* both.
//! * `goldLeaf` — `alphaMode: MASK`, `alphaCutoff` 0.5.
//!
//! Neither transmissive material is `doubleSide`, so the double-pass gap §26
//! lists is not on the critical path for this page; the stacking of two
//! single-sided transmissive meshes is. Despite the asset's name there is no
//! `KHR_materials_iridescence` in it — the iridescent look is the
//! `specularColorTexture` on the dish — so this page does not need an
//! iridescence port either.
//!
//! The page also animates: `glassCover rotation`, one LINEAR quaternion track
//! on the `glassCover_animation` node, driven by a `Timer`. As in
//! `webgpu_skinning`, the graded delta is the first one, which is 0, so
//! `mixer.update( 0 )` is the pose that would be graded.

use three_rs::animation::AnimationMixer;
use three_rs::loaders::{GLTFLoader, UltraHdrLoader};
use three_rs::materials::ToneMapping;
use three_rs::nodes::pmrem_node::PmremEnvironment;
use three_rs::objects::Background;
use three_rs::{PerspectiveCamera, Renderer, RendererParameters, Scene, Vector3};

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
    /// The page's module-level `mixer`.
    pub mixer: AnimationMixer,
}

pub fn init() -> App {
    // `new THREE.WebGPURenderer( { antialias: true } )` and the tone mapping
    // pair. The page builds the renderer last, but the PMREM conversion below
    // needs it, so it is first here as in the other loader examples.
    let mut renderer = Renderer::new(RendererParameters { antialias: true }).unwrap();
    renderer.set_pixel_ratio(DPR);
    renderer.set_size(INNER_WIDTH, INNER_HEIGHT);
    renderer.tone_mapping = ToneMapping::AcesFilmic;
    renderer.tone_mapping_exposure = 1.0;

    let mut scene = Scene::new();

    // `new THREE.PerspectiveCamera( 45, window.innerWidth / window.innerHeight,
    // 0.25, 20 )`, `camera.position.set( 0, 0.4, 0.7 )`.
    let mut camera = PerspectiveCamera::new(45.0, INNER_WIDTH / INNER_HEIGHT, 0.25, 20.0);
    camera.node.borrow_mut().position.set(0.0, 0.4, 0.7);

    // `new OrbitControls( … )`; `controls.target.set( 0, 0.1, 0 )` and one
    // `update()`. The distance is 0.762, inside `[ minDistance, maxDistance ]`,
    // so the clamps do nothing, and with `enableDamping` and no pointer events
    // the update is a `lookAt`. `autoRotate` turns the camera by
    // `autoRotateSpeed` * the frame delta, which on the graded frame is 0.
    camera.look_at(&Vector3::new(0.0, 0.1, 0.0));

    // `new UltraHDRLoader().setPath( 'textures/equirectangular/' ).load(
    // 'royal_esplanade_2k.hdr.jpg', … )`; the loader is synchronous here so the
    // callback body follows inline.
    let texture = UltraHdrLoader::new()
        .load(examples_dir().join("textures/equirectangular/royal_esplanade_2k.hdr.jpg"))
        .unwrap();

    // `texture.mapping = THREE.EquirectangularReflectionMapping`, then
    // `scene.background` / `scene.environment` off the one PMREM, with
    // `backgroundBlurriness = 0.35` making the skybox a cubeUV read of it
    // rather than the sharp cube (`docs/nodes.md` §26).
    let mut environment = PmremEnvironment::from_equirectangular(&texture);
    environment.update(&mut renderer).unwrap();

    scene.background = Some(Background::Pmrem(environment.handle()));
    scene.background_blurriness = 0.35;
    scene.environment = Some(environment.handle());

    // `new GLTFLoader().setPath( 'models/gltf/' ).setDRACOLoader( … ).load(
    // 'IridescentDishWithOlives.glb', … )`. The port has no DRACOLoader, so
    // this is where the example stops today.
    let gltf = GLTFLoader::load(examples_dir().join("models/gltf/IridescentDishWithOlives.glb"))
        .expect("IridescentDishWithOlives.glb (needs a Draco decoder — docs/nodes.md §28)");

    // `mixer = new THREE.AnimationMixer( gltf.scene ); mixer.clipAction(
    // gltf.animations[ 0 ] ).play();`
    let mut mixer = AnimationMixer::new(Box::new(gltf.scene_resolver()));
    let action = mixer.clip_action(&gltf.animations[0], None, None);
    mixer.play(action);

    scene.add(&gltf.scene);

    App {
        renderer,
        scene,
        camera,
        environment,
        mixer,
    }
}

/// The page's `render()`: `mixer.update( timer.getDelta() )` with the first
/// delta, which is 0, then one render.
pub fn animate(app: &mut App) {
    app.mixer.update(0.0);
    app.environment.update(&mut app.renderer).unwrap();
    app.renderer.render(&mut app.scene, &mut app.camera);
}

fn main() {
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
