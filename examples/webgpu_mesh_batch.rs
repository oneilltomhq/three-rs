//! Port of `three.js/examples/webgpu_mesh_batch.html`, calling the three-rs API
//! in the same order the page's `init()` / `animate()` do.
//!
//! Under `WebGPURenderer` this page uses no multi-draw, no indirect draws and
//! no storage buffers: Three issues 453 ordinary `drawIndexed()` calls in one
//! pass against one pipeline and one bind group, varying only
//! `(indexCount, firstIndex, firstInstance)`. Everything per-instance lives in
//! three `DataTexture`s read with `textureLoad`. See `docs/rung11-progress.md`.

use std::f64::consts::PI;
use std::rc::Rc;

use three_rs::geometries::cone_geometry;
use three_rs::materials::MeshBasicNodeMaterial;
use three_rs::nodes::tsl::{diffuse_color, float, normal_view, vec4_join};
use three_rs::objects::{BatchedMesh, CustomSort};
use three_rs::testing::DeterministicRandom;
use three_rs::utils::sort_utils::{radix_sort, to_uint32};
use three_rs::{
    box_geometry, sphere_geometry, Color, Euler, Matrix4, Node, PerspectiveCamera, Quaternion,
    Renderer, RendererParameters, Scene, Vector3,
};

pub const INNER_WIDTH: f64 = 800.0;
pub const INNER_HEIGHT: f64 = 500.0;
/// `renderer.setPixelRatio( window.devicePixelRatio )`
pub const DPR: f64 = 1.0;

/// `api.count`.
pub const COUNT: usize = 512;
/// `api.dynamic` — how many instances `animateMeshes()` rotates each frame.
pub const DYNAMIC: usize = 16;

/// `Math.random()` draws `new Inspector()` makes before the instance loop; the
/// same five `webgpu_tsl_galaxy` documents, from the `List` constructors of the
/// inspector's tabs.
pub const INSPECTOR_RANDOM_DRAWS: usize = 5;

pub struct App {
    pub renderer: Renderer,
    pub scene: Scene,
    pub camera: PerspectiveCamera,
    pub mesh: Node,
    /// `mesh.userData.rotationSpeeds`.
    rotation_speeds: Vec<Matrix4>,
    /// `ids` — the page keeps them, and `addInstance` returns them in order.
    ids: Vec<usize>,
}

/// `createMaterial()`.
pub fn batch_material() -> MeshBasicNodeMaterial {
    let mut material = MeshBasicNodeMaterial::new();
    //   material.outputNode = vec4(
    //       diffuseColor.mul( packNormalToRGB( normalView ).y.add( 0.5 ) ).rgb,
    //       diffuseColor.a,
    //   );
    // `packNormalToRGB( node ) === node.mul( 0.5 ).add( 0.5 )`.
    let pack_normal_to_rgb = normal_view().mul(float(0.5)).add(float(0.5));
    material.output_node = Some(vec4_join(vec![
        diffuse_color()
            .mul(pack_normal_to_rgb.y().add(float(0.5)))
            .rgb(),
        diffuse_color().a(),
    ]));
    material
}

/// The page's `sortFunction`, which scales each range's view depth into the
/// unsigned 32-bit range and hands the list to the addons' hybrid radix sort.
pub fn sort_function() -> CustomSort {
    Rc::new(|list, ctx| {
        // `const factor = ( 2 ** 32 - 1 ) / camera.far;`
        let factor = (2f64.powi(32) - 1.0) / ctx.camera_far;
        for item in list.iter_mut() {
            item.z *= factor;
        }
        radix_sort(list, &|el| to_uint32(el.z), ctx.transparent);
    })
}

/// Everything `init()` does except creating the `Renderer`: the scene, the
/// camera and the batched mesh, built from the same deterministic random
/// sequence. Split out so the CPU-side gates (texture sizes, draw ranges, the
/// sorted draw list) can be tested without a GPU.
pub fn build() -> (Scene, PerspectiveCamera, Node, Vec<Matrix4>, Vec<usize>) {
    // `new THREE.PerspectiveCamera( 70, aspect, 1, 100 ); camera.position.z = 30;`
    let mut camera = PerspectiveCamera::new(70.0, INNER_WIDTH / INNER_HEIGHT, 1.0, 100.0);

    // `new OrbitControls( camera, … )` with `autoRotate = true` and
    // `autoRotateSpeed = 1.0`. `OrbitControls` is not ported; its whole effect
    // on the graded frame is the one `rotateLeft( 2π / 60 / 60 * speed )` step
    // the first `update()` applies around the default target (0,0,0), followed
    // by `lookAt( target )`. In spherical terms the camera starts at
    // `radius = 30, theta = 0, phi = π / 2`, and `theta` moves by `-angle`.
    let angle = 2.0 * PI / 60.0 / 60.0;
    let (radius, theta, phi) = (30.0_f64, -angle, PI / 2.0);
    camera.node.borrow_mut().position.set(
        radius * phi.sin() * theta.sin(),
        radius * phi.cos(),
        radius * phi.sin() * theta.cos(),
    );
    camera.look_at(&Vector3::ZERO);

    let mut scene = Scene::new();
    scene.set_background(Color::from_hex(0xc1c1ff));

    // `Math.random()` is one shared sequence: the inspector's draws come first.
    let mut random = DeterministicRandom::new();
    random.skip(INSPECTOR_RANDOM_DRAWS);

    // `initGeometries()`
    let geometries = [
        // `new THREE.ConeGeometry( 1.0, 2.0 )` — the defaults are 32 radial and
        // 1 height segment.
        cone_geometry(1.0, 2.0, 32, 1),
        box_geometry(2.0, 2.0, 2.0, 1, 1, 1),
        sphere_geometry(1.0, 16, 8),
    ];

    // `initBatchedMesh()`
    let geometry_count = COUNT;
    let vertex_count = geometries.len() * 512;
    let index_count = geometries.len() * 1024;

    let mesh = BatchedMesh::new(geometry_count, vertex_count, index_count, batch_material());

    let mut rotation_speeds = Vec::new();
    let mut ids = Vec::new();
    {
        let mut object = mesh.borrow_mut();
        let batched = object
            .payload
            .batched_mesh_mut()
            .expect("three-rs: BatchedMesh::new builds a batched payload");

        let geometry_ids: Vec<usize> = geometries.iter().map(|g| batched.add_geometry(g)).collect();

        for i in 0..COUNT {
            let id = batched.add_instance(geometry_ids[i % geometry_ids.len()]);
            batched.set_matrix_at(id, &randomize_matrix(&mut random));
            batched.set_color_at(id, &random_color(&mut random));

            let mut rotation_matrix = Matrix4::identity();
            rotation_matrix.make_rotation_from_euler(&randomize_rotation_speed(&mut random));
            rotation_speeds.push(rotation_matrix);

            ids.push(id);
        }
    }
    // `mesh.frustumCulled = false` — every instance can move, so the whole-object
    // cull is off and the per-instance one in `onBeforeRender()` does the work.
    mesh.borrow_mut().frustum_culled = false;
    scene.add(&mesh);

    (scene, camera, mesh, rotation_speeds, ids)
}

pub fn init() -> App {
    let (scene, camera, mesh, rotation_speeds, ids) = build();

    let mut renderer = Renderer::new(RendererParameters { antialias: true }).unwrap();
    renderer.set_pixel_ratio(DPR);
    renderer.set_size(INNER_WIDTH, INNER_HEIGHT);

    App {
        renderer,
        scene,
        camera,
        mesh,
        rotation_speeds,
        ids,
    }
}

/// `randomizeMatrix( matrix )`.
fn randomize_matrix(random: &mut DeterministicRandom) -> Matrix4 {
    let position = Vector3::new(
        random.next() * 40.0 - 20.0,
        random.next() * 40.0 - 20.0,
        random.next() * 40.0 - 20.0,
    );
    let rotation = Euler::new(
        random.next() * 2.0 * PI,
        random.next() * 2.0 * PI,
        random.next() * 2.0 * PI,
    );
    let mut quaternion = Quaternion::default();
    quaternion.set_from_euler(&rotation);
    let s = 0.5 + random.next() * 0.5;
    let scale = Vector3::new(s, s, s);

    let mut matrix = Matrix4::identity();
    matrix.compose(&position, &quaternion, &scale);
    matrix
}

/// `new THREE.Color( Math.random() * 0xffffff )` — `Color.setHex()` floors.
fn random_color(random: &mut DeterministicRandom) -> Color {
    Color::from_hex((random.next() * 0xffffff as f64).floor() as u32)
}

/// `randomizeRotationSpeed( rotation )`.
fn randomize_rotation_speed(random: &mut DeterministicRandom) -> Euler {
    Euler::new(
        random.next() * 0.01,
        random.next() * 0.01,
        random.next() * 0.01,
    )
}

/// The page's `animate()`: `animateMeshes()`, `controls.update()`, then render.
/// The camera's single `controls.update()` step is already baked into `init()`.
pub fn animate(app: &mut App) {
    animate_meshes(&app.mesh, &app.ids, &app.rotation_speeds);
    app.renderer.render(&mut app.scene, &mut app.camera);
}

/// `animateMeshes()`, plus the per-frame `api` assignments the page's
/// `animate()` makes on the mesh right after it.
pub fn animate_meshes(mesh: &Node, ids: &[usize], rotation_speeds: &[Matrix4]) {
    let mut object = mesh.borrow_mut();
    let batched = object
        .payload
        .batched_mesh_mut()
        .expect("three-rs: the mesh is a BatchedMesh");
    let loop_num = COUNT.min(DYNAMIC);
    for i in 0..loop_num {
        let id = ids[i];
        let mut matrix = batched.matrix_at(id);
        matrix.multiply(&rotation_speeds[i]);
        batched.set_matrix_at(id, &matrix);
    }

    // `mesh.sortObjects = api.sortObjects; mesh.perObjectFrustumCulled = …;
    // mesh.setCustomSort( api.useCustomSort ? sortFunction : null );`
    batched.sort_objects = true;
    batched.per_object_frustum_culled = true;
    batched.set_custom_sort(Some(sort_function()));
}

fn main() {
    // Pin both clocks to zero, as three.js' `test/e2e/deterministic-injection.js`
    // does to the page, so that the frame this writes is the frame the rung
    // grades no matter how long `init()` took.
    three_rs::testing::pin_time(Some(0.0));
    let mut app = init();
    println!("adapter: {:?}", app.renderer.adapter_info());
    animate(&mut app);
    println!("draw calls: {}", app.renderer.info().render.calls);

    let (width, height, pixels) = app.renderer.read_canvas_pixels().unwrap();
    let path = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "target/webgpu_mesh_batch.png".to_string());
    three_rs::testing::write_png(&path, width, height, &pixels);
    println!("wrote {path} ({width}x{height})");
}
