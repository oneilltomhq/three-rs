//! Port of `three.js/examples/webgpu_instance_path.html`, calling the
//! three-rs API in the same order the page's `init()` / `animate()` do.
//!
//! Under the e2e harness the viewport is `400 * 2` x `250 * 2` with no
//! `deviceScaleFactor`, so `window.innerWidth` / `innerHeight` /
//! `devicePixelRatio` are 800, 500 and 1.
//!
//! A thousand ico-spheres sit along a heart drawn as a `THREE.Path` of six
//! `bezierCurveTo`s, each placed with `path.getPointAt( i / count )` and
//! jittered by `Math.random()` — the harness' deterministic sequence, five
//! draws per instance (x, y, z jitter, the seed, the hue). They are **one**
//! plain `Mesh` with `mesh.count = 1000`: `RenderObject.getInstanceCount()`
//! reads `object.count` off any object, so the mesh draws 1000 instances and
//! the four `instancedBufferAttribute` nodes feed each its own position,
//! colour, time and seed. The port's [`Mesh::count`](three_rs::objects::Mesh)
//! is that property.
//!
//! `time` is 0 on the graded frame (the harness pins both clocks), so
//! `modTime` is 0 and the instances scaled up by `s1` are the ones with
//! `instanceTime` near 0 or 1 — the cluster at the top of the heart.
//!
//! `renderer.inspector = new Inspector()` is created after the random loop,
//! and this page never calls `createParameters()`, so it draws no
//! `Math.random()` of its own; the `resize` listener never fires.

use std::rc::Rc;

use three_rs::addons::controls::OrbitControls;
use three_rs::extras::{Curve, Path};
use three_rs::geometries::icosahedron_geometry;
use three_rs::math::ColorSpace;
use three_rs::nodes::pmrem_node::PmremEnvironment;
use three_rs::nodes::tsl::{
    abs, distance, float, instanced_data_attribute, mod_float, position_local, screen_uv, time,
    to_const, vec3_join,
};
use three_rs::nodes::Type;
use three_rs::testing::DeterministicRandom;
use three_rs::{
    Background, Color, Mesh, MeshBasicNodeMaterial, MeshStandardNodeMaterial, PerspectiveCamera,
    Renderer, RendererParameters, RoomEnvironment, Scene, ToneMapping, Vector3,
};

pub const INNER_WIDTH: f64 = 800.0;
pub const INNER_HEIGHT: f64 = 500.0;
/// `renderer.setPixelRatio( window.devicePixelRatio )`
pub const DPR: f64 = 1.0;

/// `const count = 1000`.
const COUNT: usize = 1000;

pub struct App {
    pub renderer: Renderer,
    pub scene: Scene,
    pub camera: PerspectiveCamera,
    /// The page's `controls`.
    pub controls: OrbitControls,
}

/// `new THREE.Path().moveTo( ... ).bezierCurveTo( ... ) ...` — the heart.
pub fn heart_path() -> Path {
    let (x, y) = (0.0, 0.0);

    let mut path = Path::new();
    path.move_to(x - 2.5, y - 2.5)
        .bezier_curve_to(x - 2.5, y - 2.5, x - 2.0, y, x, y)
        .bezier_curve_to(x + 3.0, y, x + 3.0, y - 3.5, x + 3.0, y - 3.5)
        .bezier_curve_to(x + 3.0, y - 5.5, x + 1.0, y - 7.7, x - 2.5, y - 9.5)
        .bezier_curve_to(x - 6.0, y - 7.7, x - 8.0, y - 5.5, x - 8.0, y - 3.5)
        .bezier_curve_to(x - 8.0, y - 3.5, x - 8.0, y, x - 5.0, y)
        .bezier_curve_to(x - 3.5, y, x - 2.5, y - 2.5, x - 2.5, y - 2.5);
    path
}

/// The instanced ico-spheres' material: the random loop and the TSL graph,
/// from `// instance data` to `material.colorNode`. Split out of [`init`] so
/// `examples/dump_wgsl.rs` can print its WGSL without a renderer.
pub fn material(path: &Path) -> MeshBasicNodeMaterial {
    let mut material = MeshStandardNodeMaterial::standard(Color::from_hex(0xffffff), 1.0, 0.0);

    // instance data

    let mut random = DeterministicRandom::new();

    let mut positions: Vec<f32> = Vec::with_capacity(COUNT * 3);
    let mut times: Vec<f32> = Vec::with_capacity(COUNT);
    let mut seeds: Vec<f32> = Vec::with_capacity(COUNT);
    let mut colors: Vec<f32> = Vec::with_capacity(COUNT * 3);

    // `const v = new THREE.Vector3()`: `path.getPointAt( t, v )` writes x and
    // y only (a 2D curve `set()`s two components, and `Vector3.set` keeps z
    // when the third argument is undefined); z is overwritten every pass.
    let mut v = Vector3::new(0.0, 0.0, 0.0);
    let mut c = Color::default();

    for i in 0..COUNT {
        let t = i as f64 / COUNT as f64;
        let point = path.get_point_at(t);
        v.x = point.x;
        v.y = point.y;

        v.x += 0.5 - random.next();
        v.y += 0.5 - random.next();
        v.z = 0.5 - random.next();

        positions.extend([v.x as f32, v.y as f32, v.z as f32]);
        times.push(t as f32);
        seeds.push(random.next() as f32);

        // `c.setHSL( ... )` — in `ColorManagement.workingColorSpace`, so
        // no conversion.
        c.set_hsl(
            0.75 + (random.next() * 0.25),
            1.0,
            0.4,
            ColorSpace::LinearSRGB,
        );

        colors.extend([c.r as f32, c.g as f32, c.b as f32]);
    }

    // TSL

    // `instancedBufferAttribute( new InstancedBufferAttribute( array, n ) )`:
    // four separate buffers, as on the page.
    let instance_position = instanced_data_attribute(&Rc::new(positions), 3, 0, Type::Vec3);
    let instance_color = instanced_data_attribute(&Rc::new(colors), 3, 0, Type::Vec3);
    let instance_seed = instanced_data_attribute(&Rc::new(seeds), 1, 0, Type::F32);
    let instance_time = instanced_data_attribute(&Rc::new(times), 1, 0, Type::F32);

    let local_time = instance_time.add(time());
    let mod_time = mod_float(time().mul(float(0.4)), float(1.0));

    let s0 = local_time.add(instance_seed).sin().mul(float(0.25));

    // modTime and instanceTime are in the range [0,1]
    let dist = to_const(None, abs(instance_time.sub(mod_time)));
    // the normalized distance should wrap around 0/1
    let wrap_dist = to_const(
        None,
        dist.greater_than(float(0.5))
            .select(dist.one_minus(), dist.clone()),
    );
    // compute a scale in a range around the current interpolated value
    let s1 = wrap_dist.greater_than(float(0.1)).select(
        float(1.0),
        wrap_dist.remap(float(0.0), float(0.1), float(3.0), float(1.0)),
    );

    let offset = to_const(
        Some("offset"),
        vec3_join(vec![
            instance_position.x(),
            instance_position.y().add(s0),
            instance_position.z(),
        ]),
    );
    material.position_node = Some(position_local().mul(s1).add(offset));
    material.color_node = Some(instance_color);

    material
}

pub fn init() -> App {
    let mut camera = PerspectiveCamera::new(60.0, INNER_WIDTH / INNER_HEIGHT, 0.01, 100.0);
    camera.node.borrow_mut().position.z = 15.0;

    let mut scene = Scene::new();
    // `scene.backgroundNode = screenUV.distance( .5 ).remap( 0, 0.65 ).mix(
    // color( 0x94254c ), color( 0x000000 ) )`; `remap`'s outLow / outHigh
    // default to 0 and 1.
    scene.background = Some(Background::Node(
        distance(screen_uv(), float(0.5))
            .remap(float(0.0), float(0.65), float(0.0), float(1.0))
            .mix(Color::from_hex(0x94254c), Color::from_hex(0x000000)),
    ));

    // generate a path representing a heart shape

    let path = heart_path();

    // generate instanced ico-spheres along the path

    let geometry = Rc::new(icosahedron_geometry(0.1, 0));
    let material = material(&path);

    let mesh = Mesh::new(geometry, material);
    {
        let mut object = mesh.borrow_mut();
        object.position.set(2.5, 5.0, 0.0);
        object.payload.mesh_mut().unwrap().count = Some(COUNT);
        object.frustum_culled = false;
    }

    scene.add(&mesh);

    //

    let mut renderer = Renderer::new(RendererParameters { antialias: true }).unwrap();
    renderer.set_pixel_ratio(DPR);
    renderer.set_size(INNER_WIDTH, INNER_HEIGHT);
    renderer.tone_mapping = ToneMapping::Neutral;

    // `scene.environment = pmremGenerator.fromScene( new RoomEnvironment(), 0.04 ).texture`.
    let mut room = RoomEnvironment::new();
    let environment = PmremEnvironment::from_scene(&mut renderer, &mut room, 0.04).unwrap();
    scene.environment = Some(environment.handle());

    // `controls = new OrbitControls( camera, renderer.domElement );` The
    // constructor's `update()` re-derives the camera position from the
    // spherical offset to `target` (the origin), which is the position it
    // already has, and points the camera at it.
    let mut controls = OrbitControls::new(&mut camera);
    // The canvas the example renders at, standing in for the element's
    // `clientWidth` / `clientHeight`.
    controls.set_element_size(INNER_WIDTH, INNER_HEIGHT);
    controls.enable_damping = true;

    App {
        renderer,
        scene,
        camera,
        controls,
    }
}

/// The page's `animate()`.
pub fn animate(app: &mut App) {
    app.controls.update(&mut app.camera, None);

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
        .unwrap_or_else(|| "target/webgpu_instance_path.png".to_string());
    three_rs::testing::write_png(&path, width, height, &pixels);
    println!("wrote {path} ({width}x{height})");
}
