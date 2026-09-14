//! Port of `three.js/examples/webgpu_morphtargets.html`, calling the three-rs
//! API in the same order the page's `init()` / `animate()` do.
//!
//! Under the e2e harness the viewport is `400 * 2` x `250 * 2` with no
//! `deviceScaleFactor`, so `window.innerWidth` / `innerHeight` /
//! `devicePixelRatio` are 800, 500 and 1.
//!
//! `initGUI()` only registers Inspector sliders, and their `onChange` never
//! fires under the single deterministic RAF, so both morph influences are 0 in
//! the graded frame: the box is still a box. The morph path must still compile
//! and bind, which is what `tests/nodes_morph.rs` and the `MORPH_INFLUENCES`
//! environment override below exercise off the graded path.
//!
//! `OrbitControls` is constructed with `enableZoom = false` and never
//! interacted with, and `renderer.inspector = new Inspector()` does not reach
//! the image; neither is ported. The `resize` listener does not fire.
//!
//! The camera is *in* the scene (`scene.add( camera )`) and the point light is a
//! child of the camera, so the light's world matrix is the camera's: world
//! position (0, 0, 10), view position (0, 0, 0).

use std::rc::Rc;

use three_rs::core::BufferAttribute;
use three_rs::{
    box_geometry, AmbientLight, BufferGeometry, Color, Mesh, MeshPhongNodeMaterial,
    PerspectiveCamera, PointLight, Renderer, RendererParameters, Scene, Vector3,
};

pub const INNER_WIDTH: f64 = 800.0;
pub const INNER_HEIGHT: f64 = 500.0;
/// `renderer.setPixelRatio( window.devicePixelRatio )`.
pub const DPR: f64 = 1.0;

pub struct App {
    pub renderer: Renderer,
    pub scene: Scene,
    pub camera: PerspectiveCamera,
    /// The page's module-level `mesh`, which `initGUI()`'s sliders would morph.
    pub mesh: three_rs::Node,
}

/// `createGeometry()`.
pub fn create_geometry() -> BufferGeometry {
    let mut geometry = box_geometry(2.0, 2.0, 2.0, 32, 32, 32);

    // the original positions of the cube's vertices
    let position_attribute = geometry.get_attribute("position").unwrap().clone();

    // for the first morph target we'll move the cube's vertices onto the surface
    // of a sphere
    let mut sphere_positions: Vec<f32> = Vec::new();

    // for the second morph target, we'll twist the cubes vertices
    let mut twist_positions: Vec<f32> = Vec::new();
    let direction = Vector3::new(1.0, 0.0, 0.0);
    let mut vertex = Vector3::ZERO;

    for i in 0..position_attribute.count() {
        let x = position_attribute.get_x(i);
        let y = position_attribute.get_y(i);
        let z = position_attribute.get_z(i);

        sphere_positions.push(
            (x * (1.0 - (y * y / 2.0) - (z * z / 2.0) + (y * y * z * z / 3.0)).sqrt()) as f32,
        );
        sphere_positions.push(
            (y * (1.0 - (z * z / 2.0) - (x * x / 2.0) + (z * z * x * x / 3.0)).sqrt()) as f32,
        );
        sphere_positions.push(
            (z * (1.0 - (x * x / 2.0) - (y * y / 2.0) + (x * x * y * y / 3.0)).sqrt()) as f32,
        );

        // stretch along the x-axis so we can see the twist better
        vertex.set(x * 2.0, y, z);
        vertex.apply_axis_angle(&direction, std::f64::consts::PI * x / 2.0);

        twist_positions.push(vertex.x as f32);
        twist_positions.push(vertex.y as f32);
        twist_positions.push(vertex.z as f32);
    }

    // `geometry.morphAttributes.position = [ sphere, twist ]`
    geometry.set_morph_attribute(
        "position",
        vec![
            BufferAttribute::new(sphere_positions, 3),
            BufferAttribute::new(twist_positions, 3),
        ],
    );

    geometry
}

pub fn init() -> App {
    let mut scene = Scene::new();
    scene.set_background(Color::from_hex(0x8FBCD4));

    let camera = PerspectiveCamera::new(45.0, INNER_WIDTH / INNER_HEIGHT, 1.0, 20.0);
    camera.node.borrow_mut().position.z = 10.0;
    scene.add(&camera.node);

    scene.add(&AmbientLight::new(Color::from_hex(0x8FBCD4), 1.5));

    let point_light = PointLight::new(Color::from_hex(0xffffff), 200.0, 0.0);
    camera.node.add(&point_light);

    let geometry = Rc::new(create_geometry());

    let mut material = MeshPhongNodeMaterial::phong(Color::from_hex(0xff0000));
    material.flat_shading = true;

    let mesh = Mesh::new(geometry, material);
    scene.add(&mesh);

    let mut renderer = Renderer::new(RendererParameters { antialias: true });
    renderer.set_pixel_ratio(DPR);
    renderer.set_size(INNER_WIDTH, INNER_HEIGHT);

    // Off the graded path: `MORPH_INFLUENCES=1,0` drives the sliders that the
    // Inspector would, so the morph loop can be seen to work (plan §4 step 5).
    if let Ok(values) = std::env::var("MORPH_INFLUENCES") {
        let influences: Vec<f64> = values
            .split(',')
            .map(|value| value.trim().parse().unwrap())
            .collect();
        let mut mesh = mesh.borrow_mut();
        let mesh = mesh.mesh_mut().unwrap();
        for (slot, value) in influences.iter().enumerate() {
            mesh.morph_target_influences[slot] = *value;
        }
    }

    App {
        renderer,
        scene,
        camera,
        mesh,
    }
}

/// The page's animation loop: `renderer.render( scene, camera )` and nothing
/// else.
pub fn animate(app: &mut App) {
    app.renderer.render(&mut app.scene, &mut app.camera);
}

fn main() {
    let mut app = init();
    println!("adapter: {:?}", app.renderer.adapter_info());
    animate(&mut app);

    let (width, height, pixels) = app.renderer.read_canvas_pixels();
    let path = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "target/webgpu_morphtargets.png".to_string());
    three_rs::testing::write_png(&path, width, height, &pixels);
    println!("wrote {path} ({width}x{height})");
}
