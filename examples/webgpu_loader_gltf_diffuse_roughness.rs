//! Port of `three.js/examples/webgpu_loader_gltf_diffuse_roughness.html`,
//! calling the three-rs API in the same order the page's `init()` does.
//!
//! Under the e2e harness the viewport is `400 * 2` x `250 * 2` with no
//! `deviceScaleFactor`, so `window.innerWidth` / `innerHeight` /
//! `devicePixelRatio` are 800, 500 and 1.
//!
//! The page draws from `Math.random()` not at all and has no animation:
//! `animate()` is `controls.update()` (damping with no pointer input moves
//! nothing) and one `render()`.
//!
//! # What this page is
//!
//! The `KHR_materials_diffuse_roughness` parameter sweep from
//! glTF-Sample-Assets: 24 spheres in three rows, lit only by a
//! `RoomEnvironment` PMREM that is also the (half-blurred) background.
//!
//! * the top row alternates a plain `MeshStandardMaterial` with a
//!   `MeshPhysicalMaterial` at `diffuseRoughness = 1`, over four albedos;
//! * the middle and bottom rows sweep `diffuseRoughness` from 0 to 1 at a
//!   specular roughness of 0.95 and 0.35.
//!
//! Any `diffuseRoughness > 0` turns on `useDiffuseRoughness`, which swaps
//! `PhysicalLightingModel`'s Lambert lobe for the energy-preserving
//! Oren–Nayar one (`BRDF_EON.js`); with no lights, what the spheres show is
//! its directional albedo in the two indirect terms. `docs/nodes.md` §54.
//!
//! **The winding flip.** The draft asset is wound clockwise, so the page
//! swaps the second and third index of every triangle before adding the
//! model — once per geometry, whichever meshes share it.

use std::collections::HashMap;
use std::rc::Rc;

use three_rs::addons::controls::OrbitControls;
use three_rs::core::BufferGeometry;
use three_rs::loaders::GltfLoader;
use three_rs::materials::ToneMapping;
use three_rs::nodes::pmrem_node::PmremEnvironment;
use three_rs::objects::Background;
use three_rs::{PerspectiveCamera, Renderer, RendererParameters, RoomEnvironment, Scene};

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
}

pub fn init() -> App {
    // `new THREE.WebGPURenderer( { antialias: true } )` with
    // `NeutralToneMapping`.
    let mut parameters = RendererParameters::default();
    parameters.antialias = true;
    let mut renderer = Renderer::new(parameters).unwrap();
    renderer.set_pixel_ratio(DPR);
    renderer.set_size(INNER_WIDTH, INNER_HEIGHT);
    renderer.tone_mapping = ToneMapping::Neutral;

    let mut scene = Scene::new();
    scene.background_blurriness = 0.5;

    // `pmremGenerator.fromScene( new RoomEnvironment(), 0.04 ).texture`, as
    // both the background and the environment. With `backgroundBlurriness`
    // non-zero the skybox is a cubeUV read of the same PMREM.
    let mut room = RoomEnvironment::new();
    let environment = PmremEnvironment::from_scene(&mut renderer, &mut room, 0.04).unwrap();
    scene.background = Some(Background::Pmrem(environment.handle()));
    scene.environment = Some(environment.handle());

    // `new THREE.PerspectiveCamera( 35, window.innerWidth /
    // window.innerHeight, 0.1, 100 )`, `camera.position.set( 0, 0, 10 )`.
    let mut camera = PerspectiveCamera::new(35.0, INNER_WIDTH / INNER_HEIGHT, 0.1, 100.0);
    camera.node.borrow_mut().position.set(0.0, 0.0, 10.0);

    // `controls = new OrbitControls( camera, renderer.domElement );
    // controls.enableDamping = true;` — the target stays at the origin.
    let mut controls = OrbitControls::new(&mut camera);
    controls.set_element_size(INNER_WIDTH, INNER_HEIGHT);
    controls.enable_damping = true;

    // `await new GLTFLoader().setPath( 'models/gltf/' ).loadAsync(
    // 'DiffuseRoughnessParameterSweep.glb' )`.
    let gltf =
        GltfLoader::load(examples_dir().join("models/gltf/DiffuseRoughnessParameterSweep.glb"))
            .expect("DiffuseRoughnessParameterSweep.glb");

    // "The draft sample asset currently uses clockwise triangle winding." The
    // page flips each geometry once (its `geometries` set); a geometry here is
    // an `Rc`, so the flipped copy replaces every mesh's handle to it.
    let mut flipped: HashMap<*const BufferGeometry, Rc<BufferGeometry>> = HashMap::new();
    gltf.scene.traverse(&mut |node| {
        let mut object = node.borrow_mut();
        let Some(mesh) = object.mesh_mut() else {
            return;
        };
        let key = Rc::as_ptr(&mesh.geometry);
        let geometry = flipped
            .entry(key)
            .or_insert_with(|| {
                let mut geometry = (*mesh.geometry).clone();
                let index = geometry.index.as_mut().expect("an indexed geometry");
                for i in (0..index.count()).step_by(3) {
                    let second = index.get_x(i + 1);
                    let third = index.get_x(i + 2);
                    index.set_x(i + 1, third);
                    index.set_x(i + 2, second);
                }
                Rc::new(geometry)
            })
            .clone();
        mesh.geometry = geometry;
    });

    scene.add(&gltf.scene);

    App {
        renderer,
        scene,
        camera,
        environment,
        controls,
    }
}

/// The page's `animate()`: `controls.update()` and one render.
pub fn animate(app: &mut App) {
    app.controls.update(&mut app.camera, None);

    app.environment.update(&mut app.renderer).unwrap();
    app.renderer.render(&mut app.scene, &mut app.camera);
}

/// The page's `onWindowResize()`.
///
/// The rung harness never calls this — the graded frame is always
/// 800 x 500 — but the viewer and the browser shell do.
pub fn resize(app: &mut App, width: f64, height: f64) {
    app.camera.aspect = width / height;
    app.camera.update_projection_matrix();

    app.renderer.set_size(width, height);
}

/// The example's controls, for a host that has a pointer.
pub fn controls(app: &mut App) -> Option<&mut OrbitControls> {
    Some(&mut app.controls)
}

/// The controls and the camera at once, which every one of the controls'
/// event handlers needs.
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
        .unwrap_or_else(|| "target/webgpu_loader_gltf_diffuse_roughness.png".to_string());
    three_rs::testing::write_png(&path, width, height, &pixels);
    println!("wrote {path} ({width}x{height})");
}
