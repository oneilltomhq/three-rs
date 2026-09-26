//! Port of `three.js/test/unit/src/objects/LineSegments.tests.js`, plus a
//! `LineSegments.raycast()` case checked against three.js under node.

use std::rc::Rc;

use three_rs::core::{BufferGeometry, Node, Object3D, Raycaster};
use three_rs::materials::MeshBasicNodeMaterial;
use three_rs::math::Vector3;
use three_rs::objects::LineSegments;

fn segments() -> Node {
    LineSegments::new(
        Rc::new(BufferGeometry::new()),
        MeshBasicNodeMaterial::default(),
    )
}

#[test]
fn extending() {
    let object = segments();
    assert!(object.borrow().is_line(), "LineSegments extends from Line");
    object.add(&Object3D::new_node());
    assert_eq!(
        object.children().len(),
        1,
        "LineSegments extends from Object3D"
    );
}

#[test]
fn instancing() {
    assert!(
        segments().borrow().is_line_segments(),
        "Can instantiate a LineSegments."
    );
}

#[test]
fn type_name() {
    assert_eq!(
        segments().borrow().object_type,
        "LineSegments",
        "LineSegments.type should be LineSegments"
    );
}

#[test]
fn is_line_segments() {
    assert!(
        segments().borrow().is_line_segments(),
        "LineSegments.isLineSegments should be true"
    );
}

// Segments are pairs (0-1, 2-3): only the first is within 0.5 of the ray,
// where the strip over the same points also hits 1-2.
#[test]
fn raycast() {
    let mut geometry = BufferGeometry::new();
    geometry.set_from_points(&[
        Vector3::new(-1.0, 0.0, 0.0),
        Vector3::new(1.0, 0.0, 0.0),
        Vector3::new(1.0, 2.0, 0.0),
        Vector3::new(-1.0, 2.0, 0.0),
    ]);
    let line = LineSegments::new(Rc::new(geometry), MeshBasicNodeMaterial::default());
    line.update_matrix_world(false);

    let mut raycaster = Raycaster::new(
        Vector3::new(0.5, 0.2, 5.0),
        Vector3::new(0.0, 0.0, -1.0),
        0.0,
        f64::INFINITY,
    );
    raycaster.params.line.threshold = 0.5;

    let hits = raycaster.intersect_object(&line, false);
    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0].distance, 5.0);
    assert_eq!(hits[0].point, Vector3::new(0.5, 0.0, 0.0));
    assert_eq!(hits[0].index, Some(0));

    raycaster.near = 6.0;
    assert!(
        raycaster.intersect_object(&line, false).is_empty(),
        "closer than near"
    );
}
