//! Port of `three.js/src/geometries/PlaneGeometry.js`.

use crate::core::{BufferAttribute, BufferGeometry};

/// `new PlaneGeometry( width, height, widthSegments, heightSegments )`.
pub fn plane_geometry(
    width: f64,
    height: f64,
    width_segments: usize,
    height_segments: usize,
) -> BufferGeometry {
    let width_half = width / 2.0;
    let height_half = height / 2.0;

    let grid_x = width_segments;
    let grid_y = height_segments;

    let grid_x1 = grid_x + 1;
    let grid_y1 = grid_y + 1;

    let segment_width = width / grid_x as f64;
    let segment_height = height / grid_y as f64;

    //

    let mut indices: Vec<u32> = Vec::new();
    let mut vertices: Vec<f32> = Vec::new();
    let mut normals: Vec<f32> = Vec::new();
    let mut uvs: Vec<f32> = Vec::new();

    for iy in 0..grid_y1 {
        let y = iy as f64 * segment_height - height_half;

        for ix in 0..grid_x1 {
            let x = ix as f64 * segment_width - width_half;

            vertices.push(x as f32);
            vertices.push((-y) as f32);
            vertices.push(0.0);

            normals.push(0.0);
            normals.push(0.0);
            normals.push(1.0);

            uvs.push((ix as f64 / grid_x as f64) as f32);
            uvs.push((1.0 - (iy as f64 / grid_y as f64)) as f32);
        }
    }

    for iy in 0..grid_y {
        for ix in 0..grid_x {
            let a = (ix + grid_x1 * iy) as u32;
            let b = (ix + grid_x1 * (iy + 1)) as u32;
            let c = ((ix + 1) + grid_x1 * (iy + 1)) as u32;
            let d = ((ix + 1) + grid_x1 * iy) as u32;

            indices.push(a);
            indices.push(b);
            indices.push(d);
            indices.push(b);
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
