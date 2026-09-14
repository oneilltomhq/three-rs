//! The >1024-instance path, drawn.
//!
//! `createInstanceMatrixNode()` keeps the instance matrices in a uniform buffer
//! only while `count * 16 * 4 <= maxUniformBufferBindingSize` (65536 with wgpu's
//! default limits, the same as Chrome's). Past that the matrices become an
//! interleaved instanced vertex buffer read as four `vec4` attributes, and
//! `range()` past its own limit becomes one instanced `vec4` attribute. Neither
//! branch is exercised by the rungs — `webgpu_instance_mesh` draws 1000 — so
//! this test draws a grid of quads on each side of the boundary and counts
//! coverage.
//!
//! One `#[test]` on purpose: each render builds its own device, and cargo runs
//! test functions inside a binary concurrently.

use std::rc::Rc;

use three_rs::geometries::plane_geometry;
use three_rs::materials::instanced_range;
use three_rs::math::Matrix4;
use three_rs::{
    Color, InstancedMesh, MeshBasicNodeMaterial, PerspectiveCamera, Renderer, RendererParameters,
    Scene, Vector3,
};

const WIDTH: f64 = 800.0;
const HEIGHT: f64 = 500.0;

/// Grid cells across and down, at the largest count this test draws, so that
/// every count shares one layout and a smaller count is a prefix of a larger
/// one's quads.
const COLUMNS: usize = 100;
const ROWS: usize = 60;

/// Half-extent of the laid-out grid in world units, inside the camera's
/// frustum at z = 0 (half-height `10 * tan( 30° )` = 5.77, half-width × 1.6).
const SPAN_X: f64 = 8.0;
const SPAN_Y: f64 = 5.0;

/// A grid of unlit quads, `count` of them, coloured either flat or by `range()`.
fn render(count: usize, range_colors: bool) -> Vec<u8> {
    let mut camera = PerspectiveCamera::new(60.0, WIDTH / HEIGHT, 0.1, 100.0);
    camera.node.borrow_mut().position.set(0.0, 0.0, 10.0);
    camera.look_at(&Vector3::ZERO);

    let mut scene = Scene::new();
    scene.set_background(Color::from_hex(0x000000));

    let mut material = MeshBasicNodeMaterial::new();
    if range_colors {
        // Read in the fragment stage, so over the limit this goes through the
        // generated varying rather than an instance-index lookup.
        material.color_node = Some(
            instanced_range(Color::new(0.2, 0.2, 0.2), Color::new(1.0, 1.0, 1.0), count).xyz(),
        );
    } else {
        material.color = Color::new(1.0, 1.0, 1.0);
    }

    let geometry = plane_geometry(0.12, 0.12, 1, 1);
    let mesh = InstancedMesh::new(Rc::new(geometry), material, count);

    let mut matrix = Matrix4::default();
    for i in 0..count {
        let (x, y) = cell_center(i);
        matrix.make_translation(x, y, 0.0);
        mesh.borrow_mut().set_matrix_at(i, &matrix);
    }
    scene.add(&mesh);

    let mut renderer = Renderer::new(RendererParameters { antialias: false }).unwrap();
    renderer.set_pixel_ratio(1.0);
    renderer.set_size(WIDTH, HEIGHT);
    renderer.render(&mut scene, &mut camera);

    let (width, height, pixels) = renderer.read_canvas_pixels().unwrap();
    assert_eq!((width, height), (WIDTH as u32, HEIGHT as u32));
    pixels
}

/// Instance `i`'s world-space position: row-major over the fixed grid.
fn cell_center(i: usize) -> (f64, f64) {
    let column = i % COLUMNS;
    let row = i / COLUMNS;
    assert!(row < ROWS, "instance {i} falls outside the grid");
    let x = -SPAN_X + 2.0 * SPAN_X * (column as f64 + 0.5) / COLUMNS as f64;
    let y = SPAN_Y - 2.0 * SPAN_Y * (row as f64 + 0.5) / ROWS as f64;
    (x, y)
}

/// Pixels brighter than the black background.
fn covered(pixels: &[u8]) -> usize {
    pixels.chunks_exact(4).filter(|px| px[0] > 16).count()
}

/// Distinct quantised colours among the covered pixels.
fn distinct_colors(pixels: &[u8]) -> usize {
    let mut seen = std::collections::HashSet::new();
    for px in pixels.chunks_exact(4) {
        if px[0] > 16 || px[1] > 16 || px[2] > 16 {
            seen.insert((px[0] / 8, px[1] / 8, px[2] / 8));
        }
    }
    seen.len()
}

#[test]
fn instances_past_the_uniform_buffer_limit_still_draw() {
    // One instance, for the per-instance coverage this grid gives.
    let one = render(1, false);
    let one_covered = covered(&one);
    assert!(
        one_covered > 0,
        "the single-instance render drew nothing, so the comparison below proves nothing"
    );

    // 1000 instances: 1000 * 64 = 64000 <= 65536, still the uniform-buffer
    // branch, which is where `webgpu_instance_mesh` sits.
    let uniform = render(1000, false);
    let uniform_covered = covered(&uniform);

    // 2000 instances: 2000 * 64 = 128000 > 65536, so the matrices arrive as four
    // interleaved instanced `vec4` attributes. Every one of them must draw: the
    // failure mode on this stack is silent wrong output, and a dropped instance
    // buffer would show up as a render identical to the 1-instance one.
    let attributes = render(2000, false);
    let attributes_covered = covered(&attributes);

    println!(
        "covered: 1 instance {one_covered}, 1000 {uniform_covered}, 2000 {attributes_covered}"
    );

    // Coverage scales with the instance count on both branches (the quads never
    // overlap), so the attribute path draws about twice the uniform path and
    // about 2000 times one instance. Generous bounds: this is checking that
    // thousands of quads landed, not their exact antialiasing.
    assert!(
        attributes_covered > uniform_covered * 3 / 2,
        "2000 instances covered {attributes_covered} pixels, 1000 covered \
         {uniform_covered}: the instances past the uniform-buffer limit did not draw"
    );
    assert!(
        attributes_covered > one_covered * 1000,
        "2000 instances covered {attributes_covered} pixels, one covers \
         {one_covered}: most instances are missing"
    );

    // And `range()` past its own limit: 5000 * 16 = 80000 > 65536, one instanced
    // `vec4` attribute read in the fragment stage through a generated varying.
    // Every instance gets its own random colour, so a collapsed or unbound
    // buffer shows up as one flat colour.
    let ranged = render(5000, true);
    let ranged_covered = covered(&ranged);
    let colors = distinct_colors(&ranged);
    println!("range() on the attribute path: covered {ranged_covered}, {colors} colours");
    assert!(
        ranged_covered > attributes_covered,
        "5000 range()-coloured instances covered {ranged_covered} pixels, 2000 flat ones \
         covered {attributes_covered}"
    );
    assert!(
        colors > 100,
        "only {colors} distinct colours across 5000 range() instances: the per-instance \
         attribute is not reaching the fragment stage"
    );
}
