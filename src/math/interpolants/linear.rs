//! Port of `three.js/src/math/interpolants/LinearInterpolant.js`.

use crate::math::interpolant::{Interpolant, InterpolantData, Interpolation};

/// A basic linear interpolant.
#[derive(Clone, Copy, Debug, Default)]
pub struct LinearInterpolation;

/// `LinearInterpolant`.
pub type LinearInterpolant = Interpolant<LinearInterpolation>;

/// `new LinearInterpolant( parameterPositions, sampleValues, sampleSize, resultBuffer )`.
pub fn linear_interpolant(
    parameter_positions: Vec<f64>,
    sample_values: Vec<f64>,
    sample_size: usize,
    result_buffer: Option<Vec<f64>>,
) -> LinearInterpolant {
    Interpolant::new(
        parameter_positions,
        sample_values,
        sample_size,
        result_buffer,
        LinearInterpolation,
    )
}

impl Interpolation for LinearInterpolation {
    fn interpolate(&mut self, data: &mut InterpolantData, i1: usize, t0: f64, t: f64, t1: f64) {
        let stride = data.value_size;

        let offset1 = i1 * stride;
        let offset0 = offset1 - stride;

        let weight1 = (t - t0) / (t1 - t0);
        let weight0 = 1.0 - weight1;

        for i in 0..stride {
            data.result_buffer[i] =
                data.sample_values[offset0 + i] * weight0 + data.sample_values[offset1 + i] * weight1;
        }
    }
}
