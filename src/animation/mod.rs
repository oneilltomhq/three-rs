//! Ports of `three.js/src/animation`.

pub mod animation_utils;
pub mod binding_target;
pub mod keyframe_track;
pub mod property_binding;

pub use binding_target::{BindingTarget, BufferTarget, TargetResolver};
pub use keyframe_track::{InterpolationMode, KeyframeTrack, TrackInterpolant, TrackValueType};
