//! Port of `three.js/src/extras/curves/LineCurve.js` and `LineCurve3.js`.

use super::curve::{Curve, CurveVector};
use crate::math::{Vector2, Vector3};

/// `LineCurve` (`P = Vector2`) and `LineCurve3` (`P = Vector3`): a straight
/// segment from `v1` to `v2`. The two three.js classes are the same code over
/// a different vector, so they are one generic struct here.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct LineCurve<P> {
    /// The start point.
    pub v1: P,
    /// The end point.
    pub v2: P,
}

/// `LineCurve3`.
pub type LineCurve3 = LineCurve<Vector3>;

impl<P> LineCurve<P> {
    /// `new LineCurve( v1, v2 )` / `new LineCurve3( v1, v2 )`.
    pub fn new(v1: P, v2: P) -> Self {
        Self { v1, v2 }
    }
}

/// The arithmetic `LineCurve.getPoint` / `getTangent` need beyond
/// [`CurveVector`]; implemented for `Vector2` and `Vector3`.
pub trait LineVector: CurveVector {
    /// `point.copy( v2 ).sub( v1 ); point.multiplyScalar( t ).add( v1 )`.
    fn lerp_line(v1: &Self, v2: &Self, t: f64) -> Self;
    /// `optionalTarget.subVectors( v2, v1 ).normalize()`.
    fn direction(v1: &Self, v2: &Self) -> Self;
    /// `'LineCurve'` or `'LineCurve3'`.
    const TYPE_NAME: &'static str;
}

macro_rules! line_vector {
    ($v:ty, $name:literal) => {
        impl LineVector for $v {
            fn lerp_line(v1: &Self, v2: &Self, t: f64) -> Self {
                let mut point = *v2;
                point.sub(v1);
                point.multiply_scalar(t).add(v1);
                point
            }
            fn direction(v1: &Self, v2: &Self) -> Self {
                let mut target = <$v>::default();
                target.sub_vectors(v2, v1).normalize();
                target
            }
            const TYPE_NAME: &'static str = $name;
        }
    };
}

line_vector!(Vector2, "LineCurve");
line_vector!(Vector3, "LineCurve3");

impl<P: LineVector> Curve for LineCurve<P> {
    type Point = P;

    fn type_name(&self) -> &'static str {
        P::TYPE_NAME
    }

    /// `isLineCurve` / `isLineCurve3`: a single division.
    fn curve_path_resolution(&self, _divisions: usize) -> usize {
        1
    }

    fn get_point(&self, t: f64) -> P {
        if t == 1.0 {
            self.v2
        } else {
            P::lerp_line(&self.v1, &self.v2, t)
        }
    }

    /// Line curves do not need an arc length mapping.
    fn get_point_at(&self, u: f64) -> P {
        self.get_point(u)
    }

    fn get_tangent(&self, _t: f64) -> P {
        P::direction(&self.v1, &self.v2)
    }

    fn get_tangent_at(&self, u: f64) -> P {
        self.get_tangent(u)
    }
}
