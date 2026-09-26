//! Port of `three.js/src/extras/core/Interpolations.js`: Bézier curves and
//! the uniform Catmull-Rom spline, one scalar component at a time.
//!
//! Each function keeps three.js' expression shape exactly — `QuadraticBezier`
//! is the sum of three separately rounded terms, not a Horner form — because
//! that is what makes the rounding match.

/// `CatmullRom( t, p0, p1, p2, p3 )`, the uniform spline `SplineCurve` uses.
pub fn catmull_rom(t: f64, p0: f64, p1: f64, p2: f64, p3: f64) -> f64 {
    let v0 = (p2 - p0) * 0.5;
    let v1 = (p3 - p1) * 0.5;
    let t2 = t * t;
    let t3 = t * t2;
    (2.0 * p1 - 2.0 * p2 + v0 + v1) * t3 + (-3.0 * p1 + 3.0 * p2 - 2.0 * v0 - v1) * t2 + v0 * t + p1
}

fn quadratic_bezier_p0(t: f64, p: f64) -> f64 {
    let k = 1.0 - t;
    k * k * p
}

fn quadratic_bezier_p1(t: f64, p: f64) -> f64 {
    2.0 * (1.0 - t) * t * p
}

fn quadratic_bezier_p2(t: f64, p: f64) -> f64 {
    t * t * p
}

/// `QuadraticBezier( t, p0, p1, p2 )`.
pub fn quadratic_bezier(t: f64, p0: f64, p1: f64, p2: f64) -> f64 {
    quadratic_bezier_p0(t, p0) + quadratic_bezier_p1(t, p1) + quadratic_bezier_p2(t, p2)
}

fn cubic_bezier_p0(t: f64, p: f64) -> f64 {
    let k = 1.0 - t;
    k * k * k * p
}

fn cubic_bezier_p1(t: f64, p: f64) -> f64 {
    let k = 1.0 - t;
    3.0 * k * k * t * p
}

fn cubic_bezier_p2(t: f64, p: f64) -> f64 {
    3.0 * (1.0 - t) * t * t * p
}

fn cubic_bezier_p3(t: f64, p: f64) -> f64 {
    t * t * t * p
}

/// `CubicBezier( t, p0, p1, p2, p3 )`.
pub fn cubic_bezier(t: f64, p0: f64, p1: f64, p2: f64, p3: f64) -> f64 {
    cubic_bezier_p0(t, p0)
        + cubic_bezier_p1(t, p1)
        + cubic_bezier_p2(t, p2)
        + cubic_bezier_p3(t, p3)
}
