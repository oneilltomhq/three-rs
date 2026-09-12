//! Ports of `three.js/src/math`.

mod box2;
mod box3;
mod color;
mod color_management;
mod cylindrical;
mod euler;
mod frustum;
mod line3;
mod matrix2;
pub mod interpolant;
pub mod interpolants;
mod matrix3;
mod matrix4;
pub mod math_utils;
mod plane;
mod quaternion;
mod ray;
mod sphere;
mod spherical;
mod spherical_harmonics3;
mod triangle;
mod vector2;
mod vector3;
mod vector4;

pub use box2::Box2;
pub use box3::Box3;
pub use color::{linear_to_srgb, srgb_to_linear, Color, ColorSpace, Hsl};
pub use color_management::ColorManagement;
pub use cylindrical::Cylindrical;
pub use euler::{Euler, EulerOrder};
pub use frustum::Frustum;
pub use line3::Line3;
pub use interpolant::{Ending, Interpolant, InterpolantData, InterpolantSettings, Interpolation};
pub use interpolants::{
    cubic_interpolant, discrete_interpolant, linear_interpolant, quaternion_linear_interpolant,
    CubicInterpolant, DiscreteInterpolant, LinearInterpolant, QuaternionLinearInterpolant,
};
pub use math_utils::{DEG2RAD, RAD2DEG};
pub use matrix2::Matrix2;
pub use matrix3::Matrix3;
pub use matrix4::{CoordinateSystem, Matrix4};
pub use plane::Plane;
pub use quaternion::Quaternion;
pub use ray::Ray;
pub use sphere::Sphere;
pub use spherical::Spherical;
pub use spherical_harmonics3::SphericalHarmonics3;
pub use triangle::Triangle;
pub use vector2::Vector2;
pub use vector3::Vector3;
pub use vector4::Vector4;
