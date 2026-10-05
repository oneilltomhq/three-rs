//! Port of `three.js/examples/webgpu_backdrop.html`, calling the three-rs API
//! in the same order the page's `init()` / `animate()` do.
//!
//! Under the e2e harness the viewport is `400 * 2` x `250 * 2` with no
//! `deviceScaleFactor`, so `window.innerWidth` / `innerHeight` /
//! `devicePixelRatio` are 800, 500 and 1.
//!
//! Michelle dances inside a ring of eight glass spheres. Each sphere is a
//! transparent `MeshStandardNodeMaterial` whose `backdropNode` reads
//! `viewportSharedTexture()` — the frame drawn so far — and filters it: a hue
//! shift, an inversion, grayscale, saturation, an overlay, two pixelations and
//! a blue channel. The spheres are in the transparent list, so each one draws
//! after Michelle and the background; and as every `viewportSharedTexture()`
//! call is its own node, each sphere's material copies the frame again before
//! it draws — so a sphere sees the spheres drawn before it.
//!
//! `GLTFLoader.load()` is asynchronous on the page, but the harness fires its
//! single RAF only once the network is idle, so Michelle is in the scene for
//! the graded frame; here the load is synchronous and the ordering is the same.
//!
//! **The graded time is zero.** `timer.getDelta()` is 0 on the harness' one
//! frame (see `webgpu_skinning`), so the mixer poses Michelle at her first
//! keyframe and the portals have not turned. `time` is 0 too, so every
//! `oscSine()` on the page is `sin( 0.75 * 2π ) * 0.5 + 0.5 = 0`: the hue
//! shift is 0, the saturated sphere's backdrop alpha is 0 (it shows its own
//! lit colour), and Michelle's `outputNode` mix picks plain `output`.
//!
//! The camera is in the scene and the spot light is its child, aimed at the
//! light's default target at the origin.
//!
//! The controls' `start` / `end` listeners stop and restart the portals'
//! rotation while dragging; no host here sends them, so `rotate` stays true.

use std::f64::consts::PI;
use std::rc::Rc;

use three_rs::addons::controls::OrbitControls;
use three_rs::animation::AnimationMixer;
use three_rs::core::{Object3D, ObjectRef};
use three_rs::geometries::sphere_geometry;
use three_rs::loaders::GltfLoader;
use three_rs::math::math_utils::deg_to_rad;
use three_rs::nodes::display::{
    viewport_safe_uv, viewport_shared_texture, viewport_shared_texture_at,
};
use three_rs::nodes::tsl::{
    blend_overlay, checker, float, grayscale, hue, osc_sine, output_property, posterize,
    saturation, screen_uv, time, uv, vec3_join,
};
use three_rs::nodes::NodeRef;
use three_rs::Timer;
use three_rs::{
    Background, Color, Mesh, MeshBasicNodeMaterial, PerspectiveCamera, Renderer,
    RendererParameters, Scene, SpotLight, ToneMapping, Vector3,
};

pub const INNER_WIDTH: f64 = 800.0;
pub const INNER_HEIGHT: f64 = 500.0;
/// `renderer.setPixelRatio( window.devicePixelRatio )`.
pub const DPR: f64 = 1.0;

pub struct App {
    pub renderer: Renderer,
    pub scene: Scene,
    pub camera: PerspectiveCamera,
    pub controls: OrbitControls,
    /// The page's module-level `portals` group.
    pub portals: ObjectRef,
    /// The page's module-level `mixer`.
    pub mixer: AnimationMixer,
    /// The page's module-level `timer`.
    pub timer: Timer,
    /// The page's module-level `rotate`: false while the controls are dragged.
    pub rotate: bool,
}

/// `oscSine()`: `oscSine( time )`.
fn osc_sine_time() -> NodeRef {
    osc_sine(time())
}

/// The eight `addBackdropSphere( backdropNode, backdropAlphaNode )` calls'
/// arguments, in the page's order. `pub` so `examples/dump_wgsl.rs` can build
/// the same materials.
pub fn backdrops() -> Vec<(NodeRef, Option<NodeRef>)> {
    vec![
        (
            hue(
                viewport_shared_texture().swizzle("bgr"),
                osc_sine_time().mul(PI),
            ),
            None,
        ),
        (viewport_shared_texture().rgb().one_minus(), None),
        (grayscale(viewport_shared_texture().rgb()), None),
        (
            saturation(viewport_shared_texture().rgb(), float(10.0)),
            Some(osc_sine_time()),
        ),
        (
            blend_overlay(viewport_shared_texture().rgb(), checker(uv().mul(10.0))),
            None,
        ),
        (
            viewport_shared_texture_at(viewport_safe_uv(screen_uv().mul(40.0).floor().div(40.0))),
            None,
        ),
        (
            viewport_shared_texture_at(viewport_safe_uv(screen_uv().mul(80.0).floor().div(80.0)))
                .add(Color::from_hex(0x0033ff)),
            None,
        ),
        (
            vec3_join(vec![
                float(0.0),
                float(0.0),
                viewport_shared_texture().swizzle("b"),
            ]),
            None,
        ),
    ]
}

/// `addBackdropSphere()`'s material: `MeshStandardNodeMaterial( { color:
/// 0x0066ff } )` with `roughnessNode = float( .2 )`, `metalnessNode =
/// float( 0 )`, the backdrop pair, and `transparent = true`.
pub fn backdrop_material(
    backdrop_node: NodeRef,
    backdrop_alpha_node: Option<NodeRef>,
) -> MeshBasicNodeMaterial {
    let mut material = MeshBasicNodeMaterial::standard(Color::from_hex(0x0066ff), 1.0, 0.0);
    material.roughness_node = Some(float(0.2));
    material.metalness_node = Some(float(0.0));
    material.backdrop_node = Some(backdrop_node);
    material.backdrop_alpha_node = backdrop_alpha_node;
    material.transparent = true;
    material
}

/// Michelle's `material.outputNode = oscSine( time.mul( .1 ) ).mix( output,
/// posterize( output.add( .1 ), 4 ).mul( 2 ) )`.
pub fn michelle_output_node() -> NodeRef {
    osc_sine(time().mul(0.1)).mix(
        output_property(),
        posterize(output_property().add(0.1), float(4.0)).mul(2.0),
    )
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

    let timer = Timer::new();

    // lights
    //
    // `light.power = 2000` is `SpotLight`'s setter for
    // `intensity = power / PI`; the division is here.
    let light = SpotLight::new(Color::from_hex(0xffffff), 2000.0 / PI);
    camera.node.add(&light);
    scene.add(&camera.node);

    let gltf = GltfLoader::load(three.join("examples/models/gltf/Michelle.glb"))
        .expect("three-rs: Michelle.glb loads");

    let mut mixer = AnimationMixer::new(Box::new(gltf.scene_resolver()));

    // `object.children[ 0 ].children[ 0 ].material`.
    let body = gltf.scene.children()[0].children()[0].clone();
    {
        let mut body = body.borrow_mut();
        let mesh = body
            .mesh_mut()
            .expect("Michelle's first child's child is a mesh");
        let material = mesh.material.as_mut().expect("a mesh material");
        material.output_node = Some(michelle_output_node());
    }

    let action = mixer.clip_action(&gltf.animations[0], None, None);
    mixer.play(action);

    scene.add(&gltf.scene);

    // portals

    let geometry = Rc::new(sphere_geometry(0.3, 32, 16));

    let portals = Object3D::default().into_node();
    scene.add(&portals);

    for (backdrop_node, backdrop_alpha_node) in backdrops() {
        let distance = 1.0;
        let id = portals.children().len();
        let rotation = deg_to_rad(id as f64 * 45.0);

        let material = backdrop_material(backdrop_node, backdrop_alpha_node);

        let mesh = Mesh::new(geometry.clone(), material);
        mesh.borrow_mut()
            .position
            .set(rotation.cos() * distance, 1.0, rotation.sin() * distance);

        portals.add(&mesh);
    }

    // renderer

    let mut parameters = RendererParameters::default();
    parameters.antialias = false;
    let mut renderer = Renderer::new(parameters).unwrap();
    renderer.set_pixel_ratio(DPR);
    renderer.set_size(INNER_WIDTH, INNER_HEIGHT);
    renderer.tone_mapping = ToneMapping::Neutral;
    renderer.tone_mapping_exposure = 0.3;

    let mut controls = OrbitControls::new(&mut camera);
    // The canvas the example renders at, standing in for the element's
    // `clientWidth` / `clientHeight`.
    controls.set_element_size(INNER_WIDTH, INNER_HEIGHT);
    controls.target.set(0.0, 1.0, 0.0);
    controls.update(&mut camera, None);

    App {
        renderer,
        scene,
        camera,
        controls,
        portals,
        mixer,
        timer,
        rotate: true,
    }
}

/// The page's `animate()`. `timer.getDelta()` is 0 on the graded frame and on
/// the steady frames, so the pose and the ring do not move.
pub fn animate(app: &mut App) {
    app.timer.update();

    let delta = app.timer.get_delta();

    app.mixer.update(delta);

    if app.rotate {
        let mut portals = app.portals.borrow_mut();
        let r = portals.rotation;
        portals.set_rotation(r.x, r.y + delta * 0.5, r.z);
    }

    app.renderer.render(&mut app.scene, &mut app.camera);
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

/// The example's controls, for a host that has a pointer.
pub fn controls(app: &mut App) -> Option<&mut OrbitControls> {
    Some(&mut app.controls)
}

/// The controls and the camera at once, which every one of the controls'
/// event handlers needs: the JS holds the camera as `this.object` and Rust
/// cannot, so `pointer_move` and the rest take it as an argument.
pub fn controls_and_camera(app: &mut App) -> Option<(&mut OrbitControls, &mut PerspectiveCamera)> {
    Some((&mut app.controls, &mut app.camera))
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
        .unwrap_or_else(|| "target/webgpu_backdrop.png".to_string());
    three_rs::testing::write_png(&path, width, height, &pixels);
    println!("wrote {path} ({width}x{height})");
}
