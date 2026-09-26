//! Port of `three.js/examples/webgpu_sprites.html`, calling the three-rs API in
//! the same order the page's `init()` / `render()` do.
//!
//! Two hundred `Sprite`s share one `SpriteNodeMaterial` and differ only in
//! their transform and in `sprite.userData.rotation`, which the material reads
//! per draw through `userData( 'rotation', 'float' )` (`docs/nodes.md` §41).
//!
//! Under the e2e harness the viewport is `400 * 2` x `250 * 2` with no
//! `deviceScaleFactor`, so `window.innerWidth` / `innerHeight` /
//! `devicePixelRatio` are 800, 500 and 1; `Date.now()` is 0.
//!
//! The page's `textureLoader.load()` is asynchronous and its callback sets
//! `imageWidth` / `imageHeight`, which `render()` scales every sprite by. The
//! harness fires its single RAF only once the network is idle, so the graded
//! frame always sees the loaded image's size; here the texture is loaded
//! synchronously during `init()` and the callback's two assignments run there.

use three_rs::addons::controls::OrbitControls;
use three_rs::nodes::tsl::{color, fog, range_fog_factor, texture, user_data, uv};
use three_rs::nodes::Type;
use three_rs::testing::DeterministicRandom;
use three_rs::utils::date_now_ms;
use three_rs::{
    Color, Group, MeshBasicNodeMaterial, Node, PerspectiveCamera, Renderer, RendererParameters,
    Scene, Sprite, TextureLoader,
};

pub const INNER_WIDTH: f64 = 800.0;
pub const INNER_HEIGHT: f64 = 500.0;
/// `renderer.setPixelRatio( window.devicePixelRatio )`
pub const DPR: f64 = 1.0;

pub struct App {
    pub renderer: Renderer,
    pub scene: Scene,
    pub camera: PerspectiveCamera,
    pub group: Node,
    pub image_width: f64,
    pub image_height: f64,
}

fn examples_dir() -> std::path::PathBuf {
    three_rs::testing::three_js_dir().join("examples")
}

pub fn init() -> App {
    let mut random = DeterministicRandom::new();

    let width = INNER_WIDTH;
    let height = INNER_HEIGHT;

    let camera = PerspectiveCamera::new(60.0, width / height, 1.0, 2100.0);
    camera.node.borrow_mut().position.z = 1500.0;

    let mut scene = Scene::new();
    scene.set_background(Color::from_hex(0x000000));
    scene.fog_node = Some(fog(color(0x0000ff), range_fog_factor(1500.0, 2100.0)));

    // create sprites

    let amount = 200;
    let radius = 500.0;

    let map = TextureLoader::new()
        .load(examples_dir().join("textures/sprite1.png"))
        .unwrap();

    // The load callback: `imageWidth = map.image.width; imageHeight = …`.
    let (image_width, image_height) = map.size();

    let group = Group::new();

    let texture_node = texture(&map);

    let mut material = MeshBasicNodeMaterial::sprite();
    material.color_node = Some(texture_node.mul(uv()).mul(2.0).saturate());
    material.opacity_node = Some(texture_node.w());
    // get value of: sprite.userData.rotation
    material.rotation_node = Some(user_data("rotation", Type::F32));

    for _ in 0..amount {
        let x = random.next() - 0.5;
        let y = random.next() - 0.5;
        let z = random.next() - 0.5;

        let sprite = Sprite::new(material.clone());

        {
            let mut object = sprite.borrow_mut();
            object.position.set(x, y, z);
            object.position.normalize();
            object.position.multiply_scalar(radius);

            // individual rotation per sprite
            object
                .user_data
                .insert("rotation".to_owned(), serde_json::json!(0.0));
        }

        group.add(&sprite);
    }

    scene.add(&group);

    //

    let mut renderer = Renderer::new(RendererParameters { antialias: true }).unwrap();
    renderer.set_pixel_ratio(DPR);
    renderer.set_size(INNER_WIDTH, INNER_HEIGHT);

    App {
        renderer,
        scene,
        camera,
        group,
        image_width: image_width as f64,
        image_height: image_height as f64,
    }
}

/// The page's `render()`.
pub fn animate(app: &mut App) {
    animate_cpu_only(app);
    app.renderer.render(&mut app.scene, &mut app.camera);
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

/// The CPU half of `render()`, split out so tests can inspect it.
pub fn animate_cpu_only(app: &mut App) {
    let time = date_now_ms() / 1000.0;

    let children = app.group.children();
    let l = children.len();
    for (i, sprite) in children.iter().enumerate() {
        let mut sprite = sprite.borrow_mut();
        let scale = (time + sprite.position.x * 0.01).sin() * 0.3 + 1.0;

        // `sprite.userData.rotation += 0.1 * ( i / l );`
        let rotation = sprite
            .user_data
            .get("rotation")
            .and_then(serde_json::Value::as_f64)
            .unwrap_or(0.0);
        sprite.user_data.insert(
            "rotation".to_owned(),
            serde_json::json!(rotation + 0.1 * (i as f64 / l as f64)),
        );
        sprite
            .scale
            .set(scale * app.image_width, scale * app.image_height, 1.0);
    }

    let mut group = app.group.borrow_mut();
    group.set_rotation(time * 0.5, time * 0.75, time * 1.0);
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
        .unwrap_or_else(|| "target/webgpu_sprites.png".to_string());
    three_rs::testing::write_png(&path, width, height, &pixels);
    println!("wrote {path} ({width}x{height})");
}
