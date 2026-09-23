//! Port of `three.js/examples/webgpu_postprocessing_radial_blur.html`, calling
//! the three-rs API in the same order the page's `init()` / `animate()` do.
//!
//! Under the e2e harness the viewport is `400 * 2` x `250 * 2` with no
//! `deviceScaleFactor`, so `window.innerWidth` / `innerHeight` /
//! `devicePixelRatio` are 800, 500 and 1, and `performance.now()` is pinned to
//! 0 — so `timer.getDelta()` is 0 and `group.rotation.y` stays at 0 for the
//! graded frame.
//!
//! `Math.random` is the harness's seeded sequence
//! ([`three_rs::testing::DeterministicRandom`]). The page
//! draws from it seven times per instance, in this order: `position.x`,
//! `position.y`, `position.z`, `scale`, `rotation.x`, `rotation.y`,
//! `rotation.z` — 700 draws, and every one of them has to land in the same
//! place or the tetrahedra move. Nothing in the page touches `Math.random`
//! before the loop; `new Inspector()` runs after it and only registers the
//! renderer with the inspector panel, so it changes nothing in the frame.
//!
//! `renderer.toneMapping = NeutralToneMapping` is set *before* the
//! `RenderPipeline` is constructed, and `RenderPipeline._update()` captures it,
//! so the Khronos PBR Neutral tone mapper ends up inside the post-processing
//! quad's own shader rather than in a second output pass. See
//! `docs/postprocessing.md`.

use std::rc::Rc;

use three_rs::addons::controls::OrbitControls;
use three_rs::geometries::tetrahedron_geometry;
use three_rs::math::ColorSpace;
use three_rs::nodes::display::{radial_blur, RadialBlurOptions};
use three_rs::nodes::tsl::uniform_value;
use three_rs::nodes::Type;
use three_rs::testing::DeterministicRandom;
use three_rs::Timer;
use three_rs::{
    Color, Group, HemisphereLight, InstancedMesh, MeshStandardNodeMaterial, Object3D, PassNode,
    PerspectiveCamera, PointLight, RenderPipeline, Renderer, RendererParameters, Scene,
    ToneMapping, Vector2,
};

pub const INNER_WIDTH: f64 = 800.0;
pub const INNER_HEIGHT: f64 = 500.0;
/// `renderer.setPixelRatio( window.devicePixelRatio )`.
pub const DPR: f64 = 1.0;

/// `new THREE.InstancedMesh( geometry, material, 100 )`.
const COUNT: usize = 100;

pub struct App {
    pub renderer: Renderer,
    pub scene: Scene,
    pub camera: PerspectiveCamera,
    pub group: three_rs::Node,
    /// The page's module-level `timer`.
    pub timer: Timer,
    pub scene_pass: PassNode,
    pub render_pipeline: RenderPipeline,
}

pub fn init() -> App {
    let camera = PerspectiveCamera::new(45.0, INNER_WIDTH / INNER_HEIGHT, 0.1, 200.0);
    camera.node.borrow_mut().position.z = 50.0;

    let mut scene = Scene::new();
    scene.set_background(Color::from_hex(0x000000));

    //

    let hemi_light =
        HemisphereLight::new(Color::from_hex(0xffffff), Color::from_hex(0x8d8d8d), 1.0);
    hemi_light.borrow_mut().position.set(0.0, 1000.0, 0.0);
    scene.add(&hemi_light);

    let point_light = PointLight::new(Color::from_hex(0xffffff), 1000.0, 0.0);
    point_light.borrow_mut().position.set(0.0, 0.0, 0.0);
    scene.add(&point_light);

    //

    let group = Group::new();

    let geometry = tetrahedron_geometry(1.0, 0);
    let mut material = MeshStandardNodeMaterial::standard(Color::from_hex(0xffffff), 1.0, 0.0);
    material.flat_shading = true;

    let mesh = InstancedMesh::new(Rc::new(geometry), material, COUNT);
    let mut dummy = Object3D::default();
    let mut col = Color::new(1.0, 1.0, 1.0);
    let mut center = Vector2::default();
    let mut random = DeterministicRandom::new();

    for i in 0..COUNT {
        dummy.position.x = random.next() * 50.0 - 25.0;
        dummy.position.y = random.next() * 50.0 - 25.0;
        dummy.position.z = random.next() * 50.0 - 25.0;

        center.set(dummy.position.x, dummy.position.y);

        // make sure tetrahedrons are not positioned at the origin

        if center.length() < 6.0 {
            center.normalize().multiply_scalar(6.0);
            dummy.position.x = center.x;
            dummy.position.y = center.y;
        }

        dummy.scale.set_scalar(random.next() * 2.0 + 1.0);

        dummy.set_rotation(
            random.next() * std::f64::consts::PI,
            random.next() * std::f64::consts::PI,
            random.next() * std::f64::consts::PI,
        );

        dummy.update_matrix();
        mesh.borrow_mut().set_matrix_at(i, &dummy.matrix);

        col.set_hsl(
            0.55 + (i as f64 / COUNT as f64) * 0.15,
            1.0,
            0.2,
            ColorSpace::LinearSRGB,
        );

        mesh.borrow_mut().set_color_at(i, &col);
    }

    group.add(&mesh);
    scene.add(&group);

    let mut renderer = Renderer::new(RendererParameters { antialias: false }).unwrap();
    renderer.set_pixel_ratio(DPR);
    renderer.set_size(INNER_WIDTH, INNER_HEIGHT);
    renderer.tone_mapping = ToneMapping::Neutral;

    // post processing

    let scene_pass = PassNode::new();

    // `uniform( float( 0.9 ) )` and friends. `uniform( int( 32 ) )` is an
    // `int` *node* wrapped in a uniform, and `UniformNode` takes its type from
    // the JavaScript value it is given, which is a node — so three.js falls
    // back to `float` and the dump carries four `f32` object uniforms. The
    // loop bound is `i32( count )` in the shader, not an `i32` uniform.
    let options = RadialBlurOptions {
        weight: uniform_value(Type::F32, vec![0.9]),
        decay: uniform_value(Type::F32, vec![0.95]),
        exposure: uniform_value(Type::F32, vec![5.0]),
        count: uniform_value(Type::F32, vec![32.0]),
        ..Default::default()
    };

    let blur_pass = radial_blur(&scene_pass.texture(), &options);

    let mut render_pipeline = RenderPipeline::new();
    render_pipeline.output_node = Some(blur_pass);

    App {
        // `const timer = new THREE.Timer();` — constructed in `init()`, as the
        // page does, so its `_startTime` is the moment the scene was built.
        timer: Timer::new(),
        renderer,
        scene,
        camera,
        group,
        scene_pass,
        render_pipeline,
    }
}

/// The page's `animate()`, run once by the harness's single RAF.
pub fn animate(app: &mut App) {
    // `timer.update(); const delta = timer.getDelta();`, then the rotation
    // under `params.animated`, which defaults to true.
    app.timer.update();
    let delta = app.timer.get_delta();

    {
        let mut group = app.group.borrow_mut();
        let rotation = group.rotation;
        group.set_rotation(rotation.x, rotation.y + delta * 0.1, rotation.z);
    }

    // `PassNode.updateBefore()`, then the output quad — see
    // `docs/postprocessing.md` for why the port fires the pass explicitly.
    app.scene_pass
        .render(&mut app.renderer, &mut app.scene, &mut app.camera);
    app.render_pipeline.render(&mut app.renderer);
}

/// The page's `onWindowResize()`.
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
        .unwrap_or_else(|| "target/webgpu_postprocessing_radial_blur.png".to_string());
    three_rs::testing::write_png(&path, width, height, &pixels);
    println!("wrote {path} ({width}x{height})");
}
