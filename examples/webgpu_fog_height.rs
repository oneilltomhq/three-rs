//! Port of `three.js/examples/webgpu_fog_height.html`, calling the three-rs
//! API in the same order the page's `init()` / `animate()` do.
//!
//! Under the e2e harness the viewport is `400 * 2` x `250 * 2` with no
//! `deviceScaleFactor`, so `window.innerWidth` / `innerHeight` /
//! `devicePixelRatio` are 800, 500 and 1. The page reads no clock and draws
//! from `Math.random()` not at all.
//!
//! A hundred tall pink boxes, one `InstancedMesh` of `MeshPhongNodeMaterial`,
//! standing in a peach height fog: `scene.fogNode = fog( color( 0xffdfc1 ),
//! exponentialHeightFogFactor( density, height ) )`, whose factor is zero above
//! the world height `height` and thickens with the depth below it times the
//! view distance. `density` and `height` are `uniform()`s the page's GUI
//! writes; they sit at their initial `0.04` and `2` for the graded frame.
//!
//! `OrbitControls` has damping on and is first updated in `animate()`, which
//! is what turns the camera from its constructed `-z` gaze to the origin. With
//! no pointer input the damped update has nothing to decay, so every frame is
//! the same. `renderer.inspector = new Inspector()` only registers the GUI.

use std::f64::consts::PI;
use std::rc::Rc;

use three_rs::addons::controls::OrbitControls;
use three_rs::nodes::tsl::{exponential_height_fog_factor, fog, uniform_value};
use three_rs::nodes::Type;
use three_rs::objects::Background;
use three_rs::{
    box_geometry, AmbientLight, Color, DirectionalLight, InstancedMesh, MeshPhongNodeMaterial,
    Object3D, PerspectiveCamera, Renderer, RendererParameters, Scene,
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
}

pub fn init() -> App {
    let mut camera = PerspectiveCamera::new(45.0, INNER_WIDTH / INNER_HEIGHT, 1.0, 600.0);
    camera.node.borrow_mut().position.set(20.0, 10.0, 25.0);

    let mut scene = Scene::new();

    // height fog

    // `const density = uniform( 0.04 ); const height = uniform( 2 );` — object
    // group uniforms, as in three's dump (`object.nodeUniform15` / `16`).
    // Divergence (`docs/nodes.md` §36.3): plain values, not the settable
    // form, because no host writes the page's two GUI sliders.
    let density = uniform_value(Type::F32, vec![0.04]);
    let height = uniform_value(Type::F32, vec![2.0]);

    let fog_factor = exponential_height_fog_factor(density, height);

    // `scene.fogNode = fog( color( 0xffdfc1 ), fogFactor );
    // scene.backgroundNode = color( 0xffdfc1 );`
    scene.fog_node = Some(fog(Color::from_hex(0xffdfc1), fog_factor));
    scene.background = Some(Background::Node(Color::from_hex(0xffdfc1).into()));

    // meshes

    let geometry = Rc::new(box_geometry(1.0, 25.0, 1.0, 1, 1, 1));
    let material = MeshPhongNodeMaterial::phong(Color::from_hex(0xcd959a));

    let mesh = InstancedMesh::new(geometry, material, 100);
    mesh.borrow_mut().position.y = -10.0;
    scene.add(&mesh);

    let mut dummy = Object3D::default();

    let mut index = 0;

    {
        let mut mesh = mesh.borrow_mut();
        for i in 0..10 {
            for j in 0..10 {
                dummy.position.x = -18.0 + (i as f64 * 4.0);
                dummy.position.z = -18.0 + (j as f64 * 4.0);

                dummy.update_matrix();

                mesh.set_matrix_at(index, &dummy.matrix);
                index += 1;
            }
        }
    }

    // lights

    let directional_light = DirectionalLight::new(Color::from_hex(0xffc0cb), 2.0);
    directional_light
        .borrow_mut()
        .position
        .set(-10.0, 10.0, 10.0);
    scene.add(&directional_light);

    let ambient_light = AmbientLight::new(Color::from_hex(0xcccccc), 1.0);
    scene.add(&ambient_light);

    // renderer

    let mut renderer = Renderer::new(RendererParameters { antialias: true }).unwrap();
    renderer.set_pixel_ratio(DPR);
    renderer.set_size(INNER_WIDTH, INNER_HEIGHT);

    // controls

    let mut controls = OrbitControls::new(&mut camera);
    // The canvas the example renders at, standing in for the element's
    // `clientWidth` / `clientHeight`.
    controls.set_element_size(INNER_WIDTH, INNER_HEIGHT);
    controls.min_distance = 7.0;
    controls.max_distance = 100.0;
    controls.max_polar_angle = PI / 2.0;
    controls.enable_damping = true;

    App {
        renderer,
        scene,
        camera,
        controls,
    }
}

/// The page's `animate()`: `controls.update(); renderer.render( scene, camera )`.
pub fn animate(app: &mut App) {
    app.controls.update(&mut app.camera, None);
    app.renderer.render(&mut app.scene, &mut app.camera);
}

/// The page's `resize()`.
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

/// The example's controls, for a host that has a pointer.
pub fn controls(app: &mut App) -> Option<&mut OrbitControls> {
    Some(&mut app.controls)
}

/// The controls and the camera at once, which every one of the controls'
/// event handlers needs (see `webgpu_custom_fog_background`).
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
        .unwrap_or_else(|| "target/webgpu_fog_height.png".to_string());
    three_rs::testing::write_png(&path, width, height, &pixels);
    println!("wrote {path} ({width}x{height})");
}
