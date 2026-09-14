//! Port of `three.js/test/unit/src/math/Cylindrical.tests.js`.

mod support;

use support::{close, EPS};
use three_rs::math::{Cylindrical, Vector3};

#[test]
fn instancing() {
    let a = Cylindrical::default();
    let radius = 10.0;
    let theta = std::f64::consts::PI;
    let y = 5.0;

    assert_eq!(a.radius, 1.0, "Default values: check radius");
    assert_eq!(a.theta, 0.0, "Default values: check theta");
    assert_eq!(a.y, 0.0, "Default values: check y");

    let a = Cylindrical::new(radius, theta, y);
    assert_eq!(a.radius, radius, "Custom values: check radius");
    assert_eq!(a.theta, theta, "Custom values: check theta");
    assert_eq!(a.y, y, "Custom values: check y");
}

// PUBLIC
#[test]
fn set() {
    let mut a = Cylindrical::default();
    let radius = 10.0;
    let theta = std::f64::consts::PI;
    let y = 5.0;

    a.set(radius, theta, y);
    assert_eq!(a.radius, radius, "Check radius");
    assert_eq!(a.theta, theta, "Check theta");
    assert_eq!(a.y, y, "Check y");
}

#[test]
fn clone() {
    let radius = 10.0;
    let theta = std::f64::consts::PI;
    let y = 5.0;
    let mut a = Cylindrical::new(radius, theta, y);
    let b = a;

    assert_eq!(a, b, "Check a and b are equal after clone()");

    a.radius = 1.0;
    assert_ne!(a, b, "Check a and b are not equal after modification");
}

#[test]
fn copy() {
    let radius = 10.0;
    let theta = std::f64::consts::PI;
    let y = 5.0;
    let mut a = Cylindrical::new(radius, theta, y);
    let mut b = Cylindrical::default();
    b.copy(&a);

    assert_eq!(a, b, "Check a and b are equal after copy()");

    a.radius = 1.0;
    assert_ne!(a, b, "Check a and b are not equal after modification");
}

#[test]
fn set_from_vector3() {
    let mut a = Cylindrical::new(1.0, 1.0, 1.0);
    let b = Vector3::new(0.0, 0.0, 0.0);
    let c = Vector3::new(3.0, -1.0, -3.0);
    let expected = Cylindrical::new(18f64.sqrt(), 3f64.atan2(-3.0), -1.0);

    a.set_from_vector3(&b);
    assert_eq!(a.radius, 0.0, "Zero-length vector: check radius");
    assert_eq!(a.theta, 0.0, "Zero-length vector: check theta");
    assert_eq!(a.y, 0.0, "Zero-length vector: check y");

    a.set_from_vector3(&c);
    close(
        a.radius,
        expected.radius,
        EPS,
        "Normal vector: check radius",
    );
    close(a.theta, expected.theta, EPS, "Normal vector: check theta");
    close(a.y, expected.y, EPS, "Normal vector: check y");
}
