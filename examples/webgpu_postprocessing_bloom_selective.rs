//! Port of `three.js/examples/webgpu_postprocessing_bloom_selective.html`,
//! calling the three-rs API in the same order the page's top-level script does.
//!
//! Under the e2e harness the viewport is `400 * 2` x `250 * 2` with no
//! `deviceScaleFactor`, so `window.innerWidth` / `innerHeight` /
//! `devicePixelRatio` are 800, 500 and 1. The page has no timer at all: its
//! `animate()` is one `renderPipeline.render()`, so every frame is the same
//! frame and the graded one is the first.
//!
//! `Math.random` is the harness's seeded sequence
//! ([`three_rs::testing::DeterministicRandom`]). The page draws from it **nine**
//! times per sphere, in this order: the hue and the lightness of
//! `color.setHSL( h, 0.7, l )` (JavaScript evaluates the arguments left to
//! right, so the hue comes first), the `bloomIntensity` coin flip,
//! `position.x`, `.y`, `.z`, the `multiplyScalar` radius, and **two** for
//! `scale.setScalar( Math.random() * Math.random() + 0.5 )` — 450 draws. The
//! third of each nine decides whether that sphere blooms, so an off-by-one in
//! the sequence is not a small pixel move but a different picture;
//! `tests/e2e/main.rs` asserts the fifty spheres against the scout's oracle
//! before it compares a pixel.
//!
//! Nothing draws from `Math.random` before the loop: `new Inspector()` is
//! constructed after it, unlike `webgpu_postprocessing_ssaa`'s.
//!
//! `renderer.toneMapping = NeutralToneMapping` is set before the
//! `RenderPipeline` exists, and `outputColorTransform = false` with an explicit
//! `.renderOutput()` on the output node puts the whole colour transform inside
//! the graph instead of around it — the same shader either way, and the reason
//! the final quad's WGSL is `m12` rather than a bare `output.color = …`.

use std::rc::Rc;

use three_rs::addons::controls::OrbitControls;
use three_rs::geometries::icosahedron_geometry;
use three_rs::materials::render_output;
use three_rs::math::ColorSpace;
use three_rs::nodes::display::{bloom, BloomNode};
use three_rs::nodes::tsl::{float, output_property, uniform_value};
use three_rs::nodes::{mrt, Type};
use three_rs::testing::DeterministicRandom;
use three_rs::{
    Color, Mesh, MeshBasicNodeMaterial, PassNode, PerspectiveCamera, RenderPipeline, Renderer,
    RendererParameters, Scene, ToneMapping, Vector3,
};

pub const INNER_WIDTH: f64 = 800.0;
pub const INNER_HEIGHT: f64 = 500.0;
/// `renderer.setPixelRatio( window.devicePixelRatio )`.
pub const DPR: f64 = 1.0;

/// `for ( let i = 0; i < 50; i ++ )`.
const COUNT: usize = 50;

pub struct App {
    pub renderer: Renderer,
    pub scene: Scene,
    pub camera: PerspectiveCamera,
    /// The page's `controls`.
    pub controls: OrbitControls,
    pub scene_pass: PassNode,
    pub bloom_pass: BloomNode,
    pub render_pipeline: RenderPipeline,
}

pub fn init() -> App {
    let scene = Scene::new();

    let mut camera = PerspectiveCamera::new(40.0, INNER_WIDTH / INNER_HEIGHT, 1.0, 200.0);
    camera.node.borrow_mut().position.set(0.0, 0.0, 20.0);
    camera.look_at(&Vector3::new(0.0, 0.0, 0.0));

    // `new THREE.IcosahedronGeometry( 1, 15 )` — one geometry for all fifty
    // spheres, 7200 triangles each.
    let geometry = Rc::new(icosahedron_geometry(1.0, 15));

    let mut random = DeterministicRandom::new();

    for _ in 0..COUNT {
        let mut color = Color::new(1.0, 1.0, 1.0);
        // `color.setHSL( Math.random(), 0.7, Math.random() * 0.2 + 0.05 )`:
        // `setHSL`'s default colour space is the *working* one, so the hsl
        // triple lands in linear-sRGB with no transfer function applied.
        let hue = random.next();
        let lightness = random.next() * 0.2 + 0.05;
        color.set_hsl(hue, 0.7, lightness, ColorSpace::LinearSRGB);

        // `const bloomIntensity = Math.random() > 0.5 ? 1 : 0` — the coin flip
        // that decides which spheres glow.
        let bloom_intensity = if random.next() > 0.5 { 1.0 } else { 0.0 };

        let mut material = MeshBasicNodeMaterial::new();
        material.color = color;
        // `material.mrtNode = mrt( { bloomIntensity: uniform( bloomIntensity ) } )`
        // — a per-material override of the pass's `bloomIntensity` channel,
        // merged over the pass's own `mrt( { output, bloomIntensity: float( 0 ) } )`.
        material.mrt_node = Some(mrt(vec![(
            "bloomIntensity",
            uniform_value(Type::F32, vec![bloom_intensity]),
        )]));

        let sphere = Mesh::new(geometry.clone(), material);
        {
            let mut object = sphere.borrow_mut();
            object.position.x = random.next() * 10.0 - 5.0;
            object.position.y = random.next() * 10.0 - 5.0;
            object.position.z = random.next() * 10.0 - 5.0;
            let radius = random.next() * 4.0 + 2.0;
            object.position.normalize().multiply_scalar(radius);
            object.scale.set_scalar(random.next() * random.next() + 0.5);
        }
        scene.add(&sphere);
    }

    // renderer

    let mut renderer = Renderer::new(RendererParameters { antialias: false }).unwrap();
    renderer.set_pixel_ratio(DPR);
    renderer.set_size(INNER_WIDTH, INNER_HEIGHT);
    renderer.tone_mapping = ToneMapping::Neutral;

    // post processing

    let scene_pass = PassNode::new();
    scene_pass.set_mrt(mrt(vec![
        ("output", output_property()),
        ("bloomIntensity", float(0.0)),
    ]));

    // `toInspector( … )` is a no-op on the rendered frame: it only names the
    // node for the inspector panel and returns it unchanged.
    let output_pass = scene_pass.texture_node("output");
    let bloom_intensity_pass = scene_pass.texture_node("bloomIntensity");

    let bloom_pass = bloom(output_pass.mul(bloom_intensity_pass));

    let mut render_pipeline = RenderPipeline::new();
    // `renderPipeline.outputColorTransform = false` with
    // `outputNode = outputPass.add( bloomPass ).renderOutput()`: the transform
    // is in the graph, so the pipeline must not add a second one.
    render_pipeline.output_color_transform = false;
    render_pipeline.output_node = Some(render_output(
        scene_pass.texture_node("output").add(bloom_pass.node()),
        renderer.tone_mapping,
    ));

    // controls

    // `const controls = new OrbitControls( camera, renderer.domElement )`,
    // built here because the page builds it here, after the render pipeline.
    // Its constructor's `update()` is the `camera.look_at` above, and
    // `animate()` never touches it again.
    let mut controls = OrbitControls::new(&mut camera);
    // The canvas the example renders at, standing in for the element's
    // `clientWidth` / `clientHeight`.
    controls.set_element_size(INNER_WIDTH, INNER_HEIGHT);
    controls.max_polar_angle = std::f64::consts::PI * 0.5;
    controls.min_distance = 1.0;
    controls.max_distance = 100.0;

    App {
        renderer,
        scene,
        camera,
        controls,
        scene_pass,
        bloom_pass,
        render_pipeline,
    }
}

/// The page's `animate()`, run once by the harness's single RAF: one
/// `renderPipeline.render()`, which fires both `updateBefore()`s on its way.
/// See `docs/postprocessing.md` for why the port fires them explicitly.
pub fn animate(app: &mut App) {
    app.scene_pass
        .render(&mut app.renderer, &mut app.scene, &mut app.camera);
    app.bloom_pass.render(&mut app.renderer);
    app.render_pipeline.render(&mut app.renderer);
}

/// The page's resize handler, which it installs as `window.onresize =
/// function () { … }` rather than as a named `onWindowResize`.
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
///
/// They are two fields of the same `App`, so borrowing both is sound — but
/// only this module can say so; a host holding `&mut App` and calling
/// [`controls`] and then reaching for the camera cannot. Hence the pair.
pub fn controls_and_camera(app: &mut App) -> Option<(&mut OrbitControls, &mut PerspectiveCamera)> {
    Some((&mut app.controls, &mut app.camera))
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
        .unwrap_or_else(|| "target/webgpu_postprocessing_bloom_selective.png".to_string());
    three_rs::testing::write_png(&path, width, height, &pixels);
    println!("wrote {path} ({width}x{height})");
}
