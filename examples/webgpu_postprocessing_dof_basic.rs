//! Port of `three.js/examples/webgpu_postprocessing_dof_basic.html`, calling
//! the three-rs API in the same order the page's `init()` / `animate()` do.
//!
//! The page's depth of field is not `DepthOfFieldNode` but the "simple DOF"
//! of the lettier tutorial it cites: the beauty pass and a `boxBlur` of it are
//! mixed by `smoothstep( minDistance, maxDistance, | viewZ - focus.z | )`,
//! where `focus` is a fixed world point carried into view space every frame.
//! The mix is tone-mapped by `renderOutput()` and anti-aliased by `fxaa()`.
//!
//! Under the e2e harness the viewport is `400 * 2` x `250 * 2` with no
//! `deviceScaleFactor`, so `window.innerWidth` / `innerHeight` /
//! `devicePixelRatio` are 800, 500 and 1.
//!
//! Things the page does that are easy to lose:
//!
//! * **`scene.environmentRotation.y = -π / 2`.** The PMREM is sampled through
//!   `materialEnvRotation`, which the port reads off
//!   [`Scene::environment_rotation`](three_rs::Scene::environment_rotation).
//! * **The background is a flat colour**, not the environment map.
//! * **`controls.update()` runs in `init()` and in every `animate()`**, with
//!   damping on and no input, so the camera stays at `( -6, 5, 6 )` looking
//!   at `( 0, 2, 0 )`.
//! * **The mixer is updated with `timer.getDelta()`**, 0 on the graded frame,
//!   which still poses the bath toys at the clip's first keyframe.
//! * **The tween and the raycast click** move the focus point on pointer
//!   input only; neither runs here, so the focus stays at `( 1, 1.75, -0.4 )`.
//! * **The GUI** (min/max distance, blur size/spread) is not ported; the
//!   uniforms keep their defaults.

use std::cell::RefCell;
use std::rc::Rc;

use three_rs::addons::controls::OrbitControls;
use three_rs::animation::AnimationMixer;
use three_rs::loaders::{GltfLoader, UltraHdrLoader};
use three_rs::materials::{render_output, ToneMapping};
use three_rs::math::Vector3;
use three_rs::nodes::display::{
    box_blur, convert_to_texture, fxaa, BoxBlurOptions, FxaaNode, RttNode,
};
use three_rs::nodes::node::SettableValue;
use three_rs::nodes::pmrem_node::PmremEnvironment;
use three_rs::nodes::tsl::{mix, smoothstep, uniform_settable, uniform_value};
use three_rs::nodes::{NodeRef, Type};
use three_rs::Timer;
use three_rs::{
    pass, Color, PassNode, PerspectiveCamera, RenderPipeline, Renderer, RendererParameters, Scene,
};

pub const INNER_WIDTH: f64 = 800.0;
pub const INNER_HEIGHT: f64 = 500.0;
/// `renderer.setPixelRatio( window.devicePixelRatio )`.
pub const DPR: f64 = 1.0;

fn examples_dir() -> std::path::PathBuf {
    three_rs::testing::three_js_dir().join("examples")
}

pub struct App {
    pub renderer: Renderer,
    /// Shared with `scene_pass`, which renders it from `updateBefore()`.
    pub scene: Rc<RefCell<Scene>>,
    /// Shared with `scene_pass`.
    pub camera: Rc<RefCell<PerspectiveCamera>>,
    /// The page's `controls`.
    pub controls: OrbitControls,
    /// The page's module-level `mixer`.
    pub mixer: AnimationMixer,
    /// The page's module-level `timer`.
    pub timer: Timer,
    /// `scene.environment`, PMREM-filtered.
    pub environment: PmremEnvironment,
    /// The page's `focusPoint`, in world space.
    pub focus_point: Vector3,
    /// `focusPointView = uniform( vec3() )`, written every frame.
    pub focus_point_view: SettableValue,
    /// `uniform( 2 )`, the kernel's half-width.
    pub blur_size: NodeRef,
    /// `uniform( 4 )`, the tap spacing in texels.
    pub blur_spread: NodeRef,
    /// `uniform( 1 )`, at or below which a fragment is fully in focus.
    pub min_distance: NodeRef,
    /// `uniform( 3 )`, at or beyond which a fragment is fully blurred.
    pub max_distance: NodeRef,
    pub scene_pass: PassNode,
    /// The `RTTNode` `fxaa()` makes of `renderOutput( dofPass )`.
    pub fxaa_input: RttNode,
    pub fxaa: FxaaNode,
    pub render_pipeline: RenderPipeline,
}

pub fn init() -> App {
    let mut camera = PerspectiveCamera::new(60.0, INNER_WIDTH / INNER_HEIGHT, 0.1, 100.0);
    camera.node.borrow_mut().position.set(-6.0, 5.0, 6.0);

    let mut controls = OrbitControls::new(&mut camera);
    // The canvas the example renders at, standing in for the element's
    // `clientWidth` / `clientHeight`.
    controls.set_element_size(INNER_WIDTH, INNER_HEIGHT);
    controls.target.set(0.0, 2.0, 0.0);
    controls.enable_damping = true;
    controls.update(&mut camera, None);

    let mut scene = Scene::new();
    scene.set_background(Color::from_hex(0x90d5ff));

    let timer = Timer::new();

    // `loader.loadAsync( 'models/gltf/bath_day.glb' )`, Draco-compressed. The
    // page awaits it before building the renderer; the loader here is
    // synchronous.
    let gltf = GltfLoader::load(examples_dir().join("models/gltf/bath_day.glb"))
        .expect("three-rs: bath_day.glb loads");
    let model = gltf.scene.clone();
    scene.add(&model);

    let mut mixer = AnimationMixer::new(Box::new(gltf.scene_resolver()));
    let action = mixer.clip_action(&gltf.animations[0], None, None);
    mixer.play(action);

    // renderer — built before the environment here, because the PMREM
    // conversion takes it.

    let mut renderer = Renderer::new(RendererParameters::default()).unwrap();
    renderer.set_pixel_ratio(DPR);
    renderer.set_size(INNER_WIDTH, INNER_HEIGHT);
    renderer.tone_mapping = ToneMapping::Neutral;

    //

    let env_map = UltraHdrLoader::new()
        .load(examples_dir().join("textures/equirectangular/spruit_sunrise_2k.hdr.jpg"))
        .unwrap();
    // `envMap.mapping = THREE.EquirectangularReflectionMapping`: the PMREM
    // conversion below reads it as one.
    scene.environment_rotation.y = std::f64::consts::PI * -0.5;
    let mut environment = PmremEnvironment::from_equirectangular(&env_map);
    environment.update(&mut renderer).unwrap();
    scene.environment = Some(environment.handle());

    // post processing

    let mut render_pipeline = RenderPipeline::new();
    render_pipeline.output_color_transform = false;

    // DOF uniforms

    let blur_size = uniform_value(Type::F32, vec![2.0]); // determines the kernel size of the blur
    let blur_spread = uniform_value(Type::F32, vec![4.0]); // determines how far the blur is spread
    let min_distance = uniform_value(Type::F32, vec![1.0]); // all positions at or below minDistance will be completely in focus.
    let max_distance = uniform_value(Type::F32, vec![3.0]); // all positions at or beyond maxDistance will be completely out of focus.

    let (focus_point_view_node, focus_point_view) =
        uniform_settable(Type::Vec3, vec![0.0, 0.0, 0.0]);

    // beauty and blur/out-of-focus pass

    let scene = Rc::new(RefCell::new(scene));
    let camera = Rc::new(RefCell::new(camera));
    let scene_pass = pass(scene.clone(), camera.clone());

    // `toInspector( 'Color' )` names the node for the inspector panel and
    // returns it unchanged.
    let scene_pass_color = scene_pass.node();
    let scene_pass_view_z = scene_pass.view_z_node("depth");
    let mut blur_options = BoxBlurOptions::default();
    blur_options.size = blur_size.clone();
    blur_options.separation = blur_spread.clone();
    let scene_pass_blurred = box_blur(&scene_pass.texture(), blur_options);

    // simple DOF from https://lettier.github.io/3d-game-shaders-for-beginners/depth-of-field.html

    let blur = smoothstep(
        min_distance.clone(),
        max_distance.clone(),
        scene_pass_view_z.sub(focus_point_view_node.z()).abs(),
    );
    let dof_pass = mix(scene_pass_color, scene_pass_blurred, blur);

    let output_pass = render_output(dof_pass, renderer.tone_mapping);
    let fxaa_input = convert_to_texture(output_pass);
    let fxaa = fxaa(&fxaa_input.texture());

    render_pipeline.output_node = Some(fxaa.node());

    App {
        renderer,
        scene,
        camera,
        controls,
        mixer,
        timer,
        environment,
        focus_point: Vector3::new(1.0, 1.75, -0.4),
        focus_point_view,
        blur_size,
        blur_spread,
        min_distance,
        max_distance,
        scene_pass,
        fxaa_input,
        fxaa,
        render_pipeline,
    }
}

/// The page's `animate()`.
pub fn animate(app: &mut App) {
    // `TWEEN.update()` — no tween runs without a click.

    app.controls.update(&mut app.camera.borrow_mut(), None);

    app.timer.update();

    app.mixer.update(app.timer.get_delta());

    // since the focus point is expressed in view space, it must be updated on every
    // camera change. for simplicity, do this every frame.

    {
        let mut camera = app.camera.borrow_mut();
        camera.update_matrix_world();
        let mut view = app.focus_point;
        view.apply_matrix4(&camera.matrix_world_inverse);
        app.focus_point_view.set(vec![view.x, view.y, view.z]);
    }

    app.environment.update(&mut app.renderer).unwrap();

    // `FXAANode` is not itself a `NodeUpdate` node, so `fxaa.update()` stays a
    // hand call for its `invSize` uniform, and the RTT is sized first (see
    // `webgpu_postprocessing_fxaa`).
    let (width, height) = app.renderer.drawing_buffer_size();
    app.fxaa_input.set_size(width, height);
    app.fxaa.update();
    app.render_pipeline.render(&mut app.renderer);
}

/// The page's `onWindowResize()`.
pub fn resize(app: &mut App, width: f64, height: f64) {
    let mut camera = app.camera.borrow_mut();
    camera.aspect = width / height;
    camera.update_projection_matrix();
    app.renderer.set_size(width, height);
}

/// The example's controls, for a host that has a pointer.
pub fn controls(app: &mut App) -> Option<&mut OrbitControls> {
    Some(&mut app.controls)
}

/// The controls and the camera at once, which every one of the controls'
/// event handlers needs.
pub fn controls_and_camera(
    app: &mut App,
) -> Option<(&mut OrbitControls, std::cell::RefMut<'_, PerspectiveCamera>)> {
    Some((&mut app.controls, app.camera.borrow_mut()))
}

fn main() {
    three_rs::testing::pin_time(Some(0.0));
    let mut app = init();
    println!("adapter: {:?}", app.renderer.adapter_info());
    animate(&mut app);

    let (width, height, pixels) = app.renderer.read_canvas_pixels().unwrap();
    let path = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "target/webgpu_postprocessing_dof_basic.png".to_string());
    three_rs::testing::write_png(&path, width, height, &pixels);
    println!("wrote {path} ({width}x{height})");
}
