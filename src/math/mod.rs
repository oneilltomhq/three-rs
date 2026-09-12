//! Ports of `three.js/src/math`.

mod color;
mod euler;
mod matrix3;
mod matrix4;
pub mod math_utils;
mod quaternion;
mod vector2;
mod vector3;
mod vector4;

pub use color::{linear_to_srgb, srgb_to_linear, Color, ColorSpace, Hsl};
pub use euler::{Euler, EulerOrder};
pub use math_utils::{DEG2RAD, RAD2DEG};
pub use matrix3::Matrix3;
pub use matrix4::{CoordinateSystem, Matrix4};
pub use quaternion::Quaternion;
pub use vector2::Vector2;
pub use vector3::Vector3;
pub use vector4::Vector4;
