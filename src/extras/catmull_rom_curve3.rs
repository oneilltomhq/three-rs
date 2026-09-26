//! Port of `three.js/src/extras/curves/CatmullRomCurve3.js`.
//!
//! All arithmetic is `f64`, matching JavaScript number semantics, and every
//! expression keeps three.js' own order and grouping so the rounding matches
//! bit for bit.

use super::curve::Curve;
use crate::math::Vector3;

/// `CatmullRomCurve3.curveType`, three.js' `'centripetal' | 'chordal' |
/// `'catmullrom'` string.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum CurveType {
    /// `'centripetal'`, three.js' default.
    #[default]
    Centripetal,
    /// `'chordal'`.
    Chordal,
    /// `'catmullrom'`.
    CatmullRom,
}

/// Centripetal CatmullRom Curve - which is useful for avoiding cusps and
/// self-intersections in non-uniform catmull rom curves.
/// <http://www.cemyuksel.com/research/catmullrom_param/catmullrom.pdf>
///
/// Based on an optimized c++ solution in
/// - <http://stackoverflow.com/questions/9489736/catmull-rom-curve-with-no-cusps-and-no-self-intersections/>
/// - <http://ideone.com/NoEbVM>
///
/// three.js keeps three module-level `CubicPoly` singletons (`px`, `py`, `pz`)
/// and re-initialises them on every `getPoint` call; that is a
/// mutation-reuse trick to avoid allocating, not observable behaviour. Nothing
/// reads a `CubicPoly` before it is initialised on the path taken here, so
/// plain local values are equivalent and correct.
#[derive(Clone, Copy, Debug, Default)]
struct CubicPoly {
    c0: f64,
    c1: f64,
    c2: f64,
    c3: f64,
}

impl CubicPoly {
    /// Compute coefficients for a cubic polynomial
    ///   `p(s) = c0 + c1*s + c2*s^2 + c3*s^3`
    /// such that
    ///   `p(0) = x0`, `p(1) = x1`
    /// and
    ///   `p'(0) = t0`, `p'(1) = t1`.
    fn init(&mut self, x0: f64, x1: f64, t0: f64, t1: f64) {
        self.c0 = x0;
        self.c1 = t0;
        self.c2 = -3.0 * x0 + 3.0 * x1 - 2.0 * t0 - t1;
        self.c3 = 2.0 * x0 - 2.0 * x1 + t0 + t1;
    }

    /// `CubicPoly.initCatmullRom()`.
    fn init_catmull_rom(&mut self, x0: f64, x1: f64, x2: f64, x3: f64, tension: f64) {
        self.init(x1, x2, tension * (x2 - x0), tension * (x3 - x1));
    }

    /// `CubicPoly.initNonuniformCatmullRom()`.
    #[allow(clippy::too_many_arguments)]
    fn init_non_uniform_catmull_rom(
        &mut self,
        x0: f64,
        x1: f64,
        x2: f64,
        x3: f64,
        dt0: f64,
        dt1: f64,
        dt2: f64,
    ) {
        // compute tangents when parameterized in [t1,t2]
        let mut t1 = (x1 - x0) / dt0 - (x2 - x0) / (dt0 + dt1) + (x2 - x1) / dt1;
        let mut t2 = (x2 - x1) / dt1 - (x3 - x1) / (dt1 + dt2) + (x3 - x2) / dt2;

        // rescale tangents for parametrization in [0,1]
        t1 *= dt1;
        t2 *= dt1;

        self.init(x1, x2, t1, t2);
    }

    /// `CubicPoly.calc()`.
    fn calc(&self, t: f64) -> f64 {
        let t2 = t * t;
        let t3 = t2 * t;
        self.c0 + self.c1 * t + self.c2 * t2 + self.c3 * t3
    }
}

/// `CatmullRomCurve3`: a curve representing a Catmull-Rom spline.
///
/// No `Default` impl: three.js' constructor defaults `tension` to 0.5, which
/// `#[derive(Default)]` could not give; use [`CatmullRomCurve3::new`].
#[derive(Clone, Debug)]
pub struct CatmullRomCurve3 {
    /// An array of 3D points defining the curve.
    pub points: Vec<Vector3>,
    /// Whether the curve is closed or not.
    pub closed: bool,
    /// The curve type.
    pub curve_type: CurveType,
    /// Tension of the curve; only used by [`CurveType::CatmullRom`].
    pub tension: f64,
}

impl CatmullRomCurve3 {
    /// `new CatmullRomCurve3( points )`: not closed, centripetal, tension 0.5.
    pub fn new(points: Vec<Vector3>) -> Self {
        Self {
            points,
            closed: false,
            curve_type: CurveType::Centripetal,
            tension: 0.5,
        }
    }
}

impl Curve for CatmullRomCurve3 {
    type Point = Vector3;

    fn type_name(&self) -> &'static str {
        "CatmullRomCurve3"
    }

    fn is_closed_catmull_rom(&self) -> bool {
        self.closed
    }

    /// `CatmullRomCurve3.getPoint( t, optionalTarget )`.
    fn get_point(&self, t: f64) -> Vector3 {
        let mut point = Vector3::default();

        let points = &self.points;
        let l = points.len();

        let p = (l as f64 - if self.closed { 0.0 } else { 1.0 }) * t;
        let mut int_point = p.floor();
        let mut weight = p - int_point;

        if self.closed {
            int_point += if int_point > 0.0 {
                0.0
            } else {
                ((int_point.abs() / l as f64).floor() + 1.0) * l as f64
            };
        } else if weight == 0.0 && int_point == l as f64 - 1.0 {
            int_point = l as f64 - 2.0;
            weight = 1.0;
        }

        // three.js indexes with JS `%`, which keeps the sign of the dividend and
        // so differs from Rust's `%` for a negative left operand. It cannot be
        // negative here: the `points[ ( intPoint - 1 ) % l ]` line only runs when
        // `this.closed || intPoint > 0`, and when `closed` is true the branch
        // above has already added a positive multiple of `l` to any `intPoint`
        // that was <= 0, leaving `intPoint >= l - ...` i.e. `>= 0`. In the other
        // case `intPoint > 0` is the guard itself. So `intPoint - 1 >= 0`
        // everywhere the expression is evaluated, and Rust's `%` agrees with
        // JS'. (Getting this wrong is a silently wrong shape on a closed curve,
        // not a panic.)
        let int_point = int_point as isize;
        let li = l as isize;

        // 4 points (p1 & p2 defined below). `tmp` / `tmp2` stand in for three.js'
        // two module-level scratch vectors of the same names.
        let tmp2;

        let p0 = if self.closed || int_point > 0 {
            &points[((int_point - 1) % li) as usize]
        } else {
            // extrapolate first point
            let mut v = Vector3::default();
            v.sub_vectors(&points[0], &points[1]).add(&points[0]);
            tmp2 = v;
            &tmp2
        };

        let p1 = &points[(int_point % li) as usize];
        let p2 = &points[((int_point + 1) % li) as usize];

        let tmp;

        let p3 = if self.closed || int_point + 2 < li {
            &points[((int_point + 2) % li) as usize]
        } else {
            // extrapolate last point
            let mut v = Vector3::default();
            v.sub_vectors(&points[l - 1], &points[l - 2])
                .add(&points[l - 1]);
            tmp = v;
            &tmp
        };

        let mut px = CubicPoly::default();
        let mut py = CubicPoly::default();
        let mut pz = CubicPoly::default();

        match self.curve_type {
            CurveType::Centripetal | CurveType::Chordal => {
                // init Centripetal / Chordal Catmull-Rom
                let pow = if self.curve_type == CurveType::Chordal {
                    0.5
                } else {
                    0.25
                };
                let mut dt0 = p0.distance_to_squared(p1).powf(pow);
                let mut dt1 = p1.distance_to_squared(p2).powf(pow);
                let mut dt2 = p2.distance_to_squared(p3).powf(pow);

                // safety check for repeated points
                if dt1 < 1e-4 {
                    dt1 = 1.0;
                }
                if dt0 < 1e-4 {
                    dt0 = dt1;
                }
                if dt2 < 1e-4 {
                    dt2 = dt1;
                }

                px.init_non_uniform_catmull_rom(p0.x, p1.x, p2.x, p3.x, dt0, dt1, dt2);
                py.init_non_uniform_catmull_rom(p0.y, p1.y, p2.y, p3.y, dt0, dt1, dt2);
                pz.init_non_uniform_catmull_rom(p0.z, p1.z, p2.z, p3.z, dt0, dt1, dt2);
            }
            CurveType::CatmullRom => {
                px.init_catmull_rom(p0.x, p1.x, p2.x, p3.x, self.tension);
                py.init_catmull_rom(p0.y, p1.y, p2.y, p3.y, self.tension);
                pz.init_catmull_rom(p0.z, p1.z, p2.z, p3.z, self.tension);
            }
        }

        point.set(px.calc(weight), py.calc(weight), pz.calc(weight));

        point
    }
}
