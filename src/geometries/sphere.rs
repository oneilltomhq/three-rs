//! Port of `three.js/src/geometries/SphereGeometry.js`.

use crate::core::{BufferAttribute, BufferGeometry};
use crate::math::Vector3;

/// `new SphereGeometry( radius, widthSegments, heightSegments )` with the
/// default `phiStart`/`phiLength`/`thetaStart`/`thetaLength`.
pub fn sphere_geometry(radius: f64, width_segments: usize, height_segments: usize) -> BufferGeometry {
    sphere_geometry_full(
        radius,
        width_segments,
        height_segments,
        0.0,
        std::f64::consts::PI * 2.0,
        0.0,
        std::f64::consts::PI,
    )
}

pub fn sphere_geometry_full(
    radius: f64,
    width_segments: usize,
    height_segments: usize,
    phi_start: f64,
    phi_length: f64,
    theta_start: f64,
    theta_length: f64,
) -> BufferGeometry {
    let width_segments = width_segments.max(3);
    let height_segments = height_segments.max(2);

    let theta_end = (theta_start + theta_length).min(std::f64::consts::PI);

    let mut index = 0u32;
    let mut grid: Vec<Vec<u32>> = Vec::new();

    let mut vertex = Vector3::ZERO;
    let mut normal;

    let mut indices: Vec<u32> = Vec::new();
    let mut vertices: Vec<f32> = Vec::new();
    let mut normals: Vec<f32> = Vec::new();
    let mut uvs: Vec<f32> = Vec::new();

    // generate vertices, normals and uvs

    for iy in 0..=height_segments {
        let mut vertices_row = Vec::new();

        let v = iy as f64 / height_segments as f64;
        let theta = theta_start + v * theta_length;

        let y = radius * theta.cos();
        let ring_radius = (radius * radius - y * y).sqrt();

        // special case for the poles

        let mut u_offset = 0.0;

        if iy == 0 && theta_start == 0.0 {
            u_offset = 0.5 / width_segments as f64;
        } else if iy == height_segments && theta_end == std::f64::consts::PI {
            u_offset = -0.5 / width_segments as f64;
        }

        for ix in 0..=width_segments {
            let u = ix as f64 / width_segments as f64;
            let phi = phi_start + u * phi_length;

            // vertex

            vertex.x = -ring_radius * phi.cos();
            vertex.y = y;
            vertex.z = ring_radius * phi.sin();

            vertices.push(vertex.x as f32);
            vertices.push(vertex.y as f32);
            vertices.push(vertex.z as f32);

            // normal

            normal = vertex;
            normal.normalize();
            normals.push(normal.x as f32);
            normals.push(normal.y as f32);
            normals.push(normal.z as f32);

            // uv

            uvs.push((u + u_offset) as f32);
            uvs.push((1.0 - v) as f32);

            vertices_row.push(index);
            index += 1;
        }

        grid.push(vertices_row);
    }

    // indices

    for iy in 0..height_segments {
        for ix in 0..width_segments {
            let a = grid[iy][ix + 1];
            let b = grid[iy][ix];
            let c = grid[iy + 1][ix];
            let d = grid[iy + 1][ix + 1];

            if iy != 0 || theta_start > 0.0 {
                indices.extend_from_slice(&[a, b, d]);
            }
            if iy != height_segments - 1 || theta_end < std::f64::consts::PI {
                indices.extend_from_slice(&[b, c, d]);
            }
        }
    }

    let mut geometry = BufferGeometry::new();
    geometry.set_index(&indices);
    geometry.set_attribute("position", BufferAttribute::new(vertices, 3));
    geometry.set_attribute("normal", BufferAttribute::new(normals, 3));
    geometry.set_attribute("uv", BufferAttribute::new(uvs, 2));
    geometry
}
