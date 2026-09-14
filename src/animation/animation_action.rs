//! Port of `three.js/src/animation/AnimationAction.js`.
//!
//! Ownership: Three's action holds `this._mixer` and calls back into it
//! (`_activateAction`, `_lendControlInterpolant`, `dispatchEvent`, …). Here the
//! mixer owns the actions in an arena (see
//! [`crate::animation::animation_mixer`]) and the action never names its mixer.
//! The two mixer-owned pools an action mutates —
//! [`BindingPool`](crate::animation::animation_mixer::BindingPool) (Three's
//! `_bindings`, holding the `PropertyMixer`s) and
//! [`ControlPool`](crate::animation::animation_mixer::ControlPool) (Three's
//! `_controlInterpolants`) — are passed in as arguments to the methods that
//! need them. Those methods carry a trailing underscore (`reset_`, `warp_`, …);
//! the un-suffixed names live on `AnimationMixer`, which takes an
//! `ActionHandle` and supplies the pools. `getMixer` therefore has no port: the
//! mixer is the thing you are calling.
//!
//! Divergences from Three, all forced by Rust:
//! - Three aliases `interpolant.resultBuffer = binding.buffer` so that
//!   `evaluate()` writes straight into the `PropertyMixer`'s `incoming` region.
//!   Two owners cannot share one buffer here, so `_update` evaluates into the
//!   interpolant's own result buffer and copies `valueSize` values into
//!   `buffer[ 0 .. valueSize ]` — bit-for-bit the same input to `accumulate`.
//! - Three shares one `_interpolantSettings` object between every interpolant
//!   of the action, so `_setEndings` mutates all of them at once. Here the
//!   settings live on the action and are pushed into each interpolant's
//!   `data.settings` right before it is evaluated.
//! - Events are dropped: Three's `_updateTime` dispatches `'loop'` and
//!   `'finished'` on the mixer. There is no `EventDispatcher` port in this
//!   crate, so the state changes happen (`paused` / `enabled` / clamped `time`)
//!   and the notification does not.

use crate::animation::animation_clip::{AnimationBlendMode, AnimationClip};
use crate::animation::animation_mixer::{BindingPool, ControlHandle, ControlPool, RootId};
use crate::animation::keyframe_track::TrackInterpolant;
use crate::math::interpolant::{Ending, InterpolantSettings};

/// `constants.js` loop modes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LoopMode {
    /// `LoopOnce` (2200).
    Once,
    /// `LoopRepeat` (2201).
    Repeat,
    /// `LoopPingPong` (2202).
    PingPong,
}

impl LoopMode {
    /// The numeric constant three.js uses.
    pub fn as_number(self) -> u32 {
        match self {
            Self::Once => 2200,
            Self::Repeat => 2201,
            Self::PingPong => 2202,
        }
    }

    /// The inverse of [`LoopMode::as_number`].
    pub fn from_number(n: u32) -> Option<Self> {
        match n {
            2200 => Some(Self::Once),
            2201 => Some(Self::Repeat),
            2202 => Some(Self::PingPong),
            _ => None,
        }
    }
}

/// `AnimationAction`.
pub struct AnimationAction {
    /// `_clip`. Three keeps a reference; the arena keeps a clone.
    pub(crate) clip: AnimationClip,
    /// `_localRoot` — `None` means "the mixer's own root".
    pub(crate) local_root: Option<RootId>,
    /// `blendMode`.
    pub blend_mode: AnimationBlendMode,

    /// `_interpolants`. `None` for a track whose `createInterpolant` returns
    /// `null` (a string track, or a Bezier track, which is not ported).
    pub(crate) interpolants: Vec<Option<TrackInterpolant>>,
    /// `_interpolantSettings`, shared by every interpolant in Three.
    pub(crate) interpolant_settings: InterpolantSettings,
    /// `_propertyBindings`, as handles into the mixer's [`BindingPool`].
    /// `None` for a track that did not resolve against the root.
    pub(crate) property_bindings: Vec<Option<usize>>,

    /// `_cacheIndex` — index into the mixer's action order, `None` when the
    /// memory manager has forgotten this action.
    pub(crate) cache_index: Option<usize>,
    /// `_byClipCacheIndex`.
    pub(crate) by_clip_cache_index: Option<usize>,

    /// `_timeScaleInterpolant`.
    pub(crate) time_scale_interpolant: Option<ControlHandle>,
    /// `_restoreTimeScale`.
    pub(crate) restore_time_scale: Option<f64>,
    /// `_weightInterpolant`.
    pub(crate) weight_interpolant: Option<ControlHandle>,

    /// `loop` (a Rust keyword), set via `setLoop`.
    pub loop_mode: LoopMode,
    /// `_loopCount`.
    pub loop_count: i32,

    /// `_startTime`: the global mixer time when the action is to be started.
    pub start_time: Option<f64>,

    /// `time` — the local time of this action, in seconds.
    pub time: f64,
    /// `timeScale`.
    pub time_scale: f64,
    /// `_effectiveTimeScale`.
    pub(crate) effective_time_scale: f64,
    /// `weight`.
    pub weight: f64,
    /// `_effectiveWeight`.
    pub(crate) effective_weight: f64,
    /// `repetitions` (`Infinity` by default).
    pub repetitions: f64,
    /// `paused`.
    pub paused: bool,
    /// `enabled`.
    pub enabled: bool,
    /// `clampWhenFinished`.
    pub clamp_when_finished: bool,
    /// `zeroSlopeAtStart`.
    pub zero_slope_at_start: bool,
    /// `zeroSlopeAtEnd`.
    pub zero_slope_at_end: bool,
}

impl AnimationAction {
    /// `new AnimationAction( mixer, clip, localRoot, blendMode )`. The mixer is
    /// implicit (it is the arena that will hold this action).
    pub fn new(
        clip: AnimationClip,
        local_root: Option<RootId>,
        blend_mode: Option<AnimationBlendMode>,
    ) -> Self {
        let blend_mode = blend_mode.unwrap_or(clip.blend_mode);

        let interpolants: Vec<Option<TrackInterpolant>> = clip
            .tracks
            .iter()
            .map(|track| track.create_interpolant(None))
            .collect();

        let n_tracks = interpolants.len();

        Self {
            clip,
            local_root,
            blend_mode,
            interpolants,
            interpolant_settings: InterpolantSettings {
                ending_start: Ending::ZeroCurvature,
                ending_end: Ending::ZeroCurvature,
            },
            property_bindings: vec![None; n_tracks],
            cache_index: None,
            by_clip_cache_index: None,
            time_scale_interpolant: None,
            restore_time_scale: None,
            weight_interpolant: None,
            loop_mode: LoopMode::Repeat,
            loop_count: -1,
            start_time: None,
            time: 0.0,
            time_scale: 1.0,
            effective_time_scale: 1.0,
            weight: 1.0,
            effective_weight: 1.0,
            repetitions: f64::INFINITY,
            paused: false,
            enabled: true,
            clamp_when_finished: false,
            zero_slope_at_start: true,
            zero_slope_at_end: true,
        }
    }

    /// `getClip()`.
    pub fn get_clip(&self) -> &AnimationClip {
        &self.clip
    }

    /// `_localRoot` — `getRoot()` is on the mixer, since `_localRoot ||
    /// mixer._root` needs the mixer.
    pub fn local_root(&self) -> Option<RootId> {
        self.local_root
    }

    /// `reset()`.
    pub fn reset_(&mut self, control: &mut ControlPool) -> &mut Self {
        self.paused = false;
        self.enabled = true;

        self.time = 0.0; // restart clip
        self.loop_count = -1; // forget previous loops
        self.start_time = None; // forget scheduling

        self.stop_fading_(control).stop_warping_(control)
    }

    /// The `enabled && ! paused && timeScale !== 0 && _startTime === null` half
    /// of `isRunning()`; the mixer ands in `_isActiveAction`.
    pub(crate) fn is_running_locally(&self) -> bool {
        self.enabled && !self.paused && self.time_scale != 0.0 && self.start_time.is_none()
    }

    /// `startAt( time )`.
    pub fn start_at_(&mut self, time: f64) -> &mut Self {
        self.start_time = Some(time);
        self
    }

    /// `setLoop( mode, repetitions )`.
    pub fn set_loop_(&mut self, mode: LoopMode, repetitions: f64) -> &mut Self {
        self.loop_mode = mode;
        self.repetitions = repetitions;
        self
    }

    /// `setEffectiveWeight( weight )`.
    pub fn set_effective_weight_(&mut self, weight: f64, control: &mut ControlPool) -> &mut Self {
        self.weight = weight;

        // note: same logic as when updated at runtime
        self.effective_weight = if self.enabled { weight } else { 0.0 };

        self.stop_fading_(control)
    }

    /// `getEffectiveWeight()`.
    pub fn get_effective_weight(&self) -> f64 {
        self.effective_weight
    }

    /// `fadeIn( duration )`.
    pub fn fade_in_(&mut self, duration: f64, now: f64, control: &mut ControlPool) -> &mut Self {
        self.schedule_fading_(duration, 0.0, 1.0, now, control)
    }

    /// `fadeOut( duration )`.
    pub fn fade_out_(&mut self, duration: f64, now: f64, control: &mut ControlPool) -> &mut Self {
        self.schedule_fading_(duration, 1.0, 0.0, now, control)
    }

    /// `stopFading()`.
    pub fn stop_fading_(&mut self, control: &mut ControlPool) -> &mut Self {
        if let Some(handle) = self.weight_interpolant.take() {
            control.take_back_control_interpolant(handle);
        }
        self
    }

    /// `setEffectiveTimeScale( timeScale )`.
    pub fn set_effective_time_scale_(
        &mut self,
        time_scale: f64,
        control: &mut ControlPool,
    ) -> &mut Self {
        self.time_scale = time_scale;
        self.effective_time_scale = if self.paused { 0.0 } else { time_scale };

        self.stop_warping_(control)
    }

    /// `getEffectiveTimeScale()`.
    pub fn get_effective_time_scale(&self) -> f64 {
        self.effective_time_scale
    }

    /// `setDuration( duration )`.
    pub fn set_duration_(&mut self, duration: f64, control: &mut ControlPool) -> &mut Self {
        self.time_scale = self.clip.duration / duration;
        self.stop_warping_(control)
    }

    /// `syncWith( action )`, with the other action's `time` / `timeScale` read
    /// out by the mixer (the arena cannot hand out two `&mut` actions).
    pub fn sync_with_(
        &mut self,
        other_time: f64,
        other_time_scale: f64,
        control: &mut ControlPool,
    ) -> &mut Self {
        self.time = other_time;
        self.time_scale = other_time_scale;
        self.stop_warping_(control)
    }

    /// `halt( duration )`.
    pub fn halt_(&mut self, duration: f64, now: f64, control: &mut ControlPool) -> &mut Self {
        let from = self.effective_time_scale;
        self.warp_(from, 0.0, duration, now, control)
    }

    /// `warp( startTimeScale, endTimeScale, duration )`.
    pub fn warp_(
        &mut self,
        start_time_scale: f64,
        end_time_scale: f64,
        duration: f64,
        now: f64,
        control: &mut ControlPool,
    ) -> &mut Self {
        let time_scale = self.time_scale;

        let handle = match self.time_scale_interpolant {
            Some(handle) => handle,
            None => {
                let handle = control.lend_control_interpolant();
                self.time_scale_interpolant = Some(handle);
                handle
            }
        };

        let data = control.data_mut(handle);
        data.parameter_positions[0] = now;
        data.parameter_positions[1] = now + duration;
        data.sample_values[0] = start_time_scale / time_scale;
        data.sample_values[1] = end_time_scale / time_scale;

        self
    }

    /// `stopWarping()`.
    pub fn stop_warping_(&mut self, control: &mut ControlPool) -> &mut Self {
        if let Some(handle) = self.time_scale_interpolant.take() {
            control.take_back_control_interpolant(handle);
        }

        self.restore_time_scale = None;

        self
    }

    // Internal

    /// `_update( time, deltaTime, timeDirection, accuIndex )`.
    pub(crate) fn update_(
        &mut self,
        time: f64,
        mut delta_time: f64,
        time_direction: f64,
        accu_index: usize,
        bindings: &mut BindingPool,
        control: &mut ControlPool,
    ) {
        // called by the mixer

        if !self.enabled {
            // call ._updateWeight() to update ._effectiveWeight

            self.update_weight_(time, control);
            return;
        }

        if let Some(start_time) = self.start_time {
            // check for scheduled start of action

            let time_running = (time - start_time) * time_direction;
            if time_running < 0.0 || time_direction == 0.0 {
                delta_time = 0.0;
            } else {
                self.start_time = None; // unschedule
                delta_time = time_direction * time_running;
            }
        }

        // apply time scale and advance time

        delta_time *= self.update_time_scale_(time, control);
        let clip_time = self.update_time_(delta_time);

        // note: _updateTime may disable the action resulting in
        // an effective weight of 0

        let weight = self.update_weight_(time, control);

        if weight > 0.0 {
            let blend_mode = self.blend_mode;

            for j in 0..self.interpolants.len() {
                let Some(binding_handle) = self.property_bindings[j] else {
                    continue;
                };
                let Some(interpolant) = self.interpolants[j].as_mut() else {
                    continue;
                };

                // Three's shared `_interpolantSettings` object, pushed in.
                interpolant.data().settings = Some(self.interpolant_settings);

                let values = interpolant.evaluate(clip_time);

                let Some(property_mixer) = bindings.get_mut(binding_handle) else {
                    continue;
                };

                // Stands in for `interpolant.resultBuffer = binding.buffer`.
                let n = property_mixer.value_size;
                property_mixer.buffer[..n].copy_from_slice(&values[..n]);

                match blend_mode {
                    AnimationBlendMode::Additive => {
                        property_mixer.accumulate_additive(weight);
                    }
                    AnimationBlendMode::Normal => {
                        property_mixer.accumulate(accu_index, weight);
                    }
                }
            }
        }
    }

    /// `_updateWeight( time )`.
    pub(crate) fn update_weight_(&mut self, time: f64, control: &mut ControlPool) -> f64 {
        let mut weight = 0.0;

        if self.enabled {
            weight = self.weight;

            if let Some(handle) = self.weight_interpolant {
                let interpolant_value = control.evaluate(handle, time);

                weight *= interpolant_value;

                if time > control.data_mut(handle).parameter_positions[1] {
                    self.stop_fading_(control);

                    if interpolant_value == 0.0 {
                        // faded out, disable
                        self.enabled = false;
                    }
                }
            }
        }

        self.effective_weight = weight;
        weight
    }

    /// `_updateTimeScale( time )`.
    pub(crate) fn update_time_scale_(&mut self, time: f64, control: &mut ControlPool) -> f64 {
        let mut time_scale = 0.0;

        if !self.paused {
            time_scale = self.time_scale;

            if let Some(handle) = self.time_scale_interpolant {
                let interpolant_value = control.evaluate(handle, time);

                time_scale *= interpolant_value;

                if time > control.data_mut(handle).parameter_positions[1] {
                    if time_scale == 0.0 {
                        // motion has halted, pause
                        self.paused = true;
                    } else {
                        if let Some(restore) = self.restore_time_scale {
                            time_scale = restore;
                        }

                        // warp done - apply final time scale
                        self.time_scale = time_scale;
                    }

                    self.stop_warping_(control);
                }
            }
        }

        self.effective_time_scale = time_scale;
        time_scale
    }

    /// `_updateTime( deltaTime )`.
    ///
    /// The `'loop'` / `'finished'` events are dropped (see the module docs);
    /// everything else is Three's control flow verbatim.
    pub(crate) fn update_time_(&mut self, delta_time: f64) -> f64 {
        let duration = self.clip.duration;
        let loop_mode = self.loop_mode;

        let mut time = self.time + delta_time;
        let mut loop_count = self.loop_count;

        let ping_pong = loop_mode == LoopMode::PingPong;

        if delta_time == 0.0 {
            if loop_count == -1 {
                return time;
            }

            return if ping_pong && (loop_count & 1) == 1 {
                duration - time
            } else {
                time
            };
        }

        if loop_mode == LoopMode::Once {
            if loop_count == -1 {
                // just started

                self.loop_count = 0;
                self.set_endings_(true, true, false);
            }

            // `handle_stop:` labelled block.
            loop {
                if time >= duration {
                    time = duration;
                } else if time < 0.0 {
                    time = 0.0;
                } else {
                    self.time = time;

                    break;
                }

                if self.clamp_when_finished {
                    self.paused = true;
                } else {
                    self.enabled = false;
                }

                self.time = time;

                // DROPPED: mixer.dispatchEvent( { type: 'finished', … } ).

                break;
            }
        } else {
            // repetitive Repeat or PingPong

            if loop_count == -1 {
                // just started

                if delta_time >= 0.0 {
                    loop_count = 0;

                    self.set_endings_(true, self.repetitions == 0.0, ping_pong);
                } else {
                    // when looping in reverse direction, the initial
                    // transition through zero counts as a repetition,
                    // so leave loopCount at -1

                    self.set_endings_(self.repetitions == 0.0, true, ping_pong);
                }
            }

            if time >= duration || time < 0.0 {
                // wrap around

                let loop_delta = (time / duration).floor(); // signed
                time -= duration * loop_delta;

                loop_count += loop_delta.abs() as i32;

                let pending = self.repetitions - loop_count as f64;

                if pending <= 0.0 {
                    // have to stop (switch state, clamp time, fire event)

                    if self.clamp_when_finished {
                        self.paused = true;
                    } else {
                        self.enabled = false;
                    }

                    time = if delta_time > 0.0 { duration } else { 0.0 };

                    self.time = time;

                    // DROPPED: mixer.dispatchEvent( { type: 'finished', … } ).
                } else {
                    // keep running

                    if pending == 1.0 {
                        // entering the last round

                        let at_start = delta_time < 0.0;
                        self.set_endings_(at_start, !at_start, ping_pong);
                    } else {
                        self.set_endings_(false, false, ping_pong);
                    }

                    self.loop_count = loop_count;

                    self.time = time;

                    // DROPPED: mixer.dispatchEvent( { type: 'loop', … } ).
                }
            } else {
                self.loop_count = loop_count;
                self.time = time;
            }

            if ping_pong && (loop_count & 1) == 1 {
                // invert time for the "pong round"

                return duration - time;
            }
        }

        time
    }

    /// `_setEndings( atStart, atEnd, pingPong )`.
    pub(crate) fn set_endings_(&mut self, at_start: bool, at_end: bool, ping_pong: bool) {
        let settings = &mut self.interpolant_settings;

        if ping_pong {
            settings.ending_start = Ending::ZeroSlope;
            settings.ending_end = Ending::ZeroSlope;
        } else {
            // assuming for LoopOnce atStart == atEnd == true

            settings.ending_start = if at_start {
                if self.zero_slope_at_start {
                    Ending::ZeroSlope
                } else {
                    Ending::ZeroCurvature
                }
            } else {
                Ending::WrapAround
            };

            settings.ending_end = if at_end {
                if self.zero_slope_at_end {
                    Ending::ZeroSlope
                } else {
                    Ending::ZeroCurvature
                }
            } else {
                Ending::WrapAround
            };
        }
    }

    /// `_scheduleFading( duration, weightNow, weightThen )`.
    pub(crate) fn schedule_fading_(
        &mut self,
        duration: f64,
        weight_now: f64,
        weight_then: f64,
        now: f64,
        control: &mut ControlPool,
    ) -> &mut Self {
        let handle = match self.weight_interpolant {
            Some(handle) => handle,
            None => {
                let handle = control.lend_control_interpolant();
                self.weight_interpolant = Some(handle);
                handle
            }
        };

        let data = control.data_mut(handle);
        data.parameter_positions[0] = now;
        data.sample_values[0] = weight_now;
        data.parameter_positions[1] = now + duration;
        data.sample_values[1] = weight_then;

        self
    }
}
