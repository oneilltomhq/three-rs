//! Port of `three.js/examples/webgpu_pmrem_scene.html`, calling the three-rs
//! API in the same order the page's `init()` / `render()` do.
//!
//! Under the e2e harness the viewport is `400 * 2` x `250 * 2` with no
//! `deviceScaleFactor`, so `window.innerWidth` / `innerHeight` /
//! `devicePixelRatio` are 800, 500 and 1.
//!
//! The page has no `Math.random()` and no clock; `CubeTextureLoader.loadAsync`
//! is awaited, and the harness only fires its single RAF once the network is
//! idle, so the six JPEG faces are always present for the graded frame. Here
//! they are decoded synchronously.
//!
//! # Why this example and not `webgpu_furnace_test`
//!
//! It is the *same* `PMREMGenerator.fromScene`, on an environment scene that
//! is not a solid colour: a cube-texture background plus six coloured spheres.
//! That flips `_sceneToCubeUV`'s `useSolidColor` to false, which is the branch
//! `webgpu_furnace_test` could not reach —
//!
//! * the background is **not** lifted off the scene and drawn once over the
//!   whole atlas; it stays on, and the cube camera draws it as an ordinary
//!   skybox sphere inside each face's viewport;
//! * the six face tiles therefore hold six *different* images, so the viewport
//!   slicing, the `upSign`/`forwardSign` table and the `autoClear = false`
//!   handling are observable for the first time. `assert_face_tiles` in
//!   `tests/e2e/main.rs` reads the atlas back and asserts exactly that, before
//!   it looks at a pixel of the frame.
//!
//! And the consumer side has no PBR in it at all: the big sphere is a
//! `MeshBasicNodeMaterial` whose `colorNode` is
//! `pmremTexture( sceneRT.texture, normalWorld, uniform( 0.5 ) )`, so the
//! cube-UV read is the whole fragment shader
//! (`dump-pmrem_scene/m08_fragment_fragment.wgsl`).
//!
//! # Things that are easy to get wrong
//!
//! **The PMREM is generated before the seventh sphere is added.** The page
//! calls `fromScene( scene )` on the scene that holds the background and the
//! six small spheres, and only then adds the `pmremTexture` sphere to the same
//! scene. Adding it first would put the sphere in its own environment.
//!
//! **The 45 is a plain vertical FoV**, as in `webgpu_furnace_test` and unlike
//! `webgpu_pmrem_test`: `PerspectiveCamera( 45, aspect, 0.25, 20 )`.
//!
//! **No tone mapping.** The page sets none, so the frame is the linear PMREM
//! through the output colour transform only.

use std::rc::Rc;

use three_rs::geometries::sphere_geometry;
use three_rs::nodes::pmrem_node::PmremEnvironment;
use three_rs::nodes::tsl::{normal_world, uniform_settable};
use three_rs::nodes::Type;

use three_rs::{
    Color, CubeTexture, CubeTextureLoader, Mesh, MeshBasicNodeMaterial, PerspectiveCamera,
    Renderer, RendererParameters, Scene, Vector3,
};

pub const INNER_WIDTH: f64 = 800.0;
pub const INNER_HEIGHT: f64 = 500.0;
/// `renderer.setPixelRatio( window.devicePixelRatio )`.
pub const DPR: f64 = 1.0;

/// The six small spheres, in the page's order: colour, then the axis offset
/// applied to the default position.
pub const SPHERES: [(u32, [f64; 3]); 6] = [
    (0x0000ff, [0.0, 0.0, -1.0]),
    (0xff0000, [0.0, 0.0, 1.0]),
    (0xff00ff, [1.0, 0.0, 0.0]),
    (0x00ffff, [-1.0, 0.0, 0.0]),
    (0xffff00, [0.0, -1.0, 0.0]),
    (0x00ff00, [0.0, 1.0, 0.0]),
];

pub struct App {
    pub renderer: Renderer,
    pub scene: Scene,
    pub camera: PerspectiveCamera,
    pub environment: PmremEnvironment,
    /// The six decoded JPEG faces, kept so the e2e gate can hold the atlas'
    /// face tiles against the images they were rendered from.
    pub cube: CubeTexture,
}

fn examples_dir() -> std::path::PathBuf {
    three_rs::testing::three_js_dir().join("examples")
}

pub fn init() -> App {
    // `new THREE.PerspectiveCamera( 45, window.innerWidth / window.innerHeight,
    // 0.25, 20 )`, `camera.position.set( - 1.8, 0.6, 2.7 )`.
    let mut camera = PerspectiveCamera::new(45.0, INNER_WIDTH / INNER_HEIGHT, 0.25, 20.0);
    camera.node.borrow_mut().position.set(-1.8, 0.6, 2.7);

    let mut scene = Scene::new();

    let mut renderer = Renderer::new(RendererParameters { antialias: true }).unwrap();
    renderer.set_pixel_ratio(DPR);
    renderer.set_size(INNER_WIDTH, INNER_HEIGHT);
    // `renderer.toneMapping` is left at `NoToneMapping`; the page sets none.

    // `new OrbitControls( camera, renderer.domElement )`, `minDistance = 2`,
    // `maxDistance = 10`, `controls.update()`. The camera is 3.3 from the
    // origin, so the distance clamp is inert, and with no pointer events the
    // orbit is the identity — `update()` reduces to pointing the camera at the
    // target.
    camera.look_at(&Vector3::ZERO);

    // `new THREE.CubeTextureLoader().setPath( './textures/cube/Park3Med/' )`,
    // then `scene.background = await loader.loadAsync( [ … ] )`.
    let path = examples_dir().join("textures/cube/Park3Med");
    let urls = [
        path.join("px.jpg"),
        path.join("nx.jpg"),
        path.join("py.jpg"),
        path.join("ny.jpg"),
        path.join("pz.jpg"),
        path.join("nz.jpg"),
    ];
    let cube = CubeTextureLoader::new().load(urls).unwrap();
    scene.set_background(cube.clone());

    // Six `SphereGeometry( .2, 64, 64 )` / `MeshBasicMaterial` pairs. The page
    // builds a fresh geometry per mesh; one shared `Rc` is the same 24 192
    // indices six times over.
    let small = Rc::new(sphere_geometry(0.2, 64, 64));
    for (hex, offset) in SPHERES {
        let mut material = MeshBasicNodeMaterial::new();
        material.color = Color::from_hex(hex);

        let mesh = Mesh::new(small.clone(), material);
        {
            let mut object = mesh.borrow_mut();
            object.position.set(offset[0], offset[1], offset[2]);
        }
        scene.add(&mesh);
    }

    // `const sceneRT = new THREE.PMREMGenerator( renderer ).fromScene( scene )`
    // — over the scene as it stands: cube background, six spheres, and *not*
    // the sphere added below.
    let environment = PmremEnvironment::from_scene(&mut renderer, &mut scene).unwrap();

    // `const pmremRoughness = uniform( .5 )`, `pmremTexture( sceneRT.texture,
    // normalWorld, pmremRoughness )`. The GUI slider that writes it is removed
    // by `clean-page.js`, so it stays 0.5.
    let (roughness, _roughness) = uniform_settable(Type::F32, vec![0.5]);
    let mut material = MeshBasicNodeMaterial::new();
    material.color_node = Some(environment.sample(normal_world(), roughness));

    let mesh = Mesh::new(Rc::new(sphere_geometry(0.5, 64, 64)), material);
    scene.add(&mesh);

    App {
        renderer,
        scene,
        camera,
        environment,
        cube,
    }
}

/// The page's `render()`.
pub fn animate(app: &mut App) {
    // `fromScene` built the PMREM eagerly in `init()`; this is a no-op, and is
    // here so the steady-frame assertion runs over the same call the other
    // PMREM examples make.
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
        .unwrap_or_else(|| "target/webgpu_pmrem_scene.png".to_string());
    three_rs::testing::write_png(&path, width, height, &pixels);
    println!("wrote {path} ({width}x{height})");
}
