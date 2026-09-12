//! Port of `three.js/src/math/Line3.js`.

use super::math_utils::clamp;
use super::{Matrix4, Vector3};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Line3 {
    pub start: Vector3,
    pub end: Vector3,
}

impl Default for Line3 {
    fn default() -> Self {
        Self {
            start: Vector3::ZERO,
            end: Vector3::ZERO,
        }
    }
}

impl Line3 {
    pub const fn new(start: Vector3, end: Vector3) -> Self {
        Self { start, end }
    }

    /// `Line3.set()`.
    pub fn set(&mut self, start: &Vector3, end: &Vector3) -> &mut Self {
        self.start.copy(start);
        self.end.copy(end);
        self
    }

    /// `Line3.copy()`.
    pub fn copy(&mut self, line: &Self) -> &mut Self {
        self.start.copy(&line.start);
        self.end.copy(&line.end);
        self
    }

    /// `Line3.getCenter()`.
    pub fn get_center(&self) -> Vector3 {
        let mut target = Vector3::default();
        target.add_vectors(&self.start, &self.end).multiply_scalar(0.5);
        target
    }

    /// `Line3.delta()`.
    pub fn delta(&self) -> Vector3 {
        let mut target = Vector3::default();
        target.sub_vectors(&self.end, &self.start);
        target
    }

    /// `Line3.distanceSq()`.
    pub fn distance_sq(&self) -> f64 {
        self.start.distance_to_squared(&self.end)
    }

    /// `Line3.distance()`.
    pub fn distance(&self) -> f64 {
        self.start.distance_to(&self.end)
    }

    /// `Line3.at()`.
    pub fn at(&self, t: f64) -> Vector3 {
        let mut target = self.delta();
        target.multiply_scalar(t).add(&self.start);
        target
    }

    /// `Line3.closestPointToPointParameter()`.
    pub fn closest_point_to_point_parameter(&self, point: &Vector3, clamp_to_line: bool) -> f64 {
        let mut start_p = Vector3::default();
        let mut start_end = Vector3::default();

        start_p.sub_vectors(point, &self.start);
        start_end.sub_vectors(&self.end, &self.start);

        let start_end2 = start_end.dot(&start_end);

        if start_end2 == 0.0 {
            return 0.0;
        }

        let start_end_start_p = start_end.dot(&start_p);

        let mut t = start_end_start_p / start_end2;

        if clamp_to_line {
            t = clamp(t, 0.0, 1.0);
        }

        t
    }

    /// `Line3.closestPointToPoint()`.
    pub fn closest_point_to_point(&self, point: &Vector3, clamp_to_line: bool) -> Vector3 {
        let t = self.closest_point_to_point_parameter(point, clamp_to_line);

        let mut target = self.delta();
        target.multiply_scalar(t).add(&self.start);
        target
    }

    /// `Line3.distanceSqToLine3()`. `c1`/`c2` are three.js' optional out-params:
    /// the closest point on this line segment and on the given one.
    pub fn distance_sq_to_line3(
        &self,
        line: &Self,
        c1: Option<&mut Vector3>,
        c2: Option<&mut Vector3>,
    ) -> f64 {
        // from Real-Time Collision Detection by Christer Ericson, chapter 5.1.9

        // Computes closest points C1 and C2 of S1(s)=P1+s*(Q1-P1) and
        // S2(t)=P2+t*(Q2-P2), returning s and t. Function result is squared
        // distance between between S1(s) and S2(t)

        const EPSILON: f64 = 1e-8 * 1e-8; // must be squared since we compare squared length
        let s: f64;
        let mut t: f64;

        let mut out1 = Vector3::default();
        let mut out2 = Vector3::default();

        let p1 = self.start;
        let p2 = line.start;
        let q1 = self.end;
        let q2 = line.end;

        let mut d1 = Vector3::default();
        let mut d2 = Vector3::default();
        let mut r = Vector3::default();

        d1.sub_vectors(&q1, &p1); // Direction vector of segment S1
        d2.sub_vectors(&q2, &p2); // Direction vector of segment S2
        r.sub_vectors(&p1, &p2);

        let a = d1.dot(&d1); // Squared length of segment S1, always nonnegative
        let e = d2.dot(&d2); // Squared length of segment S2, always nonnegative
        let f = d2.dot(&r);

        // Check if either or both segments degenerate into points

        if a <= EPSILON && e <= EPSILON {
            // Both segments degenerate into points

            out1.copy(&p1);
            out2.copy(&p2);

            out1.sub(&out2);

            let result = out1.dot(&out1);

            if let Some(c1) = c1 {
                c1.copy(&out1);
            }
            if let Some(c2) = c2 {
                c2.copy(&out2);
            }

            return result;
        }

        if a <= EPSILON {
            // First segment degenerates into a point

            s = 0.0;
            t = f / e; // s = 0 => t = (b*s + f) / e = f / e
            t = clamp(t, 0.0, 1.0);
        } else {
            let c = d1.dot(&r);

            if e <= EPSILON {
                // Second segment degenerates into a point

                t = 0.0;
                s = clamp(-c / a, 0.0, 1.0); // t = 0 => s = (b*t - c) / a = -c / a
            } else {
                // The general nondegenerate case starts here

                let b = d1.dot(&d2);
                let denom = a * e - b * b; // Always nonnegative

                // If segments not parallel, compute closest point on L1 to L2 and
                // clamp to segment S1. Else pick arbitrary s (here 0)

                let mut s_tmp = if denom != 0.0 {
                    clamp((b * f - c * e) / denom, 0.0, 1.0)
                } else {
                    0.0
                };

                // Compute point on L2 closest to S1(s) using
                // t = Dot((P1 + D1*s) - P2,D2) / Dot(D2,D2) = (b*s + f) / e

                t = (b * s_tmp + f) / e;

                // If t in [0,1] done. Else clamp t, recompute s for the new value
                // of t using s = Dot((P2 + D2*t) - P1,D1) / Dot(D1,D1)= (t*b - c) / a
                // and clamp s to [0, 1]

                if t < 0.0 {
                    t = 0.0;
                    s_tmp = clamp(-c / a, 0.0, 1.0);
                } else if t > 1.0 {
                    t = 1.0;
                    s_tmp = clamp((b - c) / a, 0.0, 1.0);
                }

                s = s_tmp;
            }
        }

        out1.copy(&p1).add_scaled_vector(&d1, s);
        out2.copy(&p2).add_scaled_vector(&d2, t);

        let result = out1.distance_to_squared(&out2);

        if let Some(c1) = c1 {
            c1.copy(&out1);
        }
        if let Some(c2) = c2 {
            c2.copy(&out2);
        }

        result
    }

    /// `Line3.applyMatrix4()`.
    pub fn apply_matrix4(&mut self, matrix: &Matrix4) -> &mut Self {
        self.start.apply_matrix4(matrix);
        self.end.apply_matrix4(matrix);
        self
    }

    /// `Line3.equals()`.
    pub fn equals(&self, line: &Self) -> bool {
        line.start.equals(&self.start) && line.end.equals(&self.end)
    }
}
