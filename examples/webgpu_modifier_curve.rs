//! Port of `three.js/examples/webgpu_modifier_curve.html`, calling the
//! three-rs API in the same order the page's `init()` / `animate()` do.
//!
//! Under the e2e harness the viewport is `400 * 2` x `250 * 2` with no
//! `deviceScaleFactor`, so `window.innerWidth` / `innerHeight` /
//! `devicePixelRatio` are 800, 500 and 1.
//!
//! "Hello three.js!" as a `TextGeometry` — `ExtrudeGeometry` over the shapes
//! `Font.generateShapes()` builds from `helvetiker_regular.typeface.json` —
//! bent along a closed centripetal `CatmullRomCurve3` by `Flow`
//! (`CurveModifierGPU`), which bakes the curve's spaced points and Frenet
//! frames into a half-float `DataTexture` and rebuilds each vertex in its
//! frame in the vertex shader.
//!
//! The font arrives through `FontLoader.load()`'s callback. The grader waits
//! for the network to settle before its one frame, so the text is always in
//! the graded frame; here the load is synchronous and the callback body runs
//! inline, in the same place in `init()`.
//!
//! Not ported: the `TransformControls` and the `Raycaster`. The page only
//! creates the helper's scene entry on a `pointerdown` that hits a handle,
//! so neither draws anything on the graded frame, and the port has no
//! `TransformControls`.

use std::rc::Rc;

use three_rs::addons::controls::OrbitControls;
use three_rs::addons::curve_modifier_gpu::Flow;
use three_rs::addons::text_geometry::{text_geometry, TextGeometryOptions};
use three_rs::core::BufferGeometry;
use three_rs::extras::{CatmullRomCurve3, Curve, CurveType};
use three_rs::loaders::FontLoader;
use three_rs::{
    box_geometry, AmbientLight, Color, DirectionalLight, Line, LineBasicNodeMaterial, Mesh,
    MeshBasicNodeMaterial, PerspectiveCamera, Renderer, RendererParameters, Scene, Vector3,
};

pub const INNER_WIDTH: f64 = 800.0;
pub const INNER_HEIGHT: f64 = 500.0;
/// `renderer.setPixelRatio( window.devicePixelRatio )`
pub const DPR: f64 = 1.0;

pub struct App {
    pub renderer: Renderer,
    pub scene: Scene,
    pub camera: PerspectiveCamera,
    /// The page's `flow`: `animate()` moves it along the curve every frame.
    pub flow: Flow,
}

fn examples_dir() -> std::path::PathBuf {
    three_rs::testing::three_js_dir().join("examples")
}

pub fn init() -> App {
    let scene = Scene::new();

    let mut camera = PerspectiveCamera::new(40.0, INNER_WIDTH / INNER_HEIGHT, 1.0, 1000.0);
    camera.node.borrow_mut().position.set(2.0, 2.0, 4.0);
    camera.look_at(&Vector3::ZERO);

    let initial_points = [
        Vector3::new(1.0, 0.0, -1.0),
        Vector3::new(1.0, 0.0, 1.0),
        Vector3::new(-1.0, 0.0, 1.0),
        Vector3::new(-1.0, 0.0, -1.0),
    ];

    let box_geometry = Rc::new(box_geometry(0.1, 0.1, 0.1, 1, 1, 1));
    let box_material = MeshBasicNodeMaterial::new();

    for handle_pos in &initial_points {
        let handle = Mesh::new(box_geometry.clone(), box_material.clone());
        handle.borrow_mut().position = *handle_pos;
        scene.add(&handle);
    }

    // `new THREE.CatmullRomCurve3( curveHandles.map( ( handle ) =>
    // handle.position ) )`: upstream holds the handles' own position vectors,
    // so dragging a handle moves the curve. Nothing moves them here.
    let mut curve = CatmullRomCurve3::new(initial_points.to_vec());
    curve.curve_type = CurveType::Centripetal;
    curve.closed = true;

    let points = curve.get_points(50);
    let mut line_geometry = BufferGeometry::new();
    line_geometry.set_from_points(&points);
    let line = Line::new(
        Rc::new(line_geometry),
        LineBasicNodeMaterial::line(Color::from_hex(0x00ff00)),
    );

    scene.add(&line);

    //

    let light = DirectionalLight::new(Color::from_hex(0xffaa33), 3.0);
    light.borrow_mut().position.set(-10.0, 10.0, 10.0);
    scene.add(&light);

    let light2 = AmbientLight::new(Color::from_hex(0x003973), 3.0);
    scene.add(&light2);

    //

    let font = FontLoader::new()
        .load(examples_dir().join("fonts/helvetiker_regular.typeface.json"))
        .unwrap();

    let mut parameters = TextGeometryOptions::new();
    parameters.size = 0.2;
    parameters.extrude.depth = 0.05;
    parameters.extrude.curve_segments = 12;
    parameters.extrude.bevel_enabled = true;
    parameters.extrude.bevel_thickness = 0.02;
    parameters.extrude.bevel_size = Some(0.01);
    parameters.extrude.bevel_offset = 0.0;
    parameters.extrude.bevel_segments = 5;
    let mut geometry = text_geometry("Hello three.js!", &font, &parameters);

    geometry.rotate_x(std::f64::consts::PI);

    let material = MeshBasicNodeMaterial::standard(Color::from_hex(0x99ffff), 1.0, 0.0);

    let mut flow = Flow::new(Rc::new(geometry), &material, 1);
    flow.update_curve(0, &curve);
    scene.add(&flow.object3d);

    //

    let mut renderer = Renderer::new(RendererParameters { antialias: true }).unwrap();
    renderer.set_pixel_ratio(DPR);
    renderer.set_size(INNER_WIDTH, INNER_HEIGHT);

    App {
        renderer,
        scene,
        camera,
        flow,
    }
}

/// The page's `animate()`. With no `pointerdown`, `action` stays
/// `ACTION_NONE` and the raycast never runs.
///
/// `flow.moveAlongCurve( 0.001 )` runs before every render, so the graded
/// frame is already 0.001 of the curve (about one spline texel) along.
pub fn animate(app: &mut App) {
    app.flow.move_along_curve(0.001);

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

/// The example's controls, for a host that has a pointer. `None` here:
/// the page's only controls are the `TransformControls`, which the port does
/// not have.
pub fn controls(_app: &mut App) -> Option<&mut OrbitControls> {
    None
}

/// The controls and the camera at once, for a host delivering pointer events.
/// `None` here, as for [`controls`].
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
        .unwrap_or_else(|| "target/webgpu_modifier_curve.png".to_string());
    three_rs::testing::write_png(&path, width, height, &pixels);
    println!("wrote {path} ({width}x{height})");
}
