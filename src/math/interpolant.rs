//! Port of `three.js/src/math/Interpolant.js`.
//!
//! Three's `Interpolant` is an abstract class: the base holds the sample data
//! and the cached interval index, does the interval seek in `evaluate()`, and
//! defers `interpolate_()` / `intervalChanged_()` / `copySampleValue_()` to the
//! subclass. The Rust shape splits that into data plus strategy:
//!
//! - [`InterpolantData`] is the base class' state (`parameterPositions`,
//!   `sampleValues`, `resultBuffer`, `valueSize`, `settings`).
//! - [`Interpolation`] is the subclass' three overridable methods, each taking
//!   the data as an argument instead of `this`.
//! - [`Interpolant`] is the pair, and owns `_cachedIndex` and `evaluate()`.

/// `constants.js` ending modes, the `endingStart` / `endingEnd` settings
/// `CubicInterpolant` reads.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Ending {
    /// `ZeroCurvatureEnding` (2400).
    ZeroCurvature,
    /// `ZeroSlopeEnding` (2401).
    ZeroSlope,
    /// `WrapAroundEnding` (2402).
    WrapAround,
}

/// The interpolation settings object: in Three this is a plain object with
/// `endingStart` / `endingEnd`, read through `getSettings_()`.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct InterpolantSettings {
    pub ending_start: Ending,
    pub ending_end: Ending,
}

impl Default for InterpolantSettings {
    fn default() -> Self {
        Self {
            ending_start: Ending::ZeroCurvature,
            ending_end: Ending::ZeroCurvature,
        }
    }
}

/// The state Three keeps on the `Interpolant` base class.
#[derive(Clone, Debug, Default)]
pub struct InterpolantData {
    /// `parameterPositions`.
    pub parameter_positions: Vec<f64>,
    /// `resultBuffer`.
    pub result_buffer: Vec<f64>,
    /// `sampleValues`.
    pub sample_values: Vec<f64>,
    /// `valueSize`.
    pub value_size: usize,
    /// `settings`, `null` until the owner (a `PropertyMixer`/`AnimationAction`)
    /// sets it; `getSettings_()` falls back to `DefaultSettings_`.
    pub settings: Option<InterpolantSettings>,
}

impl InterpolantData {
    /// `copySampleValue_( index )`. Kept on the data so that an
    /// [`Interpolation`] which overrides `copy_sample_value` (as
    /// `GLTFCubicSplineInterpolant` does) can still reach the default.
    pub fn copy_sample_value(&mut self, index: usize) -> &[f64] {
        // copies a sample value to the result buffer

        let stride = self.value_size;
        let offset = index * stride;

        for i in 0..stride {
            self.result_buffer[i] = self.sample_values[offset + i];
        }

        &self.result_buffer
    }

    /// `pp[ index ]`, where a read outside the array is JS `undefined`.
    fn position(&self, index: isize) -> Option<f64> {
        if index < 0 {
            return None;
        }
        self.parameter_positions.get(index as usize).copied()
    }
}

/// The three template methods of `Interpolant`'s subclasses.
pub trait Interpolation {
    /// `DefaultSettings_`.
    fn default_settings(&self) -> InterpolantSettings {
        InterpolantSettings::default()
    }

    /// `intervalChanged_( i1, t0, t1 )`. Empty on the base class.
    fn interval_changed(&mut self, _data: &InterpolantData, _i1: usize, _t0: f64, _t1: f64) {}

    /// `interpolate_( i1, t0, t, t1 )`.
    fn interpolate(&mut self, data: &mut InterpolantData, i1: usize, t0: f64, t: f64, t1: f64);

    /// `copySampleValue_( index )`, overridable: `GLTFLoader`'s cubic spline
    /// interpolant reads a different stride out of the sample buffer.
    fn copy_sample_value(&mut self, data: &mut InterpolantData, index: usize) {
        data.copy_sample_value(index);
    }
}

/// `Interpolant`: the sample data plus the interval seek, over an
/// [`Interpolation`] strategy.
#[derive(Clone, Debug)]
pub struct Interpolant<I> {
    /// The base class' state.
    pub data: InterpolantData,
    /// The subclass' template methods.
    pub interpolation: I,
    /// `_cachedIndex`.
    cached_index: usize,
}

impl<I: Interpolation> Interpolant<I> {
    /// `new Interpolant( parameterPositions, sampleValues, sampleSize, resultBuffer )`.
    ///
    /// `result_buffer` of `None` is JS `undefined`: a fresh buffer of
    /// `sample_size` elements.
    pub fn new(
        parameter_positions: Vec<f64>,
        sample_values: Vec<f64>,
        sample_size: usize,
        result_buffer: Option<Vec<f64>>,
        interpolation: I,
    ) -> Self {
        let mut result_buffer = result_buffer.unwrap_or_else(|| vec![0.0; sample_size]);

        // JS lets `new Mock( …, 2, [] )` grow the result buffer on first write;
        // a `Vec` has to be sized up front, so a short buffer is padded here.
        // Same end state, and `valueSize` elements is all anyone ever reads.
        if result_buffer.len() < sample_size {
            result_buffer.resize(sample_size, 0.0);
        }

        Self {
            data: InterpolantData {
                parameter_positions,
                result_buffer,
                sample_values,
                value_size: sample_size,
                settings: None,
            },
            interpolation,
            cached_index: 0,
        }
    }

    /// `getSettings_()`.
    pub fn settings(&self) -> InterpolantSettings {
        self.data
            .settings
            .unwrap_or_else(|| self.interpolation.default_settings())
    }

    /// The current `_cachedIndex`, exposed because `PropertyMixer` and the
    /// interpolant tests reason about it.
    pub fn cached_index(&self) -> usize {
        self.cached_index
    }

    /// `resultBuffer` after the last `evaluate()`.
    pub fn result_buffer(&self) -> &[f64] {
        &self.data.result_buffer
    }

    /// `copySampleValue_( index )`, through the strategy.
    pub fn copy_sample_value(&mut self, index: usize) -> &[f64] {
        self.interpolation.copy_sample_value(&mut self.data, index);
        &self.data.result_buffer
    }

    /// `evaluate( t )`.
    ///
    /// A transcription of Three's labelled-block control flow; Rust's labelled
    /// blocks carry it over unchanged. JS `undefined` positions become `None`,
    /// so `! ( t < t1 )` with `t1 === undefined` (always true in JS) becomes
    /// `!t1.is_some_and( |t1| t < t1 )`.
    pub fn evaluate(&mut self, t: f64) -> &[f64] {
        let mut i1 = self.cached_index as isize;
        let mut t1 = self.data.position(i1);
        let mut t0 = self.data.position(i1 - 1);

        'validate_interval: {
            'seek: {
                let mut right: isize;

                'linear_scan: {
                    //- See http://jsperf.com/comparison-to-undefined/3
                    //- slower code:
                    //-
                    //- 				if ( t >= t1 || t1 === undefined ) {
                    'forward_scan: {
                        if !t1.is_some_and(|t1| t < t1) {
                            let give_up_at = i1 + 2;
                            loop {
                                if t1.is_none() {
                                    if t0.is_some_and(|t0| t < t0) {
                                        break 'forward_scan;
                                    }

                                    // after end

                                    i1 = self.data.parameter_positions.len() as isize;
                                    self.cached_index = i1 as usize;
                                    self.interpolation
                                        .copy_sample_value(&mut self.data, (i1 - 1) as usize);
                                    return &self.data.result_buffer;
                                }

                                if i1 == give_up_at {
                                    break; // this loop
                                }

                                t0 = t1;
                                i1 += 1;
                                t1 = self.data.position(i1);

                                if t1.is_some_and(|t1| t < t1) {
                                    // we have arrived at the sought interval
                                    break 'seek;
                                }
                            }

                            // prepare binary search on the right side of the index
                            right = self.data.parameter_positions.len() as isize;
                            break 'linear_scan;
                        }
                    }

                    //- slower code:
                    //-					if ( t < t0 || t0 === undefined ) {
                    if !t0.is_some_and(|t0| t >= t0) {
                        // looping?

                        let t1global = self.data.position(1);

                        if t1global.is_some_and(|t1global| t < t1global) {
                            i1 = 2; // + 1, using the scan for the details
                            t0 = t1global;
                        }

                        // linear reverse scan

                        let give_up_at = i1 - 2;
                        loop {
                            if t0.is_none() {
                                // before start

                                self.cached_index = 0;
                                self.interpolation.copy_sample_value(&mut self.data, 0);
                                return &self.data.result_buffer;
                            }

                            if i1 == give_up_at {
                                break; // this loop
                            }

                            t1 = t0;
                            i1 -= 1;
                            t0 = self.data.position(i1 - 1);

                            if t0.is_some_and(|t0| t >= t0) {
                                // we have arrived at the sought interval
                                break 'seek;
                            }
                        }

                        // prepare binary search on the left side of the index
                        right = i1;
                        i1 = 0;
                        break 'linear_scan;
                    }

                    // the interval is valid

                    break 'validate_interval;
                } // linear scan

                // binary search

                while i1 < right {
                    let mid = ((i1 + right) as usize) >> 1;

                    if t < self.data.parameter_positions[mid] {
                        right = mid as isize;
                    } else {
                        i1 = mid as isize + 1;
                    }
                }

                t1 = self.data.position(i1);
                t0 = self.data.position(i1 - 1);

                // check boundary cases, again

                if t0.is_none() {
                    self.cached_index = 0;
                    self.interpolation.copy_sample_value(&mut self.data, 0);
                    return &self.data.result_buffer;
                }

                if t1.is_none() {
                    i1 = self.data.parameter_positions.len() as isize;
                    self.cached_index = i1 as usize;
                    self.interpolation
                        .copy_sample_value(&mut self.data, (i1 - 1) as usize);
                    return &self.data.result_buffer;
                }
            } // seek

            self.cached_index = i1 as usize;

            let (t0, t1) = (
                t0.expect("three-rs: the interval search leaves t0 set"),
                t1.expect("three-rs: the interval search leaves t1 set"),
            );
            self.interpolation
                .interval_changed(&self.data, i1 as usize, t0, t1);
        } // validate_interval

        let (t0, t1) = (
            t0.expect("three-rs: the interval search leaves t0 set"),
            t1.expect("three-rs: the interval search leaves t1 set"),
        );
        self.interpolation
            .interpolate(&mut self.data, i1 as usize, t0, t, t1);

        &self.data.result_buffer
    }
}
