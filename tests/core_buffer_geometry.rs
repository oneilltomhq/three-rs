//! Port of `three.js/test/unit/src/core/BufferGeometry.tests.js`.
//!
//! Skipped: `Extending`, `Instancing`, `type` and `isBufferGeometry` (no class
//! hierarchy or `type` string), `toJSON`, `clone`, `copy`, `dispose` (no
//! serialisation, and `Clone` is derived), and the Float16 variants of
//! computeBounding{Box,Sphere} (the port stores f32 arrays only).
//!
//! three.js has no test covering the morph-target branches of
//! computeBounding{Box,Sphere}; the two below (`compute_bounding_box_morph` and
//! `compute_bounding_sphere_morph`) are this port's own, checking the ported
//! branch against the arithmetic in `BufferGeometry.js`.
//!
//! Divergence: three's `computeVertexNormals` line case (a position count that
//! is not a multiple of three) reads past the end of the typed array and stores
//! NaN; in Rust that indexed out of bounds and panicked, so the loops now skip a
//! trailing partial triangle and leave those normals at zero. The test accepts
//! either, as three's own assertion does (`! normals[ i ]`).

mod support;

use three_rs::core::{BufferAttribute, BufferGeometry, Index};
use three_rs::math::{Matrix4, Quaternion, Vector3};

fn geometry_with(vertices: Vec<f32>) -> BufferGeometry {
    let mut geometry = BufferGeometry::new();
    geometry.set_attribute("position", BufferAttribute::new(vertices, 3));
    geometry
}

fn normals_for_vertices(vertices: Vec<f32>) -> Vec<f32> {
    let mut geometry = geometry_with(vertices);
    geometry.compute_vertex_normals();
    let normal = geometry
        .normal()
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

    let position = &geometry.position().unwrap().array;
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

    let pos = &geometry.position().unwrap().array;
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
    a.set_attribute("position", position.clone());
    a.compute_vertex_normals();
    attribute_equals(&normal, a.normal().unwrap());

    // a second time, to see if the existing normals get properly reset
    a.compute_vertex_normals();
    attribute_equals(&normal, a.normal().unwrap());

    // indexed geometry
    let mut a = BufferGeometry::new();
    a.set_attribute("position", position);
    a.set_index(&[0, 2, 1, 3, 5, 4]);
    a.compute_vertex_normals();
    attribute_equals(&flipped_normals, a.normal().unwrap());
}

/// `new BufferGeometry()` with a `position` attribute of the given vertices.
fn position_geometry(vertices: Vec<f32>) -> BufferGeometry {
    geometry_with(vertices)
}

/// `geometry.attributes.position.array`.
fn pos(geometry: &BufferGeometry) -> Vec<f32> {
    geometry.position().unwrap().array.clone()
}

#[test]
fn set_delete_attribute() {
    let mut geometry = BufferGeometry::new();
    let attribute_name = "position";

    assert!(
        !geometry.has_attribute(attribute_name),
        "no attribute defined"
    );

    geometry.set_attribute(attribute_name, BufferAttribute::new(vec![1.0, 2.0, 3.0], 1));

    assert!(geometry.has_attribute(attribute_name), "attribute is defined");
    assert!(
        geometry.get_attribute(attribute_name).is_some(),
        "attribute is defined"
    );

    geometry.delete_attribute(attribute_name);

    assert!(
        !geometry.has_attribute(attribute_name),
        "no attribute defined"
    );
}

#[test]
fn add_group() {
    use three_rs::core::Group;

    let mut a = BufferGeometry::new();
    let expected = [
        Group {
            start: 0,
            count: 1,
            material_index: 0,
        },
        Group {
            start: 1,
            count: 2,
            material_index: 2,
        },
    ];

    a.add_group(0, 1, 0);
    a.add_group(1, 2, 2);

    assert_eq!(
        a.groups, expected,
        "Check groups were stored correctly and in order"
    );

    a.clear_groups();
    assert_eq!(a.groups.len(), 0, "Check groups were deleted correctly");
}

#[test]
fn set_draw_range() {
    use three_rs::core::DrawRange;

    let mut a = BufferGeometry::new();

    assert_eq!(
        a.draw_range,
        DrawRange {
            start: 0,
            count: None
        },
        "The default draw range is the whole geometry (three's `count: Infinity`)"
    );

    a.set_draw_range(1, 7);

    assert_eq!(
        a.draw_range,
        DrawRange {
            start: 1,
            count: Some(7)
        },
        "Check draw range was stored correctly"
    );
}

#[test]
fn apply_quaternion() {
    let mut geometry = position_geometry(vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0]);

    let q = Quaternion::new(0.5, 0.5, 0.5, 0.5);
    geometry.apply_quaternion(&q);

    let pos = pos(&geometry);

    // geometry was rotated around the (1, 1, 1) axis.
    assert!(
        pos[0] == 3.0
            && pos[1] == 1.0
            && pos[2] == 2.0
            && pos[3] == 6.0
            && pos[4] == 4.0
            && pos[5] == 5.0,
        "vertices were rotated properly: {pos:?}"
    );
}

#[test]
fn rotate_x_y_z() {
    const DEG_TO_RAD: f64 = std::f64::consts::PI / 180.0;

    let mut geometry = position_geometry(vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0]);

    geometry.rotate_x(180.0 * DEG_TO_RAD);

    // object was rotated around x so all items should be flipped but the x ones
    let p = pos(&geometry);
    assert!(
        p[0] == 1.0 && p[1] == -2.0 && p[2] == -3.0 && p[3] == 4.0 && p[4] == -5.0 && p[5] == -6.0,
        "vertices were rotated around x by 180 degrees: {p:?}"
    );

    geometry.rotate_y(180.0 * DEG_TO_RAD);

    // vertices were rotated around y so all items should be flipped again but
    // the y ones
    let p = pos(&geometry);
    assert!(
        p[0] == -1.0 && p[1] == -2.0 && p[2] == 3.0 && p[3] == -4.0 && p[4] == -5.0 && p[5] == 6.0,
        "vertices were rotated around y by 180 degrees: {p:?}"
    );

    geometry.rotate_z(180.0 * DEG_TO_RAD);

    // vertices were rotated around z so all items should be flipped again but
    // the z ones
    let p = pos(&geometry);
    assert!(
        p[0] == 1.0 && p[1] == 2.0 && p[2] == 3.0 && p[3] == 4.0 && p[4] == 5.0 && p[5] == 6.0,
        "vertices were rotated around z by 180 degrees: {p:?}"
    );
}

#[test]
fn translate() {
    let mut geometry = position_geometry(vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0]);

    geometry.translate(10.0, 20.0, 30.0);

    let p = pos(&geometry);
    assert!(
        p[0] == 11.0
            && p[1] == 22.0
            && p[2] == 33.0
            && p[3] == 14.0
            && p[4] == 25.0
            && p[5] == 36.0,
        "vertices were translated: {p:?}"
    );
}

#[test]
fn look_at() {
    let mut a = position_geometry(vec![
        -1.0, -1.0, 1.0, //
        1.0, -1.0, 1.0, //
        1.0, 1.0, 1.0, //
        1.0, 1.0, 1.0, //
        -1.0, 1.0, 1.0, //
        -1.0, -1.0, 1.0,
    ]);

    let sqrt = 2.0_f32.sqrt();
    let expected = BufferAttribute::new(
        vec![
            1.0, 0.0, -sqrt, //
            -1.0, 0.0, -sqrt, //
            -1.0, sqrt, 0.0, //
            -1.0, sqrt, 0.0, //
            1.0, sqrt, 0.0, //
            1.0, 0.0, -sqrt,
        ],
        3,
    );

    a.look_at(&Vector3::new(0.0, 1.0, -1.0));

    attribute_equals(a.position().unwrap(), &expected);
}

#[test]
fn center() {
    let mut geometry = position_geometry(vec![
        -1.0, -1.0, -1.0, //
        1.0, 1.0, 1.0, //
        4.0, 4.0, 4.0,
    ]);

    geometry.center();

    let p = pos(&geometry);

    // the boundingBox should go from (-1, -1, -1) to (4, 4, 4) so it has a size
    // of (5, 5, 5); after centering it the vertices should be placed between
    // (-2.5, -2.5, -2.5) and (2.5, 2.5, 2.5)
    assert!(
        p[0] == -2.5
            && p[1] == -2.5
            && p[2] == -2.5
            && p[3] == -0.5
            && p[4] == -0.5
            && p[5] == -0.5
            && p[6] == 2.5
            && p[7] == 2.5
            && p[8] == 2.5,
        "vertices were replaced by boundingBox dimensions: {p:?}"
    );
}

#[test]
fn to_non_indexed() {
    let mut geometry = position_geometry(vec![
        0.5, 0.5, 0.5, 0.5, 0.5, -0.5, 0.5, -0.5, 0.5, 0.5, -0.5, -0.5,
    ]);
    let expected = vec![
        0.5, 0.5, 0.5, 0.5, -0.5, 0.5, 0.5, 0.5, -0.5, //
        0.5, -0.5, 0.5, 0.5, -0.5, -0.5, 0.5, 0.5, -0.5,
    ];

    geometry.set_index(&[0, 2, 1, 2, 3, 1]);

    let non_indexed = geometry.to_non_indexed();

    assert_eq!(
        non_indexed.get_attribute("position").unwrap().array,
        expected,
        "Expected vertices"
    );
}

#[test]
fn to_non_indexed_carries_groups_and_morphs() {
    let mut geometry = position_geometry(vec![0.0, 0.0, 0.0, 1.0, 1.0, 1.0]);
    geometry.set_index(&[0, 1, 0]);
    geometry.set_morph_attribute(
        "position",
        vec![BufferAttribute::new(vec![1.0, 0.0, 0.0, 0.0, 1.0, 0.0], 3)],
    );
    geometry.morph_targets_relative = true;
    geometry.add_group(0, 3, 1);

    let non_indexed = geometry.to_non_indexed();

    assert_eq!(
        non_indexed.get_morph_attribute("position").unwrap()[0].array,
        vec![1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 1.0, 0.0, 0.0],
        "morph attributes are expanded too"
    );
    assert!(
        non_indexed.morph_targets_relative,
        "morphTargetsRelative is carried over"
    );
    assert_eq!(non_indexed.groups, geometry.groups, "groups are carried over");
    assert!(non_indexed.index.is_none(), "the result is not indexed");
}

/// The real generated geometry, exercising the path `geometries::to_non_indexed()`
/// used to own before it was folded into this method: a `BoxGeometry`'s index is
/// expanded and every one of its attributes comes along.
#[test]
fn to_non_indexed_expands_a_generated_geometry() {
    let indexed = three_rs::geometries::box_geometry(1.0, 1.0, 1.0, 1, 1, 1);
    let flat = indexed.to_non_indexed();

    let index_count = indexed.index.as_ref().unwrap().count();
    assert!(flat.index.is_none());
    for name in ["position", "normal", "uv"] {
        assert_eq!(
            flat.get_attribute(name).unwrap().count(),
            index_count,
            "{name} is expanded to one entry per index"
        );
    }

    // every expanded vertex is the indexed one it came from
    let src = indexed.get_attribute("position").unwrap();
    let dst = flat.get_attribute("position").unwrap();
    let index = match indexed.index.as_ref().unwrap() {
        Index::U16(v) => v.iter().map(|&i| i as usize).collect::<Vec<_>>(),
        Index::U32(v) => v.iter().map(|&i| i as usize).collect::<Vec<_>>(),
    };
    for (i, &src_i) in index.iter().enumerate() {
        assert_eq!(dst.get_x(i), src.get_x(src_i));
        assert_eq!(dst.get_y(i), src.get_y(src_i));
        assert_eq!(dst.get_z(i), src.get_z(src_i));
    }

    // the box's six groups index the old index buffer and are copied verbatim,
    // as in three.js
    assert_eq!(flat.groups, indexed.groups);

    // a geometry with no index comes back unchanged (three.js warns and returns
    // `this`)
    let again = flat.to_non_indexed();
    assert_eq!(
        again.get_attribute("position").unwrap().array,
        dst.array
    );
}

/// `toNonIndexed()` copies every named attribute, not just position/normal/uv —
/// the old `geometries::to_non_indexed()` only knew those three.
#[test]
fn to_non_indexed_copies_every_attribute() {
    let mut geometry = position_geometry(vec![0.0, 0.0, 0.0, 1.0, 1.0, 1.0]);
    geometry.set_index(&[0, 1, 0]);
    geometry.set_attribute("color", BufferAttribute::new(vec![1.0, 0.0, 0.0, 0.0, 0.0, 1.0], 3));
    geometry.set_attribute("skinIndex", BufferAttribute::new(vec![7.0, 9.0], 1));
    // three.js does not carry drawRange over, so neither does the port
    geometry.set_draw_range(1, 2);

    let non_indexed = geometry.to_non_indexed();

    assert_eq!(
        non_indexed.get_attribute("color").unwrap().array,
        vec![1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 1.0, 0.0, 0.0]
    );
    assert_eq!(
        non_indexed.get_attribute("skinIndex").unwrap().array,
        vec![7.0, 9.0, 7.0]
    );
    assert_eq!(
        non_indexed.draw_range,
        three_rs::core::DrawRange::default(),
        "toNonIndexed() leaves drawRange at its default, as three.js does"
    );
}

#[test]
fn compute_bounding_box_morph() {
    // Not a three.js test: the morph branch of `computeBoundingBox()`.
    let mut geometry = position_geometry(vec![-1.0, -1.0, -1.0, 1.0, 1.0, 1.0]);
    geometry.set_morph_attribute(
        "position",
        vec![BufferAttribute::new(
            vec![-3.0, 0.0, 0.0, 0.0, 2.0, 0.0],
            3,
        )],
    );

    // morphTargetsRelative = false: the morph box is unioned in as it stands.
    let bb = geometry.compute_bounding_box().unwrap();
    assert_eq!(bb.min, Vector3::new(-3.0, -1.0, -1.0), "absolute morph min");
    assert_eq!(bb.max, Vector3::new(1.0, 2.0, 1.0), "absolute morph max");

    // morphTargetsRelative = true: the morph box is an offset on the base box.
    geometry.morph_targets_relative = true;
    let bb = geometry.compute_bounding_box().unwrap();
    assert_eq!(bb.min, Vector3::new(-4.0, -1.0, -1.0), "relative morph min");
    assert_eq!(bb.max, Vector3::new(1.0, 3.0, 1.0), "relative morph max");
}

#[test]
fn compute_bounding_sphere_morph() {
    // Not a three.js test: the morph branch of `computeBoundingSphere()`.
    let mut geometry = position_geometry(vec![-10.0, 0.0, 0.0, 10.0, 0.0, 0.0]);
    geometry.set_morph_attribute(
        "position",
        vec![BufferAttribute::new(
            vec![0.0, -20.0, 0.0, 0.0, 20.0, 0.0],
            3,
        )],
    );

    let bs = geometry.compute_bounding_sphere().unwrap();
    assert_eq!(bs.center, Vector3::new(0.0, 0.0, 0.0), "centre is the origin");
    assert_eq!(bs.radius, 20.0, "the morph target sets the radius");
}
