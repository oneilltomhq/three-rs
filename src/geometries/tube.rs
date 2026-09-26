//! Port of `three.js/src/geometries/TubeGeometry.js`.

use crate::core::{BufferAttribute, BufferGeometry};
use crate::extras::{Curve, FrenetFrames, QuadraticBezierCurve3};
use crate::math::Vector3;

/// The path `new TubeGeometry()` falls back to.
pub fn tube_geometry_default_path() -> QuadraticBezierCurve3 {
    QuadraticBezierCurve3::new(
        Vector3::new(-1.0, -1.0, 0.0),
        Vector3::new(-1.0, 1.0, 0.0),
        Vector3::new(1.0, 1.0, 0.0),
    )
}

/// `new TubeGeometry( path, tubularSegments, radius, radialSegments, closed )`.
///
/// Returns the geometry and the Frenet frames three.js keeps on it as
/// `tangents` / `normals` / `binormals`. three.js accepts a 2D path too and
/// then reads `P.z` as `undefined`, which fills the buffers with NaN; the
/// port takes 3D paths only.
pub fn tube_geometry<C: Curve<Point = Vector3> + ?Sized>(
    path: &C,
    tubular_segments: usize,
    radius: f64,
    radial_segments: usize,
    closed: bool,
) -> (BufferGeometry, FrenetFrames) {
    let frames = path.compute_frenet_frames(tubular_segments, closed);

    let mut vertices: Vec<f32> = Vec::new();
    let mut normals: Vec<f32> = Vec::new();
    let mut uvs: Vec<f32> = Vec::new();
    let mut indices: Vec<u32> = Vec::new();

    let mut generate_segment = |i: usize| {
        let p = path.get_point_at(i as f64 / tubular_segments as f64);

        let n = frames.normals[i];
        let b = frames.binormals[i];

        for j in 0..=radial_segments {
            let v = j as f64 / radial_segments as f64 * std::f64::consts::PI * 2.0;

            let sin = v.sin();
            let cos = -v.cos();

            let mut normal = Vector3::new(
                cos * n.x + sin * b.x,
                cos * n.y + sin * b.y,
                cos * n.z + sin * b.z,
            );
            normal.normalize();

            normals.extend([normal.x as f32, normal.y as f32, normal.z as f32]);

            let vertex = Vector3::new(
                p.x + radius * normal.x,
                p.y + radius * normal.y,
                p.z + radius * normal.z,
            );
            vertices.extend([vertex.x as f32, vertex.y as f32, vertex.z as f32]);
        }
    };

    for i in 0..tubular_segments {
        generate_segment(i);
    }

    // if the geometry is not closed, generate the last row of vertices and normals
    // at the regular position on the given path
    //
    // if the geometry is closed, duplicate the first row of vertices and normals (uvs will differ)
    generate_segment(if closed { 0 } else { tubular_segments });

    // uvs
    for i in 0..=tubular_segments {
        for j in 0..=radial_segments {
            uvs.push((i as f64 / tubular_segments as f64) as f32);
            uvs.push((j as f64 / radial_segments as f64) as f32);
        }
    }

    // indices
    for j in 1..=tubular_segments {
        for i in 1..=radial_segments {
            let a = ((radial_segments + 1) * (j - 1) + (i - 1)) as u32;
            let b = ((radial_segments + 1) * j + (i - 1)) as u32;
            let c = ((radial_segments + 1) * j + i) as u32;
            let d = ((radial_segments + 1) * (j - 1) + i) as u32;

            indices.extend([a, b, d]);
            indices.extend([b, c, d]);
        }
    }

    let mut geometry = BufferGeometry::new();
    geometry.set_index(&indices);
    geometry.set_attribute("position", BufferAttribute::new(vertices, 3));
    geometry.set_attribute("normal", BufferAttribute::new(normals, 3));
    geometry.set_attribute("uv", BufferAttribute::new(uvs, 2));

    (geometry, frames)
}
