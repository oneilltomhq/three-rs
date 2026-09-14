//! Port of `three.js/test/unit/src/math/Triangle.tests.js`.
//!
//! Expectations and epsilons are three.js' own; nothing here was recomputed
//! from the Rust implementation.

use three_rs::core::BufferAttribute;
use three_rs::math::{Box3, Triangle, Vector3};

/// `math-constants.js` `zero3`.
const ZERO3: Vector3 = Vector3::new(0.0, 0.0, 0.0);
/// `math-constants.js` `one3`.
const ONE3: Vector3 = Vector3::new(1.0, 1.0, 1.0);
/// `math-constants.js` `two3`.
const TWO3: Vector3 = Vector3::new(2.0, 2.0, 2.0);

/// `one3.clone().negate()`.
const NEG_ONE3: Vector3 = Vector3::new(-1.0, -1.0, -1.0);

// INSTANCING
#[test]
fn instancing() {
    let a = Triangle::default();
    assert!(a.a.equals(&ZERO3));
    assert!(a.b.equals(&ZERO3));
    assert!(a.c.equals(&ZERO3));

    let a = Triangle::new(NEG_ONE3, ONE3, TWO3);
    assert!(a.a.equals(&NEG_ONE3));
    assert!(a.b.equals(&ONE3));
    assert!(a.c.equals(&TWO3));
}

// PUBLIC
#[test]
fn set() {
    let mut a = Triangle::default();

    a.set(&NEG_ONE3, &ONE3, &TWO3);
    assert!(a.a.equals(&NEG_ONE3));
    assert!(a.b.equals(&ONE3));
    assert!(a.c.equals(&TWO3));
}

#[test]
fn set_from_points_and_indices() {
    let mut a = Triangle::default();

    let points = [ONE3, NEG_ONE3, TWO3];
    a.set_from_points_and_indices(&points, 1, 0, 2);
    assert!(a.a.equals(&NEG_ONE3));
    assert!(a.b.equals(&ONE3));
    assert!(a.c.equals(&TWO3));
}

#[test]
fn set_from_attribute_and_indices() {
    let mut a = Triangle::default();
    let attribute = BufferAttribute::new(vec![1.0, 1.0, 1.0, -1.0, -1.0, -1.0, 2.0, 2.0, 2.0], 3);

    a.set_from_attribute_and_indices(&attribute, 1, 0, 2);
    assert!(a.a.equals(&NEG_ONE3));
    assert!(a.b.equals(&ONE3));
    assert!(a.c.equals(&TWO3));
}

#[test]
// three.js mutates `a` after the copy purely to prove `b` is independent.
#[allow(unused_assignments)]
fn copy() {
    let mut a = Triangle::new(NEG_ONE3, ONE3, TWO3);
    let mut b = Triangle::default();
    b.copy(&a);
    assert!(b.a.equals(&NEG_ONE3));
    assert!(b.b.equals(&ONE3));
    assert!(b.c.equals(&TWO3));

    // ensure that it is a true copy
    a.a = ONE3;
    a.b = ZERO3;
    a.c = ZERO3;
    assert!(b.a.equals(&NEG_ONE3));
    assert!(b.b.equals(&ONE3));
    assert!(b.c.equals(&TWO3));
}

#[test]
fn get_area() {
    let a = Triangle::default();

    assert!(a.get_area() == 0.0);

    let a = Triangle::new(
        Vector3::new(0.0, 0.0, 0.0),
        Vector3::new(1.0, 0.0, 0.0),
        Vector3::new(0.0, 1.0, 0.0),
    );
    assert!(a.get_area() == 0.5);

    let a = Triangle::new(
        Vector3::new(2.0, 0.0, 0.0),
        Vector3::new(0.0, 0.0, 0.0),
        Vector3::new(0.0, 0.0, 2.0),
    );
    assert!(a.get_area() == 2.0);

    // colinear triangle.
    let a = Triangle::new(
        Vector3::new(2.0, 0.0, 0.0),
        Vector3::new(0.0, 0.0, 0.0),
        Vector3::new(3.0, 0.0, 0.0),
    );
    assert!(a.get_area() == 0.0);
}

#[test]
fn get_midpoint() {
    let a = Triangle::default();

    assert!(a.get_midpoint().equals(&Vector3::new(0.0, 0.0, 0.0)));

    let a = Triangle::new(
        Vector3::new(0.0, 0.0, 0.0),
        Vector3::new(1.0, 0.0, 0.0),
        Vector3::new(0.0, 1.0, 0.0),
    );
    assert!(a
        .get_midpoint()
        .equals(&Vector3::new(1.0 / 3.0, 1.0 / 3.0, 0.0)));

    let a = Triangle::new(
        Vector3::new(2.0, 0.0, 0.0),
        Vector3::new(0.0, 0.0, 0.0),
        Vector3::new(0.0, 0.0, 2.0),
    );
    assert!(a
        .get_midpoint()
        .equals(&Vector3::new(2.0 / 3.0, 0.0, 2.0 / 3.0)));
}

#[test]
fn get_normal() {
    let a = Triangle::default();

    assert!(a.get_normal().equals(&Vector3::new(0.0, 0.0, 0.0)));

    let a = Triangle::new(
        Vector3::new(0.0, 0.0, 0.0),
        Vector3::new(1.0, 0.0, 0.0),
        Vector3::new(0.0, 1.0, 0.0),
    );
    assert!(a.get_normal().equals(&Vector3::new(0.0, 0.0, 1.0)));

    let a = Triangle::new(
        Vector3::new(2.0, 0.0, 0.0),
        Vector3::new(0.0, 0.0, 0.0),
        Vector3::new(0.0, 0.0, 2.0),
    );
    assert!(a.get_normal().equals(&Vector3::new(0.0, 1.0, 0.0)));
}

#[test]
fn get_plane() {
    let a = Triangle::default();

    let plane = a.get_plane();
    assert!(!plane.distance_to_point(&a.a).is_nan());
    assert!(!plane.distance_to_point(&a.b).is_nan());
    assert!(!plane.distance_to_point(&a.c).is_nan());
    // assert.notPropEqual( plane.normal, { x: NaN, y: NaN, z: NaN } )
    assert!(!plane.normal.x.is_nan());
    assert!(!plane.normal.y.is_nan());
    assert!(!plane.normal.z.is_nan());

    let a = Triangle::new(
        Vector3::new(0.0, 0.0, 0.0),
        Vector3::new(1.0, 0.0, 0.0),
        Vector3::new(0.0, 1.0, 0.0),
    );
    let plane = a.get_plane();
    let normal = a.get_normal();
    assert!(plane.distance_to_point(&a.a) == 0.0);
    assert!(plane.distance_to_point(&a.b) == 0.0);
    assert!(plane.distance_to_point(&a.c) == 0.0);
    assert!(plane.normal.equals(&normal));

    let a = Triangle::new(
        Vector3::new(2.0, 0.0, 0.0),
        Vector3::new(0.0, 0.0, 0.0),
        Vector3::new(0.0, 0.0, 2.0),
    );
    let plane = a.get_plane();
    let normal = a.get_normal();
    assert!(plane.distance_to_point(&a.a) == 0.0);
    assert!(plane.distance_to_point(&a.b) == 0.0);
    assert!(plane.distance_to_point(&a.c) == 0.0);
    assert!(plane.normal.normalized().equals(&normal));
}

#[test]
fn get_barycoord() {
    let a = Triangle::default();

    assert!(a.get_barycoord(&a.a).is_none());
    assert!(a.get_barycoord(&a.b).is_none());
    assert!(a.get_barycoord(&a.c).is_none());

    let a = Triangle::new(
        Vector3::new(0.0, 0.0, 0.0),
        Vector3::new(1.0, 0.0, 0.0),
        Vector3::new(0.0, 1.0, 0.0),
    );
    let midpoint = a.get_midpoint();

    let barycoord = a.get_barycoord(&a.a).unwrap();
    assert!(barycoord.equals(&Vector3::new(1.0, 0.0, 0.0)));
    let barycoord = a.get_barycoord(&a.b).unwrap();
    assert!(barycoord.equals(&Vector3::new(0.0, 1.0, 0.0)));
    let barycoord = a.get_barycoord(&a.c).unwrap();
    assert!(barycoord.equals(&Vector3::new(0.0, 0.0, 1.0)));
    let barycoord = a.get_barycoord(&midpoint).unwrap();
    assert!(barycoord.distance_to(&Vector3::new(1.0 / 3.0, 1.0 / 3.0, 1.0 / 3.0)) < 0.0001);

    let a = Triangle::new(
        Vector3::new(2.0, 0.0, 0.0),
        Vector3::new(0.0, 0.0, 0.0),
        Vector3::new(0.0, 0.0, 2.0),
    );
    let midpoint = a.get_midpoint();

    let barycoord = a.get_barycoord(&a.a).unwrap();
    assert!(barycoord.equals(&Vector3::new(1.0, 0.0, 0.0)));
    let barycoord = a.get_barycoord(&a.b).unwrap();
    assert!(barycoord.equals(&Vector3::new(0.0, 1.0, 0.0)));
    let barycoord = a.get_barycoord(&a.c).unwrap();
    assert!(barycoord.equals(&Vector3::new(0.0, 0.0, 1.0)));
    let barycoord = a.get_barycoord(&midpoint).unwrap();
    assert!(barycoord.distance_to(&Vector3::new(1.0 / 3.0, 1.0 / 3.0, 1.0 / 3.0)) < 0.0001);
}

#[test]
fn contains_point() {
    let a = Triangle::default();

    assert!(!a.contains_point(&a.a));
    assert!(!a.contains_point(&a.b));
    assert!(!a.contains_point(&a.c));

    let a = Triangle::new(
        Vector3::new(0.0, 0.0, 0.0),
        Vector3::new(1.0, 0.0, 0.0),
        Vector3::new(0.0, 1.0, 0.0),
    );
    let midpoint = a.get_midpoint();
    assert!(a.contains_point(&a.a));
    assert!(a.contains_point(&a.b));
    assert!(a.contains_point(&a.c));
    assert!(a.contains_point(&midpoint));
    assert!(!a.contains_point(&Vector3::new(-1.0, -1.0, -1.0)));

    let a = Triangle::new(
        Vector3::new(2.0, 0.0, 0.0),
        Vector3::new(0.0, 0.0, 0.0),
        Vector3::new(0.0, 0.0, 2.0),
    );
    let midpoint = a.get_midpoint();
    assert!(a.contains_point(&a.a));
    assert!(a.contains_point(&a.b));
    assert!(a.contains_point(&a.c));
    assert!(a.contains_point(&midpoint));
    assert!(!a.contains_point(&Vector3::new(-1.0, -1.0, -1.0)));
}

#[test]
fn intersects_box() {
    let a = Box3::new(ONE3, TWO3);
    let b = Triangle::new(
        Vector3::new(1.5, 1.5, 2.5),
        Vector3::new(2.5, 1.5, 1.5),
        Vector3::new(1.5, 2.5, 1.5),
    );
    let c = Triangle::new(
        Vector3::new(1.5, 1.5, 3.5),
        Vector3::new(3.5, 1.5, 1.5),
        Vector3::new(1.5, 1.5, 1.5),
    );
    let d = Triangle::new(
        Vector3::new(1.5, 1.75, 3.0),
        Vector3::new(3.0, 1.75, 1.5),
        Vector3::new(1.5, 2.5, 1.5),
    );
    let e = Triangle::new(
        Vector3::new(1.5, 1.8, 3.0),
        Vector3::new(3.0, 1.8, 1.5),
        Vector3::new(1.5, 2.5, 1.5),
    );
    let f = Triangle::new(
        Vector3::new(1.5, 2.5, 3.0),
        Vector3::new(3.0, 2.5, 1.5),
        Vector3::new(1.5, 2.5, 1.5),
    );

    assert!(b.intersects_box(&a));
    assert!(c.intersects_box(&a));
    assert!(d.intersects_box(&a));
    assert!(!e.intersects_box(&a));
    assert!(!f.intersects_box(&a));
}

#[test]
fn closest_point_to_point() {
    let a = Triangle::new(
        Vector3::new(-1.0, 0.0, 0.0),
        Vector3::new(1.0, 0.0, 0.0),
        Vector3::new(0.0, 1.0, 0.0),
    );

    // point lies inside the triangle
    let point = a.closest_point_to_point(&Vector3::new(0.0, 0.5, 0.0));
    assert!(point.equals(&Vector3::new(0.0, 0.5, 0.0)));

    // point lies on a vertex
    let point = a.closest_point_to_point(&a.a);
    assert!(point.equals(&a.a));

    let point = a.closest_point_to_point(&a.b);
    assert!(point.equals(&a.b));

    let point = a.closest_point_to_point(&a.c);
    assert!(point.equals(&a.c));

    // point lies on an edge
    let point = a.closest_point_to_point(&ZERO3);
    assert!(point.equals(&ZERO3));

    // point lies outside the triangle
    let point = a.closest_point_to_point(&Vector3::new(-2.0, 0.0, 0.0));
    assert!(point.equals(&Vector3::new(-1.0, 0.0, 0.0)));

    let point = a.closest_point_to_point(&Vector3::new(2.0, 0.0, 0.0));
    assert!(point.equals(&Vector3::new(1.0, 0.0, 0.0)));

    let point = a.closest_point_to_point(&Vector3::new(0.0, 2.0, 0.0));
    assert!(point.equals(&Vector3::new(0.0, 1.0, 0.0)));

    let point = a.closest_point_to_point(&Vector3::new(0.0, -2.0, 0.0));
    assert!(point.equals(&Vector3::new(0.0, 0.0, 0.0)));
}

#[test]
fn is_front_facing() {
    let a = Triangle::default();
    let dir = Vector3::default();
    assert!(!a.is_front_facing(&dir));

    let a = Triangle::new(
        Vector3::new(0.0, 0.0, 0.0),
        Vector3::new(1.0, 0.0, 0.0),
        Vector3::new(0.0, 1.0, 0.0),
    );
    let dir = Vector3::new(0.0, 0.0, -1.0);
    assert!(a.is_front_facing(&dir));

    let a = Triangle::new(
        Vector3::new(0.0, 0.0, 0.0),
        Vector3::new(0.0, 1.0, 0.0),
        Vector3::new(1.0, 0.0, 0.0),
    );
    assert!(!a.is_front_facing(&dir));
}

#[test]
fn equals() {
    let mut a = Triangle::new(
        Vector3::new(1.0, 0.0, 0.0),
        Vector3::new(0.0, 1.0, 0.0),
        Vector3::new(0.0, 0.0, 1.0),
    );
    let b = Triangle::new(
        Vector3::new(0.0, 0.0, 1.0),
        Vector3::new(0.0, 1.0, 0.0),
        Vector3::new(1.0, 0.0, 0.0),
    );
    let c = Triangle::new(
        Vector3::new(-1.0, 0.0, 0.0),
        Vector3::new(0.0, 1.0, 0.0),
        Vector3::new(0.0, 0.0, 1.0),
    );

    assert!(a.equals(&a), "a equals a");
    assert!(!a.equals(&b), "a does not equal b");
    assert!(!a.equals(&c), "a does not equal c");
    assert!(!b.equals(&c), "b does not equal c");

    a.copy(&b);
    // three.js' own assertion here is `a.equals( a )`, not `a.equals( b )`.
    assert!(a.equals(&a), "a equals b after copy()");
}
