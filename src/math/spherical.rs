//! Port of `three.js/src/math/Spherical.js`.

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Spherical {
    pub radius: f64,
    pub phi: f64,
    pub theta: f64,
}
