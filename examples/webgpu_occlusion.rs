//! Port of `three.js/examples/webgpu_occlusion.html`, calling the three-rs API
//! in the same order the page's `init()` / `render()` do.
//!
//! Under the e2e harness the viewport is `400 * 2` x `250 * 2` with no
//! `deviceScaleFactor`, so `window.innerWidth` / `innerHeight` /
//! `devicePixelRatio` are 800, 500 and 1.
//!
//! A green Phong plane in front of a yellow Phong sphere. The sphere has
//! `occlusionTest = true`, so its draw is wrapped in an occlusion query, and
//! the plane's `colorNode` is the page's `OcclusionNode extends THREE.Node`
//! with `updateType = NodeUpdateType.OBJECT`, whose `update( frame )` writes
//! blue into its `uniform( new Color() )` while the sphere is visible and green
//! once `frame.renderer.isOccluded( sphere )` says it is not. Here that node is
//! [`three_rs::nodes::tsl::uniform_frame`], whose callback receives a
//! [`three_rs::nodes::NodeFrame`] to ask. `docs/nodes.md` §36.
//!
//! The sphere is directly behind the plane, so it is occluded from the first
//! draw — but a query's answer comes back asynchronously, two frames after the
//! draw it measured at the earliest, so the graded (first) frame shows the
//! plane blue, as three's screenshot does, and it turns green from the third
//! frame on.

use std::rc::Rc;

use three_rs::addons::controls::OrbitControls;
use three_rs::geometries::{plane_geometry, sphere_geometry};
use three_rs::materials::Side;
use three_rs::nodes::tsl::uniform_frame;
use three_rs::nodes::Type;
use three_rs::{
    AmbientLight, Color, DirectionalLight, Mesh, MeshPhongNodeMaterial, PerspectiveCamera,
    Renderer, RendererParameters, Scene,
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
    let mut camera = PerspectiveCamera::new(50.0, INNER_WIDTH / INNER_HEIGHT, 0.01, 100.0);
    camera.node.borrow_mut().position.z = 7.0;

    let scene = Scene::new();

    // lights

    let ambient_light = AmbientLight::new(Color::from_hex(0xb0b0b0), 1.0);

    let light = DirectionalLight::new(Color::from_hex(0xFFFFFF), 1.0);
    light.borrow_mut().position.set(0.32, 0.39, 0.7);

    scene.add(&ambient_light);
    scene.add(&light);

    // models

    let plane_geometry = Rc::new(plane_geometry(2.0, 2.0, 1, 1));
    // `new THREE.SphereGeometry( 0.5 )`: 32 x 16 segments.
    let sphere_geometry = Rc::new(sphere_geometry(0.5, 32, 16));

    let mut plane_material = MeshPhongNodeMaterial::phong(Color::from_hex(0x00ff00));
    plane_material.side = Side::Double;
    let sphere = Mesh::new(
        sphere_geometry,
        MeshPhongNodeMaterial::phong(Color::from_hex(0xffff00)),
    );

    // `new OcclusionNode( sphere, new THREE.Color( 0x0000ff ), new
    // THREE.Color( 0x00ff00 ) )`: `update( frame )` copies the occluded or the
    // normal colour into its uniform, and `setup()` returns that uniform.
    let instance_uniform = {
        let test_object = sphere.clone();
        let normal_color = Color::from_hex(0x0000ff);
        let occluded_color = Color::from_hex(0x00ff00);
        uniform_frame(Type::Vec3, move |frame| {
            let is_occluded = frame.is_occluded(&test_object.borrow());
            let color = if is_occluded {
                occluded_color
            } else {
                normal_color
            };
            vec![color.r, color.g, color.b]
        })
    };

    plane_material.color_node = Some(instance_uniform);
    let plane = Mesh::new(plane_geometry, plane_material);

    {
        let mut sphere = sphere.borrow_mut();
        sphere.position.z = -1.0;
        sphere.occlusion_test = true;
    }

    scene.add(&plane);
    scene.add(&sphere);

    // renderer

    let mut renderer = Renderer::new(RendererParameters { antialias: true }).unwrap();
    renderer.set_pixel_ratio(DPR);
    renderer.set_size(INNER_WIDTH, INNER_HEIGHT);

    // controls

    // `new OrbitControls( camera, renderer.domElement )`: the constructor's
    // `update()` points the camera at the origin, which it already faces, and
    // 7 is inside `[ minDistance, maxDistance ]`.
    let mut controls = OrbitControls::new(&mut camera);
    controls.set_element_size(INNER_WIDTH, INNER_HEIGHT);
    controls.min_distance = 3.0;
    controls.max_distance = 25.0;

    App {
        renderer,
        scene,
        camera,
        controls,
    }
}

/// The page's `render()`, which `setAnimationLoop` calls every frame.
pub fn animate(app: &mut App) {
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
        .unwrap_or_else(|| "target/webgpu_occlusion.png".to_string());
    three_rs::testing::write_png(&path, width, height, &pixels);
    println!("wrote {path} ({width}x{height})");
}
