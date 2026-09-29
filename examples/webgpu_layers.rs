//! Port of `three.js/examples/webgpu_layers.html`, calling the three-rs API in
//! the same order the page's `init()` / `animate()` do.
//!
//! Under the e2e harness the viewport is `400 * 2` x `250 * 2` with no
//! `deviceScaleFactor`, so `window.innerWidth` / `innerHeight` /
//! `devicePixelRatio` are 800, 500 and 1.
//!
//! Three plain `Mesh`es of 2500 instances each (`mesh.count`), every one on
//! its own layer (`particles.layers.set( i )`), with the camera enabled on
//! layers 0, 1 and 2 — so `_projectObject`'s `layers.test( camera.layers )`
//! passes for all three. The page's GUI toggles a camera layer; the graded
//! frame is the untouched default.
//!
//! Each mesh's instance data comes from `MathUtils.randFloat` and
//! `Math.random()`, the harness' deterministic sequence: eight draws per
//! instance (three `randFloat` for the position, two for the direction, three
//! `Math.random()` for the rotation), three meshes in order. `getMaterial()`
//! runs before the renderer and its `Inspector` exist, so the Inspector's own
//! draws come after all of them and do not shift the sequence.
//!
//! `time` is 0 on the graded frame (the harness pins both clocks), so each
//! petal sits at `position + direction * ( timeOffset * 50 )`, rotated by
//! `rotation * timeOffset * 20` about its own origin.

use std::rc::Rc;

use three_rs::addons::controls::OrbitControls;
use three_rs::geometries::plane_geometry;
use three_rs::materials::Side;
use three_rs::nodes::tsl::{
    color, distance, float, instanced_data_attribute, mod_float, position_local, rotate, screen_uv,
    time, vec2,
};
use three_rs::nodes::Type;
use three_rs::testing::DeterministicRandom;
use three_rs::{
    Background, Color, ColorSpace, Mesh, MeshBasicNodeMaterial, PerspectiveCamera, Renderer,
    RendererParameters, Scene, Texture, TextureLoader, Vector3,
};

pub const INNER_WIDTH: f64 = 800.0;
pub const INNER_HEIGHT: f64 = 500.0;
/// `renderer.setPixelRatio( window.devicePixelRatio )`
pub const DPR: f64 = 1.0;

/// `const count = 2500`.
const COUNT: usize = 2500;

/// `const colors = [ 0xD70654, 0xFFD95F, 0xB8D576 ]`.
const COLORS: [u32; 3] = [0xD70654, 0xFFD95F, 0xB8D576];

pub struct App {
    pub renderer: Renderer,
    pub scene: Scene,
    pub camera: PerspectiveCamera,
}

fn examples_dir() -> std::path::PathBuf {
    three_rs::testing::three_js_dir().join("examples")
}

/// `THREE.MathUtils.randFloat( low, high )` — `low + Math.random() * ( high - low )`,
/// drawn from the harness' sequence.
fn rand_float(random: &mut DeterministicRandom, low: f64, high: f64) -> f64 {
    low + random.next() * (high - low)
}

/// The page's `getMaterial( count, color, sprite )`: the instance data and the
/// TSL graph. `pub` so `examples/dump_wgsl.rs` can print its WGSL without a
/// renderer.
pub fn get_material(
    random: &mut DeterministicRandom,
    count: usize,
    color: u32,
    sprite: Option<&Texture>,
) -> MeshBasicNodeMaterial {
    // instance data

    let mut positions: Vec<f32> = Vec::with_capacity(count * 3);
    let mut rotations: Vec<f32> = Vec::with_capacity(count * 3);
    let mut directions: Vec<f32> = Vec::with_capacity(count * 3);
    let mut time_offsets: Vec<f32> = Vec::with_capacity(count);

    let mut v = Vector3::new(0.0, 0.0, 0.0);

    for i in 0..count {
        // Arguments are evaluated left to right, as in JS.
        let px = rand_float(random, -25.0, -20.0);
        let py = rand_float(random, -10.0, 50.0);
        let pz = rand_float(random, -5.0, 5.0);
        positions.extend([px as f32, py as f32, pz as f32]);

        let vx = rand_float(random, 0.7, 0.9);
        let vy = rand_float(random, -0.3, -0.15);
        v.set(vx, vy, 0.0);
        v.normalize();

        let (rx, ry, rz) = (random.next(), random.next(), random.next());
        rotations.extend([rx as f32, ry as f32, rz as f32]);

        directions.extend([v.x as f32, v.y as f32, v.z as f32]);

        time_offsets.push((i as f64 / count as f64) as f32);
    }

    // material

    let mut material = MeshBasicNodeMaterial::new();
    material.color = Color::from_hex(color);
    material.map = sprite.cloned();
    material.alpha_map = sprite.cloned();
    material.alpha_test = 0.1;
    material.side = Side::Double;
    material.force_single_pass = true;

    // TSL

    // `instancedBufferAttribute( new InstancedBufferAttribute( array, n ) )`:
    // four separate buffers, as on the page.
    let instance_position = instanced_data_attribute(&Rc::new(positions), 3, 0, Type::Vec3);
    let instance_direction = instanced_data_attribute(&Rc::new(directions), 3, 0, Type::Vec3);
    let instance_rotation = instanced_data_attribute(&Rc::new(rotations), 3, 0, Type::Vec3);

    let local_time = instanced_data_attribute(&Rc::new(time_offsets), 1, 0, Type::F32)
        .add(time().mul(float(0.02)));
    let mod_time = mod_float(local_time, float(1.0));

    let rotated_position = rotate(
        position_local(),
        instance_rotation.mul(mod_time.mul(float(20.0))),
    );
    material.position_node = Some(
        rotated_position
            .add(instance_position)
            .add(instance_direction.mul(mod_time.mul(float(50.0)))),
    );

    material
}

pub fn init() -> App {
    let mut random = DeterministicRandom::new();

    let camera = PerspectiveCamera::new(60.0, INNER_WIDTH / INNER_HEIGHT, 0.1, 100.0);
    {
        let mut node = camera.node.borrow_mut();
        node.layers.enable(0); // enabled by default
        node.layers.enable(1);
        node.layers.enable(2);

        node.position.z = 10.0;
    }

    let mut scene = Scene::new();

    let horizontal_effect = screen_uv().x().mix(color(0xf996ae), color(0xf6f0a3));
    let light_effect = distance(screen_uv(), vec2(0.5, 1.0))
        .one_minus()
        .mul(color(0xd9b6fd));

    scene.background = Some(Background::Node(horizontal_effect.add(light_effect)));

    let sprite = TextureLoader::new()
        .load(examples_dir().join("textures/sprites/blossom.png"))
        .unwrap();
    sprite.set_color_space(ColorSpace::Srgb);

    let geometry = Rc::new(plane_geometry(0.25, 0.25, 1, 1));

    for (i, &hex) in COLORS.iter().enumerate() {
        let particles = Mesh::new(
            geometry.clone(),
            get_material(&mut random, COUNT, hex, Some(&sprite)),
        );
        {
            let mut object = particles.borrow_mut();
            object.layers.set(i as u32);
            object.payload.mesh_mut().unwrap().count = Some(COUNT);
        }
        scene.add(&particles);
    }

    let mut parameters = RendererParameters::default();
    parameters.antialias = true;
    let mut renderer = Renderer::new(parameters).unwrap();
    renderer.set_pixel_ratio(DPR);
    renderer.set_size(INNER_WIDTH, INNER_HEIGHT);

    // `renderer.inspector = new Inspector()` and its `createParameters( 'Layers' )`
    // GUI come after every draw of the random sequence above, so nothing here
    // shifts it; the GUI's `camera.layers.toggle()` callbacks never run on the
    // graded frame.

    App {
        renderer,
        scene,
        camera,
    }
}

/// The page's `animate()`.
pub fn animate(app: &mut App) {
    app.renderer.render(&mut app.scene, &mut app.camera);
}

/// The page's `onWindowResize()`.
pub fn resize(app: &mut App, width: f64, height: f64) {
    app.camera.aspect = width / height;
    app.camera.update_projection_matrix();
    app.renderer.set_size(width, height);
}

/// The page creates no controls.
pub fn controls(_app: &mut App) -> Option<&mut OrbitControls> {
    None
}

/// The page creates no controls.
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
        .unwrap_or_else(|| "target/webgpu_layers.png".to_string());
    three_rs::testing::write_png(&path, width, height, &pixels);
    println!("wrote {path} ({width}x{height})");
}
