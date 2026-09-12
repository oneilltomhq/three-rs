//! Port of `three.js/test/unit/src/math/Box2.tests.js`.
//!
//! Expectations and epsilons are three.js' own; nothing here was recomputed
//! from the Rust implementation.

mod support;

use three_rs::math::{Box2, Vector2};

/// `math-constants.js` `negInf2`.
const NEG_INF2: Vector2 = Vector2::new(f64::NEG_INFINITY, f64::NEG_INFINITY);
/// `math-constants.js` `posInf2`.
const POS_INF2: Vector2 = Vector2::new(f64::INFINITY, f64::INFINITY);
/// `math-constants.js` `negOne2`.
const NEG_ONE2: Vector2 = Vector2::new(-1.0, -1.0);
/// `math-constants.js` `zero2`.
const ZERO2: Vector2 = Vector2::new(0.0, 0.0);
/// `math-constants.js` `one2`.
const ONE2: Vector2 = Vector2::new(1.0, 1.0);
/// `math-constants.js` `two2`.
const TWO2: Vector2 = Vector2::new(2.0, 2.0);

// INSTANCING
#[test]
fn instancing() {
    let a = Box2::default();
    assert!(a.min.equals(&POS_INF2));
    assert!(a.max.equals(&NEG_INF2));

    let a = Box2::new(ZERO2, ZERO2);
    assert!(a.min.equals(&ZERO2));
    assert!(a.max.equals(&ZERO2));

    let a = Box2::new(ZERO2, ONE2);
    assert!(a.min.equals(&ZERO2));
    assert!(a.max.equals(&ONE2));
}

// PUBLIC STUFF
#[test]
fn is_box2() {
    assert!(Box2::IS_BOX2);
}

#[test]
fn set() {
    let mut a = Box2::default();

    a.set(&ZERO2, &ONE2);
    assert!(a.min.equals(&ZERO2));
    assert!(a.max.equals(&ONE2));
}

#[test]
fn set_from_points() {
    let mut a = Box2::default();

    a.set_from_points(&[ZERO2, ONE2, TWO2]);
    assert!(a.min.equals(&ZERO2));
    assert!(a.max.equals(&TWO2));

    a.set_from_points(&[ONE2]);
    assert!(a.min.equals(&ONE2));
    assert!(a.max.equals(&ONE2));

    a.set_from_points(&[]);
    assert!(a.is_empty());
}

#[test]
fn set_from_center_and_size() {
    let mut a = Box2::default();

    a.set_from_center_and_size(&ZERO2, &TWO2);
    assert!(a.min.equals(&NEG_ONE2));
    assert!(a.max.equals(&ONE2));

    a.set_from_center_and_size(&ONE2, &TWO2);
    assert!(a.min.equals(&ZERO2));
    assert!(a.max.equals(&TWO2));

    a.set_from_center_and_size(&ZERO2, &ZERO2);
    assert!(a.min.equals(&ZERO2));
    assert!(a.max.equals(&ZERO2));
}

#[test]
fn clone() {
    let a = Box2::new(ZERO2, ZERO2);

    let b = a;
    assert!(b.min.equals(&ZERO2));
    assert!(b.max.equals(&ZERO2));

    let a = Box2::default();
    let b = a;
    assert!(b.min.equals(&POS_INF2));
    assert!(b.max.equals(&NEG_INF2));
}

#[test]
fn copy() {
    let mut a = Box2::new(ZERO2, ONE2);
    let mut b = Box2::default();
    b.copy(&a);
    assert!(b.min.equals(&ZERO2));
    assert!(b.max.equals(&ONE2));

    // ensure that it is a true copy
    a.min = ZERO2;
    a.max = ONE2;
    assert!(b.min.equals(&ZERO2));
    assert!(b.max.equals(&ONE2));
}

#[test]
fn empty_make_empty() {
    let a = Box2::default();

    assert!(a.is_empty());

    let mut a = Box2::new(ZERO2, ONE2);
    assert!(!a.is_empty());

    a.make_empty();
    assert!(a.is_empty());
}

#[test]
fn is_empty() {
    let a = Box2::new(ZERO2, ZERO2);
    assert!(!a.is_empty());

    let a = Box2::new(ZERO2, ONE2);
    assert!(!a.is_empty());

    let a = Box2::new(TWO2, ONE2);
    assert!(a.is_empty());

    let a = Box2::new(POS_INF2, NEG_INF2);
    assert!(a.is_empty());
}

#[test]
fn get_center() {
    let a = Box2::new(ZERO2, ZERO2);
    assert!(a.get_center().equals(&ZERO2));

    let a = Box2::new(ZERO2, ONE2);
    let midpoint = *ONE2.clone().multiply_scalar(0.5);
    assert!(a.get_center().equals(&midpoint));
}

#[test]
fn get_size() {
    let a = Box2::new(ZERO2, ZERO2);

    assert!(a.get_size().equals(&ZERO2));

    let a = Box2::new(ZERO2, ONE2);
    assert!(a.get_size().equals(&ONE2));
}

#[test]
fn expand_by_point() {
    let mut a = Box2::new(ZERO2, ZERO2);

    a.expand_by_point(&ZERO2);
    assert!(a.get_size().equals(&ZERO2));

    a.expand_by_point(&ONE2);
    assert!(a.get_size().equals(&ONE2));

    a.expand_by_point(ONE2.clone().negate());
    assert!(a.get_size().equals(ONE2.clone().multiply_scalar(2.0)));
    assert!(a.get_center().equals(&ZERO2));
}

#[test]
fn expand_by_vector() {
    let mut a = Box2::new(ZERO2, ZERO2);

    a.expand_by_vector(&ZERO2);
    assert!(a.get_size().equals(&ZERO2));

    a.expand_by_vector(&ONE2);
    assert!(a.get_size().equals(ONE2.clone().multiply_scalar(2.0)));
    assert!(a.get_center().equals(&ZERO2));
}

#[test]
fn expand_by_scalar() {
    let mut a = Box2::new(ZERO2, ZERO2);

    a.expand_by_scalar(0.0);
    assert!(a.get_size().equals(&ZERO2));

    a.expand_by_scalar(1.0);
    assert!(a.get_size().equals(ONE2.clone().multiply_scalar(2.0)));
    assert!(a.get_center().equals(&ZERO2));
}

#[test]
fn contains_point() {
    let mut a = Box2::new(ZERO2, ZERO2);

    assert!(a.contains_point(&ZERO2));
    assert!(!a.contains_point(&ONE2));

    a.expand_by_scalar(1.0);
    assert!(a.contains_point(&ZERO2));
    assert!(a.contains_point(&ONE2));
    assert!(a.contains_point(ONE2.clone().negate()));
}

#[test]
fn contains_box() {
    let a = Box2::new(ZERO2, ZERO2);
    let b = Box2::new(ZERO2, ONE2);
    let c = Box2::new(*ONE2.clone().negate(), ONE2);

    assert!(a.contains_box(&a));
    assert!(!a.contains_box(&b));
    assert!(!a.contains_box(&c));

    assert!(b.contains_box(&a));
    assert!(c.contains_box(&a));
    assert!(!b.contains_box(&c));
}

#[test]
fn get_parameter() {
    let a = Box2::new(ZERO2, ONE2);
    let b = Box2::new(*ONE2.clone().negate(), ONE2);

    let parameter = a.get_parameter(&ZERO2);
    assert!(parameter.equals(&ZERO2));
    let parameter = a.get_parameter(&ONE2);
    assert!(parameter.equals(&ONE2));

    let parameter = b.get_parameter(ONE2.clone().negate());
    assert!(parameter.equals(&ZERO2));
    let parameter = b.get_parameter(&ZERO2);
    assert!(parameter.equals(&Vector2::new(0.5, 0.5)));
    let parameter = b.get_parameter(&ONE2);
    assert!(parameter.equals(&ONE2));
}

#[test]
fn intersects_box() {
    let a = Box2::new(ZERO2, ZERO2);
    let mut b = Box2::new(ZERO2, ONE2);
    let c = Box2::new(*ONE2.clone().negate(), ONE2);

    assert!(a.intersects_box(&a));
    assert!(a.intersects_box(&b));
    assert!(a.intersects_box(&c));

    assert!(b.intersects_box(&a));
    assert!(c.intersects_box(&a));
    assert!(b.intersects_box(&c));

    b.translate(&TWO2);
    assert!(!a.intersects_box(&b));
    assert!(!b.intersects_box(&a));
    assert!(!b.intersects_box(&c));
}

#[test]
fn clamp_point() {
    let a = Box2::new(ZERO2, ZERO2);
    let b = Box2::new(*ONE2.clone().negate(), ONE2);

    let point = a.clamp_point(&ZERO2);
    assert!(point.equals(&Vector2::new(0.0, 0.0)));
    let point = a.clamp_point(&ONE2);
    assert!(point.equals(&Vector2::new(0.0, 0.0)));
    let point = a.clamp_point(ONE2.clone().negate());
    assert!(point.equals(&Vector2::new(0.0, 0.0)));

    let point = b.clamp_point(&TWO2);
    assert!(point.equals(&Vector2::new(1.0, 1.0)));
    let point = b.clamp_point(&ONE2);
    assert!(point.equals(&Vector2::new(1.0, 1.0)));
    let point = b.clamp_point(&ZERO2);
    assert!(point.equals(&Vector2::new(0.0, 0.0)));
    let point = b.clamp_point(ONE2.clone().negate());
    assert!(point.equals(&Vector2::new(-1.0, -1.0)));
    let point = b.clamp_point(TWO2.clone().negate());
    assert!(point.equals(&Vector2::new(-1.0, -1.0)));
}

#[test]
fn distance_to_point() {
    let a = Box2::new(ZERO2, ZERO2);
    let b = Box2::new(*ONE2.clone().negate(), ONE2);

    assert!(a.distance_to_point(&Vector2::new(0.0, 0.0)) == 0.0);
    assert!(a.distance_to_point(&Vector2::new(1.0, 1.0)) == 2.0_f64.sqrt());
    assert!(a.distance_to_point(&Vector2::new(-1.0, -1.0)) == 2.0_f64.sqrt());

    assert!(b.distance_to_point(&Vector2::new(2.0, 2.0)) == 2.0_f64.sqrt());
    assert!(b.distance_to_point(&Vector2::new(1.0, 1.0)) == 0.0);
    assert!(b.distance_to_point(&Vector2::new(0.0, 0.0)) == 0.0);
    assert!(b.distance_to_point(&Vector2::new(-1.0, -1.0)) == 0.0);
    assert!(b.distance_to_point(&Vector2::new(-2.0, -2.0)) == 2.0_f64.sqrt());
}

#[test]
fn intersect() {
    let a = Box2::new(ZERO2, ZERO2);
    let b = Box2::new(ZERO2, ONE2);
    let c = Box2::new(*ONE2.clone().negate(), ONE2);

    assert!(a.clone().intersect(&a).equals(&a));
    assert!(a.clone().intersect(&b).equals(&a));
    assert!(b.clone().intersect(&b).equals(&b));
    assert!(a.clone().intersect(&c).equals(&a));
    assert!(b.clone().intersect(&c).equals(&b));
    assert!(c.clone().intersect(&c).equals(&c));

    let d = Box2::new(*ONE2.clone().negate(), ZERO2);
    let e = *Box2::new(ONE2, TWO2).intersect(&d);

    assert!(
        e.min.equals(&POS_INF2) && e.max.equals(&NEG_INF2),
        "Infinite empty"
    );
}

#[test]
fn union() {
    let a = Box2::new(ZERO2, ZERO2);
    let b = Box2::new(ZERO2, ONE2);
    let c = Box2::new(*ONE2.clone().negate(), ONE2);

    assert!(a.clone().union(&a).equals(&a));
    assert!(a.clone().union(&b).equals(&b));
    assert!(a.clone().union(&c).equals(&c));
    assert!(b.clone().union(&c).equals(&c));
}

#[test]
fn translate() {
    let a = Box2::new(ZERO2, ZERO2);
    let b = Box2::new(ZERO2, ONE2);
    let c = Box2::new(*ONE2.clone().negate(), ZERO2);

    assert!(a.clone().translate(&ONE2).equals(&Box2::new(ONE2, ONE2)));
    assert!(
        a.clone()
            .translate(&ONE2)
            .translate(ONE2.clone().negate())
            .equals(&a)
    );
    assert!(c.clone().translate(&ONE2).equals(&b));
    assert!(b.clone().translate(ONE2.clone().negate()).equals(&c));
}

#[test]
fn equals() {
    let a = Box2::default();
    let b = Box2::default();
    assert!(b.equals(&a));
    assert!(a.equals(&b));

    let a = Box2::new(ONE2, TWO2);
    let b = Box2::new(ONE2, TWO2);
    assert!(b.equals(&a));
    assert!(a.equals(&b));

    let a = Box2::new(ONE2, TWO2);
    let b = a;
    assert!(b.equals(&a));
    assert!(a.equals(&b));

    let a = Box2::new(ONE2, TWO2);
    let b = Box2::new(ONE2, ONE2);
    assert!(!b.equals(&a));
    assert!(!a.equals(&b));

    let a = Box2::default();
    let b = Box2::new(ONE2, ONE2);
    assert!(!b.equals(&a));
    assert!(!a.equals(&b));

    let a = Box2::new(ONE2, TWO2);
    let b = Box2::new(ONE2, ONE2);
    assert!(!b.equals(&a));
    assert!(!a.equals(&b));
}
