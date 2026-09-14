//! Port of `three.js/test/unit/src/math/Matrix4.tests.js`.
//!
//! Skipped: `isMatrix4` (no type tag), `clone` (`Copy`), and the
//! `toArray(array, offset)` sparse-array assertions (`to_array` returns
//! `[f64; 16]`).
//!
//! `make_perspective`/`make_orthographic` take an explicit `CoordinateSystem`
//! in the Rust port (three.js reads `this.coordinateSystem`, whose default is
//! WebGL), so the tests pass `CoordinateSystem::WebGL` to reproduce three.js'
//! own expectations.

mod support;

use support::EPS;
use three_rs::math::math_utils::deg_to_rad;
use three_rs::math::{CoordinateSystem, Euler, EulerOrder, Matrix3, Matrix4, Quaternion, Vector3};

fn matrix_equals4(a: &Matrix4, b: &Matrix4, tolerance: f64) -> bool {
    a.elements
        .iter()
        .zip(b.elements.iter())
        .all(|(x, y)| (x - y).abs() <= tolerance)
}

fn euler_equals(a: &Euler, b: &Euler, tolerance: f64) -> bool {
    (a.x - b.x).abs() + (a.y - b.y).abs() + (a.z - b.z).abs() < tolerance
}

/// `new Matrix4().set( ... )` in row order.
#[allow(clippy::too_many_arguments)]
fn m4(e: [f64; 16]) -> Matrix4 {
    Matrix4::from_rows(
        e[0], e[1], e[2], e[3], e[4], e[5], e[6], e[7], e[8], e[9], e[10], e[11], e[12], e[13],
        e[14], e[15],
    )
}

fn counting() -> Matrix4 {
    m4([
        0.0, 1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0, 11.0, 12.0, 13.0, 14.0, 15.0,
    ])
}

#[test]
fn instancing() {
    let a = Matrix4::identity();
    assert_eq!(a.determinant(), 1.0);

    let b = counting();
    assert_eq!(
        b.elements,
        [0.0, 4.0, 8.0, 12.0, 1.0, 5.0, 9.0, 13.0, 2.0, 6.0, 10.0, 14.0, 3.0, 7.0, 11.0, 15.0],
        "row-major set(), column-major storage"
    );

    assert!(!matrix_equals4(&a, &b, EPS));
}

#[test]
fn set() {
    let mut b = Matrix4::identity();
    assert_eq!(b.determinant(), 1.0);

    b.set(
        0.0, 1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0, 11.0, 12.0, 13.0, 14.0, 15.0,
    );
    assert_eq!(b.elements, counting().elements);
}

#[test]
fn identity() {
    let mut b = counting();
    let a = Matrix4::identity();
    assert!(!matrix_equals4(&a, &b, EPS));

    b.set_identity();
    assert!(matrix_equals4(&a, &b, EPS));
}

#[test]
fn copy() {
    let mut a = counting();
    let mut b = Matrix4::identity();
    b.copy(&a);

    assert!(matrix_equals4(&a, &b, EPS));

    a.elements[0] = 2.0;
    assert!(!matrix_equals4(&a, &b, EPS));
}

#[test]
fn set_from_matrix3() {
    let mut a = Matrix3::identity();
    a.set(0.0, 1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0);
    let mut b = Matrix4::identity();
    let c = m4([
        0.0, 1.0, 2.0, 0.0, 3.0, 4.0, 5.0, 0.0, 6.0, 7.0, 8.0, 0.0, 0.0, 0.0, 0.0, 1.0,
    ]);
    b.set_from_matrix3(&a);
    assert!(b.equals(&c));
}

#[test]
fn copy_position() {
    let a = m4([
        1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0, 11.0, 12.0, 13.0, 14.0, 15.0, 16.0,
    ]);
    let mut b = m4([
        1.0, 2.0, 3.0, 0.0, 5.0, 6.0, 7.0, 0.0, 9.0, 10.0, 11.0, 0.0, 13.0, 14.0, 15.0, 16.0,
    ]);

    assert!(!matrix_equals4(&a, &b, EPS), "a and b initially not equal");

    b.copy_position(&a);
    assert!(
        matrix_equals4(&a, &b, EPS),
        "a and b equal after copy_position()"
    );
}

#[test]
fn make_basis_extract_basis() {
    let identity_basis = [
        Vector3::new(1.0, 0.0, 0.0),
        Vector3::new(0.0, 1.0, 0.0),
        Vector3::new(0.0, 0.0, 1.0),
    ];
    let mut a = Matrix4::identity();
    a.make_basis(&identity_basis[0], &identity_basis[1], &identity_basis[2]);
    assert!(matrix_equals4(&a, &Matrix4::identity(), EPS));

    let test_bases = [[
        Vector3::new(0.0, 1.0, 0.0),
        Vector3::new(-1.0, 0.0, 0.0),
        Vector3::new(0.0, 0.0, 1.0),
    ]];

    for test_basis in test_bases {
        let mut b = Matrix4::identity();
        b.make_basis(&test_basis[0], &test_basis[1], &test_basis[2]);

        let mut out_basis = [Vector3::ZERO; 3];
        let [o0, o1, o2] = &mut out_basis;
        b.extract_basis(o0, o1, o2);

        for (out, expected) in out_basis.iter().zip(test_basis.iter()) {
            assert!(out.equals(expected));
        }

        // get the basis out the hard way
        for j in 0..3 {
            out_basis[j].copy(&identity_basis[j]);
            out_basis[j].apply_matrix4(&b);
        }

        for (out, expected) in out_basis.iter().zip(test_basis.iter()) {
            assert!(out.equals(expected));
        }
    }
}

#[test]
fn make_rotation_from_euler_extract_rotation() {
    let test_values = [
        Euler::new_with_order(0.0, 0.0, 0.0, EulerOrder::XYZ),
        Euler::new_with_order(1.0, 0.0, 0.0, EulerOrder::XYZ),
        Euler::new_with_order(0.0, 1.0, 0.0, EulerOrder::ZYX),
        Euler::new_with_order(0.0, 0.0, 0.5, EulerOrder::YZX),
        Euler::new_with_order(0.0, 0.0, -0.5, EulerOrder::YZX),
    ];

    for (i, v) in test_values.iter().enumerate() {
        let mut m = Matrix4::identity();
        m.make_rotation_from_euler(v);

        let mut v2 = Euler::default();
        v2.set_from_rotation_matrix(&m, v.order);
        let mut m2 = Matrix4::identity();
        m2.make_rotation_from_euler(&v2);

        assert!(
            matrix_equals4(&m, &m2, EPS),
            "make_rotation_from_euler #{i}"
        );
        assert!(euler_equals(v, &v2, EPS), "make_rotation_from_euler #{i}");

        let mut m3 = Matrix4::identity();
        m3.extract_rotation(&m2);
        let mut v3 = Euler::default();
        v3.set_from_rotation_matrix(&m3, v.order);

        assert!(matrix_equals4(&m, &m3, EPS), "extract_rotation #{i}");
        assert!(euler_equals(v, &v3, EPS), "extract_rotation #{i}");
    }
}

#[test]
fn look_at() {
    let mut a = Matrix4::identity();
    let mut expected = Matrix4::identity();
    let mut eye = Vector3::new(0.0, 0.0, 0.0);
    let mut target = Vector3::new(0.0, 1.0, -1.0);
    let up = Vector3::new(0.0, 1.0, 0.0);

    a.look_at(&eye, &target, &up);
    let mut rotation = Euler::default();
    rotation.set_from_rotation_matrix(&a, EulerOrder::XYZ);
    support::close(
        rotation.x * (180.0 / std::f64::consts::PI),
        45.0,
        EPS,
        "Check the rotation",
    );

    // eye and target are in the same position
    eye.copy(&target);
    a.look_at(&eye, &target, &up);
    assert!(
        matrix_equals4(&a, &expected, EPS),
        "Check the result for eye == target"
    );

    // up and z are parallel
    eye.set(0.0, 1.0, 0.0);
    target.set(0.0, 0.0, 0.0);
    a.look_at(&eye, &target, &up);
    expected.set(
        1.0, 0.0, 0.0, 0.0, 0.0, 0.0001, 1.0, 0.0, 0.0, -1.0, 0.0001, 0.0, 0.0, 0.0, 0.0, 1.0,
    );
    assert!(
        matrix_equals4(&a, &expected, EPS),
        "Check the result for when up and z are parallel"
    );
}

const PRODUCT: [f64; 16] = [
    1585.0, 5318.0, 10514.0, 15894.0, 1655.0, 5562.0, 11006.0, 16634.0, 1787.0, 5980.0, 11840.0,
    17888.0, 1861.0, 6246.0, 12378.0, 18710.0,
];

fn primes_lhs() -> Matrix4 {
    m4([
        2.0, 3.0, 5.0, 7.0, 11.0, 13.0, 17.0, 19.0, 23.0, 29.0, 31.0, 37.0, 41.0, 43.0, 47.0, 53.0,
    ])
}

fn primes_rhs() -> Matrix4 {
    m4([
        59.0, 61.0, 67.0, 71.0, 73.0, 79.0, 83.0, 89.0, 97.0, 101.0, 103.0, 107.0, 109.0, 113.0,
        127.0, 131.0,
    ])
}

#[test]
fn multiply() {
    let mut lhs = primes_lhs();
    lhs.multiply(&primes_rhs());
    assert_eq!(lhs.elements, PRODUCT);
}

#[test]
fn premultiply() {
    let mut rhs = primes_rhs();
    rhs.premultiply(&primes_lhs());
    assert_eq!(rhs.elements, PRODUCT);
}

#[test]
fn multiply_matrices() {
    let mut ans = Matrix4::identity();
    ans.multiply_matrices(&primes_lhs(), &primes_rhs());
    assert_eq!(ans.elements, PRODUCT);
}

#[test]
fn multiply_scalar() {
    let mut b = counting();
    let before = b.elements;
    b.multiply_scalar(2.0);
    for (elem, before_elem) in b.elements.iter().zip(before.iter()) {
        assert_eq!(*elem, before_elem * 2.0);
    }
}

#[test]
fn determinant() {
    let mut a = Matrix4::identity();
    assert_eq!(a.determinant(), 1.0);

    a.elements[0] = 2.0;
    assert_eq!(a.determinant(), 2.0);

    a.elements[0] = 0.0;
    assert_eq!(a.determinant(), 0.0);

    a.set(
        2.0, 3.0, 4.0, 5.0, -1.0, -21.0, -3.0, -4.0, 6.0, 7.0, 8.0, 10.0, -8.0, -9.0, -10.0, -12.0,
    );
    assert_eq!(a.determinant(), 76.0);
}

#[test]
fn determinant_affine() {
    // For affine matrices the 3x3 result equals the full 4x4 determinant,
    // since the bottom row is [ 0, 0, 0, 1 ].
    let mut a = Matrix4::identity();
    let position = Vector3::new(5.0, -2.0, 3.0);
    let mut quaternion = Quaternion::default();
    quaternion.set_from_euler(&Euler::new(0.1, -0.7, 1.3));

    a.compose(&position, &quaternion, &Vector3::new(2.0, 3.0, 0.5));
    assert!(
        (a.determinant_affine() - a.determinant()).abs() <= EPS,
        "Affine matrix"
    );

    a.compose(&position, &quaternion, &Vector3::new(2.0, 3.0, -0.5));
    assert!(
        a.determinant_affine() < 0.0,
        "Reflection produces a negative determinant"
    );
    assert!(
        (a.determinant_affine() - a.determinant()).abs() <= EPS,
        "Reflection matrix"
    );

    let mut shear = Matrix4::identity();
    shear.make_shear(0.5, 0.0, 0.2, 0.0, 0.7, 0.0);
    a.multiply(&shear);
    assert!(
        (a.determinant_affine() - a.determinant()).abs() <= EPS,
        "Shear matrix"
    );
}

#[test]
fn determinant_affine_projective() {
    // For non-affine (projective) matrices the bottom row is not [ 0, 0, 0, 1 ],
    // so the 3x3 result generally differs from the full 4x4 determinant.
    let mut a = Matrix4::identity();
    a.make_perspective(-1.0, 1.0, 1.0, -1.0, 1.0, 100.0, CoordinateSystem::WebGL);
    assert!((a.determinant_affine() - a.determinant()).abs() > EPS);
}

#[test]
fn transpose() {
    let a = Matrix4::identity();
    let mut b = a;
    b.transpose();
    assert!(matrix_equals4(&a, &b, EPS));

    let b = counting();
    let mut c = b;
    c.transpose();
    assert!(!matrix_equals4(&b, &c, EPS));
    c.transpose();
    assert!(matrix_equals4(&b, &c, EPS));
}

#[test]
fn set_position() {
    let mut a = counting();
    let b = Vector3::new(-1.0, -2.0, -3.0);
    let c = m4([
        0.0, 1.0, 2.0, -1.0, 4.0, 5.0, 6.0, -2.0, 8.0, 9.0, 10.0, -3.0, 12.0, 13.0, 14.0, 15.0,
    ]);

    a.set_position(b.x, b.y, b.z);
    assert!(matrix_equals4(&a, &c, EPS));
}

#[test]
fn invert() {
    let zero = m4([0.0; 16]);
    let identity = Matrix4::identity();

    let mut a = Matrix4::identity();
    let b = m4([0.0; 16]);

    a.copy(&b).invert();
    assert!(matrix_equals4(&a, &zero, EPS));

    let test_matrices = [
        *Matrix4::identity().make_rotation_x(0.3),
        *Matrix4::identity().make_rotation_x(-0.3),
        *Matrix4::identity().make_rotation_y(0.3),
        *Matrix4::identity().make_rotation_y(-0.3),
        *Matrix4::identity().make_rotation_z(0.3),
        *Matrix4::identity().make_rotation_z(-0.3),
        *Matrix4::identity().make_scale(1.0, 2.0, 3.0),
        *Matrix4::identity().make_scale(1.0 / 8.0, 1.0 / 2.0, 1.0 / 3.0),
        *Matrix4::identity().make_perspective(
            -1.0,
            1.0,
            1.0,
            -1.0,
            1.0,
            1000.0,
            CoordinateSystem::WebGL,
        ),
        *Matrix4::identity().make_perspective(
            -16.0,
            16.0,
            9.0,
            -9.0,
            0.1,
            10000.0,
            CoordinateSystem::WebGL,
        ),
        *Matrix4::identity().make_translation(1.0, 2.0, 3.0),
    ];

    for m in test_matrices {
        let mut m_inverse = Matrix4::identity();
        m_inverse.copy(&m).invert();

        let mut m_self_inverse = m;
        m_self_inverse.invert();
        assert!(matrix_equals4(&m_self_inverse, &m_inverse, EPS));

        assert!((m.determinant() * m_inverse.determinant() - 1.0).abs() < EPS);

        let mut m_product = Matrix4::identity();
        m_product.multiply_matrices(&m, &m_inverse);
        assert!((m_product.determinant() - 1.0).abs() < EPS);
        assert!(matrix_equals4(&m_product, &identity, EPS));
    }
}

#[test]
fn scale() {
    let mut a = m4([
        1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0, 11.0, 12.0, 13.0, 14.0, 15.0, 16.0,
    ]);
    let b = Vector3::new(2.0, 3.0, 4.0);
    let c = m4([
        2.0, 6.0, 12.0, 4.0, 10.0, 18.0, 28.0, 8.0, 18.0, 30.0, 44.0, 12.0, 26.0, 42.0, 60.0, 16.0,
    ]);

    a.scale(&b);
    assert!(matrix_equals4(&a, &c, EPS));
}

#[test]
fn get_max_scale_on_axis() {
    let a = m4([
        1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0, 11.0, 12.0, 13.0, 14.0, 15.0, 16.0,
    ]);
    let expected = (3.0 * 3.0 + 7.0 * 7.0 + 11.0 * 11.0_f64).sqrt();

    assert!(
        (a.get_max_scale_on_axis() - expected).abs() <= EPS,
        "Check result"
    );
}

#[test]
fn make_translation() {
    let mut a = Matrix4::identity();
    let b = Vector3::new(2.0, 3.0, 4.0);
    let c = m4([
        1.0, 0.0, 0.0, 2.0, 0.0, 1.0, 0.0, 3.0, 0.0, 0.0, 1.0, 4.0, 0.0, 0.0, 0.0, 1.0,
    ]);

    a.make_translation(b.x, b.y, b.z);
    assert!(matrix_equals4(&a, &c, EPS));
}

#[test]
fn make_rotation_x() {
    let mut a = Matrix4::identity();
    let b = 3.0_f64.sqrt() / 2.0;
    let c = m4([
        1.0, 0.0, 0.0, 0.0, 0.0, b, -0.5, 0.0, 0.0, 0.5, b, 0.0, 0.0, 0.0, 0.0, 1.0,
    ]);

    a.make_rotation_x(std::f64::consts::PI / 6.0);
    assert!(matrix_equals4(&a, &c, EPS));
}

#[test]
fn make_rotation_y() {
    let mut a = Matrix4::identity();
    let b = 3.0_f64.sqrt() / 2.0;
    let c = m4([
        b, 0.0, 0.5, 0.0, 0.0, 1.0, 0.0, 0.0, -0.5, 0.0, b, 0.0, 0.0, 0.0, 0.0, 1.0,
    ]);

    a.make_rotation_y(std::f64::consts::PI / 6.0);
    assert!(matrix_equals4(&a, &c, EPS));
}

#[test]
fn make_rotation_z() {
    let mut a = Matrix4::identity();
    let b = 3.0_f64.sqrt() / 2.0;
    let c = m4([
        b, -0.5, 0.0, 0.0, 0.5, b, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
    ]);

    a.make_rotation_z(std::f64::consts::PI / 6.0);
    assert!(matrix_equals4(&a, &c, EPS));
}

#[test]
fn make_rotation_axis() {
    let mut axis = Vector3::new(1.5, 0.0, 1.0);
    axis.normalize();
    let radians = deg_to_rad(45.0);
    let mut a = Matrix4::identity();
    a.make_rotation_axis(&axis, radians);

    let expected = m4([
        0.9098790095958609,
        -0.39223227027636803,
        0.13518148560620882,
        0.0,
        0.39223227027636803,
        0.7071067811865476,
        -0.588348405414552,
        0.0,
        0.13518148560620882,
        0.588348405414552,
        0.7972277715906868,
        0.0,
        0.0,
        0.0,
        0.0,
        1.0,
    ]);

    assert!(matrix_equals4(&a, &expected, EPS), "Check numeric result");
}

#[test]
fn make_scale() {
    let mut a = Matrix4::identity();
    let c = m4([
        2.0, 0.0, 0.0, 0.0, 0.0, 3.0, 0.0, 0.0, 0.0, 0.0, 4.0, 0.0, 0.0, 0.0, 0.0, 1.0,
    ]);

    a.make_scale(2.0, 3.0, 4.0);
    assert!(matrix_equals4(&a, &c, EPS));
}

#[test]
fn make_shear() {
    let mut a = Matrix4::identity();
    let c = m4([
        1.0, 3.0, 5.0, 0.0, 1.0, 1.0, 6.0, 0.0, 2.0, 4.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
    ]);

    a.make_shear(1.0, 2.0, 3.0, 4.0, 5.0, 6.0);
    assert!(matrix_equals4(&a, &c, EPS));
}

#[test]
fn compose_decompose() {
    let t_values = [
        Vector3::ZERO,
        Vector3::new(3.0, 0.0, 0.0),
        Vector3::new(0.0, 4.0, 0.0),
        Vector3::new(0.0, 0.0, 5.0),
        Vector3::new(-6.0, 0.0, 0.0),
        Vector3::new(0.0, -7.0, 0.0),
        Vector3::new(0.0, 0.0, -8.0),
        Vector3::new(-2.0, 5.0, -9.0),
        Vector3::new(-2.0, -5.0, -9.0),
    ];

    let s_values = [
        Vector3::new(1.0, 1.0, 1.0),
        Vector3::new(2.0, 2.0, 2.0),
        Vector3::new(1.0, -1.0, 1.0),
        Vector3::new(-1.0, 1.0, 1.0),
        Vector3::new(1.0, 1.0, -1.0),
        Vector3::new(2.0, -2.0, 1.0),
        Vector3::new(-1.0, 2.0, -2.0),
        Vector3::new(-1.0, -1.0, -1.0),
        Vector3::new(-2.0, -2.0, -2.0),
    ];

    let mut r1 = Quaternion::default();
    r1.set_from_euler(&Euler::new(1.0, 1.0, 0.0));
    let mut r2 = Quaternion::default();
    r2.set_from_euler(&Euler::new(1.0, -1.0, 1.0));
    let r_values = [
        Quaternion::default(),
        r1,
        r2,
        Quaternion::new(0.0, 0.9238795292366128, 0.0, 0.38268342717215614),
    ];

    for t in &t_values {
        for s in &s_values {
            for r in &r_values {
                let mut m = Matrix4::identity();
                m.compose(t, r, s);

                let mut t2 = Vector3::ZERO;
                let mut r2 = Quaternion::default();
                let mut s2 = Vector3::ZERO;
                m.decompose(&mut t2, &mut r2, &mut s2);

                let mut m2 = Matrix4::identity();
                m2.compose(&t2, &r2, &s2);

                assert!(matrix_equals4(&m, &m2, EPS), "t {t:?} s {s:?} r {r:?}");
            }
        }
    }
}

#[test]
fn make_perspective() {
    let mut a = Matrix4::identity();
    a.make_perspective(-1.0, 1.0, -1.0, 1.0, 1.0, 100.0, CoordinateSystem::WebGL);
    let expected = m4([
        1.0,
        0.0,
        0.0,
        0.0,
        0.0,
        -1.0,
        0.0,
        0.0,
        0.0,
        0.0,
        -101.0 / 99.0,
        -200.0 / 99.0,
        0.0,
        0.0,
        -1.0,
        0.0,
    ]);
    assert!(matrix_equals4(&a, &expected, EPS), "Check result");
}

#[test]
fn make_orthographic() {
    let mut a = Matrix4::identity();
    a.make_orthographic(-1.0, 1.0, -1.0, 1.0, 1.0, 100.0, CoordinateSystem::WebGL);
    let expected = m4([
        1.0,
        0.0,
        0.0,
        0.0,
        0.0,
        -1.0,
        0.0,
        0.0,
        0.0,
        0.0,
        -2.0 / 99.0,
        -101.0 / 99.0,
        0.0,
        0.0,
        0.0,
        1.0,
    ]);

    assert!(matrix_equals4(&a, &expected, EPS), "Check result");
}

#[test]
fn equals() {
    let mut a = counting();
    let mut b = counting();
    b.set(
        0.0, -1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0, 11.0, 12.0, 13.0, 14.0, 15.0,
    );

    assert!(!a.equals(&b), "Check that a does not equal b");
    assert!(!b.equals(&a), "Check that b does not equal a");

    a.copy(&b);
    assert!(a.equals(&b), "Check that a equals b after copy()");
    assert!(b.equals(&a), "Check that b equals a after copy()");
}

#[test]
fn from_array() {
    let mut a = Matrix4::identity();
    let b = m4([
        1.0, 5.0, 9.0, 13.0, 2.0, 6.0, 10.0, 14.0, 3.0, 7.0, 11.0, 15.0, 4.0, 8.0, 12.0, 16.0,
    ]);

    let src: Vec<f64> = (1..=16).map(|i| i as f64).collect();
    a.from_array(&src, 0);
    assert!(a.equals(&b));
}

#[test]
fn to_array() {
    let a = m4([
        1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0, 11.0, 12.0, 13.0, 14.0, 15.0, 16.0,
    ]);
    let no_offset = [
        1.0, 5.0, 9.0, 13.0, 2.0, 6.0, 10.0, 14.0, 3.0, 7.0, 11.0, 15.0, 4.0, 8.0, 12.0, 16.0,
    ];
    assert_eq!(a.to_array(), no_offset, "No array, no offset");
}
