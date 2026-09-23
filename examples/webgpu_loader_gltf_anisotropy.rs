//! Port of `three.js/examples/webgpu_loader_gltf_anisotropy.html`, calling the
//! three-rs API in the same order the page's `init()` does.
//!
//! Under the e2e harness the viewport is `400 * 2` x `250 * 2` with no
//! `deviceScaleFactor`, so `window.innerWidth` / `innerHeight` /
//! `devicePixelRatio` are 800, 500 and 1.
//!
//! The page has no clock and no `Math.random()`; `animate()` is one
//! `renderer.render( scene, camera )`.
//!
//! # What this page is
//!
//! `AnisotropyBarnLamp.glb` is the Khronos sample asset that exercises
//! `KHR_materials_anisotropy`, and the page is the only graded example that
//! reaches the anisotropic branch of `PhysicalLightingModel`. The lamp is three
//! meshes: a brushed metal shade (anisotropy 0.9 with an `anisotropyMap`, plus
//! a clearcoat), an emissive filament, and a glass cover with
//! `KHR_materials_transmission` / `_volume`.
//!
//! Four things are easy to lose:
//!
//! * **The scene has no lights.** Every lit pixel comes from
//!   `scene.environment`, so `PhysicalLightingModel.direct()` is never called
//!   and the anisotropic GGX (`D_GGX_Anisotropic` /
//!   `V_GGX_SmithCorrelated_Anisotropic`) never reaches a shader. Anisotropy is
//!   visible here only through the bent normal that the radiance reflect vector
//!   is built from — `docs/nodes.md` §26.
//! * **The shade geometry carries a `TANGENT` attribute.** Anisotropy needs a
//!   tangent frame, and with the attribute present Three takes the
//!   `Tangent.js` / `Bitangent.js` path rather than the derivative frame the
//!   port had. The two produce visibly different highlights.
//! * **`scene.backgroundBlurriness = 0.5`.** The background is the PMREM read
//!   at that roughness, not the sharp cube every earlier environment example
//!   draws.
//! * **The glass is transmissive.** That is the one piece of the page the port
//!   does not have; see the progress doc.
//!
//! [`Scene::background_blurriness`]: three_rs::Scene::background_blurriness

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
}

pub fn init() -> App {
    // `new THREE.WebGPURenderer( { antialias: true } )`, then the tone mapping
    // pair. The page builds the renderer first; the environment conversion
    // below needs it, so the order is the page's.
    let mut renderer = Renderer::new(RendererParameters { antialias: true }).unwrap();
    renderer.set_pixel_ratio(DPR);
    renderer.set_size(INNER_WIDTH, INNER_HEIGHT);
    renderer.tone_mapping = ToneMapping::AcesFilmic;
    renderer.tone_mapping_exposure = 1.35;

    let mut scene = Scene::new();

    // `new THREE.PerspectiveCamera( 40, window.innerWidth / window.innerHeight,
    // 0.01, 10 )`, `camera.position.set( - 0.35, - 0.2, 0.35 )`.
    let mut camera = PerspectiveCamera::new(40.0, INNER_WIDTH / INNER_HEIGHT, 0.01, 10.0);
    camera.node.borrow_mut().position.set(-0.35, -0.2, 0.35);

    // `new OrbitControls( … )`; `controls.target.set( 0, - 0.08, 0.11 )` and
    // one `update()`, which with no pointer events is a `lookAt`. The distance
    // is 0.52, inside `[ minDistance, maxDistance ]`, so the clamps do nothing.
    camera.look_at(&Vector3::new(0.0, -0.08, 0.11));

    // `new UltraHDRLoader().setPath( 'textures/equirectangular/' )` then
    // `loadAsync( 'royal_esplanade_2k.hdr.jpg' )`. The loader is synchronous
    // here, so the `await Promise.all( … )` body follows inline.
    let texture = UltraHdrLoader::new()
        .load(examples_dir().join("textures/equirectangular/royal_esplanade_2k.hdr.jpg"))
        .unwrap();

    // `texture.mapping = THREE.EquirectangularReflectionMapping;`
    // `scene.environment = texture` PMREM-filters it once.
    let mut environment = PmremEnvironment::from_equirectangular(&texture);
    environment.update(&mut renderer).unwrap();

    // `scene.background = texture` with `backgroundBlurriness = 0.5`: because
    // the blur is non-zero, `Background.update()`'s `getTextureLevel` context
    // makes the skybox a cubeUV read of the same PMREM, not the sharp cube.
    scene.background = Some(Background::Pmrem(environment.handle()));
    scene.background_blurriness = 0.5;
    scene.environment = Some(environment.handle());

    // `new GLTFLoader().setPath( 'models/gltf/' ).loadAsync(
    // 'AnisotropyBarnLamp.glb' )`, then `scene.add( gltf.scene )`.
    let gltf = GLTFLoader::load(examples_dir().join("models/gltf/AnisotropyBarnLamp.glb"))
        .expect("AnisotropyBarnLamp.glb");
    scene.add(&gltf.scene);

    App {
        renderer,
        scene,
        camera,
        environment,
    }
}

/// The page's `animate()`: one render, nothing moved.
pub fn animate(app: &mut App) {
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
        .unwrap_or_else(|| "target/webgpu_loader_gltf_anisotropy.png".to_string());
    three_rs::testing::write_png(&path, width, height, &pixels);
    println!("wrote {path} ({width}x{height})");
}
