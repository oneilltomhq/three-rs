//! Port of `three.js/examples/webgpu_pmrem_equirectangular.html`, calling the
//! three-rs API in the same order the page's `init()` and its loader callback
//! do.
//!
//! Under the e2e harness the viewport is `400 * 2` x `250 * 2` with no
//! `deviceScaleFactor`, so `window.innerWidth` / `innerHeight` /
//! `devicePixelRatio` are 800, 500 and 1.
//!
//! The page draws from `Math.random()` not at all and reads no clock, so the
//! harness' seeded sequence and pinned `performance.now` are never touched.
//!
//! # What this rung is actually about
//!
//! The picture is `webgpu_pmrem_test`'s: an equirectangular HDR through
//! `PMREMGenerator.fromEquirectangular`, a grid of `MeshPhysicalNodeMaterial`
//! spheres over roughness and metalness, and the same environment as the
//! background. Everything on the rendering side was already ported.
//!
//! What is new is the **file**: `royal_esplanade_2k.hdr.jpg` is an UltraHDR
//! image — a baseline JPEG carrying a second JPEG (the gain map) and the
//! metadata to recombine them into HDR. That is
//! [`UltraHdrLoader`](three_rs::loaders::UltraHdrLoader), and it is where every
//! pixel of this example comes from.
//!
//! # The two things that are easy to get wrong here
//!
//! **The background is a `backgroundNode`, not a `background`.** The page
//! writes `scene.backgroundNode = pmremTexture( map, normalWorldGeometry,
//! uniform( 0.5 ) )`, so the skybox samples the atlas at a *fixed* roughness
//! of 0.5 with no `backgroundRotation` and no `backgroundBlurriness` — a
//! visibly blurred environment, not the sharp one `scene.background = map`
//! would give. `Background::Pmrem` is the other branch and is wrong here.
//!
//! **`envMap` is the equirect map, not the PMREM.** Three's `EnvironmentNode`
//! wraps a non-cubeUV `envMap` in `pmremTexture( value )` and caches the
//! generated atlas per renderer *keyed on the source texture*, so the
//! background's `pmremTexture( map, … )` and all thirty materials' `envMap`
//! share one PMREM. The port has one [`PmremEnvironment`] for the same reason,
//! and generates it once.

use std::rc::Rc;

use three_rs::geometries::sphere_geometry;
use three_rs::loaders::UltraHdrLoader;
use three_rs::materials::ToneMapping;
use three_rs::nodes::node::Type;
use three_rs::nodes::pmrem_node::PmremEnvironment;
use three_rs::nodes::tsl::{normal_world_geometry, uniform_settable};

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
    // `new THREE.PerspectiveCamera( 45, window.innerWidth / window.innerHeight,
    // 0.25, 20 )`, then `camera.position.set( 0, 0, 8 )`.
    let camera = PerspectiveCamera::new(45.0, INNER_WIDTH / INNER_HEIGHT, 0.25, 20.0);
    camera.node.borrow_mut().position.set(0.0, 0.0, 8.0);

    let mut scene = Scene::new();

    let mut renderer = Renderer::new(RendererParameters { antialias: true }).unwrap();
    renderer.set_pixel_ratio(DPR);
    renderer.set_size(INNER_WIDTH, INNER_HEIGHT);
    renderer.tone_mapping = ToneMapping::AcesFilmic;

    // `new OrbitControls( camera, renderer.domElement )` with `minDistance` /
    // `maxDistance` set, then `controls.update()`: with no pointer events the
    // orbit is the identity, and at ( 0, 0, 8 ) looking down -Z the camera is
    // already pointed at the target. Inert for the graded frame.

    // `new UltraHDRLoader().setPath( 'textures/equirectangular/' ).load(
    // 'royal_esplanade_2k.hdr.jpg', … )`. The loader resolves synchronously
    // here, so the callback's body is written inline.
    let map = UltraHdrLoader::new()
        .load(examples_dir().join("textures/equirectangular/royal_esplanade_2k.hdr.jpg"))
        .unwrap();

    // `map.mapping = THREE.EquirectangularReflectionMapping` — which is how
    // three decides between `fromCubemap` and `fromEquirectangular`. The port
    // carries that decision in `PmremSource`, so the equirect entry point is
    // named rather than inferred.
    let mut environment = PmremEnvironment::from_equirectangular(&map);
    // `PMREMNode.updateBefore()` — see `docs/nodes.md` §12 for why the trigger
    // is here rather than inside the node.
    environment.update(&mut renderer).unwrap();

    // `scene.backgroundNode = pmremTexture( map, normalWorldGeometry,
    // uniform( 0.5 ) )`.
    let (roughness, _roughness) = uniform_settable(Type::F32, vec![0.5]);
    scene.background = Some(Background::Node(
        environment.sample(normal_world_geometry(), roughness),
    ));

    // `new THREE.SphereGeometry( 0.4, 64, 64 )` — one geometry, thirty meshes,
    // thirty materials.
    let geometry = Rc::new(sphere_geometry(0.4, 64, 64));

    for i in 0..6 {
        for j in 0..5 {
            // `new THREE.MeshPhysicalNodeMaterial( { roughness: i / 5,
            // metalness: j / 4, envMap: map } )` — `color` is the white the
            // constructor defaults to.
            let mut material = MeshPhysicalNodeMaterial::physical(
                Color::from_hex(0xffffff),
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
        .unwrap_or_else(|| "target/webgpu_pmrem_equirectangular.png".to_string());
    three_rs::testing::write_png(&path, width, height, &pixels);
    println!("wrote {path} ({width}x{height})");
}
