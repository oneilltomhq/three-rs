//! Port of `three.js/examples/webgpu_postprocessing.html`, calling the three-rs
//! API in the same order the page's `init()` / `animate()` do.
//!
//! Under the e2e harness the viewport is `400 * 2` x `250 * 2` with no
//! `deviceScaleFactor`, so `window.innerWidth` / `innerHeight` /
//! `devicePixelRatio` are 800, 500 and 1. The harness's single RAF runs
//! `animate()` once, so the graded frame has `object.rotation` at
//! `( 0.005, 0.01, 0 )`.
//!
//! `Math.random` is the harness's seeded sequence
//! ([`three_rs::testing::DeterministicRandom`]). `renderer.inspector = new
//! Inspector()` runs at the top of `init()`, before the sphere loop, so its
//! five draws come first; the loop then draws eight per mesh, in the order
//! the JS evaluates them: the three position components, the radius, the
//! three rotation angles and the scale.
//!
//! The page's output is `rgbShift( dotScreen( scenePassColor, 1.57, 0.3 ),
//! 0.001 )`. `RGBShiftNode` calls `convertToTexture()` on its input, so the
//! dot screen is drawn into an `RTTNode` first and the shift reads three taps
//! of that; the `RenderPipeline`'s own quad then does the sRGB encode.

use std::cell::RefCell;
use std::rc::Rc;

use three_rs::addons::controls::OrbitControls;
use three_rs::geometries::sphere_geometry;
use three_rs::nodes::display::{convert_to_texture, dot_screen, rgb_shift, RttNode};
use three_rs::nodes::tsl::float;
use three_rs::testing::DeterministicRandom;
use three_rs::{
    pass, AmbientLight, Color, DirectionalLight, Fog, Mesh, MeshPhongNodeMaterial, Object3D,
    PassNode, PerspectiveCamera, RenderPipeline, Renderer, RendererParameters, Scene, Vector3,
};

pub const INNER_WIDTH: f64 = 800.0;
pub const INNER_HEIGHT: f64 = 500.0;
/// `renderer.setPixelRatio( window.devicePixelRatio )`.
pub const DPR: f64 = 1.0;

/// `Math.random()` draws `new Inspector()` makes — five, from the `List`
/// constructors of the inspector's tabs (see `webgpu_clearcoat`).
pub const INSPECTOR_RANDOM_DRAWS: usize = 5;

pub struct App {
    pub renderer: Renderer,
    /// Shared with `scene_pass`, which renders it from `updateBefore()`.
    pub scene: Rc<RefCell<Scene>>,
    /// Shared with `scene_pass`.
    pub camera: Rc<RefCell<PerspectiveCamera>>,
    /// The page's module-level `object`, parent of the hundred spheres.
    pub object: three_rs::Node,
    pub scene_pass: PassNode,
    /// The `RTTNode` `RGBShiftNode.setup()` makes of the dot screen.
    pub rgb_shift_input: RttNode,
    pub render_pipeline: RenderPipeline,
}

pub fn init() -> App {
    let mut renderer = Renderer::new(RendererParameters { antialias: false }).unwrap();
    renderer.set_pixel_ratio(DPR);
    renderer.set_size(INNER_WIDTH, INNER_HEIGHT);

    // `renderer.inspector = new Inspector()`.
    let mut random = DeterministicRandom::new();
    random.skip(INSPECTOR_RANDOM_DRAWS);

    //

    let camera = PerspectiveCamera::new(70.0, INNER_WIDTH / INNER_HEIGHT, 1.0, 1000.0);
    camera.node.borrow_mut().position.z = 400.0;

    let mut scene = Scene::new();
    scene.set_background(Color::from_hex(0x000000));
    scene.fog = Some(Fog::new(Color::from_hex(0x000000), 1.0, 1000.0).into());

    let object = Object3D::new_node();
    scene.add(&object);

    let geometry = Rc::new(sphere_geometry(1.0, 4, 4));
    let mut material = MeshPhongNodeMaterial::phong(Color::from_hex(0xffffff));
    material.flat_shading = true;

    for _ in 0..100 {
        let mesh = Mesh::new(geometry.clone(), material.clone());
        {
            let mut mesh = mesh.borrow_mut();
            let x = random.next() - 0.5;
            let y = random.next() - 0.5;
            let z = random.next() - 0.5;
            mesh.position = Vector3::new(x, y, z);
            mesh.position.normalize();
            mesh.position.multiply_scalar(random.next() * 400.0);
            let rx = random.next() * 2.0;
            let ry = random.next() * 2.0;
            let rz = random.next() * 2.0;
            mesh.set_rotation(rx, ry, rz);
            let s = random.next() * 50.0;
            mesh.scale.set(s, s, s);
        }
        object.add(&mesh);
    }

    scene.add(&AmbientLight::new(Color::from_hex(0xcccccc), 1.0));

    let light = DirectionalLight::new(Color::from_hex(0xffffff), 3.0);
    light.borrow_mut().position.set(1.0, 1.0, 1.0);
    scene.add(&light);

    // postprocessing

    let mut render_pipeline = RenderPipeline::new();

    // `pass( scene, camera )` — the pass holds both, and the renderer renders
    // it the first time a draw samples its texture (`docs/nodes.md` §57).
    let scene = Rc::new(RefCell::new(scene));
    let camera = Rc::new(RefCell::new(camera));
    let scene_pass = pass(scene.clone(), camera.clone());
    // `.toInspector( 'Scene Color' )` only registers the node with the
    // inspector panel; the graph is the texture node itself.
    let scene_pass_color = scene_pass.texture_node("output");

    let dot_screen_pass = dot_screen(scene_pass_color, float(1.57), float(0.3));
    let rgb_shift_input = convert_to_texture(dot_screen_pass);
    let rgb_shift_pass = rgb_shift(&rgb_shift_input.texture(), float(0.001), float(0.0));

    render_pipeline.output_node = Some(rgb_shift_pass);

    App {
        renderer,
        scene,
        camera,
        object,
        scene_pass,
        rgb_shift_input,
        render_pipeline,
    }
}

/// The page's `animate()`, run once by the harness's single RAF.
pub fn animate(app: &mut App) {
    {
        let mut object = app.object.borrow_mut();
        let rotation = object.rotation;
        object.set_rotation(rotation.x + 0.005, rotation.y + 0.01, rotation.z);
    }

    // `renderPipeline.render()`: the output quad's draw runs the scene pass's
    // and the dot screen's RTT's `updateBefore()` first (`docs/nodes.md` §57).
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
    three_rs::testing::pin_time(Some(0.0));
    let mut app = init();
    println!("adapter: {:?}", app.renderer.adapter_info());
    animate(&mut app);

    let (width, height, pixels) = app.renderer.read_canvas_pixels().unwrap();
    let path = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "target/webgpu_postprocessing.png".to_string());
    three_rs::testing::write_png(&path, width, height, &pixels);
    println!("wrote {path} ({width}x{height})");
}
