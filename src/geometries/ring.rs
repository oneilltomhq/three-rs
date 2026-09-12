//! Port of `three.js/src/geometries/RingGeometry.js`.

use crate::core::{BufferAttribute, BufferGeometry};
use crate::math::Vector3;

/// `new RingGeometry( innerRadius, outerRadius, thetaSegments, phiSegments )`
/// with the default `thetaStart`/`thetaLength`.
pub fn ring_geometry(
    inner_radius: f64,
    outer_radius: f64,
    theta_segments: usize,
    phi_segments: usize,
) -> BufferGeometry {
    ring_geometry_full(
        inner_radius,
        outer_radius,
        theta_segments,
        phi_segments,
        0.0,
        std::f64::consts::PI * 2.0,
    )
}

pub fn ring_geometry_full(
    inner_radius: f64,
    outer_radius: f64,
    theta_segments: usize,
    phi_segments: usize,
    theta_start: f64,
    theta_length: f64,
) -> BufferGeometry {
    let theta_segments = theta_segments.max(3);
    let phi_segments = phi_segments.max(1);

    // buffers

    let mut indices: Vec<u32> = Vec::new();
    let mut vertices: Vec<f32> = Vec::new();
    let mut normals: Vec<f32> = Vec::new();
    let mut uvs: Vec<f32> = Vec::new();

    // some helper variables

    let mut radius = inner_radius;
    let radius_step = (outer_radius - inner_radius) / phi_segments as f64;
    let mut vertex = Vector3::ZERO;

    // generate vertices, normals and uvs

    for _j in 0..=phi_segments {
        for i in 0..=theta_segments {
            // values are generate from the inside of the ring to the outside

            let segment = theta_start + i as f64 / theta_segments as f64 * theta_length;

            // vertex

            vertex.x = radius * segment.cos();
            vertex.y = radius * segment.sin();

            vertices.push(vertex.x as f32);
            vertices.push(vertex.y as f32);
            vertices.push(vertex.z as f32);

            // normal

            normals.push(0.0);
            normals.push(0.0);
            normals.push(1.0);

            // uv

            uvs.push(((vertex.x / outer_radius + 1.0) / 2.0) as f32);
            uvs.push(((vertex.y / outer_radius + 1.0) / 2.0) as f32);
        }

        // increase the radius for next row of vertices

        radius += radius_step;
    }

    // indices

    for j in 0..phi_segments {
        let theta_segment_level = j * (theta_segments + 1);

        for i in 0..theta_segments {
            let segment = i + theta_segment_level;

            let a = segment as u32;
            let b = (segment + theta_segments + 1) as u32;
            let c = (segment + theta_segments + 2) as u32;
            let d = (segment + 1) as u32;

            // faces

            indices.push(a);
            indices.push(b);
            indices.push(d);
            indices.push(b);
            indices.push(c);
            indices.push(d);
        }
    }

    // build geometry

    let mut geometry = BufferGeometry::new();
    geometry.set_index(&indices);
    geometry.position = Some(BufferAttribute::new(vertices, 3));
    geometry.normal = Some(BufferAttribute::new(normals, 3));
    geometry.uv = Some(BufferAttribute::new(uvs, 2));
    geometry
}
