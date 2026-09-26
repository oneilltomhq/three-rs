//! Port of `three.js/src/extras/core/Path.js`.

use std::ops::{Deref, DerefMut};
use std::rc::Rc;

use super::bezier_curves::{CubicBezierCurve, QuadraticBezierCurve};
use super::curve::Curve;
use super::curve_path::{delegate_curve_path, CurvePath};
use super::ellipse_curve::EllipseCurve;
use super::line_curve::LineCurve;
use super::spline_curve::SplineCurve;
use crate::math::Vector2;

/// `Path`: a 2D [`CurvePath`] with a canvas-like drawing API (`moveTo`,
/// `lineTo`, `bezierCurveTo`, ...).
///
/// three.js' `Path extends CurvePath`; here a `Path` holds its `CurvePath`
/// and derefs to it, so `path.curves`, `path.auto_close` and
/// `path.close_path()` read as they do in JavaScript. The builder methods
/// return `&mut Self` for chaining, as three.js' return `this`.
#[derive(Clone, Debug, Default)]
pub struct Path {
    /// The inherited `CurvePath` state.
    pub curve_path: CurvePath<Vector2>,
    /// The current offset of the path. Any new curve added will start here.
    pub current_point: Vector2,
}

impl Deref for Path {
    type Target = CurvePath<Vector2>;
    fn deref(&self) -> &Self::Target {
        &self.curve_path
    }
}

impl DerefMut for Path {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.curve_path
    }
}

delegate_curve_path!(Path, curve_path, Vector2, "Path");

impl Path {
    /// `new Path()`.
    pub fn new() -> Self {
        Self::default()
    }

    /// `new Path( points )`: `setFromPoints( points )` on a new path.
    pub fn from_points(points: &[Vector2]) -> Self {
        let mut path = Self::new();
        path.set_from_points(points);
        path
    }

    /// `Path.setFromPoints( points )`: a `moveTo` to the first point and a
    /// `lineTo` to each of the rest.
    pub fn set_from_points(&mut self, points: &[Vector2]) -> &mut Self {
        self.move_to(points[0].x, points[0].y);

        for point in &points[1..] {
            self.line_to(point.x, point.y);
        }

        self
    }

    /// `Path.moveTo( x, y )`.
    pub fn move_to(&mut self, x: f64, y: f64) -> &mut Self {
        self.current_point.set(x, y); // TODO consider referencing vectors instead of copying?

        self
    }

    /// `Path.lineTo( x, y )`.
    pub fn line_to(&mut self, x: f64, y: f64) -> &mut Self {
        let curve = LineCurve::new(self.current_point, Vector2::new(x, y));
        self.curve_path.curves.push(Rc::new(curve));

        self.current_point.set(x, y);

        self
    }

    /// `Path.quadraticCurveTo( aCPx, aCPy, aX, aY )`.
    pub fn quadratic_curve_to(
        &mut self,
        a_c_px: f64,
        a_c_py: f64,
        a_x: f64,
        a_y: f64,
    ) -> &mut Self {
        let curve = QuadraticBezierCurve::new(
            self.current_point,
            Vector2::new(a_c_px, a_c_py),
            Vector2::new(a_x, a_y),
        );

        self.curve_path.curves.push(Rc::new(curve));

        self.current_point.set(a_x, a_y);

        self
    }

    /// `Path.bezierCurveTo( aCP1x, aCP1y, aCP2x, aCP2y, aX, aY )`.
    pub fn bezier_curve_to(
        &mut self,
        a_cp1x: f64,
        a_cp1y: f64,
        a_cp2x: f64,
        a_cp2y: f64,
        a_x: f64,
        a_y: f64,
    ) -> &mut Self {
        let curve = CubicBezierCurve::new(
            self.current_point,
            Vector2::new(a_cp1x, a_cp1y),
            Vector2::new(a_cp2x, a_cp2y),
            Vector2::new(a_x, a_y),
        );

        self.curve_path.curves.push(Rc::new(curve));

        self.current_point.set(a_x, a_y);

        self
    }

    /// `Path.splineThru( pts )`: a [`SplineCurve`] from the current point
    /// through `pts`.
    pub fn spline_thru(&mut self, pts: &[Vector2]) -> &mut Self {
        let mut npts = vec![self.current_point];
        npts.extend_from_slice(pts);

        let curve = SplineCurve::new(npts);
        self.curve_path.curves.push(Rc::new(curve));

        self.current_point.copy(&pts[pts.len() - 1]);

        self
    }

    /// `Path.arc( aX, aY, aRadius, aStartAngle, aEndAngle, aClockwise )`:
    /// [`absarc`](Self::absarc) with the center relative to the current point.
    pub fn arc(
        &mut self,
        a_x: f64,
        a_y: f64,
        a_radius: f64,
        a_start_angle: f64,
        a_end_angle: f64,
        a_clockwise: bool,
    ) -> &mut Self {
        let x0 = self.current_point.x;
        let y0 = self.current_point.y;

        self.absarc(
            a_x + x0,
            a_y + y0,
            a_radius,
            a_start_angle,
            a_end_angle,
            a_clockwise,
        );

        self
    }

    /// `Path.absarc( aX, aY, aRadius, aStartAngle, aEndAngle, aClockwise )`.
    pub fn absarc(
        &mut self,
        a_x: f64,
        a_y: f64,
        a_radius: f64,
        a_start_angle: f64,
        a_end_angle: f64,
        a_clockwise: bool,
    ) -> &mut Self {
        self.absellipse(
            a_x,
            a_y,
            a_radius,
            a_radius,
            a_start_angle,
            a_end_angle,
            a_clockwise,
            0.0,
        );

        self
    }

    /// `Path.ellipse( aX, aY, xRadius, yRadius, aStartAngle, aEndAngle,
    /// aClockwise, aRotation )`: [`absellipse`](Self::absellipse) with the
    /// center relative to the current point.
    #[allow(clippy::too_many_arguments)]
    pub fn ellipse(
        &mut self,
        a_x: f64,
        a_y: f64,
        x_radius: f64,
        y_radius: f64,
        a_start_angle: f64,
        a_end_angle: f64,
        a_clockwise: bool,
        a_rotation: f64,
    ) -> &mut Self {
        let x0 = self.current_point.x;
        let y0 = self.current_point.y;

        self.absellipse(
            a_x + x0,
            a_y + y0,
            x_radius,
            y_radius,
            a_start_angle,
            a_end_angle,
            a_clockwise,
            a_rotation,
        );

        self
    }

    /// `Path.absellipse( aX, aY, xRadius, yRadius, aStartAngle, aEndAngle,
    /// aClockwise, aRotation )`. Joins the previous curve to the ellipse's
    /// start with a line when they do not meet.
    #[allow(clippy::too_many_arguments)]
    pub fn absellipse(
        &mut self,
        a_x: f64,
        a_y: f64,
        x_radius: f64,
        y_radius: f64,
        a_start_angle: f64,
        a_end_angle: f64,
        a_clockwise: bool,
        a_rotation: f64,
    ) -> &mut Self {
        let curve = EllipseCurve::new(
            a_x,
            a_y,
            x_radius,
            y_radius,
            a_start_angle,
            a_end_angle,
            a_clockwise,
            a_rotation,
        );

        if !self.curve_path.curves.is_empty() {
            // if a previous curve is present, attempt to join
            let first_point = curve.get_point(0.0);

            if !first_point.equals(&self.current_point) {
                self.line_to(first_point.x, first_point.y);
            }
        }

        let last_point = curve.get_point(1.0);
        self.curve_path.curves.push(Rc::new(curve));

        self.current_point.copy(&last_point);

        self
    }
}
