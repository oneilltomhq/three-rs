//! Port of `three.js/src/geometries/BoxGeometry.js`.

use crate::core::{BufferAttribute, BufferGeometry};
use crate::geometries::Group;
use crate::math::Vector3;

/// `new BoxGeometry( width, height, depth, widthSegments, heightSegments, depthSegments )`.
pub fn box_geometry(
    width: f64,
    height: f64,
    depth: f64,
    width_segments: usize,
    height_segments: usize,
    depth_segments: usize,
) -> BufferGeometry {
    box_geometry_with_groups(
        width,
        height,
        depth,
        width_segments,
        height_segments,
        depth_segments,
    )
    .0
}

/// `new BoxGeometry()` — all defaults.
pub fn box_geometry_default() -> BufferGeometry {
    box_geometry(1.0, 1.0, 1.0, 1, 1, 1)
}

/// Which component of the working vector a `buildPlane` axis letter names.
#[derive(Clone, Copy)]
enum Axis {
    X,
    Y,
    Z,
}

fn set_component(v: &mut Vector3, axis: Axis, value: f64) {
    match axis {
        Axis::X => v.x = value,
        Axis::Y => v.y = value,
        Axis::Z => v.z = value,
    }
}

pub fn box_geometry_with_groups(
    width: f64,
    height: f64,
    depth: f64,
    width_segments: usize,
    height_segments: usize,
    depth_segments: usize,
) -> (BufferGeometry, Vec<Group>) {
    // buffers

    let mut indices: Vec<u32> = Vec::new();
    let mut vertices: Vec<f32> = Vec::new();
    let mut normals: Vec<f32> = Vec::new();
    let mut uvs: Vec<f32> = Vec::new();
    let mut groups: Vec<Group> = Vec::new();

    // helper variables

    let mut number_of_vertices: u32 = 0;
    let mut group_start: usize = 0;

    #[allow(clippy::too_many_arguments)]
    fn build_plane(
        u: Axis,
        v: Axis,
        w: Axis,
        udir: f64,
        vdir: f64,
        width: f64,
        height: f64,
        depth: f64,
        grid_x: usize,
        grid_y: usize,
        material_index: usize,
        indices: &mut Vec<u32>,
        vertices: &mut Vec<f32>,
        normals: &mut Vec<f32>,
        uvs: &mut Vec<f32>,
        groups: &mut Vec<Group>,
        number_of_vertices: &mut u32,
        group_start: &mut usize,
    ) {
        let segment_width = width / grid_x as f64;
        let segment_height = height / grid_y as f64;

        let width_half = width / 2.0;
        let height_half = height / 2.0;
        let depth_half = depth / 2.0;

        let grid_x1 = grid_x + 1;
        let grid_y1 = grid_y + 1;

        let mut vertex_counter: u32 = 0;
        let mut group_count: usize = 0;

        let mut vector = Vector3::ZERO;

        // generate vertices, normals and uvs

        for iy in 0..grid_y1 {
            let y = iy as f64 * segment_height - height_half;

            for ix in 0..grid_x1 {
                let x = ix as f64 * segment_width - width_half;

                // set values to correct vector component

                set_component(&mut vector, u, x * udir);
                set_component(&mut vector, v, y * vdir);
                set_component(&mut vector, w, depth_half);

                // now apply vector to vertex buffer

                vertices.push(vector.x as f32);
                vertices.push(vector.y as f32);
                vertices.push(vector.z as f32);

                // set values to correct vector component

                set_component(&mut vector, u, 0.0);
                set_component(&mut vector, v, 0.0);
                set_component(&mut vector, w, if depth > 0.0 { 1.0 } else { -1.0 });

                // now apply vector to normal buffer

                normals.push(vector.x as f32);
                normals.push(vector.y as f32);
                normals.push(vector.z as f32);

                // uvs

                uvs.push((ix as f64 / grid_x as f64) as f32);
                uvs.push((1.0 - (iy as f64 / grid_y as f64)) as f32);

                // counters

                vertex_counter += 1;
            }
        }

        // indices

        for iy in 0..grid_y {
            for ix in 0..grid_x {
                let a = *number_of_vertices + (ix + grid_x1 * iy) as u32;
                let b = *number_of_vertices + (ix + grid_x1 * (iy + 1)) as u32;
                let c = *number_of_vertices + ((ix + 1) + grid_x1 * (iy + 1)) as u32;
                let d = *number_of_vertices + ((ix + 1) + grid_x1 * iy) as u32;

                // faces

                indices.push(a);
                indices.push(b);
                indices.push(d);
                indices.push(b);
                indices.push(c);
                indices.push(d);

                // increase counter

                group_count += 6;
            }
        }

        // add a group to the geometry. this will ensure multi material support

        groups.push(Group::new(*group_start, group_count, material_index));

        // calculate new start value for groups

        *group_start += group_count;

        // update total number of vertices

        *number_of_vertices += vertex_counter;
    }

    macro_rules! plane {
        ($u:expr, $v:expr, $w:expr, $udir:expr, $vdir:expr, $width:expr, $height:expr, $depth:expr, $gx:expr, $gy:expr, $mi:expr) => {
            build_plane(
                $u,
                $v,
                $w,
                $udir,
                $vdir,
                $width,
                $height,
                $depth,
                $gx,
                $gy,
                $mi,
                &mut indices,
                &mut vertices,
                &mut normals,
                &mut uvs,
                &mut groups,
                &mut number_of_vertices,
                &mut group_start,
            )
        };
    }

    // build each side of the box geometry

    // px
    plane!(Axis::Z, Axis::Y, Axis::X, -1.0, -1.0, depth, height, width, depth_segments, height_segments, 0);
    // nx
    plane!(Axis::Z, Axis::Y, Axis::X, 1.0, -1.0, depth, height, -width, depth_segments, height_segments, 1);
    // py
    plane!(Axis::X, Axis::Z, Axis::Y, 1.0, 1.0, width, depth, height, width_segments, depth_segments, 2);
    // ny
    plane!(Axis::X, Axis::Z, Axis::Y, 1.0, -1.0, width, depth, -height, width_segments, depth_segments, 3);
    // pz
    plane!(Axis::X, Axis::Y, Axis::Z, 1.0, -1.0, width, height, depth, width_segments, height_segments, 4);
    // nz
    plane!(Axis::X, Axis::Y, Axis::Z, -1.0, -1.0, width, height, -depth, width_segments, height_segments, 5);

    // build geometry

    let mut geometry = BufferGeometry::new();
    geometry.set_index(&indices);
    geometry.set_attribute("position", BufferAttribute::new(vertices, 3));
    geometry.set_attribute("normal", BufferAttribute::new(normals, 3));
    geometry.set_attribute("uv", BufferAttribute::new(uvs, 2));
    (geometry, groups)
}
