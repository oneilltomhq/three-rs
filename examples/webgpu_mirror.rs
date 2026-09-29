//! Port of `three.js/examples/webgpu_mirror.html`, calling the three-rs API in
//! the same order the page's `init()` / `animate()` do.
//!
//! Under the e2e harness the viewport is `400 * 2` x `250 * 2` with no
//! `deviceScaleFactor`, so `window.innerWidth` / `innerHeight` /
//! `devicePixelRatio` are 800, 500 and 1.
//!
//! A Cornell-style box of six Phong planes around a half sphere and a
//! flat-shaded icosahedron, lit by four point lights. The floor and the back
//! wall are mirrors: each carries a `reflector()`, which renders the scene
//! from the camera mirrored in its plane into a half-float target before the
//! plane draws, and samples it at `screenUV.flipX()` nudged by a normal map.
//! With the default `bounces: true` each mirror also renders while the other
//! one is rendering, so the frame is five scene renders: three's dump has the
//! four reflector passes nested exactly as `docs/nodes.md` §55 lists them.
//!
//! `TextureLoader.load()` is asynchronous on the page, but the harness only
//! fires its single RAF once the network is idle, so the images are always
//! present for the graded frame; here they are decoded synchronously.
//!
//! `renderer.inspector = new Inspector()` and `.toInspector( … )` only register
//! the nodes for the inspector panel and change nothing about the graded frame.

use std::f64::consts::PI;
use std::rc::Rc;

use three_rs::addons::controls::OrbitControls;
use three_rs::core::{Node, Object3D};
use three_rs::geometries::{
    cylinder_geometry_full, icosahedron_geometry, plane_geometry, sphere_geometry_full,
};
use three_rs::nodes::reflector_node::{reflector, ReflectorParameters};
use three_rs::nodes::tsl::{texture, texture_uv, uv};
use three_rs::nodes::NodeRef;
use three_rs::textures::Wrapping;
use three_rs::utils::now_ms;
use three_rs::{
    Color, ColorSpace, Mesh, MeshPhongNodeMaterial, PerspectiveCamera, PointLight, Renderer,
    RendererParameters, Scene,
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
    pub sphere_group: Node,
    pub small_sphere: Node,
}

fn examples_dir() -> std::path::PathBuf {
    three_rs::testing::three_js_dir().join("examples")
}

/// `new THREE.MeshPhongMaterial( { color } )`.
fn phong(hex: u32) -> MeshPhongNodeMaterial {
    MeshPhongNodeMaterial::phong(Color::from_hex(hex))
}

/// `new THREE.PointLight( color, intensity, distance, 0 )`.
fn point_light(hex: u32, intensity: f64, distance: f64) -> Node {
    let light = PointLight::new(Color::from_hex(hex), intensity, distance);
    light.borrow_mut().light_mut().unwrap().decay = 0.0;
    light
}

pub fn init() -> App {
    // scene
    let mut scene = Scene::new();
    scene.set_background(Color::from_hex(0x000000));

    // camera
    let mut camera = PerspectiveCamera::new(45.0, INNER_WIDTH / INNER_HEIGHT, 1.0, 500.0);
    camera.node.borrow_mut().position.set(0.0, 75.0, 160.0);

    //

    let sphere_group = Object3D::default().into_node();
    scene.add(&sphere_group);

    let geometry = Rc::new(cylinder_geometry_full(
        0.1,
        15.0 * (PI / 180.0 * 30.0).cos(),
        0.1,
        24,
        1,
        false,
        0.0,
        PI * 2.0,
    ));
    let mut material = phong(0xffffff);
    material.emissive = Color::from_hex(0x8d8d8d);
    let sphere_cap = Mesh::new(geometry, material.clone());
    {
        let mut cap = sphere_cap.borrow_mut();
        cap.position.y = -15.0 * (PI / 180.0 * 30.0).sin() - 0.05;
        cap.rotate_x(-PI);
    }

    let geometry = Rc::new(sphere_geometry_full(
        15.0,
        24,
        24,
        PI / 2.0,
        PI * 2.0,
        0.0,
        PI / 180.0 * 120.0,
    ));
    let half_sphere = Mesh::new(geometry, material);
    half_sphere.add(&sphere_cap);
    {
        let mut half = half_sphere.borrow_mut();
        half.rotate_x(-PI / 180.0 * 135.0);
        half.rotate_z(-PI / 180.0 * 20.0);
        half.position.y = 7.5 + 15.0 * (PI / 180.0 * 30.0).sin();
    }

    sphere_group.add(&half_sphere);

    let geometry = Rc::new(icosahedron_geometry(5.0, 0));
    let mut material = phong(0xffffff);
    material.emissive = Color::from_hex(0x7b7b7b);
    material.flat_shading = true;
    let small_sphere = Mesh::new(geometry, material);
    scene.add(&small_sphere);

    // textures

    let texture_loader = three_rs::TextureLoader::new();

    let floor_normal = texture_loader
        .load(examples_dir().join("textures/floors/FloorsCheckerboard_S_Normal.jpg"))
        .unwrap();
    floor_normal.set_wrapping(Wrapping::Repeat, Wrapping::Repeat);

    let decal_diffuse = texture_loader
        .load(examples_dir().join("textures/decal/decal-diffuse.png"))
        .unwrap();
    decal_diffuse.set_color_space(ColorSpace::Srgb);

    let decal_normal = texture_loader
        .load(examples_dir().join("textures/decal/decal-normal.jpg"))
        .unwrap();

    // reflectors / mirrors

    let mut ground_reflector = reflector(ReflectorParameters::default());
    let mut vertical_reflector = reflector(ReflectorParameters::default());

    let ground_normal_scale = -0.08;
    let vertical_normal_scale = 0.1;

    let ground_uv_offset = texture(&decal_normal)
        .xy()
        .mul(2.0)
        .sub(1.0)
        .mul(ground_normal_scale);
    let vertical_uv_offset = texture_uv(&floor_normal, uv().mul(5.0))
        .xy()
        .mul(2.0)
        .sub(1.0)
        .mul(vertical_normal_scale);

    ground_reflector.uv_node = ground_reflector.uv_node.add(ground_uv_offset);
    vertical_reflector.uv_node = vertical_reflector.uv_node.add(vertical_uv_offset);

    let ground_node = texture(&decal_diffuse)
        .a()
        .mix(Color::from_hex(0xffffff), &ground_reflector);
    let vertical_node = NodeRef::from(Color::from_hex(0x0000ff))
        .mul(0.1)
        .add(&vertical_reflector);

    // walls

    let plane_geo = Rc::new(plane_geometry(100.1, 100.1, 1, 1));

    //

    let mut material = MeshPhongNodeMaterial::phong(Color::from_hex(0xffffff));
    material.color_node = Some(ground_node);
    let plane_bottom = Mesh::new(plane_geo.clone(), material);
    plane_bottom.borrow_mut().rotate_x(-PI / 2.0);
    plane_bottom.add(&ground_reflector.target());
    scene.add(&plane_bottom);

    let mut material = MeshPhongNodeMaterial::phong(Color::from_hex(0xffffff));
    material.color_node = Some(vertical_node);
    let plane_back = Mesh::new(plane_geo.clone(), material);
    {
        let mut plane = plane_back.borrow_mut();
        plane.position.z = -50.0;
        plane.position.y = 50.0;
    }
    plane_back.add(&vertical_reflector.target());
    scene.add(&plane_back);

    //

    let plane_top = Mesh::new(plane_geo.clone(), phong(0xffffff));
    {
        let mut plane = plane_top.borrow_mut();
        plane.position.y = 100.0;
        plane.rotate_x(PI / 2.0);
    }
    scene.add(&plane_top);

    let plane_front = Mesh::new(plane_geo.clone(), phong(0x7f7fff));
    {
        let mut plane = plane_front.borrow_mut();
        plane.position.z = 50.0;
        plane.position.y = 50.0;
        plane.rotate_y(PI);
    }
    scene.add(&plane_front);

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

    let mut renderer = Renderer::new(RendererParameters { antialias: true }).unwrap();
    renderer.set_pixel_ratio(DPR);
    renderer.set_size(INNER_WIDTH, INNER_HEIGHT);

    // controls

    let mut camera_controls = OrbitControls::new(&mut camera);
    // The canvas the example renders at, standing in for the element's
    // `clientWidth` / `clientHeight`.
    camera_controls.set_element_size(INNER_WIDTH, INNER_HEIGHT);
    camera_controls.target.set(0.0, 40.0, 0.0);
    camera_controls.max_distance = 400.0;
    camera_controls.min_distance = 10.0;
    camera_controls.update(&mut camera, None);

    App {
        renderer,
        scene,
        camera,
        camera_controls,
        sphere_group,
        small_sphere,
    }
}

/// The page's `animate()`. `Date.now()` is 0 under the harness, so `timer` is
/// 0 and the icosahedron sits at `( 30, 25, 0 )`.
pub fn animate(app: &mut App) {
    let timer = now_ms() * 0.01;

    {
        // `sphereGroup.rotation.y -= 0.002` — `Euler.onChange` keeps the
        // quaternion in step, which is what `set_rotation` does.
        let mut group = app.sphere_group.borrow_mut();
        let rotation = group.rotation;
        group.set_rotation(rotation.x, rotation.y - 0.002, rotation.z);
    }

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
        .unwrap_or_else(|| "target/webgpu_mirror.png".to_string());
    three_rs::testing::write_png(&path, width, height, &pixels);
    println!("wrote {path} ({width}x{height})");
}
