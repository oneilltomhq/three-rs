//! Port of `three.js/examples/webgpu_equirectangular.html`, calling the
//! three-rs API in the same order the page's `init()` does.
//!
//! Under the e2e harness the viewport is `400 * 2` x `250 * 2` with no
//! `deviceScaleFactor`, so `window.innerWidth` / `innerHeight` /
//! `devicePixelRatio` are 800, 500 and 1. The page draws from `Math.random()`
//! not at all.
//!
//! # What this page is
//!
//! A 4096×2048 equirectangular JPEG as `scene.backgroundNode = texture(
//! equirectTexture, equirectUV(), 0 )`: no mesh, no light, only the skybox
//! sphere of `Background.update()`'s node branch, which samples the map at
//! mip level 0 along `positionWorldDirection` — `equirectUV()`'s default
//! direction — and multiplies by `backgroundIntensity`, the page's one GUI
//! knob, graded at its default 1.
//!
//! `controls.autoRotate` is on and `animate()` calls `controls.update()`
//! before every render. With `deltaTime` null the auto-rotation angle is
//! `2π/3600 * autoRotateSpeed`, so the graded frame's camera has turned
//! 0.1° about the target from `( 1, 0, 0 )`. The constructor's own
//! `update()` runs before `autoRotate` is set, so it turns nothing. The
//! port runs the real `OrbitControls` for both.
//!
//! `renderer.inspector = new Inspector()` and its parameters panel draw
//! nothing into the canvas and are not ported.

use three_rs::addons::controls::OrbitControls;
use three_rs::nodes::tsl::{equirect_uv, float, position_world_direction, texture_level};
use three_rs::objects::Background;
use three_rs::{ColorSpace, PerspectiveCamera, Renderer, RendererParameters, Scene, TextureLoader};

pub const INNER_WIDTH: f64 = 800.0;
pub const INNER_HEIGHT: f64 = 500.0;
/// `renderer.setPixelRatio( window.devicePixelRatio )`.
pub const DPR: f64 = 1.0;

fn examples_dir() -> std::path::PathBuf {
    three_rs::testing::three_js_dir().join("examples")
}

pub struct App {
    pub renderer: Renderer,
    pub scene: Scene,
    pub camera: PerspectiveCamera,
    /// The page's `controls`.
    pub controls: OrbitControls,
}

pub fn init() -> App {
    // `new THREE.PerspectiveCamera( 45, window.innerWidth /
    // window.innerHeight, 0.25, 20 )`, then `camera.position.set( 1, 0, 0 )`.
    let mut camera = PerspectiveCamera::new(45.0, INNER_WIDTH / INNER_HEIGHT, 0.25, 20.0);
    camera.node.borrow_mut().position.set(1.0, 0.0, 0.0);

    let equirect_texture = TextureLoader::new()
        .load(examples_dir().join("textures/2294472375_24a3b8ef46_o.jpg"))
        .unwrap();
    equirect_texture.set_color_space(ColorSpace::SRGB);

    let mut scene = Scene::new();
    // `scene.backgroundNode = texture( equirectTexture, equirectUV(), 0 )`.
    // `equirectUV()` with no argument reads `positionWorldDirection`.
    scene.background = Some(Background::Node(texture_level(
        &equirect_texture,
        equirect_uv(position_world_direction()),
        float(0.0),
    )));

    let mut parameters = RendererParameters::default();
    parameters.antialias = true;
    let mut renderer = Renderer::new(parameters).unwrap();
    renderer.set_pixel_ratio(DPR);
    renderer.set_size(INNER_WIDTH, INNER_HEIGHT);

    let mut controls = OrbitControls::new(&mut camera);
    // The canvas the example renders at, standing in for the element's
    // `clientWidth` / `clientHeight`.
    controls.set_element_size(INNER_WIDTH, INNER_HEIGHT);
    controls.auto_rotate = true;
    controls.rotate_speed = -0.125; // negative, to track mouse pointer
    controls.auto_rotate_speed = 1.0;

    App {
        renderer,
        scene,
        camera,
        controls,
    }
}

/// The page's `animate()`.
pub fn animate(app: &mut App) {
    app.controls.update(&mut app.camera, None);

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
    three_rs::testing::pin_time(Some(0.0));
    let mut app = init();
    println!("adapter: {:?}", app.renderer.adapter_info());
    animate(&mut app);

    let (width, height, pixels) = app.renderer.read_canvas_pixels().unwrap();
    let path = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "target/webgpu_equirectangular.png".to_string());
    three_rs::testing::write_png(&path, width, height, &pixels);
    println!("wrote {path} ({width}x{height})");
}
