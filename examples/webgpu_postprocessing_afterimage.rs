//! Port of `three.js/examples/webgpu_postprocessing_afterimage.html`, calling
//! the three-rs API in the same order the page's `init()` / `animate()` do.
//!
//! Under the e2e harness the viewport is `400 * 2` x `250 * 2` with no
//! `deviceScaleFactor`, so `window.innerWidth` / `innerHeight` /
//! `devicePixelRatio` are 800, 500 and 1; the harness's single RAF passes
//! `animate()` a timestamp of 0 and `time` is 0, so `particles.rotation.z` is
//! 0 and every particle sits at its `timeOffset` along its spiral.
//!
//! `renderer.inspector = new Inspector()` is created *before* the random loop,
//! so its `Math.random()` draws come first and shift the sequence the sphere
//! points read (see [`INSPECTOR_RANDOM_DRAWS`]).
//!
//! The graded frame is the first one, so `AfterImageNode`'s old target is
//! still zero and the composite is `max( scene, 0 )`: the trail only appears
//! from the second frame on, which the steady-frame test renders.

use std::rc::Rc;

use three_rs::addons::controls::OrbitControls;
use three_rs::materials::{Blending, MeshBasicNodeMaterial};
use three_rs::math::ColorSpace;
use three_rs::nodes::display::{after_image, AfterImageNode};
use three_rs::nodes::tsl::{
    float, instanced_data_attribute, mod_float, texture, time, uniform_value, vec2_join, vec3_join,
    vec4_join,
};
use three_rs::nodes::{NodeRef, Type};
use three_rs::testing::DeterministicRandom;
use three_rs::{
    Color, Node, PassNode, PerspectiveCamera, RenderPipeline, Renderer, RendererParameters, Scene,
    Sprite, TextureLoader, Vector3,
};

pub const INNER_WIDTH: f64 = 800.0;
pub const INNER_HEIGHT: f64 = 500.0;
/// `renderer.setPixelRatio( window.devicePixelRatio )`.
pub const DPR: f64 = 1.0;

/// `Math.random()` draws `new Inspector()` makes before the particle loop —
/// the same five `webgpu_mesh_batch` and `webgpu_tsl_galaxy` document, from
/// the `List` constructors of the inspector's tabs.
pub const INSPECTOR_RANDOM_DRAWS: usize = 5;

/// `const radius = 600`.
const RADIUS: f64 = 600.0;
/// `const count = 50000`.
pub const COUNT: usize = 50000;

pub struct App {
    pub renderer: Renderer,
    pub scene: Scene,
    pub camera: PerspectiveCamera,
    pub particles: Node,
    pub scene_pass: PassNode,
    pub after_image_pass: AfterImageNode,
    /// `params.damp`.
    pub damp: NodeRef,
    pub render_pipeline: RenderPipeline,
}

fn examples_dir() -> std::path::PathBuf {
    three_rs::testing::three_js_dir().join("examples")
}

/// `getRandomPointOnSphere( r, v )`.
fn get_random_point_on_sphere(random: &mut DeterministicRandom, r: f64) -> Vector3 {
    let angle = random.next() * std::f64::consts::PI * 2.0;
    let u = random.next() * 2.0 - 1.0;

    Vector3::new(
        angle.cos() * (1.0 - u.powi(2)).sqrt() * r,
        angle.sin() * (1.0 - u.powi(2)).sqrt() * r,
        u * r,
    )
}

pub fn init() -> App {
    let mut random = DeterministicRandom::new();

    // `damp: uniform( 0.8, 'float' ).setName( 'damp' )`.
    let damp = uniform_value(Type::F32, vec![0.8]);

    let mut renderer = Renderer::new(RendererParameters { antialias: false }).unwrap();
    renderer.set_pixel_ratio(DPR);
    renderer.set_size(INNER_WIDTH, INNER_HEIGHT);
    // `renderer.inspector = new Inspector()`.
    random.skip(INSPECTOR_RANDOM_DRAWS);

    let camera = PerspectiveCamera::new(60.0, INNER_WIDTH / INNER_HEIGHT, 1.0, 10000.0);
    camera.node.borrow_mut().position.z = 1000.0;

    let mut scene = Scene::new();
    scene.set_background(Color::from_hex(0x000000));

    // The page sets no colour space on the sprite, so it is sampled as is.
    let sprite = TextureLoader::new()
        .load(examples_dir().join("textures/sprites/circle.png"))
        .unwrap();

    // geometry

    let mut color = Color::new(0.0, 0.0, 0.0);

    let mut colors = Vec::with_capacity(COUNT * 3);
    let mut vertices = Vec::with_capacity(COUNT * 3);
    let mut time_offsets = Vec::with_capacity(COUNT);

    for i in 0..COUNT {
        let vertex = get_random_point_on_sphere(&mut random, RADIUS);
        vertices.extend([vertex.x as f32, vertex.y as f32, vertex.z as f32]);

        color.set_hsl(i as f64 / COUNT as f64, 0.7, 0.7, ColorSpace::Srgb);
        colors.extend([color.r as f32, color.g as f32, color.b as f32]);

        time_offsets.push((i as f64 / COUNT as f64) as f32);
    }

    // `new THREE.InstancedBufferAttribute( new Float32Array( … ), n )`.
    let position_attribute = Rc::new(vertices);
    let color_attribute = Rc::new(colors);
    let time_attribute = Rc::new(time_offsets);

    // material and TSL

    let mut material = MeshBasicNodeMaterial::sprite();
    material.blending = Blending::Additive;
    material.depth_write = false;

    let local_time =
        instanced_data_attribute(&time_attribute, 1, 0, Type::F32).add(time().mul(float(0.1)));
    let mod_time = mod_float(local_time, float(1.0));
    let acc_time = mod_time.mul(mod_time.clone());

    let angle = acc_time.mul(float(40.0));
    let pulse = vec2_join(vec![
        angle.sin().mul(float(20.0)),
        angle.cos().mul(float(20.0)),
    ]);
    let pos = instanced_data_attribute(&position_attribute, 3, 0, Type::Vec3);

    let animated = vec3_join(vec![
        pos.x().mul(acc_time.clone()).add(pulse.x()),
        pos.y().mul(acc_time.clone()).add(pulse.y()),
        pos.z().mul(acc_time).mul(float(1.75)),
    ]);
    let f_alpha = mod_time.one_minus().mul(float(2.0));

    material.color_node = Some(texture(&sprite).mul(vec4_join(vec![
        instanced_data_attribute(&color_attribute, 3, 0, Type::Vec3),
        f_alpha,
    ])));
    material.position_node = Some(animated);
    material.scale_node = Some(float(2.0));

    let particles = Sprite::new(material);
    particles
        .borrow_mut()
        .payload
        .sprite_mut()
        .expect("a Sprite's payload is a sprite")
        .count = COUNT;
    scene.add(&particles);

    // postprocessing

    let mut render_pipeline = RenderPipeline::new();

    let scene_pass = PassNode::new();

    // `afterImage( scenePass, params.damp )` — `convertToTexture( scenePass )`
    // is the pass' own output texture.
    let after_image_pass = after_image(&scene_pass.texture(), damp.clone());

    render_pipeline.output_node = Some(after_image_pass.node());

    App {
        renderer,
        scene,
        camera,
        particles,
        scene_pass,
        after_image_pass,
        damp,
        render_pipeline,
    }
}

/// The page's `animate( time )` with `params.enabled` true, under the
/// harness's frozen clock (`time` is 0): `renderPipeline.render()`, whose
/// scene pass and after-image composite the port fires in turn (see
/// `docs/postprocessing.md`).
/// The scene pass is still rendered by hand: `AfterImageNode` is not yet on
/// the renderer-owned update path (`docs/nodes.md` §57), so the frame's order
/// is kept explicit here.
#[allow(deprecated)]
pub fn animate(app: &mut App) {
    // `animate( time )` is handed the RAF timestamp, `performance.now()`.
    let time = three_rs::utils::now_ms();
    {
        let mut particles = app.particles.borrow_mut();
        let rotation = particles.rotation;
        particles.set_rotation(rotation.x, rotation.y, time * 0.001);
    }

    app.scene_pass
        .render(&mut app.renderer, &mut app.scene, &mut app.camera);
    app.after_image_pass.render(&mut app.renderer);
    app.render_pipeline.render(&mut app.renderer);
}

/// The page's `onWindowResize()`.
pub fn resize(app: &mut App, width: f64, height: f64) {
    app.camera.aspect = width / height;
    app.camera.update_projection_matrix();
    app.renderer.set_size(width, height);
}

/// The example's controls, for a host that has a pointer. `None` here: the
/// page creates none.
pub fn controls(_app: &mut App) -> Option<&mut OrbitControls> {
    None
}

/// The controls and the camera at once. `None` here: the page creates none.
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
        .unwrap_or_else(|| "target/webgpu_postprocessing_afterimage.png".to_string());
    three_rs::testing::write_png(&path, width, height, &pixels);
    println!("wrote {path} ({width}x{height})");
}
