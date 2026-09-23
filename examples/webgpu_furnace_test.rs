//! Port of `three.js/examples/webgpu_furnace_test.html`, calling the three-rs
//! API in the same order the page's `init()` / `createEnvironment()` /
//! `createObjects()` do.
//!
//! Under the e2e harness the viewport is `400 * 2` x `250 * 2` with no
//! `deviceScaleFactor`, so `window.innerWidth` / `innerHeight` /
//! `devicePixelRatio` are 800, 500 and 1.
//!
//! The page has no `Math.random()`, no clock, no controls, no loader and no
//! asset. It is 121 spheres and one solid-colour environment, and its whole
//! point is a number rather than a picture: under a **constant** environment
//! an energy-conserving BSDF must return that constant for every
//! roughness × metalness pair, so three's own frame is *uniformly* `0xcccccc`
//! — all 400 000 pixels, exactly. `tests/e2e/main.rs` asserts that before it
//! asserts the pixel diff; see `docs/webgpu_furnace_test-progress.md`.
//!
//! # The two things that are easy to get wrong here
//!
//! **The 40 is a plain vertical FoV.** Unlike `webgpu_pmrem_test`, this page
//! has no `updateCamera()` and does not reinterpret it as a horizontal one.
//! The camera is `PerspectiveCamera( 40, aspect, 1, 30 )` at `( 0, 0, 18 )`
//! and nothing else.
//!
//! **`scene.background` is the *same* `Color` the environment was built from.**
//! `createEnvironment()` hands `envScene.background` to `PMREMGenerator` and
//! then assigns it to the real scene, where it is a clear colour rather than a
//! skybox. That is why the background of the frame is exactly the environment
//! and why the furnace identity is visible at all: the spheres have to
//! disappear into it.

use std::rc::Rc;

use three_rs::addons::controls::OrbitControls;
use three_rs::geometries::sphere_geometry;
use three_rs::nodes::pmrem_node::PmremEnvironment;

use three_rs::{
    Background, Color, Mesh, MeshPhysicalNodeMaterial, PerspectiveCamera, Renderer,
    RendererParameters, Scene,
};

pub const INNER_WIDTH: f64 = 800.0;
pub const INNER_HEIGHT: f64 = 500.0;
/// `renderer.setPixelRatio( window.devicePixelRatio )`.
pub const DPR: f64 = 1.0;

/// The page's `const COLOR = 0xcccccc` — the furnace.
pub const COLOR: u32 = 0xcccccc;

pub struct App {
    pub renderer: Renderer,
    pub scene: Scene,
    pub camera: PerspectiveCamera,
    pub environment: PmremEnvironment,
}

pub fn init() -> App {
    let aspect = INNER_WIDTH / INNER_HEIGHT;

    // `init()`.
    let mut renderer = Renderer::new(RendererParameters { antialias: true }).unwrap();
    renderer.set_size(INNER_WIDTH, INNER_HEIGHT);
    renderer.set_pixel_ratio(DPR);
    // `renderer.toneMapping` is left at `NoToneMapping` — this page sets none,
    // which is the difference that makes the frame land on the environment
    // colour itself rather than on a tone-mapped version of it.

    let mut scene = Scene::new();

    // `new THREE.PerspectiveCamera( 40, aspect, 1, 30 )`, `position.set( 0, 0,
    // 18 )`. A vertical 40, not the horizontal one `webgpu_pmrem_test` has.
    let camera = PerspectiveCamera::new(40.0, aspect, 1.0, 30.0);
    camera.node.borrow_mut().position.set(0.0, 0.0, 18.0);

    // `initGui()` — one checkbox, "Tint for Visibility", whose `onChange`
    // would set every material's colour to 0xccccff. `clean-page.js` removes
    // the panel and the handler never fires, so the colour stays white.

    // `createEnvironment()`.
    let mut env_scene = Scene::new();
    env_scene.background = Some(Background::Color(Color::from_hex(COLOR)));

    // `radianceMap = pmremGenerator.fromScene( envScene ).texture`, then
    // `pmremGenerator.dispose()` — which frees the generator's own scratch and
    // leaves the returned target alive. `PmremEnvironment` owns that target.
    let environment = PmremEnvironment::from_scene(&mut renderer, &mut env_scene, 0.0).unwrap();

    // `scene.background = envScene.background`. `_sceneToCubeUV` put it back
    // on `envScene` after borrowing it for the background box, so this is the
    // same `Color` — and on the real scene a `Color` background is the pass'
    // clear colour, not a skybox draw.
    scene.background = env_scene.background.clone();

    // `createObjects()`: `SphereGeometry( 0.4, 32, 16 )`, shared, and an 11×11
    // grid of `MeshPhysicalMaterial` — roughness left to right, metalness top
    // to bottom. 121 meshes, 121 materials, one geometry.
    let geometry = Rc::new(sphere_geometry(0.4, 32, 16));

    for x in 0..=10 {
        for y in 0..=10 {
            let mut material = MeshPhysicalNodeMaterial::physical(
                Color::from_hex(0xffffff),
                x as f64 / 10.0,
                y as f64 / 10.0,
            );
            // `transmission: 0` and `ior: 1.5` are the defaults; `envMap:
            // radianceMap, envMapIntensity: 1` is the environment.
            material.ior = 1.5;
            material.pmrem_env = Some(environment.handle());

            let mesh = Mesh::new(geometry.clone(), material);
            {
                let mut object = mesh.borrow_mut();
                object.position.x = x as f64 - 5.0;
                object.position.y = 5.0 - y as f64;
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
    // `fromScene` built the PMREM eagerly in `init()`; this is a no-op, and is
    // here so the steady-frame assertion runs over the same call the other
    // PMREM examples make.
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

    // The page's handler ends with `render()`, which is this example's
    // `animate()`: it draws on resize rather than waiting for a frame,
    // because the page has no animation loop.
    animate(app);
}

/// The example's controls, for a host that has a pointer. `None` here:
/// the page creates none.
pub fn controls(_app: &mut App) -> Option<&mut OrbitControls> {
    None
}

/// The controls and the camera at once, for a host delivering pointer events.
/// `None` here: the page creates no controls.
pub fn controls_and_camera(_app: &mut App) -> Option<(&mut OrbitControls, &mut PerspectiveCamera)> {
    None
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
        .unwrap_or_else(|| "target/webgpu_furnace_test.png".to_string());
    three_rs::testing::write_png(&path, width, height, &pixels);
    println!("wrote {path} ({width}x{height})");
}
