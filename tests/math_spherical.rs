//! Port of `three.js/test/unit/src/math/Spherical.tests.js`.

mod support;

use support::{close, EPS};
use three_rs::math::{Spherical, Vector3};

#[test]
fn instancing() {
    let a = Spherical::default();
    let radius = 10.0;
    let phi = (-0.5f64).acos();
    let theta = std::f64::consts::PI.sqrt() * phi;

    assert_eq!(a.radius, 1.0, "Default values: check radius");
    assert_eq!(a.phi, 0.0, "Default values: check phi");
    assert_eq!(a.theta, 0.0, "Default values: check theta");

    let a = Spherical::new(radius, phi, theta);
    assert_eq!(a.radius, radius, "Custom values: check radius");
    assert_eq!(a.phi, phi, "Custom values: check phi");
    assert_eq!(a.theta, theta, "Custom values: check theta");
}

// PUBLIC STUFF
#[test]
fn set() {
    let mut a = Spherical::default();
    let radius = 10.0;
    let phi = (-0.5f64).acos();
    let theta = std::f64::consts::PI.sqrt() * phi;

    a.set(radius, phi, theta);
    assert_eq!(a.radius, radius, "Check radius");
    assert_eq!(a.phi, phi, "Check phi");
    assert_eq!(a.theta, theta, "Check theta");
}

#[test]
fn clone() {
    let radius = 10.0;
    let phi = (-0.5f64).acos();
    let theta = std::f64::consts::PI.sqrt() * phi;
    let mut a = Spherical::new(radius, phi, theta);
    let b = a;

    assert_eq!(a, b, "Check a and b are equal after clone()");

    a.radius = 2.0;
    assert_ne!(a, b, "Check a and b are not equal after modification");
}

#[test]
fn copy() {
    let radius = 10.0;
    let phi = (-0.5f64).acos();
    let theta = std::f64::consts::PI.sqrt() * phi;
    let mut a = Spherical::new(radius, phi, theta);
    let mut b = Spherical::default();
    b.copy(&a);

    assert_eq!(a, b, "Check a and b are equal after copy()");

    a.radius = 2.0;
    assert_ne!(a, b, "Check a and b are not equal after modification");
}

#[test]
fn make_safe() {
    const EPS_SRC: f64 = 0.000001; // from source
    let too_low = 0.0;
    let too_high = std::f64::consts::PI;
    let just_right = 1.5;
    let mut a = Spherical::new(1.0, too_low, 0.0);

    a.make_safe();
    assert_eq!(a.phi, EPS_SRC, "Check if small values are set to EPS");

    a.set(1.0, too_high, 0.0);
    a.make_safe();
    assert_eq!(
        a.phi,
        std::f64::consts::PI - EPS_SRC,
        "Check if high values are set to (Math.PI - EPS)"
    );

    a.set(1.0, just_right, 0.0);
    a.make_safe();
    assert_eq!(
        a.phi, just_right,
        "Check that valid values don't get changed"
    );
}

#[test]
fn set_from_vector3() {
    let mut a = Spherical::new(1.0, 1.0, 1.0);
    let b = Vector3::new(0.0, 0.0, 0.0);
    let c = Vector3::new(std::f64::consts::PI, 1.0, -std::f64::consts::PI);
    let expected = Spherical::new(4.554032147688322, 1.3494066171539107, 2.356194490192345);

    a.set_from_vector3(&b);
    assert_eq!(a.radius, 0.0, "Zero-length vector: check radius");
    assert_eq!(a.phi, 0.0, "Zero-length vector: check phi");
    assert_eq!(a.theta, 0.0, "Zero-length vector: check theta");

    a.set_from_vector3(&c);
    close(
        a.radius,
        expected.radius,
        EPS,
        "Normal vector: check radius",
    );
    close(a.phi, expected.phi, EPS, "Normal vector: check phi");
    close(a.theta, expected.theta, EPS, "Normal vector: check theta");
}

#[test]
fn set_from_cartesian_coords() {
    let mut a = Spherical::new(1.0, 1.0, 1.0);
    let expected = Spherical::new(4.554032147688322, 1.3494066171539107, 2.356194490192345);

    a.set_from_cartesian_coords(0.0, 0.0, 0.0);
    assert_eq!(a.radius, 0.0, "Zero-length vector: check radius");
    assert_eq!(a.phi, 0.0, "Zero-length vector: check phi");
    assert_eq!(a.theta, 0.0, "Zero-length vector: check theta");

    a.set_from_cartesian_coords(std::f64::consts::PI, 1.0, -std::f64::consts::PI);
    close(
        a.radius,
        expected.radius,
        EPS,
        "Normal vector: check radius",
    );
    close(a.phi, expected.phi, EPS, "Normal vector: check phi");
    close(a.theta, expected.theta, EPS, "Normal vector: check theta");
}
