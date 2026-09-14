//! Port of `three.js/src/animation/AnimationClip.js`.
//!
//! The six `KeyframeTrack` subclasses collapsed into one
//! [`KeyframeTrack`] + [`TrackValueType`] in this port, so
//! `getTrackTypeForValueTypeName()` / `parseKeyframeTrack()` live there as
//! [`KeyframeTrack::parse`]; this module calls into it.
//!
//! Three's `AnimationClip.parseAnimation()` — the old hierarchy/bone based
//! `.animation` JSON reader — no longer exists in the upstream source this is
//! ported from (it was removed along with `AnimationClip.CreateFromMorphTargetSequence`'s
//! sibling `parseAnimation` helper), so there is nothing to port.

use serde_json::{json, Map, Value};

use super::animation_utils;
use super::keyframe_track::KeyframeTrack;
use crate::error::Error;

/// `NormalAnimationBlendMode` / `AdditiveAnimationBlendMode` from
/// `three.js/src/constants.js`, as an enum because these two are the only
/// members of that group.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum AnimationBlendMode {
    /// `NormalAnimationBlendMode` (2500).
    #[default]
    Normal,
    /// `AdditiveAnimationBlendMode` (2501).
    Additive,
}

impl AnimationBlendMode {
    /// The numeric constant, as it appears in serialized clips.
    pub fn as_number(self) -> u32 {
        match self {
            Self::Normal => 2500,
            Self::Additive => 2501,
        }
    }

    /// The inverse of [`Self::as_number`]; `None` for any other value, which is
    /// Three's `undefined` blend mode (and therefore the default).
    pub fn from_number(n: u32) -> Option<Self> {
        match n {
            2500 => Some(Self::Normal),
            2501 => Some(Self::Additive),
            _ => None,
        }
    }
}

/// `MathUtils.generateUUID()`.
///
/// Three seeds its hex lookup from `Math.random()`. There is no `rand`
/// dependency in this crate, so the randomness comes from a xorshift over a
/// process-wide counter mixed with the wall clock; the output shape (the
/// `8-4-4-4-12` hex form with the version-4 and variant bits forced) is
/// identical, and `uuid` is only ever used as an opaque identity.
fn generate_uuid() -> String {
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::time::{SystemTime, UNIX_EPOCH};

    static COUNTER: AtomicU64 = AtomicU64::new(0);

    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(0);

    let counter = COUNTER.fetch_add(1, Ordering::Relaxed);
    let mut state = nanos ^ (counter.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1);

    let mut next = move || {
        // xorshift64*
        state ^= state >> 12;
        state ^= state << 25;
        state ^= state >> 27;
        state.wrapping_mul(0x2545_F491_4F6C_DD1D)
    };

    let mut bytes = [0u8; 16];
    for chunk in bytes.chunks_mut(8) {
        let r = next().to_le_bytes();
        chunk.copy_from_slice(&r[..chunk.len()]);
    }

    bytes[6] = (bytes[6] & 0x0f) | 0x40; // version 4
    bytes[8] = (bytes[8] & 0x3f) | 0x80; // variant

    let hex: String = bytes.iter().map(|b| format!("{b:02x}")).collect();

    format!(
        "{}-{}-{}-{}-{}",
        &hex[0..8],
        &hex[8..12],
        &hex[12..16],
        &hex[16..20],
        &hex[20..32]
    )
}

/// `AnimationClip` — a reusable set of keyframe tracks representing an
/// animation.
#[derive(Clone, Debug, PartialEq)]
pub struct AnimationClip {
    /// The clip's name.
    pub name: String,
    /// An array of keyframe tracks.
    pub tracks: Vec<KeyframeTrack>,
    /// The clip's duration in seconds.
    pub duration: f64,
    /// How the animation is blended when several play at once.
    pub blend_mode: AnimationBlendMode,
    /// The UUID of the animation clip.
    pub uuid: String,
    /// Free-form custom data, serialized as a JSON string by
    /// [`Self::to_json`].
    pub user_data: Map<String, Value>,
}

impl AnimationClip {
    /// `new AnimationClip( name, duration, tracks, blendMode )`.
    ///
    /// A negative `duration` means "figure it out by scanning the tracks", as
    /// in Three; pass `-1.0` for the JS default.
    pub fn new(
        name: &str,
        duration: f64,
        tracks: Vec<KeyframeTrack>,
        blend_mode: AnimationBlendMode,
    ) -> Self {
        let mut clip = Self {
            name: name.to_string(),
            tracks,
            duration,
            blend_mode,
            uuid: generate_uuid(),
            user_data: Map::new(),
        };

        // this means it should figure out its duration by scanning the tracks
        if clip.duration < 0.0 {
            clip.reset_duration();
        }

        clip
    }

    /// `new AnimationClip( name, - 1, tracks )` — the all-defaults form.
    pub fn from_tracks(name: &str, tracks: Vec<KeyframeTrack>) -> Self {
        Self::new(name, -1.0, tracks, AnimationBlendMode::Normal)
    }

    /// `AnimationClip.parse( json )`.
    ///
    /// Three throws from `parseKeyframeTrack()` for an unknown or missing track
    /// type; that propagates as `Err` here.
    pub fn parse(json: &Value) -> Result<Self, Error> {
        let mut tracks = Vec::new();
        let json_tracks = json
            .get("tracks")
            .and_then(Value::as_array)
            .map(|a| a.as_slice())
            .unwrap_or(&[]);
        // `1.0 / ( json.fps || 1.0 )` — a missing, zero or NaN fps is falsy.
        let fps = match json.get("fps").and_then(Value::as_f64) {
            Some(fps) if fps != 0.0 && !fps.is_nan() => fps,
            _ => 1.0,
        };
        let frame_time = 1.0 / fps;

        for json_track in json_tracks {
            let mut track = KeyframeTrack::parse(json_track)?;
            track.scale(frame_time);
            tracks.push(track);
        }

        // A missing `duration` is `undefined`, which takes the constructor's
        // `- 1` default; likewise a missing `name` takes `''`.
        let duration = json.get("duration").and_then(Value::as_f64).unwrap_or(-1.0);
        let name = json.get("name").and_then(Value::as_str).unwrap_or("");
        let blend_mode = json
            .get("blendMode")
            .and_then(Value::as_u64)
            .and_then(|n| AnimationBlendMode::from_number(n as u32))
            .unwrap_or_default();

        let mut clip = Self::new(name, duration, tracks, blend_mode);

        // `clip.uuid = json.uuid` — a missing uuid leaves the generated one
        // rather than Three's `undefined`.
        if let Some(uuid) = json.get("uuid").and_then(Value::as_str) {
            clip.uuid = uuid.to_string();
        }

        // `JSON.parse( json.userData || '{}' )`
        clip.user_data = json
            .get("userData")
            .and_then(Value::as_str)
            .and_then(|s| serde_json::from_str::<Value>(s).ok())
            .and_then(|v| match v {
                Value::Object(map) => Some(map),
                _ => None,
            })
            .unwrap_or_default();

        Ok(clip)
    }

    /// `AnimationClip.toJSON( clip )`, and the instance `toJSON()` — the latter
    /// just forwards to the former, so one method covers both.
    pub fn to_json(&self) -> Value {
        let tracks: Vec<Value> = self.tracks.iter().map(KeyframeTrack::to_json).collect();

        json!({
            "name": self.name,
            "duration": self.duration,
            "tracks": tracks,
            "uuid": self.uuid,
            "blendMode": self.blend_mode.as_number(),
            "userData": Value::Object(self.user_data.clone()).to_string(),
        })
    }

    /// `AnimationClip.CreateFromMorphTargetSequence( name, morphTargetSequence, fps, noLoop )`.
    ///
    /// Three only reads `.name` off each morph target, so the sequence is a
    /// slice of names here rather than of morph-target objects.
    pub fn create_from_morph_target_sequence(
        name: &str,
        morph_target_sequence: &[&str],
        fps: f64,
        no_loop: bool,
    ) -> Result<Self, Error> {
        let num_morph_targets = morph_target_sequence.len();
        let mut tracks = Vec::new();

        for (i, morph_target_name) in morph_target_sequence.iter().enumerate() {
            let mut times: Vec<f64> = Vec::new();
            let mut values: Vec<f64> = Vec::new();

            let n = num_morph_targets as f64;
            let i_f = i as f64;
            times.push((i_f + n - 1.0) % n);
            times.push(i_f);
            times.push((i_f + 1.0) % n);

            values.push(0.0);
            values.push(1.0);
            values.push(0.0);

            let order = animation_utils::get_keyframe_order(&times);
            times = animation_utils::sorted_array(&times, 1, &order);
            values = animation_utils::sorted_array(&values, 1, &order);

            // if there is a key at the first frame, duplicate it as the
            // last frame as well for perfect loop.
            if !no_loop && times[0] == 0.0 {
                times.push(n);
                values.push(values[0]);
            }

            let mut track = KeyframeTrack::number(
                &format!(".morphTargetInfluences[{}]", morph_target_name),
                times,
                values,
                None,
            )?;
            track.scale(1.0 / fps);
            tracks.push(track);
        }

        Ok(Self::new(name, -1.0, tracks, AnimationBlendMode::Normal))
    }

    /// `AnimationClip.findByName( objectOrClipArray, name )`.
    ///
    /// SKIPPED: the `Object3D` overload (`o.geometry.animations || o.animations`)
    /// — the ported `Object3D` carries no `animations` list, so only the array
    /// form exists.
    pub fn find_by_name<'a>(
        clip_array: &'a [AnimationClip],
        name: &str,
    ) -> Option<&'a AnimationClip> {
        clip_array.iter().find(|clip| clip.name == name)
    }

    /// `AnimationClip.CreateClipsFromMorphTargetSequences( morphTargets, fps, noLoop )`.
    ///
    /// Groups names by Three's `/^([\w-]*?)([\d]+)$/`: a trailing run of digits
    /// preceded only by word characters or hyphens. The regex is matched by
    /// hand (no `regex` dependency); the lazy prefix plus the end anchor means
    /// the digit group is the longest trailing digit run, which is what the
    /// split below computes.
    ///
    /// The returned clips keep first-appearance order, matching `for ( const
    /// name in animationToMorphTargets )` over a JS object.
    pub fn create_clips_from_morph_target_sequences(
        morph_targets: &[&str],
        fps: f64,
        no_loop: bool,
    ) -> Result<Vec<Self>, Error> {
        let mut animation_to_morph_targets: Vec<(String, Vec<&str>)> = Vec::new();

        for morph_target in morph_targets {
            let Some(name) = morph_target_group(morph_target) else {
                continue;
            };

            match animation_to_morph_targets
                .iter_mut()
                .find(|(key, _)| key == name)
            {
                Some((_, group)) => group.push(morph_target),
                None => animation_to_morph_targets.push((name.to_string(), vec![morph_target])),
            }
        }

        let mut clips = Vec::new();

        for (name, group) in &animation_to_morph_targets {
            clips.push(Self::create_from_morph_target_sequence(
                name, group, fps, no_loop,
            )?);
        }

        Ok(clips)
    }

    /// `resetDuration()` — sets the duration to that of the longest track.
    pub fn reset_duration(&mut self) -> &mut Self {
        let mut duration: f64 = 0.0;

        for track in &self.tracks {
            // `track.times[ track.times.length - 1 ]`; a track can never have
            // empty `times` (its constructor rejects that), but be defensive.
            if let Some(&last) = track.times.last() {
                duration = duration.max(last);
            }
        }

        self.duration = duration;

        self
    }

    /// `trim()` — trims all tracks to the clip's duration.
    pub fn trim(&mut self) -> &mut Self {
        let duration = self.duration;
        for track in &mut self.tracks {
            track.trim(0.0, duration);
        }

        self
    }

    /// `validate()`.
    pub fn validate(&self) -> bool {
        let mut valid = true;

        for track in &self.tracks {
            valid = valid && track.validate();
        }

        valid
    }

    /// `optimize()`.
    pub fn optimize(&mut self) -> &mut Self {
        for track in &mut self.tracks {
            track.optimize();
        }

        self
    }

    /// `clone()`. A fresh `uuid` is generated, as in Three (the constructor
    /// runs), and `userData` is deep-copied.
    pub fn clone_clip(&self) -> Self {
        let tracks: Vec<KeyframeTrack> =
            self.tracks.iter().map(KeyframeTrack::clone_track).collect();

        let mut clip = Self::new(&self.name, self.duration, tracks, self.blend_mode);

        clip.user_data = self.user_data.clone();

        clip
    }
}

/// The capture group `([\w-]*?)` of Three's morph-target name pattern, or
/// `None` when the name does not match `^([\w-]*?)([\d]+)$`.
fn morph_target_group(name: &str) -> Option<&str> {
    // `[\d]+` is ASCII digits in JS; `\w` is `[A-Za-z0-9_]`.
    let digits_start = name
        .char_indices()
        .rev()
        .take_while(|(_, c)| c.is_ascii_digit())
        .last()
        .map(|(i, _)| i)?;

    let prefix = &name[..digits_start];

    if prefix
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
    {
        Some(prefix)
    } else {
        None
    }
}
