//! An `InstancedMesh`'s instance matrix is written to the GPU when its
//! version moves and not otherwise (issue #89).
//!
//! three.js' `Attributes.update()` keeps one GPU buffer per attribute and
//! re-writes it only when `attribute.version` has moved. Before #89 the port
//! wrote the whole array on every draw, which at `BatchedText`'s 1 << 17
//! glyphs was most of a frame. This draws the same mesh twice with nothing
//! changed and holds the second frame to zero writes, then moves one instance
//! and holds the next frame to exactly one write, with the pixels moved and
//! matching a fresh renderer's frame of the moved scene.
//!
//! Both of `createInstanceMatrixNode()`'s branches: under the uniform-buffer
//! limit the matrices are a uniform array, over it an instanced vertex
//! buffer; each is its own buffer usage and its own cache entry.
//!
//! One `#[test]` on purpose: each render builds its own device, and cargo runs
//! test functions inside a binary concurrently.

use std::rc::Rc;

use three_rs::geometries::plane_geometry;
use three_rs::math::Matrix4;
use three_rs::{
    BuildCounts, Color, InstancedMesh, MeshBasicNodeMaterial, Node, PerspectiveCamera, Renderer,
    RendererParameters, Scene, Vector3,
};

const WIDTH: f64 = 320.0;
const HEIGHT: f64 = 200.0;
const COLUMNS: usize = 50;

/// `count` small white quads on a grid.
fn scene(count: usize) -> (Scene, PerspectiveCamera, Node) {
    let mut camera = PerspectiveCamera::new(60.0, WIDTH / HEIGHT, 0.1, 100.0);
    camera.node.borrow_mut().position.set(0.0, 0.0, 10.0);
    camera.look_at(&Vector3::ZERO);

    let mut scene = Scene::new();
    scene.set_background(Color::from_hex(0x000000));

    let mut material = MeshBasicNodeMaterial::new();
    material.color = Color::new(1.0, 1.0, 1.0);
    let mesh = InstancedMesh::new(Rc::new(plane_geometry(0.12, 0.12, 1, 1)), material, count);

    let mut matrix = Matrix4::default();
    for i in 0..count {
        let x = -8.0 + 16.0 * ((i % COLUMNS) as f64 + 0.5) / COLUMNS as f64;
        let y = -5.0 + 0.2 * (i / COLUMNS) as f64;
        matrix.make_translation(x, y, 0.0);
        mesh.borrow_mut().set_matrix_at(i, &matrix);
    }
    scene.add(&mesh);
    (scene, camera, mesh)
}

fn new_renderer() -> Renderer {
    let mut renderer = Renderer::new(RendererParameters::default()).unwrap();
    renderer.set_pixel_ratio(1.0);
    renderer.set_size(WIDTH, HEIGHT);
    renderer
}

/// One frame, and the counts it added to the renderer's running totals.
fn frame(
    renderer: &mut Renderer,
    scene: &mut Scene,
    camera: &mut PerspectiveCamera,
) -> (Vec<u8>, BuildCounts) {
    let before = renderer.info().build;
    renderer.render(scene, camera);
    let after = renderer.info().build;
    let (_, _, pixels) = renderer.read_canvas_pixels().unwrap();
    let mut added = after;
    added.buffers_written -= before.buffers_written;
    added.buffers_created -= before.buffers_created;
    (pixels, added)
}

fn version(mesh: &Node) -> u32 {
    mesh.borrow()
        .instance_matrix()
        .expect("an InstancedMesh has an instance matrix")
        .version()
}

/// Pixels brighter than the black background.
fn covered(pixels: &[u8]) -> usize {
    pixels
        .as_chunks::<4>()
        .0
        .iter()
        .filter(|px| px[0] > 16)
        .count()
}

fn check(count: usize) {
    let (mut scene, mut camera, mesh) = scene(count);
    let mut renderer = new_renderer();
    renderer.info_mut().auto_reset = false;

    let (first, built) = frame(&mut renderer, &mut scene, &mut camera);
    println!("{count} instances, frame 1: {}", renderer.info());
    assert!(
        covered(&first) > 0,
        "{count} instances: the first frame drew nothing"
    );
    assert!(
        built.buffers_written >= 1,
        "{count} instances: the first frame wrote no buffer, so the counter is not wired"
    );

    // Nothing changed: no write, and no new buffer.
    let (second, built) = frame(&mut renderer, &mut scene, &mut camera);
    assert_eq!(
        first, second,
        "{count} instances: an unchanged frame changed"
    );
    assert_eq!(
        (built.buffers_written, built.buffers_created),
        (0, 0),
        "{count} instances: a frame with nothing changed wrote or created a buffer"
    );

    // Setting a matrix to the value it already has is not a change.
    let mut same = Matrix4::default();
    {
        let object = mesh.borrow();
        let array = object.instance_matrix().unwrap().array();
        for (e, a) in same.elements.iter_mut().zip(&array[..16]) {
            *e = *a as f64;
        }
    }
    let unchanged = version(&mesh);
    mesh.borrow_mut().set_matrix_at(0, &same);
    assert_eq!(
        version(&mesh),
        unchanged,
        "{count} instances: re-setting an unchanged matrix bumped the version"
    );
    let (_, built) = frame(&mut renderer, &mut scene, &mut camera);
    assert_eq!(
        built.buffers_written, 0,
        "{count} instances: a no-op set wrote"
    );

    // Move instance 0 to the middle of the frame: the version moves once, the
    // next frame writes the one attribute once, into the buffer it has.
    let mut moved = Matrix4::default();
    moved.make_translation(0.0, 2.0, 0.0);
    mesh.borrow_mut().set_matrix_at(0, &moved);
    assert_eq!(
        version(&mesh),
        unchanged + 1,
        "{count} instances: set_matrix_at bumps the version once"
    );
    let (third, built) = frame(&mut renderer, &mut scene, &mut camera);
    println!("{count} instances, after the move: {}", renderer.info());
    assert_eq!(
        (built.buffers_written, built.buffers_created),
        (1, 0),
        "{count} instances: one moved instance is one write into the existing buffer"
    );
    let changed = second.iter().zip(&third).filter(|(a, b)| a != b).count();
    assert!(
        changed > 0,
        "{count} instances: the instance moved but no pixel changed"
    );

    // And what was written is the moved scene: a fresh renderer, which has
    // never seen the old matrices, draws the same frame.
    let mut fresh = new_renderer();
    let (reference, _) = frame(&mut fresh, &mut scene, &mut camera);
    assert_eq!(
        third, reference,
        "{count} instances: the re-written buffer draws differently from a fresh upload"
    );

    // The steady frame after the write writes nothing again.
    let (_, built) = frame(&mut renderer, &mut scene, &mut camera);
    assert_eq!(
        built.buffers_written, 0,
        "{count} instances: the frame after the write wrote again"
    );
}

#[test]
fn instance_matrix_is_written_only_when_its_version_moves() {
    // 100 * 64 bytes: the uniform-buffer branch.
    check(100);
    // 2000 * 64 = 128000 > 65536: the instanced vertex buffer branch.
    check(2000);
}
