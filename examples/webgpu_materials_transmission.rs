//! Port of `three.js/examples/webgpu_materials_transmission.html`, calling the
//! three-rs API in the same order the page's `init()` does.
//!
//! Under the e2e harness the viewport is `400 * 2` x `250 * 2` with no
//! `deviceScaleFactor`, so `window.innerWidth` / `innerHeight` /
//! `devicePixelRatio` are 800, 500 and 1.
//!
//! The page has no clock and no `Math.random()`; `animate()` is one
//! `renderer.render( scene, camera )`.
//!
//! # What this page is
//!
//! One `SphereGeometry( 20, 64, 32 )` with a `MeshPhysicalMaterial` whose
//! `transmission` is 1, in front of the UltraHDR `royal_esplanade_2k` used as
//! both background and env map. It is the ladder's direct test of the
//! transmission path that `webgpu_loader_gltf_anisotropy` brought in
//! (`docs/nodes.md` §26): the whole image behind the sphere is background, so
//! the opaque-frame copy the refraction samples is the skybox, and every
//! transmitted pixel is a re-read of it.
//!
//! Four things are easy to lose:
//!
//! * **The scene has no lights.** As on the anisotropy page, every lit pixel
//!   comes from the environment; `PhysicalLightingModel.direct()` is never
//!   called. What is new here is `material.envMap` rather than
//!   `scene.environment`: the PMREM reaches the material directly, and
//!   `scene.environment` is never set.
//! * **`thickness` stays at the `MeshPhysicalMaterial` default of 0.** The
//!   page's `params.thickness` is 0.01, but it is only ever applied by the GUI
//!   `onChange`, which the graded frame never runs. So the volume ray has zero
//!   length and `volumeAttenuation` is the identity — the transmission is a
//!   straight re-read of the frame at the fragment's own screen position,
//!   blurred by the roughness mip. Same for `params.opacity` / `metalness` /
//!   `roughness` / `ior`, which the constructor does pass.
//! * **`alphaMap` is a 2×2 canvas**, white in its bottom row and fully
//!   transparent in its top one, `NearestFilter`, `RepeatWrapping` and
//!   `repeat.set( 1, 3.5 )`. That is the band pattern on the sphere: seven
//!   horizontal stripes of alpha 1 and 0. It is ported as a `Texture` with the
//!   same four RGBA texels in canvas (top-first) order, which the uploader's
//!   `flipY` then turns the same way `CanvasTexture` does — nothing here is
//!   derived from the reference image.
//! * **`side: DoubleSide` plus `transmission: 1`** is three's
//!   `transparentDoublePass`: the sphere's back faces are drawn before its
//!   front ones in the transmission pass.
//!
//! The alpha map's transparent band is where `transparent: true` shows: those
//! fragments keep their own alpha and blend, while the transmissive ones are
//! opaque-looking because the transmission term already carries the background
//! through.

use std::rc::Rc;

use three_rs::geometries::sphere_geometry;
use three_rs::loaders::UltraHdrLoader;
use three_rs::materials::{Side, ToneMapping};
use three_rs::nodes::pmrem_node::PmremEnvironment;
use three_rs::objects::Background;
use three_rs::renderer::cube_render_target;
use three_rs::textures::{MinFilter, TextureFilter, Wrapping};
use three_rs::{
    Color, Mesh, MeshPhysicalNodeMaterial, PerspectiveCamera, Renderer, RendererParameters, Scene,
    Texture, Vector3,
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
}

/// The page's `generateTexture()`: a 2×2 canvas, cleared to transparent black,
/// with `fillRect( 0, 1, 2, 1 )` in white — so the *bottom* row is opaque
/// white and the top row is `rgba( 0, 0, 0, 0 )`. The bytes below are in the
/// canvas' own top-first row order; `Texture`'s `flipY` (true, as on a
/// `CanvasTexture`) does the rest on upload.
fn generate_texture() -> Texture {
    let mut data = Vec::with_capacity(2 * 2 * 4);
    // row y = 0: untouched canvas — transparent black.
    data.extend_from_slice(&[0, 0, 0, 0, 0, 0, 0, 0]);
    // row y = 1: `context.fillStyle = 'white'; context.fillRect( 0, 1, 2, 1 )`.
    data.extend_from_slice(&[255, 255, 255, 255, 255, 255, 255, 255]);
    Texture::new(2, 2, Some(data))
}

pub fn init() -> App {
    // `new THREE.WebGPURenderer( { antialias: true } )` and the tone mapping
    // pair. The page builds the renderer inside the loader callback; here it
    // comes first because the two environment conversions below take it.
    let mut renderer = Renderer::new(RendererParameters { antialias: true }).unwrap();
    renderer.set_pixel_ratio(DPR);
    renderer.set_size(INNER_WIDTH, INNER_HEIGHT);
    renderer.tone_mapping = ToneMapping::AcesFilmic;
    // `renderer.toneMappingExposure = params.exposure` — 1.
    renderer.tone_mapping_exposure = 1.0;

    let mut scene = Scene::new();

    // `new THREE.PerspectiveCamera( 40, window.innerWidth / window.innerHeight,
    // 1, 2000 )`, `camera.position.set( 0, 0, 120 )`.
    let mut camera = PerspectiveCamera::new(40.0, INNER_WIDTH / INNER_HEIGHT, 1.0, 2000.0);
    camera.node.borrow_mut().position.set(0.0, 0.0, 120.0);

    // `new UltraHDRLoader().setPath( 'textures/equirectangular/' ).load(
    // 'royal_esplanade_2k.hdr.jpg', … )`; the loader is synchronous here, so
    // the callback body follows inline.
    let texture = UltraHdrLoader::new()
        .load(examples_dir().join("textures/equirectangular/royal_esplanade_2k.hdr.jpg"))
        .unwrap();

    // `hdrEquirect.mapping = THREE.EquirectangularReflectionMapping` then
    // `scene.background = hdrEquirect`: with `backgroundBlurriness` 0 that is
    // `NodeManager.updateBackground()`'s `cubeMapNode( texture( background ) )`
    // — the equirect map converted to a cube once, sampled sharp.
    let background = cube_render_target::from_equirectangular_texture(&mut renderer, &texture)
        .expect("the equirectangular background converts to a cube");
    scene.background = Some(Background::CubeTexture(background));

    // `envMap: hdrEquirect` on the material: `MeshStandardNodeMaterial`'s
    // `setupEnvironment()` is `pmremTexture( envMap )`, the same PMREM
    // `scene.environment` would build — but owned by the material, and this
    // page never sets `scene.environment`.
    let mut environment = PmremEnvironment::from_equirectangular(&texture);
    environment.update(&mut renderer).unwrap();

    //

    // `new THREE.SphereGeometry( 20, 64, 32 )`.
    let geometry = Rc::new(sphere_geometry(20.0, 64, 32));

    // `new THREE.CanvasTexture( generateTexture() )` with `magFilter =
    // NearestFilter`, `wrapS = wrapT = RepeatWrapping` and
    // `repeat.set( 1, 3.5 )`.
    let alpha_map = generate_texture();
    alpha_map.set_mag_filter(TextureFilter::Nearest);
    alpha_map.set_min_filter(MinFilter::LinearMipmapLinear);
    alpha_map.set_wrapping(Wrapping::Repeat, Wrapping::Repeat);
    alpha_map.set_repeat(1.0, 3.5);
    alpha_map.update_matrix();

    // `new THREE.MeshPhysicalMaterial( { … } )`, field for field. `thickness`
    // is *not* among them (module docs), so it keeps the material default 0.
    let mut material = MeshPhysicalNodeMaterial::physical(Color::from_hex(0xffffff), 0.0, 0.0);
    material.ior = 1.5;
    material.alpha_map = Some(alpha_map);
    material.pmrem_env = Some(environment.handle());
    material.transmission = 1.0;
    material.specular_intensity = 1.0;
    material.specular_color = Color::from_hex(0xffffff);
    material.opacity = 1.0;
    material.side = Side::Double;
    material.transparent = true;

    let mesh = Mesh::new(geometry, material);
    scene.add(&mesh);

    // `new OrbitControls( camera, renderer.domElement )`: with no pointer
    // events the orbit is the identity, and the distance clamps (10 / 150) do
    // not bite at the camera's 120.
    camera.look_at(&Vector3::ZERO);

    App {
        renderer,
        scene,
        camera,
        environment,
    }
}

/// The page's `animate()`: one render, nothing moved.
pub fn animate(app: &mut App) {
    app.environment.update(&mut app.renderer).unwrap();
    app.renderer.render(&mut app.scene, &mut app.camera);
}

fn main() {
    let mut app = init();
    println!("adapter: {:?}", app.renderer.adapter_info());
    animate(&mut app);

    let (width, height, pixels) = app.renderer.read_canvas_pixels().unwrap();
    let path = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "target/webgpu_materials_transmission.png".to_string());
    three_rs::testing::write_png(&path, width, height, &pixels);
    println!("wrote {path} ({width}x{height})");
}
