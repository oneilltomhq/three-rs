//! `smoothDamp()`, the critically damped spring `camera-controls` eases every
//! field with. Not three.js: yomotsu's `src/utils/math-utils.ts`.

use three_rs::math::math_utils::clamp;

/// `Math.max` — differs from Rust's `f64::max` in that NaN propagates.
fn js_max(a: f64, b: f64) -> f64 {
    if a.is_nan() || b.is_nan() {
        f64::NAN
    } else if a > b {
        a
    } else {
        b
    }
}

/// `smoothDamp()` from yomotsu's `camera-controls`
/// (`src/utils/math-utils.ts`), itself Unity's `Mathf.SmoothDamp` and Game
/// Programming Gems 4 chapter 1.10.
///
/// Not a `MathUtils.js` function — three.js has no critically damped spring —
/// but [`crate::MapControls`] damps every one of its fields with it,
/// and the point of the port is the *feel* of camera-controls, which is this
/// polynomial approximation of `exp( -omega * dt )` and nothing else.
///
/// `velocity` is the TypeScript's `currentVelocityRef`: the caller keeps one
/// per damped field and this reads and rewrites it. `max_speed` is
/// `f64::INFINITY` for an unclamped field.
pub fn smooth_damp(
    current: f64,
    target: f64,
    velocity: &mut f64,
    smooth_time: f64,
    max_speed: f64,
    dt: f64,
) -> f64 {
    // Based on Game Programming Gems 4 Chapter 1.10
    let smooth_time = js_max(0.0001, smooth_time);
    let omega = 2.0 / smooth_time;

    let x = omega * dt;
    let exp = 1.0 / (1.0 + x + 0.48 * x * x + 0.235 * x * x * x);
    let mut change = current - target;
    let original_to = target;

    // Clamp maximum speed
    // `Infinity * smoothTime` is `Infinity`, and `clamp` then leaves `change`
    // alone, exactly as in the TypeScript.
    let max_change = max_speed * smooth_time;
    change = clamp(change, -max_change, max_change);
    let target = current - change;

    let temp = (*velocity + omega * change) * dt;
    *velocity = (*velocity - omega * temp) * exp;
    let mut output = target + (change + temp) * exp;

    // Prevent overshooting
    if (original_to - current > 0.0) == (output > original_to) {
        output = original_to;
        *velocity = (output - original_to) / dt;
    }

    output
}

#[cfg(test)]
mod smooth_damp_tests {
    use super::smooth_damp;

    /// Three steps of `smoothDamp( current, 10, v, 0.25, Infinity, 1/60 )`
    /// from `current = 0`, `v = 0`, against the TypeScript's arithmetic done
    /// out by hand.
    ///
    /// The per-step constants do not change: `smoothTime = 0.25`, so
    /// `omega = 2 / 0.25 = 8`; `x = omega * deltaTime = 8 / 60 =
    /// 0.13333333333333333`; and
    ///
    /// ```text
    /// exp = 1 / ( 1 + x + 0.48 x^2 + 0.235 x^3 )
    ///     = 1 / ( 1 + 0.13333333333333333
    ///               + 0.48   * 0.017777777777777778
    ///               + 0.235  * 0.0023703703703703703 )
    ///     = 1 / 1.1424237037037038
    ///     = 0.8753319777574903
    /// ```
    ///
    /// Step 1 — `current = 0`, `v = 0`:
    ///
    /// ```text
    /// change  = 0 - 10 = -10          ( maxChange is Infinity, no clamp )
    /// target  = 0 - ( -10 ) = 10
    /// temp    = ( 0 + 8 * -10 ) / 60 = -1.3333333333333333
    /// v       = ( 0 - 8 * -1.3333333333333333 ) * exp
    ///         = 10.666666666666666 * 0.8753319777574903 = 9.336874429413228
    /// output  = 10 + ( -10 + -1.3333333333333333 ) * exp
    ///         = 10 - 9.920429081251557 = 0.07957091874844302
    /// ```
    ///
    /// `originalTo - current = 10 > 0` while `output > originalTo` is false,
    /// so the overshoot guard does not fire — nor does it on steps 2 and 3.
    ///
    /// Step 2 — `current = 0.07957091874844302`, `v = 9.336874429413228`:
    ///
    /// ```text
    /// change  = -9.920429081251557
    /// target  = 10
    /// temp    = ( 9.336874429413228 + 8 * -9.920429081251557 ) / 60
    ///         = -1.166908726791746
    /// v       = ( 9.336874429413228 + 8 * 1.166908726791746 ) * exp
    ///         = 18.67216757437520 * 0.8753319777574903 = 16.34572952074324
    /// output  = 10 + ( -9.920429081251557 + -1.166908726791746 ) * exp
    ///         = 10 - 9.705276902941300 = 0.29472309705870003
    /// ```
    ///
    /// Step 3 — `current = 0.29472309705870003`, `v = 16.34572952074324`:
    ///
    /// ```text
    /// change  = -9.705276902941300
    /// target  = 10
    /// temp    = ( 16.34572952074324 + 8 * -9.705276902941300 ) / 60
    ///         = -1.0664670672465507
    /// v       = ( 16.34572952074324 + 8 * 1.0664670672465507 ) * exp
    ///         = 24.877465058740  * 0.8753319777574903 = 21.461909623921763
    /// output  = 10 + ( -9.705276902941300 + -1.0664670672465507 ) * exp
    ///         = 10 - 9.389585460465773 = 0.6104145395342275
    /// ```
    #[test]
    fn three_steps_match_the_typescript() {
        let mut current = 0.0;
        let mut velocity = 0.0;
        let dt = 1.0 / 60.0;

        let expected = [
            (0.07957091874844302, 9.336874429413228),
            (0.29472309705870003, 16.34572952074324),
            (0.6104145395342275, 21.461909623921763),
        ];

        for (step, (want_current, want_velocity)) in expected.into_iter().enumerate() {
            current = smooth_damp(current, 10.0, &mut velocity, 0.25, f64::INFINITY, dt);
            assert!(
                (current - want_current).abs() < 1e-12,
                "step {}: current {current} != {want_current}",
                step + 1
            );
            assert!(
                (velocity - want_velocity).abs() < 1e-12,
                "step {}: velocity {velocity} != {want_velocity}",
                step + 1
            );
        }
    }

    /// The `maxSpeed` clamp: `change` cannot exceed `maxSpeed * smoothTime`,
    /// so one step from 0 toward 1000 at 10 units/s moves by no more than the
    /// spring would move over `maxChange = 10 * 0.25 = 2.5`.
    #[test]
    fn max_speed_clamps_the_change() {
        let mut velocity = 0.0;
        let current = smooth_damp(0.0, 1000.0, &mut velocity, 0.25, 10.0, 1.0 / 60.0);
        // `target` becomes `current - clamp( -1000, -2.5, 2.5 ) = 2.5`, and the
        // step is the same spring against that near target.
        let mut reference_velocity = 0.0;
        let reference = smooth_damp(0.0, 2.5, &mut reference_velocity, 0.25, 10.0, 1.0 / 60.0);
        assert!((current - reference).abs() < 1e-12);
    }

    /// The overshoot guard: a step long enough to pass the target lands *on*
    /// it, with the velocity that would have carried it there.
    #[test]
    fn the_overshoot_guard_pins_the_target() {
        let mut velocity = 1000.0;
        let current = smooth_damp(0.0, 1.0, &mut velocity, 0.25, f64::INFINITY, 1.0);
        assert_eq!(current, 1.0);
        assert_eq!(velocity, 0.0);
    }
}
