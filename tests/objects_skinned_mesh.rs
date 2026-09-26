//! Port of `three.js/test/unit/src/objects/SkinnedMesh.tests.js`, plus a
//! `SkinnedMesh.raycast()` case: the ray hits where the bones put the
//! vertices, not where the geometry has them.

use std::cell::RefCell;
use std::rc::Rc;

use three_rs::core::{BufferAttribute, Object3D, Raycaster};
use three_rs::geometries::plane_geometry;
use three_rs::math::Vector3;
use three_rs::objects::{BindMode, Bone, Skeleton, SkinnedMesh};

fn skinned() -> three_rs::core::Node {
    SkinnedMesh::new(Rc::new(plane_geometry(1.0, 1.0, 1, 1)), None)
}

#[test]
fn extending() {
    let object = skinned();
    assert!(object.borrow().is_mesh(), "SkinnedMesh extends from Mesh");
    object.add(&Object3D::new_node());
    assert_eq!(object.children().len(), 1);
}

#[test]
fn instancing() {
    assert!(
        skinned().borrow().is_skinned_mesh(),
        "Can instantiate a SkinnedMesh."
    );
}

#[test]
fn type_name() {
    assert_eq!(
        skinned().borrow().object_type,
        "SkinnedMesh",
        "SkinnedMesh.type should be SkinnedMesh"
    );
}

#[test]
fn bind_mode() {
    let object = skinned();
    let object = object.borrow();
    assert_eq!(
        object.skinned_mesh().unwrap().bind_mode,
        BindMode::Attached,
        "SkinnedMesh.bindMode should be AttachedBindMode"
    );
}

#[test]
fn is_skinned_mesh() {
    assert!(
        skinned().borrow().is_skinned_mesh(),
        "SkinnedMesh.isSkinnedMesh should be true"
    );
}

#[test]
fn raycast_follows_the_bones() {
    let mut geometry = plane_geometry(1.0, 1.0, 1, 1);
    let count = geometry.position().unwrap().count();
    geometry.set_attribute("skinIndex", BufferAttribute::new(vec![0.0; count * 4], 4));
    geometry.set_attribute(
        "skinWeight",
        BufferAttribute::new([1.0, 0.0, 0.0, 0.0].repeat(count), 4),
    );
    let mesh = SkinnedMesh::new(Rc::new(geometry), None);
    let bone = Bone::new();
    mesh.add(&bone);
    let skeleton = Rc::new(RefCell::new(Skeleton::new(vec![bone.clone()], None)));
    SkinnedMesh::bind(&mesh, skeleton, None);

    bone.borrow_mut().position.x = 3.0;
    mesh.update_matrix_world(true);

    let raycaster = Raycaster::new(
        Vector3::new(3.25, 0.25, 5.0),
        Vector3::new(0.0, 0.0, -1.0),
        0.0,
        f64::INFINITY,
    );
    let hits = raycaster.intersect_object(&mesh, false);
    assert_eq!(hits.len(), 2, "both triangles meet at the hit point");
    assert!((hits[0].distance - 5.0).abs() < 1e-12);
    assert!((hits[0].point.x - 3.25).abs() < 1e-12);

    let at_rest = Raycaster::new(
        Vector3::new(0.25, 0.25, 5.0),
        Vector3::new(0.0, 0.0, -1.0),
        0.0,
        f64::INFINITY,
    );
    assert!(
        at_rest.intersect_object(&mesh, false).is_empty(),
        "nothing is left at rest"
    );
}
