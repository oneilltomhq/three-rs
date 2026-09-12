//! Port of `three.js/test/unit/src/math/Vector4.tests.js`.

mod support;

use support::{close, EPS, W, X, Y, Z};
use three_rs::math::{Matrix4, Vector4};

fn primes_matrix4() -> Matrix4 {
    Matrix4::from_rows(
        2.0, 3.0, 5.0, 7.0, 11.0, 13.0, 17.0, 19.0, 23.0, 29.0, 31.0, 37.0, 41.0, 43.0, 47.0, 53.0,
    )
}

#[test]
fn instancing() {
    let a = Vector4::default();
    assert_eq!(a.x, 0.0);
    assert_eq!(a.y, 0.0);
    assert_eq!(a.z, 0.0);
    assert_eq!(a.w, 1.0);

    let a = Vector4::new(X, Y, Z, W);
    assert_eq!(a.x, X);
    assert_eq!(a.y, Y);
    assert_eq!(a.z, Z);
    assert_eq!(a.w, W);
}

#[test]
fn set() {
    let mut a = Vector4::new(0.0, 0.0, 0.0, 0.0);
    assert_eq!((a.x, a.y, a.z, a.w), (0.0, 0.0, 0.0, 0.0));

    a.set(X, Y, Z, W);
    assert_eq!((a.x, a.y, a.z, a.w), (X, Y, Z, W));
}

#[test]
fn set_x_y_z_w() {
    let mut a = Vector4::new(0.0, 0.0, 0.0, 0.0);

    a.set_x(X);
    a.set_y(Y);
    a.set_z(Z);
    a.set_w(W);

    assert_eq!((a.x, a.y, a.z, a.w), (X, Y, Z, W));
}

#[test]
fn set_component_get_component() {
    let mut a = Vector4::new(0.0, 0.0, 0.0, 0.0);

    a.set_component(0, 1.0);
    a.set_component(1, 2.0);
    a.set_component(2, 3.0);
    a.set_component(3, 4.0);

    assert_eq!(a.get_component(0), 1.0);
    assert_eq!(a.get_component(1), 2.0);
    assert_eq!(a.get_component(2), 3.0);
    assert_eq!(a.get_component(3), 4.0);
}

#[test]
#[should_panic(expected = "index is out of range")]
fn set_component_out_of_range_panics() {
    Vector4::default().set_component(4, 0.0);
}

#[test]
#[should_panic(expected = "index is out of range")]
fn get_component_out_of_range_panics() {
    Vector4::default().get_component(4);
}

#[test]
fn copy() {
    let mut a = Vector4::new(X, Y, Z, W);
    let mut b = Vector4::new(0.0, 0.0, 0.0, 0.0);
    b.copy(&a);
    assert_eq!((b.x, b.y, b.z, b.w), (X, Y, Z, W));

    a.x = 0.0;
    a.y = -1.0;
    a.z = -2.0;
    a.w = -3.0;
    assert_eq!((b.x, b.y, b.z, b.w), (X, Y, Z, W));
}

#[test]
fn add() {
    let mut a = Vector4::new(X, Y, Z, W);
    let b = Vector4::new(-X, -Y, -Z, -W);

    a.add(&b);
    assert_eq!((a.x, a.y, a.z, a.w), (0.0, 0.0, 0.0, 0.0));

    let mut c = Vector4::new(0.0, 0.0, 0.0, 0.0);
    c.add_vectors(&b, &b);
    assert_eq!(
        (c.x, c.y, c.z, c.w),
        (-2.0 * X, -2.0 * Y, -2.0 * Z, -2.0 * W)
    );
}

#[test]
fn add_scalar() {
    let mut a = Vector4::new(X, Y, Z, W);
    let s = 3.0;

    a.add_scalar(s);
    assert_eq!((a.x, a.y, a.z, a.w), (X + s, Y + s, Z + s, W + s));
}

#[test]
fn add_scaled_vector() {
    let mut a = Vector4::new(X, Y, Z, W);
    let b = Vector4::new(2.0, 3.0, 4.0, 5.0);
    let s = 3.0;

    a.add_scaled_vector(&b, s);
    assert_eq!(a.x, X + b.x * s);
    assert_eq!(a.y, Y + b.y * s);
    assert_eq!(a.z, Z + b.z * s);
    assert_eq!(a.w, W + b.w * s);
}

#[test]
fn sub() {
    let mut a = Vector4::new(X, Y, Z, W);
    let b = Vector4::new(-X, -Y, -Z, -W);

    a.sub(&b);
    assert_eq!((a.x, a.y, a.z, a.w), (2.0 * X, 2.0 * Y, 2.0 * Z, 2.0 * W));

    let mut c = Vector4::new(0.0, 0.0, 0.0, 0.0);
    c.sub_vectors(&a, &a);
    assert_eq!((c.x, c.y, c.z, c.w), (0.0, 0.0, 0.0, 0.0));
}

#[test]
fn sub_scalar() {
    let mut a = Vector4::new(X, Y, Z, W);
    let s = 3.0;

    a.sub_scalar(s);
    assert_eq!((a.x, a.y, a.z, a.w), (X - s, Y - s, Z - s, W - s));
}

#[test]
fn apply_matrix4() {
    let a = Vector4::new(X, Y, Z, W);
    let m = *Matrix4::identity().make_rotation_x(std::f64::consts::PI);

    let mut v = a;
    v.apply_matrix4(&m);
    close_all4(&v, &[a.x, -a.y, -a.z, a.w], "rotation x");

    let mut v = a;
    let m = *Matrix4::identity().make_translation(5.0, 7.0, 11.0);
    v.apply_matrix4(&m);
    close_all4(&v, &[27.0, 38.0, 59.0, 5.0], "translation");

    let mut v = a;
    let mut m = Matrix4::identity();
    m.set(
        1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0,
    );
    v.apply_matrix4(&m);
    close_all4(&v, &[a.x, a.y, a.z, a.z], "w from z row");

    let mut v = a;
    v.apply_matrix4(&primes_matrix4());
    close_all4(&v, &[68.0, 224.0, 442.0, 664.0], "primes");
}

#[track_caller]
fn close_all4(v: &Vector4, expected: &[f64], what: &str) {
    close(v.x, expected[0], EPS, what);
    close(v.y, expected[1], EPS, what);
    close(v.z, expected[2], EPS, what);
    close(v.w, expected[3], EPS, what);
}

#[test]
fn multiply_scalar_divide_scalar() {
    let mut a = Vector4::new(X, Y, Z, W);
    let mut b = Vector4::new(-X, -Y, -Z, -W);

    a.multiply_scalar(-2.0);
    assert_eq!(
        (a.x, a.y, a.z, a.w),
        (X * -2.0, Y * -2.0, Z * -2.0, W * -2.0)
    );

    b.multiply_scalar(-2.0);
    assert_eq!((b.x, b.y, b.z, b.w), (2.0 * X, 2.0 * Y, 2.0 * Z, 2.0 * W));

    a.divide_scalar(-2.0);
    assert_eq!((a.x, a.y, a.z, a.w), (X, Y, Z, W));

    b.divide_scalar(-2.0);
    assert_eq!((b.x, b.y, b.z, b.w), (-X, -Y, -Z, -W));
}

#[test]
fn divide() {
    let mut a = Vector4::new(7.0, 8.0, 9.0, 0.0);
    let b = Vector4::new(2.0, 2.0, 3.0, 4.0);

    a.divide(&b);
    assert_eq!((a.x, a.y, a.z, a.w), (3.5, 4.0, 3.0, 0.0));
}

#[test]
fn set_from_matrix_position() {
    let mut a = Vector4::default();
    a.set_from_matrix_position(&primes_matrix4());
    assert_eq!(a.x, 7.0);
    assert_eq!(a.y, 19.0);
    assert_eq!(a.z, 37.0);
    assert_eq!(a.w, 53.0);
}

#[test]
fn min_max_clamp() {
    let a = Vector4::new(X, Y, Z, W);
    let b = Vector4::new(-X, -Y, -Z, -W);
    let mut c = Vector4::new(0.0, 0.0, 0.0, 0.0);

    c.copy(&a).min(&b);
    assert_eq!((c.x, c.y, c.z, c.w), (-X, -Y, -Z, -W));

    c.copy(&a).max(&b);
    assert_eq!((c.x, c.y, c.z, c.w), (X, Y, Z, W));

    c.set(-2.0 * X, 2.0 * Y, -2.0 * Z, 2.0 * W);
    c.clamp(&b, &a);
    assert_eq!((c.x, c.y, c.z, c.w), (-X, Y, -Z, W));
}

#[test]
fn clamp_scalar() {
    let mut a = Vector4::new(-0.1, 0.01, 0.5, 1.5);
    let clamped = Vector4::new(0.1, 0.1, 0.5, 1.0);

    a.clamp_scalar(0.1, 1.0);
    close(a.x, clamped.x, 0.001, "x");
    close(a.y, clamped.y, 0.001, "y");
    close(a.z, clamped.z, 0.001, "z");
    close(a.w, clamped.w, 0.001, "w");
}

#[test]
fn negate() {
    let mut a = Vector4::new(X, Y, Z, W);
    a.negate();
    assert_eq!((a.x, a.y, a.z, a.w), (-X, -Y, -Z, -W));
}

#[test]
fn dot() {
    let a = Vector4::new(X, Y, Z, W);
    let b = Vector4::new(-X, -Y, -Z, -W);
    let c = Vector4::new(0.0, 0.0, 0.0, 0.0);

    assert_eq!(a.dot(&b), -X * X - Y * Y - Z * Z - W * W);
    assert_eq!(a.dot(&c), 0.0);
}

#[test]
fn length_length_sq() {
    let mut a = Vector4::new(X, 0.0, 0.0, 0.0);
    let b = Vector4::new(0.0, -Y, 0.0, 0.0);
    let c = Vector4::new(0.0, 0.0, Z, 0.0);
    let d = Vector4::new(0.0, 0.0, 0.0, W);
    let e = Vector4::new(0.0, 0.0, 0.0, 0.0);

    assert_eq!(a.length(), X);
    assert_eq!(a.length_sq(), X * X);
    assert_eq!(b.length(), Y);
    assert_eq!(b.length_sq(), Y * Y);
    assert_eq!(c.length(), Z);
    assert_eq!(c.length_sq(), Z * Z);
    assert_eq!(d.length(), W);
    assert_eq!(d.length_sq(), W * W);
    assert_eq!(e.length(), 0.0);
    assert_eq!(e.length_sq(), 0.0);

    a.set(X, Y, Z, W);
    assert_eq!(a.length(), (X * X + Y * Y + Z * Z + W * W).sqrt());
    assert_eq!(a.length_sq(), X * X + Y * Y + Z * Z + W * W);
}

#[test]
fn manhattan_length() {
    let mut a = Vector4::new(X, 0.0, 0.0, 0.0);
    let b = Vector4::new(0.0, -Y, 0.0, 0.0);
    let c = Vector4::new(0.0, 0.0, Z, 0.0);
    let d = Vector4::new(0.0, 0.0, 0.0, W);
    let e = Vector4::new(0.0, 0.0, 0.0, 0.0);

    assert_eq!(a.manhattan_length(), X);
    assert_eq!(b.manhattan_length(), Y);
    assert_eq!(c.manhattan_length(), Z);
    assert_eq!(d.manhattan_length(), W);
    assert_eq!(e.manhattan_length(), 0.0);

    a.set(X, Y, Z, W);
    assert_eq!(
        a.manhattan_length(),
        X.abs() + Y.abs() + Z.abs() + W.abs()
    );
}

#[test]
fn normalize() {
    let mut a = Vector4::new(X, 0.0, 0.0, 0.0);
    let mut b = Vector4::new(0.0, -Y, 0.0, 0.0);
    let mut c = Vector4::new(0.0, 0.0, Z, 0.0);
    let mut d = Vector4::new(0.0, 0.0, 0.0, -W);

    a.normalize();
    assert_eq!(a.length(), 1.0);
    assert_eq!(a.x, 1.0);

    b.normalize();
    assert_eq!(b.length(), 1.0);
    assert_eq!(b.y, -1.0);

    c.normalize();
    assert_eq!(c.length(), 1.0);
    assert_eq!(c.z, 1.0);

    d.normalize();
    assert_eq!(d.length(), 1.0);
    assert_eq!(d.w, -1.0);
}

#[test]
fn set_length() {
    let mut a = Vector4::new(X, 0.0, 0.0, 0.0);
    assert_eq!(a.length(), X);
    a.set_length(Y);
    assert_eq!(a.length(), Y);

    let mut a = Vector4::new(0.0, 0.0, 0.0, 0.0);
    assert_eq!(a.length(), 0.0);
    a.set_length(Y);
    assert_eq!(a.length(), 0.0);

    a.set_length(f64::NAN);
    assert!(a.length().is_nan());
}

#[test]
fn lerp_clone() {
    let a = Vector4::new(X, 0.0, Z, 0.0);
    let b = Vector4::new(0.0, -Y, 0.0, -W);

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
    assert_eq!(c.z, Z * 0.5);
    assert_eq!(c.w, -W * 0.5);

    let mut c = a;
    assert!(c.lerp(&b, 1.0).equals(&b));
}

#[test]
fn lerp_vectors() {
    let v1 = Vector4::new(X, Y, Z, W);
    let v2 = Vector4::new(2.0 * X, 2.0 * Y, 2.0 * Z, 2.0 * W);
    let mut a = Vector4::default();

    a.lerp_vectors(&v1, &v2, 0.5);
    assert_eq!(a.x, 1.5 * X);
    assert_eq!(a.y, 1.5 * Y);
    assert_eq!(a.z, 1.5 * Z);
    assert_eq!(a.w, 1.5 * W);
}

#[test]
fn equals() {
    let mut a = Vector4::new(X, 0.0, Z, 0.0);
    let b = Vector4::new(0.0, -Y, 0.0, -W);

    assert_ne!(a.x, b.x);
    assert_ne!(a.y, b.y);
    assert_ne!(a.z, b.z);
    assert_ne!(a.w, b.w);
    assert!(!a.equals(&b));
    assert!(!b.equals(&a));

    a.copy(&b);
    assert!(a.equals(&b));
    assert!(b.equals(&a));
}

#[test]
fn from_array() {
    let mut a = Vector4::default();
    let array = [1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0];

    a.from_array(&array, 0);
    assert_eq!((a.x, a.y, a.z, a.w), (1.0, 2.0, 3.0, 4.0));

    a.from_array(&array, 4);
    assert_eq!((a.x, a.y, a.z, a.w), (5.0, 6.0, 7.0, 8.0));
}

#[test]
fn to_array() {
    let a = Vector4::new(X, Y, Z, W);
    let array = a.to_array();
    assert_eq!(array, [X, Y, Z, W]);
}

#[test]
fn set_scalar() {
    let mut a = Vector4::default();
    let s = 3.0;

    a.set_scalar(s);
    assert_eq!((a.x, a.y, a.z, a.w), (s, s, s, s));
}

#[test]
fn multiply() {
    let mut a = Vector4::new(X, Y, Z, W);
    let b = Vector4::new(2.0 * X, 2.0 * Y, 2.0 * Z, 2.0 * W);

    a.multiply(&b);
    assert_eq!(
        (a.x, a.y, a.z, a.w),
        (2.0 * X * X, 2.0 * Y * Y, 2.0 * Z * Z, 2.0 * W * W)
    );
}

#[test]
fn rounding() {
    let trip = |v: Vector4, f: fn(&mut Vector4) -> &mut Vector4| {
        let mut v = v;
        f(&mut v);
        v
    };

    assert_eq!(
        trip(Vector4::new(-0.1, 0.1, -0.1, 0.1), Vector4::floor),
        Vector4::new(-1.0, 0.0, -1.0, 0.0)
    );
    assert_eq!(
        trip(Vector4::new(-0.1, 0.1, -0.1, 0.1), Vector4::ceil),
        Vector4::new(0.0, 1.0, 0.0, 1.0)
    );
    assert_eq!(
        trip(Vector4::new(-0.5, 0.5, -0.5, 0.5), Vector4::round),
        Vector4::new(0.0, 1.0, 0.0, 1.0)
    );
    assert_eq!(
        trip(Vector4::new(-1.5, 1.5, -1.9, 1.9), Vector4::round_to_zero),
        Vector4::new(-1.0, 1.0, -1.0, 1.0)
    );
}
