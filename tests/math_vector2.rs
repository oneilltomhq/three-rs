//! Port of `three.js/test/unit/src/math/Vector2.tests.js`.

mod support;

use support::{close, EPS, X, Y};
use three_rs::math::{Matrix3, Vector2};

#[test]
fn instancing() {
    let a = Vector2::default();
    assert_eq!(a.x, 0.0);
    assert_eq!(a.y, 0.0);

    let a = Vector2::new(X, Y);
    assert_eq!(a.x, X);
    assert_eq!(a.y, Y);
}

#[test]
fn properties() {
    // `width` / `height` are `Vector2`'s aliases for `x` / `y`.
    let mut a = Vector2::new(0.0, 0.0);
    let width = 100.0;
    let height = 200.0;

    a.set(width, height);

    assert_eq!(a.width(), width);
    assert_eq!(a.height(), height);
}

#[test]
fn set() {
    let mut a = Vector2::default();
    assert_eq!((a.x, a.y), (0.0, 0.0));

    a.set(X, Y);
    assert_eq!((a.x, a.y), (X, Y));
}

#[test]
fn copy() {
    let mut a = Vector2::new(X, Y);
    let mut b = Vector2::default();
    b.copy(&a);
    assert_eq!((b.x, b.y), (X, Y));

    a.x = 0.0;
    a.y = -1.0;
    assert_eq!((b.x, b.y), (X, Y));
}

#[test]
fn add() {
    let mut a = Vector2::new(X, Y);
    let b = Vector2::new(-X, -Y);

    a.add(&b);
    assert_eq!((a.x, a.y), (0.0, 0.0));

    let mut c = Vector2::default();
    c.add_vectors(&b, &b);
    assert_eq!((c.x, c.y), (-2.0 * X, -2.0 * Y));
}

#[test]
fn add_scaled_vector() {
    let mut a = Vector2::new(X, Y);
    let b = Vector2::new(2.0, 3.0);
    let s = 3.0;

    a.add_scaled_vector(&b, s);
    assert_eq!(a.x, X + b.x * s);
    assert_eq!(a.y, Y + b.y * s);
}

#[test]
fn sub() {
    let mut a = Vector2::new(X, Y);
    let b = Vector2::new(-X, -Y);

    a.sub(&b);
    assert_eq!((a.x, a.y), (2.0 * X, 2.0 * Y));

    let mut c = Vector2::default();
    c.sub_vectors(&a, &a);
    assert_eq!((c.x, c.y), (0.0, 0.0));
}

#[test]
fn apply_matrix3() {
    let mut a = Vector2::new(X, Y);
    let mut m = Matrix3::identity();
    m.set(2.0, 3.0, 5.0, 7.0, 11.0, 13.0, 17.0, 19.0, 23.0);

    a.apply_matrix3(&m);

    assert_eq!(a.x, 18.0);
    assert_eq!(a.y, 60.0);
}

#[test]
fn negate() {
    let mut a = Vector2::new(X, Y);
    a.negate();
    assert_eq!((a.x, a.y), (-X, -Y));
}

#[test]
fn dot() {
    let a = Vector2::new(X, Y);
    let b = Vector2::new(-X, -Y);
    let c = Vector2::default();

    assert_eq!(a.dot(&b), -X * X - Y * Y);
    assert_eq!(a.dot(&c), 0.0);
}

#[test]
fn cross() {
    let a = Vector2::new(X, Y);
    let b = Vector2::new(2.0 * X, -Y);

    close(a.cross(&b), -18.0, EPS, "Check cross");
}

#[test]
fn manhattan_length() {
    let mut a = Vector2::new(X, 0.0);
    let b = Vector2::new(0.0, -Y);
    let c = Vector2::default();

    assert_eq!(a.manhattan_length(), X, "Positive x");
    assert_eq!(b.manhattan_length(), Y, "Negative y");
    assert_eq!(c.manhattan_length(), 0.0, "Empty initialization");

    a.set(X, Y);
    assert_eq!(a.manhattan_length(), X.abs() + Y.abs(), "All components");
}

#[test]
fn normalize() {
    let mut a = Vector2::new(X, 0.0);
    let mut b = Vector2::new(0.0, -Y);

    a.normalize();
    assert_eq!(a.length(), 1.0);
    assert_eq!(a.x, 1.0);

    b.normalize();
    assert_eq!(b.length(), 1.0);
    assert_eq!(b.y, -1.0);
}

#[test]
fn angle_to() {
    let a = Vector2::new(-0.18851655680720186, 0.9820700116639124);
    let b = Vector2::new(0.18851655680720186, -0.9820700116639124);

    assert_eq!(a.angle_to(&a), 0.0);
    assert_eq!(a.angle_to(&b), std::f64::consts::PI);

    let x = Vector2::new(1.0, 0.0);
    let y = Vector2::new(0.0, 1.0);

    assert_eq!(x.angle_to(&y), std::f64::consts::PI / 2.0);
    assert_eq!(y.angle_to(&x), std::f64::consts::PI / 2.0);

    assert!((x.angle_to(&Vector2::new(1.0, 1.0)) - std::f64::consts::PI / 4.0).abs() < 0.0000001);
}

#[test]
fn set_length() {
    let mut a = Vector2::new(X, 0.0);
    assert_eq!(a.length(), X);
    a.set_length(Y);
    assert_eq!(a.length(), Y);

    let mut a = Vector2::new(0.0, 0.0);
    assert_eq!(a.length(), 0.0);
    a.set_length(Y);
    assert_eq!(a.length(), 0.0);

    a.set_length(f64::NAN);
    assert!(a.length().is_nan());
}

#[test]
fn equals() {
    let mut a = Vector2::new(X, 0.0);
    let b = Vector2::new(0.0, -Y);

    assert_ne!(a.x, b.x);
    assert_ne!(a.y, b.y);
    assert!(!a.equals(&b));
    assert!(!b.equals(&a));

    a.copy(&b);
    assert_eq!(a.x, b.x);
    assert_eq!(a.y, b.y);
    assert!(a.equals(&b));
    assert!(b.equals(&a));
}

#[test]
fn from_array() {
    let mut a = Vector2::default();
    let array = [1.0, 2.0, 3.0, 4.0];

    a.from_array(&array, 0);
    assert_eq!((a.x, a.y), (1.0, 2.0));

    a.from_array(&array, 2);
    assert_eq!((a.x, a.y), (3.0, 4.0));
}

#[test]
fn to_array() {
    let a = Vector2::new(X, Y);
    let array = a.to_array();
    assert_eq!(array[0], X);
    assert_eq!(array[1], Y);
}

#[test]
fn from_buffer_attribute() {
    use three_rs::core::BufferAttribute;

    let attr = BufferAttribute::new(vec![1.0, 2.0, 3.0, 4.0], 2);

    assert_eq!(attr.get_x(0), 1.0);
    assert_eq!(attr.get_y(0), 2.0);
    assert_eq!(attr.get_x(1), 3.0);
    assert_eq!(attr.get_y(1), 4.0);
}

#[test]
fn set_x_set_y() {
    let mut a = Vector2::default();
    assert_eq!((a.x, a.y), (0.0, 0.0));

    a.set_x(X);
    a.set_y(Y);
    assert_eq!((a.x, a.y), (X, Y));
}

#[test]
fn set_component_get_component() {
    let mut a = Vector2::default();
    assert_eq!((a.x, a.y), (0.0, 0.0));

    a.set_component(0, 1.0);
    a.set_component(1, 2.0);

    assert_eq!(a.get_component(0), 1.0);
    assert_eq!(a.get_component(1), 2.0);
}

#[test]
#[should_panic(expected = "index is out of range")]
fn set_component_out_of_range_panics() {
    Vector2::default().set_component(2, 0.0);
}

#[test]
#[should_panic(expected = "index is out of range")]
fn get_component_out_of_range_panics() {
    Vector2::default().get_component(2);
}

#[test]
fn multiply_divide() {
    let mut a = Vector2::new(X, Y);
    let mut b = Vector2::new(2.0 * X, 2.0 * Y);
    let c = Vector2::new(4.0 * X, 4.0 * Y);

    let b_before = b;
    a.multiply(&b);
    assert_eq!(a.x, X * b_before.x);
    assert_eq!(a.y, Y * b_before.y);

    b.divide(&c);
    close(b.x, 0.5, EPS, "divide: check x");
    close(b.y, 0.5, EPS, "divide: check y");
}

#[test]
fn multiply_divide_scalar() {
    let mut a = Vector2::new(X, Y);
    let mut b = Vector2::new(-X, -Y);

    a.multiply_scalar(-2.0);
    assert_eq!((a.x, a.y), (X * -2.0, Y * -2.0));

    b.multiply_scalar(-2.0);
    assert_eq!((b.x, b.y), (2.0 * X, 2.0 * Y));

    a.divide_scalar(-2.0);
    assert_eq!((a.x, a.y), (X, Y));

    b.divide_scalar(-2.0);
    assert_eq!((b.x, b.y), (-X, -Y));
}

#[test]
fn min_max_clamp() {
    let a = Vector2::new(X, Y);
    let b = Vector2::new(-X, -Y);
    let mut c = Vector2::default();

    c.copy(&a).min(&b);
    assert_eq!((c.x, c.y), (-X, -Y));

    c.copy(&a).max(&b);
    assert_eq!((c.x, c.y), (X, Y));

    c.set(-2.0 * X, 2.0 * Y);
    c.clamp(&b, &a);
    assert_eq!((c.x, c.y), (-X, Y));
}

#[test]
fn rounding() {
    let round_trip = |v: Vector2, f: fn(&mut Vector2) -> &mut Vector2| {
        let mut v = v;
        f(&mut v);
        v
    };

    assert_eq!(
        round_trip(Vector2::new(-0.1, 0.1), Vector2::floor),
        Vector2::new(-1.0, 0.0)
    );
    assert_eq!(
        round_trip(Vector2::new(-0.5, 0.5), Vector2::floor),
        Vector2::new(-1.0, 0.0)
    );
    assert_eq!(
        round_trip(Vector2::new(-0.9, 0.9), Vector2::floor),
        Vector2::new(-1.0, 0.0)
    );

    assert_eq!(
        round_trip(Vector2::new(-0.1, 0.1), Vector2::ceil),
        Vector2::new(0.0, 1.0)
    );
    assert_eq!(
        round_trip(Vector2::new(-0.5, 0.5), Vector2::ceil),
        Vector2::new(0.0, 1.0)
    );
    assert_eq!(
        round_trip(Vector2::new(-0.9, 0.9), Vector2::ceil),
        Vector2::new(0.0, 1.0)
    );

    assert_eq!(
        round_trip(Vector2::new(-0.1, 0.1), Vector2::round),
        Vector2::new(0.0, 0.0)
    );
    assert_eq!(
        round_trip(Vector2::new(-0.5, 0.5), Vector2::round),
        Vector2::new(0.0, 1.0)
    );
    assert_eq!(
        round_trip(Vector2::new(-0.9, 0.9), Vector2::round),
        Vector2::new(-1.0, 1.0)
    );

    assert_eq!(
        round_trip(Vector2::new(-0.1, 0.1), Vector2::round_to_zero),
        Vector2::new(0.0, 0.0)
    );
    assert_eq!(
        round_trip(Vector2::new(-0.5, 0.5), Vector2::round_to_zero),
        Vector2::new(0.0, 0.0)
    );
    assert_eq!(
        round_trip(Vector2::new(-0.9, 0.9), Vector2::round_to_zero),
        Vector2::new(0.0, 0.0)
    );
    assert_eq!(
        round_trip(Vector2::new(-1.1, 1.1), Vector2::round_to_zero),
        Vector2::new(-1.0, 1.0)
    );
    assert_eq!(
        round_trip(Vector2::new(-1.5, 1.5), Vector2::round_to_zero),
        Vector2::new(-1.0, 1.0)
    );
    assert_eq!(
        round_trip(Vector2::new(-1.9, 1.9), Vector2::round_to_zero),
        Vector2::new(-1.0, 1.0)
    );
}

#[test]
fn length_length_sq() {
    let mut a = Vector2::new(X, 0.0);
    let b = Vector2::new(0.0, -Y);
    let c = Vector2::default();

    assert_eq!(a.length(), X);
    assert_eq!(a.length_sq(), X * X);
    assert_eq!(b.length(), Y);
    assert_eq!(b.length_sq(), Y * Y);
    assert_eq!(c.length(), 0.0);
    assert_eq!(c.length_sq(), 0.0);

    a.set(X, Y);
    assert_eq!(a.length(), (X * X + Y * Y).sqrt());
    assert_eq!(a.length_sq(), X * X + Y * Y);
}

#[test]
fn distance_to_distance_to_squared() {
    let a = Vector2::new(X, 0.0);
    let b = Vector2::new(0.0, -Y);
    let c = Vector2::default();

    assert_eq!(a.distance_to(&c), X);
    assert_eq!(a.distance_to_squared(&c), X * X);
    assert_eq!(b.distance_to(&c), Y);
    assert_eq!(b.distance_to_squared(&c), Y * Y);
}

#[test]
fn lerp_clone() {
    let a = Vector2::new(X, 0.0);
    let b = Vector2::new(0.0, -Y);

    let mut at0 = a;
    at0.lerp(&a, 0.0);
    let mut at_half = a;
    at_half.lerp(&a, 0.5);
    let mut at1 = a;
    at1.lerp(&a, 1.0);
    assert!(at0.equals(&at_half));
    assert!(at0.equals(&at1));

    let mut c = a;
    assert!(c.lerp(&b, 0.0).equals(&a));

    let mut c = a;
    c.lerp(&b, 0.5);
    assert_eq!(c.x, X * 0.5);
    assert_eq!(c.y, -Y * 0.5);

    let mut c = a;
    assert!(c.lerp(&b, 1.0).equals(&b));
}

#[test]
fn set_scalar_add_scalar_sub_scalar() {
    let mut a = Vector2::default();
    let s = 3.0;

    a.set_scalar(s);
    assert_eq!((a.x, a.y), (s, s));

    a.add_scalar(s);
    assert_eq!((a.x, a.y), (2.0 * s, 2.0 * s));

    a.sub_scalar(2.0 * s);
    assert_eq!((a.x, a.y), (0.0, 0.0));
}

#[test]
fn angle() {
    // `Vector2.angle()` measures counter-clockwise from +x, in [0, 2pi).
    close(Vector2::new(1.0, 0.0).angle(), 0.0, EPS, "+x");
    close(
        Vector2::new(0.0, 1.0).angle(),
        std::f64::consts::PI / 2.0,
        EPS,
        "+y",
    );
    close(
        Vector2::new(-1.0, 0.0).angle(),
        std::f64::consts::PI,
        EPS,
        "-x",
    );
}

#[test]
fn rotate_around() {
    let mut a = Vector2::new(1.0, 0.0);
    let center = Vector2::new(0.0, 0.0);

    a.rotate_around(&center, std::f64::consts::PI / 2.0);
    close(a.x, 0.0, EPS, "Check x");
    close(a.y, 1.0, EPS, "Check y");
}

#[test]
fn clamp_length() {
    let mut a = Vector2::new(X, 0.0);
    a.clamp_length(0.0, 1.0);
    close(a.length(), 1.0, EPS, "clamped down to max");

    let mut a = Vector2::new(0.1, 0.0);
    a.clamp_length(1.0, 10.0);
    close(a.length(), 1.0, EPS, "clamped up to min");
}

#[test]
fn lerp_vectors() {
    let v1 = Vector2::new(X, Y);
    let v2 = Vector2::new(2.0 * X, 2.0 * Y);
    let mut a = Vector2::default();

    a.lerp_vectors(&v1, &v2, 0.5);
    assert_eq!(a.x, 1.5 * X);
    assert_eq!(a.y, 1.5 * Y);
}
