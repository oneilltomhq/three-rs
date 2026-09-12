//! Port of `three.js/src/math/Box3.js`.

use super::Vector3;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Box3 {
    pub min: Vector3,
    pub max: Vector3,
}
