//! Port of `three.js/test/unit/src/math/Box3.tests.js`.
//!
//! Expectations and epsilons are three.js' own; nothing here was recomputed
//! from the Rust implementation.

use std::rc::Rc;

use three_rs::core::{BufferAttribute, BufferGeometry, Node};
use three_rs::math::{Box3, Matrix4, Plane, Sphere, Triangle, Vector3};
use three_rs::objects::{Group, Mesh};

/// `math-constants.js` `negInf3`.
const NEG_INF3: Vector3 = Vector3::new(f64::NEG_INFINITY, f64::NEG_INFINITY, f64::NEG_INFINITY);
/// `math-constants.js` `posInf3`.
const POS_INF3: Vector3 = Vector3::new(f64::INFINITY, f64::INFINITY, f64::INFINITY);
/// `math-constants.js` `zero3`.
const ZERO3: Vector3 = Vector3::new(0.0, 0.0, 0.0);
/// `math-constants.js` `one3`.
const ONE3: Vector3 = Vector3::new(1.0, 1.0, 1.0);
/// `math-constants.js` `two3`.
const TWO3: Vector3 = Vector3::new(2.0, 2.0, 2.0);

/// `Box3.tests.js` `compareBox()`.
fn compare_box(a: &Box3, b: &Box3, threshold: Option<f64>) -> bool {
    let threshold = threshold.unwrap_or(0.0001);
    a.min.distance_to(&b.min) < threshold && a.max.distance_to(&b.max) < threshold
}

/// `Vector3.clone().negate()`.
fn negated(v: Vector3) -> Vector3 {
    let mut v = v;
    *v.negate()
}

// INSTANCING
#[test]
fn instancing() {
    let a = Box3::default();
    assert!(a.min.equals(&POS_INF3));
    assert!(a.max.equals(&NEG_INF3));

    let a = Box3::new(ZERO3, ZERO3);
    assert!(a.min.equals(&ZERO3));
    assert!(a.max.equals(&ZERO3));

    let a = Box3::new(ZERO3, ONE3);
    assert!(a.min.equals(&ZERO3));
    assert!(a.max.equals(&ONE3));
}

// PUBLIC STUFF
#[test]
fn is_box3() {
    // Mirrors three.js's `assert.ok( a.isBox3 )`: this pins the public `IS_BOX3`
    // constant's value, which happens to be `true` today but is not statically
    // guaranteed to stay that way.
    #[allow(clippy::assertions_on_constants)]
    {
        assert!(Box3::IS_BOX3);
    }

    // `Sphere` carries no `IS_BOX3` flag at all, which is the Rust equivalent
    // of `! b.isBox3`.
}

#[test]
fn set() {
    let mut a = Box3::default();

    a.set(&ZERO3, &ONE3);
    assert!(a.min.equals(&ZERO3));
    assert!(a.max.equals(&ONE3));
}

#[test]
fn set_from_array() {
    let mut a = Box3::default();

    a.set_from_array(&[0.0, 0.0, 0.0, 1.0, 1.0, 1.0, 2.0, 2.0, 2.0]);
    assert!(a.min.equals(&ZERO3));
    assert!(a.max.equals(&TWO3));
}

#[test]
fn set_from_buffer_attribute() {
    let mut a = Box3::new(ZERO3, ONE3);
    let bigger = BufferAttribute::new(
        vec![
            -2.0, -2.0, -2.0, 2.0, 2.0, 2.0, 1.5, 1.5, 1.5, 0.0, 0.0, 0.0,
        ],
        3,
    );
    let smaller = BufferAttribute::new(vec![-0.5, -0.5, -0.5, 0.5, 0.5, 0.5, 0.0, 0.0, 0.0], 3);
    let mut new_min = Vector3::new(-2.0, -2.0, -2.0);
    let mut new_max = Vector3::new(2.0, 2.0, 2.0);

    a.set_from_buffer_attribute(&bigger);
    assert!(a.min.equals(&new_min), "Bigger box: correct new minimum");
    assert!(a.max.equals(&new_max), "Bigger box: correct new maximum");

    new_min.set(-0.5, -0.5, -0.5);
    new_max.set(0.5, 0.5, 0.5);

    a.set_from_buffer_attribute(&smaller);
    assert!(a.min.equals(&new_min), "Smaller box: correct new minimum");
    assert!(a.max.equals(&new_max), "Smaller box: correct new maximum");
}

#[test]
fn set_from_points() {
    let mut a = Box3::default();

    a.set_from_points(&[ZERO3, ONE3, TWO3]);
    assert!(a.min.equals(&ZERO3));
    assert!(a.max.equals(&TWO3));

    a.set_from_points(&[ONE3]);
    assert!(a.min.equals(&ONE3));
    assert!(a.max.equals(&ONE3));

    a.set_from_points(&[]);
    assert!(a.is_empty());
}

#[test]
fn set_from_center_and_size() {
    let mut a = Box3::new(ZERO3, ONE3);
    let b = a;
    let new_center = ONE3;
    let new_size = TWO3;

    let center_a = a.get_center();
    let size_a = a.get_size();
    a.set_from_center_and_size(&center_a, &size_a);
    assert!(a.equals(&b), "Same values: no changes");

    a.set_from_center_and_size(&new_center, &size_a);
    let center_a = a.get_center();
    let size_a = a.get_size();
    let size_b = b.get_size();

    assert!(
        center_a.equals(&new_center),
        "Move center: correct new center"
    );
    assert!(size_a.equals(&size_b), "Move center: no change in size");
    assert!(!a.equals(&b), "Move center: no longer equal to old values");

    a.set_from_center_and_size(&center_a, &new_size);
    let center_a = a.get_center();
    let size_a = a.get_size();
    assert!(center_a.equals(&new_center), "Resize: no change to center");
    assert!(size_a.equals(&new_size), "Resize: correct new size");
    assert!(!a.equals(&b), "Resize: no longer equal to old values");
}

#[test]
fn clone() {
    let a = Box3::new(ZERO3, ONE3);

    let b = a;
    assert!(b.min.equals(&ZERO3));
    assert!(b.max.equals(&ONE3));

    let a = Box3::default();
    let b = a;
    assert!(b.min.equals(&POS_INF3));
    assert!(b.max.equals(&NEG_INF3));
}

#[test]
// three.js mutates `a` after the copy purely to prove the copy is deep.
#[allow(unused_assignments)]
fn copy() {
    let mut a = Box3::new(ZERO3, ONE3);
    let mut b = Box3::default();
    b.copy(&a);
    assert!(b.min.equals(&ZERO3));
    assert!(b.max.equals(&ONE3));

    // ensure that it is a true copy
    a.min = ZERO3;
    a.max = ONE3;
    assert!(b.min.equals(&ZERO3));
    assert!(b.max.equals(&ONE3));
}

#[test]
fn empty_make_empty() {
    let a = Box3::default();

    assert!(a.is_empty());

    let mut a = Box3::new(ZERO3, ONE3);
    assert!(!a.is_empty());

    a.make_empty();
    assert!(a.is_empty());
}

#[test]
fn is_empty() {
    let a = Box3::new(ZERO3, ZERO3);
    assert!(!a.is_empty());

    let a = Box3::new(ZERO3, ONE3);
    assert!(!a.is_empty());

    let a = Box3::new(TWO3, ONE3);
    assert!(a.is_empty());

    let a = Box3::new(POS_INF3, NEG_INF3);
    assert!(a.is_empty());
}

#[test]
fn get_center() {
    let a = Box3::new(ZERO3, ZERO3);

    assert!(a.get_center().equals(&ZERO3));

    let a = Box3::new(ZERO3, ONE3);
    let mut midpoint = ONE3;
    let midpoint = *midpoint.multiply_scalar(0.5);
    assert!(a.get_center().equals(&midpoint));
}

#[test]
fn get_size() {
    let a = Box3::new(ZERO3, ZERO3);

    assert!(a.get_size().equals(&ZERO3));

    let a = Box3::new(ZERO3, ONE3);
    assert!(a.get_size().equals(&ONE3));
}

#[test]
fn expand_by_point() {
    let mut a = Box3::new(ZERO3, ZERO3);

    a.expand_by_point(&ZERO3);
    assert!(a.get_size().equals(&ZERO3));

    a.expand_by_point(&ONE3);
    assert!(a.get_size().equals(&ONE3));

    a.expand_by_point(&negated(ONE3));
    let mut doubled = ONE3;
    assert!(a.get_size().equals(doubled.multiply_scalar(2.0)));
    assert!(a.get_center().equals(&ZERO3));
}

#[test]
fn expand_by_vector() {
    let mut a = Box3::new(ZERO3, ZERO3);

    a.expand_by_vector(&ZERO3);
    assert!(a.get_size().equals(&ZERO3));

    a.expand_by_vector(&ONE3);
    let mut doubled = ONE3;
    assert!(a.get_size().equals(doubled.multiply_scalar(2.0)));
    assert!(a.get_center().equals(&ZERO3));
}

#[test]
fn expand_by_scalar() {
    let mut a = Box3::new(ZERO3, ZERO3);

    a.expand_by_scalar(0.0);
    assert!(a.get_size().equals(&ZERO3));

    a.expand_by_scalar(1.0);
    let mut doubled = ONE3;
    assert!(a.get_size().equals(doubled.multiply_scalar(2.0)));
    assert!(a.get_center().equals(&ZERO3));
}

#[test]
fn contains_point() {
    let mut a = Box3::new(ZERO3, ZERO3);

    assert!(a.contains_point(&ZERO3));
    assert!(!a.contains_point(&ONE3));

    a.expand_by_scalar(1.0);
    assert!(a.contains_point(&ZERO3));
    assert!(a.contains_point(&ONE3));
    assert!(a.contains_point(&negated(ONE3)));
}

#[test]
fn contains_box() {
    let a = Box3::new(ZERO3, ZERO3);
    let b = Box3::new(ZERO3, ONE3);
    let c = Box3::new(negated(ONE3), ONE3);

    assert!(a.contains_box(&a));
    assert!(!a.contains_box(&b));
    assert!(!a.contains_box(&c));

    assert!(b.contains_box(&a));
    assert!(c.contains_box(&a));
    assert!(!b.contains_box(&c));
}

#[test]
fn get_parameter() {
    let a = Box3::new(ZERO3, ONE3);
    let b = Box3::new(negated(ONE3), ONE3);

    let parameter = a.get_parameter(&ZERO3);
    assert!(parameter.equals(&ZERO3));
    let parameter = a.get_parameter(&ONE3);
    assert!(parameter.equals(&ONE3));

    let parameter = b.get_parameter(&negated(ONE3));
    assert!(parameter.equals(&ZERO3));
    let parameter = b.get_parameter(&ZERO3);
    assert!(parameter.equals(&Vector3::new(0.5, 0.5, 0.5)));
    let parameter = b.get_parameter(&ONE3);
    assert!(parameter.equals(&ONE3));
}

#[test]
fn intersects_box() {
    let a = Box3::new(ZERO3, ZERO3);
    let mut b = Box3::new(ZERO3, ONE3);
    let c = Box3::new(negated(ONE3), ONE3);

    assert!(a.intersects_box(&a));
    assert!(a.intersects_box(&b));
    assert!(a.intersects_box(&c));

    assert!(b.intersects_box(&a));
    assert!(c.intersects_box(&a));
    assert!(b.intersects_box(&c));

    b.translate(&Vector3::new(2.0, 2.0, 2.0));
    assert!(!a.intersects_box(&b));
    assert!(!b.intersects_box(&a));
    assert!(!b.intersects_box(&c));
}

#[test]
fn intersects_sphere() {
    let a = Box3::new(ZERO3, ONE3);
    let mut b = Sphere::new(ZERO3, 1.0);

    assert!(a.intersects_sphere(&b));

    b.translate(&Vector3::new(2.0, 2.0, 2.0));
    assert!(!a.intersects_sphere(&b));
}

#[test]
fn intersects_plane() {
    let a = Box3::new(ZERO3, ONE3);
    let b = Plane::new(Vector3::new(0.0, 1.0, 0.0), 1.0);
    let c = Plane::new(Vector3::new(0.0, 1.0, 0.0), 1.25);
    let d = Plane::new(Vector3::new(0.0, -1.0, 0.0), 1.25);
    let e = Plane::new(Vector3::new(0.0, 1.0, 0.0), 0.25);
    let f = Plane::new(Vector3::new(0.0, 1.0, 0.0), -0.25);
    let g = Plane::new(Vector3::new(0.0, 1.0, 0.0), -0.75);
    let h = Plane::new(Vector3::new(0.0, 1.0, 0.0), -1.0);
    let i = Plane::new(Vector3::new(1.0, 1.0, 1.0).normalized(), -1.732);
    let j = Plane::new(Vector3::new(1.0, 1.0, 1.0).normalized(), -1.733);

    assert!(!a.intersects_plane(&b));
    assert!(!a.intersects_plane(&c));
    assert!(!a.intersects_plane(&d));
    assert!(!a.intersects_plane(&e));
    assert!(a.intersects_plane(&f));
    assert!(a.intersects_plane(&g));
    assert!(a.intersects_plane(&h));
    assert!(a.intersects_plane(&i));
    assert!(!a.intersects_plane(&j));
}

#[test]
fn intersects_triangle() {
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

    assert!(a.intersects_triangle(&b));
    assert!(a.intersects_triangle(&c));
    assert!(a.intersects_triangle(&d));
    assert!(!a.intersects_triangle(&e));
    assert!(!a.intersects_triangle(&f));
}

#[test]
fn clamp_point() {
    let a = Box3::new(ZERO3, ZERO3);
    let b = Box3::new(negated(ONE3), ONE3);

    let point = a.clamp_point(&ZERO3);
    assert!(point.equals(&ZERO3));
    let point = a.clamp_point(&ONE3);
    assert!(point.equals(&ZERO3));
    let point = a.clamp_point(&negated(ONE3));
    assert!(point.equals(&ZERO3));

    let point = b.clamp_point(&Vector3::new(2.0, 2.0, 2.0));
    assert!(point.equals(&ONE3));
    let point = b.clamp_point(&ONE3);
    assert!(point.equals(&ONE3));
    let point = b.clamp_point(&ZERO3);
    assert!(point.equals(&ZERO3));
    let point = b.clamp_point(&negated(ONE3));
    assert!(point.equals(&negated(ONE3)));
    let point = b.clamp_point(&Vector3::new(-2.0, -2.0, -2.0));
    assert!(point.equals(&negated(ONE3)));
}

#[test]
fn distance_to_point() {
    let a = Box3::new(ZERO3, ZERO3);
    let b = Box3::new(negated(ONE3), ONE3);

    assert!(a.distance_to_point(&Vector3::new(0.0, 0.0, 0.0)) == 0.0);
    assert!(a.distance_to_point(&Vector3::new(1.0, 1.0, 1.0)) == 3.0_f64.sqrt());
    assert!(a.distance_to_point(&Vector3::new(-1.0, -1.0, -1.0)) == 3.0_f64.sqrt());

    assert!(b.distance_to_point(&Vector3::new(2.0, 2.0, 2.0)) == 3.0_f64.sqrt());
    assert!(b.distance_to_point(&Vector3::new(1.0, 1.0, 1.0)) == 0.0);
    assert!(b.distance_to_point(&Vector3::new(0.0, 0.0, 0.0)) == 0.0);
    assert!(b.distance_to_point(&Vector3::new(-1.0, -1.0, -1.0)) == 0.0);
    assert!(b.distance_to_point(&Vector3::new(-2.0, -2.0, -2.0)) == 3.0_f64.sqrt());
}

#[test]
fn get_bounding_sphere() {
    let a = Box3::new(ZERO3, ZERO3);
    let b = Box3::new(ZERO3, ONE3);
    let c = Box3::new(negated(ONE3), ONE3);

    assert!(a.get_bounding_sphere().equals(&Sphere::new(ZERO3, 0.0)));
    let mut half = ONE3;
    assert!(b.get_bounding_sphere().equals(&Sphere::new(
        *half.multiply_scalar(0.5),
        3.0_f64.sqrt() * 0.5
    )));
    assert!(c
        .get_bounding_sphere()
        .equals(&Sphere::new(ZERO3, 12.0_f64.sqrt() * 0.5)));

    let mut d = Box3::default();
    d.make_empty();
    assert!(
        d.get_bounding_sphere().is_empty(),
        "Empty box's bounding sphere is empty"
    );
}

#[test]
fn intersect() {
    let a = Box3::new(ZERO3, ZERO3);
    let b = Box3::new(ZERO3, ONE3);
    let c = Box3::new(negated(ONE3), ONE3);

    assert!({ a }.intersect(&a).equals(&a));
    assert!({ a }.intersect(&b).equals(&a));
    assert!({ b }.intersect(&b).equals(&b));
    assert!({ a }.intersect(&c).equals(&a));
    assert!({ b }.intersect(&c).equals(&b));
    assert!({ c }.intersect(&c).equals(&c));
}

#[test]
fn union() {
    let a = Box3::new(ZERO3, ZERO3);
    let b = Box3::new(ZERO3, ONE3);
    let c = Box3::new(negated(ONE3), ONE3);

    assert!({ a }.union(&a).equals(&a));
    assert!({ a }.union(&b).equals(&b));
    assert!({ a }.union(&c).equals(&c));
    assert!({ b }.union(&c).equals(&c));
}

#[test]
fn apply_matrix4() {
    let a = Box3::new(ZERO3, ZERO3);
    let b = Box3::new(ZERO3, ONE3);
    let c = Box3::new(negated(ONE3), ONE3);
    let d = Box3::new(negated(ONE3), ZERO3);

    let mut m = Matrix4::default();
    let m = *m.make_translation(1.0, -2.0, 1.0);
    let t1 = Vector3::new(1.0, -2.0, 1.0);

    assert!(compare_box(
        { a }.apply_matrix4(&m),
        { a }.translate(&t1),
        None
    ));
    assert!(compare_box(
        { b }.apply_matrix4(&m),
        { b }.translate(&t1),
        None
    ));
    assert!(compare_box(
        { c }.apply_matrix4(&m),
        { c }.translate(&t1),
        None
    ));
    assert!(compare_box(
        { d }.apply_matrix4(&m),
        { d }.translate(&t1),
        None
    ));
}

#[test]
fn translate() {
    let a = Box3::new(ZERO3, ZERO3);
    let b = Box3::new(ZERO3, ONE3);
    let c = Box3::new(negated(ONE3), ZERO3);

    assert!({ a }.translate(&ONE3).equals(&Box3::new(ONE3, ONE3)));
    assert!({ a }.translate(&ONE3).translate(&negated(ONE3)).equals(&a));
    assert!({ c }.translate(&ONE3).equals(&b));
    assert!({ b }.translate(&negated(ONE3)).equals(&c));
}

#[test]
fn equals() {
    let a = Box3::default();
    let b = Box3::default();
    assert!(b.equals(&a));
    assert!(a.equals(&b));

    let a = Box3::new(ONE3, TWO3);
    let b = Box3::new(ONE3, TWO3);
    assert!(b.equals(&a));
    assert!(a.equals(&b));

    let a = Box3::new(ONE3, TWO3);
    let b = a;
    assert!(b.equals(&a));
    assert!(a.equals(&b));

    let a = Box3::new(ONE3, TWO3);
    let b = Box3::new(ONE3, ONE3);
    assert!(!b.equals(&a));
    assert!(!a.equals(&b));

    let a = Box3::default();
    let b = Box3::new(ONE3, ONE3);
    assert!(!b.equals(&a));
    assert!(!a.equals(&b));
}

// `setFromObject` / `expandByObject`. three's own cases for these build a
// `SkinnedMesh` and a `Mesh` from `BoxGeometry`, neither of which the port has
// as a test fixture, so the scenes below are the same shapes written out by
// hand: a box of unit half-extent for the translation cases, and a triangle for
// the case that tells `precise` apart from the conservative path.

/// A geometry whose position attribute is the 8 corners of `[-1, 1]^3`, so its
/// bounding box is exactly that cube.
fn unit_cube_geometry() -> Rc<BufferGeometry> {
    let mut positions = Vec::new();
    for &x in &[-1.0_f32, 1.0] {
        for &y in &[-1.0_f32, 1.0] {
            for &z in &[-1.0_f32, 1.0] {
                positions.extend_from_slice(&[x, y, z]);
            }
        }
    }

    let mut geometry = BufferGeometry::new();
    geometry.set_attribute("position", BufferAttribute::new(positions, 3));
    Rc::new(geometry)
}

/// A parent holding two unit cubes translated to `(-2, 0, 0)` and `(3, 1, 0)`.
fn two_cubes() -> Node {
    let parent = Group::new();

    let geometry = unit_cube_geometry();

    let left = Mesh::new(geometry.clone(), None);
    left.borrow_mut().position.set(-2.0, 0.0, 0.0);

    let right = Mesh::new(geometry, None);
    right.borrow_mut().position.set(3.0, 1.0, 0.0);

    parent.add(&left);
    parent.add(&right);
    parent
}

#[test]
fn set_from_object() {
    let parent = two_cubes();

    // The union of [-3,-1]x[-1,1]x[-1,1] and [2,4]x[0,2]x[-1,1].
    let expected = Box3::new(Vector3::new(-3.0, -1.0, -1.0), Vector3::new(4.0, 2.0, 1.0));

    let mut a = Box3::default();
    a.set_from_object(&parent, false);
    assert!(compare_box(&a, &expected, None), "conservative: {a:?}");

    let mut b = Box3::default();
    b.set_from_object(&parent, true);
    assert!(compare_box(&b, &expected, None), "precise: {b:?}");

    // The parent's own transform is part of it.
    parent.borrow_mut().position.set(0.0, 0.0, 5.0);
    let mut c = Box3::default();
    c.set_from_object(&parent, false);
    let mut expected = expected;
    expected.translate(&Vector3::new(0.0, 0.0, 5.0));
    assert!(compare_box(&c, &expected, None), "translated parent: {c:?}");
}

#[test]
fn set_from_object_empty() {
    let mut a = Box3::new(ONE3, TWO3);
    a.set_from_object(&Group::new(), false);
    assert!(a.is_empty(), "a group with no geometry leaves an empty box");

    let mut b = Box3::new(ONE3, TWO3);
    b.set_from_object(&Group::new(), true);
    assert!(b.is_empty(), "and the precise path agrees");
}

#[test]
fn set_from_object_precise_is_tighter() {
    // A triangle in the x/y plane, whose own bounding box is much bigger than
    // the triangle: rotated 45 degrees about z, the corners of that box sweep
    // wider than the vertices do, so the two paths disagree.
    let mut geometry = BufferGeometry::new();
    geometry.set_attribute(
        "position",
        BufferAttribute::new(vec![-1.0, -1.0, 0.0, 1.0, -1.0, 0.0, 0.0, 1.0, 0.0], 3),
    );

    let mesh = Mesh::new(Rc::new(geometry), None);
    mesh.borrow_mut().rotate_z(std::f64::consts::FRAC_PI_4);

    let mut conservative = Box3::default();
    conservative.set_from_object(&mesh, false);

    let mut precise = Box3::default();
    precise.set_from_object(&mesh, true);

    // The rotated vertices, by hand: (0, -sqrt(2)), (sqrt(2), 0),
    // (-sqrt(2)/2, sqrt(2)/2).
    let root2 = std::f64::consts::SQRT_2;
    let expected = Box3::new(
        Vector3::new(-root2 / 2.0, -root2, 0.0),
        Vector3::new(root2, root2 / 2.0, 0.0),
    );
    assert!(
        compare_box(&precise, &expected, None),
        "precise: {precise:?}"
    );

    // The conservative path rotates the axis-aligned box [-1,1]x[-1,1] instead,
    // whose corners reach sqrt(2) on both axes.
    let expected = Box3::new(
        Vector3::new(-root2, -root2, 0.0),
        Vector3::new(root2, root2, 0.0),
    );
    assert!(
        compare_box(&conservative, &expected, None),
        "conservative: {conservative:?}"
    );
}
