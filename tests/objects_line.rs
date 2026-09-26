//! Port of `three.js/test/unit/src/objects/Line.tests.js`, plus `Line.raycast()`
//! cases checked against three.js under node.
//!
//! `copy/material` is not ported: it tests `Object3D.clone()` with a material
//! *array*, and the port has neither.

use std::rc::Rc;

use three_rs::core::{BufferGeometry, Node, Object3D, Raycaster};
use three_rs::materials::MeshBasicNodeMaterial;
use three_rs::math::Vector3;
use three_rs::objects::Line;

fn line() -> Node {
    Line::new(
        Rc::new(BufferGeometry::new()),
        MeshBasicNodeMaterial::line(Default::default()),
    )
}

#[test]
fn extending() {
    let object = line();
    object.add(&Object3D::new_node());
    assert_eq!(object.children().len(), 1, "Line extends from Object3D");
}

#[test]
fn instancing() {
    assert!(line().borrow().is_line(), "Can instantiate a Line.");
}

#[test]
fn type_name() {
    assert_eq!(
        line().borrow().object_type,
        "Line",
        "Line.type should be Line"
    );
}

#[test]
fn is_line() {
    assert!(line().borrow().is_line(), "Line.isLine should be true");
}

pub fn square() -> Rc<BufferGeometry> {
    let mut geometry = BufferGeometry::new();
    geometry.set_from_points(&[
        Vector3::new(-1.0, 0.0, 0.0),
        Vector3::new(1.0, 0.0, 0.0),
        Vector3::new(1.0, 2.0, 0.0),
        Vector3::new(-1.0, 2.0, 0.0),
    ]);
    Rc::new(geometry)
}

pub fn raycaster() -> Raycaster {
    let mut raycaster = Raycaster::new(
        Vector3::new(0.5, 0.2, 5.0),
        Vector3::new(0.0, 0.0, -1.0),
        0.0,
        f64::INFINITY,
    );
    raycaster.params.line.threshold = 0.5;
    raycaster
}

// A strip walks every consecutive pair: segments 0 and 1 are within 0.5.
#[test]
fn raycast() {
    let line = Line::new(square(), MeshBasicNodeMaterial::default());
    line.update_matrix_world(false);

    let hits = raycaster().intersect_object(&line, false);
    assert_eq!(hits.len(), 2);
    assert_eq!(hits[0].distance, 5.0);
    assert_eq!(hits[0].point, Vector3::new(0.5, 0.0, 0.0));
    assert_eq!(hits[0].index, Some(0));
    assert!(hits[0].face.is_none() && hits[0].face_index.is_none());
    assert_eq!(hits[1].distance, 5.0);
    assert!((hits[1].point.x - 1.0).abs() < 1e-12 && (hits[1].point.y - 0.2).abs() < 1e-12);
    assert_eq!(hits[1].index, Some(1));
}
