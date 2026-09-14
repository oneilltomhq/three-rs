//! Port of `three.js/test/unit/src/animation/AnimationClip.tests.js`, plus
//! coverage for the pure-data parts of `AnimationClip` that Three's (two-test)
//! QUnit suite leaves untested: the `parse` / `toJSON` round trip,
//! `CreateFromMorphTargetSequence`, `CreateClipsFromMorphTargetSequences`,
//! `findByName`, `resetDuration`, `trim`, `clone`, and
//! `AnimationUtils.subclip` / `makeClipAdditive`.

use serde_json::{json, Value};
use three_rs::animation::animation_clip::AnimationBlendMode;
use three_rs::animation::animation_utils::{make_clip_additive, subclip};
use three_rs::animation::{AnimationClip, KeyframeTrack};

/// `new NumberKeyframeTrack( name, times, values )`, unwrapped.
fn number_track(name: &str, times: &[f64], values: &[f64]) -> KeyframeTrack {
    KeyframeTrack::number(name, times.to_vec(), values.to_vec(), None).unwrap()
}

// INSTANCING

#[test]
fn instancing() {
    // Three writes `new AnimationClip( 'clip1', 1000, [ {} ] )` — a single
    // placeholder track object that is never read. A `KeyframeTrack` cannot be
    // empty here, so a real one-keyframe track stands in.
    let clip = AnimationClip::new(
        "clip1",
        1000.0,
        vec![number_track(".foo", &[0.0], &[0.0])],
        AnimationBlendMode::Normal,
    );
    assert_eq!(clip.tracks.len(), 1, "AnimationClip can be instantiated");
    assert_eq!(clip.duration, 1000.0);
}

// PROPERTIES

#[test]
fn name() {
    let clip = AnimationClip::new(
        "clip1",
        1000.0,
        vec![number_track(".foo", &[0.0], &[0.0])],
        AnimationBlendMode::Normal,
    );
    assert!(clip.name == "clip1", "AnimationClip can be named");
}

#[test]
fn negative_duration_scans_the_tracks() {
    // `if ( this.duration < 0 ) this.resetDuration();`
    let clip = AnimationClip::from_tracks(
        "clip",
        vec![
            number_track(".a", &[0.0, 1.0, 2.0], &[0.0, 1.0, 2.0]),
            number_track(".b", &[0.0, 5.0], &[0.0, 1.0]),
        ],
    );

    assert_eq!(clip.duration, 5.0);
    assert_eq!(clip.blend_mode, AnimationBlendMode::Normal);
    assert!(!clip.uuid.is_empty());
}

// STATIC

#[test]
fn blend_mode_constants() {
    assert_eq!(AnimationBlendMode::Normal.as_number(), 2500);
    assert_eq!(AnimationBlendMode::Additive.as_number(), 2501);
    assert_eq!(
        AnimationBlendMode::from_number(2501),
        Some(AnimationBlendMode::Additive)
    );
    assert_eq!(AnimationBlendMode::from_number(0), None);
}

#[test]
fn to_json_round_trips_through_parse() {
    let mut clip = AnimationClip::new(
        "clip1",
        2.0,
        vec![
            number_track(".position[x]", &[0.0, 1.0, 2.0], &[0.0, 10.0, 20.0]),
            KeyframeTrack::quaternion(
                ".quaternion",
                vec![0.0, 1.0],
                vec![0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0],
                None,
            )
            .unwrap(),
        ],
        AnimationBlendMode::Additive,
    );
    clip.user_data.insert("foo".into(), json!(7));

    let json = clip.to_json();

    assert_eq!(json["name"], json!("clip1"));
    assert_eq!(json["duration"], json!(2.0));
    assert_eq!(json["uuid"], json!(clip.uuid));
    assert_eq!(json["blendMode"], json!(2501));
    // `userData` is serialized as a JSON *string*.
    assert_eq!(json["userData"], json!(r#"{"foo":7}"#));
    assert_eq!(json["tracks"].as_array().unwrap().len(), 2);

    // `AnimationClip.parse()` divides the times by `json.fps || 1`, and
    // `toJSON` writes no fps, so the round trip is lossless.
    let parsed = AnimationClip::parse(&json).unwrap();

    assert_eq!(parsed.name, clip.name);
    assert_eq!(parsed.duration, clip.duration);
    assert_eq!(parsed.uuid, clip.uuid);
    assert_eq!(parsed.blend_mode, AnimationBlendMode::Additive);
    assert_eq!(parsed.user_data, clip.user_data);
    assert_eq!(parsed.tracks, clip.tracks);
    assert_eq!(parsed.to_json(), json);
}

#[test]
fn parse_applies_fps_and_defaults() {
    // No `duration` (so the constructor's `- 1` default reruns
    // `resetDuration`), no name, no blendMode, no userData, fps = 2.
    let json = json!({
        "fps": 2.0,
        "tracks": [ {
            "name": ".morphTargetInfluences[a]",
            "type": "number",
            "times": [ 0.0, 1.0, 2.0 ],
            "values": [ 0.0, 1.0, 0.0 ]
        } ]
    });

    let clip = AnimationClip::parse(&json).unwrap();

    assert_eq!(clip.name, "");
    assert_eq!(clip.blend_mode, AnimationBlendMode::Normal);
    assert!(clip.user_data.is_empty());
    // `.scale( 1 / 2 )`
    assert_eq!(clip.tracks[0].times, vec![0.0, 0.5, 1.0]);
    assert_eq!(clip.duration, 1.0);
}

#[test]
fn parse_rejects_a_track_without_a_type() {
    // Three throws `'THREE.KeyframeTrack: track type undefined, can not parse'`.
    let json = json!({ "tracks": [ { "name": ".foo", "times": [ 0 ], "values": [ 0 ] } ] });

    assert!(AnimationClip::parse(&json).is_err());
}

#[test]
fn parse_accepts_the_aos_keys_form() {
    // The `json.times === undefined` branch, via `flattenJSON`.
    let json = json!({
        "name": "aos",
        "tracks": [ {
            "name": ".foo",
            "type": "number",
            "keys": [ { "time": 0.0, "value": 1.0 }, { "time": 1.0, "value": 2.0 } ]
        } ]
    });

    let clip = AnimationClip::parse(&json).unwrap();

    assert_eq!(clip.tracks[0].times, vec![0.0, 1.0]);
    assert_eq!(clip.tracks[0].values, vec![1.0, 2.0]);
    assert_eq!(clip.duration, 1.0);
}

#[test]
fn create_from_morph_target_sequence() {
    let clip =
        AnimationClip::create_from_morph_target_sequence("walk", &["a", "b", "c"], 2.0, false)
            .unwrap();

    assert_eq!(clip.name, "walk");
    assert_eq!(clip.tracks.len(), 3);

    // i = 0: times [ 2, 0, 1 ] sorted to [ 0, 1, 2 ] with values [ 1, 0, 0 ],
    // then the t = 0 key duplicated at t = numMorphTargets, then `.scale( 1/2 )`.
    assert_eq!(clip.tracks[0].name, ".morphTargetInfluences[a]");
    assert_eq!(clip.tracks[0].times, vec![0.0, 0.5, 1.0, 1.5]);
    assert_eq!(clip.tracks[0].values, vec![1.0, 0.0, 0.0, 1.0]);

    // i = 1: times [ 0, 1, 2 ] are already sorted.
    assert_eq!(clip.tracks[1].name, ".morphTargetInfluences[b]");
    assert_eq!(clip.tracks[1].times, vec![0.0, 0.5, 1.0, 1.5]);
    assert_eq!(clip.tracks[1].values, vec![0.0, 1.0, 0.0, 0.0]);

    // i = 2: times [ 1, 2, 0 ] sorted to [ 0, 1, 2 ] with values [ 0, 0, 1 ].
    assert_eq!(clip.tracks[2].name, ".morphTargetInfluences[c]");
    assert_eq!(clip.tracks[2].values, vec![0.0, 0.0, 1.0, 0.0]);

    assert_eq!(clip.duration, 1.5);
}

#[test]
fn create_from_morph_target_sequence_no_loop() {
    // `noLoop` suppresses the duplicated wrap-around keyframe.
    let clip =
        AnimationClip::create_from_morph_target_sequence("walk", &["a", "b"], 1.0, true).unwrap();

    // n = 2, i = 0: times [ 1, 0, 1 ] sort (stably) to [ 0, 1, 1 ] with values
    // [ 1, 0, 0 ]; `noLoop` suppresses the duplicated wrap-around keyframe.
    assert_eq!(clip.tracks[0].times, vec![0.0, 1.0, 1.0]);
    assert_eq!(clip.tracks[0].values, vec![1.0, 0.0, 0.0]);
    assert_eq!(clip.duration, 1.0);
}

#[test]
fn create_clips_from_morph_target_sequences() {
    // Three's own example names, plus one that must not match the pattern.
    let clips = AnimationClip::create_clips_from_morph_target_sequences(
        &["Walk_001", "Walk_002", "Run_001", "Run_002", "nodigits"],
        1.0,
        true,
    )
    .unwrap();

    assert_eq!(clips.len(), 2);
    // Insertion order, as for `for ( const name in … )` over a JS object.
    assert_eq!(clips[0].name, "Walk_");
    assert_eq!(clips[1].name, "Run_");
    assert_eq!(clips[0].tracks.len(), 2);
    assert_eq!(clips[0].tracks[0].name, ".morphTargetInfluences[Walk_001]");
    // `crdeath0059`: the whole trailing digit run is the number group.
    let clips = AnimationClip::create_clips_from_morph_target_sequences(
        &["crdeath0059", "flamingo_flyA_003"],
        1.0,
        true,
    )
    .unwrap();
    assert_eq!(clips[0].name, "crdeath");
    assert_eq!(clips[1].name, "flamingo_flyA_");
}

#[test]
fn find_by_name() {
    // SKIPPED: the `Object3D` / geometry overload — the ported `Object3D` holds
    // no `animations` array, so only the clip-array form is reachable.
    let clips = vec![
        AnimationClip::from_tracks("a", vec![number_track(".x", &[0.0], &[0.0])]),
        AnimationClip::from_tracks("b", vec![number_track(".x", &[0.0], &[0.0])]),
    ];

    assert_eq!(AnimationClip::find_by_name(&clips, "b").unwrap().name, "b");
    assert!(AnimationClip::find_by_name(&clips, "c").is_none());
    assert!(AnimationClip::find_by_name(&[], "a").is_none());
}

// PUBLIC

#[test]
fn reset_duration() {
    let mut clip = AnimationClip::new(
        "clip",
        1000.0,
        vec![
            number_track(".a", &[0.0, 1.0], &[0.0, 1.0]),
            number_track(".b", &[0.0, 3.5], &[0.0, 1.0]),
        ],
        AnimationBlendMode::Normal,
    );

    assert_eq!(clip.duration, 1000.0);
    clip.reset_duration();
    // `Math.max` over each track's last time.
    assert_eq!(clip.duration, 3.5);

    // No tracks at all: `duration` starts at 0 and stays there.
    let mut empty = AnimationClip::new("empty", 10.0, Vec::new(), AnimationBlendMode::Normal);
    empty.reset_duration();
    assert_eq!(empty.duration, 0.0);
}

#[test]
fn trim() {
    let mut clip = AnimationClip::new(
        "clip",
        1.0,
        vec![number_track(
            ".a",
            &[0.0, 1.0, 2.0, 3.0],
            &[0.0, 10.0, 20.0, 30.0],
        )],
        AnimationBlendMode::Normal,
    );

    clip.trim();

    // `track.trim( 0, 1 )` keeps the keys inside [ 0, 1 ].
    assert_eq!(clip.tracks[0].times, vec![0.0, 1.0]);
    assert_eq!(clip.tracks[0].values, vec![0.0, 10.0]);
}

#[test]
fn validate_and_optimize() {
    let mut clip = AnimationClip::from_tracks(
        "clip",
        vec![number_track(
            ".a",
            &[0.0, 1.0, 2.0, 3.0],
            &[1.0, 1.0, 1.0, 1.0],
        )],
    );

    assert!(clip.validate());

    clip.optimize();
    // Equivalent sequential keys collapse to the two endpoints.
    assert_eq!(clip.tracks[0].times, vec![0.0, 3.0]);
    assert_eq!(clip.tracks[0].values, vec![1.0, 1.0]);
}

#[test]
fn clone() {
    let mut clip = AnimationClip::new(
        "clip",
        4.0,
        vec![number_track(".a", &[0.0, 1.0], &[0.0, 1.0])],
        AnimationBlendMode::Additive,
    );
    clip.user_data.insert("k".into(), Value::from("v"));

    let cloned = clip.clone_clip();

    assert_eq!(cloned.name, clip.name);
    assert_eq!(cloned.duration, 4.0);
    assert_eq!(cloned.blend_mode, AnimationBlendMode::Additive);
    assert_eq!(cloned.tracks, clip.tracks);
    assert_eq!(cloned.user_data, clip.user_data);
    // The constructor runs, so the clone gets a fresh uuid.
    assert_ne!(cloned.uuid, clip.uuid);
}

// AnimationUtils.subclip / makeClipAdditive

#[test]
fn subclip_keeps_the_requested_frame_window() {
    let source = AnimationClip::new(
        "source",
        4.0,
        vec![number_track(
            ".a",
            &[0.0, 1.0, 2.0, 3.0, 4.0],
            &[0.0, 10.0, 20.0, 30.0, 40.0],
        )],
        AnimationBlendMode::Normal,
    );

    // fps = 1, so frame == time. `frame < startFrame || frame >= endFrame`
    // keeps frames 1, 2 and 3.
    let clip = subclip(&source, "sub", 1.0, 4.0, 1.0);

    assert_eq!(clip.name, "sub");
    // Shifted so the clip begins at t = 0.
    assert_eq!(clip.tracks[0].times, vec![0.0, 1.0, 2.0]);
    assert_eq!(clip.tracks[0].values, vec![10.0, 20.0, 30.0]);
    assert_eq!(clip.duration, 2.0);

    // The source is untouched (`sourceClip.clone()`).
    assert_eq!(source.tracks[0].times, vec![0.0, 1.0, 2.0, 3.0, 4.0]);
    assert_eq!(source.duration, 4.0);
}

#[test]
fn subclip_drops_tracks_with_no_keys_in_range() {
    let source = AnimationClip::new(
        "source",
        4.0,
        vec![
            number_track(".a", &[0.0, 2.0], &[0.0, 20.0]),
            number_track(".b", &[0.0], &[0.0]),
        ],
        AnimationBlendMode::Normal,
    );

    let clip = subclip(&source, "sub", 1.0, 3.0, 1.0);

    assert_eq!(clip.tracks.len(), 1);
    assert_eq!(clip.tracks[0].name, ".a");
    assert_eq!(clip.tracks[0].times, vec![0.0]);
}

#[test]
fn make_clip_additive_subtracts_the_reference_frame() {
    let mut clip = AnimationClip::new(
        "clip",
        2.0,
        vec![
            number_track(".a", &[0.0, 1.0, 2.0], &[1.0, 2.0, 3.0]),
            // A string track is skipped as non-numeric.
            KeyframeTrack::string(".s", vec![0.0], vec!["x".into()]).unwrap(),
        ],
        AnimationBlendMode::Normal,
    );

    // referenceFrame 0 at fps 30 → referenceTime 0 ≤ times[ 0 ], so the first
    // keyframe's value (1) is subtracted from every key.
    make_clip_additive(&mut clip, 0.0, None, 30.0);

    assert_eq!(clip.tracks[0].values, vec![0.0, 1.0, 2.0]);
    assert_eq!(clip.tracks[1].strings, vec!["x".to_string()]);
    assert_eq!(clip.blend_mode, AnimationBlendMode::Additive);
}

#[test]
fn make_clip_additive_interpolates_an_interior_reference_frame() {
    let mut clip = AnimationClip::new(
        "clip",
        2.0,
        vec![number_track(".a", &[0.0, 1.0, 2.0], &[0.0, 10.0, 20.0])],
        AnimationBlendMode::Normal,
    );

    // referenceFrame 15 at fps 30 → referenceTime 0.5, interpolated to 5.
    make_clip_additive(&mut clip, 15.0, None, 30.0);

    assert_eq!(clip.tracks[0].values, vec![-5.0, 5.0, 15.0]);
}

#[test]
fn make_clip_additive_multiplies_the_conjugate_for_quaternions() {
    // Reference quaternion == the only target value, so target * conj(ref) is
    // the identity.
    let values = vec![0.0, 0.0, 0.70710678118654752, 0.70710678118654752];
    let mut clip = AnimationClip::new(
        "clip",
        1.0,
        vec![KeyframeTrack::quaternion(".quaternion", vec![0.0], values, None).unwrap()],
        AnimationBlendMode::Normal,
    );

    make_clip_additive(&mut clip, 0.0, None, 30.0);

    let out = &clip.tracks[0].values;
    for (i, expected) in [0.0, 0.0, 0.0, 1.0].iter().enumerate() {
        assert!(
            (out[i] - expected).abs() < 1e-12,
            "quaternion[{i}]: {} != {expected}",
            out[i]
        );
    }
    assert_eq!(clip.blend_mode, AnimationBlendMode::Additive);
}

#[test]
fn make_clip_additive_uses_a_separate_reference_clip() {
    let reference = AnimationClip::new(
        "reference",
        1.0,
        vec![number_track(".a", &[0.0, 1.0], &[100.0, 200.0])],
        AnimationBlendMode::Normal,
    );
    let mut target = AnimationClip::new(
        "target",
        1.0,
        vec![
            number_track(".a", &[0.0, 1.0], &[1.0, 2.0]),
            // No matching track in the reference clip: left alone.
            number_track(".b", &[0.0], &[5.0]),
        ],
        AnimationBlendMode::Normal,
    );

    // referenceTime 2 ≥ the reference track's last time, so its last value
    // (200) is subtracted.
    make_clip_additive(&mut target, 60.0, Some(&reference), 30.0);

    assert_eq!(target.tracks[0].values, vec![-199.0, -198.0]);
    assert_eq!(target.tracks[1].values, vec![5.0]);
}
