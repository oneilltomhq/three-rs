//! Port of `three.js/src/math/interpolants/QuaternionLinearInterpolant.js`.

use crate::math::interpolant::{Interpolant, InterpolantData, Interpolation};
use crate::math::Quaternion;

/// Spherical linear unit quaternion interpolant.
#[derive(Clone, Copy, Debug, Default)]
pub struct QuaternionLinearInterpolation;

/// `QuaternionLinearInterpolant`.
pub type QuaternionLinearInterpolant = Interpolant<QuaternionLinearInterpolation>;

/// `new QuaternionLinearInterpolant( parameterPositions, sampleValues, sampleSize, resultBuffer )`.
pub fn quaternion_linear_interpolant(
    parameter_positions: Vec<f64>,
    sample_values: Vec<f64>,
    sample_size: usize,
    result_buffer: Option<Vec<f64>>,
) -> QuaternionLinearInterpolant {
    Interpolant::new(
        parameter_positions,
        sample_values,
        sample_size,
        result_buffer,
        QuaternionLinearInterpolation,
    )
}

impl Interpolation for QuaternionLinearInterpolation {
    fn interpolate(&mut self, data: &mut InterpolantData, i1: usize, t0: f64, t: f64, t1: f64) {
        let stride = data.value_size;

        let alpha = (t - t0) / (t1 - t0);

        let mut offset = i1 * stride;
        let end = offset + stride;

        while offset != end {
            // `slerpFlat( result, 0, values, offset - stride, values, offset, alpha )`
            // — one source array in Three, split here so the borrow checker can
            // see that the result buffer and the samples are distinct.
            Quaternion::slerp_flat(
                &mut data.result_buffer,
                0,
                &data.sample_values,
                offset - stride,
                &data.sample_values,
                offset,
                alpha,
            );
            offset += 4;
        }
    }
}
