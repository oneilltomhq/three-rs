//! Port of `three.js/examples/webgpu_postprocessing_smaa.html`, calling the
//! three-rs API in the same order the page's `init()` / `animate()` do.
//!
//! Two 120-unit boxes, one white wireframe and one brick-textured, rotating;
//! the scene pass is anti-aliased by [`smaa`] before the output transform.
//!
//! The brick texture loads asynchronously on the page; three's screenshot is
//! taken once it is in, so the port loads it synchronously. The GUI is not
//! ported; its defaults are (`enabled` and `autoRotate` both true).

use std::cell::RefCell;
use std::rc::Rc;

use three_rs::addons::controls::OrbitControls;
use three_rs::geometries::box_geometry;
use three_rs::nodes::display::{smaa, SmaaNode};
use three_rs::{
    pass, Color, ColorSpace, Mesh, MeshBasicNodeMaterial, Node, PassNode, PerspectiveCamera,
    RenderPipeline, Renderer, RendererParameters, Scene, TextureLoader,
};

pub const INNER_WIDTH: f64 = 800.0;
pub const INNER_HEIGHT: f64 = 500.0;
/// `renderer.setPixelRatio( window.devicePixelRatio )`.
pub const DPR: f64 = 1.0;

/// The page's `params`, the GUI's starting values.
pub struct Params {
    pub enabled: bool,
    pub auto_rotate: bool,
}

fn examples_dir() -> std::path::PathBuf {
    three_rs::testing::three_js_dir().join("examples")
}

pub struct App {
    pub renderer: Renderer,
    pub scene: Rc<RefCell<Scene>>,
    pub camera: Rc<RefCell<PerspectiveCamera>>,
    /// `scene.children`, the two boxes `animate()` turns.
    pub children: Vec<Node>,
    pub params: Params,
    pub scene_pass: PassNode,
    pub smaa_pass: SmaaNode,
    pub render_pipeline: RenderPipeline,
}

pub fn init() -> App {
    let mut renderer = Renderer::new(RendererParameters::default()).unwrap();
    renderer.set_pixel_ratio(DPR);
    renderer.set_size(INNER_WIDTH, INNER_HEIGHT);

    //

    let camera = PerspectiveCamera::new(70.0, INNER_WIDTH / INNER_HEIGHT, 1.0, 1000.0);
    camera.node.borrow_mut().position.z = 300.0;

    let scene = Scene::new();

    let geometry = Rc::new(box_geometry(120.0, 120.0, 120.0, 1, 1, 1));
    let mut material1 = MeshBasicNodeMaterial::new();
    material1.color = Color::from_hex(0xffffff);
    material1.wireframe = true;

    let mesh1 = Mesh::new(geometry.clone(), material1);
    mesh1.borrow_mut().position.x = -100.0;
    scene.add(&mesh1);

    let texture = TextureLoader::new()
        .load(examples_dir().join("textures/brick_diffuse.jpg"))
        .expect("brick_diffuse.jpg");
    texture.set_color_space(ColorSpace::Srgb);

    let mut material2 = MeshBasicNodeMaterial::new();
    material2.map = Some(texture);

    let mesh2 = Mesh::new(geometry, material2);
    mesh2.borrow_mut().position.x = 100.0;
    scene.add(&mesh2);

    // post processing

    let mut render_pipeline = RenderPipeline::new();

    let scene = Rc::new(RefCell::new(scene));
    let camera = Rc::new(RefCell::new(camera));

    // `toInspector( 'Color' )` returns its node unchanged.
    let scene_pass = pass(scene.clone(), camera.clone());
    let smaa_pass = smaa(&scene_pass.texture());

    render_pipeline.output_node = Some(smaa_pass.node());

    App {
        renderer,
        scene,
        camera,
        children: vec![mesh1, mesh2],
        params: Params {
            enabled: true,
            auto_rotate: true,
        },
        scene_pass,
        smaa_pass,
        render_pipeline,
    }
}

/// The page's `animate()`.
pub fn animate(app: &mut App) {
    if app.params.auto_rotate {
        for child in &app.children {
            let mut child = child.borrow_mut();
            let rotation = child.rotation;
            child.set_rotation(rotation.x + 0.005, rotation.y + 0.01, rotation.z);
        }
    }

    app.render_pipeline.render(&mut app.renderer);
}

/// The page's `onWindowResize()`.
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
    // does to the page.
    three_rs::testing::pin_time(Some(0.0));
    let mut app = init();
    println!("adapter: {:?}", app.renderer.adapter_info());
    animate(&mut app);

    let (width, height, pixels) = app.renderer.read_canvas_pixels().unwrap();
    let path = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "target/webgpu_postprocessing_smaa.png".to_string());
    three_rs::testing::write_png(&path, width, height, &pixels);
    println!("wrote {path} ({width}x{height})");
}
