//! Tests for `three.js/src/math/Matrix2.js`.
//!
//! three.js ships no `Matrix2.tests.js`; these cover exactly what the source
//! does — the identity default, `set()`'s row-major-in/column-major-stored
//! ordering, `identity()` and `fromArray()` with an offset.

use three_rs::math::Matrix2;

#[test]
fn instancing() {
    // no arguments: the identity matrix
    let a = Matrix2::default();
    assert_eq!(a.elements, [1.0, 0.0, 0.0, 1.0]);

    // arguments are row-major, elements are column-major
    let b = Matrix2::new(11.0, 12.0, 21.0, 22.0);
    assert_eq!(b.elements, [11.0, 21.0, 12.0, 22.0]);
}

#[test]
fn set() {
    let mut a = Matrix2::default();

    a.set(11.0, 12.0, 21.0, 22.0);
    assert_eq!(a.elements, [11.0, 21.0, 12.0, 22.0]);
}

#[test]
fn identity() {
    let mut a = Matrix2::new(0.0, 1.0, 2.0, 3.0);
    assert_ne!(a.elements, [1.0, 0.0, 0.0, 1.0]);

    a.identity();
    assert_eq!(a.elements, [1.0, 0.0, 0.0, 1.0]);
}

#[test]
fn from_array() {
    let mut a = Matrix2::default();
    a.from_array(&[0.0, 1.0, 2.0, 3.0], 0);
    assert_eq!(a.elements, [0.0, 1.0, 2.0, 3.0]);

    // the array holds the elements in column-major order, read from `offset`
    let mut b = Matrix2::default();
    b.from_array(&[0.0, 1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0], 6);
    assert_eq!(b.elements, [6.0, 7.0, 8.0, 9.0]);
}
