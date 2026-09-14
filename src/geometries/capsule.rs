//! Port of `three.js/src/geometries/CapsuleGeometry.js`.

use crate::core::{BufferAttribute, BufferGeometry};
use crate::math::Vector3;

/// `new CapsuleGeometry( radius, height, capSegments, radialSegments, heightSegments )`.
pub fn capsule_geometry(
    radius: f64,
    height: f64,
    cap_segments: usize,
    radial_segments: usize,
    height_segments: usize,
) -> BufferGeometry {
    let height = height.max(0.0);
    let cap_segments = cap_segments.max(1);
    let radial_segments = radial_segments.max(3);
    let height_segments = height_segments.max(1);

    // buffers

    let mut indices: Vec<u32> = Vec::new();
    let mut vertices: Vec<f32> = Vec::new();
    let mut normals: Vec<f32> = Vec::new();
    let mut uvs: Vec<f32> = Vec::new();

    // helper variables

    let half_height = height / 2.0;
    let cap_arc_length = (std::f64::consts::PI / 2.0) * radius;
    let cylinder_part_length = height;
    let total_arc_length = 2.0 * cap_arc_length + cylinder_part_length;

    let num_vertical_segments = cap_segments * 2 + height_segments;
    let vertices_per_row = radial_segments + 1;

    let mut normal = Vector3::ZERO;
    let mut vertex = Vector3::ZERO;

    // generate vertices, normals, and uvs

    for iy in 0..=num_vertical_segments {
        let (profile_y, profile_radius, normal_y_component, current_arc_length) =
            if iy <= cap_segments {
                // bottom cap
                let segment_progress = iy as f64 / cap_segments as f64;
                let angle = (segment_progress * std::f64::consts::PI) / 2.0;
                (
                    -half_height - radius * angle.cos(),
                    radius * angle.sin(),
                    -radius * angle.cos(),
                    segment_progress * cap_arc_length,
                )
            } else if iy <= cap_segments + height_segments {
                // middle section
                let segment_progress = (iy - cap_segments) as f64 / height_segments as f64;
                (
                    -half_height + segment_progress * height,
                    radius,
                    0.0,
                    cap_arc_length + segment_progress * cylinder_part_length,
                )
            } else {
                // top cap
                let segment_progress =
                    (iy - cap_segments - height_segments) as f64 / cap_segments as f64;
                let angle = (segment_progress * std::f64::consts::PI) / 2.0;
                (
                    half_height + radius * angle.sin(),
                    radius * angle.cos(),
                    radius * angle.sin(),
                    cap_arc_length + cylinder_part_length + segment_progress * cap_arc_length,
                )
            };

        // `Math.min( 1, Math.max( 0, v ) )` — kept as chained min/max rather than
        // `.clamp()`, which panics on NaN input instead of matching JS's
        // fall-through behaviour.
        #[allow(clippy::manual_clamp)]
        let v = (current_arc_length / total_arc_length).min(1.0).max(0.0);

        // special case for the poles

        let mut u_offset = 0.0;

        if iy == 0 {
            u_offset = 0.5 / radial_segments as f64;
        } else if iy == num_vertical_segments {
            u_offset = -0.5 / radial_segments as f64;
        }

        for ix in 0..=radial_segments {
            let u = ix as f64 / radial_segments as f64;
            let theta = u * std::f64::consts::PI * 2.0;

            let sin_theta = theta.sin();
            let cos_theta = theta.cos();

            // vertex

            vertex.x = -profile_radius * cos_theta;
            vertex.y = profile_y;
            vertex.z = profile_radius * sin_theta;
            vertices.push(vertex.x as f32);
            vertices.push(vertex.y as f32);
            vertices.push(vertex.z as f32);

            // normal

            normal.set(
                -profile_radius * cos_theta,
                normal_y_component,
                profile_radius * sin_theta,
            );
            normal.normalize();
            normals.push(normal.x as f32);
            normals.push(normal.y as f32);
            normals.push(normal.z as f32);

            // uv

            uvs.push((u + u_offset) as f32);
            uvs.push(v as f32);
        }

        if iy > 0 {
            let prev_index_row = (iy - 1) * vertices_per_row;
            for ix in 0..radial_segments {
                let i1 = (prev_index_row + ix) as u32;
                let i2 = (prev_index_row + ix + 1) as u32;
                let i3 = (iy * vertices_per_row + ix) as u32;
                let i4 = (iy * vertices_per_row + ix + 1) as u32;

                indices.push(i1);
                indices.push(i2);
                indices.push(i3);
                indices.push(i2);
                indices.push(i4);
                indices.push(i3);
            }
        }
    }

    // build geometry

    let mut geometry = BufferGeometry::new();
    geometry.set_index(&indices);
    geometry.set_attribute("position", BufferAttribute::new(vertices, 3));
    geometry.set_attribute("normal", BufferAttribute::new(normals, 3));
    geometry.set_attribute("uv", BufferAttribute::new(uvs, 2));
    geometry
}
