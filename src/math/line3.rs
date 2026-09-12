//! Port of `three.js/src/math/Line3.js`.

use super::Vector3;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Line3 {
    pub start: Vector3,
    pub end: Vector3,
}
