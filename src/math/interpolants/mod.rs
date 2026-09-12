//! Ports of `three.js/src/math/interpolants`.

mod cubic;
mod discrete;
mod linear;
mod quaternion_linear;

pub use cubic::{cubic_interpolant, CubicInterpolant, CubicInterpolation};
pub use discrete::{discrete_interpolant, DiscreteInterpolant, DiscreteInterpolation};
pub use linear::{linear_interpolant, LinearInterpolant, LinearInterpolation};
pub use quaternion_linear::{
    quaternion_linear_interpolant, QuaternionLinearInterpolant, QuaternionLinearInterpolation,
};
