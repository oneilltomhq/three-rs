//! Port of `three.js/examples/webgpu_postprocessing_traa.html`, calling the
//! three-rs API in the same order the page's `init()` / `animate()` do.
//!
//! Two boxes, one wireframe and one with a nearest-filtered brick map, drawn
//! into an `output` + `velocity` MRT and resolved by [`traa`]: the camera is
//! jittered by a Halton offset every frame and each new frame is blended
//! with the reprojected history of the ones before it.
//!
//! **There is no rung.** three lists `webgpu_postprocessing_traa` in
//! `test/e2e/puppeteer.js`'s exception list, so it has no reference
//! screenshot to grade against. What the port checks instead is the
//! resolve shader, against three's dump in `tests/nodes_display_wgsl.rs`,
//! and the history over several frames in `tests/traa_frames.rs`.
//!
//! The frame `main()` writes is the page's first: the history is a copy of
//! the jittered beauty, so it is that frame unblended. The anti-aliasing
//! shows over the frames after it, in the viewer.

use std::cell::RefCell;
use std::rc::Rc;

use three_rs::addons::controls::OrbitControls;
use three_rs::core::ObjectRef;
use three_rs::geometries::box_geometry;
use three_rs::nodes::display::{traa, TraaNode};
use three_rs::nodes::mrt;
use three_rs::nodes::tsl::output_property;
use three_rs::nodes::velocity::velocity;
use three_rs::textures::{MinFilter, TextureFilter};
use three_rs::{
    pass, Color, ColorSpace, Mesh, MeshBasicNodeMaterial, PassNode, PerspectiveCamera,
    RenderPipeline, Renderer, RendererParameters, Scene, TextureLoader,
};

pub const INNER_WIDTH: f64 = 800.0;
pub const INNER_HEIGHT: f64 = 500.0;
/// `renderer.setPixelRatio( window.devicePixelRatio )`.
pub const DPR: f64 = 1.0;

fn examples_dir() -> std::path::PathBuf {
    three_rs::testing::three_js_dir().join("examples")
}

pub struct App {
    pub renderer: Renderer,
    /// Shared with `scene_pass`, which renders it from `updateBefore()`.
    pub scene: Rc<RefCell<Scene>>,
    /// Shared with `scene_pass` and `traa_node`, which jitters it.
    pub camera: Rc<RefCell<PerspectiveCamera>>,
    /// `scene.children`, which `animate()` turns.
    pub children: Vec<ObjectRef>,
    /// The page's module-level `index`.
    pub index: u64,
    pub scene_pass: PassNode,
    pub traa_node: TraaNode,
    pub render_pipeline: RenderPipeline,
}

pub fn init() -> App {
    let mut renderer = Renderer::new(RendererParameters::default()).unwrap();
    renderer.set_pixel_ratio(DPR);
    renderer.set_size(INNER_WIDTH, INNER_HEIGHT);

    let camera = PerspectiveCamera::new(70.0, INNER_WIDTH / INNER_HEIGHT, 0.1, 10.0);
    camera.node.borrow_mut().position.z = 2.5;

    let scene = Scene::new();

    // `new THREE.BoxGeometry()`, shared by both meshes.
    let geometry = Rc::new(box_geometry(1.0, 1.0, 1.0, 1, 1, 1));
    let mut material1 = MeshBasicNodeMaterial::new();
    material1.color = Color::from_hex(0xffffff);
    material1.wireframe = true;

    let mesh1 = Mesh::new(geometry.clone(), material1);
    mesh1.borrow_mut().position.x = -1.0;
    scene.add(&mesh1);

    let texture = TextureLoader::new()
        .load(examples_dir().join("textures/brick_diffuse.jpg"))
        .unwrap();
    texture.set_min_filter(MinFilter::Nearest);
    texture.set_mag_filter(TextureFilter::Nearest);
    texture.set_generate_mipmaps(false);
    texture.set_color_space(ColorSpace::Srgb);

    let mut material2 = MeshBasicNodeMaterial::new();
    material2.map = Some(texture);

    let mesh2 = Mesh::new(geometry, material2);
    mesh2.borrow_mut().position.x = 1.0;
    scene.add(&mesh2);

    // postprocessing

    let scene = Rc::new(RefCell::new(scene));
    let camera = Rc::new(RefCell::new(camera));

    let mut render_pipeline = RenderPipeline::new();
    let scene_pass = pass(scene.clone(), camera.clone());
    scene_pass.set_mrt(mrt(vec![
        ("output", output_property()),
        ("velocity", velocity()),
    ]));

    // `getTextureNode( name )` for each of the three inputs: besides the
    // node, it is what adds the `velocity` attachment and links all three to
    // the pass. `toInspector( … )` returns its node unchanged.
    let _ = scene_pass.texture_node("output");
    let _ = scene_pass.texture_node("depth");
    let _ = scene_pass.texture_node("velocity");

    let traa_node = traa(
        &scene_pass.texture(),
        &scene_pass.depth_texture(),
        &scene_pass.texture_named("velocity"),
        camera.clone(),
    );
    traa_node.attach(&mut render_pipeline);

    render_pipeline.output_node = Some(traa_node.node());

    App {
        renderer,
        scene,
        camera,
        children: vec![mesh1, mesh2],
        index: 0,
        scene_pass,
        traa_node,
        render_pipeline,
    }
}

/// The page's `animate()`: the boxes turn for 100 frames, rest for 200,
/// turn for 200, and so on (`Math.round( index / 200 ) % 2 === 0`).
pub fn animate(app: &mut App) {
    app.index += 1;

    if ((app.index as f64 / 200.0).round() as u64).is_multiple_of(2) {
        for child in &app.children {
            let mut child = child.borrow_mut();
            let rotation = child.rotation;
            child.set_rotation(rotation.x + 0.005, rotation.y + 0.01, rotation.z);
        }
    }

    app.render_pipeline.render(&mut app.renderer);
}

/// The page's `onWindowResize()`. The history restarts at the new size on
/// the next frame.
pub fn resize(app: &mut App, width: f64, height: f64) {
    let mut camera = app.camera.borrow_mut();
    camera.aspect = width / height;
    camera.update_projection_matrix();
    app.renderer.set_size(width, height);
}

/// The page creates no controls.
pub fn controls(_app: &mut App) -> Option<&mut OrbitControls> {
    None
}

/// The controls and the camera at once, for a host delivering pointer events.
pub fn controls_and_camera(
    _app: &mut App,
) -> Option<(&mut OrbitControls, std::cell::RefMut<'_, PerspectiveCamera>)> {
    None
}

fn main() {
    // Pin both clocks to zero, as three.js' `test/e2e/deterministic-injection.js`
    // does to the page, so the frame this writes does not depend on how long
    // `init()` took.
    three_rs::testing::pin_time(Some(0.0));
    let mut app = init();
    println!("adapter: {:?}", app.renderer.adapter_info());
    animate(&mut app);

    let (width, height, pixels) = app.renderer.read_canvas_pixels().unwrap();
    let path = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "target/webgpu_postprocessing_traa.png".to_string());
    three_rs::testing::write_png(&path, width, height, &pixels);
    println!("wrote {path} ({width}x{height})");
}
