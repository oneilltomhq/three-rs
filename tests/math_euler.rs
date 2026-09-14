//! Port of `three.js/test/unit/src/math/Euler.tests.js`.
//!
//! Skipped: the property/callback tests (`x`, `y`, `z`, `order`, `set/get
//! properties, check callbacks`, `clone/copy, check callbacks`, `_onChange`,
//! `_onChangeCallback`) — the Rust port has no property setters and therefore no
//! change hook; `Object3D` re-syncs its quaternion explicitly instead. Also
//! skipped: `isEuler`, `iterable`, and the order component of `to/from_array`
//! (the Rust arrays are `[f64; 3]`, with `order` a separate typed field).

mod support;

use support::EPS;
use three_rs::math::{Euler, EulerOrder, Matrix4, Quaternion};

fn euler_zero() -> Euler {
    Euler::new_with_order(0.0, 0.0, 0.0, EulerOrder::XYZ)
}
fn euler_axyz() -> Euler {
    Euler::new_with_order(1.0, 0.0, 0.0, EulerOrder::XYZ)
}
fn euler_azyx() -> Euler {
    Euler::new_with_order(0.0, 1.0, 0.0, EulerOrder::ZYX)
}

fn matrix_equals4(a: &Matrix4, b: &Matrix4, tolerance: f64) -> bool {
    a.elements
        .iter()
        .zip(b.elements.iter())
        .all(|(x, y)| (x - y).abs() <= tolerance)
}

fn quat_equals(a: &Quaternion, b: &Quaternion, tolerance: f64) -> bool {
    let diff = (a.x - b.x).abs() + (a.y - b.y).abs() + (a.z - b.z).abs() + (a.w - b.w).abs();
    diff < tolerance
}

#[test]
fn instancing() {
    let a = Euler::default();
    assert!(a.equals(&euler_zero()));
    assert!(!a.equals(&euler_axyz()));
    assert!(!a.equals(&euler_azyx()));
}

#[test]
fn default_order() {
    assert_eq!(EulerOrder::default(), EulerOrder::XYZ);
    assert_eq!(Euler::default().order, EulerOrder::XYZ);
    assert_eq!(Euler::new(1.0, 2.0, 3.0).order, EulerOrder::XYZ);
}

#[test]
fn copy_equals() {
    let mut a = euler_axyz();
    assert!(a.equals(&euler_axyz()));
    assert!(!a.equals(&euler_zero()));
    assert!(!a.equals(&euler_azyx()));

    a.copy(&euler_azyx());
    assert!(a.equals(&euler_azyx()));
    assert!(!a.equals(&euler_axyz()));
    assert!(!a.equals(&euler_zero()));
}

#[test]
fn quaternion_set_from_euler_euler_set_from_quaternion() {
    for v in [euler_zero(), euler_axyz(), euler_azyx()] {
        let mut q = Quaternion::default();
        q.set_from_euler(&v);

        let mut v2 = Euler::default();
        v2.set_from_quaternion(&q, v.order);
        let mut q2 = Quaternion::default();
        q2.set_from_euler(&v2);

        assert!(quat_equals(&q, &q2, EPS));
    }
}

#[test]
fn matrix4_make_rotation_from_euler_euler_set_from_rotation_matrix() {
    for v in [euler_zero(), euler_axyz(), euler_azyx()] {
        let mut m = Matrix4::identity();
        m.make_rotation_from_euler(&v);

        let mut v2 = Euler::default();
        v2.set_from_rotation_matrix(&m, v.order);
        let mut m2 = Matrix4::identity();
        m2.make_rotation_from_euler(&v2);

        assert!(matrix_equals4(&m, &m2, EPS));
    }
}

#[test]
fn reorder() {
    for v in [euler_zero(), euler_axyz(), euler_azyx()] {
        let mut v = v;
        let mut q = Quaternion::default();
        q.set_from_euler(&v);

        v.reorder(EulerOrder::YZX);
        let mut q2 = Quaternion::default();
        q2.set_from_euler(&v);
        assert!(quat_equals(&q, &q2, EPS));

        v.reorder(EulerOrder::ZXY);
        let mut q3 = Quaternion::default();
        q3.set_from_euler(&v);
        assert!(quat_equals(&q, &q3, EPS));
    }
}

#[test]
fn set_set_with_order_get() {
    let mut a = Euler::default();

    a.set(1.0, 2.0, 3.0);
    assert_eq!((a.x, a.y, a.z), (1.0, 2.0, 3.0));
    assert_eq!(a.order, EulerOrder::XYZ);

    a.set_with_order(4.0, 5.0, 6.0, EulerOrder::ZYX);
    assert_eq!((a.x, a.y, a.z), (4.0, 5.0, 6.0));
    assert_eq!(a.order, EulerOrder::ZYX);
}

#[test]
fn to_array() {
    let a = Euler::new_with_order(support::X, support::Y, support::Z, EulerOrder::YXZ);
    let array = a.to_array();
    assert_eq!(array[0], support::X);
    assert_eq!(array[1], support::Y);
    assert_eq!(array[2], support::Z);
    assert_eq!(a.order, EulerOrder::YXZ);
}

#[test]
fn from_array() {
    let mut a = Euler::default();
    a.from_array(&[support::X, support::Y, support::Z]);
    assert_eq!(a.x, support::X);
    assert_eq!(a.y, support::Y);
    assert_eq!(a.z, support::Z);
    assert_eq!(a.order, EulerOrder::XYZ, "from_array leaves order alone");
}

#[test]
fn set_from_vector3() {
    use three_rs::math::Vector3;

    let mut a = Euler::new_with_order(0.0, 0.0, 0.0, EulerOrder::ZXY);
    a.set_from_vector3(&Vector3::new(support::X, support::Y, support::Z));

    assert_eq!((a.x, a.y, a.z), (support::X, support::Y, support::Z));
    assert_eq!(a.order, EulerOrder::ZXY, "order is preserved");
}
