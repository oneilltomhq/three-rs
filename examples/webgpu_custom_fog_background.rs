//! Port of `three.js/examples/webgpu_custom_fog_background.html`, calling the
//! three-rs API in the same order the page's `init()` and its two nested loader
//! callbacks do.
//!
//! Under the e2e harness the viewport is `400 * 2` x `250 * 2` with no
//! `deviceScaleFactor`, so `window.innerWidth` / `innerHeight` /
//! `devicePixelRatio` are 800, 500 and 1. The page draws from `Math.random()`
//! not at all and reads no clock, so the graded frame is the scene exactly as
//! `init()` built it.
//!
//! # What this page is
//!
//! It is `webgpu_loader_gltf`'s scene — DamagedHelmet under the UltraHDR
//! `royal_esplanade` map as `scene.environment` — with two things taken away
//! and one put back:
//!
//! * **No `scene.background`.** The helmet is the only thing drawn; every
//!   other pixel of the scene pass is the clear, `( 0, 0, 0, 0 )`, at depth
//!   1.0. That is the point: the fog *is* the background, and it reaches the
//!   empty pixels because a cleared depth of 1 converts to a view z of −`far`,
//!   which `smoothstep( 2.7, 4 )` saturates at 1.
//! * **No renderer tone mapping.** `renderer.toneMapping = NoToneMapping`, so
//!   the scene pass writes linear values into its `rgba16float` target and the
//!   composite applies ACES itself — to the *scene* only, never to the fog
//!   colour, which is a plain sRGB `0x4080cc` converted to the working space
//!   once at build time. Tone mapping the fog too would wash it out.
//! * **`RenderPipeline.outputColorTransform` stays `true`.** With the
//!   renderer's tone mapping off, `renderOutput()` around the composite is
//!   only the alpha clamp, the unpremultiply, the sRGB OETF and the
//!   premultiply back — no second tone map.
//!
//! # `pass.getViewZNode()`
//!
//! `scenePass.getViewZNode()` binds the pass's **depth attachment** into the
//! composite shader and converts it with `perspectiveDepthToViewZ( depth,
//! cameraNear, cameraFar )`. Two things fall out of that and both are visible
//! in three's dump (`docs/nodes.md` §24):
//!
//! * the page is `antialias: true`, so the pass target is 4×MSAA and its depth
//!   attachment is never resolved — the binding is
//!   `texture_depth_multisampled_2d` and the read is `textureLoad( …, 0 )`,
//!   sample zero of the fragment;
//! * `cameraNear` / `cameraFar` are `uniform( 0 )`s the pass writes from its
//!   camera each frame, so the composite reads `object.nodeUniform1` /
//!   `object.nodeUniform2` rather than the camera's own uniform block.
//!
//! `rangeFogFactor( 2.7, 4 ).context( { getViewZ: () => scenePassViewZ } )` is
//! that node fed to the fog factor in place of `positionView.z` — which is
//! what a fragment shader running over a full-screen quad has no useful
//! version of. The port passes it as an argument
//! ([`range_fog_factor_with_view_z`]); `docs/nodes.md` §24 says why.
//!
//! [`range_fog_factor_with_view_z`]: three_rs::nodes::tsl::range_fog_factor_with_view_z

use three_rs::addons::controls::OrbitControls;
use three_rs::loaders::{GLTFLoader, UltraHdrLoader};
use three_rs::materials::{tone_mapping_node, ToneMapping};
use three_rs::nodes::pmrem_node::PmremEnvironment;
use three_rs::nodes::tsl::{float, range_fog_factor_with_view_z};
use three_rs::{
    Color, PassNode, PerspectiveCamera, RenderPipeline, Renderer, RendererParameters, Scene,
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
    pub scene: Scene,
    pub camera: PerspectiveCamera,
    pub environment: PmremEnvironment,
    pub scene_pass: PassNode,
    pub render_pipeline: RenderPipeline,
    /// The page's `controls`.
    pub controls: OrbitControls,
}

pub fn init() -> App {
    // `new THREE.PerspectiveCamera( 45, window.innerWidth / window.innerHeight,
    // 0.25, 20 ); camera.position.set( - 1.8, 0.6, 2.7 )`.
    let mut camera = PerspectiveCamera::new(45.0, INNER_WIDTH / INNER_HEIGHT, 0.25, 20.0);
    camera.node.borrow_mut().position.set(-1.8, 0.6, 2.7);

    let mut scene = Scene::new();

    // `new THREE.WebGPURenderer( { antialias: true } )`. The `antialias` is
    // what makes the pass target 4×MSAA and so its depth attachment
    // multisampled.
    let mut renderer = Renderer::new(RendererParameters { antialias: true }).unwrap();
    renderer.set_pixel_ratio(DPR);
    renderer.set_size(INNER_WIDTH, INNER_HEIGHT);
    // `renderer.toneMapping = THREE.NoToneMapping` — the composite tone maps
    // the scene pass instead.
    renderer.tone_mapping = ToneMapping::None;

    // post processing

    // `const scenePass = pass( scene, camera )`.
    let scene_pass = PassNode::new();
    // `const scenePassViewZ = scenePass.getViewZNode()`.
    let scene_pass_view_z = scene_pass.view_z_node("depth");

    // `const fogColor = color( 0x4080cc ); // in sRGB color space`.
    let fog_color = Color::from_hex(0x4080cc);

    // `const fogFactor = rangeFogFactor( 2.7, 4 ).context( { getViewZ: () =>
    // scenePassViewZ } )` — equivalent to `scene.fog = new THREE.Fog(
    // 0x4080cc, 2.7, 4 )`, but evaluated over the composite quad.
    let fog_factor = range_fog_factor_with_view_z(float(2.7), float(4.0), scene_pass_view_z);

    // `const scenePassTM = scenePass.toneMapping( THREE.ACESFilmicToneMapping, 1 )`.
    let scene_pass_tm = tone_mapping_node(ToneMapping::AcesFilmic, float(1.0), scene_pass.node());

    // `const compose = fogFactor.mix( scenePassTM, fogColor )`.
    let compose = fog_factor.mix(scene_pass_tm, fog_color);

    let mut render_pipeline = RenderPipeline::new();
    // `renderPipeline.outputColorTransform = true` — the default; with
    // `NoToneMapping` on the renderer it is the colour-space transform alone.
    render_pipeline.output_color_transform = true;
    render_pipeline.output_node = Some(compose);

    //

    // `new UltraHDRLoader().setPath( 'textures/equirectangular/' ).load(
    // 'royal_esplanade_2k.hdr.jpg', … )`. The loader resolves synchronously
    // here, so the callback's body is written inline.
    let texture = UltraHdrLoader::new()
        .load(examples_dir().join("textures/equirectangular/royal_esplanade_2k.hdr.jpg"))
        .unwrap();

    // `texture.mapping = THREE.EquirectangularReflectionMapping; scene.environment
    // = texture` — PMREM-filtered, and the *only* use of the map: this page
    // never sets `scene.background`, so no cube conversion and no skybox.
    let mut environment = PmremEnvironment::from_equirectangular(&texture);
    environment.update(&mut renderer).unwrap();
    scene.environment = Some(environment.handle());

    // `new GLTFLoader().setPath( 'models/gltf/DamagedHelmet/glTF/' ).load(
    // 'DamagedHelmet.gltf', gltf => scene.add( gltf.scene ) )`.
    let gltf =
        GLTFLoader::load(examples_dir().join("models/gltf/DamagedHelmet/glTF/DamagedHelmet.gltf"))
            .expect("DamagedHelmet.gltf");
    scene.add(&gltf.scene);

    // `const controls = new OrbitControls( camera, renderer.domElement );`
    // The one `update()` below has nothing to clamp — the distance is already
    // inside `[ minDistance, maxDistance ]` — so all it does is aim the camera
    // at the target.
    let mut controls = OrbitControls::new(&mut camera);
    // The canvas the example renders at, standing in for the element's
    // `clientWidth` / `clientHeight`.
    controls.set_element_size(INNER_WIDTH, INNER_HEIGHT);
    controls.min_distance = 2.0;
    controls.max_distance = 5.0;
    controls.target.set(0.0, -0.1, -0.2);
    // `controls.update();`
    controls.update(&mut camera, None);

    App {
        renderer,
        scene,
        camera,
        environment,
        scene_pass,
        render_pipeline,
        controls,
    }
}

/// The page's `animate()`: `renderPipeline.render()`, which fires the scene
/// pass's `updateBefore()` on its way. See `docs/postprocessing.md` for why
/// the port fires the pass explicitly.
pub fn animate(app: &mut App) {
    app.environment.update(&mut app.renderer).unwrap();
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

/// The example's controls, for a host that has a pointer. `None` when the
/// page creates none — the signature is the same for every example so the
/// viewer and the browser shell can drive any of them through one call.
pub fn controls(app: &mut App) -> Option<&mut OrbitControls> {
    Some(&mut app.controls)
}

/// The controls and the camera at once, which every one of the controls'
/// event handlers needs: the JS holds the camera as `this.object` and Rust
/// cannot, so `pointer_move` and the rest take it as an argument.
///
/// They are two fields of the same `App`, so borrowing both is sound — but
/// only this module can say so; a host holding `&mut App` and calling
/// [`controls`] and then reaching for the camera cannot. Hence the pair.
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
        .unwrap_or_else(|| "target/webgpu_custom_fog_background.png".to_string());
    three_rs::testing::write_png(&path, width, height, &pixels);
    println!("wrote {path} ({width}x{height})");
}
