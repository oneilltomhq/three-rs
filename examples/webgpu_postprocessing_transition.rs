//! Port of `three.js/examples/webgpu_postprocessing_transition.html`, calling
//! the three-rs API in the same order the page does.
//!
//! Under the e2e harness the viewport is `400 * 2` x `250 * 2` with no
//! `deviceScaleFactor`, so `window.innerWidth` / `innerHeight` /
//! `devicePixelRatio` are 800, 500 and 1, and `performance.now()` is pinned to
//! 0. The page's tween waits 2000 ms before it moves `transition` off 0, and
//! `render()` draws `fxSceneB` straight to the canvas while `transition === 0`
//! — so the graded frame is scene B alone, and the `TransitionNode` quad is
//! built but not drawn in it. `tests/nodes_display_wgsl.rs` does not cover
//! `TransitionNode` either: three's dump of this page has no module for it,
//! for the same reason. It does run in the viewer, where time passes.
//!
//! `Math.random` is the harness's seeded sequence
//! ([`three_rs::testing::DeterministicRandom`]). Both `FXScene`s are built at
//! module level, before `init()` and so before `new Inspector()`: scene A's
//! 500 boxes draw ten times each (position ×3, rotation ×3, scale ×3, colour)
//! and scene B's 500 icosahedra eight (one scale, copied to y and z), so B's
//! instances are draws 5000..8999.
//!
//! Two divergences, both only visible once time runs:
//!
//! - the tween is `TWEEN.Tween( … ).to( { transition: 1 }, 1500 ).repeat(
//!   Infinity ).delay( 2000 ).yoyo( true )` with the default linear easing,
//!   evaluated here from the timer's elapsed time rather than by `tween.js`;
//! - `cycle` swaps `transitionPass.mixTextureNode.value` at each end of the
//!   tween. A texture is part of the port's graph, not a uniform, so the mix
//!   texture stays the page's initial one, `textures[ 5 ]`.

use std::rc::Rc;

use three_rs::addons::controls::OrbitControls;
use three_rs::geometries::{box_geometry, icosahedron_geometry};
use three_rs::nodes::display::transition;
use three_rs::nodes::node::SettableValue;
use three_rs::nodes::tsl::uniform_settable;
use three_rs::nodes::Type;
use three_rs::testing::DeterministicRandom;
use three_rs::{
    AmbientLight, BufferGeometry, Color, DirectionalLight, InstancedMesh, MeshPhongNodeMaterial,
    Node, Object3D, PassNode, PerspectiveCamera, RenderPipeline, Renderer, RendererParameters,
    Scene, TextureLoader, Timer, Vector3,
};

pub const INNER_WIDTH: f64 = 800.0;
pub const INNER_HEIGHT: f64 = 500.0;
/// `renderer.setPixelRatio( window.devicePixelRatio )`.
pub const DPR: f64 = 1.0;

/// `generateInstancedMesh( geometry, material, 500 )`.
const COUNT: usize = 500;

/// `effectController.texture` — the mix texture the page starts with.
const TEXTURE: usize = 5;

/// The tween's `delay( 2000 )` (which `tween.js` also uses as the repeat
/// delay) and `to( …, 1500 )`, in seconds.
const TWEEN_DELAY: f64 = 2.0;
const TWEEN_DURATION: f64 = 1.5;

fn examples_dir() -> std::path::PathBuf {
    three_rs::testing::three_js_dir().join("examples")
}

/// The page's `FXScene`.
pub struct FxScene {
    pub scene: Scene,
    pub camera: PerspectiveCamera,
    pub mesh: Node,
    rotation_speed: Vector3,
}

impl FxScene {
    fn new(
        geometry: BufferGeometry,
        is_box: bool,
        rotation_speed: Vector3,
        background_color: u32,
        random: &mut DeterministicRandom,
    ) -> Self {
        let camera = PerspectiveCamera::new(50.0, INNER_WIDTH / INNER_HEIGHT, 0.1, 100.0);
        camera.node.borrow_mut().position.z = 20.0;

        // Setup scene
        let mut scene = Scene::new();
        scene.set_background(Color::from_hex(background_color));
        scene.add(&AmbientLight::new(Color::from_hex(0xaaaaaa), 3.0));

        let light = DirectionalLight::new(Color::from_hex(0xffffff), 3.0);
        light.borrow_mut().position.set(0.0, 1.0, 4.0);
        scene.add(&light);

        let color = if is_box { 0x0000ff } else { 0xff0000 };
        let mut material = MeshPhongNodeMaterial::phong(Color::from_hex(color));
        material.flat_shading = true;
        let mesh = generate_instanced_mesh(geometry, is_box, material, COUNT, random);
        scene.add(&mesh);

        Self {
            scene,
            camera,
            mesh,
            rotation_speed,
        }
    }

    /// `this.update( delta )` with `animateScene` true.
    fn update(&mut self, delta: f64) {
        let mut mesh = self.mesh.borrow_mut();
        let rotation = mesh.rotation;
        mesh.set_rotation(
            rotation.x + self.rotation_speed.x * delta,
            rotation.y + self.rotation_speed.y * delta,
            rotation.z + self.rotation_speed.z * delta,
        );
    }

    /// `this.resize()`.
    fn resize(&mut self, width: f64, height: f64) {
        self.camera.aspect = width / height;
        self.camera.update_projection_matrix();
    }
}

fn generate_instanced_mesh(
    geometry: BufferGeometry,
    is_box: bool,
    material: MeshPhongNodeMaterial,
    count: usize,
    random: &mut DeterministicRandom,
) -> Node {
    let mesh = InstancedMesh::new(Rc::new(geometry), material, count);

    let mut dummy = Object3D::default();

    for i in 0..count {
        dummy.position.x = random.next() * 100.0 - 50.0;
        dummy.position.y = random.next() * 60.0 - 30.0;
        dummy.position.z = random.next() * 80.0 - 40.0;

        let rx = random.next() * 2.0 * std::f64::consts::PI;
        let ry = random.next() * 2.0 * std::f64::consts::PI;
        let rz = random.next() * 2.0 * std::f64::consts::PI;
        dummy.set_rotation(rx, ry, rz);

        dummy.scale.x = random.next() * 2.0 + 1.0;

        if is_box {
            dummy.scale.y = random.next() * 2.0 + 1.0;
            dummy.scale.z = random.next() * 2.0 + 1.0;
        } else {
            dummy.scale.y = dummy.scale.x;
            dummy.scale.z = dummy.scale.x;
        }

        dummy.update_matrix();

        mesh.borrow_mut().set_matrix_at(i, &dummy.matrix);
        // `color.setScalar( 0.1 + 0.9 * Math.random() )`.
        let v = 0.1 + 0.9 * random.next();
        mesh.borrow_mut().set_color_at(i, &Color::new(v, v, v));
    }

    mesh
}

pub struct App {
    pub renderer: Renderer,
    pub fx_scene_a: FxScene,
    pub fx_scene_b: FxScene,
    pub scene_pass_a: PassNode,
    pub scene_pass_b: PassNode,
    pub render_pipeline: RenderPipeline,
    /// `effectController._transition`.
    transition_uniform: SettableValue,
    /// `effectController.transition`.
    pub transition: f64,
    pub timer: Timer,
}

pub fn init() -> App {
    // Module level: the two scenes, in the page's order.
    let mut random = DeterministicRandom::new();
    let fx_scene_a = FxScene::new(
        box_geometry(2.0, 2.0, 2.0, 1, 1, 1),
        true,
        Vector3::new(0.0, -0.4, 0.0),
        0xffffff,
        &mut random,
    );
    let fx_scene_b = FxScene::new(
        icosahedron_geometry(1.0, 1),
        false,
        Vector3::new(0.0, 0.2, 0.1),
        0x000000,
        &mut random,
    );

    // Initialize textures. Only `textures[ TEXTURE ]` is ever sampled here;
    // see the module docs.
    let mix_texture = TextureLoader::new()
        .load(examples_dir().join(format!("textures/transition/transition{}.png", TEXTURE + 1)))
        .unwrap();

    let mut renderer = Renderer::new(RendererParameters { antialias: true }).unwrap();
    renderer.set_pixel_ratio(DPR);
    renderer.set_size(INNER_WIDTH, INNER_HEIGHT);

    let scene_pass_a = PassNode::new();
    let scene_pass_b = PassNode::new();

    // `effectController._transition`, `.threshold`, `._useTexture`.
    let (transition_node, transition_uniform) = uniform_settable(Type::F32, vec![0.0]);
    let (threshold, _) = uniform_settable(Type::F32, vec![0.1]);
    let (use_texture, _) = uniform_settable(Type::F32, vec![1.0]);

    let transition_pass = transition(
        &scene_pass_a.texture(),
        &scene_pass_b.texture(),
        &mix_texture,
        transition_node,
        threshold,
        use_texture,
    );

    let mut render_pipeline = RenderPipeline::new();
    render_pipeline.output_node = Some(transition_pass);

    App {
        renderer,
        fx_scene_a,
        fx_scene_b,
        scene_pass_a,
        scene_pass_b,
        render_pipeline,
        transition_uniform,
        transition: 0.0,
        timer: Timer::new(),
    }
}

/// `TWEEN.update()` for the page's one tween: `transition` at `elapsed`
/// seconds after `start()`.
fn tween_transition(elapsed: f64) -> f64 {
    // One yoyo cycle: delay, up, delay, down.
    let period = 2.0 * (TWEEN_DELAY + TWEEN_DURATION);
    let t = elapsed.rem_euclid(period);
    if t < TWEEN_DELAY {
        0.0
    } else if t < TWEEN_DELAY + TWEEN_DURATION {
        (t - TWEEN_DELAY) / TWEEN_DURATION
    } else if t < 2.0 * TWEEN_DELAY + TWEEN_DURATION {
        1.0
    } else {
        1.0 - (t - 2.0 * TWEEN_DELAY - TWEEN_DURATION) / TWEEN_DURATION
    }
}

/// The page's `animate()`.
pub fn animate(app: &mut App) {
    app.timer.update();

    // `if ( effectController.animateTransition ) TWEEN.update();`
    app.transition = tween_transition(app.timer.get_elapsed());
    app.transition_uniform.set(vec![app.transition]);

    let delta = app.timer.get_delta();
    app.fx_scene_a.update(delta);
    app.fx_scene_b.update(delta);

    render(app);
}

/// The page's `render()`.
fn render(app: &mut App) {
    // Prevent render both scenes when it's not necessary
    if app.transition == 0.0 {
        let fx = &mut app.fx_scene_b;
        app.renderer.render(&mut fx.scene, &mut fx.camera);
    } else if app.transition == 1.0 {
        let fx = &mut app.fx_scene_a;
        app.renderer.render(&mut fx.scene, &mut fx.camera);
    } else {
        // `renderPipeline.render()`: both passes, then the transition quad.
        let a = &mut app.fx_scene_a;
        app.scene_pass_a
            .render(&mut app.renderer, &mut a.scene, &mut a.camera);
        let b = &mut app.fx_scene_b;
        app.scene_pass_b
            .render(&mut app.renderer, &mut b.scene, &mut b.camera);
        app.render_pipeline.render(&mut app.renderer);
    }
}

/// The page's `onWindowResize()`.
pub fn resize(app: &mut App, width: f64, height: f64) {
    app.fx_scene_a.resize(width, height);
    app.fx_scene_b.resize(width, height);
    app.renderer.set_size(width, height);
}

/// The page creates no controls.
pub fn controls(_app: &mut App) -> Option<&mut OrbitControls> {
    None
}

/// The page creates no controls.
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
        .unwrap_or_else(|| "target/webgpu_postprocessing_transition.png".to_string());
    three_rs::testing::write_png(&path, width, height, &pixels);
    println!("wrote {path} ({width}x{height})");
}
