//! Port of `three.js/src/extras/curves/SplineCurve.js`.

use super::curve::Curve;
use super::interpolations::catmull_rom;
use crate::math::Vector2;

/// `SplineCurve`: a smooth 2D spline through `points`, using the uniform
/// Catmull-Rom interpolation.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct SplineCurve {
    /// An array of 2D points defining the curve.
    pub points: Vec<Vector2>,
}

impl SplineCurve {
    /// `new SplineCurve( points )`.
    pub fn new(points: Vec<Vector2>) -> Self {
        Self { points }
    }
}

impl Curve for SplineCurve {
    type Point = Vector2;

    fn type_name(&self) -> &'static str {
        "SplineCurve"
    }

    /// `isSplineCurve`: the divisions times the point count.
    fn curve_path_resolution(&self, divisions: usize) -> usize {
        divisions * self.points.len()
    }

    fn get_point(&self, t: f64) -> Vector2 {
        let points = &self.points;
        let l = points.len() as f64;
        let p = (l - 1.0) * t;

        let int_point = p.floor();
        let weight = p - int_point;

        // three.js compares the float `intPoint` against `points.length - 2`
        // and `- 3`, which go negative for one or two points; the comparisons
        // are done in f64 here for the same reason.
        let i = int_point as usize;
        let last = points.len() - 1;
        let p0 = &points[if int_point == 0.0 { i } else { i - 1 }];
        let p1 = &points[i];
        let p2 = &points[if int_point > l - 2.0 { last } else { i + 1 }];
        let p3 = &points[if int_point > l - 3.0 { last } else { i + 2 }];

        Vector2::new(
            catmull_rom(weight, p0.x, p1.x, p2.x, p3.x),
            catmull_rom(weight, p0.y, p1.y, p2.y, p3.y),
        )
    }
}
