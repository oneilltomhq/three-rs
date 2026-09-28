//! Ports of `three.js/src/extras/`: the `Curve` base class and its 2D / 3D
//! subclasses, `CurvePath` / `Path` / `Shape` / `ShapePath`, `Earcut` and
//! `ShapeUtils`, and `DataUtils`.

pub(crate) mod bezier_curves;
pub(crate) mod catmull_rom_curve3;
pub(crate) mod curve;
pub(crate) mod curve_path;
pub(crate) mod data_utils;
#[doc(hidden)]
pub mod earcut;
pub(crate) mod ellipse_curve;
pub(crate) mod interpolations;
pub(crate) mod line_curve;
pub(crate) mod path;
pub(crate) mod shape;
pub(crate) mod shape_path;
pub mod shape_utils;
pub(crate) mod spline_curve;

pub use bezier_curves::{
    BezierVector, CubicBezierCurve, CubicBezierCurve3, QuadraticBezierCurve, QuadraticBezierCurve3,
};
pub use catmull_rom_curve3::{CatmullRomCurve3, CurveType};
pub use curve::{Curve, CurveVector, FrenetFrames};
pub use curve_path::{CurvePath, CurveRef};
pub use data_utils::{from_half_float, to_half_float};
pub use ellipse_curve::EllipseCurve;
pub use line_curve::{LineCurve, LineCurve3, LineVector};
pub use path::Path;
pub use shape::{Shape, ShapePoints};
pub use shape_path::{FillRule, ShapePath};
pub use spline_curve::SplineCurve;
