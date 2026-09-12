//! Port of `three.js/src/math/Plane.js`.

use super::Vector3;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Plane {
    pub normal: Vector3,
    pub constant: f64,
}
