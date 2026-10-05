//! Port of `three.js/examples/webgpu_instance_sprites.html`, calling the
//! three-rs API in the same order the page's `init()` / `render()` do.
//!
//! One `Sprite` with `count = 10000`: a single instanced draw of the sprite
//! quad, placed per instance by `instancedBufferAttribute( positionAttribute )`
//! and turned by `time.add( instanceIndex ).sin()`.
//!
//! Under the e2e harness the viewport is `400 * 2` x `250 * 2` with no
//! `deviceScaleFactor`, so `window.innerWidth` / `innerHeight` /
//! `devicePixelRatio` are 800, 500 and 1; `Date.now()` is 0 and no pointer
//! ever moves, so `mouseX` / `mouseY` stay 0.
//!
//! The page builds its 30000 random positions first and only then constructs
//! the `Inspector`, whose own `Math.random()` draws therefore come after them
//! and do not shift the sequence the positions read.
//!
//! The page's `gui.add( material, 'sizeAttenuation' )` panel is the
//! inspector's; the port has none, and the graded frame keeps the default.

use std::rc::Rc;

use three_rs::addons::controls::OrbitControls;
use three_rs::math::ColorSpace;
use three_rs::nodes::tsl::{instance_index, instanced_data_attribute, time, uniform_value};
use three_rs::nodes::Type;
use three_rs::testing::DeterministicRandom;
use three_rs::utils::date_now_ms;
use three_rs::{
    Color, FogExp2, MeshBasicNodeMaterial, ObjectRef, PerspectiveCamera, Renderer,
    RendererParameters, Scene, Sprite, TextureLoader, Vector3,
};

pub const INNER_WIDTH: f64 = 800.0;
pub const INNER_HEIGHT: f64 = 500.0;
/// `renderer.setPixelRatio( window.devicePixelRatio )`
pub const DPR: f64 = 1.0;

const COUNT: usize = 10000;

pub struct App {
    pub renderer: Renderer,
    pub scene: Scene,
    pub camera: PerspectiveCamera,
    pub particles: ObjectRef,
    pub mouse_x: f64,
    pub mouse_y: f64,
}

fn examples_dir() -> std::path::PathBuf {
    three_rs::testing::three_js_dir().join("examples")
}

pub fn init() -> App {
    let mut random = DeterministicRandom::new();

    let camera = PerspectiveCamera::new(55.0, INNER_WIDTH / INNER_HEIGHT, 2.0, 2000.0);
    camera.node.borrow_mut().position.z = 1000.0;

    let mut scene = Scene::new();
    scene.fog = Some(FogExp2::new(Color::from_hex(0x000000), 0.001).into());

    // positions

    let mut positions = Vec::with_capacity(COUNT * 3);

    for _ in 0..COUNT {
        positions.push((2000.0 * random.next() - 1000.0) as f32);
        positions.push((2000.0 * random.next() - 1000.0) as f32);
        positions.push((2000.0 * random.next() - 1000.0) as f32);
    }

    // `new THREE.InstancedBufferAttribute( new Float32Array( positions ), 3 )`
    let positions = Rc::new(positions);

    // texture

    let map = TextureLoader::new()
        .load(examples_dir().join("textures/sprites/snowflake1.png"))
        .unwrap();
    map.set_color_space(three_rs::ColorSpace::Srgb);

    // material

    let mut material = MeshBasicNodeMaterial::sprite();
    material.size_attenuation = true;
    material.map = Some(map.clone());
    material.alpha_map = Some(map);
    material.alpha_test = 0.1;
    material.color.set_hsl(1.0, 0.3, 0.7, ColorSpace::Srgb);
    material.position_node = Some(instanced_data_attribute(&positions, 3, 0, Type::Vec3));
    material.rotation_node = Some(time().add(instance_index()).sin());
    material.scale_node = Some(uniform_value(Type::F32, vec![15.0]));

    // sprites

    let particles = Sprite::new(material);
    particles
        .borrow_mut()
        .payload
        .sprite_mut()
        .expect("a Sprite's payload is a sprite")
        .count = COUNT;

    scene.add(&particles);

    //

    let mut renderer = Renderer::new(RendererParameters::default()).unwrap();
    renderer.set_pixel_ratio(DPR);
    renderer.set_size(INNER_WIDTH, INNER_HEIGHT);

    App {
        renderer,
        scene,
        camera,
        particles,
        mouse_x: 0.0,
        mouse_y: 0.0,
    }
}

/// The page's `animate()` / `render()`.
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

/// The example's controls, for a host that has a pointer. `None` here: the
/// page follows the pointer itself rather than creating controls.
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
    let time = date_now_ms() * 0.00005;

    {
        let mut camera = app.camera.node.borrow_mut();
        camera.position.x += (app.mouse_x - camera.position.x) * 0.05;
        camera.position.y += (-app.mouse_y - camera.position.y) * 0.05;
    }

    // `camera.lookAt( scene.position )`
    app.camera.look_at(&Vector3::ZERO);

    let h = (360.0 * (1.0 + time) % 360.0) / 360.0;
    app.particles
        .borrow_mut()
        .payload
        .sprite_mut()
        .expect("a Sprite's payload is a sprite")
        .material
        .color
        .set_hsl(h, 0.5, 0.5, ColorSpace::LinearSrgb);
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
        .unwrap_or_else(|| "target/webgpu_instance_sprites.png".to_string());
    three_rs::testing::write_png(&path, width, height, &pixels);
    println!("wrote {path} ({width}x{height})");
}
