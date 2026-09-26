//! Port of `three.js/examples/webgpu_postprocessing_fxaa.html`, calling the
//! three-rs API in the same order the page's `init()` / `animate()` do.
//!
//! Under the e2e harness the viewport is `400 * 2` x `250 * 2` with no
//! `deviceScaleFactor`, so `window.innerWidth` / `innerHeight` /
//! `devicePixelRatio` are 800, 500 and 1. `params.animated` defaults to
//! false, so nothing moves.
//!
//! `Math.random` is the harness's seeded sequence
//! ([`three_rs::testing::DeterministicRandom`]). The page draws from it seven
//! times per instance — `position.x`, `.y`, `.z`, `scale`, `rotation.x`,
//! `.y`, `.z` — and nothing draws before the loop: `new Inspector()` is built
//! after it, as in `webgpu_postprocessing_radial_blur`.
//!
//! The page's output is `fxaa( renderOutput( scenePass ) )` with
//! `outputColorTransform = false`: FXAA wants sRGB input, so the scene is
//! encoded first, `FXAANode` runs `convertToTexture()` on that, and the
//! pipeline's own quad is the FXAA shader alone.

use std::rc::Rc;

use three_rs::addons::controls::OrbitControls;
use three_rs::geometries::tetrahedron_geometry;
use three_rs::materials::render_output;
use three_rs::nodes::display::{convert_to_texture, fxaa, FxaaNode, RttNode};
use three_rs::testing::DeterministicRandom;
use three_rs::Timer;
use three_rs::{
    Color, DirectionalLight, Group, HemisphereLight, InstancedMesh, MeshStandardNodeMaterial,
    Object3D, PassNode, PerspectiveCamera, RenderPipeline, Renderer, RendererParameters, Scene,
};

pub const INNER_WIDTH: f64 = 800.0;
pub const INNER_HEIGHT: f64 = 500.0;
/// `renderer.setPixelRatio( window.devicePixelRatio )`.
pub const DPR: f64 = 1.0;

/// `new THREE.InstancedMesh( geometry, material, 100 )`.
const COUNT: usize = 100;

/// The page's `params`.
pub struct Params {
    pub enabled: bool,
    pub animated: bool,
}

pub struct App {
    pub renderer: Renderer,
    pub scene: Scene,
    pub camera: PerspectiveCamera,
    pub group: three_rs::Node,
    /// The page's module-level `timer`.
    pub timer: Timer,
    pub params: Params,
    pub scene_pass: PassNode,
    /// The `RTTNode` `fxaa()` makes of `renderOutput( scenePass )`.
    pub fxaa_input: RttNode,
    pub fxaa: FxaaNode,
    pub render_pipeline: RenderPipeline,
}

pub fn init() -> App {
    let camera = PerspectiveCamera::new(45.0, INNER_WIDTH / INNER_HEIGHT, 0.1, 200.0);
    camera.node.borrow_mut().position.z = 50.0;

    let mut scene = Scene::new();
    scene.set_background(Color::from_hex(0xffffff));

    // `timer.connect( document )` only listens for visibility changes.
    let timer = Timer::new();

    //

    let hemi_light =
        HemisphereLight::new(Color::from_hex(0xffffff), Color::from_hex(0x8d8d8d), 1.0);
    hemi_light.borrow_mut().position.set(0.0, 1000.0, 0.0);
    scene.add(&hemi_light);

    let dir_light = DirectionalLight::new(Color::from_hex(0xffffff), 3.0);
    dir_light
        .borrow_mut()
        .position
        .set(-3000.0, 1000.0, -1000.0);
    scene.add(&dir_light);

    //

    let group = Group::new();

    let geometry = tetrahedron_geometry(1.0, 0);
    let mut material = MeshStandardNodeMaterial::standard(Color::from_hex(0xf73232), 1.0, 0.0);
    material.flat_shading = true;

    let mesh = InstancedMesh::new(Rc::new(geometry), material, COUNT);
    let mut dummy = Object3D::default();
    let mut random = DeterministicRandom::new();

    for i in 0..COUNT {
        dummy.position.x = random.next() * 50.0 - 25.0;
        dummy.position.y = random.next() * 50.0 - 25.0;
        dummy.position.z = random.next() * 50.0 - 25.0;

        dummy.scale.set_scalar(random.next() * 2.0 + 1.0);

        dummy.set_rotation(
            random.next() * std::f64::consts::PI,
            random.next() * std::f64::consts::PI,
            random.next() * std::f64::consts::PI,
        );

        dummy.update_matrix();
        mesh.borrow_mut().set_matrix_at(i, &dummy.matrix);
    }

    group.add(&mesh);
    scene.add(&group);

    let mut renderer = Renderer::new(RendererParameters { antialias: false }).unwrap();
    renderer.set_pixel_ratio(DPR);
    renderer.set_size(INNER_WIDTH, INNER_HEIGHT);

    // post processing

    let mut render_pipeline = RenderPipeline::new();

    // ignore default output color transform ( toneMapping and outputColorSpace )
    // use renderOutput() for control the sequence

    render_pipeline.output_color_transform = false;

    // scene pass

    let scene_pass = PassNode::new();
    let output_pass = render_output(scene_pass.node(), renderer.tone_mapping);

    // FXAA must be computed in sRGB color space (so after tone mapping and color space conversion)

    let fxaa_input = convert_to_texture(output_pass);
    let fxaa = fxaa(&fxaa_input.texture());
    render_pipeline.output_node = Some(fxaa.node());

    App {
        renderer,
        scene,
        camera,
        group,
        timer,
        params: Params {
            enabled: true,
            animated: false,
        },
        scene_pass,
        fxaa_input,
        fxaa,
        render_pipeline,
    }
}

/// The page's `animate()`, run once by the harness's single RAF.
pub fn animate(app: &mut App) {
    app.timer.update();
    let delta = app.timer.get_delta();

    if app.params.animated {
        let mut group = app.group.borrow_mut();
        let rotation = group.rotation;
        group.set_rotation(rotation.x, rotation.y + delta * 0.1, rotation.z);
    }

    // `renderPipeline.render()`: the scene pass, the RTT of its render
    // output, `FXAANode.updateBefore()`, then the quad — see
    // `docs/postprocessing.md` for why the port fires them explicitly.
    app.scene_pass
        .render(&mut app.renderer, &mut app.scene, &mut app.camera);
    app.fxaa_input.render(&mut app.renderer);
    app.fxaa.update();
    app.render_pipeline.render(&mut app.renderer);
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
    three_rs::testing::pin_time(Some(0.0));
    let mut app = init();
    println!("adapter: {:?}", app.renderer.adapter_info());
    animate(&mut app);

    let (width, height, pixels) = app.renderer.read_canvas_pixels().unwrap();
    let path = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "target/webgpu_postprocessing_fxaa.png".to_string());
    three_rs::testing::write_png(&path, width, height, &pixels);
    println!("wrote {path} ({width}x{height})");
}
