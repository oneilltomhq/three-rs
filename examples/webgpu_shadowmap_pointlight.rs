//! Port of `three.js/examples/webgpu_shadowmap_pointlight.html`, calling the
//! three-rs API in the same order the page's `init()` / `animate()` do.
//!
//! Under the e2e harness the viewport is `400 * 2` x `250 * 2` with no
//! `deviceScaleFactor`, so `window.innerWidth` / `innerHeight` /
//! `devicePixelRatio` are 800, 500 and 1.
//!
//! `performance.now()` is pinned to 0, so `animate()` puts the first light at
//! `( sin 0 · 9, sin 0 · 9 + 6, sin 0 · 9 ) = ( 0, 6, 0 )` with a zero
//! rotation, and the second at the same formulas with `time = 10000`.
//!
//! Each light carries two children: a small unlit bulb whose colour is
//! multiplied by the light's intensity (so it clips to white in the frame),
//! and a double-sided Phong sphere cut into bands by a 2 × 2 `CanvasTexture`
//! alpha map with `alphaTest = 0.5`. The bands cast banded shadows through
//! the default `PCFShadowMap` point-shadow filter onto the room box, which is
//! a `BackSide` Phong box that only receives. `renderer.inspector` does not
//! touch the frame and the `resize` listener never fires.

use std::rc::Rc;

use three_rs::addons::controls::OrbitControls;
use three_rs::core::Node;
use three_rs::materials::Side;
use three_rs::textures::{Texture, TextureFilter, Wrapping};
use three_rs::utils::now_ms;
use three_rs::{
    box_geometry, sphere_geometry, AmbientLight, Color, Mesh, MeshBasicNodeMaterial,
    MeshPhongNodeMaterial, PerspectiveCamera, PointLight, Renderer, RendererParameters, Scene,
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
    pub point_light: Node,
    pub point_light2: Node,
}

/// `generateTexture()`: a 2 × 2 canvas, transparent except for
/// `fillRect( 0, 1, 2, 1 )` in white — the bottom row. `CanvasTexture` keeps
/// `Texture`'s defaults (`flipY = true`, mipmaps, `NoColorSpace`), and the
/// canvas' transparent texels read back as `( 0, 0, 0, 0 )`.
fn generate_texture() -> Texture {
    let mut pixels = vec![0u8; 2 * 2 * 4];
    pixels[8..].fill(255);
    Texture::new(2, 2, Some(pixels))
}

/// The page's `createLight( color )`.
fn create_light(color: u32) -> Node {
    let intensity = 200.0;

    let light = PointLight::new(Color::from_hex(color), intensity, 20.0);
    {
        let mut object = light.borrow_mut();
        object.cast_shadow = true;
        let shadow = object.light_mut().unwrap().shadow.as_mut().unwrap();
        // "reduces self-shadowing on double-sided objects"
        shadow.bias = -0.005;
        shadow.map_size.x = 128.0;
        shadow.map_size.y = 128.0;
        shadow.radius = 10.0;
    }

    let geometry = Rc::new(sphere_geometry(0.3, 12, 6));
    let mut material = MeshBasicNodeMaterial::new();
    material.color = Color::from_hex(color);
    material.color.multiply_scalar(intensity);
    let sphere = Mesh::new(geometry, material);
    light.add(&sphere);

    let texture = generate_texture();
    texture.set_mag_filter(TextureFilter::Nearest);
    texture.set_wrapping(Wrapping::Repeat, Wrapping::Repeat);
    texture.set_repeat(1.0, 4.5);

    let geometry = Rc::new(sphere_geometry(2.0, 32, 8));
    let mut material = MeshPhongNodeMaterial::phong(Color::from_hex(0xffffff));
    material.side = Side::Double;
    material.alpha_map = Some(texture);
    material.alpha_test = 0.5;

    let sphere = Mesh::new(geometry, material);
    {
        let mut object = sphere.borrow_mut();
        object.cast_shadow = true;
        object.receive_shadow = true;
    }
    light.add(&sphere);

    light
}

pub fn init() -> App {
    let mut camera = PerspectiveCamera::new(45.0, INNER_WIDTH / INNER_HEIGHT, 1.0, 1000.0);
    camera.node.borrow_mut().position.set(0.0, 10.0, 40.0);

    let scene = Scene::new();
    scene.add(&AmbientLight::new(Color::from_hex(0x111122), 3.0));

    // lights

    let point_light = create_light(0x0088ff);
    scene.add(&point_light);

    let point_light2 = create_light(0xff8888);
    scene.add(&point_light2);

    //

    let geometry = Rc::new(box_geometry(30.0, 30.0, 30.0, 1, 1, 1));

    let mut material = MeshPhongNodeMaterial::phong(Color::from_hex(0xa0adaf));
    material.shininess = 10.0;
    material.specular = Color::from_hex(0x111111);
    material.side = Side::Back;

    let mesh = Mesh::new(geometry, material);
    {
        let mut object = mesh.borrow_mut();
        object.position.y = 10.0;
        object.receive_shadow = true;
    }
    scene.add(&mesh);

    //

    let mut renderer = Renderer::new(RendererParameters { antialias: true }).unwrap();
    renderer.set_pixel_ratio(DPR);
    renderer.set_size(INNER_WIDTH, INNER_HEIGHT);
    renderer.shadow_map_enabled = true;

    let mut controls = OrbitControls::new(&mut camera);
    // The canvas the example renders at, standing in for the element's
    // `clientWidth` / `clientHeight`.
    controls.set_element_size(INNER_WIDTH, INNER_HEIGHT);
    controls.target.set(0.0, 10.0, 0.0);
    let _ = controls.update(&mut camera, None);

    App {
        renderer,
        scene,
        camera,
        controls,
        point_light,
        point_light2,
    }
}

/// `pointLight.position` / `.rotation` for one `time`, in seconds.
fn place(light: &Node, time: f64) {
    let mut object = light.borrow_mut();
    object.position.x = (time * 0.6).sin() * 9.0;
    object.position.y = (time * 0.7).sin() * 9.0 + 6.0;
    object.position.z = (time * 0.8).sin() * 9.0;
    let r = object.rotation;
    object.set_rotation(time, r.y, time);
}

/// The page's `animate()` with `performance.now()` pinned to 0.
pub fn animate(app: &mut App) {
    let time = now_ms() * 0.001;
    place(&app.point_light, time);
    place(&app.point_light2, time + 10000.0);

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
        .unwrap_or_else(|| "target/webgpu_shadowmap_pointlight.png".to_string());
    three_rs::testing::write_png(&path, width, height, &pixels);
    println!("wrote {path} ({width}x{height})");
}
