//! Port of `three.js/test/unit/src/math/Plane.tests.js`.

mod support;

use support::{W, X, Y, Z};
use three_rs::math::{Box3, Line3, Matrix4, Plane, Sphere, Vector3};

/// `math-constants.js` `zero3`.
const fn zero3() -> Vector3 {
    Vector3::new(0.0, 0.0, 0.0)
}

/// `math-constants.js` `one3`.
const fn one3() -> Vector3 {
    Vector3::new(1.0, 1.0, 1.0)
}

/// The test file's own `comparePlane( a, b, threshold )`.
fn compare_plane(a: &Plane, b: &Plane, threshold: Option<f64>) -> bool {
    let threshold = threshold.unwrap_or(0.0001);
    a.normal.distance_to(&b.normal) < threshold && (a.constant - b.constant).abs() < threshold
}

// INSTANCING
#[test]
fn instancing() {
    let a = Plane::default();
    assert!(a.normal.x == 1.0);
    assert!(a.normal.y == 0.0);
    assert!(a.normal.z == 0.0);
    assert!(a.constant == 0.0);

    let a = Plane::new(one3(), 0.0);
    assert!(a.normal.x == 1.0);
    assert!(a.normal.y == 1.0);
    assert!(a.normal.z == 1.0);
    assert!(a.constant == 0.0);

    let a = Plane::new(one3(), 1.0);
    assert!(a.normal.x == 1.0);
    assert!(a.normal.y == 1.0);
    assert!(a.normal.z == 1.0);
    assert!(a.constant == 1.0);
}

// PUBLIC STUFF
#[test]
fn is_plane() {
    assert!(Plane::IS_PLANE);
}

#[test]
fn set() {
    let a = Plane::default();
    assert!(a.normal.x == 1.0);
    assert!(a.normal.y == 0.0);
    assert!(a.normal.z == 0.0);
    assert!(a.constant == 0.0);

    let mut b = a;
    b.set(&Vector3::new(X, Y, Z), W);
    assert!(b.normal.x == X);
    assert!(b.normal.y == Y);
    assert!(b.normal.z == Z);
    assert!(b.constant == W);
}

#[test]
fn set_components() {
    let a = Plane::default();
    assert!(a.normal.x == 1.0);
    assert!(a.normal.y == 0.0);
    assert!(a.normal.z == 0.0);
    assert!(a.constant == 0.0);

    let mut b = a;
    b.set_components(X, Y, Z, W);
    assert!(b.normal.x == X);
    assert!(b.normal.y == Y);
    assert!(b.normal.z == Z);
    assert!(b.constant == W);
}

#[test]
fn set_from_normal_and_coplanar_point() {
    let normal = one3().normalized();
    let mut a = Plane::default();
    a.set_from_normal_and_coplanar_point(&normal, &zero3());

    assert!(a.normal.equals(&normal));
    assert!(a.constant == 0.0);
}

#[test]
fn set_from_coplanar_points() {
    let mut a = Plane::default();
    let v1 = Vector3::new(2.0, 0.5, 0.25);
    let v2 = Vector3::new(2.0, -0.5, 1.25);
    let v3 = Vector3::new(2.0, -3.5, 2.2);
    let normal = Vector3::new(1.0, 0.0, 0.0);
    let constant = -2.0;

    a.set_from_coplanar_points(&v1, &v2, &v3);

    assert!(a.normal.equals(&normal), "Check normal");
    assert_eq!(a.constant, constant, "Check constant");
}

#[test]
fn clone() {
    let a = Plane::new(Vector3::new(2.0, 0.5, 0.25), 0.0);
    let b = a;

    assert!(a.equals(&b), "clones are equal");
}

#[test]
fn copy() {
    let mut a = Plane::new(Vector3::new(X, Y, Z), W);
    let mut b = Plane::default();
    b.copy(&a);
    assert!(b.normal.x == X);
    assert!(b.normal.y == Y);
    assert!(b.normal.z == Z);
    assert!(b.constant == W);

    // ensure that it is a true copy
    a.normal.x = 0.0;
    a.normal.y = -1.0;
    a.normal.z = -2.0;
    a.constant = -3.0;
    assert!(a.normal.x == 0.0 && a.normal.y == -1.0 && a.normal.z == -2.0 && a.constant == -3.0);
    assert!(b.normal.x == X);
    assert!(b.normal.y == Y);
    assert!(b.normal.z == Z);
    assert!(b.constant == W);
}

#[test]
fn normalize() {
    let mut a = Plane::new(Vector3::new(2.0, 0.0, 0.0), 2.0);

    a.normalize();
    assert!(a.normal.length() == 1.0);
    assert!(a.normal.equals(&Vector3::new(1.0, 0.0, 0.0)));
    assert!(a.constant == 1.0);
}

#[test]
fn negate_distance_to_point() {
    let mut a = Plane::new(Vector3::new(2.0, 0.0, 0.0), -2.0);

    a.normalize();
    assert!(a.distance_to_point(&Vector3::new(4.0, 0.0, 0.0)) == 3.0);
    assert!(a.distance_to_point(&Vector3::new(1.0, 0.0, 0.0)) == 0.0);

    a.negate();
    assert!(a.distance_to_point(&Vector3::new(4.0, 0.0, 0.0)) == -3.0);
    assert!(a.distance_to_point(&Vector3::new(1.0, 0.0, 0.0)) == 0.0);
}

#[test]
fn distance_to_point() {
    let mut a = Plane::new(Vector3::new(2.0, 0.0, 0.0), -2.0);

    let point = a.normalize().project_point(&zero3());
    assert!(a.distance_to_point(&point) == 0.0);
    assert!(a.distance_to_point(&Vector3::new(4.0, 0.0, 0.0)) == 3.0);
}

#[test]
fn distance_to_sphere() {
    let mut a = Plane::new(Vector3::new(1.0, 0.0, 0.0), 0.0);

    let b = Sphere::new(Vector3::new(2.0, 0.0, 0.0), 1.0);

    assert!(a.distance_to_sphere(&b) == 1.0);

    a.set(&Vector3::new(1.0, 0.0, 0.0), 2.0);
    assert!(a.distance_to_sphere(&b) == 3.0);
    a.set(&Vector3::new(1.0, 0.0, 0.0), -2.0);
    assert!(a.distance_to_sphere(&b) == -1.0);
}

#[test]
fn project_point() {
    let a = Plane::new(Vector3::new(1.0, 0.0, 0.0), 0.0);

    let point = a.project_point(&Vector3::new(10.0, 0.0, 0.0));
    assert!(point.equals(&zero3()));
    let point = a.project_point(&Vector3::new(-10.0, 0.0, 0.0));
    assert!(point.equals(&zero3()));

    let a = Plane::new(Vector3::new(0.0, 1.0, 0.0), -1.0);
    let point = a.project_point(&Vector3::new(0.0, 0.0, 0.0));
    assert!(point.equals(&Vector3::new(0.0, 1.0, 0.0)));
    let point = a.project_point(&Vector3::new(0.0, 1.0, 0.0));
    assert!(point.equals(&Vector3::new(0.0, 1.0, 0.0)));
}

#[test]
fn intersect_line() {
    let a = Plane::new(Vector3::new(1.0, 0.0, 0.0), 0.0);

    let l1 = Line3::new(Vector3::new(-10.0, 0.0, 0.0), Vector3::new(10.0, 0.0, 0.0));
    let point = a.intersect_line(&l1, None).unwrap();
    assert!(point.equals(&Vector3::new(0.0, 0.0, 0.0)));

    let a = Plane::new(Vector3::new(1.0, 0.0, 0.0), -3.0);
    let point = a.intersect_line(&l1, None).unwrap();
    assert!(point.equals(&Vector3::new(3.0, 0.0, 0.0)));

    // plane lies outside the segment's endpoints
    let a = Plane::new(Vector3::new(1.0, 0.0, 0.0), -20.0);
    let l2 = Line3::new(Vector3::new(-10.0, 0.0, 0.0), Vector3::new(10.0, 0.0, 0.0));

    assert_eq!(
        a.intersect_line(&l2, None),
        None,
        "Default clamps to segment and returns null"
    );
    assert_eq!(
        a.intersect_line(&l2, Some(true)),
        None,
        "Explicit clampToLine=true returns null"
    );

    let result = a.intersect_line(&l2, Some(false));
    assert!(
        result.is_some(),
        "clampToLine=false returns the target vector"
    );
    let point = result.unwrap();
    assert!(
        point.equals(&Vector3::new(20.0, 0.0, 0.0)),
        "clampToLine=false returns infinite-line intersection"
    );
}

#[test]
fn intersects_box() {
    let a = Box3::new(zero3(), one3());
    let b = Plane::new(Vector3::new(0.0, 1.0, 0.0), 1.0);
    let c = Plane::new(Vector3::new(0.0, 1.0, 0.0), 1.25);
    let d = Plane::new(Vector3::new(0.0, -1.0, 0.0), 1.25);
    let e = Plane::new(Vector3::new(0.0, 1.0, 0.0), 0.25);
    let f = Plane::new(Vector3::new(0.0, 1.0, 0.0), -0.25);
    let g = Plane::new(Vector3::new(0.0, 1.0, 0.0), -0.75);
    let h = Plane::new(Vector3::new(0.0, 1.0, 0.0), -1.0);
    let i = Plane::new(Vector3::new(1.0, 1.0, 1.0).normalized(), -1.732);
    let j = Plane::new(Vector3::new(1.0, 1.0, 1.0).normalized(), -1.733);

    assert!(!b.intersects_box(&a));
    assert!(!c.intersects_box(&a));
    assert!(!d.intersects_box(&a));
    assert!(!e.intersects_box(&a));
    assert!(f.intersects_box(&a));
    assert!(g.intersects_box(&a));
    assert!(h.intersects_box(&a));
    assert!(i.intersects_box(&a));
    assert!(!j.intersects_box(&a));
}

#[test]
fn intersects_sphere() {
    let a = Sphere::new(zero3(), 1.0);
    let b = Plane::new(Vector3::new(0.0, 1.0, 0.0), 1.0);
    let c = Plane::new(Vector3::new(0.0, 1.0, 0.0), 1.25);
    let d = Plane::new(Vector3::new(0.0, -1.0, 0.0), 1.25);

    assert!(b.intersects_sphere(&a));
    assert!(!c.intersects_sphere(&a));
    assert!(!d.intersects_sphere(&a));
}

#[test]
fn coplanar_point() {
    let a = Plane::new(Vector3::new(1.0, 0.0, 0.0), 0.0);
    let point = a.coplanar_point();
    assert!(a.distance_to_point(&point) == 0.0);

    let a = Plane::new(Vector3::new(0.0, 1.0, 0.0), -1.0);
    let point = a.coplanar_point();
    assert!(a.distance_to_point(&point) == 0.0);
}

#[test]
fn apply_matrix4_translate() {
    let a = Plane::new(Vector3::new(1.0, 0.0, 0.0), 0.0);

    let mut m = Matrix4::default();
    m.make_rotation_z(std::f64::consts::PI * 0.5);

    let mut a_clone = a;
    assert!(compare_plane(
        a_clone.apply_matrix4(&m, None),
        &Plane::new(Vector3::new(0.0, 1.0, 0.0), 0.0),
        None
    ));

    let a = Plane::new(Vector3::new(0.0, 1.0, 0.0), -1.0);
    let mut a_clone = a;
    assert!(compare_plane(
        a_clone.apply_matrix4(&m, None),
        &Plane::new(Vector3::new(-1.0, 0.0, 0.0), -1.0),
        None
    ));

    m.make_translation(1.0, 1.0, 1.0);
    let mut a_clone = a;
    let mut a_translated = a;
    assert!(compare_plane(
        a_clone.apply_matrix4(&m, None),
        a_translated.translate(&Vector3::new(1.0, 1.0, 1.0)),
        None
    ));
}

#[test]
fn equals() {
    let mut a = Plane::new(Vector3::new(1.0, 0.0, 0.0), 0.0);
    let b = Plane::new(Vector3::new(1.0, 0.0, 0.0), 1.0);
    let c = Plane::new(Vector3::new(0.0, 1.0, 0.0), 0.0);

    assert!(a.normal.equals(&b.normal), "Normals: equal");
    assert!(!a.normal.equals(&c.normal), "Normals: not equal");

    assert_ne!(a.constant, b.constant, "Constants: not equal");
    assert_eq!(a.constant, c.constant, "Constants: equal");

    assert!(!a.equals(&b), "Planes: not equal");
    assert!(!a.equals(&c), "Planes: not equal");

    a.copy(&b);
    assert!(a.normal.equals(&b.normal), "Normals after copy(): equal");
    assert_eq!(a.constant, b.constant, "Constants after copy(): equal");
    assert!(a.equals(&b), "Planes after copy(): equal");
}
