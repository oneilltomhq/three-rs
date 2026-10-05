//! Port of `three.js/examples/webgpu_postprocessing_motion_blur.html`, calling
//! the three-rs API in the same order the page's `init()` / `animate()` do.
//!
//! The scene is drawn once into two attachments, the lit colour and the
//! per-pixel screen-space motion from the `velocity` node, and the second one
//! steers a 16-tap blur of the first.
//!
//! Under the e2e harness the viewport is `400 * 2` x `250 * 2` with no
//! `deviceScaleFactor`, so `window.innerWidth` / `innerHeight` /
//! `devicePixelRatio` are 800, 500 and 1.
//!
//! **What the graded frame proves.** The harness renders one frame at t = 0,
//! and on the first frame `VelocityNode` seeds every previous matrix (model,
//! view, projection, and the skeleton's bones) with the current one: the
//! velocity attachment is zero everywhere. The graded image is therefore the
//! beauty pass, brightened by the blur's `17 / 16` (see
//! [`motion_blur`]) and darkened by the vignette. That the velocity MRT builds,
//! binds and composes is what the rung checks; that it carries motion is what
//! `tests/velocity_frames.rs` checks, over two frames.
//!
//! Three things the page does that are easy to lose:
//!
//! * **The Xbot casts a shadow while it is skinned.** The shadow pass skins
//!   its casters, so the floor shows the posed figure, not the bind pose.
//! * **`floorNormal` is loaded and never used.** It is not loaded here.
//! * **`controls.update()` runs in `init()` and in every `animate()`**, with
//!   auto-rotation and damping on, so the graded camera has moved by two
//!   null-delta steps from `( 0, 1.5, 4.5 )`.

use std::cell::RefCell;
use std::f64::consts::PI;
use std::rc::Rc;

use three_rs::addons::controls::OrbitControls;
use three_rs::animation::AnimationMixer;
use three_rs::core::ObjectRef;
use three_rs::geometries::{box_geometry, torus_geometry};
use three_rs::loaders::GltfLoader;
use three_rs::materials::Side;
use three_rs::nodes::display::motion_blur;
use three_rs::nodes::mrt;
use three_rs::nodes::tsl::{
    distance, float, output_property, screen_uv, texture_uv, uniform_value, uv, vec4_join,
};
use three_rs::nodes::velocity::velocity;
use three_rs::nodes::{NodeRef, Type};
use three_rs::textures::Wrapping;
use three_rs::Timer;
use three_rs::{
    pass, Color, ColorSpace, DirectionalLight, Fog, HemisphereLight, Mesh, MeshBasicNodeMaterial,
    MeshPhongNodeMaterial, PassNode, PerspectiveCamera, RenderPipeline, Renderer,
    RendererParameters, Scene, TextureLoader,
};

pub const INNER_WIDTH: f64 = 800.0;
pub const INNER_HEIGHT: f64 = 500.0;
/// `renderer.setPixelRatio( window.devicePixelRatio )`.
pub const DPR: f64 = 1.0;

/// The page's `params.speed`.
const SPEED: f64 = 1.0;

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
    pub box_left: ObjectRef,
    pub box_right: ObjectRef,
    /// The page's module-level `mixer`.
    pub mixer: AnimationMixer,
    /// The page's module-level `timer`.
    pub timer: Timer,
    /// `uniform( 1 )`, the GUI's "blur amount".
    pub blur_amount: NodeRef,
    pub scene_pass: PassNode,
    pub render_pipeline: RenderPipeline,
}

pub fn init() -> App {
    let mut camera = PerspectiveCamera::new(50.0, INNER_WIDTH / INNER_HEIGHT, 0.25, 30.0);
    camera.node.borrow_mut().position.set(0.0, 1.5, 4.5);

    let mut scene = Scene::new();
    scene.fog = Some(Fog::new(Color::from_hex(0x0487e2), 7.0, 25.0).into());

    let sun_light = DirectionalLight::new(Color::from_hex(0xffe499), 5.0);
    {
        let mut object = sun_light.borrow_mut();
        object.cast_shadow = true;
        let shadow = object.light_mut().unwrap().shadow.as_mut().unwrap();
        shadow.camera.set_near(0.1);
        shadow.camera.set_far(10.0);
        shadow.camera.set_bounds(-2.0, 2.0, 2.0, -2.0);
        shadow.map_size.x = 1024.0;
        shadow.map_size.y = 1024.0;
        object.position.set(4.0, 4.0, 2.0);
    }

    let water_ambient_light =
        HemisphereLight::new(Color::from_hex(0x333366), Color::from_hex(0x74ccf4), 5.0);
    let sky_ambient_light =
        HemisphereLight::new(Color::from_hex(0x74ccf4), Color::from_hex(0x000000), 1.0);

    scene.add(&sun_light);
    scene.add(&sky_ambient_light);
    scene.add(&water_ambient_light);

    let timer = Timer::new();

    // animated model
    //
    // `loader.load( 'models/gltf/Xbot.glb', … )` is asynchronous on the page,
    // but the harness fires its RAF only once the network is idle, so the
    // model is in the scene for the graded frame. Its callback adds it after
    // the floor, walls and tori, which are added synchronously below; the
    // scene order is kept by adding it last.
    let gltf = GltfLoader::load(examples_dir().join("models/gltf/Xbot.glb"))
        .expect("three-rs: Xbot.glb loads");
    let model = gltf.scene.clone();
    model.borrow_mut().set_rotation(0.0, PI / 2.0, 0.0);
    model.traverse(&mut |child| {
        let mut object = child.borrow_mut();
        if object.is_mesh() {
            object.cast_shadow = true;
            object.receive_shadow = true;
        }
    });
    let mut mixer = AnimationMixer::new(Box::new(gltf.scene_resolver()));
    let action = mixer.clip_action(&gltf.animations[3], None, None);
    mixer.play(action);

    // textures

    let texture_loader = TextureLoader::new();

    let floor_color = texture_loader
        .load(examples_dir().join("textures/floors/FloorsCheckerboard_S_Diffuse.jpg"))
        .unwrap();
    floor_color.set_wrapping(Wrapping::Repeat, Wrapping::Repeat);
    floor_color.set_color_space(ColorSpace::Srgb);

    // floor

    // `texture( floorColor, uv().mul( 5 ) )`, shared by the floor and the
    // walls.
    let floor_color_node = texture_uv(&floor_color, uv().mul(5.0));

    let mut floor_material = MeshPhongNodeMaterial::phong(Color::from_hex(0xffffff));
    floor_material.color_node = Some(floor_color_node.clone());

    let floor = Mesh::new(
        Rc::new(box_geometry(15.0, 0.001, 15.0, 1, 1, 1)),
        floor_material,
    );
    {
        let mut object = floor.borrow_mut();
        object.receive_shadow = true;
        object.position.set(0.0, 0.0, 0.0);
    }
    scene.add(&floor);

    let mut walls_material = MeshPhongNodeMaterial::phong(Color::from_hex(0xffffff));
    walls_material.color_node = Some(floor_color_node);
    walls_material.side = Side::Back;
    let walls = Mesh::new(
        Rc::new(box_geometry(15.0, 15.0, 15.0, 1, 1, 1)),
        walls_material,
    );
    scene.add(&walls);

    let map = TextureLoader::new()
        .load(examples_dir().join("textures/uv_grid_opengl.jpg"))
        .unwrap();
    map.set_color_space(ColorSpace::Srgb);

    // `new THREE.TorusGeometry( .8 )`: tube 0.4, 12 radial and 48 tubular
    // segments.
    let geometry = Rc::new(torus_geometry(0.8, 0.4, 12, 48));
    let mut material = MeshBasicNodeMaterial::new();
    material.map = Some(map);

    let box_right = Mesh::new(geometry.clone(), material.clone());
    box_right.borrow_mut().position.set(3.5, 1.5, -4.0);
    scene.add(&box_right);

    let box_left = Mesh::new(geometry, material);
    box_left.borrow_mut().position.set(-3.5, 1.5, -4.0);
    scene.add(&box_left);

    scene.add(&model);

    // renderer

    let mut renderer = Renderer::new(RendererParameters::default()).unwrap();
    renderer.set_pixel_ratio(DPR);
    renderer.set_size(INNER_WIDTH, INNER_HEIGHT);
    renderer.shadow_map_enabled = true;

    let mut controls = OrbitControls::new(&mut camera);
    // The canvas the example renders at, standing in for the element's
    // `clientWidth` / `clientHeight`.
    controls.set_element_size(INNER_WIDTH, INNER_HEIGHT);
    controls.min_distance = 1.0;
    controls.max_distance = 10.0;
    controls.max_polar_angle = PI / 2.0;
    controls.auto_rotate = true;
    controls.auto_rotate_speed = 1.0;
    controls.target.set(0.0, 1.0, 0.0);
    controls.enable_damping = true;
    controls.damping_factor = 0.05;
    controls.update(&mut camera, None);

    // post-processing

    // `uniform( 1 )`.
    let blur_amount = uniform_value(Type::F32, vec![1.0]);

    let scene = Rc::new(RefCell::new(scene));
    let camera = Rc::new(RefCell::new(camera));
    let scene_pass = pass(scene.clone(), camera.clone());

    scene_pass.set_mrt(mrt(vec![
        ("output", output_property()),
        ("velocity", velocity()),
    ]));

    // `toInspector( … )` names a node for the inspector panel and returns it
    // unchanged.
    let beauty = scene_pass.texture();
    let vel = scene_pass.texture_node("velocity").mul(blur_amount.clone());

    let m_blur = motion_blur(&beauty, vel, 16);

    // `screenUV.distance( .5 )`: the scalar is splatted to a `vec2`.
    let vignette = distance(screen_uv(), float(0.5))
        .remap(0.6, 1.0, 0.0, 1.0)
        .mul(2.0)
        .clamp(0.0, 1.0)
        .one_minus();

    let mut render_pipeline = RenderPipeline::new();
    render_pipeline.output_node = Some(vec4_join(vec![m_blur.mul(vignette).xyz(), m_blur.w()]));

    App {
        renderer,
        scene,
        camera,
        controls,
        box_left,
        box_right,
        mixer,
        timer,
        blur_amount,
        scene_pass,
        render_pipeline,
    }
}

/// The page's `animate()`. On the graded frame `timer.getDelta()` is 0, so
/// the tori and the mixer stand still; the controls still take their
/// null-delta auto-rotation step.
pub fn animate(app: &mut App) {
    app.timer.update();

    app.controls.update(&mut app.camera.borrow_mut(), None);

    let delta = app.timer.get_delta();
    let speed = SPEED;

    {
        let mut box_right = app.box_right.borrow_mut();
        let rotation = box_right.rotation;
        box_right.set_rotation(rotation.x, rotation.y + delta * 4.0 * speed, rotation.z);
    }
    let scale = 1.0 + (app.timer.get_elapsed() * 10.0 * speed).sin() * 0.2;
    app.box_left.borrow_mut().scale.set(scale, scale, scale);

    app.mixer.update(delta * speed);

    app.render_pipeline.render(&mut app.renderer);
}

/// The page's `onWindowResize()`.
///
/// The rung harness never calls this — the graded frame is always
/// 800 x 500 — but the viewer and the browser shell do, so the example
/// owns its own reaction to a resized canvas instead of the host
/// guessing at one.
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
        .unwrap_or_else(|| "target/webgpu_postprocessing_motion_blur.png".to_string());
    three_rs::testing::write_png(&path, width, height, &pixels);
    println!("wrote {path} ({width}x{height})");
}
