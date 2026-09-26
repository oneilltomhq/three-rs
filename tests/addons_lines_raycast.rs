//! `LineSegments2.raycast()` from `examples/jsm/lines/webgpu/LineSegments2.js`
//! — both branches, against three.js' own results for the same scene (run
//! under node with the addon). three.js has no unit test for it.

use three_rs::addons::lines::{LineSegments2, LineSegmentsGeometry};
use three_rs::cameras::PerspectiveCamera;
use three_rs::core::{Node, Raycaster};
use three_rs::materials::MeshBasicNodeMaterial;
use three_rs::math::{Color, Vector2, Vector3};

fn close(a: f64, b: f64) -> bool {
    (a - b).abs() < 1e-9
}

fn close3(a: &Vector3, x: f64, y: f64, z: f64) -> bool {
    close(a.x, x) && close(a.y, y) && close(a.z, z)
}

fn set_resolution(line: &Node, x: f64, y: f64) {
    let mut object = line.borrow_mut();
    let mesh = object.payload.mesh_mut().unwrap();
    mesh.line_segments.as_mut().unwrap().resolution = Vector2::new(x, y);
}

#[test]
fn raycast() {
    let mut camera = PerspectiveCamera::new(40.0, 2.0, 1.0, 1000.0);
    camera.node.borrow_mut().position.set(0.0, 0.0, 10.0);
    camera.update_matrix_world();

    let mut geometry = LineSegmentsGeometry::new();
    geometry.set_positions(vec![
        -2.0, 0.0, 0.0, 2.0, 0.0, 0.0, -2.0, 1.0, 0.0, 2.0, 1.5, 0.0,
    ]);
    let mut material = MeshBasicNodeMaterial::line2(Color::from_hex(0xffffff));
    material.linewidth = 5.0;
    let line = LineSegments2::new(&geometry, material);
    line.update_matrix_world(false);

    let mut raycaster = Raycaster::default();
    raycaster.set_from_camera(&Vector2::new(0.1, 0.005), &camera);

    // Screen space: nothing until a render has set `_resolution`.
    assert!(raycaster.intersect_object(&line, false).is_empty());

    set_resolution(&line, 800.0, 400.0);
    let hits = raycaster.intersect_object(&line, false);
    assert_eq!(hits.len(), 1);
    let hit = &hits[0];
    assert!(close(hit.distance, 10.026443169495511));
    assert!(close3(
        &hit.point,
        0.727938057704717,
        0.018198451442617922,
        0.000033118473172777385
    ));
    assert!(close3(
        &hit.point_on_line.unwrap(),
        0.7279380577047171,
        0.0,
        0.0
    ));
    assert_eq!(hit.face_index, Some(0));
    assert!(hit.face.is_none() && hit.uv.is_none());

    raycaster.set_from_camera(&Vector2::new(0.1, 0.05), &camera);
    assert!(
        raycaster.intersect_object(&line, false).is_empty(),
        "5 px is not wide enough"
    );

    // World units: the width is 0.5 world units, and `params.line2` widens it.
    {
        let mut object = line.borrow_mut();
        let material = object
            .payload
            .mesh_mut()
            .unwrap()
            .material
            .as_mut()
            .unwrap();
        material.world_units = true;
        material.linewidth = 0.5;
    }
    raycaster.set_from_camera(&Vector2::new(0.1, 0.005), &camera);
    let hits = raycaster.intersect_object(&line, false);
    assert_eq!(hits.len(), 1);
    assert!(close(hits[0].distance, 10.026443169495511));

    raycaster.params.line2.threshold = 3.0;
    let hits = raycaster.intersect_object(&line, false);
    assert_eq!(hits.len(), 2);
    assert_eq!(hits[0].face_index, Some(1));
    assert!(close(hits[0].distance, 10.016970756956672));
    assert!(close3(
        &hits[0].point,
        0.7272503432810937,
        0.018181258582027342,
        0.009480517722861848
    ));
    assert!(close3(
        &hits[0].point_on_line.unwrap(),
        0.5644534159791726,
        1.3205566769973967,
        0.0
    ));
    assert_eq!(hits[1].face_index, Some(0));
}
