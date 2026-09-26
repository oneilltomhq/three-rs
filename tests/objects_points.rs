//! Port of `three.js/test/unit/src/objects/Points.tests.js`, plus a
//! `Points.raycast()` case checked against three.js under node.
//!
//! `copy/material` is not ported: it tests `Object3D.clone()` with a material
//! *array*, and the port has neither.

use std::rc::Rc;

use three_rs::core::{BufferGeometry, Node, Object3D, Raycaster};
use three_rs::materials::MeshBasicNodeMaterial;
use three_rs::math::Vector3;
use three_rs::objects::Points;

fn points() -> Node {
    Points::new(
        Rc::new(BufferGeometry::new()),
        MeshBasicNodeMaterial::points(),
    )
}

#[test]
fn extending() {
    let object = points();
    object.add(&Object3D::new_node());
    assert_eq!(object.children().len(), 1, "Points extends from Object3D");
}

#[test]
fn instancing() {
    assert!(points().borrow().is_points(), "Can instantiate a Points.");
}

#[test]
fn type_name() {
    assert_eq!(
        points().borrow().object_type,
        "Points",
        "Points.type should be Points"
    );
}

#[test]
fn is_points() {
    assert!(
        points().borrow().is_points(),
        "Points.isPoints should be true"
    );
}

// The threshold is divided by the mean scale: 2 at scale 2 is 1 in object
// space, which reaches point 1 but not point 0.
#[test]
fn raycast() {
    let mut geometry = BufferGeometry::new();
    geometry.set_from_points(&[
        Vector3::new(-1.0, 0.0, 0.0),
        Vector3::new(1.0, 0.0, 0.0),
        Vector3::new(1.0, 2.0, 0.0),
        Vector3::new(-1.0, 2.0, 0.0),
    ]);
    let points = Points::new(Rc::new(geometry), MeshBasicNodeMaterial::points());
    points.borrow_mut().scale.set(2.0, 2.0, 2.0);
    points.update_matrix_world(false);

    let mut raycaster = Raycaster::new(
        Vector3::new(0.5, 0.2, 5.0),
        Vector3::new(0.0, 0.0, -1.0),
        0.0,
        f64::INFINITY,
    );
    raycaster.params.points.threshold = 2.0;

    let hits = raycaster.intersect_object(&points, false);
    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0].distance, 5.0);
    assert_eq!(hits[0].point, Vector3::new(0.5, 0.2, 0.0));
    assert_eq!(hits[0].index, Some(1));
    assert!((hits[0].distance_to_ray.unwrap() - 0.7566372975210778).abs() < 1e-12);
}
