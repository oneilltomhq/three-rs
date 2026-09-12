//! The shared pieces of `three.js/test/unit`: the constants from
//! `test/unit/utils/math-constants.js` and the two assertions QUnit gives the
//! tests for free.
#![allow(dead_code)]

/// `math-constants.js` `x`.
pub const X: f64 = 2.0;
/// `math-constants.js` `y`.
pub const Y: f64 = 3.0;
/// `math-constants.js` `z`.
pub const Z: f64 = 4.0;
/// `math-constants.js` `w`.
pub const W: f64 = 5.0;
/// `math-constants.js` `eps`.
pub const EPS: f64 = 0.0001;

/// `assert.ok( Math.abs( a - b ) <= eps )`.
#[track_caller]
pub fn close(a: f64, b: f64, eps: f64, what: &str) {
    assert!(
        (a - b).abs() <= eps,
        "{what}: |{a} - {b}| = {} > {eps}",
        (a - b).abs()
    );
}

/// Element-wise [`close`] over two slices.
#[track_caller]
pub fn close_all(a: &[f64], b: &[f64], eps: f64, what: &str) {
    assert_eq!(a.len(), b.len(), "{what}: length");
    for (i, (&ai, &bi)) in a.iter().zip(b.iter()).enumerate() {
        close(ai, bi, eps, &format!("{what}[{i}]"));
    }
}
