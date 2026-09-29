//! Port of `three.js/examples/webgpu_cubemap_mix.html`, calling the three-rs
//! API in the same order the page's `init()` / `render()` do.
//!
//! Under the e2e harness the viewport is `400 * 2` x `250 * 2` with no
//! `deviceScaleFactor`, so `window.innerWidth` / `innerHeight` /
//! `devicePixelRatio` are 800, 500 and 1.
//!
//! Two cube maps, the Pisa HDR cube and the Milky Way, are cross-faded by
//! `oscSine( time.mul( .1 ) )` into one `scene.environmentNode`, which lights
//! DamagedHelmet. The same node, read at a fixed roughness of 0.5, is the
//! background. The harness pins `time` to 0, where `oscSine` is
//! `sin( 0.75 · 2π ) · 0.5 + 0.5 = 0`, so the graded frame is all Milky Way,
//! but both PMREMs are generated and sampled.
//!
//! # The environment node
//!
//! `pmremTexture( cube )` with no UV or level reads both from the build
//! context, which `EnvironmentNode` (for the lighting) and `Background` (for
//! the skybox) supply. The port has no node context, so
//! [`EnvironmentNode::new`] takes the graph as a function of the two, and
//! `.context( { getTextureLevel: () => float( .5 ) } )` is
//! [`EnvironmentNode::with_texture_level`]. `docs/nodes.md` §54.5.
//!
//! Each `pmremTexture()` is a `PMREMNode` that generates its PMREM on demand;
//! here that is a [`PmremEnvironment`] per cube, updated before each render.
//!
//! The page makes no draws from `Math.random()`.

use three_rs::addons::controls::OrbitControls;
use three_rs::loaders::{CubeTextureLoader, GltfLoader, HdrCubeTextureLoader};
use three_rs::materials::environment::EnvironmentNode;
use three_rs::materials::ToneMapping;
use three_rs::nodes::pmrem_node::PmremEnvironment;
use three_rs::nodes::tsl::{float, mix, osc_sine, time};
use three_rs::objects::Background;
use three_rs::textures::{MinFilter, TextureFilter};
use three_rs::{PerspectiveCamera, Renderer, RendererParameters, Scene};

pub const INNER_WIDTH: f64 = 800.0;
pub const INNER_HEIGHT: f64 = 500.0;
/// `renderer.setPixelRatio( window.devicePixelRatio )`.
pub const DPR: f64 = 1.0;

pub struct App {
    pub renderer: Renderer,
    pub scene: Scene,
    pub camera: PerspectiveCamera,
    /// The page's `controls`.
    pub controls: OrbitControls,
    /// `pmremTexture( cube1Texture )`'s PMREM: the Pisa HDR cube.
    pub cube1: PmremEnvironment,
    /// `pmremTexture( cube2Texture )`'s PMREM: the Milky Way.
    pub cube2: PmremEnvironment,
}

fn examples_dir() -> std::path::PathBuf {
    three_rs::testing::three_js_dir().join("examples")
}

pub fn init() -> App {
    let mut camera = PerspectiveCamera::new(45.0, INNER_WIDTH / INNER_HEIGHT, 0.25, 20.0);
    camera.node.borrow_mut().position.set(-1.8, 0.6, 2.7);

    let mut scene = Scene::new();

    let cube1_texture = HdrCubeTextureLoader::new()
        .set_path(examples_dir().join("textures/cube/pisaHDR/"))
        .load(["px.hdr", "nx.hdr", "py.hdr", "ny.hdr", "pz.hdr", "nz.hdr"])
        .unwrap();
    // `cube1Texture.generateMipmaps = true; cube1Texture.minFilter =
    // THREE.LinearMipmapLinearFilter;` — the loader had set both off.
    cube1_texture.set_generate_mipmaps(true);
    cube1_texture.set_filters(MinFilter::LinearMipmapLinear, TextureFilter::Linear);

    // `CubeTextureLoader` already leaves `generateMipmaps` on and the filter
    // at `LinearMipmapLinearFilter`; the page sets them again.
    let cube2_texture = CubeTextureLoader::new()
        .set_path(examples_dir().join("textures/cube/MilkyWay/"))
        .load([
            "dark-s_px.jpg",
            "dark-s_nx.jpg",
            "dark-s_py.jpg",
            "dark-s_ny.jpg",
            "dark-s_pz.jpg",
            "dark-s_nz.jpg",
        ])
        .unwrap();
    cube2_texture.set_generate_mipmaps(true);
    cube2_texture.set_filters(MinFilter::LinearMipmapLinear, TextureFilter::Linear);

    let cube1 = PmremEnvironment::new(&cube1_texture);
    let cube2 = PmremEnvironment::new(&cube2_texture);

    // `scene.environmentNode = mix( pmremTexture( cube2Texture ),
    // pmremTexture( cube1Texture ), oscSine( time.mul( .1 ) ) )`.
    let (pmrem1, pmrem2) = (cube1.handle(), cube2.handle());
    let environment_node = EnvironmentNode::new(move |uv, level| {
        mix(
            pmrem2.sample(uv.clone(), level.clone()),
            pmrem1.sample(uv, level),
            osc_sine(time().mul(0.1)),
        )
    });
    scene.environment_node = Some(environment_node.clone());

    // `scene.backgroundNode = scene.environmentNode.context( {
    // getTextureLevel: () => float( .5 ) } )`.
    scene.background = Some(Background::EnvironmentNode(
        environment_node.with_texture_level(float(0.5)),
    ));

    let gltf =
        GltfLoader::load(examples_dir().join("models/gltf/DamagedHelmet/glTF/DamagedHelmet.gltf"))
            .expect("DamagedHelmet.gltf");
    scene.add(&gltf.scene);

    let mut renderer = Renderer::new(RendererParameters { antialias: true }).unwrap();
    renderer.set_pixel_ratio(DPR);
    renderer.set_size(INNER_WIDTH, INNER_HEIGHT);
    renderer.tone_mapping = ToneMapping::Linear;

    // `const controls = new OrbitControls( camera, renderer.domElement );`
    // The constructor's `update()` turns the camera to the origin; the page
    // calls `update()` nowhere else.
    let mut controls = OrbitControls::new(&mut camera);
    controls.set_element_size(INNER_WIDTH, INNER_HEIGHT);
    controls.min_distance = 2.0;
    controls.max_distance = 10.0;

    App {
        renderer,
        scene,
        camera,
        controls,
        cube1,
        cube2,
    }
}

/// The page's `render()`, with the two `PMREMNode.updateBefore()`s that
/// build each PMREM the first time it is read.
pub fn animate(app: &mut App) {
    app.cube2.update(&mut app.renderer).unwrap();
    app.cube1.update(&mut app.renderer).unwrap();
    app.renderer.render(&mut app.scene, &mut app.camera);
}

/// The page's `onWindowResize()`.
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
/// event handlers needs.
pub fn controls_and_camera(app: &mut App) -> Option<(&mut OrbitControls, &mut PerspectiveCamera)> {
    Some((&mut app.controls, &mut app.camera))
}

fn main() {
    // Pin both clocks to zero, as three.js' `test/e2e/deterministic-injection.js`
    // does to the page.
    three_rs::testing::pin_time(Some(0.0));
    let mut app = init();
    println!("adapter: {:?}", app.renderer.adapter_info());
    animate(&mut app);

    let (width, height, pixels) = app.renderer.read_canvas_pixels().unwrap();
    let path = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "target/webgpu_cubemap_mix.png".to_string());
    three_rs::testing::write_png(&path, width, height, &pixels);
    println!("wrote {path} ({width}x{height})");
}
