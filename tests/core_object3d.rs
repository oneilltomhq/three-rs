//! Port of `three.js/test/unit/src/core/Object3D.tests.js`.
//!
//! The Rust `Object3D` has no parent/children yet (the scene graph is flat in
//! the ladder so far), so every test that builds a hierarchy is skipped:
//! `add/remove/removeFromParent/clear`, `attach`, `getObjectById/ByName/
//! ByProperty`, `getObjectsByProperty`, `traverse*`, `updateMatrixWorld` and
//! `updateWorldMatrix` (both are parent/child matrices), and the parent halves
//! of `getWorldPosition`, `localToWorld` and `worldToLocal`. Also skipped:
//! `Extending`, `Instancing`, `type`, `isObject3D`, `DEFAULT_MATRIX_AUTO_UPDATE`
//! (no global defaults or auto-update flag), `toJSON`, `clone`, `copy`, and
//! `localTransformVariableInstantiation` (a JS-only aliasing check).

mod support;

use support::{close, EPS, X, Y, Z};
use three_rs::core::Object3D;
use three_rs::math::{Euler, Matrix4, Quaternion, Vector3};

const RAD_TO_DEG: f64 = 180.0 / std::f64::consts::PI;

fn euler_equals(a: &Euler, b: &Euler, tolerance: f64) -> bool {
    (a.x - b.x).abs() + (a.y - b.y).abs() + (a.z - b.z).abs() < tolerance
}

/// QUnit's `assert.numEqual`: tolerance 0.1.
#[track_caller]
fn num_equal(a: f64, b: f64, what: &str) {
    assert!((a - b).abs() < 0.1, "{what}: {a} vs {b}");
}

#[test]
fn default_up() {
    let object = Object3D::default();
    assert_eq!(object.up, Vector3::new(0.0, 1.0, 0.0), "Y-up");
}

#[test]
fn apply_matrix4() {
    let mut a = Object3D::default();
    let mut m = Matrix4::identity();
    let expected_pos = Vector3::new(X, Y, Z);
    let sqrt = 0.5 * 2.0_f64.sqrt();
    let expected_quat = Quaternion::new(sqrt, 0.0, 0.0, sqrt);

    m.make_rotation_x(std::f64::consts::PI / 2.0);
    m.set_position(X, Y, Z);

    a.apply_matrix4(&m);

    assert_eq!(a.position, expected_pos, "Position has the expected values");
    assert!(
        (a.quaternion.x - expected_quat.x).abs() <= EPS
            && (a.quaternion.y - expected_quat.y).abs() <= EPS
            && (a.quaternion.z - expected_quat.z).abs() <= EPS,
        "Quaternion has the expected values"
    );
}

#[test]
fn apply_quaternion() {
    let mut a = Object3D::default();
    let sqrt = 0.5 * 2.0_f64.sqrt();
    let quat = Quaternion::new(0.0, sqrt, 0.0, sqrt);
    let expected = Quaternion::new(sqrt / 2.0, sqrt / 2.0, 0.0, 0.0);

    a.quaternion.set(0.25, 0.25, 0.25, 0.25);
    a.apply_quaternion(&quat);

    assert!(
        (a.quaternion.x - expected.x).abs() <= EPS
            && (a.quaternion.y - expected.y).abs() <= EPS
            && (a.quaternion.z - expected.z).abs() <= EPS,
        "Quaternion has the expected values"
    );
}

#[test]
fn set_rotation_from_axis_angle() {
    let mut a = Object3D::default();
    let mut axis = Vector3::new(0.0, 1.0, 0.0);
    let pi = std::f64::consts::PI;
    let mut expected = Euler::new(-pi, 0.0, -pi);
    let mut euler = Euler::default();

    a.set_rotation_from_axis_angle(&axis, pi);
    let q = a.get_world_quaternion();
    euler.set_from_quaternion(&q, euler.order);
    assert!(
        euler_equals(&euler, &expected, EPS),
        "Correct values after rotation"
    );

    axis.set(1.0, 0.0, 0.0);
    expected.set(0.0, 0.0, 0.0);

    a.set_rotation_from_axis_angle(&axis, 0.0);
    let q = a.get_world_quaternion();
    euler.set_from_quaternion(&q, euler.order);
    assert!(
        euler_equals(&euler, &expected, EPS),
        "Correct values after zeroing"
    );
}

#[test]
fn set_rotation_from_euler() {
    let mut a = Object3D::default();
    let rotation = Euler::new(45.0 / RAD_TO_DEG, 0.0, std::f64::consts::PI);
    let expected = rotation;
    let mut euler = Euler::default();

    a.set_rotation_from_euler(&rotation);
    let q = a.get_world_quaternion();
    euler.set_from_quaternion(&q, euler.order);
    assert!(
        euler_equals(&euler, &expected, EPS),
        "Correct values after rotation"
    );
}

#[test]
fn set_rotation_from_matrix() {
    let mut a = Object3D::default();
    let mut m = Matrix4::identity();
    let eye = Vector3::new(0.0, 0.0, 0.0);
    let target = Vector3::new(0.0, 1.0, -1.0);
    let up = Vector3::new(0.0, 1.0, 0.0);
    let mut euler = Euler::default();

    m.look_at(&eye, &target, &up);
    a.set_rotation_from_matrix(&m);
    let q = a.get_world_quaternion();
    euler.set_from_quaternion(&q, euler.order);
    num_equal(euler.x * RAD_TO_DEG, 45.0, "Correct rotation angle");
}

#[test]
fn set_rotation_from_quaternion() {
    let mut a = Object3D::default();
    let pi = std::f64::consts::PI;
    let mut rotation = Quaternion::default();
    rotation.set_from_euler(&Euler::new(pi, 0.0, -pi));
    let mut euler = Euler::default();

    a.set_rotation_from_quaternion(&rotation);
    let q = a.get_world_quaternion();
    euler.set_from_quaternion(&q, euler.order);
    assert!(
        euler_equals(&euler, &Euler::new(pi, 0.0, -pi), EPS),
        "Correct values after rotation"
    );
}

#[test]
fn rotate_x() {
    let mut obj = Object3D::default();
    let angle = 1.562;
    obj.rotate_x(angle);
    num_equal(obj.rotation.x, angle, "x is equal");
}

#[test]
fn rotate_y() {
    let mut obj = Object3D::default();
    let angle = -0.346;
    obj.rotate_y(angle);
    num_equal(obj.rotation.y, angle, "y is equal");
}

#[test]
fn rotate_z() {
    let mut obj = Object3D::default();
    let angle = 1.0;
    obj.rotate_z(angle);
    num_equal(obj.rotation.z, angle, "z is equal");
}

#[test]
fn translate_on_axis() {
    let mut obj = Object3D::default();
    obj.translate_on_axis(&Vector3::new(1.0, 0.0, 0.0), 1.0);
    obj.translate_on_axis(&Vector3::new(0.0, 1.0, 0.0), 1.23);
    obj.translate_on_axis(&Vector3::new(0.0, 0.0, 1.0), -4.56);

    assert_eq!(obj.position, Vector3::new(1.0, 1.23, -4.56));
}

#[test]
fn translate_x_y_z() {
    let mut obj = Object3D::default();
    obj.translate_x(1.234);
    num_equal(obj.position.x, 1.234, "x is equal");

    let mut obj = Object3D::default();
    obj.translate_y(1.234);
    num_equal(obj.position.y, 1.234, "y is equal");

    let mut obj = Object3D::default();
    obj.translate_z(1.234);
    num_equal(obj.position.z, 1.234, "z is equal");
}

#[test]
fn local_to_world() {
    // three's test builds a parent/child pair; without a scene graph, the
    // single-object half of it: a translated, rotated object maps its own local
    // origin to its world position.
    let mut obj = Object3D::default();
    obj.position.set(2.0, 3.0, 4.0);
    obj.rotate_y(std::f64::consts::PI / 2.0);

    let mut v = Vector3::new(0.0, 0.0, 1.0);
    obj.local_to_world(&mut v);

    close(v.x, 3.0, EPS, "x");
    close(v.y, 3.0, EPS, "y");
    close(v.z, 4.0, EPS, "z");
}

#[test]
fn world_to_local() {
    let mut obj = Object3D::default();
    obj.position.set(2.0, 3.0, 4.0);
    obj.rotate_y(std::f64::consts::PI / 2.0);

    let mut v = Vector3::new(3.0, 3.0, 4.0);
    obj.world_to_local(&mut v);

    close(v.x, 0.0, EPS, "x");
    close(v.y, 0.0, EPS, "y");
    close(v.z, 1.0, EPS, "z");
}

#[test]
fn look_at() {
    let mut obj = Object3D::default();
    obj.look_at(&Vector3::new(0.0, -1.0, 1.0));

    num_equal(obj.rotation.x * RAD_TO_DEG, 45.0, "x is equal");
}

#[test]
fn get_world_position() {
    let mut a = Object3D::default();
    let expected = Vector3::new(X, Y, Z);

    a.translate_x(X);
    a.translate_y(Y);
    a.translate_z(Z);

    assert_eq!(
        a.get_world_position(),
        expected,
        "WorldPosition as expected for single object"
    );
}

#[test]
fn get_world_scale() {
    let mut a = Object3D::default();
    let mut m = Matrix4::identity();
    m.make_scale(X, Y, Z);
    let expected = Vector3::new(X, Y, Z);

    a.apply_matrix4(&m);

    assert_eq!(a.get_world_scale(), expected, "WorldScale as expected");
}

#[test]
fn get_world_direction() {
    let mut a = Object3D::default();
    let sqrt = 0.5 * 2.0_f64.sqrt();
    let expected = Vector3::new(0.0, -sqrt, sqrt);

    a.look_at(&Vector3::new(0.0, -1.0, 1.0));
    let direction = a.get_world_direction();

    assert!(
        (direction.x - expected.x).abs() <= EPS
            && (direction.y - expected.y).abs() <= EPS
            && (direction.z - expected.z).abs() <= EPS,
        "Direction has the expected values: {direction:?}"
    );
}

#[test]
fn update_matrix() {
    let mut a = Object3D::default();
    a.position.set(2.0, 3.0, 4.0);
    a.quaternion.set(5.0, 6.0, 7.0, 8.0);
    a.scale.set(9.0, 10.0, 11.0);

    assert_eq!(
        a.matrix.elements,
        [
            1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0
        ],
        "Updating position, quaternion or scale has no effect until update_matrix()"
    );

    a.update_matrix();

    assert_eq!(
        a.matrix.elements,
        [
            -1521.0, 1548.0, -234.0, 0.0, -520.0, -1470.0, 1640.0, 0.0, 1826.0, 44.0, -1331.0,
            0.0, 2.0, 3.0, 4.0, 1.0
        ],
        "matrix is calculated from position, quaternion and scale"
    );
}
