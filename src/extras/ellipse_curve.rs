//! Port of `three.js/src/extras/curves/EllipseCurve.js` and `ArcCurve.js`.

use super::curve::Curve;
use crate::math::Vector2;

/// `EllipseCurve`: a 2D ellipse, or an arc of one.
///
/// three.js' `ArcCurve` is a subclass that only fixes `xRadius = yRadius` and
/// changes `type`; here it is the same struct built by [`EllipseCurve::arc`],
/// with `is_arc` carrying the brand.
#[derive(Clone, Debug, PartialEq)]
pub struct EllipseCurve {
    /// The X center of the ellipse.
    pub a_x: f64,
    /// The Y center of the ellipse.
    pub a_y: f64,
    /// The radius of the ellipse in the x direction.
    pub x_radius: f64,
    /// The radius of the ellipse in the y direction.
    pub y_radius: f64,
    /// The start angle of the curve in radians starting from the positive X axis.
    pub a_start_angle: f64,
    /// The end angle of the curve in radians starting from the positive X axis.
    pub a_end_angle: f64,
    /// Whether the ellipse is drawn clockwise or not.
    pub a_clockwise: bool,
    /// The rotation angle of the ellipse in radians, counterclockwise from the
    /// positive X axis.
    pub a_rotation: f64,
    /// `isArcCurve`: built by [`EllipseCurve::arc`].
    pub is_arc: bool,
}

impl Default for EllipseCurve {
    /// `new EllipseCurve()`: the unit circle, counterclockwise from angle 0.
    fn default() -> Self {
        Self::new(
            0.0,
            0.0,
            1.0,
            1.0,
            0.0,
            std::f64::consts::PI * 2.0,
            false,
            0.0,
        )
    }
}

impl EllipseCurve {
    /// `new EllipseCurve( aX, aY, xRadius, yRadius, aStartAngle, aEndAngle,
    /// aClockwise, aRotation )`.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        a_x: f64,
        a_y: f64,
        x_radius: f64,
        y_radius: f64,
        a_start_angle: f64,
        a_end_angle: f64,
        a_clockwise: bool,
        a_rotation: f64,
    ) -> Self {
        Self {
            a_x,
            a_y,
            x_radius,
            y_radius,
            a_start_angle,
            a_end_angle,
            a_clockwise,
            a_rotation,
            is_arc: false,
        }
    }

    /// `new ArcCurve( aX, aY, aRadius, aStartAngle, aEndAngle, aClockwise )`.
    pub fn arc(
        a_x: f64,
        a_y: f64,
        a_radius: f64,
        a_start_angle: f64,
        a_end_angle: f64,
        a_clockwise: bool,
    ) -> Self {
        Self {
            is_arc: true,
            ..Self::new(
                a_x,
                a_y,
                a_radius,
                a_radius,
                a_start_angle,
                a_end_angle,
                a_clockwise,
                0.0,
            )
        }
    }
}

impl Curve for EllipseCurve {
    type Point = Vector2;

    fn type_name(&self) -> &'static str {
        if self.is_arc {
            "ArcCurve"
        } else {
            "EllipseCurve"
        }
    }

    /// `isEllipseCurve` (true for an `ArcCurve` too): twice the divisions.
    fn curve_path_resolution(&self, divisions: usize) -> usize {
        divisions * 2
    }

    fn get_point(&self, t: f64) -> Vector2 {
        let mut point = Vector2::default();

        let two_pi = std::f64::consts::PI * 2.0;
        let mut delta_angle = self.a_end_angle - self.a_start_angle;
        let same_points = delta_angle.abs() < f64::EPSILON;

        // ensures that deltaAngle is 0 .. 2 PI
        while delta_angle < 0.0 {
            delta_angle += two_pi;
        }
        while delta_angle > two_pi {
            delta_angle -= two_pi;
        }

        if delta_angle < f64::EPSILON {
            if same_points {
                delta_angle = 0.0;
            } else {
                delta_angle = two_pi;
            }
        }

        if self.a_clockwise && !same_points {
            if delta_angle == two_pi {
                delta_angle = -two_pi;
            } else {
                delta_angle -= two_pi;
            }
        }

        let angle = self.a_start_angle + t * delta_angle;
        let mut x = self.a_x + self.x_radius * angle.cos();
        let mut y = self.a_y + self.y_radius * angle.sin();

        if self.a_rotation != 0.0 {
            let cos = self.a_rotation.cos();
            let sin = self.a_rotation.sin();

            let tx = x - self.a_x;
            let ty = y - self.a_y;

            // Rotate the point about the center of the ellipse.
            x = tx * cos - ty * sin + self.a_x;
            y = tx * sin + ty * cos + self.a_y;
        }

        point.set(x, y);
        point
    }
}
