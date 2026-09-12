//! Port of `three.js/src/math/Sphere.js`.

use super::Vector3;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Sphere {
    pub center: Vector3,
    pub radius: f64,
}
