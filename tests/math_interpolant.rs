//! Port of `three.js/test/unit/src/math/Interpolant.tests.js`.

use three_rs::math::interpolant::{Interpolant, InterpolantData, Interpolation};

// Since this is an abstract base class, we have to make it concrete in order
// to test its functionality...
//
// Three's `Mock` overrides `intervalChanged_` / `interpolate_` to push a record
// of every call onto `Mock.calls`; the Rust mock keeps that list on itself.

/// One entry of `Mock.calls`: `{ func, args }`.
#[derive(Clone, PartialEq, Debug)]
enum Call {
    /// `{ func: 'intervalChanged', args: [ i1, t0, t1 ] }`.
    IntervalChanged(usize, f64, f64),
    /// `{ func: 'interpolate', args: [ i1, t0, t, t1 ] }`.
    Interpolate(usize, f64, f64, f64),
}

#[derive(Default)]
struct Mock {
    /// `Mock.calls`; `None` is Three's `Mock.calls = null` (don't capture).
    calls: Option<Vec<Call>>,
}

impl Interpolation for Mock {
    fn interval_changed(&mut self, _data: &InterpolantData, i1: usize, t0: f64, t1: f64) {
        if let Some(calls) = self.calls.as_mut() {
            calls.push(Call::IntervalChanged(i1, t0, t1));
        }
    }

    fn interpolate(&mut self, data: &mut InterpolantData, i1: usize, t0: f64, t: f64, t1: f64) {
        if let Some(calls) = self.calls.as_mut() {
            calls.push(Call::Interpolate(i1, t0, t, t1));
        }

        data.copy_sample_value(i1 - 1);
    }
}

fn mock(parameter_positions: Vec<f64>, sample_values: Vec<f64>, sample_size: usize) -> Interpolant<Mock> {
    Interpolant::new(
        parameter_positions,
        sample_values,
        sample_size,
        Some(Vec::new()),
        Mock::default(),
    )
}

// INSTANCING
#[test]
fn instancing() {
    // `Mock extends from Interpolant` is a type-level fact here: `Mock` is an
    // `Interpolation` strategy inside an `Interpolant`, so it can only be
    // constructed as one.
    let interpolant = mock(Vec::new(), vec![1.0, 11.0, 2.0, 22.0, 3.0, 33.0], 2);
    assert_eq!(interpolant.data.value_size, 2);
}

// PRIVATE
#[test]
fn copy_sample_value() {
    let mut interpolant = mock(Vec::new(), vec![1.0, 11.0, 2.0, 22.0, 3.0, 33.0], 2);

    assert_eq!(interpolant.copy_sample_value(0), [1.0, 11.0], "sample fetch (0)");
    assert_eq!(interpolant.copy_sample_value(1), [2.0, 22.0], "sample fetch (1)");
    assert_eq!(interpolant.copy_sample_value(2), [3.0, 33.0], "first sample (2)");
}

#[test]
fn evaluate_interval_changed_interpolate() {
    let mut interpolant = mock(
        vec![11.0, 22.0, 33.0, 44.0, 55.0, 66.0, 77.0, 88.0, 99.0],
        Vec::new(),
        0,
    );

    /// `Mock.calls = []; interpolant.evaluate( t ); Mock.calls`.
    #[track_caller]
    fn calls(interpolant: &mut Interpolant<Mock>, t: f64) -> Vec<Call> {
        interpolant.interpolation.calls = Some(Vec::new());
        interpolant.evaluate(t);
        interpolant.interpolation.calls.take().unwrap()
    }

    assert_eq!(
        calls(&mut interpolant, 11.0),
        [
            Call::IntervalChanged(1, 11.0, 22.0),
            Call::Interpolate(1, 11.0, 11.0, 22.0),
        ],
        "no further calls"
    );

    // same interval
    assert_eq!(
        calls(&mut interpolant, 12.0),
        [Call::Interpolate(1, 11.0, 12.0, 22.0)],
        "no further calls"
    );

    // step forward
    assert_eq!(
        calls(&mut interpolant, 22.0),
        [
            Call::IntervalChanged(2, 22.0, 33.0),
            Call::Interpolate(2, 22.0, 22.0, 33.0),
        ]
    );

    // step back
    assert_eq!(
        calls(&mut interpolant, 21.0),
        [
            Call::IntervalChanged(1, 11.0, 22.0),
            Call::Interpolate(1, 11.0, 21.0, 22.0),
        ],
        "no further calls"
    );

    // same interval
    assert_eq!(
        calls(&mut interpolant, 20.0),
        [Call::Interpolate(1, 11.0, 20.0, 22.0)],
        "no further calls"
    );

    // two steps forward
    assert_eq!(
        calls(&mut interpolant, 43.0),
        [
            Call::IntervalChanged(3, 33.0, 44.0),
            Call::Interpolate(3, 33.0, 43.0, 44.0),
        ],
        "no further calls"
    );

    // two steps back
    assert_eq!(
        calls(&mut interpolant, 12.0),
        [
            Call::IntervalChanged(1, 11.0, 22.0),
            Call::Interpolate(1, 11.0, 12.0, 22.0),
        ],
        "no further calls"
    );

    // random access
    assert_eq!(
        calls(&mut interpolant, 77.0),
        [
            Call::IntervalChanged(7, 77.0, 88.0),
            Call::Interpolate(7, 77.0, 77.0, 88.0),
        ],
        "no further calls"
    );

    // same interval
    assert_eq!(
        calls(&mut interpolant, 80.0),
        [Call::Interpolate(7, 77.0, 80.0, 88.0)],
        "no further calls"
    );

    // random access
    assert_eq!(
        calls(&mut interpolant, 36.0),
        [
            Call::IntervalChanged(3, 33.0, 44.0),
            Call::Interpolate(3, 33.0, 36.0, 44.0),
        ],
        "no further calls"
    );

    // fast reset / loop (2nd)
    assert_eq!(
        calls(&mut interpolant, 24.0),
        [
            Call::IntervalChanged(2, 22.0, 33.0),
            Call::Interpolate(2, 22.0, 24.0, 33.0),
        ],
        "no further calls"
    );

    // fast reset / loop (2nd)
    assert_eq!(
        calls(&mut interpolant, 16.0),
        [
            Call::IntervalChanged(1, 11.0, 22.0),
            Call::Interpolate(1, 11.0, 16.0, 22.0),
        ],
        "no further calls"
    );
}
