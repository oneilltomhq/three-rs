//! Port of `three.js/test/unit/src/math/Quaternion.tests.js`.
//!
//! Skipped: the `properties`/`x`/`y`/`z`/`w`/`_onChange`/`_onChangeCallback`
//! tests (the Rust port has no property setters, so no change hook — `Object3D`
//! re-syncs explicitly instead), `isQuaternion`, `clone` (`Copy`), `random`
//! (`Math.random`), `toJSON`, `iterable`, and the `toArray(array, offset)`
//! sparse-array assertions (`to_array` returns a fixed-size array).

mod support;

use support::{close, EPS, W, X, Y, Z};
use three_rs::math::{Euler, EulerOrder, Matrix4, Quaternion, Vector3};

const ORDERS: [EulerOrder; 6] = [
    EulerOrder::XYZ,
    EulerOrder::YXZ,
    EulerOrder::ZXY,
    EulerOrder::ZYX,
    EulerOrder::YZX,
    EulerOrder::XZY,
];

fn euler_angles() -> Euler {
    Euler::new(0.1, -0.3, 0.25)
}

fn change_euler_order(e: &Euler, order: EulerOrder) -> Euler {
    Euler::new_with_order(e.x, e.y, e.z, order)
}

fn q_sub(a: &Quaternion, b: &Quaternion) -> Quaternion {
    Quaternion::new(a.x - b.x, a.y - b.y, a.z - b.z, a.w - b.w)
}

const SQRT1_2: f64 = std::f64::consts::FRAC_1_SQRT_2;

const SLERP_A: [f64; 4] = [
    0.6753410084407496,
    0.4087830051091744,
    0.32856700410659473,
    0.5185120064806223,
];
const SLERP_B: [f64; 4] = [
    0.6602792107657797,
    0.43647413932562285,
    0.35119011210236006,
    0.5001871596632682,
];

struct SlerpResult {
    v: [f64; 4],
    length: f64,
    dot_a: f64,
    dot_b: f64,
}

fn arr_dot(a: &[f64; 4], b: &[f64; 4]) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2] + a[3] * b[3]
}

impl SlerpResult {
    fn from(v: [f64; 4], a: &[f64; 4], b: &[f64; 4]) -> Self {
        Self {
            length: arr_dot(&v, &v).sqrt(),
            dot_a: arr_dot(&v, a),
            dot_b: arr_dot(&v, b),
            v,
        }
    }

    #[track_caller]
    fn assert_equals(&self, x: f64, y: f64, z: f64, w: f64, max_error: f64, what: &str) {
        for (got, want) in self.v.iter().zip([x, y, z, w]) {
            assert!(
                (want - got).abs() <= max_error,
                "{what}: {got} vs {want} (max error {max_error})"
            );
        }
    }
}

fn do_slerp_object(a_arr: &[f64; 4], b_arr: &[f64; 4], t: f64) -> SlerpResult {
    let b = Quaternion::new(b_arr[0], b_arr[1], b_arr[2], b_arr[3]);
    let mut c = Quaternion::new(a_arr[0], a_arr[1], a_arr[2], a_arr[3]);
    c.slerp(&b, t);
    SlerpResult::from([c.x, c.y, c.z, c.w], a_arr, b_arr)
}

fn do_slerp_array(a: &[f64; 4], b: &[f64; 4], t: f64) -> SlerpResult {
    let mut result = [0.0; 4];
    Quaternion::slerp_flat(&mut result, 0, a, 0, b, 0, t);
    SlerpResult::from(result, a, b)
}

fn slerp_test_skeleton(do_slerp: fn(&[f64; 4], &[f64; 4], f64) -> SlerpResult, max_error: f64) {
    let a = SLERP_A;
    let b = SLERP_B;

    let is_normal = |r: &SlerpResult| (1.0 - r.length).abs() <= max_error;

    let result = do_slerp(&a, &b, 0.0);
    result.assert_equals(a[0], a[1], a[2], a[3], 0.0, "Exactly A @ t = 0");

    let result = do_slerp(&a, &b, 1.0);
    result.assert_equals(b[0], b[1], b[2], b[3], 0.0, "Exactly B @ t = 1");

    let result = do_slerp(&a, &b, 0.5);
    assert!(
        (result.dot_a - result.dot_b).abs() <= f64::EPSILON,
        "Symmetry at 0.5"
    );
    assert!(is_normal(&result), "Approximately normal (at 0.5)");

    let result = do_slerp(&a, &b, 0.25);
    assert!(result.dot_a > result.dot_b, "Interpolating at 0.25");
    assert!(is_normal(&result), "Approximately normal (at 0.25)");

    let result = do_slerp(&a, &b, 0.75);
    assert!(result.dot_a < result.dot_b, "Interpolating at 0.75");
    assert!(is_normal(&result), "Approximately normal (at 0.75)");

    let d = SQRT1_2;

    let result = do_slerp(&[1.0, 0.0, 0.0, 0.0], &[0.0, 0.0, 1.0, 0.0], 0.5);
    result.assert_equals(d, 0.0, d, 0.0, f64::EPSILON, "X/Z diagonal from axes");
    assert!(is_normal(&result), "Approximately normal (X/Z diagonal)");

    let result = do_slerp(&[0.0, d, 0.0, d], &[0.0, -d, 0.0, d], 0.5);
    result.assert_equals(0.0, 0.0, 0.0, 1.0, f64::EPSILON, "W-Unit from diagonals");
    assert!(is_normal(&result), "Approximately normal (W-Unit)");
}

#[test]
fn instancing() {
    let a = Quaternion::default();
    assert_eq!((a.x, a.y, a.z, a.w), (0.0, 0.0, 0.0, 1.0));

    let a = Quaternion::new(X, Y, Z, W);
    assert_eq!((a.x, a.y, a.z, a.w), (X, Y, Z, W));
}

#[test]
fn slerp_static() {
    slerp_test_skeleton(do_slerp_object, f64::EPSILON);
}

#[test]
fn slerp_flat() {
    slerp_test_skeleton(do_slerp_array, f64::EPSILON);
}

#[test]
fn set() {
    let mut a = Quaternion::default();
    assert_eq!((a.x, a.y, a.z, a.w), (0.0, 0.0, 0.0, 1.0));

    a.set(X, Y, Z, W);
    assert_eq!((a.x, a.y, a.z, a.w), (X, Y, Z, W));
}

#[test]
fn copy() {
    let mut a = Quaternion::new(X, Y, Z, W);
    let mut b = Quaternion::default();
    b.copy(&a);
    assert_eq!((b.x, b.y, b.z, b.w), (X, Y, Z, W));

    a.x = 0.0;
    a.y = -1.0;
    a.z = 0.0;
    a.w = -1.0;
    assert_eq!((b.x, b.y), (X, Y));
}

#[test]
fn set_from_euler_set_from_quaternion() {
    let angles = [
        Vector3::new(1.0, 0.0, 0.0),
        Vector3::new(0.0, 1.0, 0.0),
        Vector3::new(0.0, 0.0, 1.0),
    ];

    for order in ORDERS {
        for angle in angles {
            let mut q = Quaternion::default();
            q.set_from_euler(&Euler::new_with_order(angle.x, angle.y, angle.z, order));

            let mut e = Euler::default();
            e.set_from_quaternion(&q, order);

            let new_angle = Vector3::new(e.x, e.y, e.z);
            assert!(
                new_angle.distance_to(&angle) < 0.001,
                "{order:?}: {new_angle:?} vs {angle:?}"
            );
        }
    }
}

#[test]
fn set_from_axis_angle() {
    let zero = Quaternion::default();

    let mut a = Quaternion::default();
    a.set_from_axis_angle(&Vector3::new(1.0, 0.0, 0.0), 0.0);
    assert!(a.equals(&zero));
    a.set_from_axis_angle(&Vector3::new(0.0, 1.0, 0.0), 0.0);
    assert!(a.equals(&zero));
    a.set_from_axis_angle(&Vector3::new(0.0, 0.0, 1.0), 0.0);
    assert!(a.equals(&zero));

    let mut b1 = Quaternion::default();
    b1.set_from_axis_angle(&Vector3::new(1.0, 0.0, 0.0), std::f64::consts::PI);
    assert!(!a.equals(&b1));

    let mut b2 = Quaternion::default();
    b2.set_from_axis_angle(&Vector3::new(1.0, 0.0, 0.0), -std::f64::consts::PI);
    assert!(!a.equals(&b2));

    b1.multiply(&b2);
    assert!(a.equals(&b1));
}

#[test]
fn set_from_euler_set_from_rotation_matrix() {
    // Euler conversion for Quaternion must match that of Matrix4.
    for order in ORDERS {
        let e = change_euler_order(&euler_angles(), order);

        let mut q = Quaternion::default();
        q.set_from_euler(&e);

        let mut m = Matrix4::identity();
        m.make_rotation_from_euler(&e);

        let mut q2 = Quaternion::default();
        q2.set_from_rotation_matrix(&m);

        assert!(q_sub(&q, &q2).length() < 0.001, "{order:?}");
    }
}

#[test]
fn set_from_rotation_matrix_branches() {
    // Contrived examples that hit the various `else if` blocks.
    let mut a = Quaternion::default();

    let mut q = Quaternion::new(-9.0, -2.0, 3.0, -4.0);
    q.normalize();
    let mut m = Matrix4::identity();
    m.make_rotation_from_quaternion(&q);
    let expected = [
        0.8581163303210332,
        0.19069251784911848,
        -0.2860387767736777,
        0.38138503569823695,
    ];

    a.set_from_rotation_matrix(&m);
    close(a.x, expected[0], EPS, "m11 > m22 && m11 > m33: check x");
    close(a.y, expected[1], EPS, "m11 > m22 && m11 > m33: check y");
    close(a.z, expected[2], EPS, "m11 > m22 && m11 > m33: check z");
    close(a.w, expected[3], EPS, "m11 > m22 && m11 > m33: check w");

    let mut q = Quaternion::new(-1.0, -2.0, 1.0, -1.0);
    q.normalize();
    m.make_rotation_from_quaternion(&q);
    let expected = [
        0.37796447300922714,
        0.7559289460184544,
        -0.37796447300922714,
        0.37796447300922714,
    ];

    a.set_from_rotation_matrix(&m);
    close(a.x, expected[0], EPS, "m22 > m33: check x");
    close(a.y, expected[1], EPS, "m22 > m33: check y");
    close(a.z, expected[2], EPS, "m22 > m33: check z");
    close(a.w, expected[3], EPS, "m22 > m33: check w");
}

#[test]
fn set_from_unit_vectors() {
    let mut a = Quaternion::default();
    let b = Vector3::new(1.0, 0.0, 0.0);
    let c = Vector3::new(0.0, 1.0, 0.0);
    let expected = Quaternion::new(0.0, 0.0, 2.0_f64.sqrt() / 2.0, 2.0_f64.sqrt() / 2.0);

    a.set_from_unit_vectors(&b, &c);
    close(a.x, expected.x, EPS, "Check x");
    close(a.y, expected.y, EPS, "Check y");
    close(a.z, expected.z, EPS, "Check z");
    close(a.w, expected.w, EPS, "Check w");
}

#[test]
fn angle_to() {
    let a = Quaternion::default();
    let mut b = Quaternion::default();
    b.set_from_euler(&Euler::new(0.0, std::f64::consts::PI, 0.0));
    let mut c = Quaternion::default();
    c.set_from_euler(&Euler::new(0.0, std::f64::consts::PI * 2.0, 0.0));

    assert_eq!(a.angle_to(&a), 0.0);
    assert_eq!(a.angle_to(&b), std::f64::consts::PI);
    assert_eq!(a.angle_to(&c), 0.0);
}

#[test]
fn rotate_towards() {
    let mut a = Quaternion::default();
    let mut b = Quaternion::default();
    b.set_from_euler(&Euler::new(0.0, std::f64::consts::PI, 0.0));
    let c = Quaternion::default();

    let half_pi = std::f64::consts::PI * 0.5;

    a.rotate_towards(&b, 0.0);
    assert!(a.equals(&a));

    a.rotate_towards(&b, std::f64::consts::PI * 2.0); // overshoot
    assert!(a.equals(&b));

    a.set(0.0, 0.0, 0.0, 1.0);
    a.rotate_towards(&b, half_pi);
    assert!(a.angle_to(&c) - half_pi <= EPS);
}

#[test]
fn identity() {
    let mut a = Quaternion::default();
    a.set(X, Y, Z, W);
    a.identity();
    assert_eq!((a.x, a.y, a.z, a.w), (0.0, 0.0, 0.0, 1.0));
}

#[test]
fn invert_conjugate() {
    let a = Quaternion::new(X, Y, Z, W);
    let mut b = a;
    b.conjugate();

    assert_eq!(a.x, -b.x);
    assert_eq!(a.y, -b.y);
    assert_eq!(a.z, -b.z);
    assert_eq!(a.w, b.w);
}

#[test]
fn dot() {
    let a = Quaternion::default();
    let b = Quaternion::default();
    assert_eq!(a.dot(&b), 1.0);

    let a = Quaternion::new(1.0, 2.0, 3.0, 1.0);
    let b = Quaternion::new(3.0, 2.0, 1.0, 1.0);
    assert_eq!(a.dot(&b), 11.0);
}

#[test]
fn normalize_length_length_sq() {
    let mut a = Quaternion::new(X, Y, Z, W);

    assert_ne!(a.length(), 1.0);
    assert_ne!(a.length_sq(), 1.0);
    a.normalize();
    assert_eq!(a.length(), 1.0);
    assert_eq!(a.length_sq(), 1.0);

    a.set(0.0, 0.0, 0.0, 0.0);
    assert_eq!(a.length_sq(), 0.0);
    assert_eq!(a.length(), 0.0);
    a.normalize();
    assert_eq!(a.length_sq(), 1.0);
    assert_eq!(a.length(), 1.0);
}

#[test]
fn multiply_quaternions_multiply() {
    let angles = [
        Euler::new(1.0, 0.0, 0.0),
        Euler::new(0.0, 1.0, 0.0),
        Euler::new(0.0, 0.0, 1.0),
    ];

    let mut q1 = Quaternion::default();
    q1.set_from_euler(&change_euler_order(&angles[0], EulerOrder::XYZ));
    let mut q2 = Quaternion::default();
    q2.set_from_euler(&change_euler_order(&angles[1], EulerOrder::XYZ));
    let mut q3 = Quaternion::default();
    q3.set_from_euler(&change_euler_order(&angles[2], EulerOrder::XYZ));

    let mut q = Quaternion::default();
    q.multiply_quaternions(&q1, &q2);
    q.multiply(&q3);

    let mut m1 = Matrix4::identity();
    m1.make_rotation_from_euler(&change_euler_order(&angles[0], EulerOrder::XYZ));
    let mut m2 = Matrix4::identity();
    m2.make_rotation_from_euler(&change_euler_order(&angles[1], EulerOrder::XYZ));
    let mut m3 = Matrix4::identity();
    m3.make_rotation_from_euler(&change_euler_order(&angles[2], EulerOrder::XYZ));

    let mut m = Matrix4::identity();
    m.multiply_matrices(&m1, &m2);
    m.multiply(&m3);

    let mut q_from_m = Quaternion::default();
    q_from_m.set_from_rotation_matrix(&m);

    assert!(q_sub(&q, &q_from_m).length() < 0.001);
}

#[test]
fn premultiply() {
    let mut a = Quaternion::new(X, Y, Z, W);
    let b = Quaternion::new(2.0 * X, -Y, -2.0 * Z, W);
    let expected = Quaternion::new(42.0, -32.0, -2.0, 58.0);

    a.premultiply(&b);
    close(a.x, expected.x, EPS, "Check x");
    close(a.y, expected.y, EPS, "Check y");
    close(a.z, expected.z, EPS, "Check z");
    close(a.w, expected.w, EPS, "Check w");
}

#[test]
fn slerp() {
    let mut a = Quaternion::new(X, Y, Z, W);
    a.normalize();
    let mut b = Quaternion::new(W, X, Y, Z);
    b.normalize();

    let mut c = a;
    c.slerp(&b, 0.0);
    let mut d = a;
    d.slerp(&b, 1.0);

    assert!(a.equals(&c));
    assert!(b.equals(&d));

    let d_const = SQRT1_2;

    let e = Quaternion::new(1.0, 0.0, 0.0, 0.0);
    let f = Quaternion::new(0.0, 0.0, 1.0, 0.0);
    let expected = Quaternion::new(d_const, 0.0, d_const, 0.0);
    let mut result = e;
    result.slerp(&f, 0.5);
    close(result.x, expected.x, EPS, "Check x");
    close(result.y, expected.y, EPS, "Check y");
    close(result.z, expected.z, EPS, "Check z");
    close(result.w, expected.w, EPS, "Check w");

    let g = Quaternion::new(0.0, d_const, 0.0, d_const);
    let h = Quaternion::new(0.0, -d_const, 0.0, d_const);
    let expected = Quaternion::new(0.0, 0.0, 0.0, 1.0);
    let mut result = g;
    result.slerp(&h, 0.5);
    close(result.x, expected.x, EPS, "Check x");
    close(result.y, expected.y, EPS, "Check y");
    close(result.z, expected.z, EPS, "Check z");
    close(result.w, expected.w, EPS, "Check w");
}

#[test]
fn slerp_quaternions() {
    let e = Quaternion::new(1.0, 0.0, 0.0, 0.0);
    let f = Quaternion::new(0.0, 0.0, 1.0, 0.0);
    let expected = Quaternion::new(SQRT1_2, 0.0, SQRT1_2, 0.0);

    let mut a = Quaternion::default();
    a.slerp_quaternions(&e, &f, 0.5);

    close(a.x, expected.x, EPS, "Check x");
    close(a.y, expected.y, EPS, "Check y");
    close(a.z, expected.z, EPS, "Check z");
    close(a.w, expected.w, EPS, "Check w");
}

#[test]
fn equals() {
    let mut a = Quaternion::new(X, Y, Z, W);
    let b = Quaternion::new(-X, -Y, -Z, -W);

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
    let mut a = Quaternion::default();
    a.from_array(&[X, Y, Z, W], 0);
    assert_eq!((a.x, a.y, a.z, a.w), (X, Y, Z, W));

    a.from_array(&[0.0, X, Y, Z, W, 0.0], 1);
    assert_eq!((a.x, a.y, a.z, a.w), (X, Y, Z, W));
}

#[test]
fn to_array() {
    let a = Quaternion::new(X, Y, Z, W);
    assert_eq!(a.to_array(), [X, Y, Z, W]);
}

#[test]
fn multiply_vector3() {
    let angles = [
        Euler::new(1.0, 0.0, 0.0),
        Euler::new(0.0, 1.0, 0.0),
        Euler::new(0.0, 0.0, 1.0),
    ];

    for order in ORDERS {
        for angle in &angles {
            let e = change_euler_order(angle, order);

            let mut q = Quaternion::default();
            q.set_from_euler(&e);
            let mut m = Matrix4::identity();
            m.make_rotation_from_euler(&e);

            let v0 = Vector3::new(1.0, 0.0, 0.0);
            let mut qv = v0;
            qv.apply_quaternion(&q);
            let mut mv = v0;
            mv.apply_matrix4(&m);

            assert!(qv.distance_to(&mv) < 0.001, "{order:?}");
        }
    }
}
