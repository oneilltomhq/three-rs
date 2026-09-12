//! Port of the addon `three.js/examples/jsm/geometries/RoundedBoxGeometry.js`.
//!
//! Needed by `webgpu_postprocessing_ao`
//! (`new RoundedBoxGeometry( 0.9, 0.25, 0.8, 4, 0.06 )`).

use crate::core::BufferGeometry;
use crate::geometries::box_geometry;
use crate::math::Vector3;

/// `Math.sign()` — `0`, `-0` and `NaN` come back unchanged, which `f64::signum`
/// does not do.
fn js_sign(x: f64) -> f64 {
    if x > 0.0 {
        1.0
    } else if x < 0.0 {
        -1.0
    } else {
        x
    }
}

/// `Vector3.angleTo()`.
fn angle_to(a: &Vector3, b: &Vector3) -> f64 {
    let denominator = (a.length_sq() * b.length_sq()).sqrt();

    if denominator == 0.0 {
        return std::f64::consts::PI / 2.0;
    }

    let theta = a.dot(b) / denominator;

    // clamp, to handle numerical problems
    theta.clamp(-1.0, 1.0).acos()
}

/// Which component of the working vector an axis letter names.
#[derive(Clone, Copy)]
enum Axis {
    X,
    Y,
    Z,
}

fn get(v: &Vector3, axis: Axis) -> f64 {
    match axis {
        Axis::X => v.x,
        Axis::Y => v.y,
        Axis::Z => v.z,
    }
}

fn set(v: &mut Vector3, axis: Axis, value: f64) {
    match axis {
        Axis::X => v.x = value,
        Axis::Y => v.y = value,
        Axis::Z => v.z = value,
    }
}

fn get_uv(
    face_dir_vector: &Vector3,
    normal: &Vector3,
    uv_axis: Axis,
    projection_axis: Axis,
    radius: f64,
    side_length: f64,
) -> f64 {
    let tot_arc_length = 2.0 * std::f64::consts::PI * radius / 4.0;

    // length of the planes between the arcs on each axis
    let center_length = (side_length - 2.0 * radius).max(0.0);
    let half_arc = std::f64::consts::PI / 4.0;

    // Get the vector projected onto the Y plane
    let mut temp_normal = *normal;
    set(&mut temp_normal, projection_axis, 0.0);
    temp_normal.normalize();

    // total amount of UV space alloted to a single arc
    let arc_uv_ratio = 0.5 * tot_arc_length / (tot_arc_length + center_length);

    // the distance along one arc the point is at
    let arc_angle_ratio = 1.0 - (angle_to(&temp_normal, face_dir_vector) / half_arc);

    if js_sign(get(&temp_normal, uv_axis)) == 1.0 {
        arc_angle_ratio * arc_uv_ratio
    } else {
        // total amount of UV space alloted to the plane between the arcs
        let len_uv = center_length / (tot_arc_length + center_length);
        len_uv + arc_uv_ratio + arc_uv_ratio * (1.0 - arc_angle_ratio)
    }
}

/// `new RoundedBoxGeometry( width, height, depth, segments, radius )`.
///
/// The addon extends `BoxGeometry`, so it inherits the box's six groups —
/// which index the pre-`toNonIndexed()` index buffer, exactly as in three.js.
pub fn rounded_box_geometry(
    width: f64,
    height: f64,
    depth: f64,
    segments: usize,
    radius: f64,
) -> BufferGeometry {
    // calculate total segments needed &
    // ensure it's odd so that we have a plane connecting the rounded corners
    let total_segments = segments * 2 + 1;

    // ensure radius isn't bigger than shortest side
    let radius = (width / 2.0).min(height / 2.0).min(depth / 2.0).min(radius);

    // start with a unit box geometry, its vertices will be modified to form the rounded box
    let mut geometry =
        box_geometry(1.0, 1.0, 1.0, total_segments, total_segments, total_segments);

    // if totalSegments is 1, no rounding is needed - return regular box
    // (the addon returns the *unit* box here, width/height/depth unused)
    if total_segments == 1 {
        return geometry;
    }

    let geometry2 = geometry.to_non_indexed();

    geometry.index = None;
    let mut positions_attr = geometry2.position().cloned().unwrap();
    let mut normals = geometry2.normal().cloned().unwrap();
    let mut uvs = geometry2.uv().cloned().unwrap();

    //

    let mut position = Vector3::ZERO;
    let mut normal;

    let mut box_v = Vector3::new(width, height, depth);
    box_v.divide_scalar(2.0);
    box_v.x -= radius;
    box_v.y -= radius;
    box_v.z -= radius;

    let positions = &mut positions_attr.array;

    let face_tris = positions.len() / 6;
    let mut face_dir_vector = Vector3::ZERO;
    let half_segment_size = 0.5 / total_segments as f64;

    // the three buffers are walked together; normals/uvs are taken out and put
    // back so the borrow checker stays happy — the values and the order of the
    // f32 stores are the addon's.

    let mut i = 0usize;
    let mut j = 0usize;
    while i < positions.len() {
        position.set(
            positions[i] as f64,
            positions[i + 1] as f64,
            positions[i + 2] as f64,
        );
        normal = position;
        normal.x -= js_sign(normal.x) * half_segment_size;
        normal.y -= js_sign(normal.y) * half_segment_size;
        normal.z -= js_sign(normal.z) * half_segment_size;
        normal.normalize();

        positions[i] = (box_v.x * js_sign(position.x) + normal.x * radius) as f32;
        positions[i + 1] = (box_v.y * js_sign(position.y) + normal.y * radius) as f32;
        positions[i + 2] = (box_v.z * js_sign(position.z) + normal.z * radius) as f32;

        normals.array[i] = normal.x as f32;
        normals.array[i + 1] = normal.y as f32;
        normals.array[i + 2] = normal.z as f32;

        let side = i / face_tris;

        match side {
            0 => {
                // right — generate UVs along Z then Y
                face_dir_vector.set(1.0, 0.0, 0.0);
                uvs.array[j] =
                    get_uv(&face_dir_vector, &normal, Axis::Z, Axis::Y, radius, depth) as f32;
                uvs.array[j + 1] = (1.0
                    - get_uv(&face_dir_vector, &normal, Axis::Y, Axis::Z, radius, height))
                    as f32;
            }
            1 => {
                // left — generate UVs along Z then Y
                face_dir_vector.set(-1.0, 0.0, 0.0);
                uvs.array[j] = (1.0
                    - get_uv(&face_dir_vector, &normal, Axis::Z, Axis::Y, radius, depth))
                    as f32;
                uvs.array[j + 1] = (1.0
                    - get_uv(&face_dir_vector, &normal, Axis::Y, Axis::Z, radius, height))
                    as f32;
            }
            2 => {
                // top — generate UVs along X then Z
                face_dir_vector.set(0.0, 1.0, 0.0);
                uvs.array[j] = (1.0
                    - get_uv(&face_dir_vector, &normal, Axis::X, Axis::Z, radius, width))
                    as f32;
                uvs.array[j + 1] =
                    get_uv(&face_dir_vector, &normal, Axis::Z, Axis::X, radius, depth) as f32;
            }
            3 => {
                // bottom — generate UVs along X then Z
                face_dir_vector.set(0.0, -1.0, 0.0);
                uvs.array[j] = (1.0
                    - get_uv(&face_dir_vector, &normal, Axis::X, Axis::Z, radius, width))
                    as f32;
                uvs.array[j + 1] = (1.0
                    - get_uv(&face_dir_vector, &normal, Axis::Z, Axis::X, radius, depth))
                    as f32;
            }
            4 => {
                // front — generate UVs along X then Y
                face_dir_vector.set(0.0, 0.0, 1.0);
                uvs.array[j] = (1.0
                    - get_uv(&face_dir_vector, &normal, Axis::X, Axis::Y, radius, width))
                    as f32;
                uvs.array[j + 1] = (1.0
                    - get_uv(&face_dir_vector, &normal, Axis::Y, Axis::X, radius, height))
                    as f32;
            }
            5 => {
                // back — generate UVs along X then Y
                face_dir_vector.set(0.0, 0.0, -1.0);
                uvs.array[j] =
                    get_uv(&face_dir_vector, &normal, Axis::X, Axis::Y, radius, width) as f32;
                uvs.array[j + 1] = (1.0
                    - get_uv(&face_dir_vector, &normal, Axis::Y, Axis::X, radius, height))
                    as f32;
            }
            _ => {}
        }

        i += 3;
        j += 2;
    }

    geometry.set_attribute("position", positions_attr);
    geometry.set_attribute("normal", normals);
    geometry.set_attribute("uv", uvs);
    geometry
}
