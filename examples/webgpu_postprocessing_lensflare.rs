//! Port of `three.js/examples/webgpu_postprocessing_lensflare.html`, calling
//! the three-rs API in the same order the page's top-level script does.
//!
//! A space-ship hallway under an icy planet: the scene is drawn once into two
//! attachments (lit colour and the emissive term), the emissive term is
//! bloomed, `lensflare()` mirrors the bloom's bright spots through the screen
//! centre into ghosts, and a Gaussian blur softens the ghosts before all three
//! are summed and tone-mapped.
//!
//! Under the e2e harness the viewport is `400 * 2` x `250 * 2` with no
//! `deviceScaleFactor`, so `window.innerWidth` / `innerHeight` /
//! `devicePixelRatio` are 800, 500 and 1.
//!
//! Three things the port spells out that the page leaves implicit:
//!
//! * **`convertToTexture()`.** `lensflare( bloomPass )` and `gaussianBlur(
//!   flarePass, 8 )` are each handed a node that is not a texture, so three
//!   wraps both in an `RTTNode`. The port's [`lensflare`] and
//!   [`gaussian_blur`] take textures, so the two [`rtt`]s are written here.
//! * **`scene.background = texture; scene.environment = texture`** with the
//!   raw equirectangular map: the background's cube conversion and the
//!   environment's PMREM are written out, as in
//!   `webgpu_postprocessing_bloom_emissive`.
//! * **`blurPass.render()`.** [`GaussianBlurNode`] is not one of the nodes the
//!   renderer runs on its own, so `animate()` draws it before the pipeline;
//!   its first pass renders the whole chain above it.
//!
//! The page's GUI (bloom strength and radius, the flare's threshold,
//! attenuation and spacing, the exposure) is the inspector panel, which the
//! port does not have; the values it drives are public on [`App`].

use std::cell::RefCell;
use std::rc::Rc;

use three_rs::addons::controls::OrbitControls;
use three_rs::loaders::{GltfLoader, UltraHdrLoader};
use three_rs::math::Box3;
use three_rs::nodes::display::{
    bloom, gaussian_blur, lensflare, rtt, BloomNode, GaussianBlurNode, GaussianBlurOptions,
    LensflareNode, LensflareParams, RttNode,
};
use three_rs::nodes::mrt;
use three_rs::nodes::node::SettableValue;
use three_rs::nodes::pmrem_node::PmremEnvironment;
use three_rs::nodes::tsl::{emissive_color, output_property, uniform_settable, vec2};
use three_rs::nodes::Type;
use three_rs::objects::Background;
use three_rs::renderer::cube_render_target;
use three_rs::{
    pass, PassNode, PerspectiveCamera, RenderPipeline, Renderer, RendererParameters, Scene,
    ToneMapping,
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
    pub controls: OrbitControls,
    pub environment: PmremEnvironment,
    pub scene_pass: PassNode,
    pub bloom_pass: BloomNode,
    /// `convertToTexture( bloomPass )`, the flare's input.
    pub bloom_texture: RttNode,
    /// The lensflare folder's `threshold`.
    pub threshold: SettableValue,
    /// The lensflare folder's `attenuation`.
    pub ghost_attenuation_factor: SettableValue,
    /// The lensflare folder's `spacing`.
    pub ghost_spacing: SettableValue,
    pub flare_pass: LensflareNode,
    /// `convertToTexture( flarePass )`, the blur's input.
    pub flare_texture: RttNode,
    pub blur_pass: GaussianBlurNode,
    pub render_pipeline: RenderPipeline,
}

pub fn init() -> App {
    let camera = PerspectiveCamera::new(45.0, INNER_WIDTH / INNER_HEIGHT, 0.1, 100.0);
    camera.node.borrow_mut().position.set(0.0, 0.5, -0.5);

    let mut scene = Scene::new();

    // The renderer is built before the two environment conversions below
    // because in this port they take it; `new THREE.WebGPURenderer()` has no
    // `antialias`.
    let mut renderer = Renderer::new(RendererParameters::default()).unwrap();
    renderer.set_pixel_ratio(DPR);
    renderer.set_size(INNER_WIDTH, INNER_HEIGHT);
    renderer.tone_mapping = ToneMapping::AcesFilmic;

    // `new UltraHDRLoader().loadAsync( 'textures/equirectangular/
    // ice_planet_close.jpg' )` with `EquirectangularReflectionMapping`.
    let texture = UltraHdrLoader::new()
        .load(examples_dir().join("textures/equirectangular/ice_planet_close.jpg"))
        .unwrap();

    // `scene.background = texture`: converted to a cube once.
    let background = cube_render_target::from_equirectangular_texture(&mut renderer, &texture)
        .expect("the equirectangular background converts to a cube");
    scene.background = Some(Background::CubeTexture(background));

    // `scene.environment = texture`: the same map, PMREM-filtered.
    let mut environment = PmremEnvironment::from_equirectangular(&texture);
    environment.update(&mut renderer).unwrap();
    scene.environment = Some(environment.handle());

    scene.background_intensity = 2.0;
    scene.environment_intensity = 15.0;

    // model

    let gltf = GltfLoader::load(examples_dir().join("models/gltf/space_ship_hallway.glb"))
        .expect("space_ship_hallway.glb");
    let object = gltf.scene;

    // Re-centre the model on the origin.
    let aabb = *Box3::default().set_from_object(&object, false);
    let center = aabb.get_center();
    {
        let mut node = object.borrow_mut();
        let position = node.position;
        node.position.x += position.x - center.x;
        node.position.y += position.y - center.y;
        node.position.z += position.z - center.z;
    }

    scene.add(&object);

    //

    let scene = Rc::new(RefCell::new(scene));
    let camera = Rc::new(RefCell::new(camera));

    let scene_pass = pass(scene.clone(), camera.clone());
    scene_pass.set_mrt(mrt(vec![
        ("output", output_property()),
        ("emissive", emissive_color()),
    ]));

    // `toInspector( … )` names the node for the inspector panel and returns
    // it unchanged.
    let output_pass = scene_pass.texture_node("output");
    let emissive_pass = scene_pass.texture_node("emissive");

    // `bloom( emissivePass, 1, 1 )`.
    let bloom_pass = bloom(emissive_pass);
    bloom_pass.strength.set(vec![1.0]);
    bloom_pass.radius.set(vec![1.0]);

    let (threshold_node, threshold) = uniform_settable(Type::F32, vec![0.5]);
    let (ghost_attenuation_factor_node, ghost_attenuation_factor) =
        uniform_settable(Type::F32, vec![25.0]);
    let (ghost_spacing_node, ghost_spacing) = uniform_settable(Type::F32, vec![0.25]);

    // `lensflare( bloomPass, { threshold, ghostAttenuationFactor,
    // ghostSpacing } )`, whose `convertToTexture( bloomPass )` is an `rtt()`.
    let bloom_texture = rtt(bloom_pass.node());
    let mut params = LensflareParams::default();
    params.threshold = threshold_node;
    params.ghost_attenuation_factor = ghost_attenuation_factor_node;
    params.ghost_spacing = ghost_spacing_node;
    let flare_pass = lensflare(&bloom_texture.texture(), params);

    // `gaussianBlur( flarePass, 8 )` — "optional (blurring produces better
    // flare quality but also adds some overhead)". `8` is the direction, so
    // the taps are eight texels apart; sigma stays at its default 4. Three's
    // `vec2( this.directionNode )` of the plain number `8` is the constant
    // `vec2( 8, 8 )`, which is what is passed here.
    let flare_texture = rtt(flare_pass.node());
    let blur_pass = gaussian_blur(
        &flare_texture.texture(),
        Some(vec2(8.0, 8.0)),
        4,
        GaussianBlurOptions::default(),
    );

    let mut render_pipeline = RenderPipeline::new();
    render_pipeline.output_node = Some(output_pass.add(bloom_pass.node()).add(blur_pass.node()));

    //

    let mut controls = OrbitControls::new(&mut camera.borrow_mut());
    controls.set_element_size(INNER_WIDTH, INNER_HEIGHT);
    controls.enable_damping = true;
    controls.enable_pan = false;
    controls.enable_zoom = false;
    // `controls.target.copy( camera.position ); controls.target.z -= 0.01`.
    let position = camera.borrow().node.borrow().position;
    controls
        .target
        .set(position.x, position.y, position.z - 0.01);
    controls.update(&mut camera.borrow_mut(), None);

    App {
        renderer,
        scene,
        camera,
        controls,
        environment,
        scene_pass,
        bloom_pass,
        bloom_texture,
        threshold,
        ghost_attenuation_factor,
        ghost_spacing,
        flare_pass,
        flare_texture,
        blur_pass,
        render_pipeline,
    }
}

/// The page's `render()`.
pub fn animate(app: &mut App) {
    app.controls.update(&mut app.camera.borrow_mut(), None);

    // See the module docs: the blur first, which renders the scene pass, the
    // bloom, both `rtt()`s and the flare on its way; then the pipeline, whose
    // output quad finds all of them already drawn this frame.
    app.blur_pass.render(&mut app.renderer);
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
        .unwrap_or_else(|| "target/webgpu_postprocessing_lensflare.png".to_string());
    three_rs::testing::write_png(&path, width, height, &pixels);
    println!("wrote {path} ({width}x{height})");
}
