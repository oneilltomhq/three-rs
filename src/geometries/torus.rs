//! Port of `three.js/src/geometries/TorusGeometry.js`.

use crate::core::{BufferAttribute, BufferGeometry};
use crate::math::Vector3;

/// `new TorusGeometry( radius, tube, radialSegments, tubularSegments )` with the
/// default `arc`/`thetaStart`/`thetaLength`.
pub fn torus_geometry(
    radius: f64,
    tube: f64,
    radial_segments: usize,
    tubular_segments: usize,
) -> BufferGeometry {
    torus_geometry_full(
        radius,
        tube,
        radial_segments,
        tubular_segments,
        std::f64::consts::PI * 2.0,
        0.0,
        std::f64::consts::PI * 2.0,
    )
}

pub fn torus_geometry_full(
    radius: f64,
    tube: f64,
    radial_segments: usize,
    tubular_segments: usize,
    arc: f64,
    theta_start: f64,
    theta_length: f64,
) -> BufferGeometry {
    // buffers

    let mut indices: Vec<u32> = Vec::new();
    let mut vertices: Vec<f32> = Vec::new();
    let mut normals: Vec<f32> = Vec::new();
    let mut uvs: Vec<f32> = Vec::new();

    // helper variables

    let mut center = Vector3::ZERO;
    let mut vertex = Vector3::ZERO;
    let mut normal = Vector3::ZERO;

    // generate vertices, normals and uvs

    for j in 0..=radial_segments {
        let v = theta_start + (j as f64 / radial_segments as f64) * theta_length;

        for i in 0..=tubular_segments {
            let u = i as f64 / tubular_segments as f64 * arc;

            // vertex

            vertex.x = (radius + tube * v.cos()) * u.cos();
            vertex.y = (radius + tube * v.cos()) * u.sin();
            vertex.z = tube * v.sin();

            vertices.push(vertex.x as f32);
            vertices.push(vertex.y as f32);
            vertices.push(vertex.z as f32);

            // normal

            center.x = radius * u.cos();
            center.y = radius * u.sin();
            normal.sub_vectors(&vertex, &center).normalize();

            normals.push(normal.x as f32);
            normals.push(normal.y as f32);
            normals.push(normal.z as f32);

            // uv

            uvs.push((i as f64 / tubular_segments as f64) as f32);
            uvs.push((j as f64 / radial_segments as f64) as f32);
        }
    }

    // generate indices

    for j in 1..=radial_segments {
        for i in 1..=tubular_segments {
            let a = ((tubular_segments + 1) * j + i - 1) as u32;
            let b = ((tubular_segments + 1) * (j - 1) + i - 1) as u32;
            let c = ((tubular_segments + 1) * (j - 1) + i) as u32;
            let d = ((tubular_segments + 1) * j + i) as u32;

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
