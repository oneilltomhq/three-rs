//! Ports of `three.js/src/math`.

mod color;
mod euler;
mod matrix3;
mod matrix4;
mod quaternion;
mod vector3;

pub use color::{srgb_to_linear, Color};
pub use euler::{Euler, EulerOrder};
pub use matrix3::Matrix3;
pub use matrix4::{CoordinateSystem, Matrix4};
pub use quaternion::Quaternion;
pub use vector3::Vector3;

/// `MathUtils.DEG2RAD`.
pub const DEG2RAD: f64 = std::f64::consts::PI / 180.0;
