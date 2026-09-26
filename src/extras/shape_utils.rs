//! Port of `three.js/src/extras/ShapeUtils.js`.

use super::earcut;
use crate::math::Vector2;

/// `ShapeUtils.area( contour )`: the signed area of a 2D polygon, positive
/// when counter-clockwise.
pub fn area(contour: &[Vector2]) -> f64 {
    let n = contour.len();
    let mut a = 0.0;

    if n == 0 {
        return a * 0.5;
    }
    let mut p = n - 1;
    for q in 0..n {
        a += contour[p].x * contour[q].y - contour[q].x * contour[p].y;
        p = q;
    }

    a * 0.5
}

/// `ShapeUtils.isClockWise( pts )`.
pub fn is_clock_wise(pts: &[Vector2]) -> bool {
    area(pts) < 0.0
}

/// `ShapeUtils.triangulateShape( contour, holes )`: the faces of a contour
/// with holes, as index triples into the contour's points followed by each
/// hole's.
///
/// Like three.js this **mutates its arguments**: a contour or hole whose last
/// point repeats its first loses that last point. `ShapeGeometry` and
/// `ExtrudeGeometry` then index into the shortened arrays, so the inputs are
/// `&mut` rather than copied.
pub fn triangulate_shape(
    contour: &mut Vec<Vector2>,
    holes: &mut [Vec<Vector2>],
) -> Vec<[usize; 3]> {
    let mut vertices = Vec::new(); // flat array of vertices like [ x0,y0, x1,y1, x2,y2, ... ]
    let mut hole_indices = Vec::new(); // array of hole indices

    remove_dup_end_pts(contour);
    add_contour(&mut vertices, contour);

    let mut hole_index = contour.len();

    for hole in holes.iter_mut() {
        remove_dup_end_pts(hole);
    }

    for hole in holes.iter() {
        hole_indices.push(hole_index);
        hole_index += hole.len();
        add_contour(&mut vertices, hole);
    }

    let triangles = earcut::triangulate(&vertices, &hole_indices, 2);

    triangles
        .chunks(3)
        .map(|t| {
            // `triangles.slice( i, i + 3 )`; earcut only ever pushes triples.
            [t[0], t[1], t[2]]
        })
        .collect()
}

fn remove_dup_end_pts(points: &mut Vec<Vector2>) {
    let l = points.len();

    if l > 2 && points[l - 1].equals(&points[0]) {
        points.pop();
    }
}

fn add_contour(vertices: &mut Vec<f64>, contour: &[Vector2]) {
    for p in contour {
        vertices.push(p.x);
        vertices.push(p.y);
    }
}
