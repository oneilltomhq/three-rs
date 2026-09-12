//! Port of `three.js/src/math/interpolants/DiscreteInterpolant.js`.

use crate::math::interpolant::{Interpolant, InterpolantData, Interpolation};

/// Interpolant that evaluates to the sample value at the position preceding the
/// parameter.
#[derive(Clone, Copy, Debug, Default)]
pub struct DiscreteInterpolation;

/// `DiscreteInterpolant`.
pub type DiscreteInterpolant = Interpolant<DiscreteInterpolation>;

/// `new DiscreteInterpolant( parameterPositions, sampleValues, sampleSize, resultBuffer )`.
pub fn discrete_interpolant(
    parameter_positions: Vec<f64>,
    sample_values: Vec<f64>,
    sample_size: usize,
    result_buffer: Option<Vec<f64>>,
) -> DiscreteInterpolant {
    Interpolant::new(
        parameter_positions,
        sample_values,
        sample_size,
        result_buffer,
        DiscreteInterpolation,
    )
}

impl Interpolation for DiscreteInterpolation {
    fn interpolate(&mut self, data: &mut InterpolantData, i1: usize, _t0: f64, _t: f64, _t1: f64) {
        data.copy_sample_value(i1 - 1);
    }
}
