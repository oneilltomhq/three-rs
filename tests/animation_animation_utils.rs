//! Port of `three.js/src/animation/AnimationUtils.js`' clip-independent
//! helpers. Three ships no `AnimationUtils.tests.js`, so the expectations here
//! are hand-computed from the JS algorithm (and from the shapes
//! `AnimationClip.parse()` feeds `flattenJSON`).

use serde_json::json;
use three_rs::animation::animation_utils::{flatten_json, get_keyframe_order, sorted_array};

#[test]
fn get_keyframe_order_returns_the_sorting_permutation() {
    // Already sorted: the identity.
    assert_eq!(get_keyframe_order(&[0.0, 1.0, 2.0]), vec![0, 1, 2]);

    // `[ 0, 2, 1 ]`: the keyframe at index 2 comes second in time.
    assert_eq!(get_keyframe_order(&[0.0, 2.0, 1.0]), vec![0, 2, 1]);

    assert_eq!(get_keyframe_order(&[3.0, -1.0, 2.5, 0.0]), vec![1, 3, 2, 0]);

    assert_eq!(get_keyframe_order(&[]), Vec::<usize>::new());
}

#[test]
fn get_keyframe_order_is_stable_for_equal_times() {
    // `Array.prototype.sort` is stable, so the two keyframes at t = 1 keep
    // their original relative order (index 0 before index 2).
    assert_eq!(get_keyframe_order(&[1.0, 0.0, 1.0]), vec![1, 0, 2]);
}

#[test]
fn sorted_array_moves_whole_stride_blocks() {
    // Three's own pairing: times [ 0, 2, 1 ] with a stride-2 value buffer.
    let times = [0.0, 2.0, 1.0];
    let values = [0.0, 10.0, 2.0, 20.0, 1.0, 15.0];

    let order = get_keyframe_order(&times);
    assert_eq!(order, vec![0, 2, 1]);

    assert_eq!(
        sorted_array(&values, 2, &order),
        vec![0.0, 10.0, 1.0, 15.0, 2.0, 20.0]
    );

    // The times themselves are sorted by the same call with stride 1.
    assert_eq!(sorted_array(&times, 1, &order), vec![0.0, 1.0, 2.0]);
}

#[test]
fn sorted_array_handles_stride_four() {
    let order = vec![1, 0];
    let values = [0.0, 1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0];

    assert_eq!(
        sorted_array(&values, 4, &order),
        vec![4.0, 5.0, 6.0, 7.0, 0.0, 1.0, 2.0, 3.0]
    );
}

#[test]
fn flatten_json_spreads_array_values() {
    let keys = vec![
        json!({ "time": 0.0, "pos": [ 1.0, 2.0, 3.0 ] }),
        json!({ "time": 1.5, "pos": [ 4.0, 5.0, 6.0 ] }),
    ];

    let mut times = Vec::new();
    let mut values = Vec::new();
    flatten_json(&keys, &mut times, &mut values, "pos");

    assert_eq!(times, vec![0.0, 1.5]);
    assert_eq!(values, vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0]);
}

#[test]
fn flatten_json_pushes_scalar_values_as_is() {
    let keys = vec![
        json!({ "time": 0.0, "value": 5.0 }),
        json!({ "time": 1.0, "value": 7.0 }),
        json!({ "time": 2.0, "value": 9.0 }),
    ];

    let mut times = Vec::new();
    let mut values = Vec::new();
    flatten_json(&keys, &mut times, &mut values, "value");

    assert_eq!(times, vec![0.0, 1.0, 2.0]);
    assert_eq!(values, vec![5.0, 7.0, 9.0]);
}

#[test]
fn flatten_json_skips_keys_without_the_property() {
    // The leading scan walks past keys that lack `valuePropertyName`, then the
    // main loop restarts at the first key that has it — so nothing is dropped
    // apart from the property-less keys themselves, wherever they sit.
    let keys = vec![
        json!({ "time": 0.0 }),
        json!({ "time": 1.0, "value": 2.0 }),
        json!({ "time": 2.0 }),
        json!({ "time": 3.0, "value": 4.0 }),
    ];

    let mut times = Vec::new();
    let mut values = Vec::new();
    flatten_json(&keys, &mut times, &mut values, "value");

    assert_eq!(times, vec![1.0, 3.0]);
    assert_eq!(values, vec![2.0, 4.0]);
}

#[test]
fn flatten_json_appends_nothing_without_data() {
    // `if ( key === undefined ) return; // no data`
    let mut times = vec![99.0];
    let mut values = vec![99.0];

    flatten_json(&[], &mut times, &mut values, "value");
    flatten_json(
        &[json!({ "time": 0.0 }), json!({ "time": 1.0 })],
        &mut times,
        &mut values,
        "value",
    );

    assert_eq!(times, vec![99.0]);
    assert_eq!(values, vec![99.0]);
}
