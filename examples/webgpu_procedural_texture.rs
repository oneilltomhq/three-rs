//! Port of `three.js/examples/webgpu_procedural_texture.html`, calling the
//! three-rs API in the same order the page's `init()` / `render()` do.
//!
//! Under the e2e harness the viewport is `400 * 2` x `250 * 2` with no
//! `deviceScaleFactor`, so `window.innerWidth` / `innerHeight` /
//! `devicePixelRatio` are 800, 500 and 1.
//!
//! A procedural checkerboard rendered into a 512 x 512 texture
//! (`convertToTexture( node, 512, 512 )`), blurred by a `gaussianBlur( …, 20 )`
//! — a 43-tap separable kernel in two passes — and shown on a unit plane under
//! an orthographic camera.
//!
//! Three fires the `RTTNode` and the `GaussianBlurNode` from inside the plane
//! material's build (`updateBefore`); the port has the page's `render()` call
//! them first, in the order three's dump has their passes: `RTT`, then
//! `Gaussian_blur_horizontal`, then `Gaussian_blur_vertical`, then the scene.
//!
//! `renderer.inspector.createParameters()` builds a GUI over the two uniforms
//! and `autoUpdate`. It draws no `Math.random()` and the graded frame is the
//! first, so every uniform still holds its constructor value.

use three_rs::addons::controls::OrbitControls;
use three_rs::materials::MeshBasicNodeMaterial;
use three_rs::nodes::display::{gaussian_blur, GaussianBlurNode, GaussianBlurOptions, RttNode};
use three_rs::nodes::tsl::{checker, uniform_value, uv};
use three_rs::nodes::Type;
use three_rs::PerspectiveCamera;
use three_rs::{plane_geometry, Mesh, OrthographicCamera, Renderer, RendererParameters, Scene};

pub const INNER_WIDTH: f64 = 800.0;
pub const INNER_HEIGHT: f64 = 500.0;
/// `renderer.setPixelRatio( window.devicePixelRatio )`.
pub const DPR: f64 = 1.0;

pub struct App {
    pub renderer: Renderer,
    pub scene: Scene,
    pub camera: OrthographicCamera,
    /// `proceduralToTexture`.
    pub procedural_to_texture: RttNode,
    /// `colorNode`'s blur.
    pub blur: GaussianBlurNode,
}

/// The page's node graph, from `uvScale` down to `gaussianBlur()`, which
/// `dump_wgsl` builds without a renderer.
pub fn nodes() -> (RttNode, GaussianBlurNode) {
    // procedural to texture

    let uv_scale = uniform_value(Type::F32, vec![4.0]);
    let blur_amount = uniform_value(Type::F32, vec![0.5]);

    let procedural = checker(uv().mul(uv_scale));
    // `convertToTexture( procedural, 512, 512 )` — ( node, width, height ):
    // a fixed-size `rtt()`.
    let procedural_to_texture = RttNode::with_size(procedural, 512, 512);

    let color_node = gaussian_blur(
        &procedural_to_texture.texture(),
        Some(blur_amount),
        20,
        GaussianBlurOptions::default(),
    );

    (procedural_to_texture, color_node)
}

pub fn init() -> App {
    let aspect = INNER_WIDTH / INNER_HEIGHT;
    let mut camera = OrthographicCamera::new(-aspect, aspect, 1.0, -1.0, 0.0, 2.0);
    camera.object.position.z = 1.0;

    let scene = Scene::new();

    let (procedural_to_texture, blur) = nodes();

    // scene

    let mut material = MeshBasicNodeMaterial::new();
    material.color_node = Some(blur.node());

    let plane = Mesh::new(std::rc::Rc::new(plane_geometry(1.0, 1.0, 1, 1)), material);
    scene.add(&plane);

    // renderer

    let mut renderer = Renderer::new(RendererParameters { antialias: true }).unwrap();
    renderer.set_pixel_ratio(DPR);
    renderer.set_size(INNER_WIDTH, INNER_HEIGHT);

    App {
        renderer,
        scene,
        camera,
        procedural_to_texture,
        blur,
    }
}

/// The page's `render()`, with the two `updateBefore()`s three fires from
/// inside it made explicit.
pub fn animate(app: &mut App) {
    app.procedural_to_texture.render(&mut app.renderer);
    app.blur.render(&mut app.renderer);
    app.renderer.render(&mut app.scene, &mut app.camera);
}

/// The page's `onWindowResize()`: the frustum keeps its height and takes the
/// new aspect.
pub fn resize(app: &mut App, width: f64, height: f64) {
    app.renderer.set_size(width, height);

    let aspect = width / height;
    let frustum_height = app.camera.top - app.camera.bottom;

    app.camera.left = -frustum_height * aspect / 2.0;
    app.camera.right = frustum_height * aspect / 2.0;

    app.camera.update_projection_matrix();
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
        .unwrap_or_else(|| "target/webgpu_procedural_texture.png".to_string());
    three_rs::testing::write_png(&path, width, height, &pixels);
    println!("wrote {path} ({width}x{height})");
}
