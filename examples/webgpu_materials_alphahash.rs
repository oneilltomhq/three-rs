//! Port of `three.js/examples/webgpu_materials_alphahash.html`, calling the
//! three-rs API in the same order the page's `init()` / `animate()` do.
//!
//! Under the e2e harness the viewport is `400 * 2` x `250 * 2` with no
//! `deviceScaleFactor`, so `window.innerWidth` / `innerHeight` /
//! `devicePixelRatio` are 800, 500 and 1, and `window.location.search` is
//! empty, so `amount` is the default 3 and the mesh has 27 instances.
//!
//! `Math.random` is the harness's seeded sequence
//! ([`three_rs::testing::DeterministicRandom`]). The page draws from it once
//! per instance, for `color.setHex( Math.random() * 0xffffff )`, and it does
//! so *before* `renderer.inspector = new Inspector()` — so unlike
//! `webgpu_postprocessing_ssaa` no draws are skipped for the inspector's DOM
//! ids.
//!
//! The page's material is a plain `MeshStandardMaterial` (white, `roughness`
//! 1, `metalness` 0) with `alphaHash: true` and `opacity: 0.5`; it is *not*
//! `transparent`, so the fragments that survive the hash get `alpha = 1` and
//! write depth. The only light is `scene.environment`, RoomEnvironment
//! through `PMREMGenerator.fromScene( …, 0.04 )`. The frame is the
//! `ssaaPass( scene, camera )` accumulator at `sampleLevel` 3 (eight jittered
//! scene renders), shown through the `RenderPipeline` output quad.

use std::rc::Rc;

use three_rs::addons::controls::OrbitControls;
use three_rs::geometries::icosahedron_geometry;
use three_rs::math::{ColorSpace, Matrix4};
use three_rs::nodes::pmrem_node::PmremEnvironment;
use three_rs::testing::DeterministicRandom;
use three_rs::{
    Color, InstancedMesh, MeshStandardNodeMaterial, PerspectiveCamera, RenderPipeline, Renderer,
    RendererParameters, RoomEnvironment, Scene, SsaaPassNode, Vector3,
};

pub const INNER_WIDTH: f64 = 800.0;
pub const INNER_HEIGHT: f64 = 500.0;
/// `renderer.setPixelRatio( window.devicePixelRatio )`.
pub const DPR: f64 = 1.0;

/// `parseInt( window.location.search.slice( 1 ) ) || 3`.
const AMOUNT: usize = 3;

/// `params` — the GUI's initial values.
const ALPHA: f64 = 0.5;
const ALPHA_HASH: bool = true;

pub struct App {
    pub renderer: Renderer,
    pub scene: Scene,
    pub camera: PerspectiveCamera,
    pub controls: OrbitControls,
    pub mesh: three_rs::ObjectRef,
    pub scene_pass: SsaaPassNode,
    pub render_pipeline: RenderPipeline,
}

pub fn init() -> App {
    let count = AMOUNT.pow(3);

    let mut camera = PerspectiveCamera::new(60.0, INNER_WIDTH / INNER_HEIGHT, 0.1, 100.0);
    let amount = AMOUNT as f64;
    camera
        .node
        .borrow_mut()
        .position
        .set(amount, amount, amount);
    camera.look_at(&Vector3::new(0.0, 0.0, 0.0));

    let mut scene = Scene::new();
    scene.set_background(Color::from_hex(0x000000));

    let geometry = icosahedron_geometry(0.5, 3);

    // `new THREE.MeshStandardMaterial( { color: 0xffffff, alphaHash:
    // params.alphaHash, opacity: params.alpha } )`.
    let mut material = MeshStandardNodeMaterial::standard(Color::from_hex(0xffffff), 1.0, 0.0);
    material.alpha_hash = ALPHA_HASH;
    material.opacity = ALPHA;

    let mesh = InstancedMesh::new(Rc::new(geometry), material, count);

    let mut random = DeterministicRandom::new();
    let mut color = Color::default();
    let offset = (amount - 1.0) / 2.0;
    let mut matrix = Matrix4::identity();

    let mut i = 0;
    for x in 0..AMOUNT {
        for y in 0..AMOUNT {
            for z in 0..AMOUNT {
                matrix.set_position(offset - x as f64, offset - y as f64, offset - z as f64);

                // `color.setHex( Math.random() * 0xffffff )` — `setHex` floors
                // its argument first, and reads it as sRGB.
                let hex = (random.next() * 0xffffff as f64).floor() as u32;
                color.set_hex(hex, ColorSpace::Srgb);

                mesh.borrow_mut().set_matrix_at(i, &matrix);
                mesh.borrow_mut().set_color_at(i, &color);

                i += 1;
            }
        }
    }

    scene.add(&mesh);

    //

    let mut renderer = Renderer::new(RendererParameters::default()).unwrap();
    renderer.set_pixel_ratio(DPR);
    renderer.set_size(INNER_WIDTH, INNER_HEIGHT);

    //

    let mut room = RoomEnvironment::new();
    let environment = PmremEnvironment::from_scene(&mut renderer, &mut room, 0.04).unwrap();
    scene.environment = Some(environment.handle());
    // `environment.dispose()` — `Drop` frees the same resources.
    drop(room);

    // postprocessing

    let mut scene_pass = SsaaPassNode::new();
    scene_pass.sample_level = 3;

    let mut render_pipeline = RenderPipeline::new();
    render_pipeline.output_node = Some(scene_pass.node());

    //

    let mut controls = OrbitControls::new(&mut camera);
    controls.set_element_size(INNER_WIDTH, INNER_HEIGHT);
    controls.enable_zoom = false;
    controls.enable_pan = false;

    App {
        renderer,
        scene,
        camera,
        controls,
        mesh,
        scene_pass,
        render_pipeline,
    }
}

/// The page's `animate()`, run once by the harness's single RAF:
/// `renderPipeline.render()`.
pub fn animate(app: &mut App) {
    // `SSAAPassNode.updateBefore()`, then the output quad — see
    // `docs/postprocessing.md` for why the port fires the pass explicitly.
    app.scene_pass
        .render(&mut app.renderer, &mut app.scene, &mut app.camera);
    app.render_pipeline.render(&mut app.renderer);
}

/// The page's `onWindowResize()`.
pub fn resize(app: &mut App, width: f64, height: f64) {
    app.camera.aspect = width / height;
    app.camera.update_projection_matrix();
    app.renderer.set_size(width, height);
}

/// The example's controls, for a host that has a pointer.
pub fn controls(app: &mut App) -> Option<&mut OrbitControls> {
    Some(&mut app.controls)
}

/// The controls and the camera at once, for a host delivering pointer events.
pub fn controls_and_camera(app: &mut App) -> Option<(&mut OrbitControls, &mut PerspectiveCamera)> {
    Some((&mut app.controls, &mut app.camera))
}

fn main() {
    three_rs::testing::pin_time(Some(0.0));
    let mut app = init();
    println!("adapter: {:?}", app.renderer.adapter_info());
    animate(&mut app);

    let (width, height, pixels) = app.renderer.read_canvas_pixels().unwrap();
    let path = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "target/webgpu_materials_alphahash.png".to_string());
    three_rs::testing::write_png(&path, width, height, &pixels);
    println!("wrote {path} ({width}x{height})");
}
