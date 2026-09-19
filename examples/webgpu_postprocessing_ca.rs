//! Port of `three.js/examples/webgpu_postprocessing_ca.html`, calling the
//! three-rs API in the same order the page's `init()` / `animate()` do.
//!
//! Under the e2e harness the viewport is `400 * 2` x `250 * 2` with no
//! `deviceScaleFactor`, so `window.innerWidth` / `innerHeight` /
//! `devicePixelRatio` are 800, 500 and 1, and `performance.now()` is pinned to
//! 0. The page's `animate()` reads `timer.getElapsed()`, so every rotation it
//! assigns is `0` and every `position.y` it assigns is the one `createShapes()`
//! already set — the graded frame is the scene exactly as `init()` built it.
//!
//! `controls.autoRotate` is on, and `update()` runs twice (once in `init()`,
//! once in `animate()`). With `deltaTime` null the auto-rotation angle is
//! `2π/3600 * -0.1`, and damping takes a tenth of the accumulated delta each
//! time, so the camera's azimuth moves 5.1e-5 rad in total: 2e-3 world units
//! at a radius of 42, a fiftieth of a pixel. It is not modelled; what
//! `controls.update()` does do that matters is aim the camera at
//! `( 0, 0.5, 0 )`, which nothing else in the page does.
//!
//! `Math.random` is the harness's seeded sequence
//! ([`three_rs::testing::DeterministicRandom`]). `new Inspector()` runs
//! **before** `createShapes()` here and draws five times (four tab `List`s plus
//! the built-in `Parameters` group), so the 200 particles take draws 5..604 —
//! the same five as `webgpu_tsl_galaxy`'s `INSPECTOR_RANDOM_DRAWS`, counted
//! again from the real page because a different order would move every
//! particle.
//!
//! `PointsMaterial.size` and `sizeAttenuation` are *not* modelled, and that is
//! three's own behaviour, not a gap: "WebGPU only supports point primitives
//! with a pixel size of 1", so `PointsNodeMaterial.setupVertexSprite()` — the
//! only reader of either — runs for a `Sprite`, never for a `Points`.

use std::rc::Rc;

use three_rs::core::BufferAttribute;
use three_rs::geometries::{
    box_geometry, cone_geometry, cylinder_geometry, icosahedron_geometry, octahedron_geometry,
    sphere_geometry, torus_geometry, torus_knot_geometry,
};
use three_rs::materials::render_output;
use three_rs::nodes::display::{chromatic_aberration, convert_to_texture, RttNode};
use three_rs::nodes::pmrem_node::PmremEnvironment;
use three_rs::nodes::tsl::uniform_value;
use three_rs::nodes::Type;
use three_rs::testing::DeterministicRandom;
use three_rs::{
    BufferGeometry, Color, GridHelper, Group, Mesh, MeshStandardNodeMaterial, PassNode,
    PerspectiveCamera, Points, PointsNodeMaterial, RenderPipeline, Renderer, RendererParameters,
    RoomEnvironment, Scene, Vector3,
};

pub const INNER_WIDTH: f64 = 800.0;
pub const INNER_HEIGHT: f64 = 500.0;
/// `renderer.setPixelRatio( window.devicePixelRatio )`.
pub const DPR: f64 = 1.0;

/// `Math.random()` draws `new Inspector()` makes before `createShapes()` — see
/// the module docs and `webgpu_tsl_galaxy`'s `INSPECTOR_RANDOM_DRAWS`.
const INSPECTOR_RANDOM_DRAWS: usize = 5;

/// `const particlesCount = 200`.
const PARTICLES_COUNT: usize = 200;

/// `params.strength` / `params.center` / `params.scale`.
const STRENGTH: f64 = 1.5;
const CENTER: [f64; 2] = [0.5, 0.5];
const SCALE: f64 = 1.2;

pub struct App {
    pub renderer: Renderer,
    pub scene: Scene,
    pub camera: PerspectiveCamera,
    pub scene_pass: PassNode,
    pub ca_input: RttNode,
    pub render_pipeline: RenderPipeline,
}

/// `createShapes()`' eight materials: `roughness 0.2, metalness 0.8` over the
/// page's eight colours.
fn shape_materials() -> Vec<MeshStandardNodeMaterial> {
    const COLORS: [u32; 8] = [
        0xff0000, 0x00ff00, 0x0000ff, 0xffff00, 0xff00ff, 0x00ffff, 0xffffff, 0xff8800,
    ];
    COLORS
        .iter()
        .map(|&color| MeshStandardNodeMaterial::standard(Color::from_hex(color), 0.2, 0.8))
        .collect()
}

/// `createShapes()`' eight geometries, in the page's order.
fn shape_geometries() -> Vec<Rc<BufferGeometry>> {
    vec![
        Rc::new(box_geometry(3.0, 3.0, 3.0, 1, 1, 1)),
        Rc::new(sphere_geometry(2.0, 32, 16)),
        Rc::new(cone_geometry(2.0, 4.0, 8, 1)),
        Rc::new(cylinder_geometry(1.5, 1.5, 4.0, 8)),
        Rc::new(torus_geometry(2.0, 0.8, 8, 16)),
        Rc::new(octahedron_geometry(2.5, 0)),
        Rc::new(icosahedron_geometry(2.5, 0)),
        Rc::new(torus_knot_geometry(1.5, 0.5, 64, 8, 2.0, 3.0)),
    ]
}

fn create_shapes(scene: &mut Scene, main_group: &three_rs::Node, random: &mut DeterministicRandom) {
    let materials = shape_materials();
    let geometries = shape_geometries();

    // Central showcase.
    let central_group = Group::new();

    let mut torus_material =
        MeshStandardNodeMaterial::standard(Color::from_hex(0xffffff), 0.1, 1.0);
    torus_material.emissive = Color::from_hex(0x222222);
    let central_torus = Mesh::new(Rc::new(torus_geometry(5.0, 1.5, 16, 32)), torus_material);
    central_group.add(&central_torus);

    // Inner rotating shapes.
    for i in 0..6 {
        let angle = (i as f64 / 6.0) * std::f64::consts::PI * 2.0;
        let radius = 3.0;

        let mesh = Mesh::new(
            geometries[i % geometries.len()].clone(),
            materials[i % materials.len()].clone(),
        );
        {
            let mut object = mesh.borrow_mut();
            object
                .position
                .set(angle.cos() * radius, 0.0, angle.sin() * radius);
            object.scale.set_scalar(0.5);
        }
        central_group.add(&mesh);
    }

    main_group.add(&central_group);

    // Outer ring of shapes. Each one is a `Group` with a single mesh in it,
    // which is what makes `animate()`'s `child.children.length > 0` branch —
    // not the `child.type === 'Group'` one — fire for them.
    const NUM_SHAPES: usize = 12;
    const OUTER_RADIUS: f64 = 15.0;

    for i in 0..NUM_SHAPES {
        let angle = (i as f64 / NUM_SHAPES as f64) * std::f64::consts::PI * 2.0;
        let shapes_group = Group::new();

        let mesh = Mesh::new(
            geometries[i % geometries.len()].clone(),
            materials[i % materials.len()].clone(),
        );
        shapes_group.add(&mesh);
        shapes_group.borrow_mut().position.set(
            angle.cos() * OUTER_RADIUS,
            (i as f64 * 0.5).sin() * 2.0,
            angle.sin() * OUTER_RADIUS,
        );

        main_group.add(&shapes_group);
    }

    // Floating particles.
    let mut positions = vec![0.0f32; PARTICLES_COUNT * 3];
    for i in (0..PARTICLES_COUNT * 3).step_by(3) {
        let radius = 25.0 + random.next() * 10.0;
        let theta = random.next() * std::f64::consts::PI * 2.0;
        let phi = random.next() * std::f64::consts::PI;

        positions[i] = (radius * phi.sin() * theta.cos()) as f32;
        positions[i + 1] = (radius * phi.cos()) as f32;
        positions[i + 2] = (radius * phi.sin() * theta.sin()) as f32;
    }

    let mut particles_geometry = BufferGeometry::new();
    particles_geometry.set_attribute("position", BufferAttribute::new(positions, 3));

    // `new THREE.PointsMaterial( { color: 0xffffff, size: 0.5,
    // sizeAttenuation: true } )` — the last two are dead on a `Points` under
    // WebGPU; see the module docs.
    let particles = Points::new(Rc::new(particles_geometry), PointsNodeMaterial::points());
    main_group.add(&particles);

    let _ = scene;
}

pub fn init() -> App {
    let mut renderer = Renderer::new(RendererParameters { antialias: true }).unwrap();
    renderer.set_pixel_ratio(DPR);
    renderer.set_size(INNER_WIDTH, INNER_HEIGHT);

    let mut camera = PerspectiveCamera::new(45.0, INNER_WIDTH / INNER_HEIGHT, 0.1, 200.0);
    camera.node.borrow_mut().position.set(0.0, 15.0, 40.0);
    // `controls.target.set( 0, 0.5, 0 ); controls.update()`.
    camera.look_at(&Vector3::new(0.0, 0.5, 0.0));

    let mut scene = Scene::new();
    scene.set_background(Color::from_hex(0x0a0a0a));

    // `scene.environment = pmremGenerator.fromScene( new RoomEnvironment(), 0.04 ).texture`.
    let mut room = RoomEnvironment::new();
    let environment = PmremEnvironment::from_scene(&mut renderer, &mut room, 0.04).unwrap();
    scene.environment = Some(environment.handle());

    let main_group = Group::new();
    scene.add(&main_group);

    let mut random = DeterministicRandom::new();
    random.skip(INSPECTOR_RANDOM_DRAWS);
    create_shapes(&mut scene, &main_group, &mut random);

    let grid_helper = GridHelper::new(
        40.0,
        20,
        Color::from_hex(0x444444),
        Color::from_hex(0x222222),
    );
    grid_helper.borrow_mut().position.y = -10.0;
    scene.add(&grid_helper);

    // post processing

    let scene_pass = PassNode::new();

    // `const outputPass = renderOutput( scenePass )` — then
    // `chromaticAberration()` calls `convertToTexture()` on it, which is an
    // `RTTNode`: the effect samples its input at four different uvs, so the
    // whole `renderOutput( pass )` graph is drawn into a texture of its own
    // first.
    let ca_input = convert_to_texture(render_output(scene_pass.node(), renderer.tone_mapping));

    let ca_pass = chromatic_aberration(
        &ca_input.texture(),
        uniform_value(Type::F32, vec![STRENGTH]),
        uniform_value(Type::Vec2, CENTER.to_vec()),
        uniform_value(Type::F32, vec![SCALE]),
    );

    let mut render_pipeline = RenderPipeline::new();
    // `renderPipeline.outputColorTransform = false`: the transform is already
    // inside the RTT pass, so the output quad must not add a second one.
    render_pipeline.output_color_transform = false;
    render_pipeline.output_node = Some(ca_pass);

    App {
        renderer,
        scene,
        camera,
        scene_pass,
        ca_input,
        render_pipeline,
    }
}

/// The page's `animate()`, run once by the harness's single RAF. Every value
/// it assigns is derived from `timer.getElapsed()`, which is 0, so the scene
/// is untouched and only the three passes run. See `docs/postprocessing.md`
/// for why the port fires the first two explicitly.
pub fn animate(app: &mut App) {
    app.scene_pass
        .render(&mut app.renderer, &mut app.scene, &mut app.camera);
    app.ca_input.render(&mut app.renderer);
    app.render_pipeline.render(&mut app.renderer);
}

fn main() {
    let mut app = init();
    println!("adapter: {:?}", app.renderer.adapter_info());
    animate(&mut app);

    let (width, height, pixels) = app.renderer.read_canvas_pixels().unwrap();
    let path = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "target/webgpu_postprocessing_ca.png".to_string());
    three_rs::testing::write_png(&path, width, height, &pixels);
    println!("wrote {path} ({width}x{height})");
}
