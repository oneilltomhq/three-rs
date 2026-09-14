//! Port of `three.js/test/unit/src/math/Line3.tests.js`.
//!
//! Expectations and epsilons are three.js' own; nothing here was recomputed
//! from the Rust implementation.

mod support;

use support::{close, X, Y, Z};
use three_rs::math::{Line3, Matrix4, Vector3, Vector4};

/// `math-constants.js` `zero3`.
const ZERO3: Vector3 = Vector3::new(0.0, 0.0, 0.0);
/// `math-constants.js` `one3`.
const ONE3: Vector3 = Vector3::new(1.0, 1.0, 1.0);
/// `math-constants.js` `two3`.
const TWO3: Vector3 = Vector3::new(2.0, 2.0, 2.0);

/// `qunit-utils.js` `assert.numEqual` compares with `diff < 0.1`.
const NUM_EQ: f64 = 0.1;

// INSTANCING
#[test]
fn instancing() {
    let a = Line3::default();
    assert!(a.start.equals(&ZERO3));
    assert!(a.end.equals(&ZERO3));

    let a = Line3::new(TWO3, ONE3);
    assert!(a.start.equals(&TWO3));
    assert!(a.end.equals(&ONE3));
}

// PUBLIC STUFF
#[test]
fn set() {
    let mut a = Line3::default();

    a.set(&ONE3, &ONE3);
    assert!(a.start.equals(&ONE3));
    assert!(a.end.equals(&ONE3));
}

#[test]
// three.js mutates `a` after the copy purely to prove `b` is independent.
#[allow(unused_assignments)]
fn copy_equals() {
    let mut a = Line3::new(ZERO3, ONE3);
    let mut b = Line3::default();
    b.copy(&a);
    assert!(b.start.equals(&ZERO3));
    assert!(b.end.equals(&ONE3));

    // ensure that it is a true copy
    a.start = ZERO3;
    a.end = ONE3;
    assert!(b.start.equals(&ZERO3));
    assert!(b.end.equals(&ONE3));
}

#[test]
fn clone_equal() {
    let a = Line3::default();
    let b = Line3::new(ZERO3, Vector3::new(1.0, 1.0, 1.0));
    let c = Line3::new(ZERO3, Vector3::new(1.0, 1.0, 0.0));

    assert!(!a.equals(&b), "Check a and b aren't equal");
    assert!(!a.equals(&c), "Check a and c aren't equal");
    assert!(!b.equals(&c), "Check b and c aren't equal");

    let mut a = b;
    assert!(a.equals(&b), "Check a and b are equal after clone()");
    assert!(!a.equals(&c), "Check a and c aren't equal after clone()");

    a.set(&ZERO3, &ZERO3);
    assert!(
        !a.equals(&b),
        "Check a and b are not equal after modification"
    );
}

#[test]
fn get_center() {
    let a = Line3::new(ZERO3, TWO3);
    assert!(a.get_center().equals(&ONE3));
}

#[test]
fn delta() {
    let a = Line3::new(ZERO3, TWO3);
    assert!(a.delta().equals(&TWO3));
}

#[test]
fn distance_sq() {
    let a = Line3::new(ZERO3, ZERO3);
    let b = Line3::new(ZERO3, ONE3);
    let c = Line3::new(*ONE3.clone().negate(), ONE3);
    let d = Line3::new(*TWO3.clone().multiply_scalar(-2.0), *TWO3.clone().negate());

    close(
        a.distance_sq(),
        0.0,
        NUM_EQ,
        "Check squared distance for zero-length line",
    );
    close(
        b.distance_sq(),
        3.0,
        NUM_EQ,
        "Check squared distance for simple line",
    );
    close(
        c.distance_sq(),
        12.0,
        NUM_EQ,
        "Check squared distance for negative to positive endpoints",
    );
    close(
        d.distance_sq(),
        12.0,
        NUM_EQ,
        "Check squared distance for negative to negative endpoints",
    );
}

#[test]
fn distance() {
    let a = Line3::new(ZERO3, ZERO3);
    let b = Line3::new(ZERO3, ONE3);
    let c = Line3::new(*ONE3.clone().negate(), ONE3);
    let d = Line3::new(*TWO3.clone().multiply_scalar(-2.0), *TWO3.clone().negate());

    close(
        a.distance(),
        0.0,
        NUM_EQ,
        "Check distance for zero-length line",
    );
    close(
        b.distance(),
        3.0_f64.sqrt(),
        NUM_EQ,
        "Check distance for simple line",
    );
    close(
        c.distance(),
        12.0_f64.sqrt(),
        NUM_EQ,
        "Check distance for negative to positive endpoints",
    );
    close(
        d.distance(),
        12.0_f64.sqrt(),
        NUM_EQ,
        "Check distance for negative to negative endpoints",
    );
}

#[test]
fn at() {
    let a = Line3::new(ONE3, Vector3::new(1.0, 1.0, 2.0));

    let point = a.at(-1.0);
    assert!(point.distance_to(&Vector3::new(1.0, 1.0, 0.0)) < 0.0001);
    let point = a.at(0.0);
    assert!(point.distance_to(&ONE3) < 0.0001);
    let point = a.at(1.0);
    assert!(point.distance_to(&Vector3::new(1.0, 1.0, 2.0)) < 0.0001);
    let point = a.at(2.0);
    assert!(point.distance_to(&Vector3::new(1.0, 1.0, 3.0)) < 0.0001);
}

#[test]
fn closest_point_to_point_closest_point_to_point_parameter() {
    let a = Line3::new(ONE3, Vector3::new(1.0, 1.0, 2.0));

    // nearby the ray
    assert!(a.closest_point_to_point_parameter(&ZERO3, true) == 0.0);
    let point = a.closest_point_to_point(&ZERO3, true);
    assert!(point.distance_to(&Vector3::new(1.0, 1.0, 1.0)) < 0.0001);

    // nearby the ray
    assert!(a.closest_point_to_point_parameter(&ZERO3, false) == -1.0);
    let point = a.closest_point_to_point(&ZERO3, false);
    assert!(point.distance_to(&Vector3::new(1.0, 1.0, 0.0)) < 0.0001);

    // nearby the ray
    assert!(a.closest_point_to_point_parameter(&Vector3::new(1.0, 1.0, 5.0), true) == 1.0);
    let point = a.closest_point_to_point(&Vector3::new(1.0, 1.0, 5.0), true);
    assert!(point.distance_to(&Vector3::new(1.0, 1.0, 2.0)) < 0.0001);

    // exactly on the ray
    assert!(a.closest_point_to_point_parameter(&ONE3, true) == 0.0);
    let point = a.closest_point_to_point(&ONE3, true);
    assert!(point.distance_to(&ONE3) < 0.0001);

    // degenerate line (zero-length)
    let b = Line3::new(ONE3, ONE3);
    assert!(b.closest_point_to_point_parameter(&ZERO3, true) == 0.0);
    let point = b.closest_point_to_point(&ZERO3, true);
    assert!(point.distance_to(&ONE3) < 0.0001);
}

#[test]
fn apply_matrix4() {
    let mut a = Line3::new(ZERO3, TWO3);
    let mut b = Vector4::new(TWO3.x, TWO3.y, TWO3.z, 1.0);
    let mut m = Matrix4::default();
    m.make_translation(X, Y, Z);
    let v = Vector3::new(X, Y, Z);

    a.apply_matrix4(&m);
    assert!(a.start.equals(&v), "Translation: check start");
    assert!(
        a.end.equals(&Vector3::new(2.0 + X, 2.0 + Y, 2.0 + Z)),
        "Translation: check start"
    );

    // reset starting conditions
    a.set(&ZERO3, &TWO3);
    m.make_rotation_x(std::f64::consts::PI);

    a.apply_matrix4(&m);
    b.apply_matrix4(&m);

    assert!(a.start.equals(&ZERO3), "Rotation: check start");
    close(a.end.x, b.x / b.w, NUM_EQ, "Rotation: check end.x");
    close(a.end.y, b.y / b.w, NUM_EQ, "Rotation: check end.y");
    close(a.end.z, b.z / b.w, NUM_EQ, "Rotation: check end.z");

    // reset starting conditions
    a.set(&ZERO3, &TWO3);
    b.set(TWO3.x, TWO3.y, TWO3.z, 1.0);
    m.set_position(v.x, v.y, v.z);

    a.apply_matrix4(&m);
    b.apply_matrix4(&m);

    assert!(a.start.equals(&v), "Both: check start");
    close(a.end.x, b.x / b.w, NUM_EQ, "Both: check end.x");
    close(a.end.y, b.y / b.w, NUM_EQ, "Both: check end.y");
    close(a.end.z, b.z / b.w, NUM_EQ, "Both: check end.z");
}

#[test]
fn equals() {
    let a = Line3::new(ZERO3, ZERO3);
    let b = Line3::default();
    assert!(a.equals(&b));
}

#[test]
fn distance_sq_to_line3() {
    let mut line1 = Line3::default();
    line1.start.set(0.0, 0.0, 0.0);
    line1.end.set(2.0, 0.0, 0.0);

    let mut line2 = Line3::default();
    line2.start.set(1.0, 10.0, 0.0);
    line2.end.set(1.0, -2.0, 0.0);

    close(
        line1.distance_sq_to_line3(&line2, None, None),
        0.0,
        NUM_EQ,
        "distanceSqToLine3",
    );

    // Parallel lines case
    line2.start.set(-2.0, 0.0, 2.0);
    line2.end.set(20.0, 0.0, 2.0);

    close(
        line1.distance_sq_to_line3(&line2, None, None),
        4.0,
        NUM_EQ,
        "distanceSqToLine3",
    );

    // Closest point on lines from one side is out of segment
    line1.start.set(0.0, 4.0, 0.0);
    line1.end.set(2.0, 2.0, 0.0);

    line2.start.set(0.0, 0.0, 0.0);
    line2.end.set(4.0, 0.0, 0.0);

    close(
        line1.distance_sq_to_line3(&line2, None, None),
        4.0,
        NUM_EQ,
        "distanceSqToLine3",
    );

    // Closest point on lines from another side is out of segment
    line1.start.set(0.0, 4.0, 0.0);
    line1.end.set(3.0, 1.0, 0.0);

    line2.start.set(0.0, 0.0, 0.0);
    line2.end.set(1.0, 0.0, 0.0);

    close(
        line1.distance_sq_to_line3(&line2, None, None),
        4.5,
        NUM_EQ,
        "distanceSqToLine3",
    );

    // Closest point on lines from both sides is out of the segment
    line1.start.set(0.0, 4.0, 0.0);
    line1.end.set(2.0, 2.0, 0.0);

    line2.start.set(0.0, 0.0, 0.0);
    line2.end.set(1.0, 0.0, 0.0);

    close(
        line1.distance_sq_to_line3(&line2, None, None),
        5.0,
        NUM_EQ,
        "distanceSqToLine3",
    );

    // General case with skew lines
    line1.start.set(4.0, 0.0, 0.0);
    line1.end.set(-4.0, 0.0, 0.0);

    line2.start.set(0.0, 4.0, 0.0);
    line2.end.set(0.0, 0.0, 4.0);

    close(
        line1.distance_sq_to_line3(&line2, None, None),
        8.0,
        NUM_EQ,
        "distanceSqToLine3",
    );
}
