//! Port of `three.js/examples/webgpu_postprocessing_ssaa.html`, calling the
//! three-rs API in the same order the page's `init()` / `animate()` do.
//!
//! Under the e2e harness the viewport is `400 * 2` x `250 * 2` with no
//! `deviceScaleFactor`, so `window.innerWidth` / `innerHeight` /
//! `devicePixelRatio` are 800, 500 and 1, and `performance.now()` is pinned to
//! 0 — so `timer.getDelta()` is 0 and `mesh.rotation` stays at 0 for the
//! graded frame.
//!
//! `Math.random` is the harness's seeded sequence
//! ([`DeterministicRandom`](three_rs::testing::DeterministicRandom)). The page
//! draws from it **eight** times per instance, in this order: `position.x`,
//! `position.y`, `position.z`, `rotation.x`, `rotation.y`, `rotation.z`,
//! `scale`, and the hue of `color.setHSL()` — 960 draws. `setMatrixAt` is
//! called before `setColorAt`, but the draws happen in the order above.
//!
//! `renderer.toneMapping` is never set, so it stays `NoToneMapping` and the
//! `RenderPipeline` quad's `renderOutput` is only the clamp / unpremultiply →
//! sRGB OETF → premultiply sandwich.

use std::rc::Rc;

use three_rs::geometries::sphere_geometry;
use three_rs::math::ColorSpace;
use three_rs::testing::DeterministicRandom;
use three_rs::{
    AmbientLight, Color, Group, InstancedMesh, MeshStandardNodeMaterial, Object3D,
    PerspectiveCamera, PointLight, RenderPipeline, Renderer, RendererParameters, Scene,
    SsaaPassNode,
};

pub const INNER_WIDTH: f64 = 800.0;
pub const INNER_HEIGHT: f64 = 500.0;
/// `renderer.setPixelRatio( window.devicePixelRatio )`.
pub const DPR: f64 = 1.0;

/// `new THREE.InstancedMesh( geometry, material, 120 )`.
const COUNT: usize = 120;

/// `Math.random()` draws `new Inspector()` makes before the instance loop.
const INSPECTOR_RANDOM_DRAWS: usize = 5;

pub struct App {
    pub renderer: Renderer,
    pub scene: Scene,
    pub camera: PerspectiveCamera,
    pub mesh: three_rs::Node,
    pub ssaa_pass: SsaaPassNode,
    pub render_pipeline: RenderPipeline,
}

pub fn init() -> App {
    let mut renderer = Renderer::new(RendererParameters { antialias: false }).unwrap();
    renderer.set_pixel_ratio(DPR);
    renderer.set_size(INNER_WIDTH, INNER_HEIGHT);

    let mut camera = PerspectiveCamera::new(65.0, INNER_WIDTH / INNER_HEIGHT, 3.0, 10.0);
    camera.node.borrow_mut().position.z = 7.0;
    // `camera.setViewOffset( width, height, params.viewOffsetX, 0, width,
    // height )` in `init()`: a zero offset, but an *enabled* one. That is what
    // `SSAAPassNode` copies as the base its jitter is added to, and it is why
    // the restore at the end of the pass is the `setViewOffset` branch rather
    // than `clearViewOffset`.
    camera.set_view_offset(
        INNER_WIDTH,
        INNER_HEIGHT,
        0.0,
        0.0,
        INNER_WIDTH,
        INNER_HEIGHT,
    );

    let scene = Scene::new();

    // `const group = new THREE.Group(); scene.add( group );` — never used
    // again, and nothing is ever added to it. It is in the port because it is
    // one more object in the render list's traversal, even if an empty one.
    let group = Group::new();
    scene.add(&group);

    let light = PointLight::new(Color::from_hex(0xefffef), 500.0, 0.0);
    light.borrow_mut().position.set(-10.0, -10.0, 10.0);
    scene.add(&light);

    let light2 = PointLight::new(Color::from_hex(0xffefef), 500.0, 0.0);
    light2.borrow_mut().position.set(-10.0, 10.0, 10.0);
    scene.add(&light2);

    let light3 = PointLight::new(Color::from_hex(0xefefff), 500.0, 0.0);
    light3.borrow_mut().position.set(10.0, -10.0, 10.0);
    scene.add(&light3);

    scene.add(&AmbientLight::new(Color::from_hex(0xffffff), 0.2));

    let geometry = sphere_geometry(3.0, 48, 24);
    // `new THREE.MeshStandardMaterial()`: white, `roughness` 1, `metalness` 0.
    let material = MeshStandardNodeMaterial::standard(Color::from_hex(0xffffff), 1.0, 0.0);

    let mesh = InstancedMesh::new(Rc::new(geometry), material, COUNT);

    // `const dummy = new THREE.Mesh()` — a `Mesh`, not an `Object3D`, but the
    // matrix composition `updateMatrix()` does is the same.
    let mut dummy = Object3D::default();
    let mut color = Color::new(1.0, 1.0, 1.0);
    let mut random = DeterministicRandom::new();
    // `renderer.inspector = new Inspector()` runs before this loop and each of
    // the inspector's five `List`s draws one `Math.random()` for its DOM id
    // (`examples/jsm/inspector/ui/List.js:11`), so the instance loop starts
    // five draws in — the same skip `webgpu_tsl_galaxy` makes.
    random.skip(INSPECTOR_RANDOM_DRAWS);

    for i in 0..COUNT {
        dummy.position.x = random.next() * 4.0 - 2.0;
        dummy.position.y = random.next() * 4.0 - 2.0;
        dummy.position.z = random.next() * 4.0 - 2.0;
        dummy.set_rotation(random.next(), random.next(), random.next());
        dummy.scale.set_scalar(random.next() * 0.2 + 0.05);

        dummy.update_matrix();

        // `color.setHSL( h, 1.0, 0.3 )`: `setHSL`'s default colour space is the
        // *working* one, not sRGB, so the hsl triple lands in linear-sRGB with
        // no transfer function applied.
        color.set_hsl(random.next(), 1.0, 0.3, ColorSpace::LinearSRGB);

        mesh.borrow_mut().set_matrix_at(i, &dummy.matrix);
        mesh.borrow_mut().set_color_at(i, &color);
    }

    scene.add(&mesh);

    // postprocessing

    let mut ssaa_pass = SsaaPassNode::new();
    let scene_pass_color = ssaa_pass.node();

    let mut render_pipeline = RenderPipeline::new();
    render_pipeline.output_node = Some(scene_pass_color);

    // `animate()`'s `ssaaRenderPass.sampleLevel = params.sampleLevel` with
    // `params.sampleLevel` 3 — eight jitter offsets, eight scene renders.
    ssaa_pass.sample_level = 3;

    App {
        renderer,
        scene,
        camera,
        mesh,
        ssaa_pass,
        render_pipeline,
    }
}

/// The page's `animate()`, run once by the harness's single RAF.
pub fn animate(app: &mut App) {
    // `timer.update(); const delta = timer.getDelta();` — `performance.now()`
    // is pinned to 0, so the delta is 0 and the spheres never rotate.
    let delta = 0.0;
    {
        let mut mesh = app.mesh.borrow_mut();
        let rotation = mesh.rotation;
        mesh.set_rotation(
            rotation.x + delta * 0.25,
            rotation.y + delta * 0.5,
            rotation.z,
        );
    }

    // `renderer.setClearColor( 0x000000, 1.0 )` — `params.clearColor` is
    // `'black'` and `params.clearAlpha` is 1, so the sample target clears
    // opaque black rather than to `WebGPURenderer`'s transparent default.
    app.renderer.set_clear_color(Color::from_hex(0x000000), 1.0);

    app.ssaa_pass.sample_level = 3;

    // `camera.view.offsetX = params.viewOffsetX` — 0, and the view is already
    // the zero offset `init()` set, so this changes nothing. (The page does
    // not call `updateProjectionMatrix()` here either; `SSAAPassNode` does it
    // eight times over on the next line.)

    // `SSAAPassNode.updateBefore()`, then the output quad — see
    // `docs/postprocessing.md` for why the port fires the pass explicitly.
    app.ssaa_pass
        .render(&mut app.renderer, &mut app.scene, &mut app.camera);
    app.render_pipeline.render(&mut app.renderer);
}

fn main() {
    let mut app = init();
    println!("adapter: {:?}", app.renderer.adapter_info());
    animate(&mut app);

    let (width, height, pixels) = app.renderer.read_canvas_pixels().unwrap();
    let path = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "target/webgpu_postprocessing_ssaa.png".to_string());
    three_rs::testing::write_png(&path, width, height, &pixels);
    println!("wrote {path} ({width}x{height})");
}
