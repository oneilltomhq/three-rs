//! Port of `three.js/test/unit/src/animation/KeyframeTrack.tests.js` and
//! `three.js/test/unit/src/animation/tracks/*.tests.js`.
//!
//! Three's `Extending` tests assert `object instanceof KeyframeTrack`; the Rust
//! port has one `KeyframeTrack` struct whose `value_type()` says which subclass
//! it is, so those assertions become a `value_type()` check.

use three_rs::animation::keyframe_track::{InterpolationMode, KeyframeTrack, TrackValueType};

// KeyframeTrack, through `NumberKeyframeTrack` as Three's suite does.
//
// const parameters = { name: '.material.opacity', times: [ 0, 1 ],
//                      values: [ 0, 0.5 ], interpolation: DefaultInterpolation };

fn parameters() -> (&'static str, Vec<f64>, Vec<f64>, InterpolationMode) {
    (
        ".material.opacity",
        vec![0.0, 1.0],
        vec![0.0, 0.5],
        TrackValueType::Number.default_interpolation(),
    )
}

// INHERITANCE
#[test]
fn extending() {
    let (name, times, values, _) = parameters();
    let object = KeyframeTrack::number(name, times, values, None).unwrap();
    assert_eq!(
        object.value_type(),
        TrackValueType::Number,
        "NumberKeyframeTrack extends from KeyframeTrack"
    );
}

// INSTANCING
#[test]
fn instancing() {
    let (name, times, values, interpolation) = parameters();

    // name, times, values
    let object = KeyframeTrack::number(name, times.clone(), values.clone(), None);
    assert!(object.is_ok(), "Can instantiate a NumberKeyframeTrack.");

    // name, times, values, interpolation
    let object_all = KeyframeTrack::number(name, times, values, Some(interpolation));
    assert!(
        object_all.is_ok(),
        "Can instantiate a NumberKeyframeTrack with name, times, values, interpolation."
    );
}

// PUBLIC

#[test]
fn validate() {
    let valid_track =
        KeyframeTrack::number(".material.opacity", vec![0.0, 1.0], vec![0.0, 0.5], None).unwrap();
    let invalid_track = KeyframeTrack::number(
        ".material.opacity",
        vec![0.0, 1.0],
        vec![0.0, f64::NAN],
        None,
    )
    .unwrap();

    assert!(valid_track.validate());

    assert!(!invalid_track.validate());
}

#[test]
fn optimize() {
    let mut track = KeyframeTrack::number(
        ".material.opacity",
        vec![0.0, 1.0, 2.0, 3.0, 4.0],
        vec![0.0, 0.0, 0.0, 0.0, 1.0],
        None,
    )
    .unwrap();

    assert_eq!(track.values.len(), 5);

    track.optimize();

    assert_eq!(track.times, [0.0, 3.0, 4.0]);
    assert_eq!(track.values, [0.0, 0.0, 1.0]);
}

// Not in Three's suite, but the three methods the clip/mixer work leans on.

#[test]
fn shift_scale_trim() {
    let mut track = KeyframeTrack::number(
        ".material.opacity",
        vec![0.0, 1.0, 2.0, 3.0],
        vec![0.0, 1.0, 2.0, 3.0],
        None,
    )
    .unwrap();

    track.shift(1.0);
    assert_eq!(track.times, [1.0, 2.0, 3.0, 4.0]);

    track.scale(2.0);
    assert_eq!(track.times, [2.0, 4.0, 6.0, 8.0]);

    track.trim(4.0, 6.0);
    assert_eq!(track.times, [4.0, 6.0]);
    assert_eq!(track.values, [1.0, 2.0]);

    // empty tracks are forbidden, so trimming past the end keeps one keyframe
    let mut track = KeyframeTrack::number(
        ".material.opacity",
        vec![0.0, 1.0, 2.0],
        vec![0.0, 1.0, 2.0],
        None,
    )
    .unwrap();
    track.trim(10.0, 20.0);
    assert_eq!(track.times, [2.0]);
    assert_eq!(track.values, [2.0]);
}

#[test]
fn create_interpolant_evaluates_the_track() {
    let track = KeyframeTrack::number(
        ".material.opacity",
        vec![0.0, 1.0],
        vec![0.0, 0.5],
        None,
    )
    .unwrap();

    let mut interpolant = track.create_interpolant(None).unwrap();
    assert_eq!(interpolant.evaluate(0.5), [0.25]);

    // `InterpolateDiscrete` holds the preceding sample instead
    let track = KeyframeTrack::number(
        ".material.opacity",
        vec![0.0, 1.0],
        vec![0.0, 0.5],
        Some(InterpolationMode::Discrete),
    )
    .unwrap();
    let mut interpolant = track.create_interpolant(None).unwrap();
    assert_eq!(interpolant.evaluate(0.5), [0.0]);
}

// `setInterpolation` falls back to the default when the subclass suppresses the
// requested factory method (`InterpolantFactoryMethodSmooth = undefined`).
#[test]
fn set_interpolation_falls_back() {
    let mut track =
        KeyframeTrack::quaternion(".quaternion", vec![0.0], vec![0.5, 0.5, 0.5, 1.0], None).unwrap();

    assert_eq!(track.get_interpolation(), InterpolationMode::Linear);
    track.set_interpolation(InterpolationMode::Smooth).unwrap();
    assert_eq!(track.get_interpolation(), InterpolationMode::Linear);

    let mut track = KeyframeTrack::boolean(".visible", vec![0.0, 1.0], &[true, false]).unwrap();
    assert_eq!(track.get_interpolation(), InterpolationMode::Discrete);
    track.set_interpolation(InterpolationMode::Linear).unwrap();
    assert_eq!(track.get_interpolation(), InterpolationMode::Discrete);
}

// `KeyframeTrack.toJSON` / `AnimationClip.parseKeyframeTrack`
#[test]
fn to_json_and_parse_round_trip() {
    let track = KeyframeTrack::vector(
        ".position",
        vec![0.0, 1.0],
        vec![0.0, 0.0, 0.0, 1.0, 2.0, 3.0],
        Some(InterpolationMode::Discrete),
    )
    .unwrap();

    let json = track.to_json();
    assert_eq!(json["type"], "vector");
    assert_eq!(json["interpolation"], 2300);

    let parsed = KeyframeTrack::parse(&json).unwrap();
    assert_eq!(parsed, track);

    // the default interpolation is left out of the JSON
    let track = KeyframeTrack::number(".material.opacity", vec![0.0], vec![1.0], None).unwrap();
    let json = track.to_json();
    assert!(json.get("interpolation").is_none());
    assert_eq!(KeyframeTrack::parse(&json).unwrap(), track);
}

// Tracks

// BooleanKeyframeTrack — name: '.visible', times: [ 0, 1 ], values: [ true, false ]
#[test]
fn boolean_keyframe_track() {
    let object = KeyframeTrack::boolean(".visible", vec![0.0, 1.0], &[true, false]).unwrap();
    assert_eq!(
        object.value_type(),
        TrackValueType::Bool,
        "BooleanKeyframeTrack extends from KeyframeTrack"
    );
    assert_eq!(object.values, [1.0, 0.0]);
    assert_eq!(object.to_json()["values"], serde_json::json!([true, false]));
}

// ColorKeyframeTrack — name: '.material.color', times: [ 0 ], values: [ 1, 1, 1 ]
#[test]
fn color_keyframe_track() {
    let object =
        KeyframeTrack::color(".material.color", vec![0.0], vec![1.0, 1.0, 1.0], None).unwrap();
    assert_eq!(object.value_type(), TrackValueType::Color);
    assert_eq!(object.get_value_size(), 3);
}

// NumberKeyframeTrack
#[test]
fn number_keyframe_track() {
    let (name, times, values, interpolation) = parameters();
    let object = KeyframeTrack::number(name, times.clone(), values.clone(), None).unwrap();
    assert_eq!(object.value_type(), TrackValueType::Number);

    let object_all = KeyframeTrack::number(name, times, values, Some(interpolation));
    assert!(object_all.is_ok());
}

// QuaternionKeyframeTrack — name: '.rotation', times: [ 0 ], values: [ .5, .5, .5, 1 ]
#[test]
fn quaternion_keyframe_track() {
    let object =
        KeyframeTrack::quaternion(".rotation", vec![0.0], vec![0.5, 0.5, 0.5, 1.0], None).unwrap();
    assert_eq!(object.value_type(), TrackValueType::Quaternion);
    assert_eq!(object.get_value_size(), 4);

    // `InterpolantFactoryMethodLinear` is the quaternion SLERP one
    let object = KeyframeTrack::quaternion(
        ".rotation",
        vec![0.0, 1.0],
        vec![
            0.0,
            0.0,
            0.0,
            1.0,
            0.0,
            0.0,
            std::f64::consts::FRAC_1_SQRT_2,
            std::f64::consts::FRAC_1_SQRT_2,
        ],
        None,
    )
    .unwrap();
    let mut interpolant = object.create_interpolant(None).unwrap();
    let actual = interpolant.evaluate(0.5).to_vec();
    let eighth = std::f64::consts::FRAC_PI_8;
    assert!((actual[2] - eighth.sin()).abs() < 1e-12);
    assert!((actual[3] - eighth.cos()).abs() < 1e-12);
}

// StringKeyframeTrack — name: '.name', times: [ 0 ], values: [ 'foo' ]
#[test]
fn string_keyframe_track() {
    let object = KeyframeTrack::string(".name", vec![0.0, 1.0], vec!["foo".into(), "bar".into()])
        .unwrap();
    assert_eq!(object.value_type(), TrackValueType::String);
    assert_eq!(object.get_interpolation(), InterpolationMode::Discrete);
    assert_eq!(object.get_value_size(), 1);
    assert_eq!(object.to_json()["values"], serde_json::json!(["foo", "bar"]));
    assert_eq!(
        KeyframeTrack::parse(&object.to_json()).unwrap().strings,
        ["foo", "bar"]
    );
}

// VectorKeyframeTrack — name: '.position', times: [ 0 ], values: [ 0, 0, 0 ]
#[test]
fn vector_keyframe_track() {
    let object = KeyframeTrack::vector(".position", vec![0.0], vec![0.0, 0.0, 0.0], None).unwrap();
    assert_eq!(object.value_type(), TrackValueType::Vector);
    assert_eq!(object.get_value_size(), 3);
}

// Three throws for an empty `times` array.
#[test]
fn no_keyframes_is_an_error() {
    assert_eq!(
        KeyframeTrack::number(".material.opacity", Vec::new(), Vec::new(), None),
        Err("THREE.KeyframeTrack: no keyframes in track named .material.opacity".to_string())
    );
}
