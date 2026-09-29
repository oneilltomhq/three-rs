//! Port of `three.js/examples/webgpu_camera_array.html`, calling the three-rs
//! API in the same order the page's `init()` / `animate()` do.
//!
//! Under the e2e harness the viewport is `400 * 2` x `250 * 2` with no
//! `deviceScaleFactor`, so `window.innerWidth` / `innerHeight` /
//! `devicePixelRatio` are 800, 500 and 1.
//!
//! One `render()` of an `ArrayCamera`: thirty-six sub-cameras, each drawn
//! into its own `134 x 84` viewport of a 6 x 6 grid. The shaders index
//! `cameraViewMatrices` / `cameraProjectionMatrices` by a flat
//! `v_cameraIndex` varying, and the backend swaps a `cameraIndex` bind group
//! and the viewport per sub-camera (`docs/nodes.md` §40).
//!
//! `subcamera.copy( camera )` copies the *array camera's* projection
//! parameters, which are `new PerspectiveCamera()`'s defaults — so every
//! sub-camera renders at `fov` 50 and `far` 2000, not the `40` / `10` it was
//! constructed with.
//!
//! The harness fires a single animation frame, so the cylinder is drawn at
//! `animate()`'s first increment. The `resize` listener never fires.

use std::rc::Rc;

use three_rs::addons::controls::OrbitControls;
use three_rs::core::Node;
use three_rs::geometries::{cylinder_geometry, plane_geometry};
use three_rs::lights::ShadowCamera;
use three_rs::math::{Vector3, Vector4};
use three_rs::{
    AmbientLight, ArrayCamera, Color, DirectionalLight, Mesh, MeshPhongNodeMaterial,
    PerspectiveCamera, Renderer, RendererParameters, Scene,
};

pub const INNER_WIDTH: f64 = 800.0;
pub const INNER_HEIGHT: f64 = 500.0;
/// `renderer.setPixelRatio( window.devicePixelRatio )`
pub const DPR: f64 = 1.0;

/// `const AMOUNT = 6`.
const AMOUNT: usize = 6;

pub struct App {
    pub renderer: Renderer,
    pub scene: Scene,
    pub camera: ArrayCamera,
    pub mesh: Node,
    /// `window.innerWidth` / `innerHeight`, which `updateCameras()` reads.
    pub width: f64,
    pub height: f64,
}

pub fn init() -> App {
    let mut sub_cameras = Vec::with_capacity(AMOUNT * AMOUNT);

    for _ in 0..AMOUNT * AMOUNT {
        let mut sub_camera = PerspectiveCamera::new(40.0, 1.0, 0.1, 10.0);
        sub_camera.viewport = Some(Vector4::new(0.0, 0.0, 0.0, 0.0));

        sub_cameras.push(sub_camera);
    }

    let mut camera = ArrayCamera::new(sub_cameras);
    camera.node.borrow_mut().position.z = 3.0;

    update_cameras(&mut camera, INNER_WIDTH, INNER_HEIGHT);

    let scene = Scene::new();

    scene.add(&AmbientLight::new(Color::from_hex(0x999999), 1.0));

    let light = DirectionalLight::new(Color::from_hex(0xffffff), 3.0);
    {
        let mut object = light.borrow_mut();
        object.position.set(0.5, 0.5, 1.0);
        object.cast_shadow = true;
        let shadow = object.light_mut().unwrap().shadow.as_mut().unwrap();
        // "tighter shadow map"
        if let ShadowCamera::Orthographic(shadow_camera) = &mut shadow.camera {
            shadow_camera.zoom = 4.0;
        }
    }
    scene.add(&light);

    let geometry_background = Rc::new(plane_geometry(100.0, 100.0, 1, 1));
    let material_background = MeshPhongNodeMaterial::phong(Color::from_hex(0x000066));

    let background = Mesh::new(geometry_background, material_background);
    {
        let mut object = background.borrow_mut();
        object.receive_shadow = true;
        object.position.set(0.0, 0.0, -1.0);
    }
    scene.add(&background);

    let geometry_cylinder = Rc::new(cylinder_geometry(0.5, 0.5, 1.0, 32));
    let material_cylinder = MeshPhongNodeMaterial::phong(Color::from_hex(0xff0000));

    let mesh = Mesh::new(geometry_cylinder, material_cylinder);
    {
        let mut object = mesh.borrow_mut();
        object.cast_shadow = true;
        object.receive_shadow = true;
    }
    scene.add(&mesh);

    let mut renderer = Renderer::new(RendererParameters::default()).unwrap();
    renderer.set_pixel_ratio(DPR);
    renderer.set_size(INNER_WIDTH, INNER_HEIGHT);
    renderer.shadow_map_enabled = true;

    App {
        renderer,
        scene,
        camera,
        mesh,
        width: INNER_WIDTH,
        height: INNER_HEIGHT,
    }
}

/// The page's `updateCameras()`.
fn update_cameras(camera: &mut ArrayCamera, width: f64, height: f64) {
    let aspect_ratio = width / height;
    let cell_width = width / AMOUNT as f64;
    let cell_height = height / AMOUNT as f64;

    camera.aspect = aspect_ratio;
    camera.update_projection_matrix();

    let base = camera.camera.clone();

    for y in 0..AMOUNT {
        for x in 0..AMOUNT {
            let sub_camera = &mut camera.cameras[AMOUNT * y + x];
            // `subcamera.copy( camera )` — "copy fov, aspect ratio, near, far
            // from the root camera". The transform it also copies is
            // overwritten below.
            sub_camera.fov = base.fov;
            sub_camera.aspect = base.aspect;
            sub_camera.near = base.near;
            sub_camera.far = base.far;
            sub_camera.zoom = base.zoom;
            sub_camera.focus = base.focus;
            sub_camera.view = base.view;
            sub_camera.film_gauge = base.film_gauge;
            sub_camera.film_offset = base.film_offset;

            sub_camera.viewport = Some(Vector4::new(
                (x as f64 * cell_width).floor(),
                (y as f64 * cell_height).floor(),
                cell_width.ceil(),
                cell_height.ceil(),
            ));
            sub_camera.update_projection_matrix();

            {
                let mut object = sub_camera.node.borrow_mut();
                object.position.x = (x as f64 / AMOUNT as f64) - 0.5;
                object.position.y = 0.5 - (y as f64 / AMOUNT as f64);
                object.position.z = 1.5 + ((x + y) as f64 * 0.5);
                object.position.multiply_scalar(2.0);
            }

            sub_camera.look_at(&Vector3::new(0.0, 0.0, 0.0));
            sub_camera.update_matrix_world();
        }
    }
}

/// The page's `animate()`.
pub fn animate(app: &mut App) {
    {
        let mut object = app.mesh.borrow_mut();
        let r = object.rotation;
        object.set_rotation(r.x + 0.005, r.y, r.z + 0.01);
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
    app.width = width;
    app.height = height;
    update_cameras(&mut app.camera, width, height);
    app.renderer.set_size(width, height);
}

/// The page creates no controls.
pub fn controls(_app: &mut App) -> Option<&mut OrbitControls> {
    None
}

/// The controls and the camera at once, for a host delivering pointer events.
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
        .unwrap_or_else(|| "target/webgpu_camera_array.png".to_string());
    three_rs::testing::write_png(&path, width, height, &pixels);
    println!("wrote {path} ({width}x{height})");
}
