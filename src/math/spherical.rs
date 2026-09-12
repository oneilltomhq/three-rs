//! Port of `three.js/src/math/Spherical.js`.

use super::math_utils::clamp;
use super::Vector3;

/// Represents points in 3D space as spherical coordinates.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Spherical {
    pub radius: f64,
    pub phi: f64,
    pub theta: f64,
}

impl Default for Spherical {
    /// `new Spherical()`: radius 1, phi 0, theta 0.
    fn default() -> Self {
        Self::new(1.0, 0.0, 0.0)
    }
}

impl Spherical {
    pub const fn new(radius: f64, phi: f64, theta: f64) -> Self {
        Self { radius, phi, theta }
    }

    /// `Spherical.set()`.
    pub fn set(&mut self, radius: f64, phi: f64, theta: f64) -> &mut Self {
        self.radius = radius;
        self.phi = phi;
        self.theta = theta;

        self
    }

    /// `Spherical.copy()`.
    pub fn copy(&mut self, other: &Self) -> &mut Self {
        self.radius = other.radius;
        self.phi = other.phi;
        self.theta = other.theta;

        self
    }

    /// `Spherical.makeSafe()`: restricts `phi` to `[ EPS, PI - EPS ]`.
    pub fn make_safe(&mut self) -> &mut Self {
        const EPS: f64 = 0.000001;
        self.phi = clamp(self.phi, EPS, std::f64::consts::PI - EPS);

        self
    }

    /// `Spherical.setFromVector3()`.
    pub fn set_from_vector3(&mut self, v: &Vector3) -> &mut Self {
        self.set_from_cartesian_coords(v.x, v.y, v.z)
    }

    /// `Spherical.setFromCartesianCoords()`.
    pub fn set_from_cartesian_coords(&mut self, x: f64, y: f64, z: f64) -> &mut Self {
        self.radius = (x * x + y * y + z * z).sqrt();

        if self.radius == 0.0 {
            self.theta = 0.0;
            self.phi = 0.0;
        } else {
            self.theta = x.atan2(z);
            self.phi = clamp(y / self.radius, -1.0, 1.0).acos();
        }

        self
    }
}
