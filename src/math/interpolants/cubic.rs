//! Port of `three.js/src/math/interpolants/CubicInterpolant.js`.

use crate::math::interpolant::{
    Ending, Interpolant, InterpolantData, InterpolantSettings, Interpolation,
};

/// Fast and simple cubic spline interpolant.
///
/// It was derived from a Hermitian construction setting the first derivative at
/// each sample position to the linear slope between neighboring positions over
/// their parameter interval.
#[derive(Clone, Copy, Debug)]
pub struct CubicInterpolation {
    weight_prev: f64,
    offset_prev: usize,
    weight_next: f64,
    offset_next: usize,
    /// `DefaultSettings_`.
    pub default_settings: InterpolantSettings,
}

impl Default for CubicInterpolation {
    fn default() -> Self {
        Self {
            weight_prev: -0.0,
            offset_prev: 0,
            weight_next: -0.0,
            offset_next: 0,
            default_settings: InterpolantSettings {
                ending_start: Ending::ZeroCurvature,
                ending_end: Ending::ZeroCurvature,
            },
        }
    }
}

/// `CubicInterpolant`.
pub type CubicInterpolant = Interpolant<CubicInterpolation>;

/// `new CubicInterpolant( parameterPositions, sampleValues, sampleSize, resultBuffer )`.
pub fn cubic_interpolant(
    parameter_positions: Vec<f64>,
    sample_values: Vec<f64>,
    sample_size: usize,
    result_buffer: Option<Vec<f64>>,
) -> CubicInterpolant {
    Interpolant::new(
        parameter_positions,
        sample_values,
        sample_size,
        result_buffer,
        CubicInterpolation::default(),
    )
}

impl Interpolation for CubicInterpolation {
    fn default_settings(&self) -> InterpolantSettings {
        self.default_settings
    }

    fn interval_changed(&mut self, data: &InterpolantData, i1: usize, t0: f64, t1: f64) {
        let pp = &data.parameter_positions;
        let i1 = i1 as isize;
        let mut i_prev = i1 - 2;
        let mut i_next = i1 + 1;

        let at = |i: isize| -> Option<f64> {
            if i < 0 {
                None
            } else {
                pp.get(i as usize).copied()
            }
        };

        let mut t_prev = at(i_prev);
        let mut t_next = at(i_next);

        let settings = data.settings.unwrap_or(self.default_settings);

        if t_prev.is_none() {
            match settings.ending_start {
                Ending::ZeroSlope => {
                    // f'(t0) = 0
                    i_prev = i1;
                    t_prev = Some(2.0 * t0 - t1);
                }

                Ending::WrapAround => {
                    // use the other end of the curve
                    i_prev = pp.len() as isize - 2;
                    t_prev = Some(t0 + pp[i_prev as usize] - pp[i_prev as usize + 1]);
                }

                Ending::ZeroCurvature => {
                    // f''(t0) = 0 a.k.a. Natural Spline
                    i_prev = i1;
                    t_prev = Some(t1);
                }
            }
        }

        if t_next.is_none() {
            match settings.ending_end {
                Ending::ZeroSlope => {
                    // f'(tN) = 0
                    i_next = i1;
                    t_next = Some(2.0 * t1 - t0);
                }

                Ending::WrapAround => {
                    // use the other end of the curve
                    i_next = 1;
                    t_next = Some(t1 + pp[1] - pp[0]);
                }

                Ending::ZeroCurvature => {
                    // f''(tN) = 0, a.k.a. Natural Spline
                    i_next = i1 - 1;
                    t_next = Some(t0);
                }
            }
        }

        let half_dt = (t1 - t0) * 0.5;
        let stride = data.value_size;

        self.weight_prev = half_dt / (t0 - t_prev.unwrap());
        self.weight_next = half_dt / (t_next.unwrap() - t1);
        self.offset_prev = i_prev as usize * stride;
        self.offset_next = i_next as usize * stride;
    }

    fn interpolate(&mut self, data: &mut InterpolantData, i1: usize, t0: f64, t: f64, t1: f64) {
        let stride = data.value_size;

        let o1 = i1 * stride;
        let o0 = o1 - stride;
        let o_p = self.offset_prev;
        let o_n = self.offset_next;
        let w_p = self.weight_prev;
        let w_n = self.weight_next;

        let p = (t - t0) / (t1 - t0);
        let pp = p * p;
        let ppp = pp * p;

        // evaluate polynomials

        let s_p = -w_p * ppp + 2.0 * w_p * pp - w_p * p;
        let s0 = (1.0 + w_p) * ppp + (-1.5 - 2.0 * w_p) * pp + (-0.5 + w_p) * p + 1.0;
        let s1 = (-1.0 - w_n) * ppp + (1.5 + w_n) * pp + 0.5 * p;
        let s_n = w_n * ppp - w_n * pp;

        // combine data linearly

        for i in 0..stride {
            data.result_buffer[i] = s_p * data.sample_values[o_p + i]
                + s0 * data.sample_values[o0 + i]
                + s1 * data.sample_values[o1 + i]
                + s_n * data.sample_values[o_n + i];
        }
    }
}
