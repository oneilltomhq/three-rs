//! Rung 12, step 1's gate: `Points` rendered headless through the real
//! renderer, read back, and checked against pixels worked out from the camera
//! alone. Nothing here is derived from an image.
//!
//! `new OrthographicCamera( 0, W, H, 0, 0, 1 )` with the camera at `z = 1`
//! makes one world unit one pixel, so a vertex at world `( x, y )` lands in
//! framebuffer column `x` and row `H - y`; a vertex at `i + 0.5` sits on the
//! centre of column `i`, which is the one place a point's one covered pixel is
//! not a tie. What the assertions catch, all silent-wrong-pixel failures:
//!
//! - **`point-list` reaches the pipeline.** `Primitive::of` reads the *object*,
//!   not the material (`WebGPUUtils.getPrimitiveTopology( object, material )`).
//!   With the `triangle-list` a `Mesh` gets, 64 points would draw 21 filled
//!   triangles and every "one pixel, neighbours dark" assertion would fail.
//! - **A point covers exactly one pixel.** wgpu has no point size; three.js'
//!   `PointsNodeMaterial` in this example does not set one either.
//! - **The far-plane trap.** `depthCompare` is `less-equal`, and with
//!   `near = 0`, `far = 1`, camera at `z = 1`, a particle at `z = 0` lands
//!   *exactly* on the far plane at NDC depth 1. Under `less` nothing draws at
//!   all — a whole-frame black failure that the graded image of
//!   `webgpu_compute_points` (a 2x2 block of lit pixels) cannot see.
//! - **`drawRange` clamps the vertex count and `object.count` is the *instance*
//!   count.** The compute-points page leans on both at once: its geometry holds
//!   a single vertex with `drawRange.count = 1`, and `mesh.count = 300000` is
//!   picked up by `RenderObject.getInstanceCount()`
//!   (`src/renderers/common/RenderObject.js:623-627`), giving
//!   `draw( 1, 300000, 0, 0 )`. Get either of the two wrong and the frame is
//!   one point, or 300 000 points stacked on one pixel.

use std::rc::Rc;

use three_rs::core::BufferGeometry;
use three_rs::nodes::tsl::{float, instance_index, position_local, vec3_join};
use three_rs::nodes::Type;
use three_rs::{
    Color, OrthographicCamera, Points, PointsNodeMaterial, Renderer, RendererParameters, Scene,
    Vector3,
};

const W: usize = 64;
const H: usize = 64;

/// The grid: 8 x 8 points on pixel centres, 7 pixels apart, inset by 4.
const SIDE: usize = 8;
const STEP: f64 = 7.0;
const INSET: f64 = 4.5;

fn grid(z: f64) -> Vec<Vector3> {
    let mut points = Vec::with_capacity(SIDE * SIDE);
    for j in 0..SIDE {
        for i in 0..SIDE {
            points.push(Vector3::new(
                INSET + STEP * i as f64,
                INSET + STEP * j as f64,
                z,
            ));
        }
    }
    points
}

/// `new OrthographicCamera( 0, W, H, 0, 0, 1 )`, `camera.position.z = 1`: the
/// compute-points example's camera, scaled to pixels.
fn pixel_camera() -> OrthographicCamera {
    let mut camera = OrthographicCamera::new(0.0, W as f64, H as f64, 0.0, 0.0, 1.0);
    camera.object.position.z = 1.0;
    camera.update_matrix_world();
    camera
}

fn from_points(points: &[Vector3]) -> Rc<BufferGeometry> {
    let mut geometry = BufferGeometry::new();
    geometry.set_from_points(points);
    Rc::new(geometry)
}

fn render(build: impl FnOnce(&Scene)) -> Vec<u8> {
    let mut scene = Scene::new();
    scene.set_background(Color::from_hex(0x000000));
    build(&scene);

    let mut camera = pixel_camera();
    let mut renderer = Renderer::new(RendererParameters { antialias: false }).unwrap();
    renderer.set_pixel_ratio(1.0);
    renderer.set_size(W as f64, H as f64);
    renderer.render(&mut scene, &mut camera);

    let (w, h, pixels) = renderer.read_canvas_pixels().unwrap();
    assert_eq!((w as usize, h as usize), (W, H));
    pixels
}

fn rgb(pixels: &[u8], column: usize, row: usize) -> [u8; 3] {
    let at = (row * W + column) * 4;
    [pixels[at], pixels[at + 1], pixels[at + 2]]
}

/// The camera flips y, so world `y = i + 0.5` is the centre of row `H - 1 - i`.
fn pixel_of(point: &Vector3) -> (usize, usize) {
    (
        (point.x - 0.5).round() as usize,
        H - 1 - (point.y - 0.5).round() as usize,
    )
}

/// Every point lit, every other pixel dark — the whole frame, not a sample.
#[track_caller]
fn assert_exactly(pixels: &[u8], points: &[Vector3], what: &str) {
    let lit: Vec<(usize, usize)> = points.iter().map(pixel_of).collect();
    for row in 0..H {
        for column in 0..W {
            let expected = lit.contains(&(column, row));
            let got = rgb(pixels, column, row);
            if expected {
                assert_eq!(
                    got,
                    [255, 255, 255],
                    "{what}: ({column}, {row}) holds a point and should be lit"
                );
            } else {
                assert_eq!(
                    got,
                    [0, 0, 0],
                    "{what}: ({column}, {row}) holds no point and should be background"
                );
            }
        }
    }
}

/// A grid of points at mid-depth: position and one-pixel coverage, with the
/// depth edge case kept out of it.
#[test]
fn every_point_lands_on_its_own_pixel() {
    let points = grid(0.5);
    let pixels = render(|scene| {
        scene.add(&Points::new(
            from_points(&points),
            PointsNodeMaterial::points(),
        ));
    });
    assert_exactly(&pixels, &points, "grid at z = 0.5");
}

/// The same grid pushed onto the far plane exactly. `depthCompare: less-equal`
/// draws it; `less` draws nothing.
#[test]
fn points_on_the_far_plane_still_draw() {
    let points = grid(0.0);
    let pixels = render(|scene| {
        scene.add(&Points::new(
            from_points(&points),
            PointsNodeMaterial::points(),
        ));
    });
    assert_exactly(&pixels, &points, "grid at z = 0 (far plane)");
}

/// `geometry.setDrawRange( 0, n )` draws the first `n` vertices and no more.
/// The page sets it to 1.
#[test]
fn draw_range_clamps_the_vertex_count() {
    let points = grid(0.5);
    let drawn = 10;
    let pixels = render(|scene| {
        let mut geometry = BufferGeometry::new();
        geometry.set_from_points(&points);
        geometry.set_draw_range(0, drawn);
        scene.add(&Points::new(
            Rc::new(geometry),
            PointsNodeMaterial::points(),
        ));
    });
    assert_exactly(&pixels, &points[..drawn], "first ten of the grid");
}

/// `points.count = n` is the *instance* count, not a vertex clamp: one vertex
/// drawn `n` times, each instance moved by `instanceIndex`. This is the page's
/// own shape (one vertex, `drawRange.count = 1`, `mesh.count = 300000`), with
/// the storage buffer the positions will come from in step 2 stood in for by
/// arithmetic on `instanceIndex`.
#[test]
fn count_is_the_instance_count() {
    let instances = SIDE;
    let origin = Vector3::new(INSET, INSET, 0.5);

    let mut geometry = BufferGeometry::new();
    geometry.set_from_points(&[origin]);
    geometry.set_draw_range(0, 1);

    let mut material = PointsNodeMaterial::points();
    material.position_node = Some(position_local().add(vec3_join(vec![
        instance_index().to(Type::F32).mul(STEP),
        float(0.0),
        float(0.0),
    ])));

    let pixels = render(|scene| {
        let node = Points::new(Rc::new(geometry), material);
        node.borrow_mut().payload.points_mut().unwrap().count = Some(instances);
        scene.add(&node);
    });

    let expected: Vec<Vector3> = (0..instances)
        .map(|i| Vector3::new(origin.x + STEP * i as f64, origin.y, origin.z))
        .collect();
    assert_exactly(&pixels, &expected, "one vertex, eight instances");
}
