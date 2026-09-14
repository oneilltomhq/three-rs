//! Port of `three.js/test/unit/src/animation/AnimationMixer.tests.js`.
//!
//! The `new Object3D()` root becomes the same [`StubRoot`] resolver the
//! `AnimationAction` suite uses (duplicated here rather than shared, to keep
//! `tests/support/mod.rs` untouched): it resolves any track name to a
//! shared-buffer [`BindingTarget`] the test can read back. `Object3D.scale`
//! starts at `( 1, 1, 1 )`, so that slot is seeded.

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use three_rs::animation::animation_clip::{AnimationBlendMode, AnimationClip};
use three_rs::animation::animation_mixer::AnimationMixer;
use three_rs::animation::binding_target::{BindingTarget, TargetResolver};
use three_rs::animation::keyframe_track::KeyframeTrack;
use three_rs::animation::property_binding::ParsedTrackName;

mod support;
use support::{EPS, X, Y, Z};

// ---------------------------------------------------------------- the stub root

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

/// One track name's shared value buffer.
type SlotValues = Rc<RefCell<Vec<f64>>>;

#[derive(Clone, Default)]
struct StubRoot {
    slots: Rc<RefCell<HashMap<String, SlotValues>>>,
}

impl StubRoot {
    fn new() -> Self {
        Self::default()
    }

    /// Gives a property its `Object3D` starting value.
    fn seed(&self, key: &str, values: Vec<f64>) {
        self.slots
            .borrow_mut()
            .insert(key.to_string(), Rc::new(RefCell::new(values)));
    }

    fn values(&self, key: &str) -> Vec<f64> {
        self.slots.borrow()[key].borrow().clone()
    }
}

impl TargetResolver for StubRoot {
    fn resolve(&mut self, parsed: &ParsedTrackName) -> Option<Box<dyn BindingTarget>> {
        let key = match &parsed.property_index {
            Some(index) => format!("{}[{}]", parsed.property_name, index),
            None => parsed.property_name.clone(),
        };
        let size = if parsed.property_index.is_some() {
            1
        } else {
            3
        };

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

/// `getClips( pos1, pos2, scale1, scale2, dur )`.
fn get_clips(
    pos1: [f64; 3],
    pos2: [f64; 3],
    scale1: [f64; 3],
    scale2: [f64; 3],
    dur: f64,
) -> Vec<AnimationClip> {
    let mut clips = Vec::new();

    let track = KeyframeTrack::vector(
        ".scale",
        vec![0.0, dur],
        vec![
            scale1[0], scale1[1], scale1[2], scale2[0], scale2[1], scale2[2],
        ],
        None,
    )
    .unwrap();
    clips.push(AnimationClip::new(
        "scale",
        dur,
        vec![track],
        AnimationBlendMode::Normal,
    ));

    let track = KeyframeTrack::vector(
        ".position",
        vec![0.0, dur],
        vec![pos1[0], pos1[1], pos1[2], pos2[0], pos2[1], pos2[2]],
        None,
    )
    .unwrap();
    clips.push(AnimationClip::new(
        "position",
        dur,
        vec![track],
        AnimationBlendMode::Normal,
    ));

    clips
}

/// `math-constants.js` `zero3` / `one3` / `two3`.
const ZERO3: [f64; 3] = [0.0, 0.0, 0.0];
const ONE3: [f64; 3] = [1.0, 1.0, 1.0];
const TWO3: [f64; 3] = [2.0, 2.0, 2.0];

fn stub_object3d() -> StubRoot {
    let root = StubRoot::new();
    // `Object3D`'s own defaults for the two properties these clips animate.
    root.seed("position", vec![0.0, 0.0, 0.0]);
    root.seed("scale", vec![1.0, 1.0, 1.0]);
    root
}

// INHERITANCE

// SKIPPED: 'Extending' — `assert object instanceof EventDispatcher`. This port
// drops `EventDispatcher` (see the `animation_mixer` module docs), so there is no
// inheritance to assert.

// INSTANCING
#[test]
fn instancing() {
    let object = AnimationMixer::new(Box::new(StubRoot::new()));
    assert_eq!(object.time, 0.0, "Can instantiate a AnimationMixer.");
    assert_eq!(object.time_scale, 1.0, "Can instantiate a AnimationMixer.");
}

// PUBLIC

#[test]
fn stop_all_action() {
    let obj = stub_object3d();
    let mut anim_mixer = AnimationMixer::new(Box::new(obj.clone()));
    let clips = get_clips(ZERO3, ONE3, TWO3, ONE3, 1.0);
    let action_a = anim_mixer.clip_action(&clips[0], None, None);
    let action_b = anim_mixer.clip_action(&clips[1], None, None);

    anim_mixer.play(action_a);
    anim_mixer.play(action_b);
    anim_mixer.update(0.1);
    anim_mixer.stop_all_action();

    assert!(
        !anim_mixer.is_running(action_a) && !anim_mixer.is_running(action_b),
        "All actions stopped"
    );

    let position = obj.values("position");
    assert!(
        position[0] == 0.0 && position[1] == 0.0 && position[2] == 0.0,
        "Position reset as expected"
    );

    let scale = obj.values("scale");
    assert!(
        scale[0] == 1.0 && scale[1] == 1.0 && scale[2] == 1.0,
        "Scale reset as expected"
    );
}

#[test]
fn get_root() {
    let obj = stub_object3d();
    let anim_mixer = AnimationMixer::new(Box::new(obj));
    // Three compares the `Object3D` itself; roots are `RootId`s here and the
    // mixer's own root is `RootId( 0 )`.
    assert_eq!(
        anim_mixer.get_root(),
        three_rs::animation::animation_mixer::RootId(0),
        "Get original root object"
    );
}

// ADDED: not in Three's (very thin) mixer suite, but these are what rung 10
// needs from the mixer, and what the arena shape is most likely to get wrong.

/// `clipAction` is a cache: the same clip and root give the same action back.
#[test]
fn clip_action_caches_by_clip_and_root() {
    let obj = stub_object3d();
    let mut mixer = AnimationMixer::new(Box::new(obj));
    let clips = get_clips(ZERO3, ONE3, TWO3, ONE3, 1.0);

    let first = mixer.clip_action(&clips[0], None, None);
    let again = mixer.clip_action(&clips[0], None, None);
    assert_eq!(
        first, again,
        "clipAction returns the same action for a clip"
    );

    let other = mixer.clip_action(&clips[1], None, None);
    assert_ne!(first, other, "a different clip gets a different action");

    assert_eq!(
        mixer.existing_action(&clips[0], None),
        Some(first),
        "existingAction finds the cached action"
    );

    // One binding per distinct track name, shared between actions on the same
    // root; neither action is active yet.
    assert_eq!(mixer.stats().actions, (2, 0));
    assert_eq!(mixer.stats().bindings, (2, 0));
}

/// `existingAction` on an unknown clip is `null`.
#[test]
fn existing_action_unknown_clip() {
    let obj = stub_object3d();
    let mixer = AnimationMixer::new(Box::new(obj));
    let clips = get_clips(ZERO3, ONE3, TWO3, ONE3, 1.0);

    assert_eq!(
        mixer.existing_action(&clips[0], None),
        None,
        "existingAction is null for a clip the mixer has never seen"
    );
}

/// What rung 10 needs: a single `update( 0 )` must leave the bound properties at
/// their `t = 0` keyframe values.
#[test]
fn update_zero_applies_the_first_keyframe() {
    let obj = stub_object3d();
    let mut mixer = AnimationMixer::new(Box::new(obj.clone()));
    let clips = get_clips(ZERO3, ONE3, TWO3, ONE3, 1.0);

    let action_a = mixer.clip_action(&clips[0], None, None);
    let action_b = mixer.clip_action(&clips[1], None, None);
    mixer.play(action_a);
    mixer.play(action_b);

    mixer.update(0.0);

    // scale1 = two3, pos1 = zero3
    assert_eq!(obj.values("scale"), vec![TWO3[0], TWO3[1], TWO3[2]]);
    assert_eq!(obj.values("position"), vec![ZERO3[0], ZERO3[1], ZERO3[2]]);

    assert_eq!(mixer.stats().bindings, (2, 2), "both bindings are in use");
}

/// Playing halfway through a clip interpolates, and `uncacheAction` forgets the
/// action and its bindings.
#[test]
fn update_interpolates_and_uncache_action_releases_bindings() {
    let obj = stub_object3d();
    let mut mixer = AnimationMixer::new(Box::new(obj.clone()));
    let clips = get_clips(ZERO3, [X, Y, Z], TWO3, ONE3, 1.0);

    let action = mixer.clip_action(&clips[1], None, None);
    mixer.play(action);
    mixer.update(0.5);

    let position = obj.values("position");
    assert!(
        (position[0] - X / 2.0).abs() <= EPS,
        "position.x is halfway"
    );
    assert!(
        (position[1] - Y / 2.0).abs() <= EPS,
        "position.y is halfway"
    );
    assert!(
        (position[2] - Z / 2.0).abs() <= EPS,
        "position.z is halfway"
    );

    mixer.stop(action);
    mixer.uncache_action(&clips[1], None);

    assert_eq!(
        mixer.existing_action(&clips[1], None),
        None,
        "the action is gone from the cache"
    );
    assert_eq!(
        mixer.stats().bindings,
        (0, 0),
        "its binding was released with it"
    );
}

/// `uncacheClip` drops every action on a clip, active or not.
#[test]
fn uncache_clip() {
    let obj = stub_object3d();
    let mut mixer = AnimationMixer::new(Box::new(obj));
    let clips = get_clips(ZERO3, ONE3, TWO3, ONE3, 1.0);

    let action_a = mixer.clip_action(&clips[0], None, None);
    let _action_b = mixer.clip_action(&clips[1], None, None);
    mixer.play(action_a);

    mixer.uncache_clip(&clips[0]);

    assert_eq!(mixer.existing_action(&clips[0], None), None);
    assert!(mixer.existing_action(&clips[1], None).is_some());
    assert_eq!(
        mixer.stats().actions,
        (1, 0),
        "the uncached action left the pool and was deactivated"
    );
    assert_eq!(mixer.stats().bindings, (1, 0));
}

/// `uncacheRoot` drops the actions and bindings of one root, restoring the
/// bound properties first.
#[test]
fn uncache_root() {
    let obj = stub_object3d();
    let mut mixer = AnimationMixer::new(Box::new(obj.clone()));
    let clips = get_clips(ZERO3, ONE3, TWO3, ONE3, 1.0);

    let action = mixer.clip_action(&clips[0], None, None);
    mixer.play(action);
    mixer.update(0.5);

    mixer.uncache_root(mixer.get_root());

    assert_eq!(mixer.stats().actions, (0, 0));
    assert_eq!(mixer.stats().bindings, (0, 0));
    assert_eq!(
        obj.values("scale"),
        vec![1.0, 1.0, 1.0],
        "uncacheRoot restores the original state"
    );
}

/// A second root, Three's `optionalRoot`: same clip, separate action and
/// separate bindings.
#[test]
fn clip_action_with_optional_root() {
    let obj = stub_object3d();
    let other = stub_object3d();
    let mut mixer = AnimationMixer::new(Box::new(obj.clone()));
    let other_root = mixer.add_root(Box::new(other.clone()));

    let clips = get_clips(ZERO3, ONE3, TWO3, ONE3, 1.0);

    let action_main = mixer.clip_action(&clips[0], None, None);
    let action_other = mixer.clip_action(&clips[0], Some(other_root), None);

    assert_ne!(
        action_main, action_other,
        "a second root gets its own action for the same clip"
    );
    assert_eq!(mixer.get_action_root(action_other), other_root);
    assert_eq!(
        mixer.stats().bindings,
        (2, 0),
        "and its own binding for the same track name"
    );

    mixer.play(action_other);
    mixer.update(0.0);

    assert_eq!(
        other.values("scale"),
        vec![TWO3[0], TWO3[1], TWO3[2]],
        "the other root is the one that moved"
    );
    assert_eq!(
        obj.values("scale"),
        vec![1.0, 1.0, 1.0],
        "the mixer's own root did not"
    );
}

/// `setTime` rewinds everything and re-runs one step.
#[test]
fn set_time() {
    let obj = stub_object3d();
    let mut mixer = AnimationMixer::new(Box::new(obj.clone()));
    let clips = get_clips(ZERO3, ONE3, TWO3, ONE3, 1.0);

    let action = mixer.clip_action(&clips[1], None, None);
    mixer.play(action);
    mixer.update(0.75);

    mixer.set_time(0.25);

    assert_eq!(mixer.time, 0.25, "mixer time is exactly the time set");
    let position = obj.values("position");
    assert!(
        (position[0] - 0.25).abs() <= EPS,
        "and the bound value matches it, not the earlier update"
    );
}
