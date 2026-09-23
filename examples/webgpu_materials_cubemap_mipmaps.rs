//! Port of `three.js/examples/webgpu_materials_cubemap_mipmaps.html`, calling
//! the three-rs API in the same order the page's `init()` /
//! `loadCubeTextureWithMipmaps()` do.
//!
//! Under the e2e harness the viewport is `400 * 2` x `250 * 2` with no
//! `deviceScaleFactor`, so `window.innerWidth` / `innerHeight` /
//! `devicePixelRatio` are 800, 500 and 1.
//!
//! The page draws from `Math.random()` not at all, so the harness' seeded
//! sequence is never touched.
//!
//! # The two cubes
//!
//! Both spheres sample the same nine levels of `textures/cube/angus/`, and the
//! point of the page is *where those levels come from*. The right-hand sphere
//! (`x = +100`) reads a `CubeTexture` whose eight mips were loaded from their
//! own files — `generateMipmaps = false`, `mipmaps` supplied by hand. The
//! left-hand one (`x = -100`) reads a *copy* of that texture with `mipmaps`
//! emptied and `generateMipmaps = true`, so the backend blits the chain
//! itself. The uploads are different; the sampler, the material and the WGSL
//! are identical, which is why the frame is two near-identical spheres and any
//! divergence between them is the mip chain and nothing else.
//!
//! `loadCubeTextureWithMipmaps()` is asynchronous on the page — 54 JPEGs
//! through nine `CubeTextureLoader.load()` calls and a `Promise.all` — but the
//! harness only fires its single RAF once the network is idle, so everything
//! inside the `.then()` is always in the graded frame; here it is decoded
//! synchronously and written inline.

use std::rc::Rc;

use three_rs::addons::controls::OrbitControls;
use three_rs::geometries::sphere_geometry;
use three_rs::{
    Color, CubeTextureLoader, Mesh, MeshBasicNodeMaterial, MinFilter, PerspectiveCamera, Renderer,
    RendererParameters, Scene, TextureFilter,
};

pub const INNER_WIDTH: f64 = 800.0;
pub const INNER_HEIGHT: f64 = 500.0;
/// `renderer.setPixelRatio( window.devicePixelRatio )`
pub const DPR: f64 = 1.0;

/// The page's `maxLevel`: `cube_m00_*` is the 256² base and `cube_m08_*` the
/// 1×1 tail, so nine levels in all and eight of them mips.
const MAX_LEVEL: usize = 8;

pub struct App {
    pub renderer: Renderer,
    pub scene: Scene,
    pub camera: PerspectiveCamera,
    /// The page's `controls`.
    pub controls: OrbitControls,
}

fn examples_dir() -> std::path::PathBuf {
    three_rs::testing::three_js_dir().join("examples")
}

/// `loadCubeTextureWithMipmaps()`.
fn load_cube_texture_with_mipmaps() -> three_rs::CubeTexture {
    let loader = CubeTextureLoader::new().set_path(examples_dir().join("textures/cube/angus"));
    let urls = |level: usize| {
        std::array::from_fn::<String, 6, _>(|face| format!("cube_m0{level}_c0{face}.jpg"))
    };

    // `const customizedCubeTexture = mipmaps.shift()`: level 0 becomes the
    // texture and levels 1.. become its `mipmaps`. Only the faces of each
    // level are ever read, so the port loads those rather than a whole
    // `CubeTexture` per level.
    let texture = loader.load(urls(0)).unwrap();
    let mipmaps = (1..=MAX_LEVEL)
        .map(|level| loader.load_images(urls(level)).unwrap())
        .collect();
    texture.set_mipmaps(mipmaps);

    // `colorSpace = SRGBColorSpace` is already what `CubeTextureLoader` sets.
    texture.set_filters(MinFilter::LinearMipmapLinear, TextureFilter::Linear);
    texture.set_generate_mipmaps(false);
    texture
}

pub fn init() -> App {
    let mut camera = PerspectiveCamera::new(50.0, INNER_WIDTH / INNER_HEIGHT, 1.0, 10000.0);
    camera.node.borrow_mut().position.z = 500.0;

    let scene = Scene::new();

    let cube_texture = load_cube_texture_with_mipmaps();

    // model
    let sphere = Rc::new(sphere_geometry(100.0, 128, 128));

    // manual mipmaps
    let mut material = MeshBasicNodeMaterial::new();
    material.color = Color::from_hex(0xffffff);
    material.env_map = Some(cube_texture.clone());

    let mesh = Mesh::new(sphere.clone(), material.clone());
    mesh.borrow_mut().position.set(100.0, 0.0, 0.0);
    scene.add(&mesh);

    // auto mipmaps
    let mut material = material.clone();

    // `cubeTexture.clone()` — a second texture over the same faces, so the
    // backend uploads and mipmaps it separately.
    let auto_cube_texture = cube_texture.clone_texture();
    auto_cube_texture.set_mipmaps(Vec::new());
    auto_cube_texture.set_generate_mipmaps(true);

    material.env_map = Some(auto_cube_texture);

    let mesh = Mesh::new(sphere, material);
    mesh.borrow_mut().position.set(-100.0, 0.0, 0.0);
    scene.add(&mesh);

    //renderer
    let mut renderer = Renderer::new(RendererParameters { antialias: true }).unwrap();
    renderer.set_pixel_ratio(DPR);
    renderer.set_size(INNER_WIDTH, INNER_HEIGHT);

    //controls
    // `const controls = new OrbitControls( camera, renderer.domElement );`
    // with polar limits and no `update()` of the page's own. The constructor's
    // `update()` aims the camera at the default `( 0, 0, 0 )` target, which
    // from ( 0, 0, 500 ) is the `-z` direction it already had, so the graded
    // frame is unchanged; with no pointer events the polar limits never bite.
    let mut controls = OrbitControls::new(&mut camera);
    // The canvas the example renders at, standing in for the element's
    // `clientWidth` / `clientHeight`.
    controls.set_element_size(INNER_WIDTH, INNER_HEIGHT);
    controls.min_polar_angle = std::f64::consts::PI / 4.0;
    controls.max_polar_angle = std::f64::consts::PI / 1.5;

    App {
        renderer,
        scene,
        camera,
        controls,
    }
}

/// The page's `animate()`.
pub fn animate(app: &mut App) {
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
        .unwrap_or_else(|| "target/webgpu_materials_cubemap_mipmaps.png".to_string());
    three_rs::testing::write_png(&path, width, height, &pixels);
    println!("wrote {path} ({width}x{height})");
}
