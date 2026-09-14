//! Port of `three.js/test/unit/src/math/interpolants/*.tests.js`:
//! `LinearInterpolant`, `DiscreteInterpolant`, `CubicInterpolant`,
//! `QuaternionLinearInterpolant` and `CustomInterpolant`.
//!
//! Three's `Extending` tests assert `object instanceof Interpolant`, which is a
//! type-level fact in Rust (each of these is an `Interpolant<…>`), so they fold
//! into the `instancing` tests.

use three_rs::math::interpolant::{Interpolant, InterpolantData, Interpolation};
use three_rs::math::{
    cubic_interpolant, discrete_interpolant, linear_interpolant, quaternion_linear_interpolant,
};

// LinearInterpolant

#[test]
fn linear_instancing() {
    // parameterPositions, sampleValues, sampleSize, resultBuffer
    let object = linear_interpolant(
        Vec::new(),
        vec![1.0, 11.0, 2.0, 22.0, 3.0, 33.0],
        2,
        Some(Vec::new()),
    );
    assert_eq!(
        object.data.value_size, 2,
        "Can instantiate a LinearInterpolant."
    );
}

#[test]
fn linear_evaluate() {
    // Not in Three's (empty) `PRIVATE - TEMPLATE METHODS` section; the straight
    // read of `interpolate_`: halfway between the samples of each interval.
    let mut object = linear_interpolant(
        vec![1.0, 2.0, 3.0],
        vec![1.0, 11.0, 2.0, 22.0, 3.0, 33.0],
        2,
        None,
    );

    assert_eq!(object.evaluate(1.0), [1.0, 11.0]);
    assert_eq!(object.evaluate(1.5), [1.5, 16.5]);
    assert_eq!(object.evaluate(2.5), [2.5, 27.5]);
    // clamped at both ends
    assert_eq!(object.evaluate(0.0), [1.0, 11.0]);
    assert_eq!(object.evaluate(4.0), [3.0, 33.0]);
}

// DiscreteInterpolant

#[test]
fn discrete_instancing() {
    // parameterPositions, sampleValues, sampleSize, resultBuffer
    let object = discrete_interpolant(
        Vec::new(),
        vec![1.0, 11.0, 2.0, 22.0, 3.0, 33.0],
        2,
        Some(Vec::new()),
    );
    assert_eq!(
        object.data.value_size, 2,
        "Can instantiate a DiscreteInterpolant."
    );
}

#[test]
fn discrete_evaluate() {
    // the sample value at the position preceding the parameter
    let mut object = discrete_interpolant(
        vec![1.0, 2.0, 3.0],
        vec![1.0, 11.0, 2.0, 22.0, 3.0, 33.0],
        2,
        None,
    );

    assert_eq!(object.evaluate(1.5), [1.0, 11.0]);
    assert_eq!(object.evaluate(1.999), [1.0, 11.0]);
    assert_eq!(object.evaluate(2.5), [2.0, 22.0]);
    assert_eq!(object.evaluate(0.0), [1.0, 11.0]);
    assert_eq!(object.evaluate(4.0), [3.0, 33.0]);
}

// CubicInterpolant

#[test]
fn cubic_instancing() {
    // parameterPositions, sampleValues, sampleSize, resultBuffer
    let object = cubic_interpolant(
        Vec::new(),
        vec![1.0, 11.0, 2.0, 22.0, 3.0, 33.0],
        2,
        Some(Vec::new()),
    );
    assert_eq!(
        object.data.value_size, 2,
        "Can instantiate a CubicInterpolant."
    );
}

#[test]
fn cubic_evaluate() {
    // A linear ramp is reproduced exactly by the Hermite construction, whatever
    // the ending mode, so this pins `intervalChanged_` + `interpolate_` without
    // restating the polynomials.
    let mut object = cubic_interpolant(
        vec![0.0, 1.0, 2.0, 3.0],
        vec![0.0, 0.0, 1.0, 10.0, 2.0, 20.0, 3.0, 30.0],
        2,
        None,
    );

    for (t, expected) in [
        (0.0, [0.0, 0.0]),
        (0.5, [0.5, 5.0]),
        (1.0, [1.0, 10.0]),
        (1.25, [1.25, 12.5]),
        (2.5, [2.5, 25.0]),
        (3.0, [3.0, 30.0]),
    ] {
        let actual = object.evaluate(t);
        assert!(
            (actual[0] - expected[0]).abs() < 1e-12 && (actual[1] - expected[1]).abs() < 1e-12,
            "evaluate({t}) = {actual:?}, expected {expected:?}"
        );
    }
}

// QuaternionLinearInterpolant

#[test]
fn quaternion_linear_instancing() {
    // parameterPositions, sampleValues, sampleSize, resultBuffer
    let object = quaternion_linear_interpolant(
        Vec::new(),
        vec![1.0, 11.0, 2.0, 22.0, 3.0, 33.0],
        2,
        Some(Vec::new()),
    );
    assert_eq!(
        object.data.value_size, 2,
        "Can instantiate a QuaternionLinearInterpolant."
    );
}

#[test]
fn quaternion_linear_evaluate() {
    // slerp from identity to a 90° rotation about z; halfway is 45°.
    let h = std::f64::consts::FRAC_1_SQRT_2;
    let mut object = quaternion_linear_interpolant(
        vec![0.0, 1.0],
        vec![0.0, 0.0, 0.0, 1.0, 0.0, 0.0, h, h],
        4,
        None,
    );

    let eighth = std::f64::consts::FRAC_PI_8;
    let expected = [0.0, 0.0, eighth.sin(), eighth.cos()];
    let actual = object.evaluate(0.5).to_vec();
    for i in 0..4 {
        assert!(
            (actual[i] - expected[i]).abs() < 1e-12,
            "evaluate(0.5)[{i}] = {}, expected {}",
            actual[i],
            expected[i]
        );
    }

    assert_eq!(object.evaluate(0.0), [0.0, 0.0, 0.0, 1.0]);
    assert_eq!(object.evaluate(1.0), [0.0, 0.0, h, h]);
}

// CustomInterpolant — a custom cubic spline interpolant mimicking
// `GLTFCubicSplineInterpolant` from `GLTFLoader`. The keyframe layout for
// CUBICSPLINE animations is:
// [ inTangent_1, splineVertex_1, outTangent_1, inTangent_2, splineVertex_2, ... ]

#[derive(Default)]
struct CubicSplineInterpolation;

impl Interpolation for CubicSplineInterpolation {
    fn copy_sample_value(&mut self, data: &mut InterpolantData, index: usize) {
        let value_size = data.value_size;
        let offset = index * value_size * 3 + value_size;

        for i in 0..value_size {
            data.result_buffer[i] = data.sample_values[offset + i];
        }
    }

    fn interpolate(&mut self, data: &mut InterpolantData, i1: usize, t0: f64, t: f64, t1: f64) {
        let stride = data.value_size;

        let stride2 = stride * 2;
        let stride3 = stride * 3;

        let td = t1 - t0;

        let p = (t - t0) / td;
        let pp = p * p;
        let ppp = pp * p;

        let offset1 = i1 * stride3;
        let offset0 = offset1 - stride3;

        let s2 = -2.0 * ppp + 3.0 * pp;
        let s3 = ppp - pp;
        let s0 = 1.0 - s2;
        let s1 = s3 - pp + p;

        for i in 0..stride {
            let p0 = data.sample_values[offset0 + i + stride];
            let m0 = data.sample_values[offset0 + i + stride2] * td;
            let p1 = data.sample_values[offset1 + i + stride];
            let m1 = data.sample_values[offset1 + i] * td;

            data.result_buffer[i] = s0 * p0 + s1 * m0 + s2 * p1 + s3 * m1;
        }
    }
}

#[test]
fn custom_instancing() {
    // parameterPositions, sampleValues, sampleSize, resultBuffer
    let object = Interpolant::new(
        vec![0.0, 1.0],
        vec![0.0; 6],
        1,
        Some(Vec::new()),
        CubicSplineInterpolation,
    );
    assert_eq!(
        object.data.value_size, 1,
        "CubicSplineInterpolant extends from Interpolant"
    );
}

#[test]
fn custom_evaluate() {
    // Two keyframes at t = 0 and t = 1, valueSize = 1.
    // Layout: [ in_0, v_0, out_0, in_1, v_1, out_1 ]
    // Vertex values 0 -> 1 with non-zero tangents to exercise all spline terms.
    let positions = vec![0.0, 1.0];
    let values = vec![0.0, 0.0, 1.0, -1.0, 1.0, 0.0];
    let mut interpolant = Interpolant::new(
        positions,
        values,
        1,
        Some(vec![0.0]),
        CubicSplineInterpolation,
    );

    assert_eq!(
        interpolant.evaluate(0.0),
        [0.0],
        "evaluate at first keyframe"
    );
    assert_eq!(
        interpolant.evaluate(1.0),
        [1.0],
        "evaluate at last keyframe"
    );

    // At t = 0.5 with td = 1, p = 0.5 → s0 = 0.5, s1 = 0.125, s2 = 0.5, s3 = -0.125
    // result = 0.5 * 0 + 0.125 * 1 + 0.5 * 1 + ( -0.125 ) * ( -1 ) = 0.75
    assert_eq!(
        interpolant.evaluate(0.5),
        [0.75],
        "evaluate inside interval"
    );

    // Out-of-range queries clamp to the boundary spline vertex.
    assert_eq!(
        interpolant.evaluate(-1.0),
        [0.0],
        "evaluate before first keyframe"
    );
    assert_eq!(
        interpolant.evaluate(2.0),
        [1.0],
        "evaluate after last keyframe"
    );
}
