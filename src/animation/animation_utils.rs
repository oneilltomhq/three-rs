//! Port of `three.js/src/animation/AnimationUtils.js`.
//!
//! Only the clip-independent helpers live here. The three.js module also
//! exports `subclip()` and `makeClipAdditive()`, which operate on
//! `AnimationClip` / `KeyframeTrack`; they are **deferred to the clip port**
//! and will land next to those types rather than here.
//!
//! Three's `convertArray( array, type )` and `isTypedArray( object )` have no
//! Rust counterpart and are deliberately not exposed:
//!
//! * `convertArray` exists so a keyframe track can re-wrap a plain `Array` in
//!   whatever typed array the track stores (`Float32Array`, `Int32Array`, …).
//!   Every buffer in this port is already an f64 `Vec<f64>`, so the conversion
//!   is the identity and the call sites simply keep their `Vec`.
//! * `isTypedArray` distinguishes a typed array from a plain `Array` at
//!   runtime. In Rust the distinction is in the type, so the question cannot
//!   be asked.
//!
//! `hasTangents( settings )` is likewise a JS `undefined` check over a
//! loose keyframe-track settings object; it belongs with the track parsing
//! code that owns that struct.

/// `getKeyframeOrder( times )` — the permutation by which `times` (and, via
/// [`sorted_array`], the matching values) can be sorted.
///
/// Three sorts the index array with `Array.prototype.sort`, which is required
/// to be stable, so equal times keep their original relative order; Rust's
/// [`slice::sort_by`] is stable too.
pub fn get_keyframe_order(times: &[f64]) -> Vec<usize> {
    let n = times.len();
    let mut result: Vec<usize> = (0..n).collect();

    // `compareTime( i, j ) { return times[ i ] - times[ j ]; }`. A NaN time
    // makes the JS comparator return NaN, which `sort` treats as "keep the
    // current order"; `Equal` is the same instruction to a stable sort.
    result.sort_by(|&i, &j| {
        (times[i] - times[j])
            .partial_cmp(&0.0)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    result
}

/// `sortedArray( values, stride, order )` — reorders `values` in blocks of
/// `stride` according to the `order` from [`get_keyframe_order`].
pub fn sorted_array(values: &[f64], stride: usize, order: &[usize]) -> Vec<f64> {
    let n_values = values.len();
    let mut result = vec![0.0; n_values];

    let mut dst_offset = 0;
    let mut i = 0;
    while dst_offset != n_values {
        let src_offset = order[i] * stride;

        for j in 0..stride {
            result[dst_offset] = values[src_offset + j];
            dst_offset += 1;
        }

        i += 1;
    }

    result
}

/// `flattenJSON( jsonKeys, times, values, valuePropertyName )` — parses the
/// array-of-structs keyframe format that `AnimationClip.parse()` feeds in,
/// appending to `times` and `values`.
///
/// Three picks its branch from the first key that actually carries
/// `valuePropertyName`: an `Array` value is spread into `values`, a
/// `THREE.Math`-ish object is written through its `toArray()`, and anything
/// else is pushed as-is. JSON has no methods, so the `toArray` branch is
/// unreachable over [`serde_json::Value`] and is not ported.
///
/// A key missing `time` pushes `undefined` in Three; here it pushes `NaN`,
/// which is what that `undefined` becomes the moment the times are read as
/// numbers.
pub fn flatten_json(
    json_keys: &[serde_json::Value],
    times: &mut Vec<f64>,
    values: &mut Vec<f64>,
    value_property_name: &str,
) {
    let number = |v: &serde_json::Value| v.as_f64().unwrap_or(f64::NAN);

    // `let i = 1, key = jsonKeys[ 0 ];` then skip keys without the property.
    let mut i = 1;
    let mut key = json_keys.first();

    while let Some(k) = key {
        if k.get(value_property_name).is_some() {
            break;
        }
        key = json_keys.get(i);
        i += 1;
    }

    let Some(first) = key else {
        return; // no data
    };
    let value = &first[value_property_name];

    if value.is_array() {
        while let Some(k) = key {
            if let Some(value) = k.get(value_property_name) {
                times.push(number(&k["time"]));
                // `values.push( ...value )`
                values.extend(value.as_array().into_iter().flatten().map(number));
            }

            key = json_keys.get(i);
            i += 1;
        }
    } else {
        // Otherwise push as-is.
        while let Some(k) = key {
            if let Some(value) = k.get(value_property_name) {
                times.push(number(&k["time"]));
                values.push(number(value));
            }

            key = json_keys.get(i);
            i += 1;
        }
    }
}

use crate::math::Quaternion;

use super::animation_clip::{AnimationBlendMode, AnimationClip};
use super::keyframe_track::{KeyframeTrack, TrackValueType};

/// `subclip( sourceClip, name, startFrame, endFrame, fps = 30 )` — a new clip
/// holding only the segment of `sourceClip` between the given frames.
///
/// Three's `convertArray( times, track.times.constructor )` is the identity
/// here (every buffer is already a `Vec<f64>`), so the rebuilt `Vec`s are
/// assigned straight across.
pub fn subclip(
    source_clip: &AnimationClip,
    name: &str,
    start_frame: f64,
    end_frame: f64,
    fps: f64,
) -> AnimationClip {
    let mut clip = source_clip.clone_clip();

    clip.name = name.to_string();

    let mut tracks: Vec<KeyframeTrack> = Vec::new();

    for track in clip.tracks.iter_mut() {
        let value_size = track.get_value_size();

        let mut times = Vec::new();
        let mut values = Vec::new();

        for j in 0..track.times.len() {
            let frame = track.times[j] * fps;

            if frame < start_frame || frame >= end_frame {
                continue;
            }

            times.push(track.times[j]);

            for k in 0..value_size {
                values.push(track.values[j * value_size + k]);
            }
        }

        if times.is_empty() {
            continue;
        }

        track.times = times;
        track.values = values;

        tracks.push(track.clone_track());
    }

    clip.tracks = tracks;

    // find minimum .times value across all tracks in the trimmed clip

    let mut min_start_time = f64::INFINITY;

    for track in &clip.tracks {
        if min_start_time > track.times[0] {
            min_start_time = track.times[0];
        }
    }

    // shift all tracks such that clip begins at t=0

    for track in clip.tracks.iter_mut() {
        track.shift(-1.0 * min_start_time);
    }

    clip.reset_duration();

    clip
}

/// `makeClipAdditive( targetClip, referenceFrame = 0, referenceClip = targetClip, fps = 30 )`
/// — rewrites `target_clip`'s keyframes relative to the values at the reference
/// frame, and flips its blend mode to additive.
///
/// `reference_clip` of `None` is Three's `referenceClip = targetClip` default.
///
/// DEFERRED: the two `createInterpolant.isInterpolantFactoryMethodGLTFCubicSpline`
/// branches. That flag is set by `GLTFLoader`'s cubic-spline interpolant, which
/// is not ported; until it is, both offsets are necessarily `0`, exactly as they
/// are for every non-glTF clip in Three. Nothing else of the function is
/// missing — it never touches `PropertyBinding` or the object tree.
pub fn make_clip_additive(
    target_clip: &mut AnimationClip,
    reference_frame: f64,
    reference_clip: Option<&AnimationClip>,
    fps: f64,
) {
    let mut fps = fps;
    if fps <= 0.0 {
        fps = 30.0;
    }

    // `referenceClip = targetClip`: the reference is only read, and only before
    // the matching target track is written, so a snapshot reproduces it.
    let reference_snapshot: AnimationClip;
    let reference: &AnimationClip = match reference_clip {
        Some(clip) => clip,
        None => {
            reference_snapshot = target_clip.clone_clip();
            &reference_snapshot
        }
    };

    let num_tracks = reference.tracks.len();
    let reference_time = reference_frame / fps;

    // Make each track's values relative to the values at the reference frame
    for i in 0..num_tracks {
        let reference_track = &reference.tracks[i];
        let reference_track_type = reference_track.value_type();

        // Skip this track if it's non-numeric
        if reference_track_type == TrackValueType::Bool
            || reference_track_type == TrackValueType::String
        {
            continue;
        }

        // Find the track in the target clip whose name and type matches the reference track
        let Some(target_index) = target_clip.tracks.iter().position(|track| {
            track.name == reference_track.name && track.value_type() == reference_track_type
        }) else {
            continue;
        };

        // `referenceOffset` / `targetOffset` — see DEFERRED above.
        let reference_offset = 0usize;
        let reference_value_size = reference_track.get_value_size();

        let target_offset = 0usize;
        let target_value_size = target_clip.tracks[target_index].get_value_size();

        let last_index = reference_track.times.len() - 1;
        let mut reference_value: Vec<f64>;

        // Find the value to subtract out of the track
        if reference_time <= reference_track.times[0] {
            // Reference frame is earlier than the first keyframe, so just use the first keyframe
            let start_index = reference_offset;
            let end_index = reference_value_size - reference_offset;
            reference_value = reference_track.values[start_index..end_index].to_vec();
        } else if reference_time >= reference_track.times[last_index] {
            // Reference frame is after the last keyframe, so just use the last keyframe
            let start_index = last_index * reference_value_size + reference_offset;
            let end_index = start_index + reference_value_size - reference_offset;
            reference_value = reference_track.values[start_index..end_index].to_vec();
        } else {
            // Interpolate to the reference value
            let start_index = reference_offset;
            let end_index = reference_value_size - reference_offset;
            reference_value = match reference_track.create_interpolant(None) {
                Some(mut interpolant) => {
                    interpolant.evaluate(reference_time);
                    interpolant.result_buffer()[start_index..end_index].to_vec()
                }
                // No interpolant exists only for a Bezier track, which Three
                // would crash on here; fall back to the first keyframe.
                None => reference_track.values[start_index..end_index].to_vec(),
            };
        }

        // Conjugate the quaternion
        if reference_track_type == TrackValueType::Quaternion {
            let mut reference_quat = Quaternion::default();
            reference_quat.from_array(&reference_value, 0);
            reference_quat.normalize().conjugate();
            reference_value[..4].copy_from_slice(&reference_quat.to_array());
        }

        // Subtract the reference value from all of the track values

        let target_track = &mut target_clip.tracks[target_index];
        let num_times = target_track.times.len();
        for j in 0..num_times {
            let value_start = j * target_value_size + target_offset;

            if reference_track_type == TrackValueType::Quaternion {
                // Multiply the conjugate for quaternion track types
                let (x, y, z, w) = (
                    target_track.values[value_start],
                    target_track.values[value_start + 1],
                    target_track.values[value_start + 2],
                    target_track.values[value_start + 3],
                );
                let src0 = [x, y, z, w];
                Quaternion::multiply_quaternions_flat(
                    &mut target_track.values,
                    value_start,
                    &src0,
                    0,
                    &reference_value,
                    0,
                );
            } else {
                let value_end = target_value_size - target_offset * 2;

                // Subtract each value for all other numeric track types
                for (t, r) in target_track.values[value_start..value_start + value_end]
                    .iter_mut()
                    .zip(reference_value.iter())
                {
                    *t -= *r;
                }
            }
        }
    }

    target_clip.blend_mode = AnimationBlendMode::Additive;
}
