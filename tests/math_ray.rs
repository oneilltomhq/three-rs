//! Port of `three.js/test/unit/src/math/Ray.tests.js`.
//!
//! Expectations and epsilons are three.js' own; nothing here was recomputed
//! from the Rust implementation.

mod support;

use support::EPS;
use three_rs::math::{Box3, Matrix4, Plane, Ray, Sphere, Vector3};

/// `math-constants.js` `zero3`.
fn zero3() -> Vector3 {
    Vector3::default()
}

/// `math-constants.js` `one3`.
fn one3() -> Vector3 {
    Vector3::new(1.0, 1.0, 1.0)
}

/// `math-constants.js` `two3`.
fn two3() -> Vector3 {
    Vector3::new(2.0, 2.0, 2.0)
}

// INSTANCING
#[test]
fn instancing() {
    let a = Ray::default();
    assert!(a.origin.equals(&zero3()));
    assert!(a.direction.equals(&Vector3::new(0.0, 0.0, -1.0)));

    let a = Ray::new(two3(), one3());
    assert!(a.origin.equals(&two3()));
    assert!(a.direction.equals(&one3()));
}

// PUBLIC
#[test]
fn set() {
    let mut a = Ray::default();

    a.set(&one3(), &one3());
    assert!(a.origin.equals(&one3()));
    assert!(a.direction.equals(&one3()));
}

#[test]
fn recast_clone() {
    let mut a = Ray::new(one3(), Vector3::new(0.0, 0.0, 1.0));

    let a_before = a;
    assert!(a.recast(0.0).equals(&a_before));

    let mut b = a;
    assert!(b.recast(-1.0).equals(&Ray::new(
        Vector3::new(1.0, 1.0, 0.0),
        Vector3::new(0.0, 0.0, 1.0)
    )));

    let mut c = a;
    assert!(c.recast(1.0).equals(&Ray::new(
        Vector3::new(1.0, 1.0, 2.0),
        Vector3::new(0.0, 0.0, 1.0)
    )));

    let d = a;
    let mut e = d;
    e.recast(1.0);
    assert!(d.equals(&a));
    assert!(!e.equals(&d));
    assert!(e.equals(&c));
}

#[test]
// three.js reassigns `a`'s fields only to prove `b` is a true copy; nothing
// reads `a` afterwards.
#[allow(unused_assignments)]
fn copy_equals() {
    let mut a = Ray::new(zero3(), one3());
    let mut b = Ray::default();
    b.copy(&a);
    assert!(b.origin.equals(&zero3()));
    assert!(b.direction.equals(&one3()));

    // ensure that it is a true copy
    a.origin = zero3();
    a.direction = one3();
    assert!(b.origin.equals(&zero3()));
    assert!(b.direction.equals(&one3()));
}

#[test]
fn at() {
    let a = Ray::new(one3(), Vector3::new(0.0, 0.0, 1.0));

    let point = a.at(0.0);
    assert!(point.equals(&one3()));
    let point = a.at(-1.0);
    assert!(point.equals(&Vector3::new(1.0, 1.0, 0.0)));
    let point = a.at(1.0);
    assert!(point.equals(&Vector3::new(1.0, 1.0, 2.0)));
}

#[test]
fn look_at() {
    let mut a = Ray::new(two3(), one3());
    let mut target = one3();
    // three.js mutates `target` in place here: `expected` and `target` are the
    // same vector, so `lookAt()` is called with the normalized difference, not
    // with `one3`.
    let expected = *target.sub(&two3()).normalize();

    a.look_at(&target);
    assert!(
        a.direction.equals(&expected),
        "Check if we're looking in the right direction"
    );
}

#[test]
fn closest_point_to_point() {
    let a = Ray::new(one3(), Vector3::new(0.0, 0.0, 1.0));

    // behind the ray
    let point = a.closest_point_to_point(&zero3());
    assert!(point.equals(&one3()));

    // front of the ray
    let point = a.closest_point_to_point(&Vector3::new(0.0, 0.0, 50.0));
    assert!(point.equals(&Vector3::new(1.0, 1.0, 50.0)));

    // exactly on the ray
    let point = a.closest_point_to_point(&one3());
    assert!(point.equals(&one3()));
}

#[test]
fn distance_to_point() {
    let a = Ray::new(one3(), Vector3::new(0.0, 0.0, 1.0));

    // behind the ray
    let b = a.distance_to_point(&zero3());
    assert!(b == 3.0_f64.sqrt());

    // front of the ray
    let c = a.distance_to_point(&Vector3::new(0.0, 0.0, 50.0));
    assert!(c == 2.0_f64.sqrt());

    // exactly on the ray
    let d = a.distance_to_point(&one3());
    assert!(d == 0.0);
}

#[test]
fn distance_sq_to_point() {
    let a = Ray::new(one3(), Vector3::new(0.0, 0.0, 1.0));

    // behind the ray
    let b = a.distance_sq_to_point(&zero3());
    assert!(b == 3.0);

    // front of the ray
    let c = a.distance_sq_to_point(&Vector3::new(0.0, 0.0, 50.0));
    assert!(c == 2.0);

    // exactly on the ray
    let d = a.distance_sq_to_point(&one3());
    assert!(d == 0.0);
}

#[test]
fn distance_sq_to_segment() {
    let a = Ray::new(one3(), Vector3::new(0.0, 0.0, 1.0));
    let mut pt_on_line = Vector3::default();
    let mut pt_on_segment = Vector3::default();

    //segment in front of the ray
    let v0 = Vector3::new(3.0, 5.0, 50.0);
    let v1 = Vector3::new(50.0, 50.0, 50.0); // just a far away point
    let dist_sqr =
        a.distance_sq_to_segment(&v0, &v1, Some(&mut pt_on_line), Some(&mut pt_on_segment));

    assert!(pt_on_segment.distance_to(&v0) < 0.0001);
    assert!(pt_on_line.distance_to(&Vector3::new(1.0, 1.0, 50.0)) < 0.0001);
    // ((3-1) * (3-1) + (5-1) * (5-1) = 4 + 16 = 20
    assert!((dist_sqr - 20.0).abs() < 0.0001);

    //segment behind the ray
    let v0 = Vector3::new(-50.0, -50.0, -50.0); // just a far away point
    let v1 = Vector3::new(-3.0, -5.0, -4.0);
    let dist_sqr =
        a.distance_sq_to_segment(&v0, &v1, Some(&mut pt_on_line), Some(&mut pt_on_segment));

    assert!(pt_on_segment.distance_to(&v1) < 0.0001);
    assert!(pt_on_line.distance_to(&one3()) < 0.0001);
    // ((-3-1) * (-3-1) + (-5-1) * (-5-1) + (-4-1) + (-4-1) = 16 + 36 + 25 = 77
    assert!((dist_sqr - 77.0).abs() < 0.0001);

    //exact intersection between the ray and the segment
    let v0 = Vector3::new(-50.0, -50.0, -50.0);
    let v1 = Vector3::new(50.0, 50.0, 50.0);
    let dist_sqr =
        a.distance_sq_to_segment(&v0, &v1, Some(&mut pt_on_line), Some(&mut pt_on_segment));

    assert!(pt_on_segment.distance_to(&one3()) < 0.0001);
    assert!(pt_on_line.distance_to(&one3()) < 0.0001);
    assert!(dist_sqr < 0.0001);
}

#[test]
fn intersect_sphere() {
    const TOL: f64 = 0.0001;

    // ray a0 origin located at ( 0, 0, 0 ) and points outward in negative-z direction
    let a0 = Ray::new(zero3(), Vector3::new(0.0, 0.0, -1.0));
    // ray a1 origin located at ( 1, 1, 1 ) and points left in negative-x direction
    let a1 = Ray::new(one3(), Vector3::new(-1.0, 0.0, 0.0));

    // sphere (radius of 2) located behind ray a0, should result in null
    let b = Sphere::new(Vector3::new(0.0, 0.0, 3.0), 2.0);
    assert!(a0.intersect_sphere(&b).is_none());

    // sphere (radius of 2) located in front of, but too far right of ray a0, should result in null
    let b = Sphere::new(Vector3::new(3.0, 0.0, -1.0), 2.0);
    assert!(a0.intersect_sphere(&b).is_none());

    // sphere (radius of 2) located below ray a1, should result in null
    let b = Sphere::new(Vector3::new(1.0, -2.0, 1.0), 2.0);
    assert!(a1.intersect_sphere(&b).is_none());

    // sphere (radius of 1) located to the left of ray a1, should result in intersection at 0, 1, 1
    let b = Sphere::new(Vector3::new(-1.0, 1.0, 1.0), 1.0);
    let point = a1.intersect_sphere(&b).unwrap();
    assert!(point.distance_to(&Vector3::new(0.0, 1.0, 1.0)) < TOL);

    // sphere (radius of 1) located in front of ray a0, should result in intersection at 0, 0, -1
    let b = Sphere::new(Vector3::new(0.0, 0.0, -2.0), 1.0);
    let point = a0.intersect_sphere(&b).unwrap();
    assert!(point.distance_to(&Vector3::new(0.0, 0.0, -1.0)) < TOL);

    // sphere (radius of 2) located in front & right of ray a0, should result in intersection at 0, 0, -1, or left-most edge of sphere
    let b = Sphere::new(Vector3::new(2.0, 0.0, -1.0), 2.0);
    let point = a0.intersect_sphere(&b).unwrap();
    assert!(point.distance_to(&Vector3::new(0.0, 0.0, -1.0)) < TOL);

    // same situation as above, but move the sphere a fraction more to the right, and ray a0 should now just miss
    let b = Sphere::new(Vector3::new(2.01, 0.0, -1.0), 2.0);
    assert!(a0.intersect_sphere(&b).is_none());

    // following QUnit.tests are for situations where the ray origin is inside the sphere

    // sphere (radius of 1) center located at ray a0 origin / sphere surrounds the ray origin, so the first intersect point 0, 0, 1,
    // is behind ray a0.  Therefore, second exit point on back of sphere will be returned: 0, 0, -1
    // thus keeping the intersection point always in front of the ray.
    let b = Sphere::new(zero3(), 1.0);
    let point = a0.intersect_sphere(&b).unwrap();
    assert!(point.distance_to(&Vector3::new(0.0, 0.0, -1.0)) < TOL);

    // sphere (radius of 4) center located behind ray a0 origin / sphere surrounds the ray origin, so the first intersect point 0, 0, 5,
    // is behind ray a0.  Therefore, second exit point on back of sphere will be returned: 0, 0, -3
    // thus keeping the intersection point always in front of the ray.
    let b = Sphere::new(Vector3::new(0.0, 0.0, 1.0), 4.0);
    let point = a0.intersect_sphere(&b).unwrap();
    assert!(point.distance_to(&Vector3::new(0.0, 0.0, -3.0)) < TOL);

    // sphere (radius of 4) center located in front of ray a0 origin / sphere surrounds the ray origin, so the first intersect point 0, 0, 3,
    // is behind ray a0.  Therefore, second exit point on back of sphere will be returned: 0, 0, -5
    // thus keeping the intersection point always in front of the ray.
    let b = Sphere::new(Vector3::new(0.0, 0.0, -1.0), 4.0);
    let point = a0.intersect_sphere(&b).unwrap();
    assert!(point.distance_to(&Vector3::new(0.0, 0.0, -5.0)) < TOL);

    // empty sphere ( negative radius ) can never be intersected, so the
    // target must be left untouched and the return value must be null,
    // consistent with intersectsSphere()
    let b = Sphere::new(Vector3::new(0.0, 0.0, -1.0), -1.0);
    assert_eq!(a0.intersect_sphere(&b), None);
}

#[test]
fn intersects_sphere() {
    let a = Ray::new(one3(), Vector3::new(0.0, 0.0, 1.0));
    let b = Sphere::new(zero3(), 0.5);
    let c = Sphere::new(zero3(), 1.5);
    let d = Sphere::new(one3(), 0.1);
    let e = Sphere::new(two3(), 0.1);
    let f = Sphere::new(two3(), 1.0);

    assert!(!a.intersects_sphere(&b));
    assert!(!a.intersects_sphere(&c));
    assert!(a.intersects_sphere(&d));
    assert!(!a.intersects_sphere(&e));
    assert!(!a.intersects_sphere(&f));
}

#[test]
fn intersect_plane() {
    let a = Ray::new(one3(), Vector3::new(0.0, 0.0, 1.0));

    // parallel plane behind
    let mut b = Plane::default();
    b.set_from_normal_and_coplanar_point(
        &Vector3::new(0.0, 0.0, 1.0),
        &Vector3::new(1.0, 1.0, -1.0),
    );
    assert!(a.intersect_plane(&b).is_none());

    // parallel plane coincident with origin
    let mut c = Plane::default();
    c.set_from_normal_and_coplanar_point(
        &Vector3::new(0.0, 0.0, 1.0),
        &Vector3::new(1.0, 1.0, 0.0),
    );
    assert!(a.intersect_plane(&c).is_none());

    // parallel plane in front
    let mut d = Plane::default();
    d.set_from_normal_and_coplanar_point(
        &Vector3::new(0.0, 0.0, 1.0),
        &Vector3::new(1.0, 1.0, 1.0),
    );
    let point = a.intersect_plane(&d).unwrap();
    assert!(point.equals(&a.origin));

    // perpendicular ray that overlaps exactly
    let mut e = Plane::default();
    e.set_from_normal_and_coplanar_point(&Vector3::new(1.0, 0.0, 0.0), &one3());
    let point = a.intersect_plane(&e).unwrap();
    assert!(point.equals(&a.origin));

    // perpendicular ray that doesn't overlap
    let mut f = Plane::default();
    f.set_from_normal_and_coplanar_point(&Vector3::new(1.0, 0.0, 0.0), &zero3());
    assert!(a.intersect_plane(&f).is_none());
}

#[test]
fn intersects_plane() {
    let a = Ray::new(one3(), Vector3::new(0.0, 0.0, 1.0));

    // parallel plane in front of the ray
    let mut b = Plane::default();
    b.set_from_normal_and_coplanar_point(
        &Vector3::new(0.0, 0.0, 1.0),
        one3().sub(&Vector3::new(0.0, 0.0, -1.0)),
    );
    assert!(a.intersects_plane(&b));

    // parallel plane coincident with origin
    let mut c = Plane::default();
    c.set_from_normal_and_coplanar_point(
        &Vector3::new(0.0, 0.0, 1.0),
        one3().sub(&Vector3::new(0.0, 0.0, 0.0)),
    );
    assert!(a.intersects_plane(&c));

    // parallel plane behind the ray
    let mut d = Plane::default();
    d.set_from_normal_and_coplanar_point(
        &Vector3::new(0.0, 0.0, 1.0),
        one3().sub(&Vector3::new(0.0, 0.0, 1.0)),
    );
    assert!(!a.intersects_plane(&d));

    // perpendicular ray that overlaps exactly
    let mut e = Plane::default();
    e.set_from_normal_and_coplanar_point(&Vector3::new(1.0, 0.0, 0.0), &one3());
    assert!(a.intersects_plane(&e));

    // perpendicular ray that doesn't overlap
    let mut f = Plane::default();
    f.set_from_normal_and_coplanar_point(&Vector3::new(1.0, 0.0, 0.0), &zero3());
    assert!(!a.intersects_plane(&f));
}

#[test]
fn intersect_box() {
    const TOL: f64 = 0.0001;

    let box3 = Box3::new(Vector3::new(-1.0, -1.0, -1.0), Vector3::new(1.0, 1.0, 1.0));

    let a = Ray::new(Vector3::new(-2.0, 0.0, 0.0), Vector3::new(1.0, 0.0, 0.0));
    //ray should intersect box at -1,0,0
    assert!(a.intersects_box(&box3));
    let point = a.intersect_box(&box3).unwrap();
    assert!(point.distance_to(&Vector3::new(-1.0, 0.0, 0.0)) < TOL);

    let b = Ray::new(Vector3::new(-2.0, 0.0, 0.0), Vector3::new(-1.0, 0.0, 0.0));
    //ray is point away from box, it should not intersect
    assert!(!b.intersects_box(&box3));
    assert!(b.intersect_box(&box3).is_none());

    let c = Ray::new(Vector3::new(0.0, 0.0, 0.0), Vector3::new(1.0, 0.0, 0.0));
    // ray is inside box, should return exit point
    assert!(c.intersects_box(&box3));
    let point = c.intersect_box(&box3).unwrap();
    assert!(point.distance_to(&Vector3::new(1.0, 0.0, 0.0)) < TOL);

    let d = Ray::new(
        Vector3::new(0.0, 2.0, 1.0),
        Vector3::new(0.0, -1.0, -1.0).normalized(),
    );
    //tilted ray should intersect box at 0,1,0
    assert!(d.intersects_box(&box3));
    let point = d.intersect_box(&box3).unwrap();
    assert!(point.distance_to(&Vector3::new(0.0, 1.0, 0.0)) < TOL);

    let e = Ray::new(
        Vector3::new(1.0, -2.0, 1.0),
        Vector3::new(0.0, 1.0, 0.0).normalized(),
    );
    //handle case where ray is coplanar with one of the boxes side - box in front of ray
    assert!(e.intersects_box(&box3));
    let point = e.intersect_box(&box3).unwrap();
    assert!(point.distance_to(&Vector3::new(1.0, -1.0, 1.0)) < TOL);

    let f = Ray::new(
        Vector3::new(1.0, -2.0, 0.0),
        Vector3::new(0.0, -1.0, 0.0).normalized(),
    );
    //handle case where ray is coplanar with one of the boxes side - box behind ray
    assert!(!f.intersects_box(&box3));
    assert!(f.intersect_box(&box3).is_none());
}

#[test]
fn intersect_triangle() {
    let mut ray = Ray::default();
    let mut a = Vector3::new(1.0, 1.0, 0.0);
    let mut b = Vector3::new(0.0, 1.0, 1.0);
    let c = Vector3::new(1.0, 0.0, 1.0);

    // DdN == 0
    let origin = ray.origin;
    ray.set(&origin, &zero3());
    assert!(
        ray.intersect_triangle(&a, &b, &c, false).is_none(),
        "No intersection if direction == zero"
    );

    // DdN > 0, backfaceCulling = true
    let origin = ray.origin;
    ray.set(&origin, &one3());
    assert!(
        ray.intersect_triangle(&a, &b, &c, true).is_none(),
        "No intersection with backside faces if backfaceCulling is true"
    );

    // DdN > 0
    let origin = ray.origin;
    ray.set(&origin, &one3());
    let point = ray.intersect_triangle(&a, &b, &c, false).unwrap();
    assert!(
        (point.x - 2.0 / 3.0).abs() <= EPS,
        "Successful intersection: check x"
    );
    assert!(
        (point.y - 2.0 / 3.0).abs() <= EPS,
        "Successful intersection: check y"
    );
    assert!(
        (point.z - 2.0 / 3.0).abs() <= EPS,
        "Successful intersection: check z"
    );

    // DdN > 0, DdQxE2 < 0
    b.multiply_scalar(-1.0);
    assert!(
        ray.intersect_triangle(&a, &b, &c, false).is_none(),
        "No intersection"
    );

    // DdN > 0, DdE1xQ < 0
    a.multiply_scalar(-1.0);
    assert!(
        ray.intersect_triangle(&a, &b, &c, false).is_none(),
        "No intersection"
    );

    // DdN > 0, DdQxE2 + DdE1xQ > DdN
    b.multiply_scalar(-1.0);
    assert!(
        ray.intersect_triangle(&a, &b, &c, false).is_none(),
        "No intersection"
    );

    // DdN < 0, QdN < 0
    a.multiply_scalar(-1.0);
    b.multiply_scalar(-1.0);
    ray.direction.multiply_scalar(-1.0);
    assert!(
        ray.intersect_triangle(&a, &b, &c, false).is_none(),
        "No intersection when looking in the wrong direction"
    );
}

#[test]
fn intersect_triangle_watertight_at_shared_edges() {
    // Two triangles forming a quad and sharing the diagonal edge from
    // ( -2, -2, -2 ) to ( 2, -2, 2 ). A ray aimed exactly at the midpoint of
    // that shared edge must be detected: a non-watertight test can let the ray
    // slip through the seam between the triangles and miss both of them.

    let t1a = Vector3::new(-2.0, -2.0, 2.0);
    let t1b = Vector3::new(-2.0, -2.0, -2.0);
    let t1c = Vector3::new(2.0, -2.0, 2.0);

    let t2a = Vector3::new(-2.0, -2.0, -2.0);
    let t2b = Vector3::new(2.0, -2.0, -2.0);
    let t2c = Vector3::new(2.0, -2.0, 2.0);

    let seam = Vector3::new(0.0, -2.0, 0.0); // midpoint of the shared edge
    let origin = Vector3::new(-4.0, -9.0, 0.4);
    let mut direction = Vector3::default();
    direction.sub_vectors(&seam, &origin).normalize();
    let ray = Ray::new(origin, direction);

    let hit1 = ray.intersect_triangle(&t1a, &t1b, &t1c, false);
    let hit2 = ray.intersect_triangle(&t2a, &t2b, &t2c, false);

    assert!(
        hit1.is_some() || hit2.is_some(),
        "Ray hitting the shared edge is not dropped"
    );

    let hit = if hit1.is_some() {
        hit1.unwrap()
    } else {
        hit2.unwrap()
    };
    assert!(
        hit.distance_to(&seam) <= EPS,
        "Intersection lies on the shared edge"
    );
}

#[test]
fn apply_matrix4() {
    let a = Ray::new(one3(), Vector3::new(0.0, 0.0, 1.0));
    let mut m = Matrix4::default();

    assert!(a.clone().apply_matrix4(&m).equals(&a));

    let mut a = Ray::new(zero3(), Vector3::new(0.0, 0.0, 1.0));
    m.make_rotation_z(std::f64::consts::PI);
    assert!(a.clone().apply_matrix4(&m).equals(&a));

    m.make_rotation_x(std::f64::consts::PI);
    let mut b = a;
    b.direction.negate();
    let a2 = *a.clone().apply_matrix4(&m);
    assert!(a2.origin.distance_to(&b.origin) < 0.0001);
    assert!(a2.direction.distance_to(&b.direction) < 0.0001);

    a.origin = Vector3::new(0.0, 0.0, 1.0);
    b.origin = Vector3::new(0.0, 0.0, -1.0);
    let a2 = *a.clone().apply_matrix4(&m);
    assert!(a2.origin.distance_to(&b.origin) < 0.0001);
    assert!(a2.direction.distance_to(&b.direction) < 0.0001);
}
