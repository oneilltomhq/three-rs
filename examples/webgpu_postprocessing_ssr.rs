//! Port of `three.js/examples/webgpu_postprocessing_ssr.html`, calling the
//! three-rs API in the same order the page's `init()` / `animate()` do.
//!
//! The steampunk camera on a mirror-metal disc, drawn into an `output` +
//! `normal` + `metalrough` MRT, with [`ssr`] reflections added on top and
//! the sum anti-aliased by [`smaa`] before the output transform.
//!
//! The page loads the model asynchronously; three's screenshot is taken
//! once it is in, so the port loads it synchronously where the callback
//! would run (after the floor is added). The GUI is not ported; its
//! defaults are, through the page's own `updateParameters()`.

use std::cell::RefCell;
use std::rc::Rc;

use three_rs::addons::controls::OrbitControls;
use three_rs::geometries::circle_geometry;
use three_rs::loaders::GltfLoader;
use three_rs::materials::{MeshStandardNodeMaterial, Side, ToneMapping};
use three_rs::nodes::display::{
    convert_to_texture, smaa, ssr, RttNode, SmaaNode, SsrNode, SsrOptions,
};
use three_rs::nodes::mrt;
use three_rs::nodes::pmrem_node::PmremEnvironment;
use three_rs::nodes::tsl::{
    distance, float, metalness, mix, normal_view, output_property, pack_normal_to_rgb, roughness,
    screen_uv, texture_uv, vec2_join, vec4_join,
};
use three_rs::objects::Background;
use three_rs::textures::TextureType;
use three_rs::{
    pass, Color, Mesh, PassNode, PerspectiveCamera, RenderPipeline, Renderer, RendererParameters,
    RoomEnvironment, Scene,
};

pub const INNER_WIDTH: f64 = 800.0;
pub const INNER_HEIGHT: f64 = 500.0;
/// `// renderer.setPixelRatio( window.devicePixelRatio )` — commented out on
/// the page, so the canvas is at CSS size.
pub const DPR: f64 = 1.0;

/// The page's `params`, the GUI's starting values. `binary_refine`,
/// `roughness` (the model folder's slider) and `enabled` are only read by the
/// GUI, which is not ported.
pub struct Params {
    pub quality: f64,
    pub blur_quality: u32,
    pub max_distance: f64,
    pub intensity: f64,
    pub thickness: f64,
    pub binary_refine: bool,
    pub roughness: f64,
    pub enabled: bool,
}

pub const PARAMS: Params = Params {
    quality: 0.5,
    blur_quality: 1,
    max_distance: 1.0,
    intensity: 1.0,
    thickness: 0.03,
    binary_refine: false,
    roughness: 1.0,
    enabled: true,
};

fn examples_dir() -> std::path::PathBuf {
    three_rs::testing::three_js_dir().join("examples")
}

pub struct App {
    pub renderer: Renderer,
    pub scene: Rc<RefCell<Scene>>,
    pub camera: Rc<RefCell<PerspectiveCamera>>,
    pub controls: OrbitControls,
    pub scene_pass: PassNode,
    pub ssr_pass: SsrNode,
    /// `convertToTexture( scenePassColor.add( ssrPass.rgb ) )`, SMAA's input.
    pub smaa_input: RttNode,
    pub smaa_pass: SmaaNode,
    pub render_pipeline: RenderPipeline,
}

pub fn init() -> App {
    let mut camera = PerspectiveCamera::new(35.0, INNER_WIDTH / INNER_HEIGHT, 0.1, 50.0);
    camera.node.borrow_mut().position.set(3.0, 2.0, 3.0);

    let mut scene = Scene::new();
    // `screenUV.distance( .5 ).remap( 0, 0.5 ).mix( color( 0x888877 ),
    // color( 0x776666 ) )`.
    let t = distance(screen_uv(), float(0.5)).remap(0.0, 0.5, 0.0, 1.0);
    scene.background = Some(Background::Node(mix(
        Color::from_hex(0x888877),
        Color::from_hex(0x776666),
        t,
    )));

    // Add a reflective plane under the camera.
    let floor_material = MeshStandardNodeMaterial::standard(Color::from_hex(0xffffff), 0.5, 1.0);
    let floor = Mesh::new(Rc::new(circle_geometry(2.0, 64)), floor_material);
    {
        let mut object = floor.borrow_mut();
        let rotation = object.rotation;
        object.set_rotation(-std::f64::consts::FRAC_PI_2, rotation.y, rotation.z);
        object.position.y = -0.8;
    }
    scene.add(&floor);

    // `loader.load( 'models/gltf/steampunk_camera.glb', … )`: the lens casing
    // is transparent, and every material is `FrontSide` to avoid overdraw.
    let gltf = GltfLoader::load(examples_dir().join("models/gltf/steampunk_camera.glb"))
        .expect("steampunk_camera.glb");
    for primitive in &gltf.primitives {
        let lense_casing = primitive
            .material
            .is_some_and(|index| gltf.materials[index].name == "Lense_Casing");
        if let Some(material) = primitive
            .node
            .borrow_mut()
            .mesh_mut()
            .and_then(|mesh| mesh.material.as_mut())
        {
            if lense_casing {
                material.transparent = true;
            }
            material.side = Side::Front;
        }
    }
    gltf.scene.borrow_mut().position.y = 0.1;
    scene.add(&gltf.scene);

    let mut renderer = Renderer::new(RendererParameters::default()).unwrap();
    renderer.set_pixel_ratio(DPR);
    renderer.set_size(INNER_WIDTH, INNER_HEIGHT);
    renderer.tone_mapping = ToneMapping::AcesFilmic;

    let mut room = RoomEnvironment::new();
    let environment = PmremEnvironment::from_scene(&mut renderer, &mut room, 0.04).unwrap();
    scene.environment = Some(environment.handle());
    scene.environment_intensity = 1.25;

    // `new OrbitControls( camera, … )` is created after the pipeline on the
    // page; nothing between reads the camera's orientation, so the order of
    // the two is immaterial and the controls are made here, where the camera
    // is still unshared.
    let mut controls = OrbitControls::new(&mut camera);
    controls.set_element_size(INNER_WIDTH, INNER_HEIGHT);
    controls.enable_damping = true;
    controls.update(&mut camera, None);

    let scene = Rc::new(RefCell::new(scene));
    let camera = Rc::new(RefCell::new(camera));

    let mut render_pipeline = RenderPipeline::new();

    let scene_pass = pass(scene.clone(), camera.clone());
    let mut scene_mrt = mrt(vec![("output", output_property())]);
    // `normal: packNormalToRGB( normalView )`, `metalrough: vec2( metalness,
    // roughness )` — per-material expressions, so deferred (`docs/nodes.md`
    // §23).
    scene_mrt.set_deferred("normal", || pack_normal_to_rgb(normal_view()));
    scene_mrt.set_deferred("metalrough", || vec2_join(vec![metalness(), roughness()]));
    scene_pass.set_mrt(scene_mrt);

    // `getTextureNode()` in the page's order; `toInspector()` returns its
    // node unchanged.
    let scene_pass_color = scene_pass.texture_node("output");
    let _ = scene_pass.texture_node("normal");
    let _ = scene_pass.texture_node("depth");
    let scene_pass_metal_rough = scene_pass.texture_node("metalrough");

    // Optional: reduce the precision of normals and metal/roughness.
    let normal_texture = scene_pass.texture_named("normal");
    normal_texture.set_texture_type(TextureType::UnsignedByte);
    scene_pass
        .texture_named("metalrough")
        .set_texture_type(TextureType::UnsignedByte);

    // `sample( ( uv ) => unpackRGBToNormal( scenePassNormal.sample( uv ) ) )`,
    // with `unpackRGBToNormal( rgb ) = rgb * 2 - 1`.
    let scene_normal: three_rs::nodes::display::SampleFn =
        Rc::new(move |coord| texture_uv(&normal_texture, coord).mul(2.0).sub(1.0));

    let ssr_pass = ssr(
        &scene_pass.texture(),
        &scene_pass.depth_texture(),
        scene_normal,
        SsrOptions::new(scene_pass_metal_rough.x(), Some(scene_pass_metal_rough.y())),
        camera.clone(),
    );

    // Blend SSR over beauty: SSR is premultiplied, so additively. The vec3
    // widens to `vec4( rgb, 1.0 )`, as three's `add` converts it.
    let smaa_input = convert_to_texture(
        scene_pass_color.add(vec4_join(vec![ssr_pass.node().rgb(), float(1.0)])),
    );
    let smaa_pass = smaa(&smaa_input.texture());

    render_pipeline.output_node = Some(smaa_pass.node());

    let app = App {
        renderer,
        scene,
        camera,
        controls,
        scene_pass,
        ssr_pass,
        smaa_input,
        smaa_pass,
        render_pipeline,
    };
    update_parameters(&app);
    app
}

/// The page's `updateParameters()`, with `params` as the GUI leaves them.
pub fn update_parameters(app: &App) {
    app.ssr_pass.quality().set(vec![PARAMS.quality]);
    app.ssr_pass.set_blur_quality(PARAMS.blur_quality);
    app.ssr_pass.max_distance().set(vec![PARAMS.max_distance]);
    app.ssr_pass.intensity().set(vec![PARAMS.intensity]);
    app.ssr_pass.thickness().set(vec![PARAMS.thickness]);
    // `ssrPass.binaryRefine = params.binaryRefine` is not ported: the page
    // starts it at `false`, `SSRNode`'s default and the one path ported.
}

/// The page's `animate()`.
pub fn animate(app: &mut App) {
    app.controls.update(&mut app.camera.borrow_mut(), None);
    app.render_pipeline.render(&mut app.renderer);
}

/// The page's `onWindowResize()`.
pub fn resize(app: &mut App, width: f64, height: f64) {
    let mut camera = app.camera.borrow_mut();
    camera.aspect = width / height;
    camera.update_projection_matrix();
    app.renderer.set_size(width, height);
    app.controls.set_element_size(width, height);
}

pub fn controls(app: &mut App) -> Option<&mut OrbitControls> {
    Some(&mut app.controls)
}

/// The controls and the camera at once, for a host delivering pointer events.
pub fn controls_and_camera(
    app: &mut App,
) -> Option<(&mut OrbitControls, std::cell::RefMut<'_, PerspectiveCamera>)> {
    Some((&mut app.controls, app.camera.borrow_mut()))
}

fn main() {
    // Pin both clocks to zero, as three.js' `test/e2e/deterministic-injection.js`
    // does to the page, so the frame this writes does not depend on how long
    // `init()` took.
    three_rs::testing::pin_time(Some(0.0));
    let mut app = init();
    println!("adapter: {:?}", app.renderer.adapter_info());
    animate(&mut app);

    let (width, height, pixels) = app.renderer.read_canvas_pixels().unwrap();
    let path = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "target/webgpu_postprocessing_ssr.png".to_string());
    three_rs::testing::write_png(&path, width, height, &pixels);
    println!("wrote {path} ({width}x{height})");
}
