//! Port of `three.js/examples/webgpu_clipping.html`, calling the three-rs API
//! in the same order the page's `init()` / `animate()` do.
//!
//! Under the e2e harness the viewport is `400 * 2` x `250 * 2` with no
//! `deviceScaleFactor`, so `window.innerWidth` / `innerHeight` /
//! `devicePixelRatio` are 800, 500 and 1.
//!
//! `Date.now()` and `performance.now()` are pinned to 0, so `startTime` and
//! `animate( currentTime )`'s argument are both 0: `time` is 0, the knot keeps
//! a zero rotation and `Math.cos( 0 ) * 0.125 + 0.875 = 1` leaves its scale
//! alone. Only `object.position.y = 0.8` moves it.
//!
//! Two `ClippingGroup`s: the global one clips everything under it (the ground
//! and the knot's group) by one plane, and the knot's own intersects two more.
//! `antialias: true` with `alphaToCoverage: true` on both materials sends the
//! fragment stage through `ClippingNode`'s alpha-to-coverage path
//! (`clippingAlpha()`), and on an adapter with `clip-distances` the knot's
//! global (union) plane becomes a hardware clip distance
//! (`hardwareClipping()`).
//!
//! The GUI (`Knot Clipping Group`, `Global Clipping Group`,
//! `alphaToCoverage`) sits at its defaults, `renderer.inspector` does not
//! touch the frame, and the `resize` listener never fires.

use std::f64::consts::PI;
use std::rc::Rc;

use three_rs::addons::controls::OrbitControls;
use three_rs::core::ObjectRef;
use three_rs::geometries::{plane_geometry, torus_knot_geometry};
use three_rs::materials::Side;
use three_rs::math::{Plane, Vector3};
use three_rs::utils::{date_now_ms, now_ms};
use three_rs::{
    AmbientLight, ClippingGroup, Color, DirectionalLight, Mesh, MeshPhongNodeMaterial,
    PerspectiveCamera, Renderer, RendererParameters, Scene, SpotLight,
};

pub const INNER_WIDTH: f64 = 800.0;
pub const INNER_HEIGHT: f64 = 500.0;
/// `renderer.setPixelRatio( window.devicePixelRatio )`
pub const DPR: f64 = 1.0;

pub struct App {
    pub renderer: Renderer,
    pub scene: Scene,
    pub camera: PerspectiveCamera,
    /// The page's `controls`.
    pub controls: OrbitControls,
    /// The page's module-level `object`: the torus knot.
    pub object: ObjectRef,
    /// The page's module-level `startTime`, `Date.now()` at the end of
    /// `init()`.
    pub start_time: f64,
}

pub fn init() -> App {
    let mut camera = PerspectiveCamera::new(36.0, INNER_WIDTH / INNER_HEIGHT, 0.25, 16.0);
    camera.node.borrow_mut().position.set(0.0, 1.3, 3.0);

    let scene = Scene::new();

    // Lights

    scene.add(&AmbientLight::new(Color::from_hex(0xcccccc), 1.0));

    let spot_light = SpotLight::new(Color::from_hex(0xffffff), 60.0);
    {
        let mut object = spot_light.borrow_mut();
        let light = object.light_mut().unwrap();
        light.angle = PI / 5.0;
        light.penumbra = 0.2;
        object.position.set(2.0, 3.0, 3.0);
        object.cast_shadow = true;
        let shadow = object.light_mut().unwrap().shadow.as_mut().unwrap();
        shadow.camera.set_near(3.0);
        shadow.camera.set_far(10.0);
        shadow.map_size.x = 2048.0;
        shadow.map_size.y = 2048.0;
        shadow.radius = 4.0;
    }
    scene.add(&spot_light);

    let dir_light = DirectionalLight::new(Color::from_hex(0x55505a), 3.0);
    {
        let mut object = dir_light.borrow_mut();
        object.position.set(0.0, 3.0, 0.0);
        object.cast_shadow = true;
        let shadow = object.light_mut().unwrap().shadow.as_mut().unwrap();
        shadow.camera.set_near(1.0);
        shadow.camera.set_far(10.0);
        shadow.camera.set_bounds(-1.0, 1.0, 1.0, -1.0);
        shadow.map_size.x = 1024.0;
        shadow.map_size.y = 1024.0;
    }
    scene.add(&dir_light);

    // Clipping planes

    let global_plane = Plane::new(Vector3::new(-1.0, 0.0, 0.0), 0.1);
    let local_plane1 = Plane::new(Vector3::new(0.0, -1.0, 0.0), 0.8);
    let local_plane2 = Plane::new(Vector3::new(0.0, 0.0, -1.0), 0.1);

    // Clipping Groups

    let global_clipping_group = ClippingGroup::of(ClippingGroup {
        clipping_planes: vec![global_plane],
        ..ClippingGroup::default()
    });

    let knot_clipping_group = ClippingGroup::of(ClippingGroup {
        clipping_planes: vec![local_plane1, local_plane2],
        clip_intersection: true,
        ..ClippingGroup::default()
    });

    scene.add(&global_clipping_group);
    global_clipping_group.add(&knot_clipping_group);

    // Geometry

    let mut material = MeshPhongNodeMaterial::phong(Color::from_hex(0x80ee10));
    material.shininess = 0.0;
    material.side = Side::Double;
    material.alpha_to_coverage = true;

    let geometry = Rc::new(torus_knot_geometry(0.4, 0.08, 95, 20, 2.0, 3.0));

    let object = Mesh::new(geometry, material);
    object.borrow_mut().cast_shadow = true;
    knot_clipping_group.add(&object);

    let mut ground_material = MeshPhongNodeMaterial::phong(Color::from_hex(0xa0adaf));
    ground_material.shininess = 150.0;
    ground_material.alpha_to_coverage = true;
    let ground = Mesh::new(Rc::new(plane_geometry(9.0, 9.0, 1, 1)), ground_material);
    {
        let mut object = ground.borrow_mut();
        object.set_rotation(-PI / 2.0, 0.0, 0.0); // rotates X/Y to X/Z
        object.receive_shadow = true;
    }
    global_clipping_group.add(&ground);

    // Renderer

    let mut parameters = RendererParameters::default();
    parameters.antialias = true;
    let mut renderer = Renderer::new(parameters).unwrap();
    renderer.shadow_map_enabled = true;
    renderer.set_pixel_ratio(DPR);
    renderer.set_size(INNER_WIDTH, INNER_HEIGHT);

    // Controls

    let mut controls = OrbitControls::new(&mut camera);
    // The canvas the example renders at, standing in for the element's
    // `clientWidth` / `clientHeight`.
    controls.set_element_size(INNER_WIDTH, INNER_HEIGHT);
    controls.target.set(0.0, 1.0, 0.0);
    let _ = controls.update(&mut camera, None);

    // Start

    App {
        start_time: date_now_ms(),
        renderer,
        scene,
        camera,
        controls,
        object,
    }
}

/// The page's `animate( currentTime )`, whose argument is
/// `requestAnimationFrame`'s: `performance.now()` at the start of the frame.
pub fn animate(app: &mut App) {
    let time = (now_ms() - app.start_time) / 1000.0;

    {
        let mut object = app.object.borrow_mut();
        object.position.y = 0.8;
        let r = object.rotation;
        object.set_rotation(time * 0.5, time * 0.2, r.z);
        object.scale.set_scalar((time.cos()) * 0.125 + 0.875);
    }

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
        .unwrap_or_else(|| "target/webgpu_clipping.png".to_string());
    three_rs::testing::write_png(&path, width, height, &pixels);
    println!("wrote {path} ({width}x{height})");
}
