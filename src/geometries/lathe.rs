//! Port of `three.js/src/geometries/LatheGeometry.js`.
//!
//! The profile is a list of `Vector2` in three.js; the crate has no `Vector2`,
//! so it is taken here as `(x, y)` pairs.

use crate::core::{BufferAttribute, BufferGeometry};
use crate::math::Vector3;

/// `LatheGeometry`'s default profile.
pub fn lathe_default_points() -> Vec<(f64, f64)> {
    vec![(0.0, -0.5), (0.5, 0.0), (0.0, 0.5)]
}

/// `new LatheGeometry( points, segments )` with the default
/// `phiStart`/`phiLength`.
pub fn lathe_geometry(points: &[(f64, f64)], segments: usize) -> BufferGeometry {
    lathe_geometry_full(points, segments, 0.0, std::f64::consts::PI * 2.0)
}

pub fn lathe_geometry_full(
    points: &[(f64, f64)],
    segments: usize,
    phi_start: f64,
    phi_length: f64,
) -> BufferGeometry {
    // clamp phiLength so it's in range of [ 0, 2PI ]

    let phi_length = phi_length.clamp(0.0, std::f64::consts::PI * 2.0);

    // buffers

    let mut indices: Vec<u32> = Vec::new();
    let mut vertices: Vec<f32> = Vec::new();
    let mut uvs: Vec<f32> = Vec::new();
    let mut init_normals: Vec<f64> = Vec::new();
    let mut normals: Vec<f32> = Vec::new();

    // helper variables

    let inverse_segments = 1.0 / segments as f64;
    let mut vertex = Vector3::ZERO;
    let mut normal = Vector3::ZERO;
    let mut cur_normal = Vector3::ZERO;
    let mut prev_normal = Vector3::ZERO;
    let mut dx;
    let mut dy;

    // three.js iterates `j <= points.length - 1`, which for an empty profile is
    // `j <= -1` and so runs zero times, leaving every buffer empty.
    if points.is_empty() {
        let mut geometry = BufferGeometry::new();
        geometry.set_index(&indices);
        geometry.set_attribute("position", BufferAttribute::new(vertices, 3));
        geometry.set_attribute("uv", BufferAttribute::new(uvs, 2));
        geometry.set_attribute("normal", BufferAttribute::new(normals, 3));
        return geometry;
    }

    let last = points.len() - 1;

    // pre-compute normals for initial "meridian"

    for j in 0..=last {
        if j == 0 {
            // special handling for 1st vertex on path

            dx = points[j + 1].0 - points[j].0;
            dy = points[j + 1].1 - points[j].1;

            normal.x = dy * 1.0;
            normal.y = -dx;
            normal.z = dy * 0.0;

            prev_normal = normal;

            normal.normalize();

            init_normals.push(normal.x);
            init_normals.push(normal.y);
            init_normals.push(normal.z);
        } else if j == last {
            // special handling for last Vertex on path

            init_normals.push(prev_normal.x);
            init_normals.push(prev_normal.y);
            init_normals.push(prev_normal.z);
        } else {
            // default handling for all vertices in between

            dx = points[j + 1].0 - points[j].0;
            dy = points[j + 1].1 - points[j].1;

            normal.x = dy * 1.0;
            normal.y = -dx;
            normal.z = dy * 0.0;

            cur_normal = normal;

            normal.x += prev_normal.x;
            normal.y += prev_normal.y;
            normal.z += prev_normal.z;

            normal.normalize();

            init_normals.push(normal.x);
            init_normals.push(normal.y);
            init_normals.push(normal.z);

            prev_normal = cur_normal;
        }
    }

    let _ = cur_normal;

    // generate vertices, uvs and normals

    for i in 0..=segments {
        let phi = phi_start + i as f64 * inverse_segments * phi_length;

        let sin = phi.sin();
        let cos = phi.cos();

        for j in 0..=last {
            // vertex

            vertex.x = points[j].0 * sin;
            vertex.y = points[j].1;
            vertex.z = points[j].0 * cos;

            vertices.push(vertex.x as f32);
            vertices.push(vertex.y as f32);
            vertices.push(vertex.z as f32);

            // uv

            uvs.push((i as f64 / segments as f64) as f32);
            uvs.push((j as f64 / last as f64) as f32);

            // normal

            let x = init_normals[3 * j] * sin;
            let y = init_normals[3 * j + 1];
            let z = init_normals[3 * j] * cos;

            normals.push(x as f32);
            normals.push(y as f32);
            normals.push(z as f32);
        }
    }

    // indices

    for i in 0..segments {
        for j in 0..last {
            let base = j + i * points.len();

            let a = base as u32;
            let b = (base + points.len()) as u32;
            let c = (base + points.len() + 1) as u32;
            let d = (base + 1) as u32;

            // faces

            indices.push(a);
            indices.push(b);
            indices.push(d);
            indices.push(c);
            indices.push(d);
            indices.push(b);
        }
    }

    // build geometry

    let mut geometry = BufferGeometry::new();
    geometry.set_index(&indices);
    geometry.set_attribute("position", BufferAttribute::new(vertices, 3));
    geometry.set_attribute("uv", BufferAttribute::new(uvs, 2));
    geometry.set_attribute("normal", BufferAttribute::new(normals, 3));
    geometry
}
