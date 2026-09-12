//! Port of `three.js/src/math/Matrix2.js`.

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Matrix2 {
    pub elements: [f64; 4],
}
