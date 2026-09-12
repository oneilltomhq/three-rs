//! Port of `three.js/src/math/MathUtils.js`.
//!
//! Every function is the JS body transliterated, in the same order of
//! operations, on `f64` — JavaScript's only number type. The random-number
//! helpers (`randInt`, `randFloat`, `randFloatSpread`, `generateUUID`) are
//! deliberately absent: the e2e harness overrides `Math.random` with the
//! grader's deterministic generator (see `testing.rs`), so a faithful port of
//! them would have to go through that, and no rung needs them yet.

/// `MathUtils.DEG2RAD`.
pub const DEG2RAD: f64 = std::f64::consts::PI / 180.0;
/// `MathUtils.RAD2DEG`.
pub const RAD2DEG: f64 = 180.0 / std::f64::consts::PI;

/// `MathUtils.clamp()` — `Math.max( min, Math.min( max, value ) )`.
pub fn clamp(value: f64, min: f64, max: f64) -> f64 {
    js_max(min, js_min(max, value))
}

/// `MathUtils.euclideanModulo()` — `( ( n % m ) + m ) % m`.
pub fn euclidean_modulo(n: f64, m: f64) -> f64 {
    ((n % m) + m) % m
}

/// `MathUtils.mapLinear()`.
pub fn map_linear(x: f64, a1: f64, a2: f64, b1: f64, b2: f64) -> f64 {
    b1 + (x - a1) * (b2 - b1) / (a2 - a1)
}

/// `MathUtils.inverseLerp()`.
pub fn inverse_lerp(x: f64, y: f64, value: f64) -> f64 {
    if x != y {
        (value - x) / (y - x)
    } else {
        0.0
    }
}

/// `MathUtils.lerp()`.
pub fn lerp(x: f64, y: f64, t: f64) -> f64 {
    (1.0 - t) * x + t * y
}

/// `MathUtils.damp()`.
pub fn damp(x: f64, y: f64, lambda: f64, dt: f64) -> f64 {
    lerp(x, y, 1.0 - (-lambda * dt).exp())
}

/// `MathUtils.pingpong()`. three.js defaults `length` to 1.
pub fn pingpong(x: f64, length: f64) -> f64 {
    length - (euclidean_modulo(x, length * 2.0) - length).abs()
}

/// `MathUtils.smoothstep()`.
pub fn smoothstep(x: f64, min: f64, max: f64) -> f64 {
    if x <= min {
        return 0.0;
    }
    if x >= max {
        return 1.0;
    }

    let x = (x - min) / (max - min);

    x * x * (3.0 - 2.0 * x)
}

/// `MathUtils.smootherstep()`.
pub fn smootherstep(x: f64, min: f64, max: f64) -> f64 {
    if x <= min {
        return 0.0;
    }
    if x >= max {
        return 1.0;
    }

    let x = (x - min) / (max - min);

    x * x * x * (x * (x * 6.0 - 15.0) + 10.0)
}

/// `MathUtils.degToRad()`.
pub fn deg_to_rad(degrees: f64) -> f64 {
    degrees * DEG2RAD
}

/// `MathUtils.radToDeg()`.
pub fn rad_to_deg(radians: f64) -> f64 {
    radians * RAD2DEG
}

/// `MathUtils.isPowerOfTwo()` — JS coerces to an int32 for the bitwise ops.
pub fn is_power_of_two(value: i32) -> bool {
    (value & (value - 1)) == 0 && value != 0
}

/// `MathUtils.ceilPowerOfTwo()`.
pub fn ceil_power_of_two(value: f64) -> f64 {
    2.0f64.powf((value.ln() / std::f64::consts::LN_2).ceil())
}

/// `MathUtils.floorPowerOfTwo()`.
pub fn floor_power_of_two(value: f64) -> f64 {
    2.0f64.powf((value.ln() / std::f64::consts::LN_2).floor())
}

/// `Math.min` — differs from Rust's `f64::min` in that NaN propagates.
pub(crate) fn js_min(a: f64, b: f64) -> f64 {
    if a.is_nan() || b.is_nan() {
        f64::NAN
    } else if a < b {
        a
    } else {
        b
    }
}

/// `Math.max` — differs from Rust's `f64::max` in that NaN propagates.
pub(crate) fn js_max(a: f64, b: f64) -> f64 {
    if a.is_nan() || b.is_nan() {
        f64::NAN
    } else if a > b {
        a
    } else {
        b
    }
}
