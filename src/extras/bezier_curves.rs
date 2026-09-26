//! Ports of `three.js/src/extras/curves/QuadraticBezierCurve.js`,
//! `QuadraticBezierCurve3.js`, `CubicBezierCurve.js` and
//! `CubicBezierCurve3.js`.
//!
//! The 2D and 3D classes are the same code over a different vector, so each
//! pair is one generic struct here.

use super::curve::{Curve, CurveVector};
use super::interpolations::{cubic_bezier, quadratic_bezier};
use crate::math::{Vector2, Vector3};

/// Applies a scalar interpolation per component; implemented for `Vector2`
/// and `Vector3`.
pub trait BezierVector: CurveVector {
    /// `point.set( f( v0.x, ... ), f( v0.y, ... )[, f( v0.z, ... )] )`.
    fn map_components(points: &[&Self], f: impl Fn(&[f64]) -> f64) -> Self;
    /// `'QuadraticBezierCurve'` / `'QuadraticBezierCurve3'`.
    const QUADRATIC_TYPE_NAME: &'static str;
    /// `'CubicBezierCurve'` / `'CubicBezierCurve3'`.
    const CUBIC_TYPE_NAME: &'static str;
}

impl BezierVector for Vector2 {
    fn map_components(points: &[&Self], f: impl Fn(&[f64]) -> f64) -> Self {
        let xs: Vec<f64> = points.iter().map(|p| p.x).collect();
        let ys: Vec<f64> = points.iter().map(|p| p.y).collect();
        Vector2::new(f(&xs), f(&ys))
    }
    const QUADRATIC_TYPE_NAME: &'static str = "QuadraticBezierCurve";
    const CUBIC_TYPE_NAME: &'static str = "CubicBezierCurve";
}

impl BezierVector for Vector3 {
    fn map_components(points: &[&Self], f: impl Fn(&[f64]) -> f64) -> Self {
        let xs: Vec<f64> = points.iter().map(|p| p.x).collect();
        let ys: Vec<f64> = points.iter().map(|p| p.y).collect();
        let zs: Vec<f64> = points.iter().map(|p| p.z).collect();
        Vector3::new(f(&xs), f(&ys), f(&zs))
    }
    const QUADRATIC_TYPE_NAME: &'static str = "QuadraticBezierCurve3";
    const CUBIC_TYPE_NAME: &'static str = "CubicBezierCurve3";
}

/// `QuadraticBezierCurve` (`P = Vector2`) and `QuadraticBezierCurve3`
/// (`P = Vector3`).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct QuadraticBezierCurve<P> {
    /// The start point.
    pub v0: P,
    /// The control point.
    pub v1: P,
    /// The end point.
    pub v2: P,
}

/// `QuadraticBezierCurve3`.
pub type QuadraticBezierCurve3 = QuadraticBezierCurve<Vector3>;

impl<P> QuadraticBezierCurve<P> {
    /// `new QuadraticBezierCurve( v0, v1, v2 )`.
    pub fn new(v0: P, v1: P, v2: P) -> Self {
        Self { v0, v1, v2 }
    }
}

impl<P: BezierVector> Curve for QuadraticBezierCurve<P> {
    type Point = P;

    fn type_name(&self) -> &'static str {
        P::QUADRATIC_TYPE_NAME
    }

    fn get_point(&self, t: f64) -> P {
        P::map_components(&[&self.v0, &self.v1, &self.v2], |c| {
            quadratic_bezier(t, c[0], c[1], c[2])
        })
    }
}

/// `CubicBezierCurve` (`P = Vector2`) and `CubicBezierCurve3` (`P = Vector3`).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct CubicBezierCurve<P> {
    /// The start point.
    pub v0: P,
    /// The first control point.
    pub v1: P,
    /// The second control point.
    pub v2: P,
    /// The end point.
    pub v3: P,
}

/// `CubicBezierCurve3`.
pub type CubicBezierCurve3 = CubicBezierCurve<Vector3>;

impl<P> CubicBezierCurve<P> {
    /// `new CubicBezierCurve( v0, v1, v2, v3 )`.
    pub fn new(v0: P, v1: P, v2: P, v3: P) -> Self {
        Self { v0, v1, v2, v3 }
    }
}

impl<P: BezierVector> Curve for CubicBezierCurve<P> {
    type Point = P;

    fn type_name(&self) -> &'static str {
        P::CUBIC_TYPE_NAME
    }

    fn get_point(&self, t: f64) -> P {
        P::map_components(&[&self.v0, &self.v1, &self.v2, &self.v3], |c| {
            cubic_bezier(t, c[0], c[1], c[2], c[3])
        })
    }
}
