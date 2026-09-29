//! Port of `three.js/examples/webgpu_textures_partialupdate.html`, calling the
//! three-rs API in the same order the page's `init()` / `animate()` do.
//!
//! A carbon-fibre plane whose texture is patched every tenth of a second: a
//! 32 x 32 `DataTexture` is refilled with one random colour and
//! `renderer.copyTextureToTexture( dataTexture, diffuseMap, null, position )`
//! copies it into the loaded `Carbon.png` at a random multiple of 32 texels.
//!
//! Under the pinned clock (`Date.now` and `performance.now` both return 0)
//! `timer.getElapsed()` is 0 on every frame, so `elapsedTime - last > 0.1`
//! never holds and the graded frame is the untouched texture — the same frame
//! three's grader sees. The copy path itself is exercised by the rung's second
//! half in `tests/e2e/main.rs`, which moves the clock past 0.1 s.
//!
//! `timer.connect( document )` is left out: it only resets the timer when a
//! hidden tab comes back (see [`Timer`]).
//!
//! # The random sequence
//!
//! Nothing on the page calls `Math.random()` before the first copy: then
//! `MathUtils.randInt( 1, 16 )` twice for the position and one
//! `Math.random()` for the colour, per copy. The port draws them in that order
//! from the harness' seeded sequence.

use std::rc::Rc;

use three_rs::addons::controls::OrbitControls;
use three_rs::testing::DeterministicRandom;
use three_rs::textures::{MinFilter, Texture};
use three_rs::{
    plane_geometry, Color, ColorSpace, Mesh, MeshBasicNodeMaterial, PerspectiveCamera, Renderer,
    RendererParameters, Scene, Timer, Vector2,
};

pub const INNER_WIDTH: f64 = 800.0;
pub const INNER_HEIGHT: f64 = 500.0;
/// `renderer.setPixelRatio( window.devicePixelRatio )`.
pub const DPR: f64 = 1.0;

const WIDTH: u32 = 32;
const HEIGHT: u32 = 32;

pub struct App {
    pub renderer: Renderer,
    pub scene: Scene,
    pub camera: PerspectiveCamera,
    pub timer: Timer,
    pub data_texture: Texture,
    pub diffuse_map: Texture,
    /// `texture.image.data` of `data_texture`, which the page rewrites in place.
    pub data: Vec<u8>,
    pub last: f64,
    pub position: Vector2,
    pub color: Color,
    pub random: DeterministicRandom,
}

fn examples_dir() -> std::path::PathBuf {
    three_rs::testing::three_js_dir().join("examples")
}

pub fn init() -> App {
    let camera = PerspectiveCamera::new(70.0, INNER_WIDTH / INNER_HEIGHT, 0.01, 10.0);
    camera.node.borrow_mut().position.z = 2.0;

    let scene = Scene::new();

    let timer = Timer::new();

    let diffuse_map = three_rs::TextureLoader::new()
        .load(examples_dir().join("textures/carbon/Carbon.png"))
        .expect("Carbon.png");
    diffuse_map.set_color_space(ColorSpace::SRGB);
    diffuse_map.set_min_filter(MinFilter::Linear);
    diffuse_map.set_generate_mipmaps(false);

    let geometry = Rc::new(plane_geometry(2.0, 2.0, 1, 1));
    let mut material = MeshBasicNodeMaterial::new();
    material.map = Some(diffuse_map.clone());

    let mesh = Mesh::new(geometry, material);
    scene.add(&mesh);

    //

    let data = vec![0u8; (WIDTH * HEIGHT * 4) as usize];
    let data_texture = Texture::data_rgba8(WIDTH, HEIGHT, data.clone());
    data_texture.set_color_space(ColorSpace::SRGB);

    //

    let mut parameters = RendererParameters::default();
    parameters.antialias = true;
    let mut renderer = Renderer::new(parameters).unwrap();
    renderer.set_pixel_ratio(DPR);
    renderer.set_size(INNER_WIDTH, INNER_HEIGHT);

    App {
        renderer,
        scene,
        camera,
        timer,
        data_texture,
        diffuse_map,
        data,
        last: 0.0,
        position: Vector2::new(0.0, 0.0),
        color: Color::default(),
        random: DeterministicRandom::new(),
    }
}

/// `THREE.MathUtils.randInt( low, high )` over the page's `Math.random()`.
fn rand_int(random: &mut DeterministicRandom, low: i32, high: i32) -> i32 {
    low + (random.next() * (high - low + 1) as f64).floor() as i32
}

pub fn animate(app: &mut App) {
    app.timer.update();

    let elapsed_time = app.timer.get_elapsed();

    app.renderer.render(&mut app.scene, &mut app.camera);

    if elapsed_time - app.last > 0.1 {
        app.last = elapsed_time;

        app.position.x = (32 * rand_int(&mut app.random, 1, 16) - 32) as f64;
        app.position.y = (32 * rand_int(&mut app.random, 1, 16) - 32) as f64;

        // generate new color data
        update_data_texture(app);

        // perform copy from src to dest texture to a random position

        app.renderer.copy_texture_to_texture(
            &app.data_texture,
            &app.diffuse_map,
            None,
            Some(&app.position),
        );
    }
}

fn update_data_texture(app: &mut App) {
    // generate a random color and update texture data

    // `color.setHex( Math.random() * 0xffffff )`: `setHex` floors its
    // argument and converts from sRGB, so the bytes below are the *linear*
    // channels of that colour — written into an sRGB texture, which the
    // sampler decodes once more. A page quirk, reproduced.
    app.color.set_hex(
        (app.random.next() * 0xffffff as f64) as u32,
        three_rs::math::ColorSpace::SRGB,
    );

    let r = (app.color.r * 255.0).floor() as u8;
    let g = (app.color.g * 255.0).floor() as u8;
    let b = (app.color.b * 255.0).floor() as u8;

    for texel in app.data.as_chunks_mut::<4>().0 {
        texel[0] = r;
        texel[1] = g;
        texel[2] = b;
        // `data[ stride + 3 ] = 1` — one, not 255; the page's own value.
        texel[3] = 1;
    }

    // `texture.needsUpdate = true`.
    app.data_texture.set_data(app.data.clone());
}

/// The page's `onWindowResize()`.
pub fn resize(app: &mut App, width: f64, height: f64) {
    app.camera.aspect = width / height;
    app.camera.update_projection_matrix();

    app.renderer.set_size(width, height);
}

/// The example's controls, for a host that has a pointer. `None` here:
/// the page creates none.
pub fn controls(_app: &mut App) -> Option<&mut OrbitControls> {
    None
}

/// The controls and the camera at once, for a host delivering pointer events.
/// `None` here: the page creates no controls.
pub fn controls_and_camera(_app: &mut App) -> Option<(&mut OrbitControls, &mut PerspectiveCamera)> {
    None
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
        .unwrap_or_else(|| "target/webgpu_textures_partialupdate.png".to_string());
    three_rs::testing::write_png(&path, width, height, &pixels);
    println!("wrote {path} ({width}x{height})");
}
