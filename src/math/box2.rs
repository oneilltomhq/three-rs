//! Port of `three.js/src/math/Box2.js`.

use super::Vector2;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Box2 {
    pub min: Vector2,
    pub max: Vector2,
}
