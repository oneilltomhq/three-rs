//! Port of `three.js/examples/webgpu_portal.html`, calling the three-rs API
//! in the same order the page's `init()` / `animate()` do.
//!
//! A plane in the main scene has as its colour a `pass()` of a second scene,
//! read at `screenUV` rather than at the plane's own uv, so the plane shows
//! the second scene exactly where it would be on screen: a portal. Both
//! scenes hold an animated `Xbot.glb`; the portal's wears a fractal-noise
//! wireframe.
//!
//! Under the e2e harness the viewport is `400 * 2` x `250 * 2` with no
//! `deviceScaleFactor`, so `window.innerWidth` / `innerHeight` /
//! `devicePixelRatio` are 800, 500 and 1.
//!
//! Three things the page does that are easy to lose:
//!
//! * **The pass is tapped at `screenUV`, not `uv()`.** The page spells it
//!   `pass( scenePortal, camera ).context( { getUV: () => screenUV } )`; the
//!   port reads a tap's uv when the tap is made, not at build (#352), so the
//!   spelling here is [`PassNode::sample`].
//! * **The portal scene is rendered with the camera the main scene is being
//!   drawn with.** The nested render runs from inside the main one, so the
//!   main render passes the camera's `Rc<RefCell<_>>` to [`Renderer::render`]
//!   rather than a `&mut`, or the pass would find it mutably borrowed.
//! * **`gltf.scene.clone()` is `Object3D.clone( true )`**, which the port's
//!   `Object3D` clone is not (#300). The model is loaded twice instead; the
//!   second load is the page's clone, down to the cloned materials.

use std::cell::RefCell;
use std::rc::Rc;

use three_rs::addons::controls::OrbitControls;
use three_rs::animation::AnimationMixer;
use three_rs::core::ObjectRef;
use three_rs::geometries::plane_geometry;
use three_rs::loaders::GltfLoader;
use three_rs::materials::{Side, ToneMapping};
use three_rs::math::Vector3;
use three_rs::nodes::materialx::{mx_fractal_noise_vec3, mx_worley_noise_float};
use three_rs::nodes::tsl::{
    call, color, distance, float, inline_fn, normal_world, screen_uv, time, uv, vec2_join,
};
use three_rs::nodes::Type;
use three_rs::Timer;
use three_rs::{
    pass, Background, Color, HemisphereLight, Mesh, MeshBasicNodeMaterial, PassNode,
    PerspectiveCamera, PointLight, Renderer, RendererParameters, Scene,
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
    /// The page's `sceneMain`.
    pub scene_main: Scene,
    /// The page's `scenePortal`, shared with the portal pass, which renders
    /// it from `updateBefore()`.
    pub scene_portal: Rc<RefCell<Scene>>,
    /// Shared with the portal pass: both scenes are drawn through it.
    pub camera: Rc<RefCell<PerspectiveCamera>>,
    /// The page's `controls`.
    pub controls: OrbitControls,
    /// The page's module-level `mixers`, one per Xbot.
    pub mixers: Vec<AnimationMixer>,
    /// The page's module-level `timer`.
    pub timer: Timer,
    /// `pass( scenePortal, camera )`.
    pub portal_pass: PassNode,
}

pub fn init() -> App {
    //

    let mut scene_main = Scene::new();
    // `sceneMain.background = new THREE.Color( 0x222222 )` is set and then
    // shadowed: `backgroundNode` wins over `background` in
    // `Background.update()`, so only the node is kept.
    //
    // Both background nodes read `normalWorld`. Three resolves it when the
    // skybox material builds, where `BackSide` negates it; the port resolves
    // it where the node is made (#352), so each is deferred in an
    // argument-less `Fn` that `backgroundNode` runs inside that build.
    scene_main.background = Some(Background::Node(call(
        &inline_fn(0, Type::Vec3, |_| {
            normal_world()
                .y()
                .mix(Color::from_hex(0x0066ff), Color::from_hex(0xff0066))
        }),
        Vec::new(),
    )));

    let mut scene_portal = Scene::new();
    // `mx_worley_noise_float( normalWorld.mul( 20 ).add( vec2( 0, time.oneMinus() ) ) )`
    // with the default jitter 1 and style 0.
    scene_portal.background = Some(Background::Node(call(
        &inline_fn(0, Type::Vec3, |_| {
            mx_worley_noise_float(
                normal_world()
                    .mul(20.0)
                    .add(vec2_join(vec![float(0.0), time().one_minus()])),
                1.0,
                0.0,
            )
            .mul(color(0x0066ff))
        }),
        Vec::new(),
    )));
    scene_portal.node.borrow_mut().name = "Portal Scene".to_string();

    //

    let mut camera = PerspectiveCamera::new(50.0, INNER_WIDTH / INNER_HEIGHT, 0.01, 30.0);
    camera.node.borrow_mut().position.set(2.5, 1.0, 3.0);
    camera.node.borrow_mut().position.multiply_scalar(0.8);
    camera.look_at(&Vector3::new(0.0, 1.0, 0.0));

    let timer = Timer::new();

    // lights

    let light = PointLight::new(Color::from_hex(0xffffff), 1.0, 0.0);
    light.borrow_mut().position.set(0.0, 1.0, 5.0);
    light.borrow_mut().light_mut().unwrap().set_power(17000.0);

    scene_main.add(&HemisphereLight::new(
        Color::from_hex(0xff0066),
        Color::from_hex(0x0066ff),
        7.0,
    ));
    scene_main.add(&light);
    // `light.clone()`: a light has no children, so `Object3D.clone()` is the
    // whole of it, power included.
    let light_portal = ObjectRef::new(light.borrow().clone());
    scene_portal.add(&light_portal);

    // models
    //
    // `loader.load( 'models/gltf/Xbot.glb', … )` is asynchronous on the page,
    // but the harness fires its RAF only once the network is idle, so both
    // models are in their scenes for the graded frame. The callback adds
    // them after the portal plane is added synchronously below; the scene
    // order is kept by adding the main one last.
    let mut mixers = Vec::new();

    // `const colorNode = mx_fractal_noise_vec3( uv().mul( 20 ).add( time ) )`
    // with the default octaves 3, lacunarity 2, diminish .5, amplitude 1.
    let color_node = mx_fractal_noise_vec3(uv().mul(20.0).add(time()), 3.0, 2.0, 0.5, 1.0);

    // `createModel()` for the first mixer: `gltf.scene` itself.
    let gltf_main = GltfLoader::load(examples_dir().join("models/gltf/Xbot.glb"))
        .expect("three-rs: Xbot.glb loads");
    let model_main = gltf_main.scene.clone();
    let mut mixer = AnimationMixer::new(Box::new(gltf_main.scene_resolver()));
    let action = mixer.clip_action(&gltf_main.animations[6], None, None);
    mixer.play(action);
    mixers.push(mixer);

    // `createModel( colorNode )` for the second: `gltf.scene.clone()`. The
    // port's `Object3D::clone` keeps no children (#300), so the page's clone
    // is a second load, which the page's `applyFX` then dresses: the first two
    // meshes under the armature get a cloned material with the noise as its
    // colour, drawn as a wireframe.
    let gltf_portal = GltfLoader::load(examples_dir().join("models/gltf/Xbot.glb"))
        .expect("three-rs: Xbot.glb loads");
    let model_portal = gltf_portal.scene.clone();
    {
        let object = model_portal.borrow();
        let children = &object.children[0].borrow().children;
        let apply_fx = |index: usize| {
            let mut child = children[index].borrow_mut();
            let mesh = child.mesh_mut().expect("three-rs: Xbot's limbs are meshes");
            let mut material = mesh
                .material
                .clone()
                .expect("three-rs: Xbot's limbs have a material");
            material.color_node = Some(color_node.clone());
            material.wireframe = true;
            mesh.material = Some(material);
        };
        apply_fx(0);
        apply_fx(1);
    }
    let mut mixer = AnimationMixer::new(Box::new(gltf_portal.scene_resolver()));
    let action = mixer.clip_action(&gltf_portal.animations[6], None, None);
    mixer.play(action);
    mixers.push(mixer);

    // portal

    let geometry = Rc::new(plane_geometry(1.7, 2.0, 1, 1));

    let scene_portal = Rc::new(RefCell::new(scene_portal));
    let camera = Rc::new(RefCell::new(camera));
    let portal_pass = pass(scene_portal.clone(), camera.clone());

    let mut material = MeshBasicNodeMaterial::new();
    // `pass( scenePortal, camera ).context( { getUV: () => screenUV } )`.
    material.color_node = Some(portal_pass.sample(screen_uv()));
    // `uv().distance( .5 ).remapClamp( .3, .5 ).oneMinus()`: the scalar is
    // splatted to a `vec2`, and `remapClamp`'s output range defaults to
    // `[ 0, 1 ]`.
    material.opacity_node = Some(
        distance(uv(), float(0.5))
            .remap_clamp(0.3, 0.5, 0.0, 1.0)
            .one_minus(),
    );
    material.side = Side::Double;
    material.transparent = true;

    let plane = Mesh::new(geometry, material);
    plane.borrow_mut().position.set(0.0, 1.0, 0.8);
    scene_main.add(&plane);

    // model portal

    scene_main.add(&model_main);
    scene_portal.borrow_mut().add(&model_portal);

    // renderer

    // `new THREE.WebGPURenderer( { antialias: true } )`.
    let mut parameters = RendererParameters::default();
    parameters.antialias = true;
    let mut renderer = Renderer::new(parameters).unwrap();
    renderer.set_pixel_ratio(DPR);
    renderer.set_size(INNER_WIDTH, INNER_HEIGHT);
    renderer.tone_mapping = ToneMapping::Linear;
    renderer.tone_mapping_exposure = 0.15;

    //

    let mut controls = OrbitControls::new(&mut camera.borrow_mut());
    // The canvas the example renders at, standing in for the element's
    // `clientWidth` / `clientHeight`.
    controls.set_element_size(INNER_WIDTH, INNER_HEIGHT);
    controls.target.set(0.0, 1.0, 0.0);
    controls.update(&mut camera.borrow_mut(), None);

    App {
        renderer,
        scene_main,
        scene_portal,
        camera,
        controls,
        mixers,
        timer,
        portal_pass,
    }
}

/// The page's `animate()`. On the graded frame `timer.getDelta()` is 0, so
/// both mixers hold the clip's first pose.
pub fn animate(app: &mut App) {
    app.timer.update();

    let delta = app.timer.get_delta();

    for mixer in &mut app.mixers {
        mixer.update(delta);
    }

    // `renderer.render( sceneMain, camera )`. The portal pass renders
    // `scenePortal` through the same camera from inside this render, so the
    // camera goes in as the `Rc<RefCell<_>>` the pass shares, not as `&mut`.
    app.renderer.render(&app.scene_main, &app.camera);
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
        .unwrap_or_else(|| "target/webgpu_portal.png".to_string());
    three_rs::testing::write_png(&path, width, height, &pixels);
    println!("wrote {path} ({width}x{height})");
}
