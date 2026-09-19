//! Port of `three.js/examples/webgpu_skinning.html`, calling the three-rs API
//! in the same order the page's `init()` / `animate()` do.
//!
//! Under the e2e harness the viewport is `400 * 2` x `250 * 2` with no
//! `deviceScaleFactor`, so `window.innerWidth` / `innerHeight` /
//! `devicePixelRatio` are 800, 500 and 1.
//!
//! `GLTFLoader.load()` is asynchronous on the page, but the harness fires its
//! single RAF only once the network is idle, so Michelle is in the scene for
//! the graded frame; here the load is synchronous and the ordering is the same.
//!
//! **The graded time is zero.** The page drives the mixer from a `Timer` that
//! is connected to `document` and updated inside the RAF; on the one frame the
//! harness renders, `timer.getDelta()` is the delta of the very first update,
//! which is 0. So `mixer.update( 0 )` is the pose that is graded — *not* the
//! bind pose: `SambaDance`'s first keyframe is at 0.0333 s and
//! `LinearInterpolant` clamps below it, so t = 0 is that first keyframe.
//!
//! The camera is *in* the scene (`scene.add( camera )`) and the point light is
//! a child of the camera, so the light's world matrix is the camera's.
//!
//! The `resize` listener does not fire.

use std::f64::consts::PI;

use three_rs::animation::AnimationMixer;
use three_rs::loaders::GLTFLoader;
use three_rs::nodes::tsl::screen_uv;
use three_rs::{
    AmbientLight, Background, Color, PerspectiveCamera, PointLight, Renderer, RendererParameters,
    Scene, ToneMapping, Vector3,
};

pub const INNER_WIDTH: f64 = 800.0;
pub const INNER_HEIGHT: f64 = 500.0;
/// `renderer.setPixelRatio( window.devicePixelRatio )`.
pub const DPR: f64 = 1.0;

pub struct App {
    pub renderer: Renderer,
    pub scene: Scene,
    pub camera: PerspectiveCamera,
    /// The page's module-level `mixer`.
    pub mixer: AnimationMixer,
}

pub fn init() -> App {
    let three = three_rs::testing::three_js_dir();

    let mut camera = PerspectiveCamera::new(50.0, INNER_WIDTH / INNER_HEIGHT, 0.01, 100.0);
    camera.node.borrow_mut().position.set(1.0, 2.0, 3.0);

    let mut scene = Scene::new();
    // `scene.backgroundNode = screenUV.y.mix( color( 0x66bbff ), color( 0x4466ff ) )`
    scene.background = Some(Background::Node(
        screen_uv()
            .y()
            .mix(Color::from_hex(0x66bbff), Color::from_hex(0x4466ff)),
    ));
    camera.look_at(&Vector3::new(0.0, 1.0, 0.0));

    // lights
    //
    // `light.power = 2500` is `PointLight`'s setter for
    // `intensity = power / ( 4 * PI )`; the port has the field, not the
    // setter, so the division is here.
    let light = PointLight::new(Color::from_hex(0xffffff), 2500.0 / (4.0 * PI), 100.0);
    camera.node.add(&light);
    scene.add(&camera.node);

    let ambient = AmbientLight::new(Color::from_hex(0x4466ff), 1.0);
    scene.add(&ambient);

    let gltf = GLTFLoader::load(three.join("examples/models/gltf/Michelle.glb"))
        .expect("three-rs: Michelle.glb loads");

    let mut mixer = AnimationMixer::new(Box::new(gltf.scene_resolver()));
    let action = mixer.clip_action(&gltf.animations[0], None, None);
    mixer.play(action);

    scene.add(&gltf.scene);

    // renderer
    let mut renderer = Renderer::new(RendererParameters { antialias: true }).unwrap();
    renderer.set_pixel_ratio(DPR);
    renderer.set_size(INNER_WIDTH, INNER_HEIGHT);
    renderer.tone_mapping = ToneMapping::Linear;
    renderer.tone_mapping_exposure = 0.4;

    App {
        renderer,
        scene,
        camera,
        mixer,
    }
}

/// The page's animation loop. `timer.getDelta()` is 0 on the graded frame (see
/// the module docs), and the steady frames repeat that same zero, so every
/// frame draws the same pose — which is what `[ "", "" ]` in
/// `steady_frame_builds_nothing` means by "nothing is done to the scene".
pub fn animate(app: &mut App) {
    app.mixer.update(0.0);
    app.renderer.render(&mut app.scene, &mut app.camera);
}

fn main() {
    let mut app = init();
    println!("adapter: {:?}", app.renderer.adapter_info());
    animate(&mut app);

    let (width, height, pixels) = app.renderer.read_canvas_pixels().unwrap();
    let path = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "target/webgpu_skinning.png".to_string());
    three_rs::testing::write_png(&path, width, height, &pixels);
    println!("wrote {path} ({width}x{height})");
}
