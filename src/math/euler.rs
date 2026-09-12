//! Port of `three.js/src/math/Euler.js` (rung 1 subset: XYZ order only, which
//! is three.js' default).

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum EulerOrder {
    #[default]
    XYZ,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Euler {
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub order: EulerOrder,
}

impl Euler {
    pub fn new(x: f64, y: f64, z: f64) -> Self {
        Self { x, y, z, order: EulerOrder::XYZ }
    }

    pub fn set(&mut self, x: f64, y: f64, z: f64) -> &mut Self {
        self.x = x;
        self.y = y;
        self.z = z;
        self
    }
}
