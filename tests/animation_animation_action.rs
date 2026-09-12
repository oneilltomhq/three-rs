//! Port of `three.js/test/unit/src/animation/AnimationAction.tests.js`.
//!
//! Three's suite builds a `new Object3D()` root and reads the animated value
//! back off it (`root.rotation.x`). This crate has no `Object3D` yet, so the
//! root here is a [`StubRoot`]: a [`TargetResolver`] that resolves any track
//! name to a shared-buffer [`BindingTarget`], which the test reads back with
//! `root.value( "rotation[x]" )`.
//!
//! Every action/mixer-state assertion is ported verbatim. Because the mixer owns
//! the actions in an arena, `animationAction.play()` is `mixer.play( action )`
//! and `animationAction.paused = true` is `mixer.action_mut( action ).paused =
//! true`; the assertions themselves are unchanged.

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use three_rs::animation::animation_action::LoopMode;
use three_rs::animation::animation_clip::{AnimationBlendMode, AnimationClip};
use three_rs::animation::animation_mixer::{ActionHandle, AnimationMixer};
use three_rs::animation::binding_target::{BindingTarget, TargetResolver};
use three_rs::animation::keyframe_track::KeyframeTrack;
use three_rs::animation::property_binding::ParsedTrackName;

// ---------------------------------------------------------------- the stub root

/// One animated property, shared between the mixer's binding and the test.
#[derive(Clone)]
struct StubTarget {
    values: Rc<RefCell<Vec<f64>>>,
}

impl BindingTarget for StubTarget {
    fn get_value(&self, buffer: &mut [f64], offset: usize) {
        for (i, &v) in self.values.borrow().iter().enumerate() {
            buffer[offset + i] = v;
        }
    }

    fn set_value(&mut self, buffer: &[f64], offset: usize) {
        let mut values = self.values.borrow_mut();
        let n = values.len();
        values.copy_from_slice(&buffer[offset..offset + n]);
    }

    fn value_size(&self) -> usize {
        self.values.borrow().len()
    }
}

/// Stands in for `new Object3D()`: resolves every track name, inventing a slot
/// of the right size the first time it is asked (`1` for an indexed property
/// such as `.rotation[x]`, `3` otherwise — enough for the tracks this suite
/// uses).
#[derive(Clone, Default)]
struct StubRoot {
    slots: Rc<RefCell<HashMap<String, Rc<RefCell<Vec<f64>>>>>>,
}

fn key_of(parsed: &ParsedTrackName) -> String {
    match &parsed.property_index {
        Some(index) => format!("{}[{}]", parsed.property_name, index),
        None => parsed.property_name.clone(),
    }
}

impl StubRoot {
    fn new() -> Self {
        Self::default()
    }

    /// Gives a property a starting value, as `Object3D` does for `scale`.
    fn seed(&self, key: &str, values: Vec<f64>) {
        self.slots
            .borrow_mut()
            .insert(key.to_string(), Rc::new(RefCell::new(values)));
    }

    fn values(&self, key: &str) -> Vec<f64> {
        self.slots.borrow()[key].borrow().clone()
    }

    /// `root.rotation.x`.
    fn value(&self, key: &str) -> f64 {
        self.values(key)[0]
    }
}

impl TargetResolver for StubRoot {
    fn resolve(&mut self, parsed: &ParsedTrackName) -> Option<Box<dyn BindingTarget>> {
        let key = key_of(parsed);
        let size = if parsed.property_index.is_some() { 1 } else { 3 };

        let slot = self
            .slots
            .borrow_mut()
            .entry(key)
            .or_insert_with(|| Rc::new(RefCell::new(vec![0.0; size])))
            .clone();

        Some(Box::new(StubTarget { values: slot }))
    }
}

// -------------------------------------------------------------------- fixtures

struct Animation {
    root: StubRoot,
    mixer: AnimationMixer,
    clip: AnimationClip,
    animation_action: ActionHandle,
}

fn create_animation() -> Animation {
    let root = StubRoot::new();
    let mut mixer = AnimationMixer::new(Box::new(root.clone()));
    let track =
        KeyframeTrack::number(".rotation[x]", vec![0.0, 1000.0], vec![0.0, 360.0], None).unwrap();
    let clip = AnimationClip::new(
        "clip1",
        1000.0,
        vec![track],
        AnimationBlendMode::Normal,
    );

    let animation_action = mixer.clip_action(&clip, None, None);

    Animation {
        root,
        mixer,
        clip,
        animation_action,
    }
}

struct TwoAnimations {
    mixer: AnimationMixer,
    animation_action: ActionHandle,
    animation_action2: ActionHandle,
}

fn create_two_animations() -> TwoAnimations {
    let root = StubRoot::new();
    let mut mixer = AnimationMixer::new(Box::new(root));
    let track =
        KeyframeTrack::number(".rotation[x]", vec![0.0, 1000.0], vec![0.0, 360.0], None).unwrap();
    let clip = AnimationClip::new("clip1", 1000.0, vec![track.clone()], AnimationBlendMode::Normal);
    let animation_action = mixer.clip_action(&clip, None, None);

    // note: Three's fixture builds `track2` but (apparently by accident) puts
    // `track` in `clip2` as well; kept verbatim.
    let clip2 = AnimationClip::new("clip2", 1000.0, vec![track], AnimationBlendMode::Normal);
    let animation_action2 = mixer.clip_action(&clip2, None, None);

    TwoAnimations {
        mixer,
        animation_action,
        animation_action2,
    }
}

// INSTANCING
#[test]
fn instancing() {
    let root = StubRoot::new();
    let mut mixer = AnimationMixer::new(Box::new(root));
    let clip = AnimationClip::new("nonname", -1.0, vec![], AnimationBlendMode::Normal);

    // `new AnimationAction( mixer, clip )` is `mixer.clip_action( &clip )` here:
    // the mixer owns the arena the action lives in.
    let animation_action = mixer.clip_action(&clip, None, None);
    assert_eq!(
        mixer.action(animation_action).time,
        0.0,
        "animationAction instantiated"
    );
}

// PUBLIC STUFF
#[test]
fn play() {
    let Animation {
        mut mixer,
        animation_action,
        ..
    } = create_animation();

    let animation_action2 = mixer.play(animation_action);
    assert_eq!(
        animation_action, animation_action2,
        "AnimationAction.play can be chained."
    );

    // SKIPPED: the `UserException` half of this test monkey-patches
    // `mixer._activateAction` to prove `play()` delegates to the mixer. There is
    // no method to replace here — `play` *is* the mixer method — so the
    // delegation is asserted through its effect instead.
    assert!(
        mixer.is_scheduled(animation_action),
        "AnimationMixer must activate AnimationAction on play."
    );
}

#[test]
fn stop() {
    let Animation {
        mut mixer,
        animation_action,
        ..
    } = create_animation();

    let animation_action2 = mixer.stop(animation_action);
    assert_eq!(
        animation_action, animation_action2,
        "AnimationAction.stop can be chained."
    );

    // SKIPPED: as in `play`, the monkey-patched `_deactivateAction` half.
    mixer.play(animation_action);
    mixer.stop(animation_action);
    assert!(
        !mixer.is_scheduled(animation_action),
        "AnimationMixer must deactivate AnimationAction on stop."
    );
}

#[test]
fn reset() {
    let Animation {
        mut mixer,
        animation_action,
        ..
    } = create_animation();

    let animation_action2 = mixer.stop(animation_action);
    assert_eq!(
        animation_action, animation_action2,
        "AnimationAction.reset can be chained."
    );
    let action = mixer.action(animation_action2);
    assert!(!action.paused, "AnimationAction.reset() sets paused false");
    assert!(action.enabled, "AnimationAction.reset() sets enabled true");
    assert_eq!(action.time, 0.0, "AnimationAction.reset() resets time.");
    assert_eq!(
        action.loop_count, -1,
        "AnimationAction.reset() resets loopcount."
    );
    assert_eq!(
        action.start_time, None,
        "AnimationAction.reset() removes starttime."
    );
}

#[test]
fn is_running() {
    let Animation {
        mut mixer,
        animation_action,
        ..
    } = create_animation();

    assert!(
        !mixer.is_running(animation_action),
        "When an animation is just made, it is not running."
    );
    mixer.play(animation_action);
    assert!(
        mixer.is_running(animation_action),
        "When an animation is started, it is running."
    );
    mixer.stop(animation_action);
    assert!(
        !mixer.is_running(animation_action),
        "When an animation is stopped, it is not running."
    );
    mixer.play(animation_action);
    mixer.action_mut(animation_action).paused = true;
    assert!(
        !mixer.is_running(animation_action),
        "When an animation is paused, it is not running."
    );
    mixer.action_mut(animation_action).paused = false;
    mixer.action_mut(animation_action).enabled = false;
    assert!(
        !mixer.is_running(animation_action),
        "When an animation is not enabled, it is not running."
    );
    mixer.action_mut(animation_action).enabled = true;
    assert!(
        mixer.is_running(animation_action),
        "When an animation is enabled, it is running."
    );
}

#[test]
fn is_scheduled() {
    let Animation {
        mut mixer,
        animation_action,
        ..
    } = create_animation();

    assert!(
        !mixer.is_scheduled(animation_action),
        "When an animation is just made, it is not scheduled."
    );
    mixer.play(animation_action);
    assert!(
        mixer.is_scheduled(animation_action),
        "When an animation is started, it is scheduled."
    );
    mixer.update(1.0);
    assert!(
        mixer.is_scheduled(animation_action),
        "When an animation is updated, it is scheduled."
    );
    mixer.stop(animation_action);
    assert!(
        !mixer.is_scheduled(animation_action),
        "When an animation is stopped, it isn't scheduled anymore."
    );
}

#[test]
fn start_at() {
    let Animation {
        mut mixer,
        animation_action,
        ..
    } = create_animation();

    mixer.start_at(animation_action, 2.0);
    mixer.play(animation_action);
    assert!(
        !mixer.is_running(animation_action),
        "When an animation is started at a specific time, it is not running."
    );
    assert!(
        mixer.is_scheduled(animation_action),
        "When an animation is started at a specific time, it is scheduled."
    );
    mixer.update(1.0);
    assert!(
        !mixer.is_running(animation_action),
        "When an animation is started at a specific time and the interval is not passed, it is not running."
    );
    assert!(
        mixer.is_scheduled(animation_action),
        "When an animation is started at a specific time and the interval is not passed, it is scheduled."
    );
    mixer.update(1.0);
    assert!(
        mixer.is_running(animation_action),
        "When an animation is started at a specific time and the interval is passed, it is running."
    );
    assert!(
        mixer.is_scheduled(animation_action),
        "When an animation is started at a specific time and the interval is passed, it is scheduled."
    );
    mixer.stop(animation_action);
    assert!(
        !mixer.is_running(animation_action),
        "When an animation is stopped, it is not running."
    );
    assert!(
        !mixer.is_scheduled(animation_action),
        "When an animation is stopped, it is not scheduled."
    );
}

#[test]
fn set_loop_loop_once() {
    let Animation {
        mut mixer,
        animation_action,
        ..
    } = create_animation();

    // Three calls `setLoop( LoopOnce )` with `repetitions` undefined; LoopOnce
    // never reads `repetitions`, so the default stands in for it.
    mixer.set_loop(animation_action, LoopMode::Once, f64::INFINITY);
    mixer.play(animation_action);
    assert!(
        mixer.is_running(animation_action),
        "When an animation is started, it is running."
    );
    mixer.update(500.0);
    assert!(
        mixer.is_running(animation_action),
        "When an animation is in the first loop, it is running."
    );
    mixer.update(500.0);
    assert!(
        !mixer.is_running(animation_action),
        "When an animation is ended, it is not running."
    );
    mixer.update(500.0);
    assert!(
        !mixer.is_running(animation_action),
        "When an animation is ended, it is not running."
    );
}

#[test]
fn set_loop_loop_repeat() {
    let Animation {
        root,
        mut mixer,
        animation_action,
        ..
    } = create_animation();

    mixer.set_loop(animation_action, LoopMode::Repeat, 3.0);
    mixer.play(animation_action);
    assert!(
        mixer.is_running(animation_action),
        "When an animation is started, it is running."
    );
    mixer.update(750.0);
    assert_eq!(
        root.value("rotation[x]"),
        270.0,
        "When an animation is 3/4 in the first loop, it has changed to 3/4 when LoopRepeat."
    );
    assert!(
        mixer.is_running(animation_action),
        "When an animation is in the first loop, it is running."
    );
    mixer.update(1000.0);
    assert_eq!(
        root.value("rotation[x]"),
        270.0,
        "When an animation is 3/4 in the second loop, it has changed to 3/4 when LoopRepeat."
    );
    assert!(
        mixer.is_running(animation_action),
        "When an animation is in second loop when in looprepeat 3 times, it is running."
    );
    mixer.update(1000.0);
    assert_eq!(
        root.value("rotation[x]"),
        270.0,
        "When an animation is 3/4 in the third loop, it has changed to 3/4 when LoopRepeat."
    );
    assert!(
        mixer.is_running(animation_action),
        "When an animation is in third loop when in looprepeat 3 times, it is running."
    );
    mixer.update(1000.0);
    assert_eq!(
        root.value("rotation[x]"),
        0.0,
        "When an animation ended his third loop when in looprepeat 3 times, it stays on the end result."
    );
    assert!(
        !mixer.is_running(animation_action),
        "When an animation ended his third loop when in looprepeat 3 times, it stays not running anymore."
    );
}

#[test]
fn set_loop_loop_ping_pong() {
    let Animation {
        root,
        mut mixer,
        animation_action,
        ..
    } = create_animation();

    mixer.set_loop(animation_action, LoopMode::PingPong, 3.0);
    mixer.play(animation_action);
    assert!(
        mixer.is_running(animation_action),
        "When an animation is started, it is running."
    );
    mixer.update(750.0);
    assert_eq!(
        root.value("rotation[x]"),
        270.0,
        "When an animation is 3/4 in the first loop, it has changed to 3/4 when LoopPingPong."
    );
    assert!(
        mixer.is_running(animation_action),
        "When an animation is in the first loop, it is running."
    );
    mixer.update(1000.0);
    assert_eq!(
        root.value("rotation[x]"),
        90.0,
        "When an animation is 3/4 in the second loop, it has changed to 1/4 when LoopPingPong."
    );
    assert!(
        mixer.is_running(animation_action),
        "When an animation is in second loop when in looprepeat 3 times, it is running."
    );
    mixer.update(1000.0);
    assert_eq!(
        root.value("rotation[x]"),
        270.0,
        "When an animation is 3/4 in the third loop, it has changed to 3/4 when LoopPingPong."
    );
    assert!(
        mixer.is_running(animation_action),
        "When an animation is in third loop when in looprepeat 3 times, it is running."
    );
    mixer.update(1000.0);
    assert_eq!(
        root.value("rotation[x]"),
        0.0,
        "When an animation ended his fourth loop when in looprepeat 3 times, it stays on the end result."
    );
    assert!(
        !mixer.is_running(animation_action),
        "When an animation ended his fourth loop when in looprepeat 3 times, it stays not running anymore."
    );
}

#[test]
fn set_effective_weight() {
    let Animation {
        mut mixer,
        animation_action,
        ..
    } = create_animation();

    assert_eq!(
        mixer.get_effective_weight(animation_action),
        1.0,
        "When an animation is created, EffectiveWeight is 1."
    );
    mixer.set_effective_weight(animation_action, 0.3);
    assert_eq!(
        mixer.get_effective_weight(animation_action),
        0.3,
        "When EffectiveWeight is set to 0.3 , EffectiveWeight is 0.3."
    );
}

#[test]
fn set_effective_weight_disabled() {
    let Animation {
        mut mixer,
        animation_action,
        ..
    } = create_animation();

    assert_eq!(
        mixer.get_effective_weight(animation_action),
        1.0,
        "When an animation is created, EffectiveWeight is 1."
    );
    mixer.action_mut(animation_action).enabled = false;
    mixer.set_effective_weight(animation_action, 0.3);
    assert_eq!(
        mixer.get_effective_weight(animation_action),
        0.0,
        "When EffectiveWeight is set to 0.3 when disabled , EffectiveWeight is 0."
    );
}

#[test]
fn set_effective_weight_over_duration() {
    let Animation {
        root,
        mut mixer,
        animation_action,
        ..
    } = create_animation();

    mixer.set_effective_weight(animation_action, 0.5);
    mixer.play(animation_action);
    mixer.update(500.0);
    assert_eq!(
        root.value("rotation[x]"),
        90.0,
        "When an animation has weight 0.5 and runs half through the animation, it has changed to 1/4."
    );
    mixer.update(1000.0);
    assert_eq!(
        root.value("rotation[x]"),
        90.0,
        "When an animation has weight 0.5 and runs one and half through the animation, it has changed to 1/4."
    );
}

#[test]
fn get_effective_weight() {
    let Animation {
        mut mixer,
        animation_action,
        ..
    } = create_animation();

    assert_eq!(
        mixer.get_effective_weight(animation_action),
        1.0,
        "When an animation is created, EffectiveWeight is 1."
    );
    mixer.set_effective_weight(animation_action, 0.3);
    assert_eq!(
        mixer.get_effective_weight(animation_action),
        0.3,
        "When EffectiveWeight is set to 0.3 , EffectiveWeight is 0.3."
    );

    let Animation {
        mut mixer,
        animation_action,
        ..
    } = create_animation();

    assert_eq!(
        mixer.get_effective_weight(animation_action),
        1.0,
        "When an animation is created, EffectiveWeight is 1."
    );
    mixer.action_mut(animation_action).enabled = false;
    mixer.set_effective_weight(animation_action, 0.3);
    assert_eq!(
        mixer.get_effective_weight(animation_action),
        0.0,
        "When EffectiveWeight is set to 0.3 when disabled , EffectiveWeight is 0."
    );
}

#[test]
fn fade_in() {
    let Animation {
        mut mixer,
        animation_action,
        ..
    } = create_animation();

    mixer.fade_in(animation_action, 1000.0);
    mixer.play(animation_action);
    assert_eq!(
        mixer.get_effective_weight(animation_action),
        1.0,
        "When an animation fadeIn is started, EffectiveWeight is 1."
    );
    mixer.update(250.0);
    assert_eq!(
        mixer.get_effective_weight(animation_action),
        0.25,
        "When an animation fadeIn happened 1/4, EffectiveWeight is 0.25."
    );
    mixer.update(250.0);
    assert_eq!(
        mixer.get_effective_weight(animation_action),
        0.5,
        "When an animation fadeIn is halfway , EffectiveWeight is 0.5."
    );
    mixer.update(250.0);
    assert_eq!(
        mixer.get_effective_weight(animation_action),
        0.75,
        "When an animation fadeIn is halfway , EffectiveWeight is 0.75."
    );
    mixer.update(500.0);
    assert_eq!(
        mixer.get_effective_weight(animation_action),
        1.0,
        "When an animation fadeIn is ended , EffectiveWeight is 1."
    );
}

#[test]
fn fade_out() {
    let Animation {
        mut mixer,
        animation_action,
        ..
    } = create_animation();

    mixer.fade_out(animation_action, 1000.0);
    mixer.play(animation_action);
    assert_eq!(
        mixer.get_effective_weight(animation_action),
        1.0,
        "When an animation fadeOut is started, EffectiveWeight is 1."
    );
    mixer.update(250.0);
    assert_eq!(
        mixer.get_effective_weight(animation_action),
        0.75,
        "When an animation fadeOut happened 1/4, EffectiveWeight is 0.75."
    );
    mixer.update(250.0);
    assert_eq!(
        mixer.get_effective_weight(animation_action),
        0.5,
        "When an animation fadeOut is halfway , EffectiveWeight is 0.5."
    );
    mixer.update(250.0);
    assert_eq!(
        mixer.get_effective_weight(animation_action),
        0.25,
        "When an animation fadeOut is happened 3/4 , EffectiveWeight is 0.25."
    );
    mixer.update(500.0);
    assert_eq!(
        mixer.get_effective_weight(animation_action),
        0.0,
        "When an animation fadeOut is ended , EffectiveWeight is 0."
    );
}

#[test]
fn cross_fade_from() {
    let TwoAnimations {
        mut mixer,
        animation_action,
        animation_action2,
    } = create_two_animations();

    mixer.cross_fade_from(animation_action, animation_action2, 1000.0, false);
    mixer.play(animation_action);
    mixer.play(animation_action2);
    assert_eq!(
        mixer.get_effective_weight(animation_action),
        1.0,
        "When an animation crossFadeFrom is started, EffectiveWeight is 1."
    );
    assert_eq!(
        mixer.get_effective_weight(animation_action2),
        1.0,
        "When an animation crossFadeFrom is started, EffectiveWeight is 1."
    );
    mixer.update(250.0);
    assert_eq!(
        mixer.get_effective_weight(animation_action),
        0.25,
        "When an animation fadeOut happened 1/4, EffectiveWeight is 0.75."
    );
    assert_eq!(
        mixer.get_effective_weight(animation_action2),
        0.75,
        "When an animation fadeOut happened 1/4, EffectiveWeight is 0.75."
    );
    mixer.update(250.0);
    assert_eq!(
        mixer.get_effective_weight(animation_action),
        0.5,
        "When an animation fadeOut happened 1/4, EffectiveWeight is 0.75."
    );
    assert_eq!(
        mixer.get_effective_weight(animation_action2),
        0.5,
        "When an animation fadeOut is halfway , EffectiveWeight is 0.5."
    );
    mixer.update(250.0);
    assert_eq!(
        mixer.get_effective_weight(animation_action),
        0.75,
        "When an animation fadeOut happened 1/4, EffectiveWeight is 0.75."
    );
    assert_eq!(
        mixer.get_effective_weight(animation_action2),
        0.25,
        "When an animation fadeOut is happened 3/4 , EffectiveWeight is 0.25."
    );
    mixer.update(500.0);
    assert_eq!(
        mixer.get_effective_weight(animation_action),
        1.0,
        "When an animation fadeOut happened 1/4, EffectiveWeight is 0.75."
    );
    assert_eq!(
        mixer.get_effective_weight(animation_action2),
        0.0,
        "When an animation fadeOut is ended , EffectiveWeight is 0."
    );
}

#[test]
fn cross_fade_to() {
    let TwoAnimations {
        mut mixer,
        animation_action,
        animation_action2,
    } = create_two_animations();

    mixer.cross_fade_to(animation_action2, animation_action, 1000.0, false);
    mixer.play(animation_action);
    mixer.play(animation_action2);
    assert_eq!(
        mixer.get_effective_weight(animation_action),
        1.0,
        "When an animation crossFadeFrom is started, EffectiveWeight is 1."
    );
    assert_eq!(
        mixer.get_effective_weight(animation_action2),
        1.0,
        "When an animation crossFadeFrom is started, EffectiveWeight is 1."
    );
    mixer.update(250.0);
    assert_eq!(
        mixer.get_effective_weight(animation_action),
        0.25,
        "When an animation fadeOut happened 1/4, EffectiveWeight is 0.75."
    );
    assert_eq!(
        mixer.get_effective_weight(animation_action2),
        0.75,
        "When an animation fadeOut happened 1/4, EffectiveWeight is 0.75."
    );
    mixer.update(250.0);
    assert_eq!(
        mixer.get_effective_weight(animation_action),
        0.5,
        "When an animation fadeOut happened 1/4, EffectiveWeight is 0.75."
    );
    assert_eq!(
        mixer.get_effective_weight(animation_action2),
        0.5,
        "When an animation fadeOut is halfway , EffectiveWeight is 0.5."
    );
    mixer.update(250.0);
    assert_eq!(
        mixer.get_effective_weight(animation_action),
        0.75,
        "When an animation fadeOut happened 1/4, EffectiveWeight is 0.75."
    );
    assert_eq!(
        mixer.get_effective_weight(animation_action2),
        0.25,
        "When an animation fadeOut is happened 3/4 , EffectiveWeight is 0.25."
    );
    mixer.update(500.0);
    assert_eq!(
        mixer.get_effective_weight(animation_action),
        1.0,
        "When an animation fadeOut happened 1/4, EffectiveWeight is 0.75."
    );
    assert_eq!(
        mixer.get_effective_weight(animation_action2),
        0.0,
        "When an animation fadeOut is ended , EffectiveWeight is 0."
    );
}

// SKIPPED: 'getMixer'. `getMixer()` has no port — the mixer owns the action
// arena, so the mixer *is* the receiver of every call that would have gone
// through `action._mixer`.

#[test]
fn get_clip() {
    let Animation {
        mixer,
        clip,
        animation_action,
        ..
    } = create_animation();

    let clip2 = mixer.get_clip(animation_action);
    assert_eq!(
        clip.uuid, clip2.uuid,
        "clip should be returned by getClip."
    );
}

#[test]
fn get_root() {
    let Animation {
        mixer,
        animation_action,
        ..
    } = create_animation();

    // Three compares the `Object3D` itself; roots here are `RootId`s, and
    // `RootId( 0 )` is the mixer's own root, which is the one this action uses.
    let root2 = mixer.get_action_root(animation_action);
    assert_eq!(
        mixer.get_root(),
        root2,
        "root should be returned by getRoot."
    );
}

// OTHERS

// SKIPPED: 'StartAt when already executed once'. The test drives the action from
// a `mixer.addEventListener( 'finished', … )` handler, and this port drops
// `EventDispatcher` (see the `animation_mixer` module docs).

#[test]
fn loop_repeat_with_time_scale_reversal_during_first_loop() {
    // Regression test for #19151
    let root = StubRoot::new();
    let mut mixer = AnimationMixer::new(Box::new(root.clone()));
    let track =
        KeyframeTrack::number(".rotation[x]", vec![0.0, 1000.0], vec![0.0, 360.0], None).unwrap();
    let clip = AnimationClip::new("clip1", 1000.0, vec![track], AnimationBlendMode::Normal);

    let animation_action = mixer.clip_action(&clip, None, None);
    mixer.set_loop(animation_action, LoopMode::Repeat, f64::INFINITY);
    mixer.play(animation_action);

    // Advance partway into the first loop
    mixer.update(500.0);
    assert_eq!(
        root.value("rotation[x]"),
        180.0,
        "At 500ms, rotation is 180 (halfway)."
    );

    // Reverse timeScale
    mixer.time_scale = -1.0;

    // Step backward — should smoothly reverse, not jump
    mixer.update(250.0);
    assert_eq!(
        root.value("rotation[x]"),
        90.0,
        "After reversing and stepping 250ms back, rotation is 90."
    );

    mixer.update(250.0);
    assert_eq!(
        root.value("rotation[x]"),
        0.0,
        "After reversing and stepping another 250ms back, rotation is 0 (start)."
    );
}

#[test]
fn loop_ping_pong_with_time_scale_reversal_during_first_loop() {
    // Regression test for #19151
    let root = StubRoot::new();
    let mut mixer = AnimationMixer::new(Box::new(root.clone()));
    let track =
        KeyframeTrack::number(".rotation[x]", vec![0.0, 1000.0], vec![0.0, 360.0], None).unwrap();
    let clip = AnimationClip::new("clip1", 1000.0, vec![track], AnimationBlendMode::Normal);

    let animation_action = mixer.clip_action(&clip, None, None);
    mixer.set_loop(animation_action, LoopMode::PingPong, f64::INFINITY);
    mixer.play(animation_action);

    // Advance partway into the first loop
    mixer.update(500.0);
    assert_eq!(
        root.value("rotation[x]"),
        180.0,
        "At 500ms, rotation is 180 (halfway)."
    );

    // Reverse timeScale
    mixer.time_scale = -1.0;

    // Step backward — should smoothly reverse, not jump
    mixer.update(250.0);
    assert_eq!(
        root.value("rotation[x]"),
        90.0,
        "After reversing and stepping 250ms back, rotation is 90."
    );

    mixer.update(250.0);
    assert_eq!(
        root.value("rotation[x]"),
        0.0,
        "After reversing and stepping another 250ms back, rotation is 0 (start)."
    );
}

// The `seed` helper is exercised by the mixer suite (`Object3D`'s `scale`
// starts at `( 1, 1, 1 )`); referenced here so this file's copy is not dead.
#[test]
fn stub_root_seed() {
    let root = StubRoot::new();
    root.seed("scale", vec![1.0, 1.0, 1.0]);
    assert_eq!(root.values("scale"), vec![1.0, 1.0, 1.0]);
}
