//! Port of `three.js/src/math/Triangle.js`.

use super::Vector3;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Triangle {
    pub a: Vector3,
    pub b: Vector3,
    pub c: Vector3,
}
