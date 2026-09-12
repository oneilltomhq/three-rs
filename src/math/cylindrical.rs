//! Port of `three.js/src/math/Cylindrical.js`.

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Cylindrical {
    pub radius: f64,
    pub theta: f64,
    pub y: f64,
}
