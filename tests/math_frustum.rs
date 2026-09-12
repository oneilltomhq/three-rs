//! Port of `three.js/test/unit/src/math/Frustum.tests.js`.
//!
//! Expectations and epsilons are three.js' own; nothing here was recomputed
//! from the Rust implementation.

mod support;

use support::EPS;
use three_rs::math::{Box3, CoordinateSystem, Frustum, Matrix4, Plane, Sphere, Vector3};

/// `const unit3 = new Vector3( 1, 0, 0 );`
fn unit3() -> Vector3 {
    Vector3::new(1.0, 0.0, 0.0)
}

/// `zero3` from `math-constants.js`.
fn zero3() -> Vector3 {
    Vector3::new(0.0, 0.0, 0.0)
}

/// `one3` from `math-constants.js`.
fn one3() -> Vector3 {
    Vector3::new(1.0, 1.0, 1.0)
}

/// `new Matrix4().makeOrthographic( -1, 1, 1, -1, 1, 100 )` /
/// `makePerspective(...)` — three.js' own defaults, i.e. the WebGL
/// coordinate system.
fn ortho() -> Matrix4 {
    let mut m = Matrix4::default();
    m.make_orthographic(-1.0, 1.0, 1.0, -1.0, 1.0, 100.0, CoordinateSystem::WebGL);
    m
}

fn perspective() -> Matrix4 {
    let mut m = Matrix4::default();
    m.make_perspective(-1.0, 1.0, 1.0, -1.0, 1.0, 100.0, CoordinateSystem::WebGL);
    m
}

/// `new Frustum().setFromProjectionMatrix( m )` — three.js' defaults, i.e. the
/// WebGL coordinate system and no reversed depth.
fn from_projection_matrix(m: &Matrix4) -> Frustum {
    let mut a = Frustum::default();
    a.set_from_projection_matrix(m, CoordinateSystem::WebGL, false);
    a
}

// INSTANCING
#[test]
fn instancing() {
    let a = Frustum::default();

    assert_eq!(a.planes.len(), 6);

    let p_default = Plane::default();
    for i in 0..6 {
        assert!(a.planes[i].equals(&p_default), "Passed!");
    }

    let p0 = Plane::new(unit3(), -1.0);
    let p1 = Plane::new(unit3(), 1.0);
    let p2 = Plane::new(unit3(), 2.0);
    let p3 = Plane::new(unit3(), 3.0);
    let p4 = Plane::new(unit3(), 4.0);
    let p5 = Plane::new(unit3(), 5.0);

    let a = Frustum::new(p0, p1, p2, p3, p4, p5);
    assert!(a.planes[0].equals(&p0), "Passed!");
    assert!(a.planes[1].equals(&p1), "Passed!");
    assert!(a.planes[2].equals(&p2), "Passed!");
    assert!(a.planes[3].equals(&p3), "Passed!");
    assert!(a.planes[4].equals(&p4), "Passed!");
    assert!(a.planes[5].equals(&p5), "Passed!");
}

// PUBLIC
#[test]
fn set() {
    let mut a = Frustum::default();
    let p0 = Plane::new(unit3(), -1.0);
    let p1 = Plane::new(unit3(), 1.0);
    let p2 = Plane::new(unit3(), 2.0);
    let p3 = Plane::new(unit3(), 3.0);
    let p4 = Plane::new(unit3(), 4.0);
    let p5 = Plane::new(unit3(), 5.0);

    a.set(&p0, &p1, &p2, &p3, &p4, &p5);

    assert!(a.planes[0].equals(&p0), "Check plane #0");
    assert!(a.planes[1].equals(&p1), "Check plane #1");
    assert!(a.planes[2].equals(&p2), "Check plane #2");
    assert!(a.planes[3].equals(&p3), "Check plane #3");
    assert!(a.planes[4].equals(&p4), "Check plane #4");
    assert!(a.planes[5].equals(&p5), "Check plane #5");
}

#[test]
fn clone() {
    let p0 = Plane::new(unit3(), -1.0);
    let p1 = Plane::new(unit3(), 1.0);
    let p2 = Plane::new(unit3(), 2.0);
    let p3 = Plane::new(unit3(), 3.0);
    let p4 = Plane::new(unit3(), 4.0);
    let p5 = Plane::new(unit3(), 5.0);

    let b = Frustum::new(p0, p1, p2, p3, p4, p5);
    let mut a = b;
    assert!(a.planes[0].equals(&p0), "Passed!");
    assert!(a.planes[1].equals(&p1), "Passed!");
    assert!(a.planes[2].equals(&p2), "Passed!");
    assert!(a.planes[3].equals(&p3), "Passed!");
    assert!(a.planes[4].equals(&p4), "Passed!");
    assert!(a.planes[5].equals(&p5), "Passed!");

    // ensure it is a true copy by modifying source
    a.planes[0].copy(&p1);
    assert!(b.planes[0].equals(&p0), "Passed!");
}

#[test]
// Three's `b.planes[ 0 ] = p1` is only there to prove `a` did not alias it.
#[allow(unused_assignments)]
fn copy() {
    let p0 = Plane::new(unit3(), -1.0);
    let p1 = Plane::new(unit3(), 1.0);
    let p2 = Plane::new(unit3(), 2.0);
    let p3 = Plane::new(unit3(), 3.0);
    let p4 = Plane::new(unit3(), 4.0);
    let p5 = Plane::new(unit3(), 5.0);

    let mut b = Frustum::new(p0, p1, p2, p3, p4, p5);
    let mut a = Frustum::default();
    a.copy(&b);
    assert!(a.planes[0].equals(&p0), "Passed!");
    assert!(a.planes[1].equals(&p1), "Passed!");
    assert!(a.planes[2].equals(&p2), "Passed!");
    assert!(a.planes[3].equals(&p3), "Passed!");
    assert!(a.planes[4].equals(&p4), "Passed!");
    assert!(a.planes[5].equals(&p5), "Passed!");

    // ensure it is a true copy by modifying source
    b.planes[0] = p1;
    assert!(a.planes[0].equals(&p0), "Passed!");
}

#[test]
fn set_from_projection_matrix_make_orthographic_contains_point() {
    let m = ortho();
    let a = from_projection_matrix(&m);

    assert!(!a.contains_point(&Vector3::new(0.0, 0.0, 0.0)), "Passed!");
    assert!(a.contains_point(&Vector3::new(0.0, 0.0, -50.0)), "Passed!");
    assert!(a.contains_point(&Vector3::new(0.0, 0.0, -1.001)), "Passed!");
    assert!(
        a.contains_point(&Vector3::new(-1.0, -1.0, -1.001)),
        "Passed!"
    );
    assert!(
        !a.contains_point(&Vector3::new(-1.1, -1.1, -1.001)),
        "Passed!"
    );
    assert!(a.contains_point(&Vector3::new(1.0, 1.0, -1.001)), "Passed!");
    assert!(
        !a.contains_point(&Vector3::new(1.1, 1.1, -1.001)),
        "Passed!"
    );
    assert!(a.contains_point(&Vector3::new(0.0, 0.0, -99.999)), "Passed!");
    assert!(
        a.contains_point(&Vector3::new(-0.999, -0.999, -99.999)),
        "Passed!"
    );
    assert!(
        !a.contains_point(&Vector3::new(-1.1, -1.1, -100.1)),
        "Passed!"
    );
    assert!(
        a.contains_point(&Vector3::new(0.999, 0.999, -99.999)),
        "Passed!"
    );
    assert!(
        !a.contains_point(&Vector3::new(1.1, 1.1, -100.1)),
        "Passed!"
    );
    assert!(!a.contains_point(&Vector3::new(0.0, 0.0, -101.0)), "Passed!");
}

#[test]
fn set_from_projection_matrix_make_perspective_contains_point() {
    let m = perspective();
    let a = from_projection_matrix(&m);

    assert!(!a.contains_point(&Vector3::new(0.0, 0.0, 0.0)), "Passed!");
    assert!(a.contains_point(&Vector3::new(0.0, 0.0, -50.0)), "Passed!");
    assert!(a.contains_point(&Vector3::new(0.0, 0.0, -1.001)), "Passed!");
    assert!(
        a.contains_point(&Vector3::new(-1.0, -1.0, -1.001)),
        "Passed!"
    );
    assert!(
        !a.contains_point(&Vector3::new(-1.1, -1.1, -1.001)),
        "Passed!"
    );
    assert!(a.contains_point(&Vector3::new(1.0, 1.0, -1.001)), "Passed!");
    assert!(
        !a.contains_point(&Vector3::new(1.1, 1.1, -1.001)),
        "Passed!"
    );
    assert!(a.contains_point(&Vector3::new(0.0, 0.0, -99.999)), "Passed!");
    assert!(
        a.contains_point(&Vector3::new(-99.999, -99.999, -99.999)),
        "Passed!"
    );
    assert!(
        !a.contains_point(&Vector3::new(-100.1, -100.1, -100.1)),
        "Passed!"
    );
    assert!(
        a.contains_point(&Vector3::new(99.999, 99.999, -99.999)),
        "Passed!"
    );
    assert!(
        !a.contains_point(&Vector3::new(100.1, 100.1, -100.1)),
        "Passed!"
    );
    assert!(!a.contains_point(&Vector3::new(0.0, 0.0, -101.0)), "Passed!");
}

#[test]
fn set_from_projection_matrix_make_perspective_intersects_sphere() {
    let m = perspective();
    let a = from_projection_matrix(&m);

    assert!(
        !a.intersects_sphere(&Sphere::new(Vector3::new(0.0, 0.0, 0.0), 0.0)),
        "Passed!"
    );
    assert!(
        !a.intersects_sphere(&Sphere::new(Vector3::new(0.0, 0.0, 0.0), 0.9)),
        "Passed!"
    );
    assert!(
        a.intersects_sphere(&Sphere::new(Vector3::new(0.0, 0.0, 0.0), 1.1)),
        "Passed!"
    );
    assert!(
        a.intersects_sphere(&Sphere::new(Vector3::new(0.0, 0.0, -50.0), 0.0)),
        "Passed!"
    );
    assert!(
        a.intersects_sphere(&Sphere::new(Vector3::new(0.0, 0.0, -1.001), 0.0)),
        "Passed!"
    );
    assert!(
        a.intersects_sphere(&Sphere::new(Vector3::new(-1.0, -1.0, -1.001), 0.0)),
        "Passed!"
    );
    assert!(
        !a.intersects_sphere(&Sphere::new(Vector3::new(-1.1, -1.1, -1.001), 0.0)),
        "Passed!"
    );
    assert!(
        a.intersects_sphere(&Sphere::new(Vector3::new(-1.1, -1.1, -1.001), 0.5)),
        "Passed!"
    );
    assert!(
        a.intersects_sphere(&Sphere::new(Vector3::new(1.0, 1.0, -1.001), 0.0)),
        "Passed!"
    );
    assert!(
        !a.intersects_sphere(&Sphere::new(Vector3::new(1.1, 1.1, -1.001), 0.0)),
        "Passed!"
    );
    assert!(
        a.intersects_sphere(&Sphere::new(Vector3::new(1.1, 1.1, -1.001), 0.5)),
        "Passed!"
    );
    assert!(
        a.intersects_sphere(&Sphere::new(Vector3::new(0.0, 0.0, -99.999), 0.0)),
        "Passed!"
    );
    assert!(
        a.intersects_sphere(&Sphere::new(Vector3::new(-99.999, -99.999, -99.999), 0.0)),
        "Passed!"
    );
    assert!(
        !a.intersects_sphere(&Sphere::new(Vector3::new(-100.1, -100.1, -100.1), 0.0)),
        "Passed!"
    );
    assert!(
        a.intersects_sphere(&Sphere::new(Vector3::new(-100.1, -100.1, -100.1), 0.5)),
        "Passed!"
    );
    assert!(
        a.intersects_sphere(&Sphere::new(Vector3::new(99.999, 99.999, -99.999), 0.0)),
        "Passed!"
    );
    assert!(
        !a.intersects_sphere(&Sphere::new(Vector3::new(100.1, 100.1, -100.1), 0.0)),
        "Passed!"
    );
    assert!(
        a.intersects_sphere(&Sphere::new(Vector3::new(100.1, 100.1, -100.1), 0.2)),
        "Passed!"
    );
    assert!(
        !a.intersects_sphere(&Sphere::new(Vector3::new(0.0, 0.0, -101.0), 0.0)),
        "Passed!"
    );
    assert!(
        a.intersects_sphere(&Sphere::new(Vector3::new(0.0, 0.0, -101.0), 1.1)),
        "Passed!"
    );
}

#[test]
fn intersects_box() {
    let m = perspective();
    let a = from_projection_matrix(&m);
    let mut box3 = Box3::new(zero3(), one3());
    let eps = EPS;

    let intersects = a.intersects_box(&box3);
    assert!(!intersects, "No intersection");

    // add eps so that we prevent box touching the frustum,
    // which might intersect depending on floating point numerics
    box3.translate(&Vector3::new(-1.0 - eps, -1.0 - eps, -1.0 - eps));

    let intersects = a.intersects_box(&box3);
    assert!(intersects, "Successful intersection");
}
