//! Port of `three.js/src/geometries/CylinderGeometry.js`.

use crate::core::{BufferAttribute, BufferGeometry, Group};
use crate::math::Vector3;

/// `new CylinderGeometry( radiusTop, radiusBottom, height, radialSegments )`
/// with the default `heightSegments`/`openEnded`/`thetaStart`/`thetaLength`.
pub fn cylinder_geometry(
    radius_top: f64,
    radius_bottom: f64,
    height: f64,
    radial_segments: usize,
) -> BufferGeometry {
    cylinder_geometry_full(
        radius_top,
        radius_bottom,
        height,
        radial_segments,
        1,
        false,
        0.0,
        std::f64::consts::PI * 2.0,
    )
}

/// `new CylinderGeometry( ... )` — sets the torso group plus one group per cap,
/// as three.js does.
#[allow(clippy::too_many_arguments)]
pub fn cylinder_geometry_full(
    radius_top: f64,
    radius_bottom: f64,
    height: f64,
    radial_segments: usize,
    height_segments: usize,
    open_ended: bool,
    theta_start: f64,
    theta_length: f64,
) -> BufferGeometry {
    // buffers

    let mut indices: Vec<u32> = Vec::new();
    let mut vertices: Vec<f32> = Vec::new();
    let mut normals: Vec<f32> = Vec::new();
    let mut uvs: Vec<f32> = Vec::new();
    let mut groups: Vec<Group> = Vec::new();

    // helper variables

    let mut index: u32 = 0;
    let mut index_array: Vec<Vec<u32>> = Vec::new();
    let half_height = height / 2.0;
    let mut group_start: usize = 0;

    // generate geometry

    // generateTorso()
    {
        let mut normal = Vector3::ZERO;
        let mut vertex = Vector3::ZERO;

        let mut group_count: usize = 0;

        // this will be used to calculate the normal
        let slope = (radius_bottom - radius_top) / height;

        // generate vertices, normals and uvs

        for y in 0..=height_segments {
            let mut index_row: Vec<u32> = Vec::new();

            let v = y as f64 / height_segments as f64;

            // calculate the radius of the current row

            let radius = v * (radius_bottom - radius_top) + radius_top;

            for x in 0..=radial_segments {
                let u = x as f64 / radial_segments as f64;

                let theta = u * theta_length + theta_start;

                let sin_theta = theta.sin();
                let cos_theta = theta.cos();

                // vertex

                vertex.x = radius * sin_theta;
                vertex.y = -v * height + half_height;
                vertex.z = radius * cos_theta;
                vertices.push(vertex.x as f32);
                vertices.push(vertex.y as f32);
                vertices.push(vertex.z as f32);

                // normal

                normal.set(sin_theta, slope, cos_theta).normalize();
                normals.push(normal.x as f32);
                normals.push(normal.y as f32);
                normals.push(normal.z as f32);

                // uv

                uvs.push(u as f32);
                uvs.push((1.0 - v) as f32);

                // save index of vertex in respective row

                index_row.push(index);
                index += 1;
            }

            // now save vertices of the row in our index array

            index_array.push(index_row);
        }

        // generate indices

        for x in 0..radial_segments {
            for y in 0..height_segments {
                // we use the index array to access the correct indices

                let a = index_array[y][x];
                let b = index_array[y + 1][x];
                let c = index_array[y + 1][x + 1];
                let d = index_array[y][x + 1];

                // faces

                if radius_top > 0.0 || y != 0 {
                    indices.push(a);
                    indices.push(b);
                    indices.push(d);
                    group_count += 3;
                }

                if radius_bottom > 0.0 || y != height_segments - 1 {
                    indices.push(b);
                    indices.push(c);
                    indices.push(d);
                    group_count += 3;
                }
            }
        }

        groups.push(Group {
            start: group_start,
            count: group_count,
            material_index: 0,
        });
        group_start += group_count;
    }

    if !open_ended {
        if radius_top > 0.0 {
            generate_cap(
                true,
                radius_top,
                radius_bottom,
                half_height,
                radial_segments,
                theta_start,
                theta_length,
                &mut index,
                &mut indices,
                &mut vertices,
                &mut normals,
                &mut uvs,
                &mut groups,
                &mut group_start,
            );
        }
        if radius_bottom > 0.0 {
            generate_cap(
                false,
                radius_top,
                radius_bottom,
                half_height,
                radial_segments,
                theta_start,
                theta_length,
                &mut index,
                &mut indices,
                &mut vertices,
                &mut normals,
                &mut uvs,
                &mut groups,
                &mut group_start,
            );
        }
    }

    // build geometry

    let mut geometry = BufferGeometry::new();
    geometry.set_index(&indices);
    geometry.set_attribute("position", BufferAttribute::new(vertices, 3));
    geometry.set_attribute("normal", BufferAttribute::new(normals, 3));
    geometry.set_attribute("uv", BufferAttribute::new(uvs, 2));
    for group in groups {
        geometry.add_group(group.start, group.count, group.material_index);
    }
    geometry
}

#[allow(clippy::too_many_arguments)]
fn generate_cap(
    top: bool,
    radius_top: f64,
    radius_bottom: f64,
    half_height: f64,
    radial_segments: usize,
    theta_start: f64,
    theta_length: f64,
    index: &mut u32,
    indices: &mut Vec<u32>,
    vertices: &mut Vec<f32>,
    normals: &mut Vec<f32>,
    uvs: &mut Vec<f32>,
    groups: &mut Vec<Group>,
    group_start: &mut usize,
) {
    // save the index of the first center vertex
    let center_index_start = *index;

    let mut uv_x;
    let mut uv_y;
    let mut vertex = Vector3::ZERO;

    let mut group_count: usize = 0;

    let radius = if top { radius_top } else { radius_bottom };
    let sign: f64 = if top { 1.0 } else { -1.0 };

    // first we generate the center vertex data of the cap.
    // because the geometry needs one set of uvs per face,
    // we must generate a center vertex per face/segment

    for _x in 1..=radial_segments {
        // vertex

        vertices.push(0.0);
        vertices.push((half_height * sign) as f32);
        vertices.push(0.0);

        // normal

        normals.push(0.0);
        normals.push(sign as f32);
        normals.push(0.0);

        // uv

        uvs.push(0.5);
        uvs.push(0.5);

        // increase index

        *index += 1;
    }

    // save the index of the last center vertex
    let center_index_end = *index;

    // now we generate the surrounding vertices, normals and uvs

    for x in 0..=radial_segments {
        let u = x as f64 / radial_segments as f64;
        let theta = u * theta_length + theta_start;

        let cos_theta = theta.cos();
        let sin_theta = theta.sin();

        // vertex

        vertex.x = radius * sin_theta;
        vertex.y = half_height * sign;
        vertex.z = radius * cos_theta;
        vertices.push(vertex.x as f32);
        vertices.push(vertex.y as f32);
        vertices.push(vertex.z as f32);

        // normal

        normals.push(0.0);
        normals.push(sign as f32);
        normals.push(0.0);

        // uv

        uv_x = (cos_theta * 0.5) + 0.5;
        uv_y = (sin_theta * 0.5 * sign) + 0.5;
        uvs.push(uv_x as f32);
        uvs.push(uv_y as f32);

        // increase index

        *index += 1;
    }

    // generate indices

    for x in 0..radial_segments {
        let c = center_index_start + x as u32;
        let i = center_index_end + x as u32;

        if top {
            // face top

            indices.push(i);
            indices.push(i + 1);
            indices.push(c);
        } else {
            // face bottom

            indices.push(i + 1);
            indices.push(i);
            indices.push(c);
        }

        group_count += 3;
    }

    groups.push(Group {
        start: *group_start,
        count: group_count,
        material_index: if top { 1 } else { 2 },
    });
    *group_start += group_count;
}
