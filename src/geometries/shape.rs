//! Port of `three.js/src/geometries/ShapeGeometry.js`.

use crate::core::{BufferAttribute, BufferGeometry};
use crate::extras::shape_utils;
use crate::extras::Shape;
use crate::math::Vector2;

/// The shape `new ShapeGeometry()` falls back to: a triangle.
pub fn shape_geometry_default_shape() -> Shape {
    Shape::from_points(&[
        Vector2::new(0.0, 0.5),
        Vector2::new(-0.5, -0.5),
        Vector2::new(0.5, -0.5),
    ])
}

/// `new ShapeGeometry( shape, curveSegments )` with a single shape: no
/// groups. three.js tells the two forms apart with `Array.isArray`; here
/// [`shape_geometry_multi`] is the array form.
pub fn shape_geometry(shape: &Shape, curve_segments: usize) -> BufferGeometry {
    build(std::slice::from_ref(shape), curve_segments, false)
}

/// `new ShapeGeometry( [ shape, ... ], curveSegments )`: one group per shape,
/// with material index `i`.
pub fn shape_geometry_multi(shapes: &[Shape], curve_segments: usize) -> BufferGeometry {
    build(shapes, curve_segments, true)
}

fn build(shapes: &[Shape], curve_segments: usize, groups: bool) -> BufferGeometry {
    let mut geometry = BufferGeometry::new();

    // buffers
    let mut indices: Vec<u32> = Vec::new();
    let mut vertices: Vec<f32> = Vec::new();
    let mut normals: Vec<f32> = Vec::new();
    let mut uvs: Vec<f32> = Vec::new();

    // helper variables
    let mut group_start = 0;
    let mut group_count = 0;

    for (i, shape) in shapes.iter().enumerate() {
        let index_offset = vertices.len() / 3;
        let points = shape.extract_points(curve_segments);
        let mut shape_vertices = points.shape;
        let mut shape_holes = points.holes;

        // check direction of vertices
        if !shape_utils::is_clock_wise(&shape_vertices) {
            shape_vertices.reverse();
        }

        for shape_hole in shape_holes.iter_mut() {
            if shape_utils::is_clock_wise(shape_hole) {
                shape_hole.reverse();
            }
        }

        // `triangulateShape` drops a repeated end point from its arguments in
        // place, and the vertices below are read from the shortened arrays.
        let faces = shape_utils::triangulate_shape(&mut shape_vertices, &mut shape_holes);

        // join vertices of inner and outer paths to a single array
        for shape_hole in &shape_holes {
            shape_vertices.extend_from_slice(shape_hole);
        }

        // vertices, normals, uvs
        for vertex in &shape_vertices {
            vertices.extend([vertex.x as f32, vertex.y as f32, 0.0]);
            normals.extend([0.0, 0.0, 1.0]);
            uvs.extend([vertex.x as f32, vertex.y as f32]); // world uvs
        }

        // indices
        for face in &faces {
            let a = face[0] + index_offset;
            let b = face[1] + index_offset;
            let c = face[2] + index_offset;
            indices.extend([a as u32, b as u32, c as u32]);
            group_count += 3;
        }

        if groups {
            geometry.add_group(group_start, group_count, i); // enables MultiMaterial support
            group_start += group_count;
            group_count = 0;
        }
    }

    // build geometry
    geometry.set_index(&indices);
    geometry.set_attribute("position", BufferAttribute::new(vertices, 3));
    geometry.set_attribute("normal", BufferAttribute::new(normals, 3));
    geometry.set_attribute("uv", BufferAttribute::new(uvs, 2));

    geometry
}
