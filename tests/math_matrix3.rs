//! Port of `three.js/test/unit/src/math/Matrix3.tests.js`.
//!
//! Skipped: `isMatrix3` (no type tag), `clone` (`Copy`), and the
//! `toArray(array, offset)` sparse-array assertions (`to_array` returns
//! `[f64; 9]`).

mod support;

use support::EPS;
use three_rs::math::{Matrix3, Matrix4, Vector2};

fn matrix_equals3(a: &Matrix3, b: &Matrix3, tolerance: f64) -> bool {
    a.elements
        .iter()
        .zip(b.elements.iter())
        .all(|(x, y)| (x - y).abs() <= tolerance)
}

fn matrix_equals4(a: &Matrix4, b: &Matrix4, tolerance: f64) -> bool {
    a.elements
        .iter()
        .zip(b.elements.iter())
        .all(|(x, y)| (x - y).abs() <= tolerance)
}

/// The test file's `toMatrix4()` helper: Matrix3 into the upper-left 3×3.
fn to_matrix4(m3: &Matrix3) -> Matrix4 {
    let mut result = Matrix4::identity();
    let me = m3.elements;
    result.elements[0] = me[0];
    result.elements[1] = me[1];
    result.elements[2] = me[2];
    result.elements[4] = me[3];
    result.elements[5] = me[4];
    result.elements[6] = me[5];
    result.elements[8] = me[6];
    result.elements[9] = me[7];
    result.elements[10] = me[8];
    result
}

fn m3(e: [f64; 9]) -> Matrix3 {
    let mut m = Matrix3::identity();
    m.set(e[0], e[1], e[2], e[3], e[4], e[5], e[6], e[7], e[8]);
    m
}

#[test]
fn instancing() {
    let a = Matrix3::identity();
    assert_eq!(a.determinant(), 1.0);

    let b = m3([0.0, 1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0]);
    assert_eq!(
        b.elements,
        [0.0, 3.0, 6.0, 1.0, 4.0, 7.0, 2.0, 5.0, 8.0],
        "row-major set(), column-major storage"
    );

    assert!(!matrix_equals3(&a, &b, EPS));
}

#[test]
fn set() {
    let mut b = Matrix3::identity();
    assert_eq!(b.determinant(), 1.0);

    b.set(0.0, 1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0);
    assert_eq!(b.elements, [0.0, 3.0, 6.0, 1.0, 4.0, 7.0, 2.0, 5.0, 8.0]);
}

#[test]
fn identity() {
    let mut b = m3([0.0, 1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0]);
    let a = Matrix3::identity();
    assert!(!matrix_equals3(&a, &b, EPS));

    b.set_identity();
    assert!(matrix_equals3(&a, &b, EPS));
}

#[test]
fn copy() {
    let mut a = m3([0.0, 1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0]);
    let mut b = Matrix3::identity();
    b.copy(&a);

    assert!(matrix_equals3(&a, &b, EPS));

    // ensure that it is a true copy
    a.elements[0] = 2.0;
    assert!(!matrix_equals3(&a, &b, EPS));
}

#[test]
fn set_from_matrix4() {
    let mut a = Matrix4::identity();
    a.set(
        0.0, 1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0, 11.0, 12.0, 13.0, 14.0, 15.0,
    );
    let mut b = Matrix3::identity();
    let c = m3([0.0, 1.0, 2.0, 4.0, 5.0, 6.0, 8.0, 9.0, 10.0]);
    b.set_from_matrix4(&a);
    assert!(b.equals(&c));
}

#[test]
fn multiply_premultiply() {
    // Both just wrap multiplyMatrices.
    let mut a = m3([2.0, 3.0, 5.0, 7.0, 11.0, 13.0, 17.0, 19.0, 23.0]);
    let b = m3([29.0, 31.0, 37.0, 41.0, 43.0, 47.0, 53.0, 59.0, 61.0]);
    let expected_multiply = [
        446.0, 1343.0, 2491.0, 486.0, 1457.0, 2701.0, 520.0, 1569.0, 2925.0,
    ];
    let expected_premultiply = [
        904.0, 1182.0, 1556.0, 1131.0, 1489.0, 1967.0, 1399.0, 1845.0, 2435.0,
    ];

    a.multiply(&b);
    assert_eq!(a.elements, expected_multiply, "multiply: check result");

    a.set(2.0, 3.0, 5.0, 7.0, 11.0, 13.0, 17.0, 19.0, 23.0);
    a.premultiply(&b);
    assert_eq!(a.elements, expected_premultiply, "premultiply: check result");
}

#[test]
fn multiply_matrices() {
    let lhs = m3([2.0, 3.0, 5.0, 7.0, 11.0, 13.0, 17.0, 19.0, 23.0]);
    let rhs = m3([29.0, 31.0, 37.0, 41.0, 43.0, 47.0, 53.0, 59.0, 61.0]);
    let mut ans = Matrix3::identity();

    ans.multiply_matrices(&lhs, &rhs);

    assert_eq!(
        ans.elements,
        [446.0, 1343.0, 2491.0, 486.0, 1457.0, 2701.0, 520.0, 1569.0, 2925.0]
    );
}

#[test]
fn multiply_scalar() {
    let mut b = m3([0.0, 1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0]);
    b.multiply_scalar(2.0);
    assert_eq!(
        b.elements,
        [0.0, 6.0, 12.0, 2.0, 8.0, 14.0, 4.0, 10.0, 16.0]
    );
}

#[test]
fn determinant() {
    let mut a = Matrix3::identity();
    assert_eq!(a.determinant(), 1.0);

    a.elements[0] = 2.0;
    assert_eq!(a.determinant(), 2.0);

    a.elements[0] = 0.0;
    assert_eq!(a.determinant(), 0.0);

    a.set(2.0, 3.0, 4.0, 5.0, 13.0, 7.0, 8.0, 9.0, 11.0);
    assert_eq!(a.determinant(), -73.0);
}

#[test]
fn invert() {
    let zero = m3([0.0; 9]);
    let identity4 = Matrix4::identity();
    let mut a = m3([0.0; 9]);
    let mut b = Matrix3::identity();

    b.copy(&a).invert();
    assert!(matrix_equals3(&b, &zero, EPS), "Matrix a is zero matrix");

    let test_matrices = [
        *Matrix4::identity().make_rotation_x(0.3),
        *Matrix4::identity().make_rotation_x(-0.3),
        *Matrix4::identity().make_rotation_y(0.3),
        *Matrix4::identity().make_rotation_y(-0.3),
        *Matrix4::identity().make_rotation_z(0.3),
        *Matrix4::identity().make_rotation_z(-0.3),
        *Matrix4::identity().make_scale(1.0, 2.0, 3.0),
        *Matrix4::identity().make_scale(1.0 / 8.0, 1.0 / 2.0, 1.0 / 3.0),
    ];

    for m in test_matrices {
        a.set_from_matrix4(&m);
        b.copy(&a).invert();
        let m_inverse3 = b;

        let m_inverse = to_matrix4(&m_inverse3);

        // the determinant of the inverse should be the reciprocal
        assert!((a.determinant() * m_inverse3.determinant() - 1.0).abs() < EPS);
        assert!((m.determinant() * m_inverse.determinant() - 1.0).abs() < EPS);

        let mut m_product = Matrix4::identity();
        m_product.multiply_matrices(&m, &m_inverse);
        assert!((m_product.determinant() - 1.0).abs() < EPS);
        assert!(matrix_equals4(&m_product, &identity4, EPS));
    }
}

#[test]
fn transpose() {
    let a = Matrix3::identity();
    let mut b = a;
    b.transpose();
    assert!(matrix_equals3(&a, &b, EPS));

    let b = m3([0.0, 1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0]);
    let mut c = b;
    c.transpose();
    assert!(!matrix_equals3(&b, &c, EPS));
    c.transpose();
    assert!(matrix_equals3(&b, &c, EPS));
}

#[test]
fn get_normal_matrix() {
    let mut a = Matrix3::identity();
    let mut b = Matrix4::identity();
    b.set(
        2.0, 3.0, 5.0, 7.0, 11.0, 13.0, 17.0, 19.0, 23.0, 29.0, 31.0, 37.0, 41.0, 43.0, 47.0, 57.0,
    );
    let expected = m3([
        -1.2857142857142856,
        0.7142857142857143,
        0.2857142857142857,
        0.7428571428571429,
        -0.7571428571428571,
        0.15714285714285714,
        -0.19999999999999998,
        0.3,
        -0.09999999999999999,
    ]);

    a.get_normal_matrix(&b);
    assert!(
        matrix_equals3(&a, &expected, EPS),
        "Check resulting Matrix3: {:?}",
        a.elements
    );
}

#[test]
fn transpose_into_array() {
    let a = m3([0.0, 1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0]);
    let b = a.transpose_into_array();
    assert_eq!(b, [0.0, 1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0]);
}

#[test]
fn set_uv_transform() {
    let mut a = m3([
        0.1767766952966369,
        0.17677669529663687,
        0.32322330470336313,
        -0.17677669529663687,
        0.1767766952966369,
        0.5,
        0.0,
        0.0,
        1.0,
    ]);
    // params
    let (center_x, center_y) = (0.5, 0.5);
    let (offset_x, offset_y) = (0.0, 0.0);
    let (repeat_x, repeat_y) = (0.25, 0.25);
    let rotation = 0.7753981633974483;

    let expected = m3([
        0.1785355940258599,
        0.17500011904519763,
        0.32323214346447127,
        -0.17500011904519763,
        0.1785355940258599,
        0.4982322625096689,
        0.0,
        0.0,
        1.0,
    ]);

    a.set_uv_transform(
        offset_x, offset_y, repeat_x, repeat_y, rotation, center_x, center_y,
    );

    assert!(matrix_equals3(&a, &expected, EPS), "Check direct method");
}

#[test]
fn make_translation() {
    let mut a = Matrix3::identity();
    let b = Vector2::new(1.0, 2.0);
    let c = m3([1.0, 0.0, 1.0, 0.0, 1.0, 2.0, 0.0, 0.0, 1.0]);

    a.make_translation(b.x, b.y);
    assert!(matrix_equals3(&a, &c, EPS), "Check translation result");
}

#[test]
fn make_rotation() {
    let mut a = Matrix3::identity();
    let theta = std::f64::consts::PI / 2.0;
    a.make_rotation(theta);
    let (c, s) = (theta.cos(), theta.sin());
    let expected = m3([c, -s, 0.0, s, c, 0.0, 0.0, 0.0, 1.0]);
    assert!(matrix_equals3(&a, &expected, EPS));
}

#[test]
fn make_scale() {
    let mut a = Matrix3::identity();
    a.make_scale(2.0, 3.0);
    let expected = m3([2.0, 0.0, 0.0, 0.0, 3.0, 0.0, 0.0, 0.0, 1.0]);
    assert!(matrix_equals3(&a, &expected, EPS));
}

#[test]
fn equals() {
    let mut a = m3([0.0, 1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0]);
    let b = m3([0.0, -1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0]);

    assert!(!a.equals(&b), "Check that a does not equal b");
    assert!(!b.equals(&a), "Check that b does not equal a");

    a.copy(&b);
    assert!(a.equals(&b), "Check that a equals b after copy()");
    assert!(b.equals(&a), "Check that b equals a after copy()");
}

#[test]
fn from_array() {
    let mut b = Matrix3::identity();
    b.from_array(&[0.0, 1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0], 0);
    assert_eq!(b.elements, [0.0, 1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0]);

    let mut b = Matrix3::identity();
    let big: Vec<f64> = (0..19).map(|i| i as f64).collect();
    b.from_array(&big, 10);
    assert_eq!(
        b.elements,
        [10.0, 11.0, 12.0, 13.0, 14.0, 15.0, 16.0, 17.0, 18.0]
    );
}

#[test]
fn to_array() {
    let a = m3([1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0]);
    let no_offset = [1.0, 4.0, 7.0, 2.0, 5.0, 8.0, 3.0, 6.0, 9.0];
    assert_eq!(a.to_array(), no_offset, "No array, no offset");
}
