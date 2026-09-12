//! Port of `three.js/src/animation/KeyframeTrack.js` and
//! `three.js/src/animation/tracks/*.js`.
//!
//! Three has one base class and six subclasses that differ only in three
//! prototype properties (`ValueTypeName`, `ValueBufferType`,
//! `DefaultInterpolation`) and in which `InterpolantFactoryMethod*` they
//! suppress. Rust collapses that into one struct plus a [`TrackValueType`]
//! discriminant, with the per-subclass facts as methods on the discriminant,
//! and constructors named after the subclasses
//! ([`KeyframeTrack::number`], [`KeyframeTrack::quaternion`], …).

use serde_json::{json, Map, Value};

use crate::animation::animation_utils;
use crate::math::interpolant::{Interpolant, InterpolantData};
use crate::math::interpolants::{
    cubic_interpolant, discrete_interpolant, linear_interpolant, quaternion_linear_interpolant,
    CubicInterpolation, DiscreteInterpolation, LinearInterpolation, QuaternionLinearInterpolation,
};

/// `constants.js`' interpolation modes.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum InterpolationMode {
    /// `InterpolateDiscrete` (2300).
    Discrete,
    /// `InterpolateLinear` (2301).
    Linear,
    /// `InterpolateSmooth` (2302).
    Smooth,
    /// `InterpolateBezier` (2303). Recognised so that a clip round-trips, but
    /// `BezierInterpolant` itself is not ported yet: `create_interpolant` on a
    /// Bezier track returns `None`.
    Bezier,
}

impl InterpolationMode {
    /// The number `constants.js` gives this mode, as it appears in clip JSON.
    pub fn as_number(self) -> u32 {
        match self {
            Self::Discrete => 2300,
            Self::Linear => 2301,
            Self::Smooth => 2302,
            Self::Bezier => 2303,
        }
    }

    /// The inverse of [`Self::as_number`]; anything else is `None`, which
    /// `setInterpolation` treats as unsupported.
    pub fn from_number(n: u32) -> Option<Self> {
        match n {
            2300 => Some(Self::Discrete),
            2301 => Some(Self::Linear),
            2302 => Some(Self::Smooth),
            2303 => Some(Self::Bezier),
            _ => None,
        }
    }
}

/// Which `KeyframeTrack` subclass a track is: its `ValueTypeName`.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum TrackValueType {
    /// `BooleanKeyframeTrack`, `'bool'`.
    Bool,
    /// `ColorKeyframeTrack`, `'color'`.
    Color,
    /// `NumberKeyframeTrack`, `'number'`.
    Number,
    /// `QuaternionKeyframeTrack`, `'quaternion'`.
    Quaternion,
    /// `StringKeyframeTrack`, `'string'`.
    String,
    /// `VectorKeyframeTrack`, `'vector'`.
    Vector,
}

impl TrackValueType {
    /// `ValueTypeName`.
    pub fn value_type_name(self) -> &'static str {
        match self {
            Self::Bool => "bool",
            Self::Color => "color",
            Self::Number => "number",
            Self::Quaternion => "quaternion",
            Self::String => "string",
            Self::Vector => "vector",
        }
    }

    /// `ValueTypeName` as `AnimationClip.parse` spells it, case-insensitively
    /// (`'bool'`/`'boolean'`, `'vector2'`/`'vector3'`/`'vector4'`, …).
    pub fn from_value_type_name(name: &str) -> Option<Self> {
        match name.to_ascii_lowercase().as_str() {
            "scalar" | "double" | "float" | "number" | "integer" => Some(Self::Number),
            "vector" | "vector2" | "vector3" | "vector4" => Some(Self::Vector),
            "color" => Some(Self::Color),
            "quaternion" => Some(Self::Quaternion),
            "bool" | "boolean" => Some(Self::Bool),
            "string" => Some(Self::String),
            _ => None,
        }
    }

    /// `DefaultInterpolation`.
    pub fn default_interpolation(self) -> InterpolationMode {
        match self {
            // `BooleanKeyframeTrack` / `StringKeyframeTrack`
            Self::Bool | Self::String => InterpolationMode::Discrete,
            _ => InterpolationMode::Linear,
        }
    }

    /// Whether this subclass suppresses the factory method for `interpolation`
    /// (`InterpolantFactoryMethodLinear = undefined` and friends).
    pub fn supports(self, interpolation: InterpolationMode) -> bool {
        match interpolation {
            InterpolationMode::Discrete => true,
            InterpolationMode::Linear => !matches!(self, Self::Bool | Self::String),
            InterpolationMode::Smooth | InterpolationMode::Bezier => {
                !matches!(self, Self::Bool | Self::String | Self::Quaternion)
            }
        }
    }
}

/// The interpolant a track's `createInterpolant` produced: one variant per
/// `InterpolantFactoryMethod*`, since `Interpolant<I>` is generic over its
/// strategy.
pub enum TrackInterpolant {
    /// `InterpolantFactoryMethodDiscrete`.
    Discrete(Interpolant<DiscreteInterpolation>),
    /// `InterpolantFactoryMethodLinear`.
    Linear(Interpolant<LinearInterpolation>),
    /// `InterpolantFactoryMethodSmooth`.
    Smooth(Interpolant<CubicInterpolation>),
    /// `QuaternionKeyframeTrack`'s `InterpolantFactoryMethodLinear`.
    QuaternionLinear(Interpolant<QuaternionLinearInterpolation>),
}

impl TrackInterpolant {
    /// `interpolant.evaluate( t )`.
    pub fn evaluate(&mut self, t: f64) -> &[f64] {
        match self {
            Self::Discrete(i) => i.evaluate(t),
            Self::Linear(i) => i.evaluate(t),
            Self::Smooth(i) => i.evaluate(t),
            Self::QuaternionLinear(i) => i.evaluate(t),
        }
    }

    /// The interpolant's `InterpolantData`, where `settings` and the result
    /// buffer live.
    pub fn data(&mut self) -> &mut InterpolantData {
        match self {
            Self::Discrete(i) => &mut i.data,
            Self::Linear(i) => &mut i.data,
            Self::Smooth(i) => &mut i.data,
            Self::QuaternionLinear(i) => &mut i.data,
        }
    }

    /// `interpolant.resultBuffer`.
    pub fn result_buffer(&self) -> &[f64] {
        match self {
            Self::Discrete(i) => i.result_buffer(),
            Self::Linear(i) => i.result_buffer(),
            Self::Smooth(i) => i.result_buffer(),
            Self::QuaternionLinear(i) => i.result_buffer(),
        }
    }
}

/// `KeyframeTrack`.
#[derive(Clone, Debug, PartialEq)]
pub struct KeyframeTrack {
    /// The track name, as `PropertyBinding` parses it.
    pub name: String,
    /// `times`.
    pub times: Vec<f64>,
    /// `values`. A `'bool'` track stores `0.0` / `1.0` here; a `'string'` track
    /// leaves it empty and keeps its values in [`Self::strings`].
    pub values: Vec<f64>,
    /// `StringKeyframeTrack`'s values — Three's `ValueBufferType = Array` case
    /// that carries no numbers at all.
    pub strings: Vec<String>,
    value_type: TrackValueType,
    interpolation: InterpolationMode,
}

impl KeyframeTrack {
    /// `new KeyframeTrack( name, times, values, interpolation )`.
    ///
    /// Three throws for a missing name or empty `times`; those become `Err`.
    /// `interpolation` of `None` is Three's `undefined`: `DefaultInterpolation`.
    pub fn new(
        value_type: TrackValueType,
        name: &str,
        times: Vec<f64>,
        values: Vec<f64>,
        interpolation: Option<InterpolationMode>,
    ) -> Result<Self, String> {
        if times.is_empty() {
            return Err(format!(
                "THREE.KeyframeTrack: no keyframes in track named {name}"
            ));
        }

        let mut track = Self {
            name: name.to_string(),
            times,
            values,
            strings: Vec::new(),
            value_type,
            interpolation: value_type.default_interpolation(),
        };

        track.set_interpolation(interpolation.unwrap_or_else(|| value_type.default_interpolation()))?;

        Ok(track)
    }

    /// `new BooleanKeyframeTrack( name, times, values )`.
    pub fn boolean(name: &str, times: Vec<f64>, values: &[bool]) -> Result<Self, String> {
        let values = values.iter().map(|&v| if v { 1.0 } else { 0.0 }).collect();
        Self::new(TrackValueType::Bool, name, times, values, None)
    }

    /// `new ColorKeyframeTrack( name, times, values, interpolation )`.
    pub fn color(
        name: &str,
        times: Vec<f64>,
        values: Vec<f64>,
        interpolation: Option<InterpolationMode>,
    ) -> Result<Self, String> {
        Self::new(TrackValueType::Color, name, times, values, interpolation)
    }

    /// `new NumberKeyframeTrack( name, times, values, interpolation )`.
    pub fn number(
        name: &str,
        times: Vec<f64>,
        values: Vec<f64>,
        interpolation: Option<InterpolationMode>,
    ) -> Result<Self, String> {
        Self::new(TrackValueType::Number, name, times, values, interpolation)
    }

    /// `new QuaternionKeyframeTrack( name, times, values, interpolation )`.
    pub fn quaternion(
        name: &str,
        times: Vec<f64>,
        values: Vec<f64>,
        interpolation: Option<InterpolationMode>,
    ) -> Result<Self, String> {
        Self::new(TrackValueType::Quaternion, name, times, values, interpolation)
    }

    /// `new StringKeyframeTrack( name, times, values )`.
    pub fn string(name: &str, times: Vec<f64>, values: Vec<String>) -> Result<Self, String> {
        let mut track = Self::new(TrackValueType::String, name, times, Vec::new(), None)?;
        track.strings = values;
        Ok(track)
    }

    /// `new VectorKeyframeTrack( name, times, values, interpolation )`.
    pub fn vector(
        name: &str,
        times: Vec<f64>,
        values: Vec<f64>,
        interpolation: Option<InterpolationMode>,
    ) -> Result<Self, String> {
        Self::new(TrackValueType::Vector, name, times, values, interpolation)
    }

    /// `ValueTypeName`, i.e. which subclass this is.
    pub fn value_type(&self) -> TrackValueType {
        self.value_type
    }

    /// `setInterpolation( interpolation )`.
    ///
    /// Three falls back to `DefaultInterpolation` when the subclass suppresses
    /// the requested factory method, and throws only if the default itself is
    /// unsupported.
    pub fn set_interpolation(&mut self, interpolation: InterpolationMode) -> Result<(), String> {
        if !self.value_type.supports(interpolation) {
            let message = format!(
                "unsupported interpolation for {} keyframe track named {}",
                self.value_type.value_type_name(),
                self.name
            );

            // fall back to default, unless the default itself is messed up
            let default = self.value_type.default_interpolation();
            if interpolation != default {
                self.set_interpolation(default)?;
            } else {
                return Err(message); // fatal, in this case
            }

            return Ok(());
        }

        self.interpolation = interpolation;

        Ok(())
    }

    /// `getInterpolation()`.
    pub fn get_interpolation(&self) -> InterpolationMode {
        self.interpolation
    }

    /// `createInterpolant( result )`, i.e. whichever
    /// `InterpolantFactoryMethod*` `setInterpolation` installed. `None` for a
    /// Bezier track (`BezierInterpolant` is not ported) and for a `'string'`
    /// track, which has no numeric samples to interpolate.
    pub fn create_interpolant(&self, result: Option<Vec<f64>>) -> Option<TrackInterpolant> {
        if self.value_type == TrackValueType::String {
            return None;
        }

        let times = self.times.clone();
        let values = self.values.clone();
        let size = self.get_value_size();

        Some(match self.interpolation {
            InterpolationMode::Discrete => {
                TrackInterpolant::Discrete(discrete_interpolant(times, values, size, result))
            }
            InterpolationMode::Linear => {
                if self.value_type == TrackValueType::Quaternion {
                    TrackInterpolant::QuaternionLinear(quaternion_linear_interpolant(
                        times, values, size, result,
                    ))
                } else {
                    TrackInterpolant::Linear(linear_interpolant(times, values, size, result))
                }
            }
            InterpolationMode::Smooth => {
                TrackInterpolant::Smooth(cubic_interpolant(times, values, size, result))
            }
            InterpolationMode::Bezier => return None,
        })
    }

    /// `getValueSize()`.
    pub fn get_value_size(&self) -> usize {
        if self.value_type == TrackValueType::String {
            return self.strings.len() / self.times.len();
        }
        self.values.len() / self.times.len()
    }

    /// `shift( timeOffset )`.
    pub fn shift(&mut self, time_offset: f64) -> &mut Self {
        if time_offset != 0.0 {
            for time in &mut self.times {
                *time += time_offset;
            }
        }

        self
    }

    /// `scale( timeScale )`. The `hasTangents` branch is not ported: Bezier
    /// tangents come with `BezierInterpolant`, which is out of scope here.
    pub fn scale(&mut self, time_scale: f64) -> &mut Self {
        if time_scale != 1.0 {
            for time in &mut self.times {
                *time *= time_scale;
            }
        }

        self
    }

    /// `trim( startTime, endTime )`.
    pub fn trim(&mut self, start_time: f64, end_time: f64) -> &mut Self {
        let n_keys = self.times.len();
        let mut from = 0usize;
        let mut to = n_keys as isize - 1;

        while from != n_keys && self.times[from] < start_time {
            from += 1;
        }

        while to != -1 && self.times[to as usize] > end_time {
            to -= 1;
        }

        to += 1; // inclusive -> exclusive bound
        let mut to = to as usize;

        if from != 0 || to != n_keys {
            // empty tracks are forbidden, so keep at least one keyframe
            if from >= to {
                to = to.max(1);
                from = to - 1;
            }

            let stride = self.get_value_size();
            self.times = self.times[from..to].to_vec();
            if self.value_type == TrackValueType::String {
                self.strings = self.strings[from * stride..to * stride].to_vec();
            } else {
                self.values = self.values[from * stride..to * stride].to_vec();
            }
        }

        self
    }

    /// `validate()`. Three logs to `console.error`; the Rust port returns the
    /// same boolean and writes nothing.
    pub fn validate(&self) -> bool {
        let mut valid = true;

        let value_size = if self.value_type == TrackValueType::String {
            self.strings.len() as f64 / self.times.len() as f64
        } else {
            self.values.len() as f64 / self.times.len() as f64
        };

        if value_size - value_size.floor() != 0.0 {
            // 'KeyframeTrack: Invalid value size in track.'
            valid = false;
        }

        let n_keys = self.times.len();

        if n_keys == 0 {
            // 'KeyframeTrack: Track is empty.'
            valid = false;
        }

        let mut prev_time: Option<f64> = None;

        for i in 0..n_keys {
            let curr_time = self.times[i];

            if curr_time.is_nan() {
                // 'KeyframeTrack: Time is not a valid number.'
                valid = false;
                break;
            }

            if prev_time.is_some_and(|prev_time| prev_time > curr_time) {
                // 'KeyframeTrack: Out of order keys.'
                valid = false;
                break;
            }

            prev_time = Some(curr_time);
        }

        for &value in &self.values {
            if value.is_nan() {
                // 'KeyframeTrack: Value is not a valid number.'
                valid = false;
                break;
            }
        }

        valid
    }

    /// `optimize()`.
    pub fn optimize(&mut self) -> &mut Self {
        // (0,0,0,0,1,1,1,0,0,0,0,0,0,0) --> (0,0,1,1,0,0)

        // times or values may be shared with other tracks, so overwriting is unsafe
        let mut times = self.times.clone();
        let mut values = self.values.clone();
        let stride = self.get_value_size();
        let smooth_interpolation = self.get_interpolation() == InterpolationMode::Smooth;
        let last_index = times.len() - 1;

        let mut write_index = 1usize;

        for i in 1..last_index {
            let mut keep = false;

            let time = times[i];
            let time_next = times[i + 1];

            // remove adjacent keyframes scheduled at the same time
            if time != time_next && (i != 1 || time != times[0]) {
                if !smooth_interpolation {
                    // remove unnecessary keyframes same as their neighbors
                    let offset = i * stride;
                    let offset_p = offset - stride;
                    let offset_n = offset + stride;

                    for j in 0..stride {
                        let value = values[offset + j];

                        if value != values[offset_p + j] || value != values[offset_n + j] {
                            keep = true;
                            break;
                        }
                    }
                } else {
                    keep = true;
                }
            }

            // in-place compaction
            if keep {
                if i != write_index {
                    times[write_index] = times[i];

                    let read_offset = i * stride;
                    let write_offset = write_index * stride;

                    for j in 0..stride {
                        values[write_offset + j] = values[read_offset + j];
                    }
                }

                write_index += 1;
            }
        }

        // flush last keyframe (compaction looks ahead)
        if last_index > 0 {
            times[write_index] = times[last_index];

            let read_offset = last_index * stride;
            let write_offset = write_index * stride;
            for j in 0..stride {
                values[write_offset + j] = values[read_offset + j];
            }

            write_index += 1;
        }

        if write_index != times.len() {
            self.times = times[0..write_index].to_vec();
            self.values = values[0..write_index * stride].to_vec();
        } else {
            self.times = times;
            self.values = values;
        }

        self
    }

    /// `clone()`. Three copies `createInterpolant` across rather than the
    /// `interpolation` argument; the mode is the same thing here.
    pub fn clone_track(&self) -> Self {
        self.clone()
    }

    /// `KeyframeTrack.toJSON( track )`.
    pub fn to_json(&self) -> Value {
        let mut json = Map::new();

        json.insert("name".into(), json!(self.name));
        json.insert(
            "times".into(),
            Value::Array(self.times.iter().map(|&t| json!(t)).collect()),
        );

        if self.value_type == TrackValueType::String {
            json.insert(
                "values".into(),
                Value::Array(self.strings.iter().map(|s| json!(s)).collect()),
            );
        } else if self.value_type == TrackValueType::Bool {
            json.insert(
                "values".into(),
                Value::Array(self.values.iter().map(|&v| json!(v != 0.0)).collect()),
            );
        } else {
            json.insert(
                "values".into(),
                Value::Array(self.values.iter().map(|&v| json!(v)).collect()),
            );
        }

        let interpolation = self.get_interpolation();

        if interpolation != self.value_type.default_interpolation() {
            json.insert("interpolation".into(), json!(interpolation.as_number()));
        }

        json.insert(
            "type".into(),
            json!(self.value_type.value_type_name()), // mandatory
        );

        Value::Object(json)
    }

    /// `AnimationClip.parseKeyframeTrack( json )` — the inverse of
    /// [`Self::to_json`], kept next to it. `flattenJSON` handles the `keys`
    /// form that older clip JSON uses.
    pub fn parse(json: &Value) -> Result<Self, String> {
        let type_name = json
            .get("type")
            .and_then(Value::as_str)
            .ok_or_else(|| "THREE.KeyframeTrack: Unsupported trackType: undefined".to_string())?;

        let value_type = TrackValueType::from_value_type_name(type_name)
            .ok_or_else(|| format!("THREE.KeyframeTrack: Unsupported trackType: {type_name}"))?;

        let name = json
            .get("name")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string();

        let mut times: Vec<f64> = Vec::new();
        let mut values: Vec<f64> = Vec::new();

        if json.get("times").is_none() || json.get("values").is_none() {
            // `AnimationClip.parseKeyframeTrack`: the `keys` form
            let keys = json
                .get("keys")
                .and_then(Value::as_array)
                .ok_or_else(|| format!("THREE.KeyframeTrack: no keyframes in track named {name}"))?;

            animation_utils::flatten_json(keys, &mut times, &mut values, "value");
        } else {
            times = json["times"]
                .as_array()
                .map(|a| a.iter().map(|v| v.as_f64().unwrap_or(f64::NAN)).collect())
                .unwrap_or_default();

            let raw = json["values"].as_array().cloned().unwrap_or_default();

            if value_type == TrackValueType::String {
                let strings: Vec<String> = raw
                    .iter()
                    .map(|v| v.as_str().unwrap_or_default().to_string())
                    .collect();
                let mut track = Self::new(value_type, &name, times, Vec::new(), None)?;
                track.strings = strings;
                if let Some(n) = json.get("interpolation").and_then(Value::as_u64) {
                    if let Some(mode) = InterpolationMode::from_number(n as u32) {
                        track.set_interpolation(mode)?;
                    }
                }
                return Ok(track);
            }

            values = raw
                .iter()
                .map(|v| match v {
                    Value::Bool(b) => {
                        if *b {
                            1.0
                        } else {
                            0.0
                        }
                    }
                    other => other.as_f64().unwrap_or(f64::NAN),
                })
                .collect();
        }

        let interpolation = json
            .get("interpolation")
            .and_then(Value::as_u64)
            .and_then(|n| InterpolationMode::from_number(n as u32));

        Self::new(value_type, &name, times, values, interpolation)
    }
}
