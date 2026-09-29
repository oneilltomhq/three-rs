//! Port of `three.js/examples/webgpu_lights_custom.html`, calling the three-rs
//! API in the same order the page's `init()` / `animate()` do.
//!
//! Under the e2e harness the viewport is `400 * 2` x `250 * 2` with no
//! `deviceScaleFactor`, so `window.innerWidth` / `innerHeight` /
//! `devicePixelRatio` are 800, 500 and 1.
//!
//! `Date.now()` is pinned to 0, so `time` is 0: the three lights sit at
//! (0, 0.5, 0.5), (0.5, 0, 0) and (0, 0.5, 0), and `scene.rotation.y` is 0.
//!
//! The half million points are the page's own `Vector3().random()` draws —
//! x, y, z in that order — from the harness' seeded `Math.random`; nothing
//! on the page draws from it earlier. `OrbitControls` is constructed but never
//! updated, and the `resize` listener does not fire.

use std::rc::Rc;

use three_rs::addons::controls::OrbitControls;
use three_rs::core::Node;
use three_rs::materials::lighting_model::{DirectLightData, LightingBuilder, LightingModel};
use three_rs::testing::DeterministicRandom;
use three_rs::utils::date_now_ms;
use three_rs::{
    sphere_geometry, BufferGeometry, Color, Mesh, MeshBasicNodeMaterial, PerspectiveCamera,
    PointLight, Points, PointsNodeMaterial, Renderer, RendererParameters, Scene, Vector3,
};

pub const INNER_WIDTH: f64 = 800.0;
pub const INNER_HEIGHT: f64 = 500.0;
/// `renderer.setPixelRatio( window.devicePixelRatio )`
pub const DPR: f64 = 1.0;

/// The page's `class CustomLightingModel extends THREE.LightingModel`: every
/// direct light's colour goes straight into `directDiffuse`, with no `dotNL`
/// and no diffuse colour.
#[derive(Debug)]
struct CustomLightingModel;

impl LightingModel for CustomLightingModel {
    fn direct(&self, data: &DirectLightData, builder: &mut LightingBuilder) {
        builder.push(
            data.reflected_light
                .direct_diffuse
                .add_assign(data.light_color.clone()),
        );
    }
}

pub struct App {
    pub renderer: Renderer,
    pub scene: Scene,
    pub camera: PerspectiveCamera,
    /// `light1`, `light2`, `light3`, which `animate()` moves.
    pub lights: Vec<Node>,
    /// The page's `controls`.
    pub controls: OrbitControls,
}

pub fn init() -> App {
    let mut camera = PerspectiveCamera::new(70.0, INNER_WIDTH / INNER_HEIGHT, 0.1, 10.0);
    camera.node.borrow_mut().position.z = 1.5;

    let mut scene = Scene::new();
    scene.set_background(Color::from_hex(0x000000));

    // lights

    let sphere_geometry = Rc::new(sphere_geometry(0.02, 16, 8));

    let add_light = |scene: &Scene, hex: u32| -> Node {
        // `new THREE.NodeMaterial()` — a bare `NodeMaterial`, which the port
        // models as the `Basic` kind (see `MaterialKind`).
        let material = MeshBasicNodeMaterial {
            color_node: Some(Color::from_hex(hex).into()),
            // `lights()` — an empty `LightsNode`: "ignore scene lights".
            lights_node: Some(Vec::new()),
            ..Default::default()
        };

        let mesh = Mesh::new(sphere_geometry.clone(), material);

        let light = PointLight::new(Color::from_hex(hex), 0.1, 1.0);
        light.add(&mesh);

        scene.add(&light);
        light
    };

    let lights = vec![
        add_light(&scene, 0xffaa00),
        add_light(&scene, 0x0040ff),
        add_light(&scene, 0x80ff80),
    ];

    //light nodes ( selective lights )

    // `lights( [ light1, light2, light3 ] )` — indices into the renderer's
    // light list, which is scene-traversal order.
    let all_lights_node = vec![0, 1, 2];

    // points

    let mut random = DeterministicRandom::new();
    let points: Vec<Vector3> = (0..500_000)
        .map(|_| {
            let x = random.next();
            let y = random.next();
            let z = random.next();
            let mut point = Vector3::new(x, y, z);
            point.sub_scalar(0.5).multiply_scalar(3.0);
            point
        })
        .collect();

    let mut geometry_points = BufferGeometry::new();
    geometry_points.set_from_points(&points);
    let mut material_points = PointsNodeMaterial::points();

    // custom lighting model

    // `allLightsNode.context( { lightingModel } )`.
    material_points.lights_node = Some(all_lights_node);
    material_points.lighting_model = Some(Rc::new(CustomLightingModel));

    //

    let point_cloud = Points::new(Rc::new(geometry_points), material_points);
    scene.add(&point_cloud);

    //

    let mut parameters = RendererParameters::default();
    parameters.antialias = true;
    let mut renderer = Renderer::new(parameters).unwrap();
    renderer.set_pixel_ratio(DPR);
    renderer.set_size(INNER_WIDTH, INNER_HEIGHT);

    // controls

    let mut controls = OrbitControls::new(&mut camera);
    controls.set_element_size(INNER_WIDTH, INNER_HEIGHT);
    controls.min_distance = 0.0;
    controls.max_distance = 4.0;

    App {
        renderer,
        scene,
        camera,
        lights,
        controls,
    }
}

/// The page's `animate()`.
pub fn animate(app: &mut App) {
    let time = date_now_ms() * 0.001;
    let scale = 0.5;

    {
        let mut light = app.lights[0].borrow_mut();
        light.position.x = (time * 0.7).sin() * scale;
        light.position.y = (time * 0.5).cos() * scale;
        light.position.z = (time * 0.3).cos() * scale;
    }
    {
        let mut light = app.lights[1].borrow_mut();
        light.position.x = (time * 0.3).cos() * scale;
        light.position.y = (time * 0.5).sin() * scale;
        light.position.z = (time * 0.7).sin() * scale;
    }
    {
        let mut light = app.lights[2].borrow_mut();
        light.position.x = (time * 0.7).sin() * scale;
        light.position.y = (time * 0.3).cos() * scale;
        light.position.z = (time * 0.5).sin() * scale;
    }

    app.scene
        .node
        .borrow_mut()
        .set_rotation(0.0, time * 0.1, 0.0);

    app.renderer.render(&mut app.scene, &mut app.camera);
}

/// The page's `onWindowResize()`.
pub fn resize(app: &mut App, width: f64, height: f64) {
    app.camera.aspect = width / height;
    app.camera.update_projection_matrix();
    app.renderer.set_size(width, height);
}

/// The example's controls, for a host that has a pointer.
pub fn controls(app: &mut App) -> Option<&mut OrbitControls> {
    Some(&mut app.controls)
}

/// The controls and the camera at once; see `webgpu_lights_phong`.
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
        .unwrap_or_else(|| "target/webgpu_lights_custom.png".to_string());
    three_rs::testing::write_png(&path, width, height, &pixels);
    println!("wrote {path} ({width}x{height})");
}
