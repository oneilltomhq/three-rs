//! Tests for the port of `three.js/src/animation/PropertyMixer.js`.
//!
//! Three has no `PropertyMixer.tests.js`, so these are not a port of an
//! upstream test file: each expectation below is worked out by hand from the JS
//! (`accumulate`, `apply`, `_lerp`, `_slerp`, `_select`, `_lerpAdditive` and the
//! `_setAdditiveIdentity*` trio), with [`BufferTarget`] standing in for the
//! `PropertyBinding` that `PropertyMixer` would otherwise drive.
//!
//! The interpolators write their result into the mixer's `incoming` region,
//! which is `buffer[ 0 .. valueSize ]`; these tests write it there directly
//! rather than running a `KeyframeTrack` first.

mod support;

use support::{close, close_all, EPS};
use three_rs::animation::{BufferTarget, PropertyMixer, TrackValueType};

/// Write an interpolator result into the `incoming` region.
fn set_incoming(mixer: &mut PropertyMixer, incoming: &[f64]) {
    mixer.buffer[..incoming.len()].copy_from_slice(incoming);
}

/// Read the bound property back through the `BindingTarget` seam.
fn read_target(mixer: &PropertyMixer) -> Vec<f64> {
    let mut out = vec![0.0; mixer.value_size];
    mixer.binding.get_value(&mut out, 0);
    out
}

fn vector_mixer(original: &[f64]) -> PropertyMixer {
    let target = BufferTarget::new(original.to_vec());
    let mut mixer = PropertyMixer::new(Box::new(target), TrackValueType::Vector, original.len());
    mixer.save_original_state();
    mixer
}

#[test]
fn instancing() {
    let mixer = PropertyMixer::new(
        Box::new(BufferTarget::new(vec![0.0; 3])),
        TrackValueType::Vector,
        3,
    );

    assert_eq!(mixer.value_size, 3);
    // `buffer = new Float64Array( valueSize * 5 )` for non-quaternion types.
    assert_eq!(mixer.buffer.len(), 15);
    assert_eq!(mixer.cumulative_weight, 0.0);
    assert_eq!(mixer.cumulative_weight_additive, 0.0);
    assert_eq!(mixer.use_count, 0);
    assert_eq!(mixer.reference_count, 0);

    // `'quaternion'` gets the extra `work` region: `valueSize * 6`.
    let quat = PropertyMixer::new(
        Box::new(BufferTarget::new(vec![0.0, 0.0, 0.0, 1.0])),
        TrackValueType::Quaternion,
        4,
    );
    assert_eq!(quat.buffer.len(), 24);
}

#[test]
fn save_original_state_seeds_both_accus() {
    let mixer = vector_mixer(&[2.0, 4.0, 6.0]);

    // `accu[0..1] := orig`, and the numeric additive identity is zero.
    close_all(&mixer.buffer[3..6], &[2.0, 4.0, 6.0], EPS, "accu0");
    close_all(&mixer.buffer[6..9], &[2.0, 4.0, 6.0], EPS, "accu1");
    close_all(&mixer.buffer[9..12], &[2.0, 4.0, 6.0], EPS, "orig");
    close_all(&mixer.buffer[12..15], &[0.0, 0.0, 0.0], EPS, "add");
}

#[test]
fn accumulate_and_apply_at_full_weight() {
    // cumulativeWeight === 0 -> `accuN := incoming` verbatim (no scaling), and
    // weight === 1 means `apply` never mixes the original back in.
    let mut mixer = vector_mixer(&[0.0, 0.0, 0.0]);

    set_incoming(&mut mixer, &[1.0, 2.0, 3.0]);
    mixer.accumulate(0, 1.0);
    close(mixer.cumulative_weight, 1.0, EPS, "cumulativeWeight");
    close_all(&mixer.buffer[3..6], &[1.0, 2.0, 3.0], EPS, "accu0");

    mixer.apply(0);

    close_all(&read_target(&mixer), &[1.0, 2.0, 3.0], EPS, "value");
    // `apply` resets the weights for the next frame.
    close(mixer.cumulative_weight, 0.0, EPS, "cumulativeWeight");
}

#[test]
fn apply_at_half_weight_lerps_against_original_state() {
    // accu0 := incoming; then apply's `_lerp( accu0, orig, 1 - 0.5 )`:
    //   10 * 0.5 + 2 * 0.5 = 6, 20 * 0.5 + 4 * 0.5 = 12, 30 * .5 + 6 * .5 = 18
    let mut mixer = vector_mixer(&[2.0, 4.0, 6.0]);

    set_incoming(&mut mixer, &[10.0, 20.0, 30.0]);
    mixer.accumulate(0, 0.5);
    mixer.apply(0);

    close_all(&read_target(&mixer), &[6.0, 12.0, 18.0], EPS, "value");
}

#[test]
fn two_accumulations_blend_as_lerp() {
    // First accumulate (weight 0.25) takes the `currentWeight === 0` branch, so
    // accu0 := [ 4, 8, 12 ] and cumulativeWeight = 0.25. The second
    // (weight 0.75) raises it to 1.0 with mix = 0.75 / 1.0, so
    //   _lerp: accu0 = accu0 * 0.25 + [ 8, 4, 0 ] * 0.75 = [ 7, 5, 3 ].
    // cumulativeWeight is then 1, so `apply` does not mix the original in.
    let mut mixer = vector_mixer(&[0.0, 0.0, 0.0]);

    set_incoming(&mut mixer, &[4.0, 8.0, 12.0]);
    mixer.accumulate(0, 0.25);

    set_incoming(&mut mixer, &[8.0, 4.0, 0.0]);
    mixer.accumulate(0, 0.75);

    close(mixer.cumulative_weight, 1.0, EPS, "cumulativeWeight");
    close_all(&mixer.buffer[3..6], &[7.0, 5.0, 3.0], EPS, "accu0");

    mixer.apply(0);
    close_all(&read_target(&mixer), &[7.0, 5.0, 3.0], EPS, "value");
}

#[test]
fn accumulate_into_accu1() {
    // accuIndex 1 -> offset = 1 * stride + stride = the third region.
    let mut mixer = vector_mixer(&[0.0, 0.0, 0.0]);

    set_incoming(&mut mixer, &[5.0, 6.0, 7.0]);
    mixer.accumulate(1, 1.0);

    close_all(&mixer.buffer[6..9], &[5.0, 6.0, 7.0], EPS, "accu1");
    close_all(&mixer.buffer[3..6], &[0.0, 0.0, 0.0], EPS, "accu0 untouched");

    mixer.apply(1);
    close_all(&read_target(&mixer), &[5.0, 6.0, 7.0], EPS, "value");
}

#[test]
fn save_and_restore_original_state_round_trips() {
    let mut mixer = vector_mixer(&[7.0, 8.0, 9.0]);

    set_incoming(&mut mixer, &[-1.0, -2.0, -3.0]);
    mixer.accumulate(0, 1.0);
    mixer.apply(0);
    close_all(&read_target(&mixer), &[-1.0, -2.0, -3.0], EPS, "changed");

    mixer.restore_original_state();
    close_all(&read_target(&mixer), &[7.0, 8.0, 9.0], EPS, "restored");
}

#[test]
fn quaternion_mixer_slerps_rather_than_lerps() {
    // orig = identity, incoming = 90 degrees about +Z.
    let half = std::f64::consts::FRAC_1_SQRT_2;
    let target = BufferTarget::new(vec![0.0, 0.0, 0.0, 1.0]);
    let mut mixer = PropertyMixer::new(Box::new(target), TrackValueType::Quaternion, 4);
    mixer.save_original_state();

    // `_setAdditiveIdentityQuaternion`: zeros plus w = 1.
    close_all(&mixer.buffer[16..20], &[0.0, 0.0, 0.0, 1.0], EPS, "add identity");

    set_incoming(&mut mixer, &[0.0, 0.0, half, half]);
    mixer.accumulate(0, 0.5);
    // apply -> `_slerp( accu0, orig, 0.5 )`: halfway along the arc from 90 deg
    // to identity is the 45 deg rotation, i.e. ( 0, 0, sin 22.5, cos 22.5 ).
    mixer.apply(0);

    let value = read_target(&mixer);
    let s = (22.5f64).to_radians().sin();
    let c = (22.5f64).to_radians().cos();
    close_all(&value, &[0.0, 0.0, s, c], EPS, "slerped");

    // A plain `_lerp` would have produced ( 0, 0, 0.353553, 0.853553 ), whose
    // length is 0.92388, not 1 — so the unit norm is what rules out `_lerp`.
    let norm = value.iter().map(|v| v * v).sum::<f64>().sqrt();
    close(norm, 1.0, EPS, "normalized");
}

#[test]
fn additive_accumulation_builds_on_the_additive_identity() {
    // cumulativeWeightAdditive === 0 -> `_setAdditiveIdentityNumeric` zeroes
    // `add`, then `_lerpAdditive( add, incoming, 0.5 )` gives [ 5, 10, 15 ].
    let mut mixer = vector_mixer(&[1.0, 2.0, 3.0]);

    set_incoming(&mut mixer, &[10.0, 20.0, 30.0]);
    mixer.accumulate_additive(0.5);

    close(
        mixer.cumulative_weight_additive,
        0.5,
        EPS,
        "cumulativeWeightAdditive",
    );
    close_all(&mixer.buffer[12..15], &[5.0, 10.0, 15.0], EPS, "add");

    // apply with cumulativeWeight 0: `_lerp( accu0, orig, 1 )` restores the
    // original into accu0, then `_lerpAdditive( accu0, add, 1 )` adds the
    // additive result on top: [ 1, 2, 3 ] + [ 5, 10, 15 ].
    mixer.apply(0);
    close_all(&read_target(&mixer), &[6.0, 12.0, 18.0], EPS, "value");
    close(
        mixer.cumulative_weight_additive,
        0.0,
        EPS,
        "cumulativeWeightAdditive",
    );
}

#[test]
fn two_additive_accumulations_sum() {
    // The identity is only installed on the first call, so the second
    // `_lerpAdditive` adds on top: [ 5, 10, 15 ] + [ 1, 1, 1 ] * 0.5.
    let mut mixer = vector_mixer(&[0.0, 0.0, 0.0]);

    set_incoming(&mut mixer, &[10.0, 20.0, 30.0]);
    mixer.accumulate_additive(0.5);
    set_incoming(&mut mixer, &[1.0, 1.0, 1.0]);
    mixer.accumulate_additive(0.5);

    close(
        mixer.cumulative_weight_additive,
        1.0,
        EPS,
        "cumulativeWeightAdditive",
    );
    close_all(&mixer.buffer[12..15], &[5.5, 10.5, 15.5], EPS, "add");
}

#[test]
fn bool_mixer_selects() {
    // `'bool'` uses `_select` for both mix functions: the destination is
    // overwritten only when t >= 0.5.
    //
    // weight 0.3 -> apply mixes with t = 1 - 0.3 = 0.7 >= 0.5, so the original
    // wins and the incoming `true` is discarded.
    let mut mixer =
        PropertyMixer::new(Box::new(BufferTarget::new(vec![0.0])), TrackValueType::Bool, 1);
    mixer.save_original_state();
    // `_setAdditiveIdentityOther` copies `orig` into `add`.
    close(mixer.buffer[4], 0.0, EPS, "add identity");

    set_incoming(&mut mixer, &[1.0]);
    mixer.accumulate(0, 0.3);
    mixer.apply(0);
    close_all(&read_target(&mixer), &[0.0], EPS, "original wins");

    // weight 0.6 -> t = 0.4 < 0.5, the incoming value survives.
    let mut mixer =
        PropertyMixer::new(Box::new(BufferTarget::new(vec![0.0])), TrackValueType::Bool, 1);
    mixer.save_original_state();
    set_incoming(&mut mixer, &[1.0]);
    mixer.accumulate(0, 0.6);
    mixer.apply(0);
    close_all(&read_target(&mixer), &[1.0], EPS, "incoming wins");
}

#[test]
fn apply_skips_the_binding_when_nothing_changed() {
    // The accus agree with the original after `saveOriginalState`, so an
    // `apply` with no accumulation leaves them equal and never calls setValue.
    let mut mixer = vector_mixer(&[1.0, 2.0, 3.0]);

    mixer.apply(0);
    close_all(&read_target(&mixer), &[1.0, 2.0, 3.0], EPS, "value");
    close_all(&mixer.buffer[3..6], &[1.0, 2.0, 3.0], EPS, "accu0");
}

#[test]
fn quaternion_additive_accumulation_multiplies_then_slerps() {
    // `_slerpAdditive` is `work := add * incoming` then `slerp( add, work, t )`.
    // From the quaternion additive identity, one accumulation at weight 1 of a
    // 90 degree rotation about +Z leaves `add` equal to that rotation, and
    // `apply` (cumulativeWeight 0, so accu0 := orig = identity) then composes it
    // onto the identity, giving the 90 degree rotation at the binding.
    let half = std::f64::consts::FRAC_1_SQRT_2;
    let target = BufferTarget::new(vec![0.0, 0.0, 0.0, 1.0]);
    let mut mixer = PropertyMixer::new(Box::new(target), TrackValueType::Quaternion, 4);
    mixer.save_original_state();

    set_incoming(&mut mixer, &[0.0, 0.0, half, half]);
    mixer.accumulate_additive(1.0);
    close_all(&mixer.buffer[16..20], &[0.0, 0.0, half, half], EPS, "add");

    mixer.apply(0);
    close_all(&read_target(&mixer), &[0.0, 0.0, half, half], EPS, "value");
}
