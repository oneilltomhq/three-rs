//! Port of `three.js/examples/webgpu_postprocessing_bloom_emissive.html`,
//! calling the three-rs API in the same order the page's top-level script does.
//!
//! The page is the shortest statement of what MRT is *for*: the scene is drawn
//! once into two attachments — the lit colour and the material's emissive term
//! — and only the second one is fed to the bloom chain, so the helmet's glowing
//! panels bloom and the environment behind them does not.
//!
//! Under the e2e harness the viewport is `400 * 2` x `250 * 2` with no
//! `deviceScaleFactor`, so `window.innerWidth` / `innerHeight` /
//! `devicePixelRatio` are 800, 500 and 1.
//!
//! Four things the page does that are easy to lose:
//!
//! * **`scene.background = texture; scene.environment = texture` with a raw
//!   equirectangular HDR.** The page calls no `PMREMGenerator` at all: the
//!   renderer converts the map to a 512² cube for the background
//!   ([`cube_render_target`]) and PMREM-filters it for the environment
//!   ([`PmremEnvironment`]), both on the first frame. Both conversions are
//!   written out here because this port has no `scene.environment` field yet —
//!   see the progress doc.
//! * **`mrt( { output, emissive: vec4( emissive, output.a ) } )`** — `emissive`
//!   is the `EmissiveColor` var of the standard material's fragment flow, which
//!   the background's own material never assigns, so the sky writes 0 to
//!   attachment 1 and blooms not at all.
//! * **`emissiveTexture.type = UnsignedByteType`** — attachment 1 is
//!   `rgba8unorm` while attachment 0 stays `rgba16float`, which is why the
//!   pipeline needs a colour target per attachment.
//! * **`OrbitControls` is constructed and never updated.** `controls.update()`
//!   runs once, so the camera pose is `lookAt( 0, 0, - 0.2 )` for the whole
//!   page.
//!
//! [`cube_render_target`]: three_rs::renderer::cube_render_target
//! [`PmremEnvironment`]: three_rs::nodes::pmrem_node::PmremEnvironment

use three_rs::loaders::{GLTFLoader, HdrLoader};
use three_rs::materials::Blending;
use three_rs::nodes::display::{bloom, BloomNode};
use three_rs::nodes::mrt;
use three_rs::nodes::pmrem_node::PmremEnvironment;
use three_rs::nodes::tsl::{emissive_color, output_property, vec4_join};
use three_rs::objects::Background;
use three_rs::renderer::cube_render_target;
use three_rs::textures::TextureType;
use three_rs::{
    PassNode, PerspectiveCamera, RenderPipeline, Renderer, RendererParameters, Scene, ToneMapping,
    Vector3,
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
    pub bloom_pass: BloomNode,
    pub render_pipeline: RenderPipeline,
}

pub fn init() -> App {
    let mut camera = PerspectiveCamera::new(45.0, INNER_WIDTH / INNER_HEIGHT, 0.25, 20.0);
    camera.node.borrow_mut().position.set(-1.8, 0.6, 2.7);

    let mut scene = Scene::new();

    // `renderer = new THREE.WebGPURenderer()` — `antialias` is not in the
    // parameter object, so it is false. The renderer is built before the two
    // environment conversions below because in this port they take it.
    let mut renderer = Renderer::new(RendererParameters { antialias: false }).unwrap();
    renderer.set_pixel_ratio(DPR);
    renderer.set_size(INNER_WIDTH, INNER_HEIGHT);
    renderer.tone_mapping = ToneMapping::AcesFilmic;

    // `new HDRLoader().setPath( 'textures/equirectangular/' ).load(
    // 'moonless_golf_1k.hdr', … )`. The loader resolves synchronously here, so
    // the callback's body is written inline.
    let texture = HdrLoader::new()
        .load(examples_dir().join("textures/equirectangular/moonless_golf_1k.hdr"))
        .unwrap();

    // `scene.background = texture` with `EquirectangularReflectionMapping`:
    // `CubeMapNode.updateBefore()` converts the 1024×512 map into a 512² cube
    // once and the skybox samples that.
    let background = cube_render_target::from_equirectangular_texture(&mut renderer, &texture)
        .expect("the equirectangular background converts to a cube");
    scene.background = Some(Background::CubeTexture(background));

    // `scene.environment = texture`: the same map, PMREM-filtered, on every
    // material of the scene. This port has no `Scene::environment`, so the
    // handle is put on the loaded materials by hand — the same node graph
    // `EnvironmentNode` would build.
    let mut environment = PmremEnvironment::from_equirectangular(&texture);
    environment.update(&mut renderer).unwrap();

    // `new GLTFLoader().setPath( 'models/gltf/DamagedHelmet/glTF/' ).load(
    // 'DamagedHelmet.gltf', … )` — five external JPEG maps: albedo,
    // metalRoughness, normal, emissive and AO.
    let gltf =
        GLTFLoader::load(examples_dir().join("models/gltf/DamagedHelmet/glTF/DamagedHelmet.gltf"))
            .expect("DamagedHelmet.gltf");
    gltf.scene.traverse(&mut |node| {
        if let Some(material) = node
            .borrow_mut()
            .mesh_mut()
            .and_then(|m| m.material.as_mut())
        {
            material.pmrem_env = Some(environment.handle());
        }
    });
    scene.add(&gltf.scene);

    // `new OrbitControls( camera, renderer.domElement )` with `target.set( 0,
    // 0, - 0.2 )` and one `update()`: the camera's pose for the whole page.
    camera.look_at(&Vector3::new(0.0, 0.0, -0.2));

    // post processing

    let scene_pass = PassNode::new();

    // `mrt( { output, emissive: vec4( emissive, output.a ) } )`, with
    // `NormalBlending` on the emissive attachment. `_getBlending()` reads the
    // MRT's blend mode whatever `material.transparent` is, so attachment 1 gets
    // a src-alpha blend state where attachment 0 has none.
    let mut mrt_node = mrt(vec![
        ("output", output_property()),
        (
            "emissive",
            vec4_join(vec![emissive_color(), output_property().w()]),
        ),
    ]);
    mrt_node.set_blend_mode("emissive", Blending::Normal);
    scene_pass.set_mrt(mrt_node);

    // `const emissiveTexture = scenePass.getTexture( 'emissive' );
    // emissiveTexture.type = THREE.UnsignedByteType` — "optimize the
    // bandwidth": the emissive attachment is LDR, the colour one stays
    // `rgba16float`.
    scene_pass
        .texture_named("emissive")
        .set_texture_type(TextureType::UnsignedByte);

    // `toInspector( … )` is a no-op on the rendered frame: it names the node for
    // the inspector panel and returns it unchanged.
    let output_pass = scene_pass.texture_node("output");
    let emissive_pass = scene_pass.texture_node("emissive");

    // `bloom( emissivePass, 2.5, .5 )` — threshold left at 0.
    let bloom_pass = bloom(emissive_pass);
    bloom_pass.strength.set(vec![2.5]);
    bloom_pass.radius.set(vec![0.5]);

    let mut render_pipeline = RenderPipeline::new();
    render_pipeline.output_node = Some(output_pass.add(bloom_pass.node()));

    App {
        renderer,
        scene,
        camera,
        environment,
        scene_pass,
        bloom_pass,
        render_pipeline,
    }
}

/// The page's `render()`, run once by the harness's single RAF.
pub fn animate(app: &mut App) {
    // `renderPipeline.render()` alone upstream: `PassNode.updateBefore()`, then
    // `BloomNode.updateBefore()`'s twelve quads, then the output quad — see
    // `docs/postprocessing.md` for why the port fires them explicitly.
    app.scene_pass
        .render(&mut app.renderer, &mut app.scene, &mut app.camera);
    app.bloom_pass.render(&mut app.renderer);
    app.render_pipeline.render(&mut app.renderer);
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
        .unwrap_or_else(|| "target/webgpu_postprocessing_bloom_emissive.png".to_string());
    three_rs::testing::write_png(&path, width, height, &pixels);
    println!("wrote {path} ({width}x{height})");
}
