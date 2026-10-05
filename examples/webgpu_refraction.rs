//! Port of `three.js/examples/webgpu_refraction.html`, calling the three-rs
//! API in the same order the page's `init()` / `animate()` do.
//!
//! Under the e2e harness the viewport is `400 * 2` x `250 * 2` with no
//! `deviceScaleFactor`, so `window.innerWidth` / `innerHeight` /
//! `devicePixelRatio` are 800, 500 and 1.
//!
//! `webgpu_mirror`'s box of five Phong planes and flat-shaded icosahedron,
//! with a vertical plane across its middle that refracts: its
//! `MeshBasicNodeMaterial` has a `backdropNode` of
//! `viewportSharedTexture( viewportSafeUV( screenUV + offset ) )`, the offset
//! from a checkerboard normal map. The plane is transparent, so it draws
//! after the opaque walls and the icosahedron; before it does, the renderer
//! copies the frame so far into the shared viewport texture, and the depth
//! buffer into `viewportSafeUV`'s depth texture, which turns the offset off
//! where it would pull in something in front of the plane — the half of the
//! icosahedron on the camera's side.
//!
//! `TextureLoader.load()` is asynchronous on the page, but the harness only
//! fires its single RAF once the network is idle, so the image is always
//! present for the graded frame; here it is decoded synchronously.
//!
//! `renderer.inspector = new Inspector()` and `.toInspector( … )` only register
//! the nodes for the inspector panel and change nothing about the graded frame.

use std::f64::consts::PI;
use std::rc::Rc;

use three_rs::addons::controls::OrbitControls;
use three_rs::core::ObjectRef;
use three_rs::geometries::{icosahedron_geometry, plane_geometry};
use three_rs::nodes::display::{viewport_safe_uv, viewport_shared_texture_at};
use three_rs::nodes::tsl::{screen_uv, texture_uv, uv};
use three_rs::nodes::NodeRef;
use three_rs::textures::{Texture, Wrapping};
use three_rs::utils::now_ms;
use three_rs::{
    Color, Mesh, MeshBasicNodeMaterial, MeshPhongNodeMaterial, PerspectiveCamera, PointLight,
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
    pub camera_controls: OrbitControls,
    pub small_sphere: ObjectRef,
}

fn examples_dir() -> std::path::PathBuf {
    three_rs::testing::three_js_dir().join("examples")
}

/// `new THREE.MeshPhongMaterial( { color } )`.
fn phong(hex: u32) -> MeshPhongNodeMaterial {
    MeshPhongNodeMaterial::phong(Color::from_hex(hex))
}

/// `new THREE.PointLight( color, intensity, distance, 0 )`.
fn point_light(hex: u32, intensity: f64, distance: f64) -> ObjectRef {
    let light = PointLight::new(Color::from_hex(hex), intensity, distance);
    light.borrow_mut().light_mut().unwrap().decay = 0.0;
    light
}

/// The refractor's `MeshBasicNodeMaterial( { backdropNode } )`, transparent.
/// `pub` so `examples/dump_wgsl.rs` can print its WGSL without a renderer.
pub fn refractor_material(floor_normal: &Texture) -> MeshBasicNodeMaterial {
    let vertical_normal_scale = 0.1;
    let vertical_uv_offset = texture_uv(floor_normal, uv().mul(5.0))
        .xy()
        .mul(2.0)
        .sub(1.0)
        .mul(vertical_normal_scale);

    let refractor_uv: NodeRef = screen_uv().add(vertical_uv_offset);
    let vertical_refractor = viewport_shared_texture_at(viewport_safe_uv(refractor_uv));

    let mut material = MeshBasicNodeMaterial::new();
    material.backdrop_node = Some(vertical_refractor);
    material.transparent = true;
    material
}

pub fn init() -> App {
    // scene
    let scene = Scene::new();

    // camera
    let mut camera = PerspectiveCamera::new(45.0, INNER_WIDTH / INNER_HEIGHT, 1.0, 500.0);
    camera.node.borrow_mut().position.set(0.0, 50.0, 160.0);

    //

    let geometry = Rc::new(icosahedron_geometry(5.0, 0));
    let mut material = phong(0xffffff);
    material.emissive = Color::from_hex(0x7b7b7b);
    material.flat_shading = true;
    let small_sphere = Mesh::new(geometry, material);
    scene.add(&small_sphere);

    // textures

    let loader = three_rs::TextureLoader::new();

    let floor_normal = loader
        .load(examples_dir().join("textures/floors/FloorsCheckerboard_S_Normal.jpg"))
        .unwrap();
    floor_normal.set_wrapping(Wrapping::Repeat, Wrapping::Repeat);

    // refractor

    let plane_geo = Rc::new(plane_geometry(100.1, 100.1, 1, 1));

    let plane_refractor = Mesh::new(plane_geo.clone(), refractor_material(&floor_normal));
    plane_refractor.borrow_mut().position.y = 50.0;
    scene.add(&plane_refractor);

    // walls

    let plane_top = Mesh::new(plane_geo.clone(), phong(0xffffff));
    {
        let mut plane = plane_top.borrow_mut();
        plane.position.y = 100.0;
        plane.rotate_x(PI / 2.0);
    }
    scene.add(&plane_top);

    let plane_bottom = Mesh::new(plane_geo.clone(), phong(0xffffff));
    plane_bottom.borrow_mut().rotate_x(-PI / 2.0);
    scene.add(&plane_bottom);

    let plane_back = Mesh::new(plane_geo.clone(), phong(0x7f7fff));
    {
        let mut plane = plane_back.borrow_mut();
        plane.position.z = -50.0;
        plane.position.y = 50.0;
    }
    scene.add(&plane_back);

    let plane_right = Mesh::new(plane_geo.clone(), phong(0x00ff00));
    {
        let mut plane = plane_right.borrow_mut();
        plane.position.x = 50.0;
        plane.position.y = 50.0;
        plane.rotate_y(-PI / 2.0);
    }
    scene.add(&plane_right);

    let plane_left = Mesh::new(plane_geo, phong(0xff0000));
    {
        let mut plane = plane_left.borrow_mut();
        plane.position.x = -50.0;
        plane.position.y = 50.0;
        plane.rotate_y(PI / 2.0);
    }
    scene.add(&plane_left);

    // lights

    let main_light = point_light(0xe7e7e7, 2.5, 250.0);
    main_light.borrow_mut().position.y = 60.0;
    scene.add(&main_light);

    let green_light = point_light(0x00ff00, 0.5, 1000.0);
    green_light.borrow_mut().position.set(550.0, 50.0, 0.0);
    scene.add(&green_light);

    let red_light = point_light(0xff0000, 0.5, 1000.0);
    red_light.borrow_mut().position.set(-550.0, 50.0, 0.0);
    scene.add(&red_light);

    let blue_light = point_light(0xbbbbfe, 0.5, 1000.0);
    blue_light.borrow_mut().position.set(0.0, 50.0, 550.0);
    scene.add(&blue_light);

    // renderer

    // `new THREE.WebGPURenderer( /*{ antialias: true }*/ )`: no MSAA, so the
    // depth buffer `viewportSafeUV` reads can be copied as it is.
    let mut renderer = Renderer::new(RendererParameters::default()).unwrap();
    renderer.set_pixel_ratio(DPR);
    renderer.set_size(INNER_WIDTH, INNER_HEIGHT);

    // controls

    let mut camera_controls = OrbitControls::new(&mut camera);
    // The canvas the example renders at, standing in for the element's
    // `clientWidth` / `clientHeight`.
    camera_controls.set_element_size(INNER_WIDTH, INNER_HEIGHT);
    camera_controls.target.set(0.0, 50.0, 0.0);
    camera_controls.max_distance = 400.0;
    camera_controls.min_distance = 10.0;
    camera_controls.update(&mut camera, None);

    App {
        renderer,
        scene,
        camera,
        camera_controls,
        small_sphere,
    }
}

/// The page's `animate()`. `Date.now()` is 0 under the harness, so `timer` is
/// 0 and the icosahedron sits at `( 30, 25, 0 )`, half through the refractor.
pub fn animate(app: &mut App) {
    let timer = now_ms() * 0.01;

    {
        let mut small_sphere = app.small_sphere.borrow_mut();
        small_sphere.position.set(
            (timer * 0.1).cos() * 30.0,
            (timer * 0.2).cos().abs() * 20.0 + 5.0,
            (timer * 0.1).sin() * 30.0,
        );
        let rotation = small_sphere.rotation;
        small_sphere.set_rotation(rotation.x, (PI / 2.0) - timer * 0.1, timer * 0.8);
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

/// The example's controls, for a host that has a pointer.
pub fn controls(app: &mut App) -> Option<&mut OrbitControls> {
    Some(&mut app.camera_controls)
}

/// The controls and the camera at once, which every one of the controls'
/// event handlers needs: the JS holds the camera as `this.object` and Rust
/// cannot, so `pointer_move` and the rest take it as an argument.
pub fn controls_and_camera(app: &mut App) -> Option<(&mut OrbitControls, &mut PerspectiveCamera)> {
    Some((&mut app.camera_controls, &mut app.camera))
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
        .unwrap_or_else(|| "target/webgpu_refraction.png".to_string());
    three_rs::testing::write_png(&path, width, height, &pixels);
    println!("wrote {path} ({width}x{height})");
}
