//! Port of `three.js/test/unit/src/objects/InstancedMesh.tests.js`, plus an
//! `InstancedMesh.raycast()` case checked against three.js under node.
//!
//! `dispose` is not ported: GPU resources are released by `Drop`.

use std::rc::Rc;

use three_rs::core::{Object3D, Raycaster};
use three_rs::geometries::box_geometry;
use three_rs::materials::MeshBasicNodeMaterial;
use three_rs::math::{Matrix4, Vector3};
use three_rs::objects::InstancedMesh;

fn instanced(count: usize) -> three_rs::core::Node {
    InstancedMesh::new(
        Rc::new(box_geometry(1.0, 1.0, 1.0, 1, 1, 1)),
        MeshBasicNodeMaterial::default(),
        count,
    )
}

#[test]
fn extending() {
    let object = instanced(1);
    assert!(object.borrow().is_mesh(), "InstancedMesh extends from Mesh");
    object.add(&Object3D::new_node());
    assert_eq!(object.children().len(), 1);
}

#[test]
fn instancing() {
    assert!(
        instanced(1).borrow().is_instanced_mesh(),
        "Can instantiate a InstancedMesh."
    );
}

#[test]
fn is_instanced_mesh() {
    assert!(
        instanced(1).borrow().is_instanced_mesh(),
        "InstancedMesh.isInstancedMesh should be true"
    );
}

#[test]
fn raycast() {
    let mesh = instanced(3);
    for i in 0..3 {
        let mut matrix = Matrix4::identity();
        matrix.make_translation(2.0 * i as f64 - 2.0, 0.0, 0.0);
        mesh.borrow_mut().set_matrix_at(i, &matrix);
    }
    mesh.update_matrix_world(false);

    let raycaster = Raycaster::new(
        Vector3::new(2.0, 0.1, 5.0),
        Vector3::new(0.0, 0.0, -1.0),
        0.0,
        f64::INFINITY,
    );
    let hits = raycaster.intersect_object(&mesh, false);
    assert_eq!(hits.len(), 1);
    let hit = &hits[0];
    assert_eq!(hit.instance_id, Some(2));
    assert_eq!(hit.distance, 4.5);
    assert_eq!(hit.point, Vector3::new(2.0, 0.1, 0.5));
    assert_eq!(hit.face_index, Some(8));
    let face = hit.face.unwrap();
    assert_eq!((face.a, face.b, face.c), (16, 18, 17));
    let uv = hit.uv.unwrap();
    assert!((uv.x - 0.5).abs() < 1e-12 && (uv.y - 0.6).abs() < 1e-12);
}
