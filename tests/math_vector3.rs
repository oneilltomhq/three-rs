//! Port of `three.js/test/unit/src/math/Vector3.tests.js`.
//!
//! Expectations and epsilons are three.js' own; nothing here was recomputed
//! from the Rust implementation.

mod support;

use support::{close, EPS, W, X, Y, Z};
use three_rs::cameras::PerspectiveCamera;
use three_rs::math::{CoordinateSystem, Euler, Matrix3, Matrix4, Quaternion, Vector3, Vector4};

#[test]
fn instancing() {
    let a = Vector3::default();
    assert_eq!(a.x, 0.0);
    assert_eq!(a.y, 0.0);
    assert_eq!(a.z, 0.0);

    let a = Vector3::new(X, Y, Z);
    assert_eq!(a.x, X);
    assert_eq!(a.y, Y);
    assert_eq!(a.z, Z);
}

#[test]
fn set() {
    let mut a = Vector3::default();
    assert_eq!((a.x, a.y, a.z), (0.0, 0.0, 0.0));

    a.set(X, Y, Z);
    assert_eq!((a.x, a.y, a.z), (X, Y, Z));
}

#[test]
fn copy() {
    let mut a = Vector3::new(X, Y, Z);
    let mut b = Vector3::default();
    b.copy(&a);
    assert_eq!((b.x, b.y, b.z), (X, Y, Z));

    // ensure that it is a true copy
    a.x = 0.0;
    a.y = -1.0;
    a.z = -2.0;
    assert_eq!((b.x, b.y, b.z), (X, Y, Z));
}

#[test]
fn add() {
    let mut a = Vector3::new(X, Y, Z);
    let b = Vector3::new(-X, -Y, -Z);

    a.add(&b);
    assert_eq!((a.x, a.y, a.z), (0.0, 0.0, 0.0));

    let mut c = Vector3::default();
    c.add_vectors(&b, &b);
    assert_eq!((c.x, c.y, c.z), (-2.0 * X, -2.0 * Y, -2.0 * Z));
}

#[test]
fn add_scaled_vector() {
    let mut a = Vector3::new(X, Y, Z);
    let b = Vector3::new(2.0, 3.0, 4.0);
    let s = 3.0;

    a.add_scaled_vector(&b, s);
    assert_eq!(a.x, X + b.x * s);
    assert_eq!(a.y, Y + b.y * s);
    assert_eq!(a.z, Z + b.z * s);
}

#[test]
fn sub() {
    let mut a = Vector3::new(X, Y, Z);
    let b = Vector3::new(-X, -Y, -Z);

    a.sub(&b);
    assert_eq!((a.x, a.y, a.z), (2.0 * X, 2.0 * Y, 2.0 * Z));

    let mut c = Vector3::default();
    c.sub_vectors(&a, &a);
    assert_eq!((c.x, c.y, c.z), (0.0, 0.0, 0.0));
}

#[test]
fn multiply_vectors() {
    let a = Vector3::new(X, Y, Z);
    let b = Vector3::new(2.0, 3.0, -5.0);

    let mut c = Vector3::default();
    c.multiply_vectors(&a, &b);

    assert_eq!(c.x, X * 2.0);
    assert_eq!(c.y, Y * 3.0);
    assert_eq!(c.z, Z * -5.0);
}

#[test]
fn apply_euler() {
    let mut a = Vector3::new(X, Y, Z);
    let euler = Euler::new(90.0, -45.0, 0.0);
    let expected = Vector3::new(-2.352970120501014, -4.7441750936226645, 0.9779234597246458);

    a.apply_euler(&euler);

    close(a.x, expected.x, EPS, "Check x");
    close(a.y, expected.y, EPS, "Check y");
    close(a.z, expected.z, EPS, "Check z");
}

#[test]
fn apply_axis_angle() {
    let mut a = Vector3::new(X, Y, Z);
    let axis = Vector3::new(0.0, 1.0, 0.0);
    let angle = std::f64::consts::PI / 4.0;
    let expected = Vector3::new(3.0 * 2.0f64.sqrt(), 3.0, 2.0f64.sqrt());

    a.apply_axis_angle(&axis, angle);

    close(a.x, expected.x, EPS, "Check x");
    close(a.y, expected.y, EPS, "Check y");
    close(a.z, expected.z, EPS, "Check z");
}

#[test]
fn apply_matrix3() {
    let mut a = Vector3::new(X, Y, Z);
    let mut m = Matrix3::identity();
    m.set(2.0, 3.0, 5.0, 7.0, 11.0, 13.0, 17.0, 19.0, 23.0);

    a.apply_matrix3(&m);

    assert_eq!(a.x, 33.0);
    assert_eq!(a.y, 99.0);
    assert_eq!(a.z, 183.0);
}

#[test]
fn apply_matrix4() {
    let mut a = Vector3::new(X, Y, Z);
    let mut b = Vector4::new(X, Y, Z, 1.0);

    let mut m = Matrix4::identity();
    m.make_rotation_x(std::f64::consts::PI);
    a.apply_matrix4(&m);
    b.apply_matrix4(&m);
    assert_eq!(a.x, b.x / b.w);
    assert_eq!(a.y, b.y / b.w);
    assert_eq!(a.z, b.z / b.w);

    let mut m = Matrix4::identity();
    m.make_translation(3.0, 2.0, 1.0);
    a.apply_matrix4(&m);
    b.apply_matrix4(&m);
    assert_eq!(a.x, b.x / b.w);
    assert_eq!(a.y, b.y / b.w);
    assert_eq!(a.z, b.z / b.w);

    let m = Matrix4::from_rows(
        1.0, 0.0, 0.0, 0.0, //
        0.0, 1.0, 0.0, 0.0, //
        0.0, 0.0, 1.0, 0.0, //
        0.0, 0.0, 1.0, 0.0,
    );
    a.apply_matrix4(&m);
    b.apply_matrix4(&m);
    assert_eq!(a.x, b.x / b.w);
    assert_eq!(a.y, b.y / b.w);
    assert_eq!(a.z, b.z / b.w);
}

#[test]
fn apply_quaternion() {
    let mut a = Vector3::new(X, Y, Z);

    a.apply_quaternion(&Quaternion::default());
    assert_eq!(a.x, X, "Identity rotation: check x");
    assert_eq!(a.y, Y, "Identity rotation: check y");
    assert_eq!(a.z, Z, "Identity rotation: check z");
}

#[test]
fn transform_direction() {
    let mut a = Vector3::new(X, Y, Z);
    let m = Matrix4::identity();
    let transformed = Vector3::new(0.3713906763541037, 0.5570860145311556, 0.7427813527082074);

    a.transform_direction(&m);

    close(a.x, transformed.x, EPS, "Check x");
    close(a.y, transformed.y, EPS, "Check y");
    close(a.z, transformed.z, EPS, "Check z");
}

#[test]
fn clamp_scalar() {
    let mut a = Vector3::new(-0.01, 0.5, 1.5);
    let clamped = Vector3::new(0.1, 0.5, 1.0);

    a.clamp_scalar(0.1, 1.0);

    close(a.x, clamped.x, 0.001, "Check x");
    close(a.y, clamped.y, 0.001, "Check y");
    close(a.z, clamped.z, 0.001, "Check z");
}

#[test]
fn negate() {
    let mut a = Vector3::new(X, Y, Z);
    a.negate();
    assert_eq!((a.x, a.y, a.z), (-X, -Y, -Z));
}

#[test]
fn dot() {
    let a = Vector3::new(X, Y, Z);
    let b = Vector3::new(-X, -Y, -Z);
    let c = Vector3::default();

    assert_eq!(a.dot(&b), -X * X - Y * Y - Z * Z);
    assert_eq!(a.dot(&c), 0.0);
}

#[test]
fn manhattan_length() {
    let mut a = Vector3::new(X, 0.0, 0.0);
    let b = Vector3::new(0.0, -Y, 0.0);
    let c = Vector3::new(0.0, 0.0, Z);
    let d = Vector3::default();

    assert_eq!(a.manhattan_length(), X, "Positive x");
    assert_eq!(b.manhattan_length(), Y, "Negative y");
    assert_eq!(c.manhattan_length(), Z, "Positive z");
    assert_eq!(d.manhattan_length(), 0.0, "Empty initialization");

    a.set(X, Y, Z);
    assert_eq!(
        a.manhattan_length(),
        X.abs() + Y.abs() + Z.abs(),
        "All components"
    );
}

#[test]
fn normalize() {
    let mut a = Vector3::new(X, 0.0, 0.0);
    let mut b = Vector3::new(0.0, -Y, 0.0);
    let mut c = Vector3::new(0.0, 0.0, Z);

    a.normalize();
    assert_eq!(a.length(), 1.0);
    assert_eq!(a.x, 1.0);

    b.normalize();
    assert_eq!(b.length(), 1.0);
    assert_eq!(b.y, -1.0);

    c.normalize();
    assert_eq!(c.length(), 1.0);
    assert_eq!(c.z, 1.0);
}

#[test]
fn set_length() {
    let mut a = Vector3::new(X, 0.0, 0.0);
    assert_eq!(a.length(), X);
    a.set_length(Y);
    assert_eq!(a.length(), Y);

    let mut a = Vector3::new(0.0, 0.0, 0.0);
    assert_eq!(a.length(), 0.0);
    a.set_length(Y);
    assert_eq!(a.length(), 0.0);

    // `a.setLength()` with no argument — `length` is `undefined`, so every
    // component becomes NaN.
    a.set_length(f64::NAN);
    assert!(a.length().is_nan());
}

#[test]
fn cross() {
    let mut a = Vector3::new(X, Y, Z);
    let b = Vector3::new(2.0 * X, -Y, 0.5 * Z);
    let crossed = Vector3::new(18.0, 12.0, -18.0);

    a.cross(&b);

    close(a.x, crossed.x, EPS, "Check x");
    close(a.y, crossed.y, EPS, "Check y");
    close(a.z, crossed.z, EPS, "Check z");
}

#[test]
fn cross_vectors() {
    let a = Vector3::new(X, Y, Z);
    let b = Vector3::new(X, -Y, Z);
    let mut c = Vector3::default();
    let crossed = Vector3::new(24.0, 0.0, -12.0);

    c.cross_vectors(&a, &b);

    close(c.x, crossed.x, EPS, "Check x");
    close(c.y, crossed.y, EPS, "Check y");
    close(c.z, crossed.z, EPS, "Check z");
}

#[test]
fn project_on_vector() {
    let mut a = Vector3::new(1.0, 0.0, 0.0);
    let mut b = Vector3::default();
    let normal = Vector3::new(10.0, 0.0, 0.0);

    assert!(b.copy(&a).project_on_vector(&normal).equals(&Vector3::new(1.0, 0.0, 0.0)));

    a.set(0.0, 1.0, 0.0);
    assert!(b.copy(&a).project_on_vector(&normal).equals(&Vector3::new(0.0, 0.0, 0.0)));

    a.set(0.0, 0.0, -1.0);
    assert!(b.copy(&a).project_on_vector(&normal).equals(&Vector3::new(0.0, 0.0, 0.0)));

    a.set(-1.0, 0.0, 0.0);
    assert!(b.copy(&a).project_on_vector(&normal).equals(&Vector3::new(-1.0, 0.0, 0.0)));
}

#[test]
fn project_on_plane() {
    let mut a = Vector3::new(1.0, 0.0, 0.0);
    let mut b = Vector3::default();
    let normal = Vector3::new(1.0, 0.0, 0.0);

    assert!(b.copy(&a).project_on_plane(&normal).equals(&Vector3::new(0.0, 0.0, 0.0)));

    a.set(0.0, 1.0, 0.0);
    assert!(b.copy(&a).project_on_plane(&normal).equals(&Vector3::new(0.0, 1.0, 0.0)));

    a.set(0.0, 0.0, -1.0);
    assert!(b.copy(&a).project_on_plane(&normal).equals(&Vector3::new(0.0, 0.0, -1.0)));

    a.set(-1.0, 0.0, 0.0);
    assert!(b.copy(&a).project_on_plane(&normal).equals(&Vector3::new(0.0, 0.0, 0.0)));
}

#[test]
fn reflect() {
    let mut a = Vector3::default();
    let mut normal = Vector3::new(0.0, 1.0, 0.0);
    let mut b = Vector3::default();

    a.set(0.0, -1.0, 0.0);
    assert!(b.copy(&a).reflect(&normal).equals(&Vector3::new(0.0, 1.0, 0.0)));

    a.set(1.0, -1.0, 0.0);
    assert!(b.copy(&a).reflect(&normal).equals(&Vector3::new(1.0, 1.0, 0.0)));

    a.set(1.0, -1.0, 0.0);
    normal.set(0.0, -1.0, 0.0);
    assert!(b.copy(&a).reflect(&normal).equals(&Vector3::new(1.0, 1.0, 0.0)));
}

#[test]
fn angle_to() {
    let a = Vector3::new(0.0, -0.18851655680720186, 0.9820700116639124);
    let b = Vector3::new(0.0, 0.18851655680720186, -0.9820700116639124);

    assert_eq!(a.angle_to(&a), 0.0);
    assert_eq!(a.angle_to(&b), std::f64::consts::PI);

    let x = Vector3::new(1.0, 0.0, 0.0);
    let y = Vector3::new(0.0, 1.0, 0.0);
    let z = Vector3::new(0.0, 0.0, 1.0);

    assert_eq!(x.angle_to(&y), std::f64::consts::PI / 2.0);
    assert_eq!(x.angle_to(&z), std::f64::consts::PI / 2.0);
    assert_eq!(z.angle_to(&x), std::f64::consts::PI / 2.0);

    assert!(
        (x.angle_to(&Vector3::new(1.0, 1.0, 0.0)) - std::f64::consts::PI / 4.0).abs() < 0.0000001
    );
}

#[test]
fn set_from_spherical_coords() {
    // `setFromSpherical( new Spherical( radius, phi, theta ) )` — the crate has
    // no `Spherical`, so the coords overload carries the same expectation.
    let mut a = Vector3::default();
    let phi = (-0.5f64).acos();
    let theta = std::f64::consts::PI.sqrt() * phi;
    let expected = Vector3::new(-4.677914006701843, -5.0, -7.288149322420796);

    a.set_from_spherical_coords(10.0, phi, theta);

    close(a.x, expected.x, EPS, "Check x");
    close(a.y, expected.y, EPS, "Check y");
    close(a.z, expected.z, EPS, "Check z");
}

#[test]
fn set_from_cylindrical_coords() {
    // `setFromCylindrical( new Cylindrical( 10, PI * 0.125, 20 ) )`.
    let mut a = Vector3::default();
    let expected = Vector3::new(3.826834323650898, 20.0, 9.238795325112868);

    a.set_from_cylindrical_coords(10.0, std::f64::consts::PI * 0.125, 20.0);

    close(a.x, expected.x, EPS, "Check x");
    close(a.y, expected.y, EPS, "Check y");
    close(a.z, expected.z, EPS, "Check z");
}

/// The matrix `Matrix4.tests.js` and friends use for the `setFromMatrix*` tests.
fn primes_matrix4() -> Matrix4 {
    Matrix4::from_rows(
        2.0, 3.0, 5.0, 7.0, //
        11.0, 13.0, 17.0, 19.0, //
        23.0, 29.0, 31.0, 37.0, //
        41.0, 43.0, 47.0, 53.0,
    )
}

#[test]
fn set_from_matrix_position() {
    let mut a = Vector3::default();
    a.set_from_matrix_position(&primes_matrix4());

    assert_eq!(a.x, 7.0);
    assert_eq!(a.y, 19.0);
    assert_eq!(a.z, 37.0);
}

#[test]
fn set_from_matrix_scale() {
    let mut a = Vector3::default();
    let expected = Vector3::new(25.573423705088842, 31.921779399024736, 35.70714214271425);

    a.set_from_matrix_scale(&primes_matrix4());

    close(a.x, expected.x, EPS, "Check x");
    close(a.y, expected.y, EPS, "Check y");
    close(a.z, expected.z, EPS, "Check z");
}

#[test]
fn set_from_matrix_column() {
    let mut a = Vector3::default();
    let m = primes_matrix4();

    a.set_from_matrix_column(&m, 0);
    assert_eq!(a.x, 2.0);
    assert_eq!(a.y, 11.0);
    assert_eq!(a.z, 23.0);

    a.set_from_matrix_column(&m, 2);
    assert_eq!(a.x, 5.0);
    assert_eq!(a.y, 17.0);
    assert_eq!(a.z, 31.0);
}

#[test]
fn equals() {
    let mut a = Vector3::new(X, 0.0, Z);
    let b = Vector3::new(0.0, -Y, 0.0);

    assert_ne!(a.x, b.x);
    assert_ne!(a.y, b.y);
    assert_ne!(a.z, b.z);
    assert!(!a.equals(&b));
    assert!(!b.equals(&a));

    a.copy(&b);
    assert_eq!(a.x, b.x);
    assert_eq!(a.y, b.y);
    assert_eq!(a.z, b.z);
    assert!(a.equals(&b));
    assert!(b.equals(&a));
}

#[test]
fn from_array() {
    let mut a = Vector3::default();
    let array = [1.0, 2.0, 3.0, 4.0, 5.0, 6.0];

    a.from_array(&array, 0);
    assert_eq!((a.x, a.y, a.z), (1.0, 2.0, 3.0));

    a.from_array(&array, 3);
    assert_eq!((a.x, a.y, a.z), (4.0, 5.0, 6.0));
}

#[test]
fn to_array() {
    let a = Vector3::new(X, Y, Z);
    let array = a.to_array();

    assert_eq!(array[0], X);
    assert_eq!(array[1], Y);
    assert_eq!(array[2], Z);
}

#[test]
fn from_buffer_attribute() {
    use three_rs::core::BufferAttribute;

    let attr = BufferAttribute::new(vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0], 3);

    let a = attr.get_vector3(0);
    assert_eq!((a.x, a.y, a.z), (1.0, 2.0, 3.0));

    let a = attr.get_vector3(1);
    assert_eq!((a.x, a.y, a.z), (4.0, 5.0, 6.0));
}

#[test]
fn set_x_set_y_set_z() {
    let mut a = Vector3::default();
    assert_eq!((a.x, a.y, a.z), (0.0, 0.0, 0.0));

    a.set_x(X);
    a.set_y(Y);
    a.set_z(Z);

    assert_eq!((a.x, a.y, a.z), (X, Y, Z));
}

#[test]
fn set_component_get_component() {
    let mut a = Vector3::default();
    assert_eq!((a.x, a.y, a.z), (0.0, 0.0, 0.0));

    a.set_component(0, 1.0);
    a.set_component(1, 2.0);
    a.set_component(2, 3.0);

    assert_eq!(a.get_component(0), 1.0);
    assert_eq!(a.get_component(1), 2.0);
    assert_eq!(a.get_component(2), 3.0);
}

#[test]
#[should_panic(expected = "index is out of range")]
fn set_component_out_of_range_panics() {
    Vector3::default().set_component(3, 0.0);
}

#[test]
#[should_panic(expected = "index is out of range")]
fn get_component_out_of_range_panics() {
    Vector3::default().get_component(3);
}

#[test]
fn min_max_clamp() {
    let a = Vector3::new(X, Y, Z);
    let b = Vector3::new(-X, -Y, -Z);
    let mut c = Vector3::default();

    c.copy(&a).min(&b);
    assert_eq!((c.x, c.y, c.z), (-X, -Y, -Z));

    c.copy(&a).max(&b);
    assert_eq!((c.x, c.y, c.z), (X, Y, Z));

    c.set(-2.0 * X, 2.0 * Y, -2.0 * Z);
    c.clamp(&b, &a);
    assert_eq!((c.x, c.y, c.z), (-X, Y, -Z));
}

#[test]
fn distance_to_distance_to_squared() {
    let a = Vector3::new(X, 0.0, 0.0);
    let b = Vector3::new(0.0, -Y, 0.0);
    let c = Vector3::new(0.0, 0.0, Z);
    let d = Vector3::default();

    assert_eq!(a.distance_to(&d), X);
    assert_eq!(a.distance_to_squared(&d), X * X);
    assert_eq!(b.distance_to(&d), Y);
    assert_eq!(b.distance_to_squared(&d), Y * Y);
    assert_eq!(c.distance_to(&d), Z);
    assert_eq!(c.distance_to_squared(&d), Z * Z);
}

#[test]
fn manhattan_distance_to() {
    let a = Vector3::new(X, Y, Z);
    let b = Vector3::new(-X, -Y, -Z);

    assert_eq!(a.manhattan_distance_to(&b), 2.0 * X + 2.0 * Y + 2.0 * Z);
}

#[test]
fn set_scalar_add_scalar_sub_scalar() {
    let mut a = Vector3::default();
    let s = 3.0;

    a.set_scalar(s);
    assert_eq!((a.x, a.y, a.z), (s, s, s));

    a.add_scalar(s);
    assert_eq!((a.x, a.y, a.z), (2.0 * s, 2.0 * s, 2.0 * s));

    a.sub_scalar(2.0 * s);
    assert_eq!((a.x, a.y, a.z), (0.0, 0.0, 0.0));
}

#[test]
fn multiply_divide() {
    let mut a = Vector3::new(X, Y, Z);
    let mut b = Vector3::new(2.0 * X, 2.0 * Y, 2.0 * Z);
    let c = Vector3::new(4.0 * X, 4.0 * Y, 4.0 * Z);

    let b_before = b;
    a.multiply(&b);
    assert_eq!(a.x, X * b_before.x);
    assert_eq!(a.y, Y * b_before.y);
    assert_eq!(a.z, Z * b_before.z);

    b.divide(&c);
    close(b.x, 0.5, EPS, "divide: check x");
    close(b.y, 0.5, EPS, "divide: check y");
    close(b.z, 0.5, EPS, "divide: check z");
}

#[test]
fn multiply_divide_scalar() {
    let mut a = Vector3::new(X, Y, Z);
    let mut b = Vector3::new(-X, -Y, -Z);

    a.multiply_scalar(-2.0);
    assert_eq!((a.x, a.y, a.z), (X * -2.0, Y * -2.0, Z * -2.0));

    b.multiply_scalar(-2.0);
    assert_eq!((b.x, b.y, b.z), (2.0 * X, 2.0 * Y, 2.0 * Z));

    a.divide_scalar(-2.0);
    assert_eq!((a.x, a.y, a.z), (X, Y, Z));

    b.divide_scalar(-2.0);
    assert_eq!((b.x, b.y, b.z), (-X, -Y, -Z));
}

#[test]
fn project_unproject() {
    let mut a = Vector3::new(X, Y, Z);

    // three.js' `PerspectiveCamera` defaults to `WebGLCoordinateSystem`; the
    // crate's defaults to `WebGPU` because that is what `WebGPURenderer` sets.
    // The expectations below are three.js', so the camera is put back on the
    // WebGL convention for them.
    let mut camera = PerspectiveCamera::new(75.0, 16.0 / 9.0, 0.1, 300.0);
    camera.coordinate_system = CoordinateSystem::WebGL;
    camera.update_projection_matrix();
    camera.update_matrix_world();

    let projected = Vector3::new(
        -0.36653213611158914,
        -0.9774190296309043,
        1.0506835611870624,
    );

    a.project(&camera);
    close(a.x, projected.x, EPS, "project: check x");
    close(a.y, projected.y, EPS, "project: check y");
    close(a.z, projected.z, EPS, "project: check z");

    a.unproject(&camera);
    close(a.x, X, EPS, "unproject: check x");
    close(a.y, Y, EPS, "unproject: check y");
    close(a.z, Z, EPS, "unproject: check z");
}

#[test]
fn length_length_sq() {
    let mut a = Vector3::new(X, 0.0, 0.0);
    let b = Vector3::new(0.0, -Y, 0.0);
    let c = Vector3::new(0.0, 0.0, Z);
    let d = Vector3::default();

    assert_eq!(a.length(), X);
    assert_eq!(a.length_sq(), X * X);
    assert_eq!(b.length(), Y);
    assert_eq!(b.length_sq(), Y * Y);
    assert_eq!(c.length(), Z);
    assert_eq!(c.length_sq(), Z * Z);
    assert_eq!(d.length(), 0.0);
    assert_eq!(d.length_sq(), 0.0);

    a.set(X, Y, Z);
    assert_eq!(a.length(), (X * X + Y * Y + Z * Z).sqrt());
    assert_eq!(a.length_sq(), X * X + Y * Y + Z * Z);
}

#[test]
fn lerp_clone() {
    let a = Vector3::new(X, 0.0, Z);
    let b = Vector3::new(0.0, -Y, 0.0);

    // lerping a vector with itself is the identity at any alpha
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

    let mut c = a;
    assert!(c.lerp(&b, 1.0).equals(&b));
}

#[test]
fn lerp_vectors() {
    let v1 = Vector3::new(X, Y, Z);
    let v2 = Vector3::new(W, W, W);
    let mut a = Vector3::default();

    a.lerp_vectors(&v1, &v2, 0.5);
    assert_eq!(a.x, (X + W) / 2.0);
    assert_eq!(a.y, (Y + W) / 2.0);
    assert_eq!(a.z, (Z + W) / 2.0);
}

#[test]
fn clamp_length() {
    let mut a = Vector3::new(X, 0.0, 0.0);

    a.clamp_length(0.0, 1.0);
    close(a.length(), 1.0, EPS, "clamped down to max");

    let mut a = Vector3::new(0.1, 0.0, 0.0);
    a.clamp_length(1.0, 10.0);
    close(a.length(), 1.0, EPS, "clamped up to min");
}

#[test]
fn floor_ceil_round_round_to_zero() {
    let mut a = Vector3::new(-0.5, 0.5, 1.5);
    a.floor();
    assert_eq!((a.x, a.y, a.z), (-1.0, 0.0, 1.0));

    let mut a = Vector3::new(-0.5, 0.5, 1.5);
    a.ceil();
    assert_eq!((a.x, a.y, a.z), (-0.0, 1.0, 2.0));

    // Math.round rounds half toward +Infinity
    let mut a = Vector3::new(-0.5, 0.5, 1.5);
    a.round();
    assert_eq!((a.x, a.y, a.z), (0.0, 1.0, 2.0));

    let mut a = Vector3::new(-1.5, 0.5, 1.5);
    a.round_to_zero();
    assert_eq!((a.x, a.y, a.z), (-1.0, 0.0, 1.0));
}

#[test]
fn set_from_euler_set_from_color() {
    use three_rs::math::Color;

    let mut a = Vector3::default();
    a.set_from_euler(&Euler::new(X, Y, Z));
    assert_eq!((a.x, a.y, a.z), (X, Y, Z));

    let mut a = Vector3::default();
    a.set_from_color(&Color::new(0.25, 0.5, 0.75));
    assert_eq!((a.x, a.y, a.z), (0.25, 0.5, 0.75));
}

#[test]
fn set_from_matrix3_column() {
    let mut m = Matrix3::identity();
    m.set(2.0, 3.0, 5.0, 7.0, 11.0, 13.0, 17.0, 19.0, 23.0);

    let mut a = Vector3::default();
    a.set_from_matrix3_column(&m, 0);
    assert_eq!((a.x, a.y, a.z), (2.0, 7.0, 17.0));

    a.set_from_matrix3_column(&m, 2);
    assert_eq!((a.x, a.y, a.z), (5.0, 13.0, 23.0));
}
