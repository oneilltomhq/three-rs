//! Port of `three.js/examples/webgpu_loader_gltf_iridescence.html`, calling the
//! three-rs API in the same order the page's `init()` does.
//!
//! Under the e2e harness the viewport is `400 * 2` x `250 * 2` with no
//! `deviceScaleFactor`, so `window.innerWidth` / `innerHeight` /
//! `devicePixelRatio` are 800, 500 and 1.
//!
//! The page has no clock and no `Math.random()`; `render()` is
//! `controls.update()` (a no-op without pointer events) and one
//! `renderer.render( scene, camera )`.
//!
//! # What this page is
//!
//! `IridescenceLamp.glb` is the Khronos sample asset for
//! `KHR_materials_iridescence`, and this page is the only graded example that
//! reaches `evalIridescence`. The lamp is three meshes sharing one base colour
//! and one occlusion/roughness/metalness map:
//!
//! * `lamp` — no extension at all, a plain glTF PBR material;
//! * `lamp_iridescence` — `iridescenceFactor` 1, IOR 1.8, thickness
//!   `[ 485, 515 ]` nm through the green channel of
//!   `IridescenceLamp_Iridescence.png`;
//! * `lamp_transmission` — the same at IOR 1.67 and `[ 395, 405 ]` nm, plus
//!   `KHR_materials_transmission` (factor 1) and `KHR_materials_volume`
//!   (thickness 0.005), so it is drawn in the transmission pass over the
//!   copied opaque frame.
//!
//! Three things are easy to lose:
//!
//! * **The scene has no lights.** Every lit pixel comes from
//!   `scene.environment`, so `PhysicalLightingModel.direct()` is never called
//!   and the `USE_IRIDESCENCE` branch of `BRDF_GGX` never reaches a shader.
//!   All the iridescence on screen arrives through the iridescent F0 in
//!   `computeMultiscattering` — `docs/nodes.md` §30.
//! * **The environment is a Radiance `.hdr`,** not an UltraHDR JPEG: this is
//!   the first graded page to take [`HdrLoader`] straight to a PMREM and to a
//!   cube background.
//! * **`backgroundBlurriness` is 0,** so the skybox is the sharp cube
//!   `CubeMapNode` converts the equirect into, not a PMREM read.
//!
//! [`HdrLoader`]: three_rs::loaders::HdrLoader

use three_rs::loaders::{GLTFLoader, HdrLoader};
use three_rs::materials::ToneMapping;
use three_rs::nodes::pmrem_node::PmremEnvironment;
use three_rs::objects::Background;
use three_rs::renderer::cube_render_target;
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
    // `new THREE.WebGPURenderer( { antialias: true } )` and the tone mapping.
    let mut renderer = Renderer::new(RendererParameters { antialias: true }).unwrap();
    renderer.set_pixel_ratio(DPR);
    renderer.set_size(INNER_WIDTH, INNER_HEIGHT);
    renderer.tone_mapping = ToneMapping::AcesFilmic;

    let mut scene = Scene::new();

    // `new THREE.PerspectiveCamera( 50, window.innerWidth / window.innerHeight,
    // 0.05, 20 )`, `camera.position.set( 0.35, 0.05, 0.35 )`.
    let mut camera = PerspectiveCamera::new(50.0, INNER_WIDTH / INNER_HEIGHT, 0.05, 20.0);
    camera.node.borrow_mut().position.set(0.35, 0.05, 0.35);

    // `new OrbitControls( … )`; `autoRotate` needs a clock the page never
    // ticks, so `controls.target.set( 0, 0.2, 0 )` and one `update()` is a
    // `lookAt`.
    camera.look_at(&Vector3::new(0.0, 0.2, 0.0));

    // `new HDRLoader().setPath( 'textures/equirectangular/' ).loadAsync(
    // 'venice_sunset_1k.hdr' )` — synchronous here, so the `await
    // Promise.all( … )` body follows inline.
    let texture = HdrLoader::new()
        .load(examples_dir().join("textures/equirectangular/venice_sunset_1k.hdr"))
        .unwrap();

    // `texture.mapping = THREE.EquirectangularReflectionMapping; scene.background
    // = texture`: with `backgroundBlurriness` 0, `CubeMapNode.updateBefore()`
    // converts the 1024×512 map into a 512² cube once and the skybox samples
    // that at level 0.
    let background = cube_render_target::from_equirectangular_texture(&mut renderer, &texture)
        .expect("the equirectangular background converts to a cube");
    scene.background = Some(Background::CubeTexture(background));

    // `scene.environment = texture`: the same map, PMREM-filtered.
    let mut environment = PmremEnvironment::from_equirectangular(&texture);
    environment.update(&mut renderer).unwrap();
    scene.environment = Some(environment.handle());

    // `new GLTFLoader().setPath( 'models/gltf/' ).loadAsync(
    // 'IridescenceLamp.glb' )`, then `scene.add( gltf.scene )`.
    let gltf = GLTFLoader::load(examples_dir().join("models/gltf/IridescenceLamp.glb"))
        .expect("IridescenceLamp.glb");
    scene.add(&gltf.scene);

    App {
        renderer,
        scene,
        camera,
        environment,
    }
}

/// The page's `render()`: one render, nothing moved.
pub fn animate(app: &mut App) {
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
        .unwrap_or_else(|| "target/webgpu_loader_gltf_iridescence.png".to_string());
    three_rs::testing::write_png(&path, width, height, &pixels);
    println!("wrote {path} ({width}x{height})");
}
