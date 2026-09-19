//! Ports of the parts of `three.js/src/extras/` the rungs use: the `Curve`
//! base class and the Catmull-Rom spline built on it, and `DataUtils`.

pub mod catmull_rom_curve3;
pub mod curve;
pub mod data_utils;

pub use catmull_rom_curve3::{CatmullRomCurve3, CurveType};
pub use curve::{Curve, FrenetFrames};
pub use data_utils::{from_half_float, to_half_float};
