//! Port of `three.js/src/math/Ray.js`.

use super::Vector3;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Ray {
    pub origin: Vector3,
    pub direction: Vector3,
}
