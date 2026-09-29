//! Port of `three.js/examples/webgpu_multisampled_renderbuffers.html`, calling
//! the three-rs API in the same order the page's `init()` / `animate()` do.
//!
//! Fifty wireframe boxes and fifty red boxes inside them, two
//! `InstancedMesh`es, drawn into a `RenderTarget` with `samples: 4` — an MSAA
//! colour attachment resolved into the target's `rgba8unorm` texture, beside a
//! multisampled depth buffer — and then shown through a `QuadMesh` that samples
//! `renderTarget.texture`. The wireframe is `material.wireframe = true`: a
//! `line-list` through `Geometries.getWireframeAttribute()`'s index.
//!
//! `renderer.inspector = new Inspector()` and its `Settings` panel only toggle
//! `params`, which stay at their defaults (`multisampling` and `animated` both
//! true); neither the `mousemove` nor the `resize` listener fires.
//!
//! # The random sequence
//!
//! `positions` is built at module scope, before `init()` and so before
//! `new Inspector()` makes its own draws: the first hundred values of the
//! harness' seeded `Math.random()`, two per box — `phi` first, then `theta`.

use std::rc::Rc;

use three_rs::addons::controls::OrbitControls;
use three_rs::nodes::tsl::texture;
use three_rs::renderer::RenderTargetOptions;
use three_rs::testing::DeterministicRandom;
use three_rs::{
    box_geometry, Color, InstancedMesh, Matrix4, MeshBasicNodeMaterial, Node, PerspectiveCamera,
    QuadMesh, RenderTarget, Renderer, RendererParameters, Scene, Vector3,
};

pub const INNER_WIDTH: f64 = 800.0;
pub const INNER_HEIGHT: f64 = 500.0;
/// `const dpr = 1;`
pub const DPR: f64 = 1.0;

/// `const params = { animated: true, multisampling: true }`.
const ANIMATED: bool = true;
const MULTISAMPLING: bool = true;

const COUNT: usize = 50;
/// Radius of the sphere.
const FULL_RADIUS: f64 = 20.0;
/// Radius of the sphere.
const HALF_RADIUS: f64 = 10.0;

pub struct App {
    pub renderer: Renderer,
    pub scene: Scene,
    pub camera: PerspectiveCamera,
    pub quad_mesh: QuadMesh,
    pub render_target: RenderTarget,
    pub boxes: [Node; 2],
}

/// The page's module-scope `positions`: points on two spheres, alternating
/// between the full and the half radius.
fn positions() -> Vec<Vector3> {
    let mut random = DeterministicRandom::new();
    (0..COUNT)
        .map(|i| {
            let radius = if i % 2 == 0 { FULL_RADIUS } else { HALF_RADIUS };

            // phi: latitude, range -π/2 to π/2
            let phi = (2.0 * random.next() - 1.0).acos() - std::f64::consts::PI / 2.0;
            // theta: longitude, range 0 to 2π
            let theta = 2.0 * std::f64::consts::PI * random.next();

            Vector3::new(
                radius * phi.cos() * theta.cos(), // x
                radius * phi.sin(),               // y
                radius * phi.cos() * theta.sin(), // z
            )
        })
        .collect()
}

pub fn init() -> App {
    let positions = positions();
    let mut mat4 = Matrix4::identity();

    let camera = PerspectiveCamera::new(70.0, INNER_WIDTH / INNER_HEIGHT, 0.1, 50.0);
    camera.node.borrow_mut().position.z = 3.0;

    let mut scene = Scene::new();
    scene.set_background(Color::from_hex(0x111111));

    // textured mesh

    let geometry_box = Rc::new(box_geometry(7.0, 7.0, 7.0, 12, 12, 12));
    let mut material_box = MeshBasicNodeMaterial::new();
    let mut material_box_inner = MeshBasicNodeMaterial::new();
    material_box_inner.color = Color::from_hex(0xff0000);

    material_box.wireframe = true;

    //

    let boxed = InstancedMesh::new(geometry_box.clone(), material_box, COUNT);
    let box2 = InstancedMesh::new(geometry_box, material_box_inner, COUNT);

    for (i, position) in positions.iter().enumerate() {
        boxed.borrow_mut().set_matrix_at(
            i,
            mat4.set_identity()
                .set_position(position.x, position.y, position.z),
        );
        // `mat4.multiplyScalar( 0.996 )` scales the whole matrix, the
        // translation column and `w` included, before `setPosition` puts the
        // translation back: the instance matrix keeps an `elements[ 15 ]` of
        // 0.996, which the vertex stage drops with `.xyz`. The page's own
        // arithmetic, reproduced.
        box2.borrow_mut().set_matrix_at(
            i,
            mat4.multiply_scalar(0.996)
                .set_position(position.x, position.y, position.z),
        );
    }

    scene.add(&boxed);
    scene.add(&box2);

    //

    let mut parameters = RendererParameters::default();
    parameters.antialias = true;
    let mut renderer = Renderer::new(parameters).unwrap();
    renderer.set_pixel_ratio(DPR);
    renderer.set_size(INNER_WIDTH, INNER_HEIGHT);

    let mut options = RenderTargetOptions::default();
    options.samples = if MULTISAMPLING { 4 } else { 1 };
    let render_target = RenderTarget::new_with_options(
        (INNER_WIDTH * DPR) as u32,
        (INNER_HEIGHT * DPR) as u32,
        options,
    )
    .expect("the default texture type is a colour type");

    // FX

    // modulate the final color based on the mouse position

    let mut material_fx = MeshBasicNodeMaterial::new();
    material_fx.color_node = Some(texture(&render_target.texture()).rgb());

    let quad_mesh = QuadMesh::new(material_fx);

    App {
        renderer,
        scene,
        camera,
        quad_mesh,
        render_target,
        boxes: [boxed, box2],
    }
}

pub fn animate(app: &mut App) {
    if ANIMATED {
        for node in &app.boxes {
            let mut object = node.borrow_mut();
            let rotation = object.rotation;
            object.set_rotation(rotation.x + 0.001, rotation.y + 0.002, rotation.z);
        }
    }

    app.render_target
        .set_samples(if MULTISAMPLING { 4 } else { 1 });

    app.renderer
        .set_render_target(Some(app.render_target.clone()));
    app.renderer.render(&mut app.scene, &mut app.camera);

    app.renderer.set_render_target(None);
    app.renderer.render_quad(&app.quad_mesh);
}

/// The page's `onWindowResize()`.
pub fn resize(app: &mut App, width: f64, height: f64) {
    app.camera.aspect = width / height;
    app.camera.update_projection_matrix();

    app.renderer.set_size(width, height);
    app.render_target
        .set_size((width * DPR) as u32, (height * DPR) as u32);
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
        .unwrap_or_else(|| "target/webgpu_multisampled_renderbuffers.png".to_string());
    three_rs::testing::write_png(&path, width, height, &pixels);
    println!("wrote {path} ({width}x{height})");
}
