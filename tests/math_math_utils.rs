//! Port of `three.js/test/unit/src/math/MathUtils.tests.js`.
//!
//! Skipped: `generateUUID`, `randInt`, `randFloat`, `randFloatSpread` — the
//! crate deliberately omits the random helpers (the e2e harness drives its own
//! deterministic generator, and no rung needs them yet).

use three_rs::math::math_utils as m;

#[test]
fn clamp() {
    assert_eq!(m::clamp(0.5, 0.0, 1.0), 0.5, "Value already within limits");
    assert_eq!(m::clamp(0.0, 0.0, 1.0), 0.0, "Value equal to one limit");
    assert_eq!(m::clamp(-0.1, 0.0, 1.0), 0.0, "Value too low");
    assert_eq!(m::clamp(1.1, 0.0, 1.0), 1.0, "Value too high");
}

#[test]
fn euclidean_modulo() {
    assert!(
        m::euclidean_modulo(6.0, 0.0).is_nan(),
        "Division by zero returns NaN"
    );
    assert_eq!(m::euclidean_modulo(6.0, 1.0), 0.0);
    assert_eq!(m::euclidean_modulo(6.0, 2.0), 0.0);
    assert_eq!(m::euclidean_modulo(6.0, 5.0), 1.0);
    assert_eq!(m::euclidean_modulo(6.0, 6.0), 0.0);
    assert_eq!(m::euclidean_modulo(6.0, 7.0), 6.0);
}

#[test]
fn map_linear() {
    assert_eq!(m::map_linear(0.5, 0.0, 1.0, 0.0, 10.0), 5.0);
    assert_eq!(m::map_linear(0.0, 0.0, 1.0, 0.0, 10.0), 0.0);
    assert_eq!(m::map_linear(1.0, 0.0, 1.0, 0.0, 10.0), 10.0);
}

#[test]
fn inverse_lerp() {
    assert_eq!(m::inverse_lerp(1.0, 2.0, 1.5), 0.5);
    assert_eq!(m::inverse_lerp(1.0, 2.0, 2.0), 1.0);
    assert_eq!(m::inverse_lerp(1.0, 2.0, 1.0), 0.0);
    assert_eq!(
        m::inverse_lerp(1.0, 1.0, 1.0),
        0.0,
        "0% Percentage, no NaN value"
    );
}

#[test]
fn lerp() {
    assert_eq!(m::lerp(1.0, 2.0, 0.0), 1.0);
    assert_eq!(m::lerp(1.0, 2.0, 1.0), 2.0);
    assert_eq!(m::lerp(1.0, 2.0, 0.4), 1.4);
}

#[test]
fn damp() {
    assert_eq!(m::damp(1.0, 2.0, 0.0, 0.016), 1.0);
    assert_eq!(m::damp(1.0, 2.0, 10.0, 0.016), 1.1478562110337887);
}

#[test]
fn pingpong() {
    assert_eq!(m::pingpong(2.5, 1.0), 0.5, "Value at 2.5 is 0.5");
    assert_eq!(
        m::pingpong(2.5, 2.0),
        1.5,
        "Value at 2.5 with length of 2 is 1.5"
    );
    assert_eq!(m::pingpong(-1.5, 1.0), 0.5, "Value at -1.5 is 0.5");
}

#[test]
fn smoothstep() {
    assert_eq!(m::smoothstep(-1.0, 0.0, 2.0), 0.0);
    assert_eq!(m::smoothstep(0.0, 0.0, 2.0), 0.0);
    assert_eq!(m::smoothstep(0.5, 0.0, 2.0), 0.15625);
    assert_eq!(m::smoothstep(1.0, 0.0, 2.0), 0.5);
    assert_eq!(m::smoothstep(1.5, 0.0, 2.0), 0.84375);
    assert_eq!(m::smoothstep(2.0, 0.0, 2.0), 1.0);
    assert_eq!(m::smoothstep(3.0, 0.0, 2.0), 1.0);
}

#[test]
fn smootherstep() {
    assert_eq!(m::smootherstep(-1.0, 0.0, 2.0), 0.0);
    assert_eq!(m::smootherstep(0.0, 0.0, 2.0), 0.0);
    assert_eq!(m::smootherstep(0.5, 0.0, 2.0), 0.103515625);
    assert_eq!(m::smootherstep(1.0, 0.0, 2.0), 0.5);
    assert_eq!(m::smootherstep(1.5, 0.0, 2.0), 0.896484375);
    assert_eq!(m::smootherstep(2.0, 0.0, 2.0), 1.0);
    assert_eq!(m::smootherstep(3.0, 0.0, 2.0), 1.0);
}

#[test]
fn deg_to_rad() {
    assert_eq!(m::deg_to_rad(0.0), 0.0);
    assert_eq!(m::deg_to_rad(90.0), std::f64::consts::PI / 2.0);
    assert_eq!(m::deg_to_rad(180.0), std::f64::consts::PI);
    assert_eq!(m::deg_to_rad(360.0), std::f64::consts::PI * 2.0);
}

#[test]
fn rad_to_deg() {
    assert_eq!(m::rad_to_deg(0.0), 0.0);
    assert_eq!(m::rad_to_deg(std::f64::consts::PI / 2.0), 90.0);
    assert_eq!(m::rad_to_deg(std::f64::consts::PI), 180.0);
    assert_eq!(m::rad_to_deg(std::f64::consts::PI * 2.0), 360.0);
}

#[test]
fn is_power_of_two() {
    assert!(!m::is_power_of_two(0));
    assert!(m::is_power_of_two(1));
    assert!(m::is_power_of_two(2));
    assert!(!m::is_power_of_two(3));
    assert!(m::is_power_of_two(4));
}

#[test]
fn ceil_power_of_two() {
    assert_eq!(m::ceil_power_of_two(1.0), 1.0);
    assert_eq!(m::ceil_power_of_two(3.0), 4.0);
    assert_eq!(m::ceil_power_of_two(4.0), 4.0);
}

#[test]
fn floor_power_of_two() {
    assert_eq!(m::floor_power_of_two(1.0), 1.0);
    assert_eq!(m::floor_power_of_two(3.0), 2.0);
    assert_eq!(m::floor_power_of_two(4.0), 4.0);
}
