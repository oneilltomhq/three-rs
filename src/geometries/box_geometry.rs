//! Port of `three.js/src/geometries/BoxGeometry.js`.
//!
//! `geometry.groups` is not kept: the ladder only ever draws a box with a
//! single material, and `Renderer._projectObject()` only splits a geometry into
//! its groups when the material is an array.

use crate::core::{BufferAttribute, BufferGeometry};

/// `new BoxGeometry( width, height, depth, widthSegments, heightSegments,
/// depthSegments )`.
pub fn box_geometry(
    width: f64,
    height: f64,
    depth: f64,
    width_segments: usize,
    height_segments: usize,
    depth_segments: usize,
) -> BufferGeometry {
    let mut indices: Vec<u32> = Vec::new();
    let mut vertices: Vec<f32> = Vec::new();
    let mut normals: Vec<f32> = Vec::new();
    let mut uvs: Vec<f32> = Vec::new();

    let mut number_of_vertices = 0u32;

    // `u`, `v` and `w` as component indices into the working vector.
    #[allow(clippy::too_many_arguments)]
    fn build_plane(
        u: usize,
        v: usize,
        w: usize,
        udir: f64,
        vdir: f64,
        width: f64,
        height: f64,
        depth: f64,
        grid_x: usize,
        grid_y: usize,
        indices: &mut Vec<u32>,
        vertices: &mut Vec<f32>,
        normals: &mut Vec<f32>,
        uvs: &mut Vec<f32>,
        number_of_vertices: &mut u32,
    ) {
        let segment_width = width / grid_x as f64;
        let segment_height = height / grid_y as f64;

        let width_half = width / 2.0;
        let height_half = height / 2.0;
        let depth_half = depth / 2.0;

        let grid_x1 = grid_x + 1;
        let grid_y1 = grid_y + 1;

        let mut vertex_counter = 0u32;

        for iy in 0..grid_y1 {
            let y = iy as f64 * segment_height - height_half;

            for ix in 0..grid_x1 {
                let x = ix as f64 * segment_width - width_half;

                let mut vector = [0.0f64; 3];
                vector[u] = x * udir;
                vector[v] = y * vdir;
                vector[w] = depth_half;

                vertices.push(vector[0] as f32);
                vertices.push(vector[1] as f32);
                vertices.push(vector[2] as f32);

                let mut vector = [0.0f64; 3];
                vector[u] = 0.0;
                vector[v] = 0.0;
                vector[w] = if depth > 0.0 { 1.0 } else { -1.0 };

                normals.push(vector[0] as f32);
                normals.push(vector[1] as f32);
                normals.push(vector[2] as f32);

                uvs.push((ix as f64 / grid_x as f64) as f32);
                uvs.push((1.0 - (iy as f64 / grid_y as f64)) as f32);

                vertex_counter += 1;
            }
        }

        for iy in 0..grid_y {
            for ix in 0..grid_x {
                let a = *number_of_vertices + (ix + grid_x1 * iy) as u32;
                let b = *number_of_vertices + (ix + grid_x1 * (iy + 1)) as u32;
                let c = *number_of_vertices + ((ix + 1) + grid_x1 * (iy + 1)) as u32;
                let d = *number_of_vertices + ((ix + 1) + grid_x1 * iy) as u32;

                indices.extend_from_slice(&[a, b, d, b, c, d]);
            }
        }

        *number_of_vertices += vertex_counter;
    }

    const X: usize = 0;
    const Y: usize = 1;
    const Z: usize = 2;

    // px, nx, py, ny, pz, nz — in three.js' order.
    let planes: [(usize, usize, usize, f64, f64, f64, f64, f64, usize, usize); 6] = [
        (Z, Y, X, -1.0, -1.0, depth, height, width, depth_segments, height_segments),
        (Z, Y, X, 1.0, -1.0, depth, height, -width, depth_segments, height_segments),
        (X, Z, Y, 1.0, 1.0, width, depth, height, width_segments, depth_segments),
        (X, Z, Y, 1.0, -1.0, width, depth, -height, width_segments, depth_segments),
        (X, Y, Z, 1.0, -1.0, width, height, depth, width_segments, height_segments),
        (X, Y, Z, -1.0, -1.0, width, height, -depth, width_segments, height_segments),
    ];

    for (u, v, w, udir, vdir, pw, ph, pd, gx, gy) in planes {
        build_plane(
            u,
            v,
            w,
            udir,
            vdir,
            pw,
            ph,
            pd,
            gx,
            gy,
            &mut indices,
            &mut vertices,
            &mut normals,
            &mut uvs,
            &mut number_of_vertices,
        );
    }

    let mut geometry = BufferGeometry::new();
    geometry.set_index(&indices);
    geometry.position = Some(BufferAttribute::new(vertices, 3));
    geometry.normal = Some(BufferAttribute::new(normals, 3));
    geometry.uv = Some(BufferAttribute::new(uvs, 2));
    geometry
}
