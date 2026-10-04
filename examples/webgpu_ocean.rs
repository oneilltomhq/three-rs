//! Port of `three.js/examples/webgpu_ocean.html`, calling the three-rs API in
//! the same order the page's `init()` does.
//!
//! Under the e2e harness the viewport is `400 * 2` x `250 * 2` with no
//! `deviceScaleFactor`, so `window.innerWidth` / `innerHeight` /
//! `devicePixelRatio` are 800, 500 and 1. The page draws from `Math.random()`
//! not at all.
//!
//! # What this page is
//!
//! A [`WaterMesh`] on a 10 000 x 10 000 plane, a [`SkyMesh`] box scaled to
//! 10 000 with the sun 2° above the horizon behind the camera's target, and a
//! 30-unit [`MeshStandardNodeMaterial`] cube with roughness 0 bobbing in the
//! water. The water reflects the sky and the cube through a planar mirror
//! rendered at half resolution, distorted by four scrolling taps of
//! `waternormals.jpg`. The cube is lit only by `scene.environment`, a PMREM of
//! the sky alone. The frame goes through a `RenderPipeline` that adds a faint
//! bloom (`strength` 0.1, `radius` 0, `threshold` 0) on top of the scene
//! pass, then ACES Filmic at exposure 0.1.
//!
//! # Order
//!
//! `renderer.init().then( updateSun )`: the PMREM is built after `init()`'s
//! synchronous body has run, and `updateSun()` moves the sky into its own
//! scene for `fromScene()` and then back with `scene.add( sky )`, which puts
//! it *after* the cube among the scene's children. [`update_sun`] runs at the
//! end of [`init`] for both reasons. The harness's first frame comes after the
//! promise has resolved, so it already sees the sun and the environment.
//!
//! Time is pinned to 0, so the water's `time` uniform is 0 and the cube sits
//! at `y = 5` with no rotation.
//!
//! The graded frame is the first, and on it the water is dark: upstream adds
//! the mirror's target to the water inside the material's `Fn()`, during the
//! first render and after the scene's world matrices were updated, so that
//! frame's mirror is the plane `z = 0` rather than the water. The port keeps
//! that timing ([`WaterMesh`]'s docs); from the second frame on, the water
//! reflects the sunset and the cube.
//!
//! `renderer.inspector = new Inspector()` and its parameters panel draw
//! nothing into the canvas and are not ported.

use std::cell::{RefCell, RefMut};
use std::f64::consts::PI;
use std::rc::Rc;

use three_rs::addons::controls::OrbitControls;
use three_rs::addons::objects::{SkyMesh, WaterMesh, WaterMeshOptions};
use three_rs::geometries::{box_geometry, plane_geometry};
use three_rs::loaders::TextureLoader;
use three_rs::math::math_utils::deg_to_rad;
use three_rs::nodes::display::{bloom, BloomNode};
use three_rs::nodes::pmrem_node::PmremEnvironment;
use three_rs::objects::Mesh;
use three_rs::textures::Wrapping;
use three_rs::utils::now_ms;
use three_rs::{
    pass, Color, MeshStandardNodeMaterial, Node, PassNode, PerspectiveCamera, RenderPipeline,
    Renderer, RendererParameters, Scene, ToneMapping, Vector3,
};

pub const INNER_WIDTH: f64 = 800.0;
pub const INNER_HEIGHT: f64 = 500.0;
/// `renderer.setPixelRatio( window.devicePixelRatio )`.
pub const DPR: f64 = 1.0;

/// The page's `parameters`: the GUI's values.
#[derive(Clone, Debug)]
pub struct Parameters {
    /// Degrees above the horizon.
    pub elevation: f64,
    /// Degrees.
    pub azimuth: f64,
    pub exposure: f64,
}

impl Default for Parameters {
    fn default() -> Self {
        Self {
            elevation: 2.0,
            azimuth: 180.0,
            exposure: 0.1,
        }
    }
}

pub struct App {
    pub renderer: Renderer,
    /// Shared with `scene_pass`, which renders it from `updateBefore()`.
    pub scene: Rc<RefCell<Scene>>,
    /// Shared with `scene_pass`.
    pub camera: Rc<RefCell<PerspectiveCamera>>,
    /// The page's `controls`.
    pub controls: OrbitControls,
    pub water: WaterMesh,
    pub sky: SkyMesh,
    /// The page's `mesh`, the cube.
    pub mesh: Node,
    /// The page's `sun`.
    pub sun: Vector3,
    pub parameters: Parameters,
    /// `sceneEnv`, the sky's own scene for `fromScene()`.
    pub scene_env: Scene,
    /// `renderTarget`, the PMREM `updateSun()` last built.
    pub environment: Option<PmremEnvironment>,
    pub scene_pass: PassNode,
    pub bloom_pass: BloomNode,
    pub render_pipeline: RenderPipeline,
}

/// `updateSun()`: point the sky's sun and the water's sun direction at
/// `parameters`, and rebuild `scene.environment` from the sky alone.
pub fn update_sun(app: &mut App) {
    let phi = deg_to_rad(90.0 - app.parameters.elevation);
    let theta = deg_to_rad(app.parameters.azimuth);

    app.sun.set_from_spherical_coords(1.0, phi, theta);

    let sun = app.sun;
    app.sky.sun_position.set(vec![sun.x, sun.y, sun.z]);
    // `water.sunDirection.value.copy( sun ).normalize()`.
    let direction = sun.normalized();
    app.water
        .sun_direction
        .set(vec![direction.x, direction.y, direction.z]);

    // `if ( renderTarget !== undefined ) renderTarget.dispose()` — dropping
    // the old environment releases it.
    app.environment = None;

    app.scene_env.add(&app.sky.mesh);
    let environment = PmremEnvironment::from_scene(&mut app.renderer, &mut app.scene_env, 0.0)
        .expect("three-rs: the sky's PMREM");
    app.scene.borrow().add(&app.sky.mesh);

    app.scene.borrow_mut().environment = Some(environment.handle());
    app.environment = Some(environment);
}

pub fn init() -> App {
    let mut renderer = Renderer::new(RendererParameters::default()).unwrap();
    renderer.set_pixel_ratio(DPR);
    renderer.set_size(INNER_WIDTH, INNER_HEIGHT);
    renderer.tone_mapping = ToneMapping::AcesFilmic;
    renderer.tone_mapping_exposure = 0.1;

    //

    let scene = Scene::new();

    let camera = PerspectiveCamera::new(55.0, INNER_WIDTH / INNER_HEIGHT, 1.0, 20000.0);
    camera.node.borrow_mut().position.set(30.0, 30.0, 100.0);

    // Post-processing

    let scene = Rc::new(RefCell::new(scene));
    let camera = Rc::new(RefCell::new(camera));
    let scene_pass = pass(scene.clone(), camera.clone());
    let scene_pass_color = scene_pass.texture_node("output");

    let bloom_pass = bloom(scene_pass_color.clone());
    bloom_pass.threshold.set(vec![0.0]);
    bloom_pass.strength.set(vec![0.1]);
    bloom_pass.radius.set(vec![0.0]);

    let mut render_pipeline = RenderPipeline::new();
    render_pipeline.output_node = Some(scene_pass_color.add(bloom_pass.node()));

    //

    let sun = Vector3::default();

    // Water

    let water_geometry = Rc::new(plane_geometry(10000.0, 10000.0, 1, 1));
    let loader = TextureLoader::new();
    let water_normals = loader
        .load(
            three_rs::testing::three_js_dir()
                .join("examples")
                .join("textures/waternormals.jpg"),
        )
        .expect("waternormals.jpg");
    water_normals.set_wrapping(Wrapping::Repeat, Wrapping::Repeat);

    let mut options = WaterMeshOptions::new(water_normals);
    options.sun_direction = Vector3::default();
    options.sun_color = Color::from_hex(0xffffff);
    options.water_color = Color::from_hex(0x001e0f);
    options.distortion_scale = 3.7;
    let water = WaterMesh::new(water_geometry, options);

    // `water.rotation.x = - Math.PI / 2` — the port's `rotation` has no
    // `onChange` into the quaternion, so it is set through `set_rotation`.
    water.mesh.borrow_mut().set_rotation(-PI / 2.0, 0.0, 0.0);

    scene.borrow().add(&water.mesh);

    // Skybox

    let sky = SkyMesh::new();
    sky.mesh.borrow_mut().scale.set_scalar(10000.0);
    scene.borrow().add(&sky.mesh);

    sky.turbidity.set(vec![10.0]);
    sky.rayleigh.set(vec![2.0]);
    sky.mie_coefficient.set(vec![0.005]);
    sky.mie_directional_g.set(vec![0.8]);
    sky.cloud_coverage.set(vec![0.4]);
    sky.cloud_density.set(vec![0.5]);
    sky.cloud_elevation.set(vec![0.5]);

    // `const pmremGenerator = new THREE.PMREMGenerator( renderer )` — the
    // port's `PmremEnvironment::from_scene` owns its generator.
    let scene_env = Scene::new();

    //

    let geometry = Rc::new(box_geometry(30.0, 30.0, 30.0, 1, 1, 1));
    // `new THREE.MeshStandardMaterial( { roughness: 0 } )`: white, metalness 0.
    let material = MeshStandardNodeMaterial::standard(Color::from_hex(0xffffff), 0.0, 0.0);

    let mesh = Mesh::new(geometry, material);
    scene.borrow().add(&mesh);

    //

    let mut controls = OrbitControls::new(&mut camera.borrow_mut());
    // The canvas the example renders at, standing in for the element's
    // `clientWidth` / `clientHeight`.
    controls.set_element_size(INNER_WIDTH, INNER_HEIGHT);
    controls.max_polar_angle = PI * 0.495;
    controls.target.set(0.0, 10.0, 0.0);
    controls.min_distance = 40.0;
    controls.max_distance = 200.0;
    controls.update(&mut camera.borrow_mut(), None);

    let mut app = App {
        renderer,
        scene,
        camera,
        controls,
        water,
        sky,
        mesh,
        sun,
        parameters: Parameters::default(),
        scene_env,
        environment: None,
        scene_pass,
        bloom_pass,
        render_pipeline,
    };

    // `renderer.init().then( updateSun )` — after the rest of `init()`; see
    // the module comment.
    update_sun(&mut app);

    app
}

/// The page's `render()`, the animation loop.
pub fn animate(app: &mut App) {
    let time = now_ms() * 0.001;

    {
        let mut mesh = app.mesh.borrow_mut();
        mesh.position.y = time.sin() * 20.0 + 5.0;
        // `mesh.rotation.x = time * 0.5; mesh.rotation.z = time * 0.51;`
        let y = mesh.rotation.y;
        mesh.set_rotation(time * 0.5, y, time * 0.51);
    }

    app.render_pipeline.render(&mut app.renderer);
}

/// The page's `onWindowResize()`.
///
/// The rung harness never calls this — the graded frame is always
/// 800 x 500 — but the viewer and the browser shell do, so the example
/// owns its own reaction to a resized canvas instead of the host
/// guessing at one.
pub fn resize(app: &mut App, width: f64, height: f64) {
    let mut camera = app.camera.borrow_mut();
    camera.aspect = width / height;
    camera.update_projection_matrix();

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
pub fn controls_and_camera(
    app: &mut App,
) -> Option<(&mut OrbitControls, RefMut<'_, PerspectiveCamera>)> {
    Some((&mut app.controls, app.camera.borrow_mut()))
}

fn main() {
    three_rs::testing::pin_time(Some(0.0));
    let mut app = init();
    println!("adapter: {:?}", app.renderer.adapter_info());
    animate(&mut app);

    let (width, height, pixels) = app.renderer.read_canvas_pixels().unwrap();
    let path = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "target/webgpu_ocean.png".to_string());
    three_rs::testing::write_png(&path, width, height, &pixels);
    println!("wrote {path} ({width}x{height})");
}
