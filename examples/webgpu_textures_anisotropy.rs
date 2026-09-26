//! Port of `three.js/examples/webgpu_textures_anisotropy.html`, calling the
//! three-rs API in the same order the page's `init()` / `render()` do.
//!
//! Two scenes, one camera, one frame: the same crate-textured floor twice, the
//! left one sampled with `texture.anisotropy = renderer.getMaxAnisotropy()`
//! (16) and the right one with 1, each drawn into its own half of the canvas
//! through the scissor, with `autoClear` off and one manual `clear()` first.
//!
//! Under the e2e harness the viewport is `400 * 2` x `250 * 2` with no
//! `deviceScaleFactor`, so `window.innerWidth` / `innerHeight` /
//! `devicePixelRatio` are 800, 500 and 1. No `mousemove` ever fires, so
//! `mouseX` and `mouseY` stay 0: the graded frame's camera has eased
//! `x += ( 0 - 0 ) * .05` and `y = clamp( 0 + ( 200 - 0 ) * .05, 50, 1000 )`,
//! which is 50. `Math.random` is never called.
//!
//! The canvas is not opaque. The renderer's `alpha` is on, so its clear
//! colour is `( 0, 0, 0, 0 )`, and the floor, 100 000 units across, is cut
//! by the camera's far plane at 25 000 below the horizon. Everything above
//! it, and the two-pixel gap `setScissor( 0, 0, SCREEN_WIDTH / 2 - 2, … )`
//! leaves between the halves, is the page's `body { background-color:
//! #f1f1f1 }` seen through the canvas: [`PAGE_BACKGROUND`], which the e2e
//! harness composites the frame over the way Chrome does.

use std::f64::consts::PI;
use std::rc::Rc;

use three_rs::addons::controls::OrbitControls;
use three_rs::geometries::plane_geometry;
use three_rs::math::math_utils::clamp;
use three_rs::math::Vector3;
use three_rs::textures::Wrapping;
use three_rs::{
    AmbientLight, Color, ColorSpace, DirectionalLight, Fog, Mesh, MeshPhongNodeMaterial,
    PerspectiveCamera, Renderer, RendererParameters, Scene, TextureLoader,
};

pub const INNER_WIDTH: f64 = 800.0;
pub const INNER_HEIGHT: f64 = 500.0;
/// `renderer.setPixelRatio( window.devicePixelRatio )`.
pub const DPR: f64 = 1.0;

/// The page's own `<style>`: `body { background-color: #f1f1f1; }`, which
/// shows wherever the canvas is transparent.
pub const PAGE_BACKGROUND: u32 = 0xf1f1f1;

pub struct App {
    pub renderer: Renderer,
    pub scene1: Scene,
    pub scene2: Scene,
    pub camera: PerspectiveCamera,
    /// The page's `mouseX` / `mouseY`, written by `onDocumentMouseMove` only.
    pub mouse_x: f64,
    pub mouse_y: f64,
    /// `window.innerWidth` / `innerHeight`, which `render()` reads each frame
    /// for the two scissor boxes.
    pub inner_width: f64,
    pub inner_height: f64,
}

pub fn init() -> App {
    // RENDERER

    let mut renderer = Renderer::new(RendererParameters { antialias: true }).unwrap();
    renderer.set_pixel_ratio(DPR);
    renderer.set_size(INNER_WIDTH, INNER_HEIGHT);
    renderer.auto_clear = false;

    //

    let camera = PerspectiveCamera::new(35.0, INNER_WIDTH / INNER_HEIGHT, 1.0, 25000.0);
    camera.node.borrow_mut().position.z = 1500.0;

    let mut scene1 = Scene::new();
    scene1.fog = Some(Fog::new(Color::from_hex(0xf2f7ff), 1.0, 25000.0).into());

    let mut scene2 = Scene::new();
    scene2.fog = Some(Fog::new(Color::from_hex(0xf2f7ff), 1.0, 25000.0).into());

    scene1.add(&AmbientLight::new(Color::from_hex(0xeef0ff), 3.0));
    scene2.add(&AmbientLight::new(Color::from_hex(0xeef0ff), 3.0));

    let light1 = DirectionalLight::new(Color::from_hex(0xffffff), 6.0);
    light1.borrow_mut().position.set(1.0, 1.0, 1.0);
    scene1.add(&light1);

    let light2 = DirectionalLight::new(Color::from_hex(0xffffff), 6.0);
    light2.borrow_mut().position.set(1.0, 1.0, 1.0);
    scene2.add(&light2);

    // GROUND

    let texture_loader = TextureLoader::new();
    let crate_gif = three_rs::testing::three_js_dir().join("examples/textures/crate.gif");

    let max_anisotropy = renderer.get_max_anisotropy();

    let texture1 = texture_loader.load(&crate_gif).unwrap();
    let mut material1 = MeshPhongNodeMaterial::phong(Color::from_hex(0xffffff));
    material1.map = Some(texture1.clone());

    texture1.set_color_space(ColorSpace::SRGB);
    texture1.set_anisotropy(max_anisotropy);
    texture1.set_wrapping(Wrapping::Repeat, Wrapping::Repeat);
    texture1.set_repeat(512.0, 512.0);

    let texture2 = texture_loader.load(&crate_gif).unwrap();
    let mut material2 = MeshPhongNodeMaterial::phong(Color::from_hex(0xffffff));
    material2.map = Some(texture2.clone());

    texture2.set_color_space(ColorSpace::SRGB);
    texture2.set_anisotropy(1);
    texture2.set_wrapping(Wrapping::Repeat, Wrapping::Repeat);
    texture2.set_repeat(512.0, 512.0);

    // The `val_left` / `val_right` labels are DOM, removed by the grader's
    // `clean-page.js`; nothing of them reaches the canvas.

    //

    let geometry = Rc::new(plane_geometry(100.0, 100.0, 1, 1));

    let mesh1 = Mesh::new(geometry.clone(), material1);
    {
        let mut object = mesh1.borrow_mut();
        object.set_rotation(-PI / 2.0, 0.0, 0.0);
        object.scale.set(1000.0, 1000.0, 1000.0);
    }

    let mesh2 = Mesh::new(geometry, material2);
    {
        let mut object = mesh2.borrow_mut();
        object.set_rotation(-PI / 2.0, 0.0, 0.0);
        object.scale.set(1000.0, 1000.0, 1000.0);
    }

    scene1.add(&mesh1);
    scene2.add(&mesh2);

    App {
        renderer,
        scene1,
        scene2,
        camera,
        mouse_x: 0.0,
        mouse_y: 0.0,
        inner_width: INNER_WIDTH,
        inner_height: INNER_HEIGHT,
    }
}

/// The page's `render()`, run once by the harness's single RAF.
pub fn animate(app: &mut App) {
    let screen_width = app.inner_width;
    let screen_height = app.inner_height;

    {
        let mut camera = app.camera.node.borrow_mut();
        let position = &mut camera.position;
        position.x += (app.mouse_x - position.x) * 0.05;
        position.y = clamp(
            position.y + (-(app.mouse_y - 200.0) - position.y) * 0.05,
            50.0,
            1000.0,
        );
    }

    // `camera.lookAt( scene1.position )` — the scene sits at the origin.
    app.camera.look_at(&Vector3::ZERO);
    app.renderer.clear(true, true);

    app.renderer.set_scissor_test(true);

    app.renderer
        .set_scissor(0.0, 0.0, screen_width / 2.0 - 2.0, screen_height);
    app.renderer.render(&mut app.scene1, &mut app.camera);

    app.renderer.set_scissor_test(true);

    app.renderer.set_scissor(
        screen_width / 2.0,
        0.0,
        screen_width / 2.0 - 2.0,
        screen_height,
    );
    app.renderer.render(&mut app.scene2, &mut app.camera);

    app.renderer.set_scissor_test(false);
}

/// The page's `onDocumentMouseMove()`, for a host that has a pointer.
pub fn mouse_move(app: &mut App, client_x: f64, client_y: f64) {
    let window_half_x = app.inner_width / 2.0;
    let window_half_y = app.inner_height / 2.0;

    app.mouse_x = client_x - window_half_x;
    app.mouse_y = client_y - window_half_y;
}

/// The page's `onWindowResize()`.
///
/// The rung harness never calls this — the graded frame is always
/// 800 x 500 — but the viewer and the browser shell do. `render()` reads
/// `window.innerWidth` / `innerHeight` for its scissor boxes every frame, so
/// the new size is kept on [`App`] too.
pub fn resize(app: &mut App, width: f64, height: f64) {
    app.inner_width = width;
    app.inner_height = height;

    app.camera.aspect = width / height;
    app.camera.update_projection_matrix();

    app.renderer.set_size(width, height);
}

/// The page creates no controls.
pub fn controls(_app: &mut App) -> Option<&mut OrbitControls> {
    None
}

/// See [`controls`].
pub fn controls_and_camera(_app: &mut App) -> Option<(&mut OrbitControls, &mut PerspectiveCamera)> {
    None
}

fn main() {
    // Pin both clocks to zero, as three.js' `test/e2e/deterministic-injection.js`
    // does to the page, so that the frame this writes is the frame the rung
    // grades no matter how long `init()` took.
    three_rs::testing::pin_time(Some(0.0));
    let mut app = init();
    println!("adapter: {:?}", app.renderer.adapter_info());
    animate(&mut app);

    let (width, height, mut pixels) = app.renderer.read_canvas_pixels().unwrap();
    three_rs::testing::composite_over_page(&mut pixels, PAGE_BACKGROUND);
    let path = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "target/webgpu_textures_anisotropy.png".to_string());
    three_rs::testing::write_png(&path, width, height, &pixels);
    println!("wrote {path} ({width}x{height})");
}
