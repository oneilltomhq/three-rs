//! Port of `three.js/examples/webgpu_lights_physical.html`, calling the
//! three-rs API in the same order the page's `init()` / `animate()` do.
//!
//! Under the e2e harness the viewport is `400 * 2` x `250 * 2` with no
//! `deviceScaleFactor`, so `window.innerWidth` / `innerHeight` /
//! `devicePixelRatio` are 800, 500 and 1, and `Date.now()` is 0.
//!
//! `TextureLoader.load()` is asynchronous on the page, but the harness only
//! fires its single RAF once the network is idle, so all seven JPEGs are
//! present for the graded frame; here they are decoded synchronously.
//!
//! `OrbitControls`' constructor calls `update()`, which points the camera at
//! its target — the origin — so the camera's rotation is `lookAt( 0, 0, 0 )`.
//! The GUI only registers `onChange` callbacks and the `resize` listener never
//! fires, so neither touches the graded frame.
//!
//! `animate()` runs once before the render and mutates four things away from
//! their constructor values: the exposure (`0.68^5`), the bulb's power (400 lm),
//! the bulb material's emissive intensity (`intensity / 0.02^2`) and the
//! hemisphere light's intensity (0.0001 lx). `Date.now()` is 0, so
//! `bulbLight.position.y = cos( 0 ) * 0.75 + 1.25` is 2.0 — the same value
//! `init()` set, by coincidence.

use std::rc::Rc;

use three_rs::core::Object3DNode;
use three_rs::textures::Wrapping;
use three_rs::{
    box_geometry, plane_geometry, sphere_geometry, Color, ColorSpace, HemisphereLight, Mesh,
    MeshStandardNodeMaterial, Node, PerspectiveCamera, PointLight, Renderer, RendererParameters,
    Scene, ToneMapping, Vector3,
};

pub const INNER_WIDTH: f64 = 800.0;
pub const INNER_HEIGHT: f64 = 500.0;
/// `renderer.setPixelRatio( window.devicePixelRatio )`
pub const DPR: f64 = 1.0;

/// `params` — the GUI's defaults, which is what the graded frame renders with.
const EXPOSURE: f64 = 0.68;
/// `Object.keys( bulbLuminousPowers )[ 4 ]` — '400 lm (40W)'.
const BULB_POWER: f64 = 400.0;
/// `Object.keys( hemiLuminousIrradiances )[ 0 ]` — '0.0001 lx (Moonless Night)'.
const HEMI_IRRADIANCE: f64 = 0.0001;

pub struct App {
    pub renderer: Renderer,
    pub scene: Scene,
    pub camera: PerspectiveCamera,
    /// The page's module-level handles that `animate()` mutates.
    pub bulb_light: Node,
    pub hemi_light: Node,
    pub bulb_mesh: Node,
}

fn examples_dir() -> std::path::PathBuf {
    let three = three_rs::testing::three_js_dir();
    three.join("examples")
}

pub fn init() -> App {
    let mut camera = PerspectiveCamera::new(50.0, INNER_WIDTH / INNER_HEIGHT, 0.1, 100.0);
    camera.node.borrow_mut().position.set(-4.0, 2.0, 4.0);

    let scene = Scene::new();

    let bulb_geometry = Rc::new(sphere_geometry(0.02, 16, 8));
    let bulb_light = PointLight::new(Color::from_hex(0xffee88), 1.0, 100.0);

    let mut bulb_mat = MeshStandardNodeMaterial::standard(Color::from_hex(0x000000), 1.0, 0.0);
    bulb_mat.emissive = Color::from_hex(0xffffee);
    bulb_mat.emissive_intensity = 1.0;

    let bulb_mesh = Mesh::new(bulb_geometry);
    bulb_mesh.borrow_mut().mesh_mut().unwrap().material = Some(bulb_mat);
    bulb_light.add(&bulb_mesh);
    bulb_light.borrow_mut().position.set(0.0, 2.0, 0.0);
    bulb_light.borrow_mut().cast_shadow = true;
    scene.add(&bulb_light);

    let hemi_light = HemisphereLight::new(
        Color::from_hex(0xddeeff),
        Color::from_hex(0x0f0e0d),
        0.02,
    );
    scene.add(&hemi_light);

    // floorMat
    let mut floor_mat = MeshStandardNodeMaterial::standard(Color::from_hex(0xffffff), 0.8, 0.2);
    floor_mat.bump_scale = 1.0;

    let texture_loader = three_rs::TextureLoader::new();

    let floor_map = texture_loader.load(examples_dir().join("textures/hardwood2_diffuse.jpg"));
    floor_map.set_wrapping(Wrapping::Repeat, Wrapping::Repeat);
    floor_map.set_anisotropy(4);
    floor_map.set_repeat(10.0, 24.0);
    floor_map.set_color_space(ColorSpace::SRGB);
    floor_mat.map = Some(floor_map);

    let floor_bump = texture_loader.load(examples_dir().join("textures/hardwood2_bump.jpg"));
    floor_bump.set_wrapping(Wrapping::Repeat, Wrapping::Repeat);
    floor_bump.set_anisotropy(4);
    floor_bump.set_repeat(10.0, 24.0);
    floor_mat.bump_map = Some(floor_bump);

    let floor_roughness =
        texture_loader.load(examples_dir().join("textures/hardwood2_roughness.jpg"));
    floor_roughness.set_wrapping(Wrapping::Repeat, Wrapping::Repeat);
    floor_roughness.set_anisotropy(4);
    floor_roughness.set_repeat(10.0, 24.0);
    floor_mat.roughness_map = Some(floor_roughness);

    // cubeMat
    let mut cube_mat = MeshStandardNodeMaterial::standard(Color::from_hex(0xffffff), 0.7, 0.2);
    cube_mat.bump_scale = 1.0;

    let brick_map = texture_loader.load(examples_dir().join("textures/brick_diffuse.jpg"));
    brick_map.set_wrapping(Wrapping::Repeat, Wrapping::Repeat);
    brick_map.set_anisotropy(4);
    brick_map.set_repeat(1.0, 1.0);
    brick_map.set_color_space(ColorSpace::SRGB);
    cube_mat.map = Some(brick_map);

    let brick_bump = texture_loader.load(examples_dir().join("textures/brick_bump.jpg"));
    brick_bump.set_wrapping(Wrapping::Repeat, Wrapping::Repeat);
    brick_bump.set_anisotropy(4);
    brick_bump.set_repeat(1.0, 1.0);
    cube_mat.bump_map = Some(brick_bump);

    // ballMat
    let mut ball_mat = MeshStandardNodeMaterial::standard(Color::from_hex(0xffffff), 0.5, 1.0);

    let earth_map =
        texture_loader.load(examples_dir().join("textures/planets/earth_atmos_2048.jpg"));
    earth_map.set_anisotropy(4);
    earth_map.set_color_space(ColorSpace::SRGB);
    ball_mat.map = Some(earth_map);

    // The page marks the metalness map sRGB too. Faithful port: reproduce it.
    let earth_specular =
        texture_loader.load(examples_dir().join("textures/planets/earth_specular_2048.jpg"));
    earth_specular.set_anisotropy(4);
    earth_specular.set_color_space(ColorSpace::SRGB);
    ball_mat.metalness_map = Some(earth_specular);

    let floor_geometry = Rc::new(plane_geometry(20.0, 20.0, 1, 1));
    let floor_mesh = Mesh::new(floor_geometry);
    floor_mesh.borrow_mut().mesh_mut().unwrap().material = Some(floor_mat);
    floor_mesh.borrow_mut().receive_shadow = true;
    floor_mesh
        .borrow_mut()
        .set_rotation(-std::f64::consts::PI / 2.0, 0.0, 0.0);
    scene.add(&floor_mesh);

    let ball_geometry = Rc::new(sphere_geometry(0.25, 32, 32));
    let ball_mesh = Mesh::new(ball_geometry);
    ball_mesh.borrow_mut().mesh_mut().unwrap().material = Some(ball_mat);
    ball_mesh.borrow_mut().position.set(1.0, 0.25, 1.0);
    ball_mesh
        .borrow_mut()
        .set_rotation(0.0, std::f64::consts::PI, 0.0);
    ball_mesh.borrow_mut().cast_shadow = true;
    scene.add(&ball_mesh);

    let box_geom = Rc::new(box_geometry(0.5, 0.5, 0.5, 1, 1, 1));
    for position in [
        Vector3::new(-0.5, 0.25, -1.0),
        Vector3::new(0.0, 0.25, -5.0),
        Vector3::new(7.0, 0.25, 0.0),
    ] {
        let box_mesh = Mesh::new(box_geom.clone());
        box_mesh.borrow_mut().mesh_mut().unwrap().material = Some(cube_mat.clone());
        box_mesh.borrow_mut().position = position;
        box_mesh.borrow_mut().cast_shadow = true;
        scene.add(&box_mesh);
    }

    let mut renderer = Renderer::new(RendererParameters { antialias: false });
    renderer.set_pixel_ratio(DPR);
    renderer.set_size(INNER_WIDTH, INNER_HEIGHT);
    renderer.shadow_map_enabled = true;
    renderer.tone_mapping = ToneMapping::Reinhard;

    // `new OrbitControls( camera, renderer.domElement )` — its `update()` points
    // the camera at the target.
    camera.look_at(&Vector3::new(0.0, 0.0, 0.0));

    App {
        renderer,
        scene,
        camera,
        bulb_light,
        hemi_light,
        bulb_mesh,
    }
}

/// The page's `animate()`. `Date.now()` is 0 under the harness.
pub fn animate(app: &mut App) {
    app.renderer.tone_mapping_exposure = EXPOSURE.powf(5.0);
    app.renderer.shadow_map_enabled = true;
    app.bulb_light.borrow_mut().cast_shadow = true;

    // `bulbLight.power = …` sets `intensity = power / ( 4 * PI )`.
    app.bulb_light
        .borrow_mut()
        .light_mut()
        .unwrap()
        .set_power(BULB_POWER);
    let intensity = app.bulb_light.borrow().light().unwrap().light.intensity;

    // `bulbMat.emissiveIntensity = bulbLight.intensity / Math.pow( 0.02, 2 )`.
    app.bulb_mesh
        .borrow_mut()
        .mesh_mut()
        .unwrap()
        .material
        .as_mut()
        .unwrap()
        .emissive_intensity = intensity / 0.02f64.powf(2.0);

    app.hemi_light
        .borrow_mut()
        .light_mut()
        .unwrap()
        .light
        .intensity = HEMI_IRRADIANCE;

    let time = 0.0f64;
    app.bulb_light.borrow_mut().position.y = time.cos() * 0.75 + 1.25;

    app.renderer.render(&mut app.scene, &mut app.camera);
}

fn main() {
    let mut app = init();
    println!("adapter: {:?}", app.renderer.adapter_info());
    animate(&mut app);

    let (width, height, pixels) = app.renderer.read_canvas_pixels();
    let path = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "target/webgpu_lights_physical.png".to_string());
    three_rs::testing::write_png(&path, width, height, &pixels);
    println!("wrote {path} ({width}x{height})");
}
