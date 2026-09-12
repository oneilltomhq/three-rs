//! Port of `three.js/src/geometries/CircleGeometry.js`.

use crate::core::{BufferAttribute, BufferGeometry};
use crate::math::Vector3;

/// `new CircleGeometry( radius, segments )` with the default
/// `thetaStart`/`thetaLength`.
pub fn circle_geometry(radius: f64, segments: usize) -> BufferGeometry {
    circle_geometry_full(radius, segments, 0.0, std::f64::consts::PI * 2.0)
}

pub fn circle_geometry_full(
    radius: f64,
    segments: usize,
    theta_start: f64,
    theta_length: f64,
) -> BufferGeometry {
    let segments = segments.max(3);

    // buffers

    let mut indices: Vec<u32> = Vec::new();
    let mut vertices: Vec<f64> = Vec::new();
    let mut normals: Vec<f32> = Vec::new();
    let mut uvs: Vec<f32> = Vec::new();

    // helper variables

    let mut vertex = Vector3::ZERO;

    // center point

    vertices.push(0.0);
    vertices.push(0.0);
    vertices.push(0.0);
    normals.push(0.0);
    normals.push(0.0);
    normals.push(1.0);
    uvs.push(0.5);
    uvs.push(0.5);

    let mut i = 3usize;
    for s in 0..=segments {
        let segment = theta_start + s as f64 / segments as f64 * theta_length;

        // vertex

        vertex.x = radius * segment.cos();
        vertex.y = radius * segment.sin();

        vertices.push(vertex.x);
        vertices.push(vertex.y);
        vertices.push(vertex.z);

        // normal

        normals.push(0.0);
        normals.push(0.0);
        normals.push(1.0);

        // uvs — three.js reads them back out of the (full precision) vertex array

        uvs.push(((vertices[i] / radius + 1.0) / 2.0) as f32);
        uvs.push(((vertices[i + 1] / radius + 1.0) / 2.0) as f32);

        i += 3;
    }

    // indices

    for i in 1..=segments {
        indices.push(i as u32);
        indices.push((i + 1) as u32);
        indices.push(0);
    }

    // build geometry

    let mut geometry = BufferGeometry::new();
    geometry.set_index(&indices);
    geometry.set_attribute(
        "position",
        BufferAttribute::new(vertices.iter().map(|&v| v as f32).collect(), 3),
    );
    geometry.set_attribute("normal", BufferAttribute::new(normals, 3));
    geometry.set_attribute("uv", BufferAttribute::new(uvs, 2));
    geometry
}
