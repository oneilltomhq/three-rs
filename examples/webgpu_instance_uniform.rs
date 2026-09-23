//! Port of `three.js/examples/webgpu_instance_uniform.html`, calling the
//! three-rs API in the same order the page's `init()` / `animate()` do.
//!
//! Under the e2e harness the viewport is `400 * 2` x `250 * 2` with no
//! `deviceScaleFactor`, so `window.innerWidth` / `innerHeight` /
//! `devicePixelRatio` are 800, 500 and 1.
//!
//! Twelve teapots share **one** `MeshBasicNodeMaterial`, one program and one
//! pipeline, and differ only in a per-object `vec3` uniform. On the page that
//! is a custom `InstanceUniformNode extends THREE.Node` with `updateType =
//! NodeUpdateType.OBJECT`, whose `update( frame )` copies `frame.object.color`
//! into a `uniform( new Color() )`. The port has no place to hang a `.color` on
//! an `Object3D`, so [`three_rs::nodes::tsl::uniform_object`] hands the
//! callback the object and the example answers from a map keyed by
//! `Object3D.id` — the same graph, the same one uniform, twelve values.
//! `docs/nodes.md` §18.
//!
//! `new CubeTextureLoader().load( urls )` is asynchronous on the page, but the
//! harness fires its single RAF only once the network is idle, so the six faces
//! are present for the graded frame; here they are decoded synchronously.
//!
//! `renderer.inspector = new Inspector()` only registers the renderer with the
//! inspector panel — this page never calls `createParameters()`, so unlike
//! `webgpu_tsl_galaxy` the inspector draws no `Math.random()` — and the
//! `resize` listener never fires.

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use three_rs::nodes::tsl::{
    cube_texture, float, material_env_rotation, reflect_vector, uniform_object, vec4_join,
};
use three_rs::nodes::Type;
use three_rs::testing::DeterministicRandom;
use three_rs::{
    teapot_geometry, Color, CubeTextureLoader, GridHelper, Mesh, MeshBasicNodeMaterial,
    PerspectiveCamera, Renderer, RendererParameters, Scene, Vector3,
};

pub const INNER_WIDTH: f64 = 800.0;
pub const INNER_HEIGHT: f64 = 500.0;
/// `renderer.setPixelRatio( window.devicePixelRatio )`
pub const DPR: f64 = 1.0;

pub struct App {
    pub renderer: Renderer,
    pub scene: Scene,
    pub camera: PerspectiveCamera,
    /// The page's `objects` array — the twelve teapots, without the grid.
    pub objects: Vec<three_rs::core::Node>,
    /// The page's `mesh.color`, which an `Object3D` has nowhere to keep.
    pub colors: Rc<RefCell<HashMap<u32, Color>>>,
}

fn examples_dir() -> std::path::PathBuf {
    three_rs::testing::three_js_dir().join("examples")
}

pub fn init() -> App {
    let mut camera = PerspectiveCamera::new(45.0, INNER_WIDTH / INNER_HEIGHT, 1.0, 4000.0);
    camera.node.borrow_mut().position.set(0.0, 200.0, 1200.0);

    let scene = Scene::new();

    // Grid

    let helper = GridHelper::new(
        1000.0,
        40,
        Color::from_hex(0x303030),
        Color::from_hex(0x303030),
    );
    helper.borrow_mut().position.y = -75.0;
    scene.add(&helper);

    // CubeMap

    let path = examples_dir().join("textures/cube/SwedishRoyalCastle/");
    let c_texture = CubeTextureLoader::new()
        .load([
            path.join("px.jpg"),
            path.join("nx.jpg"),
            path.join("py.jpg"),
            path.join("ny.jpg"),
            path.join("pz.jpg"),
            path.join("nz.jpg"),
        ])
        .unwrap();

    // Materials

    let colors: Rc<RefCell<HashMap<u32, Color>>> = Rc::new(RefCell::new(HashMap::new()));
    let instance_uniform = {
        let colors = colors.clone();
        // `update( frame ) { this.uniformNode.value.copy( frame.object.color ) }`.
        // A mesh the page never gave a `.color` would read `undefined` there;
        // here it reads black, which is the same "the application forgot"
        // rather than a second code path.
        uniform_object(Type::Vec3, move |object| {
            let color = colors.borrow().get(&object.id).copied().unwrap_or_default();
            vec![color.r, color.g, color.b]
        })
    };

    // `cubeTexture( cTexture )` with no uv: `CubeTextureNode.getDefaultUV()` is
    // `reflectVector` for a `CubeReflectionMapping`, and `setupUV()` rotates it
    // by `materialEnvRotation` (the identity here) and negates x for WebGPU.
    let cube_texture_node = cube_texture(
        &c_texture,
        material_env_rotation().mul(vec4_join(vec![reflect_vector(), float(1.0)])),
    );

    let mut material = MeshBasicNodeMaterial::new();
    material.color_node = Some(instance_uniform.clone().add(cube_texture_node.clone()));
    material.emissive_node = Some(instance_uniform.mul(cube_texture_node));

    // Geometry

    let geometry = Rc::new(teapot_geometry(50.0, 18));

    // `Math.random()` is the harness' deterministic sequence: four draws per
    // mesh — the colour, then the three Euler angles.
    let mut random = DeterministicRandom::new();
    let mut objects: Vec<three_rs::core::Node> = Vec::new();
    for _ in 0..12 {
        let mesh = Mesh::new(geometry.clone(), material.clone());
        {
            let mut object = mesh.borrow_mut();
            // `new THREE.Color( Math.random() * 0xffffff )` — `Color.setHex()`
            // floors the float before it splits the channels.
            let hex = (random.next() * 0xffffff as f64).floor() as u32;
            colors.borrow_mut().insert(object.id, Color::from_hex(hex));
            object.position.x = (objects.len() % 4) as f64 * 200.0 - 300.0;
            object.position.z = (objects.len() / 4) as f64 * 200.0 - 200.0;
            object.set_rotation(
                random.next() * 200.0 - 100.0,
                random.next() * 200.0 - 100.0,
                random.next() * 200.0 - 100.0,
            );
        }
        objects.push(mesh.clone());
        scene.add(&mesh);
    }

    //

    let mut renderer = Renderer::new(RendererParameters { antialias: true }).unwrap();
    renderer.set_pixel_ratio(DPR);
    renderer.set_size(INNER_WIDTH, INNER_HEIGHT);

    // `new OrbitControls( camera, renderer.domElement )`: the constructor's
    // `update()` re-derives the camera position from the spherical offset to
    // `target` (the origin) and points the camera at it. With no pointer events
    // the offset is unchanged and the distance, 1216.55, is inside
    // `[ minDistance, maxDistance ]`, so all that survives is the `lookAt`.
    camera.look_at(&Vector3::ZERO);

    App {
        renderer,
        scene,
        camera,
        objects,
        colors,
    }
}

/// The page's `animate()`.
pub fn animate(app: &mut App) {
    for object in &app.objects {
        let mut object = object.borrow_mut();
        let (x, y, z) = (object.rotation.x, object.rotation.y, object.rotation.z);
        object.set_rotation(x + 0.01, y + 0.005, z);
    }

    app.renderer.render(&mut app.scene, &mut app.camera);
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
        .unwrap_or_else(|| "target/webgpu_instance_uniform.png".to_string());
    three_rs::testing::write_png(&path, width, height, &pixels);
    println!("wrote {path} ({width}x{height})");
}
