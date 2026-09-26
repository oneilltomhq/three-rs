//! Port of `three.js/test/unit/src/core/Raycaster.tests.js`.
//!
//! The two `reversed depth` cases are not ported: the port's cameras have no
//! `_reversedDepth`. The two `WebGPU coordinate system` cases run on the
//! cameras' default, which is already `WebGPUCoordinateSystem`.

use std::rc::Rc;

use three_rs::cameras::{OrthographicCamera, PerspectiveCamera};
use three_rs::core::{BufferGeometry, Node, Raycaster};
use three_rs::geometries::sphere_geometry;
use three_rs::materials::MeshBasicNodeMaterial;
use three_rs::math::{Vector2, Vector3};
use three_rs::objects::{Line, Mesh, Points};

fn check_ray_direction_against_reference_vector(ray_direction: &Vector3, ref_vector: &Vector3) {
    assert!(
        ref_vector.x - ray_direction.x <= f64::EPSILON
            && ref_vector.y - ray_direction.y <= f64::EPSILON
            && ref_vector.z - ray_direction.z <= f64::EPSILON,
        "camera is pointing to the same direction as expected"
    );
}

fn get_raycaster() -> Raycaster {
    Raycaster::new(
        Vector3::new(0.0, 0.0, 0.0),
        Vector3::new(0.0, 0.0, -1.0),
        1.0,
        100.0,
    )
}

fn get_sphere() -> Node {
    Mesh::new(Rc::new(sphere_geometry(1.0, 100, 100)), None)
}

fn get_objects_to_check() -> Vec<Node> {
    let sphere1 = get_sphere();
    sphere1.borrow_mut().position.set(0.0, 0.0, -10.0);
    sphere1.borrow_mut().name = "1".into();

    let sphere11 = get_sphere();
    sphere11.borrow_mut().position.set(0.0, 0.0, 1.0);
    sphere11.borrow_mut().name = "11".into();
    sphere1.add(&sphere11);

    let sphere12 = get_sphere();
    sphere12.borrow_mut().position.set(0.0, 0.0, -1.0);
    sphere12.borrow_mut().name = "12".into();
    sphere1.add(&sphere12);

    let sphere2 = get_sphere();
    sphere2.borrow_mut().position.set(-5.0, 0.0, -5.0);
    sphere2.borrow_mut().name = "2".into();

    let objects = vec![sphere1, sphere2];
    for object in &objects {
        object.update_matrix_world(false);
    }
    objects
}

#[test]
fn instancing() {
    // no params
    let object = Raycaster::default();
    assert_eq!(object.near, 0.0, "Can instantiate a Raycaster.");
    assert_eq!(object.far, f64::INFINITY);
    assert_eq!(object.params.line.threshold, 1.0);
    assert_eq!(object.params.points.threshold, 1.0);
    assert!(object.camera.is_none());
}

#[test]
fn set() {
    let mut origin = Vector3::new(0.0, 0.0, 0.0);
    let mut direction = Vector3::new(0.0, 0.0, -1.0);
    let mut a = Raycaster::new(origin, direction, 0.0, f64::INFINITY);

    assert_eq!(a.ray.origin, origin, "Origin is correct");
    assert_eq!(a.ray.direction, direction, "Direction is correct");

    origin.set(1.0, 1.0, 1.0);
    direction.set(-1.0, 0.0, 0.0);
    a.set(&origin, &direction);

    assert_eq!(a.ray.origin, origin, "Origin was set correctly");
    assert_eq!(a.ray.direction, direction, "Direction was set correctly");
}

#[test]
fn set_from_camera_perspective() {
    let mut raycaster = Raycaster::default();
    let camera = PerspectiveCamera::new(90.0, 1.0, 1.0, 1000.0);

    raycaster.set_from_camera(&Vector2::new(0.0, 0.0), &camera);
    let ray_direction = raycaster.ray.direction;
    assert!(
        ray_direction.x == 0.0 && ray_direction.y == 0.0 && ray_direction.z == -1.0,
        "camera is looking straight to -z and so does the ray in the middle of the screen"
    );

    let step = 0.1;
    let mut x = -1.0;
    while x <= 1.0 {
        let mut y = -1.0;
        while y <= 1.0 {
            raycaster.set_from_camera(&Vector2::new(x, y), &camera);
            let mut ref_vector = Vector3::new(x, y, -1.0);
            ref_vector.normalize();
            check_ray_direction_against_reference_vector(&raycaster.ray.direction, &ref_vector);
            y += step;
        }
        x += step;
    }
}

#[test]
fn set_from_camera_orthographic() {
    let mut raycaster = Raycaster::default();
    let camera = OrthographicCamera::new(-1.0, 1.0, 1.0, -1.0, 0.0, 1000.0);
    let expected_origin = Vector3::new(0.0, 0.0, 0.0);
    let expected_direction = Vector3::new(0.0, 0.0, -1.0);

    raycaster.set_from_camera(&Vector2::new(0.0, 0.0), &camera);
    assert_eq!(
        raycaster.ray.origin, expected_origin,
        "Ray origin has the right coordinates"
    );
    assert_eq!(
        raycaster.ray.direction, expected_direction,
        "Camera and Ray are pointing towards -z"
    );
}

#[test]
fn intersect_object() {
    let raycaster = get_raycaster();
    let objects_to_check = get_objects_to_check();

    assert_eq!(
        raycaster
            .intersect_object(&objects_to_check[0], false)
            .len(),
        1,
        "no recursive search should lead to one hit"
    );

    assert_eq!(
        raycaster.intersect_object(&objects_to_check[0], true).len(),
        3,
        "recursive search should lead to three hits"
    );

    let intersections = raycaster.intersect_object(&objects_to_check[0], true);
    for pair in intersections.windows(2) {
        assert!(
            pair[0].distance <= pair[1].distance,
            "intersections are sorted"
        );
    }
}

#[test]
fn intersect_objects() {
    let raycaster = get_raycaster();
    let objects_to_check = get_objects_to_check();

    assert_eq!(
        raycaster.intersect_objects(&objects_to_check, false).len(),
        1,
        "no recursive search should lead to one hit"
    );

    assert_eq!(
        raycaster.intersect_objects(&objects_to_check, true).len(),
        3,
        "recursive search should lead to three hits"
    );

    let intersections = raycaster.intersect_objects(&objects_to_check, true);
    for pair in intersections.windows(2) {
        assert!(
            pair[0].distance <= pair[1].distance,
            "intersections are sorted"
        );
    }
}

fn front_and_behind() -> (Node, Node) {
    let front = get_sphere();
    front.borrow_mut().position.set(0.0, 0.0, -5.0);
    front.update_matrix_world(false);

    let behind = get_sphere();
    behind.borrow_mut().position.set(0.0, 0.0, 5.0);
    behind.update_matrix_world(false);
    (front, behind)
}

#[test]
fn intersect_object_perspective_webgpu_coordinate_system() {
    let mut camera = PerspectiveCamera::new(90.0, 1.0, 0.1, 100.0);
    camera.coordinate_system = three_rs::math::CoordinateSystem::WebGPU;
    camera.update_projection_matrix();

    let (front, behind) = front_and_behind();

    let mut raycaster = Raycaster::default();
    raycaster.set_from_camera(&Vector2::new(0.0, 0.0), &camera);

    assert_eq!(
        raycaster.intersect_object(&front, true).len(),
        1,
        "Sphere in front of the perspective camera is intersected under WebGPU coordinate system."
    );
    assert_eq!(
        raycaster.intersect_object(&behind, true).len(),
        0,
        "Sphere behind the perspective camera is not intersected under WebGPU coordinate system."
    );
}

#[test]
fn intersect_object_orthographic_webgpu_coordinate_system() {
    let mut camera = OrthographicCamera::new(-1.0, 1.0, 1.0, -1.0, 0.1, 10.0);
    camera.coordinate_system = three_rs::math::CoordinateSystem::WebGPU;
    camera.update_projection_matrix();

    let (front, behind) = front_and_behind();

    let mut raycaster = Raycaster::default();
    raycaster.set_from_camera(&Vector2::new(0.0, 0.0), &camera);

    assert_eq!(
        raycaster.intersect_object(&front, true).len(),
        1,
        "Sphere in front of the orthographic camera is intersected under WebGPU coordinate system."
    );
    assert_eq!(
        raycaster.intersect_object(&behind, true).len(),
        0,
        "Sphere behind the orthographic camera is not intersected under WebGPU coordinate system."
    );
}

#[test]
fn line_intersection_threshold() {
    let mut raycaster = get_raycaster();
    let points = [
        Vector3::new(-2.0, -10.0, -5.0),
        Vector3::new(-2.0, 10.0, -5.0),
    ];
    let mut geometry = BufferGeometry::new();
    geometry.set_from_points(&points);
    // `new Line( geometry, null )` — `Line.raycast()` never reads the material.
    let line = Line::new(Rc::new(geometry), MeshBasicNodeMaterial::default());

    raycaster.params.line.threshold = 1.999;
    assert_eq!(
        raycaster.intersect_object(&line, true).len(),
        0,
        "no Line intersection with a not-large-enough threshold"
    );

    raycaster.params.line.threshold = 2.001;
    assert_eq!(
        raycaster.intersect_object(&line, true).len(),
        1,
        "successful Line intersection with a large-enough threshold"
    );
}

#[test]
fn points_intersection_threshold() {
    let mut raycaster = get_raycaster();
    let coordinates = [Vector3::new(-2.0, 0.0, -5.0)];
    let mut geometry = BufferGeometry::new();
    geometry.set_from_points(&coordinates);
    let points = Points::new(Rc::new(geometry), MeshBasicNodeMaterial::default());

    raycaster.params.points.threshold = 1.999;
    assert_eq!(
        raycaster.intersect_object(&points, true).len(),
        0,
        "no Points intersection with a not-large-enough threshold"
    );

    raycaster.params.points.threshold = 2.001;
    assert_eq!(
        raycaster.intersect_object(&points, true).len(),
        1,
        "successful Points intersection with a large-enough threshold"
    );
}

// Not in three's suite: `Raycaster.layers` gates the object's own test but not
// its children's, which is how `intersect()` reads.
#[test]
fn layers_gate_the_object_not_its_children() {
    let mut raycaster = get_raycaster();
    let objects = get_objects_to_check();
    raycaster.layers.set(1);
    assert_eq!(raycaster.intersect_object(&objects[0], true).len(), 0);

    objects[0].children()[0].borrow_mut().layers.set(1);
    assert_eq!(
        raycaster.intersect_object(&objects[0], true).len(),
        1,
        "a child on the raycaster's layer is hit under a parent that is not"
    );
}

// Not in three's suite: `webgpu_lines_fat_raycasting` parks the pointer at
// `( Infinity, Infinity )` until the mouse moves, so the ray's direction is all
// `NaN`. Every comparison against `NaN` is false, so no early-out fires and
// three.js reports a hit on *every* triangle of a mesh (19 800 for this
// sphere, checked against `src/Three.js` under node). The port must do the
// same, and must not panic on the way.
#[test]
fn a_ray_through_infinity_matches_three() {
    let mut raycaster = Raycaster::default();
    let camera = PerspectiveCamera::new(40.0, 1.0, 1.0, 1000.0);
    raycaster.set_from_camera(&Vector2::new(f64::INFINITY, f64::INFINITY), &camera);
    assert_eq!(raycaster.ray.origin, Vector3::new(0.0, 0.0, 0.0));
    assert!(raycaster.ray.direction.x.is_nan());
    let (front, _) = front_and_behind();
    assert_eq!(raycaster.intersect_object(&front, true).len(), 19800);
}
