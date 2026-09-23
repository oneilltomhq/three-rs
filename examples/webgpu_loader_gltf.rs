//! Port of `three.js/examples/webgpu_loader_gltf.html`, calling the three-rs
//! API in the same order the page's `init()` and its two nested loader
//! callbacks do.
//!
//! Under the e2e harness the viewport is `400 * 2` x `250 * 2` with no
//! `deviceScaleFactor`, so `window.innerWidth` / `innerHeight` /
//! `devicePixelRatio` are 800, 500 and 1.
//!
//! The page draws from `Math.random()` not at all, and its `Timer` only feeds
//! an `AnimationMixer` that DamagedHelmet never creates (the model has no
//! animations), so the harness' pinned clock is never observed.
//!
//! # What this page is
//!
//! It is the shortest statement of the whole environment path: one UltraHDR
//! equirectangular map is both `scene.background` — converted to a 512² cube
//! and drawn as the skybox — and `scene.environment` — PMREM-filtered and used
//! as the indirect light of every material under the scene that carries no
//! `envMap` of its own. On top of that sits one glTF model with five maps.
//!
//! Three things are easy to lose:
//!
//! * **`scene.environment` is a scene field, not a per-material one.** The
//!   helmet's material has no `envMap`, so `NodeMaterial.setupEnvironment()`
//!   falls back to `scene.environmentNode`. The port grew
//!   [`Scene::environment`] for this page; `docs/nodes.md` §23 says why, and
//!   why `webgpu_postprocessing_bloom_emissive` still puts the handle on the
//!   materials by hand.
//! * **`scene.backgroundBlurriness` / `backgroundIntensity` are GUI knobs at
//!   their defaults.** The page adds `backgroundBlurriness` to the inspector
//!   panel; the graded frame is the untouched 0, with intensity 1. The skybox
//!   is the sharp cube, not a blurred PMREM read.
//! * **`fitCameraToSelection()` throws the page's own camera away.** The
//!   `camera.position.set( - 1.8, 0.6, 2.7 )` in `init()` survives only as the
//!   *direction* the helper keeps; the distance, the target, `near` and `far`
//!   are all recomputed from the model's bounding box once it has loaded. The
//!   function is transcribed below line for line, including its unused
//!   `fitWidthDistance` and the `2 * atan( PI * fov / 360 )` that is not the
//!   half-angle tangent anyone would write from scratch.
//!
//! # The model
//!
//! The page fetches Khronos' `model-index.json` and loads `DamagedHelmet` from
//! `glTF-Sample-Assets/…/glTF-Binary/DamagedHelmet.glb` over the network. That
//! GLB and the `DamagedHelmet.gltf` the three.js checkout ships under
//! `examples/models/gltf/` are the same asset: same node rotation, same
//! `Material_MR`, and the five embedded JPEGs are byte-for-byte the five files
//! beside the `.gltf` (checked by MD5 — `docs/webgpu_loader_gltf-progress.md`).
//! This port loads the local copy, so the example needs no network.
//!
//! [`Scene::environment`]: three_rs::Scene::environment

use three_rs::loaders::{GLTFLoader, UltraHdrLoader};
use three_rs::materials::ToneMapping;
use three_rs::math::Box3;
use three_rs::nodes::pmrem_node::PmremEnvironment;
use three_rs::objects::Background;
use three_rs::renderer::cube_render_target;
use three_rs::Timer;
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
    /// The page's module-level `timer`.
    pub timer: Timer,
}

/// `fitCameraToSelection( camera, controls, selection, fitOffset = 1.3 )`,
/// transcribed. `target` is `controls.target`, which the page has set to
/// `( 0, 0, - 0.2 )` and which this leaves holding the model's centre.
fn fit_camera_to_selection(
    camera: &mut PerspectiveCamera,
    target: &mut Vector3,
    selection: &three_rs::Node,
    fit_offset: f64,
) {
    let mut bounds = Box3::default();
    bounds.set_from_object(selection, false);

    let size = bounds.get_size();
    let center = bounds.get_center();

    let max_size = size.x.max(size.y).max(size.z);
    let fit_height_distance = max_size / (2.0 * (std::f64::consts::PI * camera.fov / 360.0).atan());
    // `const fitWidthDistance = fitHeightDistance / camera.aspect;`
    // `const distance = fitOffset * Math.max( fitHeightDistance, fitWidthDistance );`
    // are commented out upstream: only the height fit is used.
    let distance = fit_offset * fit_height_distance;

    let position = camera.node.borrow().position;
    let direction = *target
        .clone()
        .sub(&position)
        .normalize()
        .multiply_scalar(distance);

    // `controls.maxDistance` / `minDistance` are set here too; with no pointer
    // events they clamp nothing.
    *target = center;

    camera.near = distance / 100.0;
    camera.far = distance * 100.0;
    camera.update_projection_matrix();

    camera.node.borrow_mut().position = *target.clone().sub(&direction);

    // `controls.update()`.
    camera.look_at(target);
}

pub fn init() -> App {
    // `new THREE.PerspectiveCamera( 45, window.innerWidth / window.innerHeight,
    // 0.25, 20 )`, then `camera.position.set( - 1.8, 0.6, 2.7 )`.
    let mut camera = PerspectiveCamera::new(45.0, INNER_WIDTH / INNER_HEIGHT, 0.25, 20.0);
    camera.node.borrow_mut().position.set(-1.8, 0.6, 2.7);

    let mut scene = Scene::new();

    // `renderer = new THREE.WebGPURenderer( { antialias: true } )`. The page
    // builds it after the loader call; here it is first, because the two
    // environment conversions below take it.
    let mut renderer = Renderer::new(RendererParameters { antialias: true }).unwrap();
    renderer.set_pixel_ratio(DPR);
    renderer.set_size(INNER_WIDTH, INNER_HEIGHT);
    renderer.tone_mapping = ToneMapping::AcesFilmic;

    // `new UltraHDRLoader().setPath( 'textures/equirectangular/' ).load(
    // 'royal_esplanade_2k.hdr.jpg', … )`. The loader resolves synchronously
    // here, so the callback's body is written inline.
    let texture = UltraHdrLoader::new()
        .load(examples_dir().join("textures/equirectangular/royal_esplanade_2k.hdr.jpg"))
        .unwrap();

    // `texture.mapping = THREE.EquirectangularReflectionMapping; scene.background
    // = texture`: `CubeMapNode.updateBefore()` converts the 2048×1024 map into
    // a 512² cube once and the skybox samples that, at `backgroundBlurriness`
    // 0 and `backgroundIntensity` 1.
    let background = cube_render_target::from_equirectangular_texture(&mut renderer, &texture)
        .expect("the equirectangular background converts to a cube");
    scene.background = Some(Background::CubeTexture(background));

    // `scene.environment = texture`: the same map, PMREM-filtered.
    let mut environment = PmremEnvironment::from_equirectangular(&texture);
    environment.update(&mut renderer).unwrap();
    scene.environment = Some(environment.handle());

    // `loadModel( { name: 'DamagedHelmet', … } )` — the GLB from Khronos, which
    // is the checkout's own `DamagedHelmet.gltf` (module docs). Five external
    // JPEG maps: albedo, metalRoughness, normal, emissive and AO.
    let gltf =
        GLTFLoader::load(examples_dir().join("models/gltf/DamagedHelmet/glTF/DamagedHelmet.gltf"))
            .expect("DamagedHelmet.gltf");

    // `await renderer.compileAsync( currentModel, camera, scene )` before the
    // add: it only warms the pipeline cache, and this port compiles on the
    // first draw either way.
    scene.add(&gltf.scene);

    // `new OrbitControls( … )` with `target.set( 0, 0, - 0.2 )` and one
    // `update()`, then `fitCameraToSelection` replaces almost all of it.
    let mut target = Vector3::new(0.0, 0.0, -0.2);
    camera.look_at(&target);
    fit_camera_to_selection(&mut camera, &mut target, &gltf.scene, 1.3);

    App {
        // `const timer = new THREE.Timer();` — constructed in `init()`, as the
        // page does, so its `_startTime` is the moment the scene was built.
        timer: Timer::new(),
        renderer,
        scene,
        camera,
        environment,
    }
}

/// The page's `render()`. `timer.update()` and `controls.update()` move
/// nothing, and `mixer` is never created — DamagedHelmet has no animations.
pub fn animate(app: &mut App) {
    // `timer.update()`. DamagedHelmet has no animations, so the page never
    // builds the `AnimationMixer` this delta would drive and nothing reads it;
    // the call is here because the page makes it.
    app.timer.update();

    app.environment.update(&mut app.renderer).unwrap();
    app.renderer.render(&mut app.scene, &mut app.camera);
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
        .unwrap_or_else(|| "target/webgpu_loader_gltf.png".to_string());
    three_rs::testing::write_png(&path, width, height, &pixels);
    println!("wrote {path} ({width}x{height})");
}
