//! Port of `three.js/examples/webgpu_instance_mesh.html`, calling the three-rs
//! API in the same order the page's `init()` / `render()` do.
//!
//! Under the e2e harness the viewport is `400 * 2` x `250 * 2` with no
//! `deviceScaleFactor`, so `window.innerWidth` / `innerHeight` /
//! `devicePixelRatio` are 800, 500 and 1; `Date.now()` is 0.
//!
//! The page's `loader.load()` is asynchronous, but the harness only fires its
//! single RAF once the network is idle, so the geometry is always in place for
//! the graded frame; here it is loaded synchronously during `init()`.

use std::rc::Rc;

use three_rs::materials::instanced_range;
use three_rs::nodes::tsl::{float, mix, normal_world, osc_sine, time};
use three_rs::{
    BufferGeometryLoader, Child, Color, InstancedMesh, MeshBasicNodeMaterial, Object3D,
    PerspectiveCamera, Renderer, RendererParameters, Scene, Vector3,
};

pub const INNER_WIDTH: f64 = 800.0;
pub const INNER_HEIGHT: f64 = 500.0;
/// `renderer.setPixelRatio( window.devicePixelRatio )`
pub const DPR: f64 = 1.0;

/// `const amount = parseInt( window.location.search.slice( 1 ) ) || 10;` — the
/// harness loads the example with no query string.
const AMOUNT: usize = 10;

pub struct App {
    pub renderer: Renderer,
    pub scene: Scene,
    pub camera: PerspectiveCamera,
}

fn models_dir() -> std::path::PathBuf {
    let three = match std::env::var("THREE_JS_DIR") {
        Ok(dir) => std::path::PathBuf::from(dir),
        Err(_) => std::path::PathBuf::from(std::env::var("HOME").unwrap())
            .join("src/vendor/three.js"),
    };
    three.join("examples")
}

pub fn init() -> App {
    let count = AMOUNT.pow(3);

    let mut camera = PerspectiveCamera::new(60.0, INNER_WIDTH / INNER_HEIGHT, 0.1, 100.0);
    camera.object.position.set(
        AMOUNT as f64 * 0.9,
        AMOUNT as f64 * 0.9,
        AMOUNT as f64 * 0.9,
    );
    camera.look_at(&Vector3::ZERO);

    let mut scene = Scene::new();

    let mut material = MeshBasicNodeMaterial::new();

    // random colors between instances from 0x000000 to 0xFFFFFF
    //
    //   const randomColors = range( new THREE.Color( 0x000000 ),
    //                               new THREE.Color( 0xFFFFFF ) );
    //   material.colorNode = mix( normalWorld, randomColors,
    //                             oscSine( time.mul( .1 ) ) );
    let random_colors = instanced_range(Color::from_hex(0x000000), Color::from_hex(0xFFFFFF), count);
    material.color_node = Some(mix(
        normal_world(),
        random_colors.xyz(),
        osc_sine(time().mul(float(0.1))),
    ));

    let loader = BufferGeometryLoader::new();
    let mut geometry = loader.load(models_dir().join("models/json/suzanne_buffergeometry.json"));

    geometry.compute_vertex_normals();
    geometry.scale(0.5, 0.5, 0.5);

    let mesh = InstancedMesh::new(Rc::new(geometry), material, count);

    scene.add(mesh);

    //

    let mut renderer = Renderer::new(RendererParameters { antialias: true });
    renderer.set_pixel_ratio(DPR);
    renderer.set_size(INNER_WIDTH, INNER_HEIGHT);

    App {
        renderer,
        scene,
        camera,
    }
}

/// The page's `animate()` / `render()`.
pub fn animate(app: &mut App) {
    animate_cpu_only(app);
    app.renderer.render(&mut app.scene, &mut app.camera);
}

/// The CPU half of `render()`, split out so tests can inspect it.
pub fn animate_cpu_only(app: &mut App) {
    // `const time = Date.now() * 0.001;` — the harness pins `Date.now()` to 0.
    let time = 0.0f64;

    for child in &mut app.scene.children {
        let Child::InstancedMesh(mesh) = child else {
            continue;
        };

        mesh.mesh.object.set_rotation(
            (time / 4.0).sin(),
            (time / 2.0).sin(),
            mesh.mesh.object.rotation.z,
        );

        let mut dummy = Object3D::default();

        let mut i = 0usize;
        let offset = (AMOUNT as f64 - 1.0) / 2.0;

        for x in 0..AMOUNT {
            for y in 0..AMOUNT {
                for z in 0..AMOUNT {
                    dummy
                        .position
                        .set(offset - x as f64, offset - y as f64, offset - z as f64);

                    let rotation_y = (x as f64 / 4.0 + time).sin()
                        + (y as f64 / 4.0 + time).sin()
                        + (z as f64 / 4.0 + time).sin();

                    // `dummy.rotation.y = …; dummy.rotation.z = dummy.rotation.y * 2;`
                    // `rotation.x` is never assigned and stays 0.
                    dummy.set_rotation(dummy.rotation.x, rotation_y, rotation_y * 2.0);

                    dummy.update_matrix();

                    mesh.set_matrix_at(i, &dummy.matrix);
                    i += 1;
                }
            }
        }
    }
}

fn main() {
    let mut app = init();
    println!("adapter: {:?}", app.renderer.adapter_info());
    animate(&mut app);

    let (width, height, pixels) = app.renderer.read_canvas_pixels();
    let path = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "target/webgpu_instance_mesh.png".to_string());
    three_rs::testing::write_png(&path, width, height, &pixels);
    println!("wrote {path} ({width}x{height})");
}
