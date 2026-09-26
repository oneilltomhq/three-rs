//! Port of `three.js/test/unit/src/objects/Mesh.tests.js`, plus raycast cases
//! for `Mesh.getVertexPosition()` (morph targets) and the material sides.
//!
//! `copy/material` is not ported: it tests `Object3D.clone()` with a material
//! *array*, and the port has neither.
//!
//! three's `raycast` case is a `QUnit.todo`: it expects `face` to be exactly
//! `{ a, b, c }`, which is not what `Mesh.raycast()` builds, and the first hit
//! to be face 1. It is ported against what three.js actually returns for the
//! same scene (run under node against `src/`): both triangles of the plane are
//! hit at the shared diagonal point, face 0 first.

use std::rc::Rc;

use three_rs::core::{BufferAttribute, Node, Object3D, Raycaster};
use three_rs::geometries::{box_geometry, plane_geometry};
use three_rs::materials::{MeshBasicNodeMaterial, Side};
use three_rs::math::{Vector2, Vector3};
use three_rs::objects::Mesh;

fn mesh() -> Node {
    Mesh::new(Rc::new(plane_geometry(1.0, 1.0, 1, 1)), None)
}

#[test]
fn extending() {
    let object = mesh();
    let child = Object3D::new_node();
    object.add(&child);
    assert_eq!(object.children().len(), 1, "Mesh extends from Object3D");
}

#[test]
fn instancing() {
    let object = mesh();
    assert!(object.borrow().is_mesh(), "Can instantiate a Mesh.");
}

#[test]
fn type_name() {
    assert_eq!(
        mesh().borrow().object_type,
        "Mesh",
        "Mesh.type should be Mesh"
    );
}

#[test]
fn is_mesh() {
    assert!(mesh().borrow().is_mesh(), "Mesh.isMesh should be true");
}

#[test]
fn raycast() {
    let mesh = Mesh::new(
        Rc::new(plane_geometry(1.0, 1.0, 1, 1)),
        MeshBasicNodeMaterial::default(),
    );

    let mut raycaster = Raycaster::default();
    raycaster.ray.origin.set(0.25, 0.25, 1.0);
    raycaster.ray.direction.set(0.0, 0.0, -1.0);

    let mut intersections = Vec::new();
    mesh.raycast(&raycaster, &mut intersections);
    assert_eq!(intersections.len(), 2);

    let intersection = &intersections[0];
    assert!(
        Node::ptr_eq(&intersection.object, &mesh),
        "intersection object"
    );
    assert_eq!(intersection.distance, 1.0, "intersection distance");
    assert_eq!(intersection.face_index, Some(0), "intersection face index");
    let face = intersection.face.unwrap();
    assert_eq!(
        (face.a, face.b, face.c),
        (0, 2, 1),
        "intersection vertex indices"
    );
    assert_eq!(face.normal, Vector3::new(0.0, 0.0, 1.0));
    assert_eq!(face.material_index, 0);
    assert_eq!(
        intersection.point,
        Vector3::new(0.25, 0.25, 0.0),
        "intersection point"
    );
    assert_eq!(
        intersection.uv,
        Some(Vector2::new(0.75, 0.75)),
        "intersection uv"
    );
    assert_eq!(intersection.normal, Some(Vector3::new(0.0, 0.0, 1.0)));

    let second = &intersections[1];
    assert_eq!(second.face_index, Some(1));
    let face = second.face.unwrap();
    assert_eq!((face.a, face.b, face.c), (2, 3, 1));
}

#[test]
fn raycast_range() {
    let geometry = Rc::new(box_geometry(1.0, 1.0, 1.0, 1, 1, 1));
    let material = MeshBasicNodeMaterial {
        side: Side::Double,
        ..Default::default()
    };
    let mesh = Mesh::new(geometry, material);
    let mut raycaster = Raycaster::default();
    let mut intersections = Vec::new();

    raycaster.ray.origin.set(0.0, 0.0, 0.0);
    raycaster.ray.direction.set(1.0, 0.0, 0.0);
    raycaster.near = 100.0;
    raycaster.far = 200.0;

    let mut place = |x: f64, scale_y: f64| {
        mesh.borrow_mut().position.x = x;
        mesh.borrow_mut().scale.y = scale_y;
        mesh.update_matrix_world(true);
        intersections.clear();
        mesh.raycast(&raycaster, &mut intersections);
        intersections.len()
    };

    assert!(
        place(150.0, 1.0) > 0,
        "bounding sphere between near and far"
    );
    assert!(
        place(raycaster.near, 1.0) > 0,
        "bounding sphere across near"
    );
    assert!(place(raycaster.far, 1.0) > 0, "bounding sphere across far");
    assert!(
        place(150.0, 9999.0) > 0,
        "bounding sphere across near and far"
    );
    assert_eq!(place(250.0, 1.0), 0, "bounding sphere beyond far");
    assert_eq!(place(50.0, 1.0), 0, "bounding sphere before near");
}

// Not in three's suite: the side decides which winding is hit, and the
// reported normal is flipped to face the ray. Values from three.js under node.
#[test]
fn raycast_sides() {
    let mesh = Mesh::new(
        Rc::new(plane_geometry(1.0, 1.0, 1, 1)),
        MeshBasicNodeMaterial::default(),
    );
    let mut raycaster = Raycaster::default();
    raycaster.ray.origin.set(0.25, 0.25, -1.0);
    raycaster.ray.direction.set(0.0, 0.0, 1.0);

    assert!(
        raycaster.intersect_object(&mesh, false).is_empty(),
        "FrontSide from behind"
    );

    if let Some(m) = mesh.borrow_mut().payload.mesh_mut() {
        m.material.as_mut().unwrap().side = Side::Back;
    }
    let hits = raycaster.intersect_object(&mesh, false);
    assert_eq!(hits.len(), 2, "BackSide from behind");
    assert_eq!(hits[0].normal, Some(Vector3::new(0.0, 0.0, -1.0)));
    assert_eq!(hits[0].face.unwrap().normal, Vector3::new(0.0, 0.0, 1.0));
}

// Not in three's suite: `getVertexPosition()` applies the morph targets, so a
// mesh is hit where its morphed surface is. Relative and absolute targets.
#[test]
fn get_vertex_position_morphs() {
    for relative in [false, true] {
        let mut geometry = plane_geometry(1.0, 1.0, 1, 1);
        let position = geometry.position().unwrap().clone();
        let count = position.count();
        let morph: Vec<f32> = (0..count)
            .flat_map(|i| {
                let v = position.get_vector3(i);
                if relative {
                    [0.0, 0.0, -2.0]
                } else {
                    [v.x as f32, v.y as f32, v.z as f32 - 2.0]
                }
            })
            .collect();
        geometry.morph_targets_relative = relative;
        geometry.set_morph_attribute("position", vec![BufferAttribute::new(morph, 3)]);

        let node = Mesh::new(Rc::new(geometry), None);
        let raycaster = Raycaster::new(
            Vector3::new(0.25, 0.25, 5.0),
            Vector3::new(0.0, 0.0, -1.0),
            0.0,
            f64::INFINITY,
        );

        let hits = raycaster.intersect_object(&node, false);
        assert_eq!(hits[0].distance, 5.0, "influence 0 leaves the plane alone");

        node.borrow_mut()
            .payload
            .mesh_mut()
            .unwrap()
            .morph_target_influences = vec![0.5];
        let mesh_ref = node.borrow();
        let mesh = mesh_ref.payload.mesh().unwrap();
        assert_eq!(mesh.get_vertex_position(0).z, -1.0);
        drop(mesh_ref);

        let hits = raycaster.intersect_object(&node, false);
        assert_eq!(hits.len(), 2);
        assert_eq!(
            hits[0].distance, 6.0,
            "half of a -2 morph (relative: {relative})"
        );
        assert_eq!(hits[0].point, Vector3::new(0.25, 0.25, -1.0));
    }
}
