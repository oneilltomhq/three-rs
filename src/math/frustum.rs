//! Port of `three.js/src/math/Frustum.js`.

use super::Plane;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Frustum {
    pub planes: [Plane; 6],
}
