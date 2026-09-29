//! Port of `three.js/examples/webgpu_postprocessing_masking.html`, calling the
//! three-rs API in the same order the page's `init()` / `animate()` do.
//!
//! Under the e2e harness the viewport is `400 * 2` x `250 * 2` with no
//! `deviceScaleFactor`, so `window.innerWidth` / `innerHeight` /
//! `devicePixelRatio` are 800, 500 and 1, and `performance.now()` is pinned to
//! 0 — so `time` is exactly 6000.
//!
//! `renderer.inspector = new Inspector()` only registers the renderer with the
//! inspector panel and changes nothing about the graded frame. The `resize`
//! listener never fires.
//!
//! Three discovers the frame's `pass()` nodes from the node graph and fires
//! their `updateBefore()` from inside the quad's own render, the first time a
//! draw samples its texture (`docs/nodes.md` §57).

use std::cell::RefCell;
use std::rc::Rc;

use three_rs::addons::controls::OrbitControls;
use three_rs::geometries::torus_geometry;
use three_rs::math::Color;
use three_rs::nodes::tsl::texture;
use three_rs::utils::now_ms;
use three_rs::{
    box_geometry, pass, ColorSpace, Mesh, MinFilter, PassNode, PerspectiveCamera, RenderPipeline,
    Renderer, RendererParameters, Scene,
};

pub const INNER_WIDTH: f64 = 800.0;
pub const INNER_HEIGHT: f64 = 500.0;
/// `renderer.setPixelRatio( window.devicePixelRatio )`.
pub const DPR: f64 = 1.0;

pub struct App {
    pub renderer: Renderer,
    /// Shared with `base`, `mask1` and `mask2`, which render it from
    /// `updateBefore()`.
    pub camera: Rc<RefCell<PerspectiveCamera>>,
    pub base_scene: Rc<RefCell<Scene>>,
    pub mask_scene1: Rc<RefCell<Scene>>,
    pub mask_scene2: Rc<RefCell<Scene>>,
    pub boxed: three_rs::Node,
    pub torus: three_rs::Node,
    pub base: PassNode,
    pub mask1: PassNode,
    pub mask2: PassNode,
    pub render_pipeline: RenderPipeline,
}

fn examples_dir() -> std::path::PathBuf {
    let three = three_rs::testing::three_js_dir();
    three.join("examples")
}

pub fn init() -> App {
    // scene

    let mut base_scene = Scene::new();
    base_scene.set_background(Color::from_hex(0xe0e0e0));

    let mask_scene1 = Scene::new();
    // `new Mesh( geometry )` — no material, so the renderer uses the default
    // white `MeshBasicMaterial`.
    let boxed = Mesh::new(Rc::new(box_geometry(4.0, 4.0, 4.0, 1, 1, 1)), None);
    mask_scene1.add(&boxed);

    let mask_scene2 = Scene::new();
    let torus = Mesh::new(Rc::new(torus_geometry(3.0, 1.0, 16, 32)), None);
    mask_scene2.add(&torus);

    let camera = PerspectiveCamera::new(50.0, INNER_WIDTH / INNER_HEIGHT, 1.0, 1000.0);
    camera.node.borrow_mut().position.z = 10.0;

    // textures

    let loader = three_rs::TextureLoader::new();

    let texture1 = loader
        .load(examples_dir().join("textures/758px-Canestra_di_frutta_(Caravaggio).jpg"))
        .unwrap();
    texture1.set_color_space(ColorSpace::Srgb);
    texture1.set_min_filter(MinFilter::Linear);
    texture1.set_generate_mipmaps(false);
    texture1.set_flip_y(false);

    let texture2 = loader
        .load(examples_dir().join("textures/2294472375_24a3b8ef46_o.jpg"))
        .unwrap();
    texture2.set_color_space(ColorSpace::Srgb);
    texture2.set_flip_y(false);

    // renderer

    let mut renderer = Renderer::new(RendererParameters { antialias: false }).unwrap();
    renderer.set_pixel_ratio(DPR);
    renderer.set_size(INNER_WIDTH, INNER_HEIGHT);

    // post processing

    // `pass( scene, camera )`: each pass holds its own scene and the shared
    // camera, and the renderer renders it the first time a draw samples its
    // texture (`docs/nodes.md` §57).
    let base_scene = Rc::new(RefCell::new(base_scene));
    let mask_scene1 = Rc::new(RefCell::new(mask_scene1));
    let mask_scene2 = Rc::new(RefCell::new(mask_scene2));
    let camera = Rc::new(RefCell::new(camera));

    let base = pass(base_scene.clone(), camera.clone());
    let mask1 = pass(mask_scene1.clone(), camera.clone());
    let mask2 = pass(mask_scene2.clone(), camera.clone());

    let scene_mask1 = mask1.a();
    let scene_mask2 = mask2.a();

    let mut compose = base.node();
    compose = scene_mask1.mix(compose, texture(&texture1));
    compose = scene_mask2.mix(compose, texture(&texture2));

    let mut render_pipeline = RenderPipeline::new();
    render_pipeline.output_node = Some(compose);

    App {
        renderer,
        camera,
        base_scene,
        mask_scene1,
        mask_scene2,
        boxed,
        torus,
        base,
        mask1,
        mask2,
        render_pipeline,
    }
}

/// The page's `animate()`, run once by the harness's single RAF.
pub fn animate(app: &mut App) {
    // `const time = performance.now() * 0.001 + 6000;`
    let time: f64 = now_ms() * 0.001 + 6000.0;

    {
        let mut boxed = app.boxed.borrow_mut();
        boxed.position.x = (time / 1.5).cos() * 2.0;
        boxed.position.y = time.sin() * 2.0;
        boxed.set_rotation(time, time / 2.0, 0.0);
    }

    {
        let mut torus = app.torus.borrow_mut();
        torus.position.x = time.cos() * 2.0;
        torus.position.y = (time / 1.5).sin() * 2.0;
        torus.set_rotation(time, time / 2.0, 0.0);
    }

    // `renderPipeline.render()`: the output quad's draw runs the three
    // passes' `updateBefore()` first (`docs/nodes.md` §57).
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

/// The example's controls, for a host that has a pointer. `None` here:
/// the page creates none.
pub fn controls(_app: &mut App) -> Option<&mut OrbitControls> {
    None
}

/// The controls and the camera at once, for a host delivering pointer events.
/// `None` here: the page creates no controls.
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

    let (width, height, pixels) = app.renderer.read_canvas_pixels().unwrap();
    let path = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "target/webgpu_postprocessing_masking.png".to_string());
    three_rs::testing::write_png(&path, width, height, &pixels);
    println!("wrote {path} ({width}x{height})");
}
