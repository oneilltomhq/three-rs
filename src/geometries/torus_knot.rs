//! Port of `three.js/src/geometries/TorusKnotGeometry.js`.

use crate::core::{BufferAttribute, BufferGeometry};
use crate::math::Vector3;

/// `new TorusKnotGeometry( radius, tube, tubularSegments, radialSegments, p, q )`.
pub fn torus_knot_geometry(
    radius: f64,
    tube: f64,
    tubular_segments: usize,
    radial_segments: usize,
    p: f64,
    q: f64,
) -> BufferGeometry {
    let mut indices: Vec<u32> = Vec::new();
    let mut vertices: Vec<f32> = Vec::new();
    let mut normals: Vec<f32> = Vec::new();
    let mut uvs: Vec<f32> = Vec::new();

    let mut vertex = Vector3::ZERO;
    let mut normal = Vector3::ZERO;

    let mut p1 = Vector3::ZERO;
    let mut p2 = Vector3::ZERO;

    let mut b = Vector3::ZERO;
    let mut t = Vector3::ZERO;
    let mut n = Vector3::ZERO;

    for i in 0..=tubular_segments {
        let u = i as f64 / tubular_segments as f64 * p * std::f64::consts::PI * 2.0;

        calculate_position_on_curve(u, p, q, radius, &mut p1);
        calculate_position_on_curve(u + 0.01, p, q, radius, &mut p2);

        t.sub_vectors(&p2, &p1);
        n.add_vectors(&p2, &p1);
        b.cross_vectors(&t, &n);
        n.cross_vectors(&b, &t);

        b.normalize();
        n.normalize();

        for j in 0..=radial_segments {
            let v = j as f64 / radial_segments as f64 * std::f64::consts::PI * 2.0;
            let cx = -tube * v.cos();
            let cy = tube * v.sin();

            vertex.x = p1.x + (cx * n.x + cy * b.x);
            vertex.y = p1.y + (cx * n.y + cy * b.y);
            vertex.z = p1.z + (cx * n.z + cy * b.z);

            vertices.push(vertex.x as f32);
            vertices.push(vertex.y as f32);
            vertices.push(vertex.z as f32);

            normal.sub_vectors(&vertex, &p1).normalize();

            normals.push(normal.x as f32);
            normals.push(normal.y as f32);
            normals.push(normal.z as f32);

            uvs.push((i as f64 / tubular_segments as f64) as f32);
            uvs.push((j as f64 / radial_segments as f64) as f32);
        }
    }

    for j in 1..=tubular_segments {
        for i in 1..=radial_segments {
            let a = ((radial_segments + 1) * (j - 1) + (i - 1)) as u32;
            let b_i = ((radial_segments + 1) * j + (i - 1)) as u32;
            let c = ((radial_segments + 1) * j + i) as u32;
            let d = ((radial_segments + 1) * (j - 1) + i) as u32;

            indices.push(a);
            indices.push(b_i);
            indices.push(d);
            indices.push(b_i);
            indices.push(c);
            indices.push(d);
        }
    }

    let mut geometry = BufferGeometry::new();
    geometry.set_index(&indices);
    geometry.position = Some(BufferAttribute::new(vertices, 3));
    geometry.normal = Some(BufferAttribute::new(normals, 3));
    geometry.uv = Some(BufferAttribute::new(uvs, 2));
    geometry
}

fn calculate_position_on_curve(u: f64, p: f64, q: f64, radius: f64, position: &mut Vector3) {
    let cu = u.cos();
    let su = u.sin();
    let qu_over_p = q / p * u;
    let cs = qu_over_p.cos();

    position.x = radius * (2.0 + cs) * 0.5 * cu;
    position.y = radius * (2.0 + cs) * su * 0.5;
    position.z = radius * qu_over_p.sin() * 0.5;
}
