//! The `lines` branch's own gate: `Line` and `LineSegments` rendered headless
//! through the real renderer, read back, and checked pixel by pixel against
//! positions worked out from the camera alone.
//!
//! Nothing here is derived from an image. An `OrthographicCamera` set to
//! `( 0, W, H, 0, -1, 1 )` with the camera at the origin makes one world unit
//! one pixel, so a vertex at world `( x, y )` lands at framebuffer column `x`
//! and row `H - y` — and a vertex placed at `i + 0.5` sits exactly on the
//! centre of column `i`. Every line here is axis-aligned and pinned to pixel
//! centres, which is the one case where a hairline rasteriser has no freedom:
//! the covered pixels are the ones whose centres the segment passes through.
//!
//! What each assertion catches, all of them silent-wrong-output failures:
//!
//! - **Topology reaches the pipeline.** With the `triangle-list` the renderer
//!   used to hardcode, a 5-vertex `Line` would draw one filled triangle and the
//!   interior test would fail; the strip's edges would not be drawn at all.
//! - **`Line` is a strip, not a loop.** An open four-point strip must draw
//!   three segments. `LineLoop`'s closing segment — which three.js refuses to
//!   render at all — would light the fourth edge.
//! - **A five-point closed strip is four segments**, so the shape closes
//!   because the geometry says so and not because the renderer adds anything.
//! - **`LineSegments` is a list, not a strip.** Four vertices are two
//!   independent segments; a strip would join vertex 1 to vertex 2 across the
//!   middle of the frame.
//!
//! One rasterisation rule is deliberately not asserted: a segment's *final*
//! pixel is not produced (the diamond-exit rule — Vulkan §Basic Line Segment
//! Rasterization, and the same rule in D3D and GL), so at a shared vertex the
//! covered set depends on which segment claims it. Every edge assertion below
//! therefore stops one pixel short of each corner, and no assertion is made
//! about the corner pixels themselves. Nothing about the *shape* is weakened by
//! that: a missing segment still leaves its whole span dark.

use std::rc::Rc;

use three_rs::core::BufferGeometry;
use three_rs::{
    Color, Line, LineSegments, MeshBasicNodeMaterial, OrthographicCamera, Renderer,
    RendererParameters, Scene, Vector3,
};

const W: usize = 64;
const H: usize = 64;

/// The rectangle's corners, in pixel centres.
const X0: f64 = 10.5;
const Y0: f64 = 10.5;
const X1: f64 = 40.5;
const Y1: f64 = 30.5;

/// `new OrthographicCamera( 0, W, H, 0, -1, 1 )` at the origin: one world unit
/// per pixel, +y up, looking down -z.
fn pixel_camera() -> OrthographicCamera {
    OrthographicCamera::new(0.0, W as f64, H as f64, 0.0, -1.0, 1.0)
}

/// `new BufferGeometry().setFromPoints( points )`.
fn from_points(points: &[Vector3]) -> Rc<BufferGeometry> {
    let mut geometry = BufferGeometry::new();
    geometry.set_from_points(points);
    Rc::new(geometry)
}

/// `new LineBasicNodeMaterial( { color: 0xffffff } )`.
fn white() -> MeshBasicNodeMaterial {
    MeshBasicNodeMaterial::line(Color::from_hex(0xffffff))
}

/// Render one scene of lines on a black background and read the canvas back.
fn render(build: impl FnOnce(&Scene)) -> Vec<u8> {
    let mut scene = Scene::new();
    scene.set_background(Color::from_hex(0x000000));
    build(&scene);

    let mut camera = pixel_camera();
    let mut renderer = Renderer::new(RendererParameters { antialias: false });
    renderer.set_pixel_ratio(1.0);
    renderer.set_size(W as f64, H as f64);
    renderer.render(&mut scene, &mut camera);

    let (w, h, pixels) = renderer.read_canvas_pixels();
    assert_eq!((w as usize, h as usize), (W, H));
    pixels
}

/// The RGB of framebuffer pixel `( column, row )`, row 0 at the top.
fn rgb(pixels: &[u8], column: usize, row: usize) -> [u8; 3] {
    let at = (row * W + column) * 4;
    [pixels[at], pixels[at + 1], pixels[at + 2]]
}

/// The row a world `y` lands on: the camera flips y, so world `y = i + 0.5` is
/// the centre of row `H - 1 - i`.
fn row_of(y: f64) -> usize {
    H - 1 - (y - 0.5).round() as usize
}

fn column_of(x: f64) -> usize {
    (x - 0.5).round() as usize
}

#[track_caller]
fn assert_white(pixels: &[u8], column: usize, row: usize, what: &str) {
    assert_eq!(
        rgb(pixels, column, row),
        [255, 255, 255],
        "{what}: pixel ({column}, {row}) should be on the line"
    );
}

#[track_caller]
fn assert_background(pixels: &[u8], column: usize, row: usize, what: &str) {
    assert_eq!(
        rgb(pixels, column, row),
        [0, 0, 0],
        "{what}: pixel ({column}, {row}) should be background"
    );
}

/// The page's own shape: `new Line( geometry.setFromPoints( [ a, b, c, e, a ] ) )`.
#[test]
fn a_five_point_line_strip_draws_a_closed_rectangle() {
    let a = Vector3::new(X0, Y0, 0.0);
    let b = Vector3::new(X1, Y0, 0.0);
    let c = Vector3::new(X1, Y1, 0.0);
    let e = Vector3::new(X0, Y1, 0.0);

    let pixels = render(|scene| {
        scene.add(&Line::new(from_points(&[a, b, c, e, a]), white()));
    });

    let (left, right) = (column_of(X0), column_of(X1));
    let (bottom, top) = (row_of(Y0), row_of(Y1));

    // All four edges, sampled away from the corners so a missing segment cannot
    // hide behind its neighbour's endpoint.
    for column in (left + 1)..right {
        assert_white(&pixels, column, bottom, "bottom edge");
        assert_white(&pixels, column, top, "top edge");
    }
    for row in (top + 1)..bottom {
        assert_white(&pixels, left, row, "left edge");
        assert_white(&pixels, right, row, "right edge");
    }

    // Hairlines, not a filled quad: the inside is background. This is the
    // assertion a `triangle-list` pipeline fails.
    for row in (top + 1)..bottom {
        for column in (left + 1)..right {
            assert_background(&pixels, column, row, "interior");
        }
    }

    // And nothing outside the rectangle.
    assert_background(&pixels, left - 1, bottom, "outside, left of the bottom edge");
    assert_background(&pixels, right + 1, top, "outside, right of the top edge");
    assert_background(&pixels, left, bottom + 1, "outside, below the left edge");
    assert_background(&pixels, right, top - 1, "outside, above the right edge");
}

/// Four points are three segments — the strip does not close itself. Three.js
/// refuses to render a `LineLoop` at all (`Renderer._projectObject()` errors),
/// so a closing segment here could only come from this port inventing one.
#[test]
fn an_open_four_point_strip_draws_three_segments_and_does_not_close() {
    let a = Vector3::new(X0, Y0, 0.0);
    let b = Vector3::new(X1, Y0, 0.0);
    let c = Vector3::new(X1, Y1, 0.0);
    let e = Vector3::new(X0, Y1, 0.0);

    let pixels = render(|scene| {
        scene.add(&Line::new(from_points(&[a, b, c, e]), white()));
    });

    let (left, right) = (column_of(X0), column_of(X1));
    let (bottom, top) = (row_of(Y0), row_of(Y1));

    for column in (left + 1)..right {
        assert_white(&pixels, column, bottom, "bottom edge");
        assert_white(&pixels, column, top, "top edge");
    }
    for row in (top + 1)..bottom {
        assert_white(&pixels, right, row, "right edge");
        // The segment `e -> a` is the one the geometry does not have.
        assert_background(&pixels, left, row, "the closing edge");
    }
}

/// `LineSegments` is `line-list`: vertices in pairs, with no segment joining
/// one pair to the next.
#[test]
fn line_segments_draws_pairs_and_never_joins_them() {
    // Two horizontal bars, one above the other, ordered so that a strip would
    // join `p1` to `p2` with a long diagonal across the middle.
    let p0 = Vector3::new(X0, Y0, 0.0);
    let p1 = Vector3::new(X1, Y0, 0.0);
    let p2 = Vector3::new(X0, Y1, 0.0);
    let p3 = Vector3::new(X1, Y1, 0.0);

    let pixels = render(|scene| {
        scene.add(&LineSegments::new(from_points(&[p0, p1, p2, p3]), white()));
    });

    let (left, right) = (column_of(X0), column_of(X1));
    let (bottom, top) = (row_of(Y0), row_of(Y1));

    for column in (left + 1)..right {
        assert_white(&pixels, column, bottom, "first segment");
        assert_white(&pixels, column, top, "second segment");
    }

    // The join a strip would draw runs from `( X1, Y0 )` to `( X0, Y1 )`. Every
    // row strictly between the two bars is empty under a line list.
    for row in (top + 1)..bottom {
        for column in 0..W {
            assert_background(&pixels, column, row, "between the two segments");
        }
    }
}

/// The same four vertices as a `Line` *do* join, which is what makes the test
/// above a statement about the topology rather than about the geometry.
#[test]
fn the_same_four_vertices_as_a_line_strip_do_join() {
    let p0 = Vector3::new(X0, Y0, 0.0);
    let p1 = Vector3::new(X1, Y0, 0.0);
    let p2 = Vector3::new(X0, Y1, 0.0);
    let p3 = Vector3::new(X1, Y1, 0.0);

    let pixels = render(|scene| {
        scene.add(&Line::new(from_points(&[p0, p1, p2, p3]), white()));
    });

    let (top, bottom) = (row_of(Y1), row_of(Y0));

    // Some pixel of the diagonal `p1 -> p2` is lit on every row between the
    // bars. Which column it is depends on the rasteriser's tie-breaking, so the
    // assertion is "the row is not empty", not "this exact pixel".
    for row in (top + 1)..bottom {
        let lit = (0..W).any(|column| rgb(&pixels, column, row) != [0, 0, 0]);
        assert!(lit, "line-strip: row {row} of the joining diagonal is empty");
    }
}
