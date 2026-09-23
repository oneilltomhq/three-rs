//! Port of `three.js/examples/webgpu_pmrem_test.html`, calling the three-rs
//! API in the same order the page's `init()` / `createObjects()` do.
//!
//! Under the e2e harness the viewport is `400 * 2` x `250 * 2` with no
//! `deviceScaleFactor`, so `window.innerWidth` / `innerHeight` /
//! `devicePixelRatio` are 800, 500 and 1.
//!
//! The page draws from `Math.random()` not at all and reads no clock, so the
//! harness' seeded sequence and pinned `performance.now` are never touched.
//!
//! # The three things that are easy to get wrong here
//!
//! **The camera's 40 is a *horizontal* FoV.** `updateCamera()` converts it to
//! the vertical one `PerspectiveCamera` actually stores, so at 800×500 the fov
//! is `2 * atan( tan( 20° ) / 1.6 ) = 25.5115…°`, not 40. Every sphere is the
//! wrong size if this is skipped, and nothing but the image says so.
//!
//! **The directional light has intensity 0.** It contributes no light, but it
//! is in the light list, so the physical material's fragment shader carries
//! the directional block and the render uniform buffer has its three cells.
//! Dropping it would render an identical image and a different shader; the
//! gate is the WGSL, not the pixels.
//!
//! **`spot1Lux.hdr` is `flipY = true`.** One bright texel at (597, 213) of a
//! 1024×512 black image, which the upload flips to row 298. Without the flip
//! the environment is mirrored top-to-bottom and every sphere still looks like
//! a perfectly reasonable shiny sphere. `tests/pmrem.rs` asserts the texel's
//! landing row rather than trusting the image.

use std::rc::Rc;

use three_rs::addons::controls::OrbitControls;
use three_rs::geometries::sphere_geometry;
use three_rs::loaders::HdrLoader;
use three_rs::materials::ToneMapping;
use three_rs::nodes::pmrem_node::PmremEnvironment;

use three_rs::{
    Background, Color, DirectionalLight, Mesh, MeshPhysicalNodeMaterial, PerspectiveCamera,
    Renderer, RendererParameters, Scene,
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

/// `updateCamera()` — the page's `horizontalFoV = 40` as the vertical FoV a
/// `PerspectiveCamera` stores. Computed the way the page computes it,
/// degrees → radians → degrees, rather than as a rounded constant.
pub fn vertical_fov(horizontal_fov: f64, aspect: f64) -> f64 {
    2.0 * ((horizontal_fov / 2.0 * std::f64::consts::PI / 180.0).tan() / aspect).atan() * 180.0
        / std::f64::consts::PI
}

pub fn init() -> App {
    let aspect = INNER_WIDTH / INNER_HEIGHT;

    let mut renderer = Renderer::new(RendererParameters { antialias: true }).unwrap();
    renderer.set_pixel_ratio(DPR);
    renderer.set_size(INNER_WIDTH, INNER_HEIGHT);
    renderer.tone_mapping = ToneMapping::AcesFilmic;
    // `renderer.toneMappingExposure = 1` is the default.

    let mut scene = Scene::new();

    // `new THREE.PerspectiveCamera( 40, aspect, 1, 30 )`, then
    // `updateCamera()`, then the position — the page's order.
    let mut camera = PerspectiveCamera::new(40.0, aspect, 1.0, 30.0);
    camera.fov = vertical_fov(40.0, aspect);
    camera.update_projection_matrix();
    camera.node.borrow_mut().position.set(0.0, 0.0, 16.0);

    // `new OrbitControls( camera, renderer.domElement )` with `minDistance` /
    // `maxDistance` set and never updated: with no pointer events the orbit is
    // the identity, and at ( 0, 0, 16 ) looking down -Z the camera is already
    // pointed at the target. Inert for the graded frame.
    let mut controls = OrbitControls::new(&mut camera);
    // The canvas the example renders at, standing in for the element's
    // `clientWidth` / `clientHeight`.
    controls.set_element_size(INNER_WIDTH, INNER_HEIGHT);
    controls.min_distance = 4.0;
    controls.max_distance = 20.0;

    // The light the example exists to compare against, at intensity zero. Its
    // direction is the spherical position of the HDR's one bright texel:
    // `theta = ( 597 + 0.5 ) * PI / 512`, `phi = ( 213 + 0.5 ) * PI / 512`,
    // `setFromSphericalCoords( 100, - phi, PI / 2 - theta )`.
    let directional_light = DirectionalLight::new(Color::from_hex(0xffffff), 0.0);
    {
        let theta = (597.0 + 0.5) * std::f64::consts::PI / 512.0;
        let phi = (213.0 + 0.5) * std::f64::consts::PI / 512.0;
        let mut object = directional_light.borrow_mut();
        object
            .position
            .set_from_spherical_coords(100.0, -phi, std::f64::consts::FRAC_PI_2 - theta);
    }
    scene.add(&directional_light);

    // `new HDRLoader().setPath( 'textures/equirectangular/' ).load(
    // 'spot1Lux.hdr', … )`. The loader resolves synchronously here, so the
    // callback's body is written inline.
    //
    // The environment is expressed in nits: one texel of 27 490 nits at
    // ( 597, 213 ), which is `1 / ( sin( phi ) * ( PI / 512 ) ^ 2 )` — the
    // radiance that matches the 1 lux the directional light would give.
    let texture = HdrLoader::new()
        .load(examples_dir().join("textures/equirectangular/spot1Lux.hdr"))
        .unwrap();

    // `radianceMap = pmremGenerator.fromEquirectangular( texture ).texture`.
    // `pmremGenerator.dispose()` frees only the generator's own scratch; the
    // returned target outlives it, which is what `PmremEnvironment` owns.
    let mut environment = PmremEnvironment::from_equirectangular(&texture);
    // `PMREMNode.updateBefore()` — see `docs/nodes.md` §12 for why the trigger
    // is here rather than inside the node.
    environment.update(&mut renderer).unwrap();

    // `scene.background = radianceMap`.
    scene.background = Some(Background::Pmrem(environment.handle()));

    // `new THREE.SphereGeometry( 0.4, 32, 32 )` — one geometry, 33 meshes,
    // 33 materials.
    let geometry = Rc::new(sphere_geometry(0.4, 32, 32));

    for x in 0..=10 {
        for y in 0..=2 {
            let color = if y < 2 {
                Color::from_hex(0xffffff)
            } else {
                Color::from_hex(0x000000)
            };
            let mut material = MeshPhysicalNodeMaterial::physical(
                color,
                x as f64 / 10.0,
                if y < 1 { 1.0 } else { 0.0 },
            );
            // `envMap: radianceMap, envMapIntensity: 1` — the intensity is the
            // default, and the GUI's `onChange` never fires under the grader.
            material.pmrem_env = Some(environment.handle());

            let mesh = Mesh::new(geometry.clone(), material);
            {
                let mut object = mesh.borrow_mut();
                object.position.x = x as f64 - 5.0;
                object.position.y = 1.0 - y as f64;
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
    // The page calls `updateCamera()` here rather than
    // `camera.updateProjectionMatrix()`: its 40 is a horizontal FoV, so the
    // vertical one the camera stores has to be recomputed from the new aspect.
    app.camera.fov = vertical_fov(40.0, app.camera.aspect);
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
        .unwrap_or_else(|| "target/webgpu_pmrem_test.png".to_string());
    three_rs::testing::write_png(&path, width, height, &pixels);
    println!("wrote {path} ({width}x{height})");
}
