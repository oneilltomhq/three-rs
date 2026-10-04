//! Port of `three.js/examples/webgpu_lightprobe.html`, calling the three-rs
//! API in the same order the page's `init()` / `animate()` do.
//!
//! Under the e2e harness the viewport is `400 * 2` x `250 * 2` with no
//! `deviceScaleFactor`, so `window.innerWidth` / `innerHeight` /
//! `devicePixelRatio` are 800, 500 and 1.
//!
//! The page draws from `Math.random()` not at all, so the harness' seeded
//! sequence is never touched.
//!
//! `CubeTextureLoader.load()` is asynchronous on the page, but the harness only
//! fires its single RAF once the network is idle, so everything inside its
//! callback — the background, the probe's coefficients, the sphere and the
//! helper — is in place for the graded frame; here it runs inline.
//!
//! # The light probe
//!
//! `LightProbeGenerator.fromCubeTexture()` projects the pisa cube onto nine
//! spherical-harmonic coefficients on the CPU, and the probe then adds
//! `getShIrradianceAt( normalWorld, sh )` to every lit material's
//! `irradiance` — the only diffuse light the sphere gets besides the
//! directional light, since a roughness-0 envMap contributes almost nothing
//! diffuse. The helper on the left draws that irradiance over π on a unit
//! sphere at the probe's position, which the lighting itself ignores. It
//! follows the probe through `LightProbeHelper.onBeforeRender()`, the
//! `on_before_render` hook the renderer calls before each draw of it.
//!
//! # The PMREM
//!
//! `envMap: cubeTexture` on a `MeshStandardMaterial` becomes a `PMREMNode`
//! in three; the port makes the atlas explicit, as `webgpu_pmrem_cubemap`
//! does: one [`PmremEnvironment`] built before the first render and handed
//! to the material as `pmrem_env`.

use std::rc::Rc;

use three_rs::addons::controls::OrbitControls;
use three_rs::addons::helpers::LightProbeHelper;
use three_rs::addons::lights::LightProbeGenerator;
use three_rs::geometries::sphere_geometry;
use three_rs::materials::ToneMapping;
use three_rs::math::SphericalHarmonics3;
use three_rs::nodes::pmrem_node::PmremEnvironment;
use three_rs::{
    Color, CubeTextureLoader, DirectionalLight, LightProbe, Mesh, MeshStandardNodeMaterial,
    PerspectiveCamera, Renderer, RendererParameters, Scene,
};

pub const INNER_WIDTH: f64 = 800.0;
pub const INNER_HEIGHT: f64 = 500.0;
/// `renderer.setPixelRatio( window.devicePixelRatio )`
pub const DPR: f64 = 1.0;

/// The page's `API` — the GUI's starting values.
const LIGHT_PROBE_INTENSITY: f64 = 1.0;
const DIRECTIONAL_LIGHT_INTENSITY: f64 = 0.6;
const ENV_MAP_INTENSITY: f64 = 1.0;

pub struct App {
    pub renderer: Renderer,
    pub scene: Scene,
    pub camera: PerspectiveCamera,
    /// The page's `controls`.
    pub controls: OrbitControls,
    /// The PMREM of the pisa cube that `envMap: cubeTexture` reads.
    pub environment: PmremEnvironment,
    /// The page's `lightProbe`.
    pub light_probe: three_rs::core::Node,
    /// The page's `directionalLight`.
    pub directional_light: three_rs::core::Node,
    /// The `LightProbeHelper` beside the sphere.
    pub helper: LightProbeHelper,
}

fn examples_dir() -> std::path::PathBuf {
    three_rs::testing::three_js_dir().join("examples")
}

pub fn init() -> App {
    // renderer
    let mut parameters = RendererParameters::default();
    parameters.antialias = true;
    let mut renderer = Renderer::new(parameters).unwrap();
    renderer.set_pixel_ratio(DPR);
    renderer.set_size(INNER_WIDTH, INNER_HEIGHT);

    // tone mapping
    renderer.tone_mapping = ToneMapping::None;

    // scene
    let mut scene = Scene::new();

    // camera
    let mut camera = PerspectiveCamera::new(40.0, INNER_WIDTH / INNER_HEIGHT, 1.0, 1000.0);
    camera.node.borrow_mut().position.set(0.0, 0.0, 30.0);

    // controls
    let mut controls = OrbitControls::new(&mut camera);
    controls.set_element_size(INNER_WIDTH, INNER_HEIGHT);
    controls.min_distance = 10.0;
    controls.max_distance = 50.0;
    controls.enable_pan = false;

    // probe
    let light_probe = LightProbe::new(SphericalHarmonics3::default(), 1.0);
    scene.add(&light_probe);

    // light
    let directional_light =
        DirectionalLight::new(Color::from_hex(0xffffff), DIRECTIONAL_LIGHT_INTENSITY);
    directional_light
        .borrow_mut()
        .position
        .set(10.0, 10.0, 10.0);
    scene.add(&directional_light);

    // envmap
    let cube_texture = CubeTextureLoader::new()
        .set_path(examples_dir().join("textures/cube/pisa/"))
        .load(["px.png", "nx.png", "py.png", "ny.png", "pz.png", "nz.png"])
        .unwrap();

    scene.set_background(cube_texture.clone());

    LightProbe::copy(
        &light_probe,
        &LightProbeGenerator::from_cube_texture(&cube_texture).unwrap(),
    );
    {
        let mut object = light_probe.borrow_mut();
        object.light_mut().unwrap().light.intensity = LIGHT_PROBE_INTENSITY;
        // position not used in scene lighting calculations (helper honors the
        // position, however)
        object.position.set(-10.0, 0.0, 0.0);
    }

    let geometry = Rc::new(sphere_geometry(5.0, 64, 32));

    let mut environment = PmremEnvironment::new(&cube_texture);
    environment.update(&mut renderer).unwrap();

    let mut material = MeshStandardNodeMaterial::standard(Color::from_hex(0xffffff), 0.0, 0.0);
    material.pmrem_env = Some(environment.handle());
    // `envMapIntensity: API.envMapIntensity` is 1, which is the only value
    // the port's `materialEnvIntensity` uniform takes.
    let _ = ENV_MAP_INTENSITY;

    // mesh
    let mesh = Mesh::new(geometry, material);
    scene.add(&mesh);

    // helper
    let helper = LightProbeHelper::new(&light_probe, 1.0);
    scene.add(&helper.node);

    App {
        renderer,
        scene,
        camera,
        controls,
        environment,
        light_probe,
        directional_light,
        helper,
    }
}

/// The page's `animate()`.
pub fn animate(app: &mut App) {
    app.environment.update(&mut app.renderer).unwrap();
    app.renderer.render(&mut app.scene, &mut app.camera);
}

/// The page's `onWindowResize()`.
pub fn resize(app: &mut App, width: f64, height: f64) {
    app.renderer.set_size(width, height);

    app.camera.aspect = width / height;
    app.camera.update_projection_matrix();
}

/// The example's controls, for a host that has a pointer.
pub fn controls(app: &mut App) -> Option<&mut OrbitControls> {
    Some(&mut app.controls)
}

/// The controls and the camera at once, which every one of the controls'
/// event handlers needs.
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
        .unwrap_or_else(|| "target/webgpu_lightprobe.png".to_string());
    three_rs::testing::write_png(&path, width, height, &pixels);
    println!("wrote {path} ({width}x{height})");
}
