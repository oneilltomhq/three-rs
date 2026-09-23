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
//! [`three_rs::loaders::UltraHdrLoader`], and it is where every
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

use three_rs::addons::controls::OrbitControls;
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
    /// The page's `controls`.
    pub controls: OrbitControls,
    pub environment: PmremEnvironment,
}

fn examples_dir() -> std::path::PathBuf {
    three_rs::testing::three_js_dir().join("examples")
}

pub fn init() -> App {
    // `new THREE.PerspectiveCamera( 45, window.innerWidth / window.innerHeight,
    // 0.25, 20 )`, then `camera.position.set( 0, 0, 8 )`.
    let mut camera = PerspectiveCamera::new(45.0, INNER_WIDTH / INNER_HEIGHT, 0.25, 20.0);
    camera.node.borrow_mut().position.set(0.0, 0.0, 8.0);

    let mut scene = Scene::new();

    let mut renderer = Renderer::new(RendererParameters { antialias: true }).unwrap();
    renderer.set_pixel_ratio(DPR);
    renderer.set_size(INNER_WIDTH, INNER_HEIGHT);
    renderer.tone_mapping = ToneMapping::AcesFilmic;

    // `const controls = new OrbitControls( camera, renderer.domElement );`
    let mut controls = OrbitControls::new(&mut camera);
    // The renderer's canvas stands in for the element's `clientWidth` /
    // `clientHeight`.
    controls.set_element_size(INNER_WIDTH, INNER_HEIGHT);
    // `controls.minDistance = 2; controls.maxDistance = 10;`
    controls.min_distance = 2.0;
    controls.max_distance = 10.0;
    // `controls.update();` — with no pointer events the orbit is the identity,
    // and at ( 0, 0, 8 ) looking down -Z the camera is already pointed at the
    // target. Inert for the graded frame.
    controls.update(&mut camera, None);

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
        controls,
        environment,
    }
}

/// The page's `render()`.
pub fn animate(app: &mut App) {
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
        .unwrap_or_else(|| "target/webgpu_pmrem_equirectangular.png".to_string());
    three_rs::testing::write_png(&path, width, height, &pixels);
    println!("wrote {path} ({width}x{height})");
}
