//! Port of `three.js/examples/webgpu_sky.html`, calling the three-rs API in
//! the same order the page's `init()` does.
//!
//! Under the e2e harness the viewport is `400 * 2` x `250 * 2` with no
//! `deviceScaleFactor`, so `window.innerWidth` / `innerHeight` /
//! `devicePixelRatio` are 800, 500 and 1. The page draws from `Math.random()`
//! not at all.
//!
//! # What this page is
//!
//! A [`SkyMesh`] — the Preetham daylight model with a sun disc and an fbm
//! cloud layer — scaled to 450 000, and a chrome-like sphere of radius 400 at
//! the origin that reflects it. The reflection is a [`CubeCamera`] at the
//! origin rendering the scene into a 256² half-float [`CubeRenderTarget`]
//! every frame, with the sphere hidden, which the sphere's
//! `MeshBasicNodeMaterial( { envMap } )` samples along the reflected view
//! vector. Rendering into a target skips tone mapping, so the cube holds the
//! sky's linear radiance and the sphere and the sky go through ACES Filmic
//! at exposure 0.05 together when the frame is drawn to the canvas.
//!
//! The graded frame is the GUI's defaults: turbidity 10, rayleigh 3,
//! mieCoefficient 0.005, mieDirectionalG 0.7, the sun at elevation 65° and
//! azimuth 0, coverage 0.4, density 0.4, cloud elevation 0.5, the sun disc
//! on. `guiChanged()` runs once in `init()`, which is when the exposure goes
//! from the constructor's 0.5 to the controller's 0.05. Time is pinned to 0,
//! so the clouds' drift and evolution are both zero.
//!
//! `OrbitControls` has zoom and pan off and is never updated in `animate()`;
//! its constructor's `update()` is what turns the camera at `( 0, 100, 2000
//! )` to look at the origin.
//!
//! `renderer.inspector = new Inspector()` and its parameters panel draw
//! nothing into the canvas and are not ported.

use three_rs::addons::controls::OrbitControls;
use three_rs::addons::objects::SkyMesh;
use three_rs::geometries::sphere_geometry;
use three_rs::math::math_utils::deg_to_rad;
use three_rs::objects::Mesh;
use three_rs::{
    CubeCamera, CubeRenderTarget, MeshBasicNodeMaterial, Node, PerspectiveCamera, Renderer,
    RendererParameters, Scene, TextureType, ToneMapping, Vector3,
};

use std::rc::Rc;

pub const INNER_WIDTH: f64 = 800.0;
pub const INNER_HEIGHT: f64 = 500.0;
/// `renderer.setPixelRatio( window.devicePixelRatio )`.
pub const DPR: f64 = 1.0;

/// The page's `effectController` — the GUI's values, which `guiChanged()`
/// copies onto the sky and the renderer.
#[derive(Clone, Debug)]
pub struct EffectController {
    pub turbidity: f64,
    pub rayleigh: f64,
    pub mie_coefficient: f64,
    pub mie_directional_g: f64,
    /// Degrees above the horizon.
    pub elevation: f64,
    /// Degrees.
    pub azimuth: f64,
    pub exposure: f64,
    pub cloud_coverage: f64,
    pub cloud_density: f64,
    pub cloud_elevation: f64,
    pub show_sun_disc: bool,
}

impl Default for EffectController {
    fn default() -> Self {
        Self {
            turbidity: 10.0,
            rayleigh: 3.0,
            mie_coefficient: 0.005,
            mie_directional_g: 0.7,
            elevation: 65.0,
            azimuth: 0.0,
            exposure: 0.05,
            cloud_coverage: 0.4,
            cloud_density: 0.4,
            cloud_elevation: 0.5,
            show_sun_disc: true,
        }
    }
}

pub struct App {
    pub renderer: Renderer,
    pub scene: Scene,
    pub camera: PerspectiveCamera,
    /// The page's `controls`.
    pub controls: OrbitControls,
    pub sky: SkyMesh,
    pub sphere: Node,
    pub cube_camera: CubeCamera,
    pub effect_controller: EffectController,
}

/// `guiChanged()`.
pub fn gui_changed(app: &mut App) {
    let effect_controller = &app.effect_controller;
    let sky = &app.sky;

    sky.turbidity.set(vec![effect_controller.turbidity]);
    sky.rayleigh.set(vec![effect_controller.rayleigh]);
    sky.mie_coefficient
        .set(vec![effect_controller.mie_coefficient]);
    sky.mie_directional_g
        .set(vec![effect_controller.mie_directional_g]);
    sky.cloud_coverage
        .set(vec![effect_controller.cloud_coverage]);
    sky.cloud_density.set(vec![effect_controller.cloud_density]);
    sky.cloud_elevation
        .set(vec![effect_controller.cloud_elevation]);
    sky.show_sun_disc
        .set(vec![f64::from(u8::from(effect_controller.show_sun_disc))]);

    let phi = deg_to_rad(90.0 - effect_controller.elevation);
    let theta = deg_to_rad(effect_controller.azimuth);

    let mut sun = Vector3::default();
    sun.set_from_spherical_coords(1.0, phi, theta);

    sky.sun_position.set(vec![sun.x, sun.y, sun.z]);

    app.renderer.tone_mapping_exposure = effect_controller.exposure;
}

/// `initSky()`, less the GUI: the sky, and one `guiChanged()`.
fn init_sky(app: &mut App) {
    // Add Sky
    app.sky.mesh.borrow_mut().scale.set_scalar(450000.0);
    app.scene.add(&app.sky.mesh);

    gui_changed(app);
}

pub fn init() -> App {
    // `new THREE.PerspectiveCamera( 60, window.innerWidth /
    // window.innerHeight, 100, 2000000 )`, then `camera.position.set( 0,
    // 100, 2000 )`.
    let mut camera = PerspectiveCamera::new(60.0, INNER_WIDTH / INNER_HEIGHT, 100.0, 2000000.0);
    camera.node.borrow_mut().position.set(0.0, 100.0, 2000.0);

    let scene = Scene::new();

    let cube_render_target = CubeRenderTarget::new(256, TextureType::HalfFloat).unwrap();
    let cube_camera = CubeCamera::new(1.0, 1000.0, cube_render_target);

    let mut material = MeshBasicNodeMaterial::new();
    material.env_map = Some(cube_camera.render_target.texture.clone());
    let sphere = Mesh::new(Rc::new(sphere_geometry(400.0, 64, 32)), material);
    scene.add(&sphere);

    let mut renderer = Renderer::new(RendererParameters::default()).unwrap();
    renderer.set_pixel_ratio(DPR);
    renderer.set_size(INNER_WIDTH, INNER_HEIGHT);
    renderer.tone_mapping = ToneMapping::AcesFilmic;
    renderer.tone_mapping_exposure = 0.5;

    let mut controls = OrbitControls::new(&mut camera);
    // The canvas the example renders at, standing in for the element's
    // `clientWidth` / `clientHeight`.
    controls.set_element_size(INNER_WIDTH, INNER_HEIGHT);
    //controls.maxPolarAngle = Math.PI / 2;
    controls.enable_zoom = false;
    controls.enable_pan = false;

    let mut app = App {
        renderer,
        scene,
        camera,
        controls,
        sky: SkyMesh::new(),
        sphere,
        cube_camera,
        effect_controller: EffectController::default(),
    };

    init_sky(&mut app);

    app
}

/// The page's `animate()`.
pub fn animate(app: &mut App) {
    app.sphere.borrow_mut().visible = false;
    app.cube_camera.update(&mut app.renderer, &mut app.scene);
    app.sphere.borrow_mut().visible = true;

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
        .unwrap_or_else(|| "target/webgpu_sky.png".to_string());
    three_rs::testing::write_png(&path, width, height, &pixels);
    println!("wrote {path} ({width}x{height})");
}
