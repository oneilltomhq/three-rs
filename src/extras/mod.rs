//! Ports of the parts of `three.js/src/extras/` the rungs use: the `Curve`
//! base class and the Catmull-Rom spline built on it.

pub mod catmull_rom_curve3;
pub mod curve;

pub use catmull_rom_curve3::{CatmullRomCurve3, CurveType};
pub use curve::{Curve, FrenetFrames};
