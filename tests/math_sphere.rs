//! Port of `three.js/test/unit/src/math/Sphere.tests.js`.
//!
//! Expectations and epsilons are three.js' own; nothing here was recomputed
//! from the Rust implementation.

mod support;

use support::EPS;
use three_rs::math::{Box3, Matrix4, Plane, Sphere, Vector3};

/// `math-constants.js` `zero3`.
const ZERO3: Vector3 = Vector3::new(0.0, 0.0, 0.0);
/// `math-constants.js` `one3`.
const ONE3: Vector3 = Vector3::new(1.0, 1.0, 1.0);
/// `math-constants.js` `two3`.
const TWO3: Vector3 = Vector3::new(2.0, 2.0, 2.0);

/// `Vector3.clone().negate()`.
fn negated(v: Vector3) -> Vector3 {
    let mut v = v;
    *v.negate()
}

// INSTANCING
#[test]
fn instancing() {
    let a = Sphere::default();
    assert!(a.center.equals(&ZERO3));
    assert!(a.radius == -1.0);

    let a = Sphere::new(ONE3, 1.0);
    assert!(a.center.equals(&ONE3));
    assert!(a.radius == 1.0);
}

// PUBLIC
#[test]
fn is_sphere() {
    assert!(Sphere::IS_SPHERE);

    // `Box3` carries no `IS_SPHERE` flag at all, which is the Rust equivalent
    // of `! b.isSphere`.
}

#[test]
fn set() {
    let mut a = Sphere::default();
    assert!(a.center.equals(&ZERO3));
    assert!(a.radius == -1.0);

    a.set(&ONE3, 1.0);
    assert!(a.center.equals(&ONE3));
    assert!(a.radius == 1.0);
}

#[test]
fn set_from_points() {
    let mut a = Sphere::default();
    let expected_center = Vector3::new(0.9330126941204071, 0.0, 0.0);
    let mut expected_radius = 1.3676668773461689;
    let optional_center = Vector3::new(1.0, 1.0, 1.0);
    let points = [
        Vector3::new(1.0, 1.0, 0.0),
        Vector3::new(1.0, 1.0, 0.0),
        Vector3::new(1.0, 1.0, 0.0),
        Vector3::new(1.0, 1.0, 0.0),
        Vector3::new(1.0, 1.0, 0.0),
        Vector3::new(0.8660253882408142, 0.5, 0.0),
        Vector3::new(-0.0, 0.5, 0.8660253882408142),
        Vector3::new(1.8660253882408142, 0.5, 0.0),
        Vector3::new(0.0, 0.5, -0.8660253882408142),
        Vector3::new(0.8660253882408142, 0.5, -0.0),
        Vector3::new(0.8660253882408142, -0.5, 0.0),
        Vector3::new(-0.0, -0.5, 0.8660253882408142),
        Vector3::new(1.8660253882408142, -0.5, 0.0),
        Vector3::new(0.0, -0.5, -0.8660253882408142),
        Vector3::new(0.8660253882408142, -0.5, -0.0),
        Vector3::new(-0.0, -1.0, 0.0),
        Vector3::new(-0.0, -1.0, 0.0),
        Vector3::new(0.0, -1.0, 0.0),
        Vector3::new(0.0, -1.0, -0.0),
        Vector3::new(-0.0, -1.0, -0.0),
    ];

    a.set_from_points(&points, None);
    support::close(
        a.center.x,
        expected_center.x,
        EPS,
        "Default center: check center.x",
    );
    support::close(
        a.center.y,
        expected_center.y,
        EPS,
        "Default center: check center.y",
    );
    support::close(
        a.center.z,
        expected_center.z,
        EPS,
        "Default center: check center.z",
    );
    support::close(
        a.radius,
        expected_radius,
        EPS,
        "Default center: check radius",
    );

    expected_radius = 2.5946195770400102;
    a.set_from_points(&points, Some(&optional_center));
    support::close(
        a.center.x,
        optional_center.x,
        EPS,
        "Optional center: check center.x",
    );
    support::close(
        a.center.y,
        optional_center.y,
        EPS,
        "Optional center: check center.y",
    );
    support::close(
        a.center.z,
        optional_center.z,
        EPS,
        "Optional center: check center.z",
    );
    support::close(
        a.radius,
        expected_radius,
        EPS,
        "Optional center: check radius",
    );
}

#[test]
// three.js mutates `a` after the copy purely to prove the copy is deep.
#[allow(unused_assignments)]
fn copy() {
    let mut a = Sphere::new(ONE3, 1.0);
    let mut b = Sphere::default();
    b.copy(&a);

    assert!(b.center.equals(&ONE3));
    assert!(b.radius == 1.0);

    // ensure that it is a true copy
    a.center = ZERO3;
    a.radius = 0.0;
    assert!(b.center.equals(&ONE3));
    assert!(b.radius == 1.0);
}

#[test]
fn is_empty() {
    let mut a = Sphere::default();
    assert!(a.is_empty());

    a.set(&ONE3, 1.0);
    assert!(!a.is_empty());

    // Negative radius contains no points
    a.set(&ONE3, -1.0);
    assert!(a.is_empty());

    // Zero radius contains only the center point
    a.set(&ONE3, 0.0);
    assert!(!a.is_empty());
}

#[test]
fn make_empty() {
    let mut a = Sphere::new(ONE3, 1.0);

    assert!(!a.is_empty());

    a.make_empty();
    assert!(a.is_empty());
    assert!(a.center.equals(&ZERO3));
}

#[test]
fn contains_point() {
    let mut a = Sphere::new(ONE3, 1.0);

    assert!(!a.contains_point(&ZERO3));
    assert!(a.contains_point(&ONE3));

    a.set(&ZERO3, 0.0);
    let center = a.center;
    assert!(a.contains_point(&center));
}

#[test]
fn distance_to_point() {
    let a = Sphere::new(ONE3, 1.0);

    assert!((a.distance_to_point(&ZERO3) - 0.7320) < 0.001);
    assert!(a.distance_to_point(&ONE3) == -1.0);
}

#[test]
fn intersects_sphere() {
    let a = Sphere::new(ONE3, 1.0);
    let b = Sphere::new(ZERO3, 1.0);
    let c = Sphere::new(ZERO3, 0.25);

    assert!(a.intersects_sphere(&b));
    assert!(!a.intersects_sphere(&c));
}

#[test]
fn intersects_box() {
    let a = Sphere::new(ZERO3, 1.0);
    let b = Sphere::new(Vector3::new(-5.0, -5.0, -5.0), 1.0);
    let box_ = Box3::new(ZERO3, ONE3);

    assert!(a.intersects_box(&box_), "Check unit sphere");
    assert!(!b.intersects_box(&box_), "Check shifted sphere");
}

#[test]
fn intersects_plane() {
    let a = Sphere::new(ZERO3, 1.0);
    let b = Plane::new(Vector3::new(0.0, 1.0, 0.0), 1.0);
    let c = Plane::new(Vector3::new(0.0, 1.0, 0.0), 1.25);
    let d = Plane::new(Vector3::new(0.0, -1.0, 0.0), 1.25);

    assert!(a.intersects_plane(&b));
    assert!(!a.intersects_plane(&c));
    assert!(!a.intersects_plane(&d));
}

#[test]
fn clamp_point() {
    let a = Sphere::new(ONE3, 1.0);

    let point = a.clamp_point(&Vector3::new(1.0, 1.0, 3.0));
    assert!(point.equals(&Vector3::new(1.0, 1.0, 2.0)));
    let point = a.clamp_point(&Vector3::new(1.0, 1.0, -3.0));
    assert!(point.equals(&Vector3::new(1.0, 1.0, 0.0)));
}

#[test]
fn get_bounding_box() {
    let mut a = Sphere::new(ONE3, 1.0);

    let aabb = a.get_bounding_box();
    assert!(aabb.equals(&Box3::new(ZERO3, TWO3)));

    a.set(&ZERO3, 0.0);
    let aabb = a.get_bounding_box();
    assert!(aabb.equals(&Box3::new(ZERO3, ZERO3)));

    // Empty sphere produces empty bounding box
    a.make_empty();
    let aabb = a.get_bounding_box();
    assert!(aabb.is_empty());
}

#[test]
fn apply_matrix4() {
    let a = Sphere::new(ONE3, 1.0);
    let mut m = Matrix4::default();
    let m = *m.make_translation(1.0, -2.0, 1.0);

    let aabb1 = { a }.apply_matrix4(&m).get_bounding_box();
    let mut aabb2 = a.get_bounding_box();

    assert!(aabb1.equals(aabb2.apply_matrix4(&m)));
}

#[test]
fn translate() {
    let mut a = Sphere::new(ONE3, 1.0);

    a.translate(&negated(ONE3));
    assert!(a.center.equals(&ZERO3));
}

#[test]
fn expand_by_point() {
    let mut a = Sphere::new(ZERO3, 1.0);
    let p = Vector3::new(2.0, 0.0, 0.0);

    assert!(!a.contains_point(&p), "a does not contain p");

    a.expand_by_point(&p);

    assert!(a.contains_point(&p), "a does contain p");
    assert!(a.center.equals(&Vector3::new(0.5, 0.0, 0.0)));
    assert!(a.radius == 1.5);
}

#[test]
fn union() {
    let mut a = Sphere::new(ZERO3, 1.0);
    let b = Sphere::new(Vector3::new(2.0, 0.0, 0.0), 1.0);

    a.union(&b);

    assert!(a.center.equals(&Vector3::new(1.0, 0.0, 0.0)));
    assert!(a.radius == 2.0);

    // d contains c (demonstrates why it is necessary to process two points in union)

    let mut c = Sphere::new(Vector3::default(), 1.0);
    let d = Sphere::new(Vector3::new(1.0, 0.0, 0.0), 4.0);

    c.union(&d);

    assert!(c.center.equals(&Vector3::new(1.0, 0.0, 0.0)));
    assert!(c.radius == 4.0);

    // edge case: both spheres have the same center point

    let mut e = Sphere::new(Vector3::default(), 1.0);
    let f = Sphere::new(Vector3::default(), 4.0);

    e.union(&f);

    assert!(e.center.equals(&Vector3::new(0.0, 0.0, 0.0)));
    assert!(e.radius == 4.0);
}

#[test]
fn equals() {
    let mut a = Sphere::default();
    // `new Sphere( new Vector3( 1, 0, 0 ) )` — the radius keeps its `-1` default.
    let b = Sphere::new(Vector3::new(1.0, 0.0, 0.0), -1.0);
    let c = Sphere::new(Vector3::new(1.0, 0.0, 0.0), 1.0);

    assert!(!a.equals(&b), "a does not equal b");
    assert!(!a.equals(&c), "a does not equal c");
    assert!(!b.equals(&c), "b does not equal c");

    a.copy(&b);
    assert!(a.equals(&b), "a equals b after copy()");
}
