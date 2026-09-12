//! Port of `three.js/test/unit/src/core/BufferGeometry.tests.js`.
//!
//! The Rust geometry holds three named attributes (`position`, `normal`, `uv`)
//! plus an index rather than a generic attribute map, and carries no uuid/name/
//! groups/drawRange/morph data. So these are skipped: `Extending`, `Instancing`,
//! `type`, `isBufferGeometry`, `set / delete Attribute`, `addGroup`,
//! `setDrawRange`, `applyQuaternion`, `rotateX/Y/Z`, `translate`, `lookAt`,
//! `center`, `toNonIndexed`, `toJSON`, `clone`, `copy`, `dispose`, and the
//! Float16 variants of computeBounding{Box,Sphere} (the port stores f32 only).
//!
//! Divergence: three's `computeVertexNormals` line case (a position count that
//! is not a multiple of three) reads past the end of the typed array and stores
//! NaN; in Rust that indexed out of bounds and panicked, so the loops now skip a
//! trailing partial triangle and leave those normals at zero. The test accepts
//! either, as three's own assertion does (`! normals[ i ]`).

mod support;

use three_rs::core::{BufferAttribute, BufferGeometry, Index};
use three_rs::math::{Matrix4, Vector3};

fn geometry_with(vertices: Vec<f32>) -> BufferGeometry {
    let mut geometry = BufferGeometry::new();
    geometry.position = Some(BufferAttribute::new(vertices, 3));
    geometry
}

fn normals_for_vertices(vertices: Vec<f32>) -> Vec<f32> {
    let mut geometry = geometry_with(vertices);
    geometry.compute_vertex_normals();
    let normal = geometry
        .normal
        .as_ref()
        .expect("normal attribute was created");
    normal.array.clone()
}

#[track_caller]
fn attribute_equals(a: &BufferAttribute, b: &BufferAttribute) {
    let tolerance = 0.0001_f32;
    assert_eq!(a.count(), b.count(), "count");
    assert_eq!(a.item_size, b.item_size, "itemSize");
    for i in 0..a.count() * a.item_size {
        let delta = (a.array[i] - b.array[i]).abs();
        assert!(
            delta <= tolerance,
            "element {i}: {} vs {}",
            a.array[i],
            b.array[i]
        );
    }
}

#[test]
fn set_index_get_index() {
    let mut a = BufferGeometry::new();
    let uint16 = [1_u32, 2, 3];
    let uint32 = [65535_u32, 65536, 65537];

    a.set_index(&uint16);
    match a.index.as_ref().unwrap() {
        Index::U16(v) => assert_eq!(v, &[1_u16, 2, 3], "Small index gets stored correctly"),
        Index::U32(_) => panic!("Index has the right type"),
    }

    a.set_index(&uint32);
    match a.index.as_ref().unwrap() {
        Index::U32(v) => assert_eq!(v, &uint32, "Large index gets stored correctly"),
        Index::U16(_) => panic!("Index has the right type"),
    }
}

#[test]
fn apply_matrix4() {
    let mut geometry = geometry_with(vec![0.0; 6]);

    #[rustfmt::skip]
    let matrix = Matrix4::from_rows(
        1.0, 0.0, 0.0, 1.5,
        0.0, 1.0, 0.0, -2.0,
        0.0, 0.0, 1.0, 3.0,
        0.0, 0.0, 0.0, 1.0,
    );
    geometry.apply_matrix4(&matrix);

    let position = &geometry.position.as_ref().unwrap().array;
    let m = matrix.elements;
    assert!(
        position[0] as f64 == m[12] && position[1] as f64 == m[13] && position[2] as f64 == m[14],
        "position was extracted from matrix"
    );
    assert!(
        position[3] as f64 == m[12] && position[4] as f64 == m[13] && position[5] as f64 == m[14],
        "position was extracted from matrix twice"
    );
}

#[test]
fn scale() {
    let mut geometry = geometry_with(vec![-1.0, -1.0, -1.0, 2.0, 2.0, 2.0]);

    geometry.scale(1.0, 2.0, 3.0);

    let pos = &geometry.position.as_ref().unwrap().array;
    assert!(
        pos[0] == -1.0
            && pos[1] == -2.0
            && pos[2] == -3.0
            && pos[3] == 2.0
            && pos[4] == 4.0
            && pos[5] == 6.0,
        "vertices were scaled"
    );
}

#[test]
fn compute_bounding_box() {
    let bb = geometry_with(vec![-1.0, -2.0, -3.0, 13.0, -2.0, -3.5, -1.0, -20.0, 0.0, -4.0, 5.0, 6.0])
        .compute_bounding_box()
        .unwrap();

    assert!(
        bb.min.x == -4.0 && bb.min.y == -20.0 && bb.min.z == -3.5,
        "min values are set correctly"
    );
    assert!(
        bb.max.x == 13.0 && bb.max.y == 5.0 && bb.max.z == 6.0,
        "max values are set correctly"
    );

    let bb = geometry_with(vec![-1.0, -1.0, -1.0])
        .compute_bounding_box()
        .unwrap();

    assert!(
        bb.min.x == bb.max.x && bb.min.y == bb.max.y && bb.min.z == bb.max.z,
        "since there is only one vertex, max and min are equal"
    );
    assert!(
        bb.min.x == -1.0 && bb.min.y == -1.0 && bb.min.z == -1.0,
        "since there is only one vertex, min and max are this vertex"
    );
}

#[test]
fn compute_bounding_sphere() {
    let bs = geometry_with(vec![-10.0, 0.0, 0.0, 10.0, 0.0, 0.0])
        .compute_bounding_sphere()
        .unwrap();

    assert!(bs.radius == 10.0, "radius is equal to deltaMinMax / 2");
    assert!(
        bs.center.x == 0.0 && bs.center.y == 0.0 && bs.center.z == 0.0,
        "bounding sphere is at ( 0, 0, 0 )"
    );

    let bs = geometry_with(vec![-5.0, 11.0, -3.0, 5.0, -11.0, 3.0])
        .compute_bounding_sphere()
        .unwrap();
    let radius = Vector3::new(5.0, 11.0, 3.0).length();

    assert!(bs.radius == radius, "radius is equal to directionLength");
    assert!(
        bs.center.x == 0.0 && bs.center.y == 0.0 && bs.center.z == 0.0,
        "bounding sphere is at ( 0, 0, 0 )"
    );
}

#[test]
fn compute_bounding_box_empty() {
    assert!(
        BufferGeometry::new().compute_bounding_box().is_none(),
        "no position attribute means no box"
    );
    assert!(
        BufferGeometry::new().compute_bounding_sphere().is_none(),
        "no position attribute means no sphere"
    );
}

#[test]
fn compute_vertex_normals() {
    // counter-clockwise triangle
    let normals = normals_for_vertices(vec![-1.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0]);
    assert!(
        normals[0] == 0.0 && normals[1] == 0.0 && normals[2] == 1.0,
        "first normal is pointing to screen since the triangle was created counter clockwise"
    );
    assert!(
        normals[3] == 0.0 && normals[4] == 0.0 && normals[5] == 1.0,
        "second normal is pointing to screen since the triangle was created counter clockwise"
    );
    assert!(
        normals[6] == 0.0 && normals[7] == 0.0 && normals[8] == 1.0,
        "third normal is pointing to screen since the triangle was created counter clockwise"
    );

    // clockwise triangle
    let normals = normals_for_vertices(vec![1.0, 0.0, 0.0, -1.0, 0.0, 0.0, 0.0, 1.0, 0.0]);
    assert!(
        normals[0] == 0.0 && normals[1] == 0.0 && normals[2] == -1.0,
        "first normal is pointing to screen since the triangle was created clockwise"
    );
    assert!(
        normals[3] == 0.0 && normals[4] == 0.0 && normals[5] == -1.0,
        "second normal is pointing to screen since the triangle was created clockwise"
    );
    assert!(
        normals[6] == 0.0 && normals[7] == 0.0 && normals[8] == -1.0,
        "third normal is pointing to screen since the triangle was created clockwise"
    );

    let normals = normals_for_vertices(vec![0.0, 0.0, 1.0, 0.0, 0.0, -1.0, 1.0, 1.0, 0.0]);
    // the triangle is rotated 45 degrees, so the normals should point along
    // (1, -1, 0).normalize(); check via a vector at 90 degrees to them.
    let direction = *Vector3::new(1.0, 1.0, 0.0).normalize();
    let difference = direction.dot(&Vector3::new(
        normals[0] as f64,
        normals[1] as f64,
        normals[2] as f64,
    ));
    assert!(
        difference < f64::EPSILON,
        "normal is equal to reference vector: {difference}"
    );

    // a line has no triangle, so normals cannot be calculated
    let normals = normals_for_vertices(vec![1.0, 0.0, 0.0, -1.0, 0.0, 0.0]);
    for (i, n) in normals.iter().enumerate() {
        assert!(
            *n == 0.0 || n.is_nan(),
            "normals can't be calculated which is good (element {i} = {n})"
        );
    }
}

#[test]
fn compute_vertex_normals_indexed() {
    let sqrt = 0.5 * 2.0_f32.sqrt();
    #[rustfmt::skip]
    let normal = BufferAttribute::new(
        vec![
            -1.0, 0.0, 0.0, -1.0, 0.0, 0.0, -1.0, 0.0, 0.0,
            sqrt, sqrt, 0.0, sqrt, sqrt, 0.0, sqrt, sqrt, 0.0,
        ],
        3,
    );
    #[rustfmt::skip]
    let position = BufferAttribute::new(
        vec![
            0.5, 0.5, 0.5, 0.5, 0.5, -0.5, 0.5, -0.5, 0.5,
            0.5, -0.5, -0.5, -0.5, 0.5, -0.5, -0.5, 0.5, 0.5,
        ],
        3,
    );
    // the index buffer defines the same two triangles but in counter-clockwise
    // order, which should result in flipped normals.
    let mut flipped_normals = normal.clone();
    let mut flip = Matrix4::identity();
    flip.make_scale(-1.0, -1.0, -1.0);
    flipped_normals.apply_matrix4(&flip);

    let mut a = BufferGeometry::new();
    a.position = Some(position.clone());
    a.compute_vertex_normals();
    attribute_equals(&normal, a.normal.as_ref().unwrap());

    // a second time, to see if the existing normals get properly reset
    a.compute_vertex_normals();
    attribute_equals(&normal, a.normal.as_ref().unwrap());

    // indexed geometry
    let mut a = BufferGeometry::new();
    a.position = Some(position);
    a.set_index(&[0, 2, 1, 3, 5, 4]);
    a.compute_vertex_normals();
    attribute_equals(&flipped_normals, a.normal.as_ref().unwrap());
}
