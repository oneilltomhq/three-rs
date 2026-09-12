//! Port of `three.js/src/math/SphericalHarmonics3.js`.

use super::Vector3;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SphericalHarmonics3 {
    pub coefficients: [Vector3; 9],
}
