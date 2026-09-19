//! Port of `three.js/examples/webgpu_loader_gltf_sheen.html`, calling the
//! three-rs API in the same order the page's `init()` and its two loader
//! callbacks do.
//!
//! Under the e2e harness the viewport is `400 * 2` x `250 * 2` with no
//! `deviceScaleFactor`, so `window.innerWidth` / `innerHeight` /
//! `devicePixelRatio` are 800, 500 and 1.
//!
//! The page draws from `Math.random()` not at all and has no animation of any
//! kind: `animate()` is `controls.update()` (damping with no pointer input
//! moves nothing) and one `render()`. The harness' pinned clock is never
//! observed.
//!
//! # What this page is
//!
//! `webgpu_loader_gltf`'s environment, with a different model in front of it —
//! the same UltraHDR equirectangular map as both `scene.background` (converted
//! to a 512² cube and drawn as the skybox) and `scene.environment`
//! (PMREM-filtered). What is new is entirely in the model:
//!
//! * **`KHR_materials_sheen`.** `SheenChair_fabric` is the ladder's first
//!   `MeshPhysicalMaterial` with `sheen > 0`, which turns on the sheen half of
//!   `PhysicalLightingModel`: a retroreflective lobe on top of the usual
//!   specular one, and an energy compensation that takes what the lobe
//!   reflected away from the layers underneath. `docs/nodes.md` §25.
//! * **`KHR_texture_transform`.** Every map in the asset carries one. The
//!   fabric's base colour is tiled seven times (`offset ( -3, 3 ), scale ( 7,
//!   7 )`), its normal map twice, and the wood's two maps are also *rotated* —
//!   which glTF composes as `T * R * S` where three.js' `Texture.updateMatrix`
//!   composes `T * S * R`, so the loader writes that matrix itself.
//! * **`TEXCOORD_1`.** The occlusion map of all four materials is `texCoord:
//!   1`, so this is the first page whose fragment shader reads two uv sets.
//!
//! There are **no lights in the scene** — the page's one
//! `DirectionalLight` is commented out upstream. Everything visible is
//! indirect, out of the PMREM, which is why the sheen shows as a rim on the
//! fabric rather than a highlight.
//!
//! # What the page's GUI is
//!
//! `renderer.inspector.createParameters( 'SheenChair_fabric' )` adds one
//! slider, `material.sheen` over `[ 0, 1 ]`. The graded frame is the untouched
//! asset value, which `GLTFMaterialsSheenExtension` sets to 1.

use three_rs::loaders::{GLTFLoader, UltraHdrLoader};
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
    // `new THREE.PerspectiveCamera( 45, window.innerWidth / window.innerHeight,
    // 0.1, 20 )`, then `camera.position.set( - 0.75, 0.7, 1.25 )`.
    let mut camera = PerspectiveCamera::new(45.0, INNER_WIDTH / INNER_HEIGHT, 0.1, 20.0);
    camera.node.borrow_mut().position.set(-0.75, 0.7, 1.25);

    let mut scene = Scene::new();
    // `//scene.add( new THREE.DirectionalLight( 0xffffff, 2 ) )` — commented
    // out upstream, and the reason the fabric is lit only by the environment.

    // `renderer = new THREE.WebGPURenderer( { antialias: true } )`. The page
    // builds it after the loader call; here it is first, because the two
    // environment conversions below take it.
    let mut renderer = Renderer::new(RendererParameters { antialias: true }).unwrap();
    renderer.set_pixel_ratio(DPR);
    renderer.set_size(INNER_WIDTH, INNER_HEIGHT);
    renderer.tone_mapping = ToneMapping::AcesFilmic;
    // `renderer.toneMappingExposure = 1` is the default.

    // `new GLTFLoader().setPath( 'models/gltf/' ).load( 'SheenChair.glb', … )`.
    // The loader resolves synchronously here, so the callback's body is written
    // inline; `renderer.inspector.createParameters` and its one `sheen` slider
    // are the rest of it.
    let gltf = GLTFLoader::load(examples_dir().join("models/gltf/SheenChair.glb"))
        .expect("SheenChair.glb");
    scene.add(&gltf.scene);

    // `scene.background = new THREE.Color( 0xAAAAAA )`, replaced below by the
    // UltraHDR map before a frame is ever drawn. It is written here only
    // because the page writes it here, and because a reader looking for the
    // grey has to find it.
    scene.background = Some(Background::Color(three_rs::math::Color::from_hex(0xAAAAAA)));

    // `new UltraHDRLoader().setPath( 'textures/equirectangular/' ).load(
    // 'royal_esplanade_2k.hdr.jpg', … )`, again synchronous here.
    let texture = UltraHdrLoader::new()
        .load(examples_dir().join("textures/equirectangular/royal_esplanade_2k.hdr.jpg"))
        .unwrap();

    // `texture.mapping = THREE.EquirectangularReflectionMapping; scene.background
    // = texture`: `CubeMapNode.updateBefore()` converts the 2048×1024 map into
    // a 512² cube once and the skybox samples that.
    //
    // `//scene.backgroundBlurriness = 1; // @TODO: Needs PMREM` is commented
    // out upstream, so the skybox is the sharp cube.
    let background = cube_render_target::from_equirectangular_texture(&mut renderer, &texture)
        .expect("the equirectangular background converts to a cube");
    scene.background = Some(Background::CubeTexture(background));

    // `scene.environment = texture`: the same map, PMREM-filtered.
    let mut environment = PmremEnvironment::from_equirectangular(&texture);
    environment.update(&mut renderer).unwrap();
    scene.environment = Some(environment.handle());

    // `new OrbitControls( camera, renderer.domElement )` with `target.set( 0,
    // 0.35, 0 )` and one `update()`. `enableDamping` / `minDistance` /
    // `maxDistance` clamp nothing with no pointer events.
    camera.look_at(&Vector3::new(0.0, 0.35, 0.0));

    App {
        renderer,
        scene,
        camera,
        environment,
    }
}

/// The page's `render()`, with `controls.update()` ahead of it — damping with
/// no pointer input moves the camera nowhere.
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
        .unwrap_or_else(|| "target/webgpu_loader_gltf_sheen.png".to_string());
    three_rs::testing::write_png(&path, width, height, &pixels);
    println!("wrote {path} ({width}x{height})");
}
