//! Port of `three.js/examples/webgpu_pmrem_cubemap.html`, calling the three-rs
//! API in the same order the page's `init()` / `render()` do.
//!
//! Under the e2e harness the viewport is `400 * 2` x `250 * 2` with no
//! `deviceScaleFactor`, so `window.innerWidth` / `innerHeight` /
//! `devicePixelRatio` are 800, 500 and 1.
//!
//! The page draws from `Math.random()` not at all, so the harness' seeded
//! sequence is never touched.
//!
//! `HDRCubeTextureLoader` resolves synchronously here, so everything inside
//! its callback — the background node and the thirty spheres — is written
//! inline.
//!
//! # The PMREM
//!
//! The page hands the *raw* HDR cube to both `pmremTexture( map, … )` and
//! `envMap: map`, and three turns each into a `PMREMNode` that generates the
//! atlas on demand, sharing one per texture. This port makes that sharing
//! explicit: one [`PmremEnvironment`], built once by
//! [`PmremEnvironment::update`] before the first render, read by the
//! background node and by every sphere's `pmrem_env` handle.

use std::rc::Rc;

use three_rs::geometries::sphere_geometry;
use three_rs::loaders::HdrCubeTextureLoader;
use three_rs::materials::ToneMapping;
use three_rs::math::Vector3;
use three_rs::nodes::pmrem_node::PmremEnvironment;
use three_rs::nodes::tsl::{normal_world_geometry, uniform_settable};
use three_rs::nodes::Type;

use three_rs::{
    Background, Color, Mesh, MeshPhysicalNodeMaterial, PerspectiveCamera, Renderer,
    RendererParameters, Scene,
};

pub const INNER_WIDTH: f64 = 800.0;
pub const INNER_HEIGHT: f64 = 500.0;
/// `renderer.setPixelRatio( window.devicePixelRatio )`.
pub const DPR: f64 = 1.0;

pub struct App {
    pub renderer: Renderer,
    pub scene: Scene,
    pub camera: PerspectiveCamera,
    pub environment: PmremEnvironment,
}

fn examples_dir() -> std::path::PathBuf {
    three_rs::testing::three_js_dir().join("examples")
}

pub fn init() -> App {
    let mut camera = PerspectiveCamera::new(45.0, INNER_WIDTH / INNER_HEIGHT, 0.25, 20.0);
    camera.node.borrow_mut().position.set(0.0, 0.0, 8.0);

    let mut scene = Scene::new();

    let mut renderer = Renderer::new(RendererParameters { antialias: true }).unwrap();
    renderer.set_pixel_ratio(DPR);
    renderer.set_size(INNER_WIDTH, INNER_HEIGHT);
    renderer.tone_mapping = ToneMapping::AcesFilmic;

    // `new OrbitControls( camera, renderer.domElement )` then
    // `controls.update()`: with no pointer events the orbit is the identity,
    // but `update()` still points the camera at the target. At ( 0, 0, 8 )
    // looking at the origin that is the identity too.
    camera.look_at(&Vector3::ZERO);

    let path = examples_dir().join("textures/cube/pisaHDR/");
    let map = HdrCubeTextureLoader::new()
        .set_path(path)
        .load(["px.hdr", "nx.hdr", "py.hdr", "ny.hdr", "pz.hdr", "nz.hdr"])
        .unwrap();

    let mut environment = PmremEnvironment::new(&map);
    // `PMREMNode.updateBefore()` — see `docs/nodes.md` §12 for why the trigger
    // is here rather than inside the node.
    environment.update(&mut renderer).unwrap();

    // `scene.backgroundNode = pmremTexture( map, normalWorldGeometry,
    // uniform( 0.5 ) )`.
    let (level, _level) = uniform_settable(Type::F32, vec![0.5]);
    scene.background = Some(Background::Node(
        environment.sample(normal_world_geometry(), level),
    ));

    // `new THREE.SphereGeometry( 0.4, 64, 64 )` — one geometry, thirty meshes.
    let geometry = Rc::new(sphere_geometry(0.4, 64, 64));

    for i in 0..6 {
        for j in 0..5 {
            let mut material = MeshPhysicalNodeMaterial::physical(
                Color::new(1.0, 1.0, 1.0),
                i as f64 / 5.0,
                j as f64 / 4.0,
            );
            material.pmrem_env = Some(environment.handle());

            let mesh = Mesh::new(geometry.clone(), material);
            {
                let mut object = mesh.borrow_mut();
                object.position.x = i as f64 - 2.5;
                object.position.y = j as f64 - 2.0;
            }
            scene.add(&mesh);
        }
    }

    App {
        renderer,
        scene,
        camera,
        environment,
    }
}

/// The page's `render()`.
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
        .unwrap_or_else(|| "target/webgpu_pmrem_cubemap.png".to_string());
    three_rs::testing::write_png(&path, width, height, &pixels);
    println!("wrote {path} ({width}x{height})");
}
