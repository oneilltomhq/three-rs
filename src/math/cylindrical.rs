//! Port of `three.js/src/math/Cylindrical.js`.

use super::Vector3;

/// Represents points in 3D space as cylindrical coordinates.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Cylindrical {
    pub radius: f64,
    pub theta: f64,
    pub y: f64,
}

impl Default for Cylindrical {
    /// `new Cylindrical()`: radius 1, theta 0, y 0.
    fn default() -> Self {
        Self::new(1.0, 0.0, 0.0)
    }
}

impl Cylindrical {
    pub const fn new(radius: f64, theta: f64, y: f64) -> Self {
        Self { radius, theta, y }
    }

    /// `Cylindrical.set()`.
    pub fn set(&mut self, radius: f64, theta: f64, y: f64) -> &mut Self {
        self.radius = radius;
        self.theta = theta;
        self.y = y;

        self
    }

    /// `Cylindrical.copy()`.
    pub fn copy(&mut self, other: &Self) -> &mut Self {
        self.radius = other.radius;
        self.theta = other.theta;
        self.y = other.y;

        self
    }

    /// `Cylindrical.setFromVector3()`.
    pub fn set_from_vector3(&mut self, v: &Vector3) -> &mut Self {
        self.set_from_cartesian_coords(v.x, v.y, v.z)
    }

    /// `Cylindrical.setFromCartesianCoords()`.
    pub fn set_from_cartesian_coords(&mut self, x: f64, y: f64, z: f64) -> &mut Self {
        self.radius = (x * x + z * z).sqrt();
        self.theta = x.atan2(z);
        self.y = y;

        self
    }
}
