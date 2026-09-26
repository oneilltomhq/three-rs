//! Port of `three.js/examples/webgpu_textures_2d-array_compressed.html`,
//! calling the three-rs API in the same order the page's `init()` /
//! `animate()` do.
//!
//! Under the e2e harness the viewport is `400 * 2` x `250 * 2` with no
//! `deviceScaleFactor`, so `window.innerWidth` / `innerHeight` /
//! `devicePixelRatio` are 800, 500 and 1, and `performance.now()` is pinned
//! to 0.
//!
//! # What this page is
//!
//! `textures/spiritedaway.ktx2` is a six-layer UASTC array, 496 x 260, one
//! mip level. `KTX2Loader.detectSupport( renderer )` picks the transcode
//! target from the device's features — BC7 on a desktop adapter with
//! `texture-compression-bc`, ETC2 or ASTC on a phone, uncompressed RGBA
//! elsewhere — and the loader hands back a `CompressedArrayTexture`. The
//! material samples one layer of it with `texture( map, uv ).depth( depth )`,
//! which is a `texture_2d_array<f32>` binding and a four-argument
//! `textureSample` in the WGSL.
//!
//! The page advances the layer by `timer.getDelta() * 10` each frame. The
//! harness pins the clock, so the delta is 0 and the graded frame shows layer
//! `1 % 5 = 1`.
//!
//! `KTX2Loader.load()` is asynchronous on the page, but the harness only
//! fires its RAF once the network is idle, so the mesh is always in the
//! scene for the graded frame; here the file is transcoded synchronously.

use std::rc::Rc;

use three_rs::addons::controls::OrbitControls;
use three_rs::nodes::node::SettableValue;
use three_rs::nodes::tsl::{texture_array, uniform_settable, uv};
use three_rs::nodes::Type;
use three_rs::{
    plane_geometry, Ktx2Loader, Mesh, MeshBasicNodeMaterial, PerspectiveCamera, Renderer,
    RendererParameters, Scene, Timer,
};

pub const INNER_WIDTH: f64 = 800.0;
pub const INNER_HEIGHT: f64 = 500.0;
/// `renderer.setPixelRatio( window.devicePixelRatio )`.
pub const DPR: f64 = 1.0;

const PLANE_WIDTH: f64 = 50.0;
const PLANE_HEIGHT: f64 = 25.0;

fn examples_dir() -> std::path::PathBuf {
    three_rs::testing::three_js_dir().join("examples")
}

pub struct App {
    pub renderer: Renderer,
    pub scene: Scene,
    pub camera: PerspectiveCamera,
    /// `const timer = new THREE.Timer()`.
    pub timer: Timer,
    /// `const depth = uniform( 0 )` — the handle `animate()` writes.
    pub depth: SettableValue,
    /// `let depthStep = 1`.
    pub depth_step: f64,
}

pub fn init() -> App {
    let camera = PerspectiveCamera::new(45.0, INNER_WIDTH / INNER_HEIGHT, 0.1, 2000.0);
    camera.node.borrow_mut().position.z = 70.0;

    let scene = Scene::new();

    //
    let timer = Timer::new();

    let mut renderer = Renderer::new(RendererParameters { antialias: false }).unwrap();
    renderer.set_pixel_ratio(DPR);
    renderer.set_size(INNER_WIDTH, INNER_HEIGHT);

    //

    let ktx2_loader = Ktx2Loader::new().detect_support(&renderer);

    let texturearray = ktx2_loader
        .load(examples_dir().join("textures/spiritedaway.ktx2"))
        .and_then(|texture| texture.into_texture())
        .unwrap();

    let (depth_node, depth) = uniform_settable(Type::F32, vec![0.0]);

    // `new THREE.NodeMaterial()`. With a `colorNode` and no lighting model,
    // the port's unlit material generates the same program — the fragment
    // is `DiffuseColor = colorNode`, the opacity product, `w = 1.0` and the
    // opaque clamp, statement for statement against three's dump.
    let mut material = MeshBasicNodeMaterial::new();
    material.color_node = Some(texture_array(&texturearray, uv().flip_y(), depth_node));

    let geometry = Rc::new(plane_geometry(PLANE_WIDTH, PLANE_HEIGHT, 1, 1));

    let mesh = Mesh::new(geometry, material);

    scene.add(&mesh);

    App {
        renderer,
        scene,
        camera,
        timer,
        depth,
        depth_step: 1.0,
    }
}

/// The page's `animate()`.
pub fn animate(app: &mut App) {
    app.timer.update();

    let delta = app.timer.get_delta() * 10.0;

    app.depth_step += delta;

    let value = app.depth_step % 5.0;

    app.depth.set(vec![value]);

    app.renderer.render(&mut app.scene, &mut app.camera);
}

/// The page's `onWindowResize()`.
pub fn resize(app: &mut App, width: f64, height: f64) {
    app.camera.aspect = width / height;
    app.camera.update_projection_matrix();

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
        .unwrap_or_else(|| "target/webgpu_textures_2d-array_compressed.png".to_string());
    three_rs::testing::write_png(&path, width, height, &pixels);
    println!("wrote {path} ({width}x{height})");
}
