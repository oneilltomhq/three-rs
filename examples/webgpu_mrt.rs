//! Port of `three.js/examples/webgpu_mrt.html`, calling the three-rs API in
//! the same order the page's `init()` and its nested loader callbacks do.
//!
//! Under the e2e harness the viewport is `400 * 2` x `250 * 2` with no
//! `deviceScaleFactor`, so `window.innerWidth` / `innerHeight` /
//! `devicePixelRatio` are 800, 500 and 1.
//!
//! # What the page shows
//!
//! `webgpu_loader_gltf`'s scene — one UltraHDR map as both `scene.background`
//! and `scene.environment`, with DamagedHelmet in front of it — drawn **once**
//! into four colour attachments, and then all four composited side by side in
//! fifths of the screen:
//!
//! | x | attachment | through |
//! |---|---|---|
//! | 0.0 – 0.2 | `output` | `renderOutput()`: ACES, then sRGB |
//! | 0.2 – 0.4 | `output` | raw, linear, untransformed |
//! | 0.4 – 0.6 | `normal` | `packNormalToRGB( normalView )` |
//! | 0.6 – 0.8 | `emissive` | `EmissiveColor` |
//! | 0.8 – 1.0 | `diffuse` | `DiffuseColor` |
//!
//! Five bands from four attachments: the first two are the *same* texture, and
//! the whole point of the leftmost is that it is the only one of the five the
//! output colour transform is applied to. `renderPipeline.outputColorTransform
//! = false` is what makes the other four land in the canvas untouched.
//!
//! # Three things this rung is the gate on
//!
//! * **`pass( scene, camera, { minFilter: NearestFilter, magFilter:
//!   NearestFilter } )` changes the generated code.** A texture that is
//!   `NearestFilter` on both sides is *unfilterable*
//!   (`WGSLNodeBuilder.isUnfilterable`): three binds it with a `non-filtering`
//!   sample type and no sampler at all, and every tap on it becomes a
//!   `textureLoad` against `textureDimensions` instead of a `textureSample`.
//!   Three's `dump-mrt/m12` has four bare `texture_2d<f32>` bindings and not
//!   one `_sampler`. See `docs/nodes.md` §23.
//! * **A `vec3` MRT output is written as `vec4( value, 1.0 )`.** `normal` and
//!   `emissive` are `vec3`s; `OutputStructNode` converts each member to the
//!   attachment's `vec4`, appending 1.0 rather than the fragment's alpha —
//!   unlike `webgpu_postprocessing_bloom_emissive`, whose page writes the
//!   `vec4( emissive, output.a )` out by hand.
//! * **`normal` / `diffuse` / `emissive` are `UnsignedByteType`.** "optimize
//!   textures": attachment 0 stays `rgba16float` and the other three are
//!   `rgba8unorm`, so the pipeline needs a colour target per attachment. With
//!   `antialias: true` all four are also 4x multisampled and resolved.
//!
//! `requiredLimits: { maxColorAttachments: 5 }` is a WebGPU device request for
//! a limit this port already gets from wgpu's default adapter limits (8).

use three_rs::loaders::{GLTFLoader, UltraHdrLoader};
use three_rs::materials::{render_output, ToneMapping};
use three_rs::nodes::mrt;
use three_rs::nodes::pmrem_node::PmremEnvironment;
use three_rs::nodes::tsl::{
    diffuse_color, emissive_color, mix, normal_view, pack_normal_to_rgb, screen_uv, step,
};
use three_rs::objects::Background;
use three_rs::renderer::{cube_render_target, PassOptions};
use three_rs::textures::{TextureFilter, TextureType};
use three_rs::{
    PassNode, PerspectiveCamera, RenderPipeline, Renderer, RendererParameters, Scene, Vector3,
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
}

pub fn init() -> App {
    // `new THREE.PerspectiveCamera( 45, window.innerWidth / window.innerHeight,
    // 0.25, 20 )`, then `camera.position.set( - 1.8, 0.6, 2.7 )`. Unlike
    // `webgpu_loader_gltf`, nothing moves it afterwards.
    let mut camera = PerspectiveCamera::new(45.0, INNER_WIDTH / INNER_HEIGHT, 0.25, 20.0);
    camera.node.borrow_mut().position.set(-1.8, 0.6, 2.7);

    let mut scene = Scene::new();

    // `new THREE.WebGPURenderer( { antialias: true, requiredLimits: {
    // maxColorAttachments: 5 } } )`. The page builds it after the loader call;
    // here it is first, because the two environment conversions below take it.
    let mut renderer = Renderer::new(RendererParameters { antialias: true }).unwrap();
    renderer.set_pixel_ratio(DPR);
    renderer.set_size(INNER_WIDTH, INNER_HEIGHT);
    renderer.tone_mapping = ToneMapping::AcesFilmic;

    // `new UltraHDRLoader().setPath( 'textures/equirectangular/' ).load(
    // 'royal_esplanade_2k.hdr.jpg', … )`; the loader resolves synchronously
    // here, so the callback's body is written inline.
    let texture = UltraHdrLoader::new()
        .load(examples_dir().join("textures/equirectangular/royal_esplanade_2k.hdr.jpg"))
        .unwrap();

    // `texture.mapping = EquirectangularReflectionMapping; scene.background =
    // texture` — the 512² cube the skybox samples.
    let background = cube_render_target::from_equirectangular_texture(&mut renderer, &texture)
        .expect("the equirectangular background converts to a cube");
    scene.background = Some(Background::CubeTexture(background));

    // `scene.environment = texture`: the same map, PMREM-filtered, and the
    // only light in the scene.
    let mut environment = PmremEnvironment::from_equirectangular(&texture);
    environment.update(&mut renderer).unwrap();
    scene.environment = Some(environment.handle());

    // `new GLTFLoader().setPath( 'models/gltf/DamagedHelmet/glTF/' ).load(
    // 'DamagedHelmet.gltf', … )`.
    let gltf =
        GLTFLoader::load(examples_dir().join("models/gltf/DamagedHelmet/glTF/DamagedHelmet.gltf"))
            .expect("DamagedHelmet.gltf");
    scene.add(&gltf.scene);

    // `new OrbitControls( … )` with `target.set( 0, 0, - 0.2 )` and one
    // `update()`: the camera's pose for the whole page.
    camera.look_at(&Vector3::new(0.0, 0.0, -0.2));

    // post processing

    // `pass( scene, camera, { minFilter: NearestFilter, magFilter:
    // NearestFilter } )` — the filters are what make all four attachments
    // unfilterable, so the composite below reads them with `textureLoad`.
    let scene_pass = PassNode::new_with_options(PassOptions {
        min_filter: TextureFilter::Nearest,
        mag_filter: TextureFilter::Nearest,
        ..PassOptions::default()
    });

    // `scenePass.setMRT( mrt( { output, normal: packNormalToRGB( normalView ),
    // diffuse: diffuseColor, emissive } ) )`. `normal` and `emissive` are
    // `vec3`s and land as `vec4( v, 1.0 )`.
    let mut scene_mrt = mrt(vec![("output", three_rs::nodes::tsl::output_property())]);
    // `normal: packNormalToRGB( normalView )`. `normalView` is a node object
    // upstream, so its `setup()` runs per material: the helmet's is its normal
    // map's, and the skybox's is `normalViewGeometry * - 1` because
    // `Background.material` is `BackSide`. This port's TSL is eager, so the
    // expression is handed over as a closure instead. See `docs/nodes.md` §23.
    scene_mrt.set_deferred("normal", || pack_normal_to_rgb(normal_view()));
    scene_mrt.set("diffuse", diffuse_color());
    scene_mrt.set("emissive", emissive_color());
    scene_pass.set_mrt(scene_mrt);

    // `normalTexture.type = diffuseTexture.type = emissiveTexture.type =
    // THREE.UnsignedByteType` — "optimize textures". Attachment 0 stays
    // `rgba16float`.
    for name in ["normal", "diffuse", "emissive"] {
        scene_pass
            .texture_named(name)
            .set_texture_type(TextureType::UnsignedByte);
    }

    let output = scene_pass.texture_node("output");
    let normal = scene_pass.texture_node("normal");
    let diffuse = scene_pass.texture_node("diffuse");
    let emissive = scene_pass.texture_node("emissive");

    // `renderPipeline.outputColorTransform = false` and the `Fn` that
    // composites the four. The leftmost fifth is the only band that goes
    // through `renderOutput()`, which is what the transform being off buys.
    let mut render_pipeline = RenderPipeline::new();
    render_pipeline.output_color_transform = false;

    let out = mix(
        render_output(output.clone(), ToneMapping::AcesFilmic),
        output,
        step(0.2, screen_uv().x()),
    );
    let nor = mix(out, normal, step(0.4, screen_uv().x()));
    let emi = mix(nor, emissive, step(0.6, screen_uv().x()));
    let dif = mix(emi, diffuse, step(0.8, screen_uv().x()));
    render_pipeline.output_node = Some(dif);

    App {
        renderer,
        scene,
        camera,
        environment,
        scene_pass,
        render_pipeline,
    }
}

/// The page's `render()` — `renderPipeline.render()` alone upstream; see
/// `docs/postprocessing.md` for why the pass is fired explicitly here.
pub fn animate(app: &mut App) {
    app.scene_pass
        .render(&mut app.renderer, &mut app.scene, &mut app.camera);
    app.render_pipeline.render(&mut app.renderer);
}

fn main() {
    let mut app = init();
    println!("adapter: {:?}", app.renderer.adapter_info());
    animate(&mut app);

    let (width, height, pixels) = app.renderer.read_canvas_pixels().unwrap();
    let path = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "target/webgpu_mrt.png".to_string());
    three_rs::testing::write_png(&path, width, height, &pixels);
    println!("wrote {path} ({width}x{height})");
}
