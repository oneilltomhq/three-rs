//! Port of `three.js/src/animation/AnimationMixer.js`.
//!
//! # Ownership shape
//!
//! Three's mixer and its actions point at each other: `mixer._actions` holds the
//! actions, every action holds `this._mixer` and calls back into it. That cycle
//! is modelled here as an **arena**:
//!
//! - The mixer owns `actions: Vec<Option<AnimationAction>>`. The slot index is a
//!   stable [`ActionHandle`], valid for the life of the mixer. Uncaching an
//!   action clears its `cache_index` (Three's `_cacheIndex = null`) but leaves
//!   the action in its slot, exactly as Three leaves the JS object alive while
//!   the caller still holds it — `play()` then rebinds it. The `Option` is there
//!   so `update` can lift one action out while holding the pools `&mut`.
//! - Three's `_actions` array — the `[ active | inactive ]` partition the
//!   `_lendAction` / `_takeBackAction` swap maintains — is `action_order:
//!   Vec<usize>` of handles plus `n_active_actions`. An action's
//!   `cache_index` is its index *into `action_order`*, exactly as Three's
//!   `_cacheIndex` is its index into `_actions`.
//! - The `PropertyMixer`s live in the same shape, in [`BindingPool`]
//!   (`_bindings`, `_nActiveBindings`, `_bindingsByRootAndName`), addressed by a
//!   stable slot index; an action's `_propertyBindings` is a `Vec<Option<usize>>`
//!   of those. Because `PropertyMixer` is owned by another module and has no
//!   `_cacheIndex` / `rootNode` / `path` fields, the pool keeps those alongside
//!   (`cache_index`, `key`).
//! - The weight / time-scale interpolants live in [`ControlPool`]
//!   (`_controlInterpolants`), addressed by [`ControlHandle`].
//!
//! Everything an action would call on its mixer therefore becomes a mixer method
//! taking an [`ActionHandle`]: `mixer.play( handle )` for `action.play()`, and so
//! on. `AnimationAction::getMixer` has no port — the mixer is the receiver. The
//! action's own methods that need one of the pools take it as an argument and
//! carry a trailing underscore; see [`crate::animation::animation_action`].
//!
//! # Roots
//!
//! There is no `Object3D` in this crate yet, so a "root" is a
//! [`TargetResolver`]: `PropertyBinding.create( root, trackName )` becomes
//! `resolver.resolve( &parse_track_name( trackName ) )`. Roots are registered
//! with the mixer and addressed by [`RootId`]; `RootId( 0 )` is the mixer's own
//! root (Three's `_root`), and [`AnimationMixer::add_root`] registers the extra
//! roots that Three passes as `optionalRoot` / `action._localRoot`. The root's
//! `uuid`, used as a cache key in Three, is the `RootId` itself.
//!
//! # Not ported
//!
//! - `AnimationObjectGroup`: deferred to the object-tree branch. It is a
//!   `TargetResolver` that fans out over its members; nothing in the mixer needs
//!   to change to accept one, so there is nothing to stub here.
//! - `EventDispatcher`: Three's mixer dispatches `'loop'` and `'finished'`.
//!   There is no `EventDispatcher` port in this crate, so the state changes
//!   happen and the notifications do not.
//! - `clipAction( 'name' )` / `existingAction( 'name' )`: the string form calls
//!   `AnimationClip.findByName( root, name )`, which reads `root.animations` off
//!   an `Object3D`. Deferred to the object-tree branch; pass the clip itself.
//! - `stats`: the getter bag over the three pool sizes. The counts are readable
//!   through [`AnimationMixer::stats`] instead.

use std::collections::HashMap;

use crate::animation::animation_action::{AnimationAction, LoopMode};
use crate::animation::animation_clip::{AnimationBlendMode, AnimationClip};
use crate::animation::binding_target::TargetResolver;
use crate::animation::property_binding::parse_track_name;
use crate::animation::property_mixer::PropertyMixer;
use crate::math::interpolant::InterpolantData;
use crate::math::interpolants::{linear_interpolant, LinearInterpolant};

/// A registered root object: `RootId( 0 )` is the mixer's own `_root`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct RootId(pub usize);

/// A handle to an [`AnimationAction`] owned by an [`AnimationMixer`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ActionHandle(pub usize);

/// A handle to one of the mixer's `_controlInterpolants`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ControlHandle(pub usize);

/// The `[ active | inactive ]` pool of `PropertyMixer`s: Three's `_bindings`,
/// `_nActiveBindings` and `_bindingsByRootAndName`.
#[derive(Default)]
pub struct BindingPool {
    /// Stable slots; `None` is a removed binding.
    slots: Vec<Option<PropertyMixer>>,
    /// Per-slot `_cacheIndex` (an index into `order`).
    cache_index: Vec<Option<usize>>,
    /// Per-slot `( rootUuid, trackName )`, Three's `binding.binding.rootNode.uuid`
    /// and `binding.binding.path`.
    key: Vec<Option<(RootId, String)>>,
    /// `_bindings`, as slot indices.
    order: Vec<usize>,
    /// `_nActiveBindings`.
    n_active: usize,
    /// `_bindingsByRootAndName`.
    by_root_and_name: HashMap<(RootId, String), usize>,
}

impl BindingPool {
    /// The `PropertyMixer` behind a handle.
    pub fn get(&self, handle: usize) -> Option<&PropertyMixer> {
        self.slots.get(handle).and_then(|s| s.as_ref())
    }

    /// The `PropertyMixer` behind a handle, mutably.
    pub fn get_mut(&mut self, handle: usize) -> Option<&mut PropertyMixer> {
        self.slots.get_mut(handle).and_then(|s| s.as_mut())
    }

    /// `_addInactiveBinding( binding, rootUuid, trackName )`.
    fn add_inactive_binding(&mut self, handle: usize, root: RootId, track_name: &str) {
        self.by_root_and_name
            .insert((root, track_name.to_string()), handle);
        self.key[handle] = Some((root, track_name.to_string()));

        self.cache_index[handle] = Some(self.order.len());
        self.order.push(handle);
    }

    /// `_removeInactiveBinding( binding )`.
    fn remove_inactive_binding(&mut self, handle: usize) {
        let Some(cache_index) = self.cache_index[handle] else {
            return;
        };

        let last = *self.order.last().expect("non-empty binding order");
        self.cache_index[last] = Some(cache_index);
        self.order[cache_index] = last;
        self.order.pop();
        self.cache_index[handle] = None;

        if let Some((root, track_name)) = self.key[handle].take() {
            self.by_root_and_name.remove(&(root, track_name));
        }

        // note: the `PropertyMixer` itself stays in its slot. Three drops its
        // last *cache* reference here and lets the garbage collector decide; an
        // action that still holds this binding keeps it alive, and
        // `_bindAction` re-registers it (`if ( binding._cacheIndex === null )`).
    }

    /// `_lendBinding( binding )`.
    fn lend_binding(&mut self, handle: usize) {
        let prev_index = self.cache_index[handle].expect("lending a cached binding");

        let last_active_index = self.n_active;
        self.n_active += 1;

        let first_inactive = self.order[last_active_index];

        self.cache_index[handle] = Some(last_active_index);
        self.order[last_active_index] = handle;

        self.cache_index[first_inactive] = Some(prev_index);
        self.order[prev_index] = first_inactive;
    }

    /// `_takeBackBinding( binding )`.
    fn take_back_binding(&mut self, handle: usize) {
        let prev_index = self.cache_index[handle].expect("taking back a cached binding");

        self.n_active -= 1;
        let first_inactive_index = self.n_active;

        let last_active = self.order[first_inactive_index];

        self.cache_index[handle] = Some(first_inactive_index);
        self.order[first_inactive_index] = handle;

        self.cache_index[last_active] = Some(prev_index);
        self.order[prev_index] = last_active;
    }
}

/// The pool of weight / time-scale interpolants: `_controlInterpolants`,
/// `_nActiveControlInterpolants`.
#[derive(Default)]
pub struct ControlPool {
    /// The interpolants themselves, by stable slot.
    slots: Vec<LinearInterpolant>,
    /// Per-slot `__cacheIndex`.
    cache_index: Vec<usize>,
    /// `_controlInterpolants`, as slot indices.
    order: Vec<usize>,
    /// `_nActiveControlInterpolants`.
    n_active: usize,
}

impl ControlPool {
    /// `_lendControlInterpolant()`.
    pub fn lend_control_interpolant(&mut self) -> ControlHandle {
        let last_active_index = self.n_active;
        self.n_active += 1;

        if last_active_index == self.order.len() {
            // new LinearInterpolant( new Float32Array( 2 ), new Float32Array( 2 ),
            //                        1, _controlInterpolantsResultBuffer )
            let slot = self.slots.len();
            self.slots.push(linear_interpolant(
                vec![0.0, 0.0],
                vec![0.0, 0.0],
                1,
                Some(vec![0.0]),
            ));
            self.cache_index.push(last_active_index);
            self.order.push(slot);
        }

        ControlHandle(self.order[last_active_index])
    }

    /// `_takeBackControlInterpolant( interpolant )`.
    pub fn take_back_control_interpolant(&mut self, handle: ControlHandle) {
        let prev_index = self.cache_index[handle.0];

        self.n_active -= 1;
        let first_inactive_index = self.n_active;

        let last_active = self.order[first_inactive_index];

        self.cache_index[handle.0] = first_inactive_index;
        self.order[first_inactive_index] = handle.0;

        self.cache_index[last_active] = prev_index;
        self.order[prev_index] = last_active;
    }

    /// `interpolant.parameterPositions` / `.sampleValues`.
    pub fn data_mut(&mut self, handle: ControlHandle) -> &mut InterpolantData {
        &mut self.slots[handle.0].data
    }

    /// `interpolant.evaluate( t )[ 0 ]`.
    pub fn evaluate(&mut self, handle: ControlHandle, t: f64) -> f64 {
        self.slots[handle.0].evaluate(t)[0]
    }

    /// `stats.controlInterpolants`.
    pub fn stats(&self) -> (usize, usize) {
        (self.order.len(), self.n_active)
    }
}

/// `_actionsByClip[ clipUuid ]`.
#[derive(Default)]
struct ActionsForClip {
    /// `knownActions` — used as prototypes.
    known_actions: Vec<ActionHandle>,
    /// `actionByRoot` — lookup.
    action_by_root: HashMap<RootId, ActionHandle>,
}

/// The three pool sizes Three exposes through `mixer.stats`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct MixerStats {
    /// `stats.actions.total` / `.inUse`.
    pub actions: (usize, usize),
    /// `stats.bindings.total` / `.inUse`.
    pub bindings: (usize, usize),
    /// `stats.controlInterpolants.total` / `.inUse`.
    pub control_interpolants: (usize, usize),
}

/// `AnimationMixer`.
///
/// Three extends `EventDispatcher`; that is not ported (see the module docs).
pub struct AnimationMixer {
    /// `_root` is `roots[ 0 ]`; the rest are the registered `optionalRoot`s.
    roots: Vec<Box<dyn TargetResolver>>,
    /// The action arena. `None` is an uncached action's hole.
    actions: Vec<Option<AnimationAction>>,
    /// `_actions` — the `[ active | inactive ]` partition, as handles.
    action_order: Vec<usize>,
    /// `_nActiveActions`.
    n_active_actions: usize,
    /// `_actionsByClip`.
    actions_by_clip: HashMap<String, ActionsForClip>,
    /// `_bindings` and friends.
    bindings: BindingPool,
    /// `_controlInterpolants` and friends.
    control: ControlPool,
    /// `_accuIndex`.
    accu_index: usize,
    /// `time` — the global mixer time.
    pub time: f64,
    /// `timeScale`.
    pub time_scale: f64,
}

impl AnimationMixer {
    /// `new AnimationMixer( root )`.
    pub fn new(root: Box<dyn TargetResolver>) -> Self {
        Self {
            roots: vec![root],
            actions: Vec::new(),
            action_order: Vec::new(),
            n_active_actions: 0,
            actions_by_clip: HashMap::new(),
            bindings: BindingPool::default(),
            control: ControlPool::default(),
            accu_index: 0,
            time: 0.0,
            time_scale: 1.0,
        }
    }

    /// Registers an extra root, for Three's `optionalRoot` / `_localRoot`.
    /// Not in Three, where roots are `Object3D`s passed by reference.
    pub fn add_root(&mut self, root: Box<dyn TargetResolver>) -> RootId {
        self.roots.push(root);
        RootId(self.roots.len() - 1)
    }

    /// `getRoot()` — `RootId( 0 )`, this mixer's own root.
    pub fn get_root(&self) -> RootId {
        RootId(0)
    }

    /// The resolver behind a [`RootId`].
    pub fn root(&self, root: RootId) -> &dyn TargetResolver {
        &*self.roots[root.0]
    }

    /// `mixer.stats`.
    pub fn stats(&self) -> MixerStats {
        MixerStats {
            actions: (self.action_order.len(), self.n_active_actions),
            bindings: (self.bindings.order.len(), self.bindings.n_active),
            control_interpolants: self.control.stats(),
        }
    }

    /// The [`BindingPool`], so callers can read a bound `PropertyMixer`.
    pub fn bindings(&self) -> &BindingPool {
        &self.bindings
    }

    /// An action by handle.
    pub fn action(&self, handle: ActionHandle) -> &AnimationAction {
        self.actions[handle.0]
            .as_ref()
            .expect("handle of a live action")
    }

    /// An action by handle, mutably — Three's public action fields (`weight`,
    /// `time`, `paused`, `enabled`, …) are plain properties.
    pub fn action_mut(&mut self, handle: ActionHandle) -> &mut AnimationAction {
        self.actions[handle.0]
            .as_mut()
            .expect("handle of a live action")
    }

    /// The root an action is bound to: `_localRoot || mixer._root`.
    fn action_root(&self, handle: ActionHandle) -> RootId {
        self.action(handle).local_root().unwrap_or(RootId(0))
    }

    // ---------------------------------------------------------------- binding

    /// `_bindAction( action, prototypeAction )`.
    ///
    /// The `prototypeAction` argument exists in Three only to copy the already
    /// parsed `parsedPath` off a sibling action's `PropertyBinding`; this port
    /// re-parses the track name (cheap, and `PropertyBinding` is not ported), so
    /// the prototype is not needed and is not taken.
    fn bind_action(&mut self, handle: ActionHandle) {
        let root = self.action_root(handle);

        let tracks = self.action(handle).get_clip().tracks.clone();

        for (i, track) in tracks.iter().enumerate() {
            let track_name = track.name.clone();

            if let Some(&existing) = self
                .bindings
                .by_root_and_name
                .get(&(root, track_name.clone()))
            {
                if let Some(binding) = self.bindings.get_mut(existing) {
                    binding.reference_count += 1;
                }
                self.action_mut(handle).property_bindings[i] = Some(existing);
                continue;
            }

            if let Some(own) = self.action(handle).property_bindings[i] {
                // existing binding, make sure the cache knows
                if self.bindings.cache_index[own].is_none() {
                    if let Some(binding) = self.bindings.get_mut(own) {
                        binding.reference_count += 1;
                    }
                    self.bindings.add_inactive_binding(own, root, &track_name);
                }
                continue;
            }

            // PropertyBinding.create( root, trackName ) — a track that does not
            // resolve is skipped. Three instead builds a binding that logs
            // 'THREE.PropertyBinding: Trying to update node for track …' on every
            // getValue; there is nowhere to log from here, so the track is simply
            // left unbound.
            let Ok(parsed) = parse_track_name(&track_name) else {
                continue;
            };
            let Some(target) = self.roots[root.0].resolve(&parsed) else {
                continue;
            };

            let binding = PropertyMixer::new(target, track.value_type(), track.get_value_size());

            let slot = self.bindings.slots.len();
            self.bindings.slots.push(Some(binding));
            self.bindings.cache_index.push(None);
            self.bindings.key.push(None);

            self.bindings.get_mut(slot).unwrap().reference_count += 1;
            self.bindings.add_inactive_binding(slot, root, &track_name);

            self.action_mut(handle).property_bindings[i] = Some(slot);

            // DIVERGENCE: Three also does
            // `interpolants[ i ].resultBuffer = binding.buffer` here; see the
            // `animation_action` module docs for what replaces the aliasing.
        }
    }

    /// `_activateAction( action )`.
    fn activate_action(&mut self, handle: ActionHandle) {
        if self.is_active_action(handle) {
            return;
        }

        if self.action(handle).cache_index.is_none() {
            // this action has been forgotten by the cache, but the user
            // appears to be still using it -> rebind

            let root = self.action_root(handle);
            let clip_uuid = self.action(handle).get_clip().uuid.clone();

            self.bind_action(handle);
            self.add_inactive_action(handle, &clip_uuid, root);
        }

        let bindings = self.action(handle).property_bindings.clone();

        // increment reference counts / sort out state
        for binding_handle in bindings.into_iter().flatten() {
            let use_count = match self.bindings.get_mut(binding_handle) {
                Some(binding) => {
                    let previous = binding.use_count;
                    binding.use_count += 1;
                    previous
                }
                None => continue,
            };

            if use_count == 0 {
                self.bindings.lend_binding(binding_handle);
                self.bindings
                    .get_mut(binding_handle)
                    .unwrap()
                    .save_original_state();
            }
        }

        self.lend_action(handle);
    }

    /// `_deactivateAction( action )`.
    fn deactivate_action(&mut self, handle: ActionHandle) {
        if !self.is_active_action(handle) {
            return;
        }

        let bindings = self.action(handle).property_bindings.clone();

        // decrement reference counts / sort out state
        for binding_handle in bindings.into_iter().flatten() {
            let use_count = match self.bindings.get_mut(binding_handle) {
                Some(binding) => {
                    binding.use_count -= 1;
                    binding.use_count
                }
                None => continue,
            };

            if use_count == 0 {
                self.bindings
                    .get_mut(binding_handle)
                    .unwrap()
                    .restore_original_state();
                self.bindings.take_back_binding(binding_handle);
            }
        }

        self.take_back_action(handle);
    }

    // --------------------------------------------------------- memory manager

    /// `_isActiveAction( action )`.
    fn is_active_action(&self, handle: ActionHandle) -> bool {
        match self.actions[handle.0].as_ref().and_then(|a| a.cache_index) {
            Some(index) => index < self.n_active_actions,
            None => false,
        }
    }

    /// `_addInactiveAction( action, clipUuid, rootUuid )`.
    fn add_inactive_action(&mut self, handle: ActionHandle, clip_uuid: &str, root: RootId) {
        let actions_for_clip = self
            .actions_by_clip
            .entry(clip_uuid.to_string())
            .or_default();

        let by_clip_cache_index = actions_for_clip.known_actions.len();
        actions_for_clip.known_actions.push(handle);
        actions_for_clip.action_by_root.insert(root, handle);

        let cache_index = self.action_order.len();
        self.action_order.push(handle.0);

        let action = self.action_mut(handle);
        action.by_clip_cache_index = Some(by_clip_cache_index);
        action.cache_index = Some(cache_index);
    }

    /// `_removeInactiveAction( action )`.
    fn remove_inactive_action(&mut self, handle: ActionHandle) {
        let Some(cache_index) = self.action(handle).cache_index else {
            return;
        };

        let last = *self.action_order.last().expect("non-empty action order");
        self.actions[last].as_mut().unwrap().cache_index = Some(cache_index);
        self.action_order[cache_index] = last;
        self.action_order.pop();
        self.action_mut(handle).cache_index = None;

        let clip_uuid = self.action(handle).get_clip().uuid.clone();
        let by_clip_cache_index = self.action(handle).by_clip_cache_index;
        let root = self.action_root(handle);

        let mut clip_now_empty = false;

        if let (Some(actions_for_clip), Some(by_clip_cache_index)) = (
            self.actions_by_clip.get_mut(&clip_uuid),
            by_clip_cache_index,
        ) {
            let known = &mut actions_for_clip.known_actions;
            let last_known = *known.last().expect("non-empty knownActions");
            known[by_clip_cache_index] = last_known;
            known.pop();

            if last_known != handle {
                self.actions[last_known.0]
                    .as_mut()
                    .unwrap()
                    .by_clip_cache_index = Some(by_clip_cache_index);
            }

            actions_for_clip.action_by_root.remove(&root);
            clip_now_empty = actions_for_clip.known_actions.is_empty();
        }

        self.action_mut(handle).by_clip_cache_index = None;

        if clip_now_empty {
            self.actions_by_clip.remove(&clip_uuid);
        }

        self.remove_inactive_bindings_for_action(handle);
    }

    /// `_removeInactiveBindingsForAction( action )`.
    fn remove_inactive_bindings_for_action(&mut self, handle: ActionHandle) {
        let bindings = self.action(handle).property_bindings.clone();

        for binding_handle in bindings.into_iter().flatten() {
            let reference_count = match self.bindings.get_mut(binding_handle) {
                Some(binding) => {
                    binding.reference_count -= 1;
                    binding.reference_count
                }
                None => continue,
            };

            if reference_count == 0 {
                self.bindings.remove_inactive_binding(binding_handle);
            }
        }
    }

    /// `_lendAction( action )`.
    fn lend_action(&mut self, handle: ActionHandle) {
        let prev_index = self.action(handle).cache_index.expect("cached action");

        let last_active_index = self.n_active_actions;
        self.n_active_actions += 1;

        let first_inactive = self.action_order[last_active_index];

        self.action_mut(handle).cache_index = Some(last_active_index);
        self.action_order[last_active_index] = handle.0;

        self.actions[first_inactive].as_mut().unwrap().cache_index = Some(prev_index);
        self.action_order[prev_index] = first_inactive;
    }

    /// `_takeBackAction( action )`.
    fn take_back_action(&mut self, handle: ActionHandle) {
        let prev_index = self.action(handle).cache_index.expect("cached action");

        self.n_active_actions -= 1;
        let first_inactive_index = self.n_active_actions;

        let last_active = self.action_order[first_inactive_index];

        self.action_mut(handle).cache_index = Some(first_inactive_index);
        self.action_order[first_inactive_index] = handle.0;

        self.actions[last_active].as_mut().unwrap().cache_index = Some(prev_index);
        self.action_order[prev_index] = last_active;
    }

    // ----------------------------------------------------------------- public

    /// `clipAction( clip, optionalRoot, blendMode )`.
    ///
    /// The string form (`clipAction( 'name' )`) is not ported: it needs
    /// `AnimationClip.findByName( root, name )` over `root.animations`.
    pub fn clip_action(
        &mut self,
        clip: &AnimationClip,
        optional_root: Option<RootId>,
        blend_mode: Option<AnimationBlendMode>,
    ) -> ActionHandle {
        let root = optional_root.unwrap_or(RootId(0));
        let clip_uuid = clip.uuid.clone();
        let blend_mode = blend_mode.unwrap_or(clip.blend_mode);

        if let Some(actions_for_clip) = self.actions_by_clip.get(&clip_uuid) {
            if let Some(&existing) = actions_for_clip.action_by_root.get(&root) {
                if self.action(existing).blend_mode == blend_mode {
                    return existing;
                }
            }

            // we know the clip, so we don't have to parse all the bindings
            // again but can just copy — see `bind_action` for why this port
            // does not need the prototype action.
        }

        // allocate all resources required to run it
        let action = AnimationAction::new(clip.clone(), optional_root, Some(blend_mode));

        let handle = ActionHandle(self.actions.len());
        self.actions.push(Some(action));

        self.bind_action(handle);

        // and make the action known to the memory manager
        self.add_inactive_action(handle, &clip_uuid, root);

        handle
    }

    /// `existingAction( clip, optionalRoot )`.
    pub fn existing_action(
        &self,
        clip: &AnimationClip,
        optional_root: Option<RootId>,
    ) -> Option<ActionHandle> {
        self.existing_action_by_uuid(&clip.uuid, optional_root)
    }

    /// `existingAction`, by clip uuid — the `clipUuid` Three keys
    /// `_actionsByClip` on.
    pub fn existing_action_by_uuid(
        &self,
        clip_uuid: &str,
        optional_root: Option<RootId>,
    ) -> Option<ActionHandle> {
        let root = optional_root.unwrap_or(RootId(0));

        self.actions_by_clip
            .get(clip_uuid)
            .and_then(|actions_for_clip| actions_for_clip.action_by_root.get(&root).copied())
    }

    /// `stopAllAction()`.
    pub fn stop_all_action(&mut self) -> &mut Self {
        for i in (0..self.n_active_actions).rev() {
            let handle = ActionHandle(self.action_order[i]);
            self.stop(handle);
        }

        self
    }

    /// `update( deltaTime )`.
    pub fn update(&mut self, delta_time: f64) -> &mut Self {
        let delta_time = delta_time * self.time_scale;

        self.time += delta_time;
        let time = self.time;
        let time_direction = if delta_time > 0.0 {
            1.0
        } else if delta_time < 0.0 {
            -1.0
        } else {
            0.0 // Math.sign( 0 ) / Math.sign( - 0 )
        };

        self.accu_index ^= 1;
        let accu_index = self.accu_index;

        // run active actions

        for i in 0..self.n_active_actions {
            let slot = self.action_order[i];

            // The action is lifted out of its slot for the call so that it can
            // be `&mut` at the same time as the two pools; `_update` never
            // touches the action arena.
            let mut action = self.actions[slot].take().expect("live action");
            action.update_(
                time,
                delta_time,
                time_direction,
                accu_index,
                &mut self.bindings,
                &mut self.control,
            );
            self.actions[slot] = Some(action);
        }

        // update scene graph

        for i in 0..self.bindings.n_active {
            let slot = self.bindings.order[i];
            if let Some(binding) = self.bindings.get_mut(slot) {
                binding.apply(accu_index);
            }
        }

        self
    }

    /// `setTime( time )`.
    pub fn set_time(&mut self, time: f64) -> &mut Self {
        self.time = 0.0; // Zero out time attribute for AnimationMixer object;
        for action in self.actions.iter_mut().flatten() {
            action.time = 0.0; // and for all associated AnimationAction objects.
        }

        self.update(time)
    }

    /// `uncacheClip( clip )`.
    pub fn uncache_clip(&mut self, clip: &AnimationClip) {
        let Some(actions_for_clip) = self.actions_by_clip.remove(&clip.uuid) else {
            return;
        };

        // note: just calling _removeInactiveAction would mess up the iteration
        // state and also require updating the state we can just throw away
        for handle in actions_for_clip.known_actions {
            self.deactivate_action(handle);

            let Some(cache_index) = self.action(handle).cache_index else {
                continue;
            };
            let last = *self.action_order.last().expect("non-empty action order");

            self.action_mut(handle).cache_index = None;
            self.action_mut(handle).by_clip_cache_index = None;

            if last != handle.0 {
                self.actions[last].as_mut().unwrap().cache_index = Some(cache_index);
            }
            self.action_order[cache_index] = last;
            self.action_order.pop();

            self.remove_inactive_bindings_for_action(handle);
        }
    }

    /// `uncacheRoot( root )`.
    pub fn uncache_root(&mut self, root: RootId) {
        let actions: Vec<ActionHandle> = self
            .actions_by_clip
            .values()
            .filter_map(|for_clip| for_clip.action_by_root.get(&root).copied())
            .collect();

        for handle in actions {
            self.deactivate_action(handle);
            self.remove_inactive_action(handle);
        }

        let bindings: Vec<usize> = self
            .bindings
            .by_root_and_name
            .iter()
            .filter(|((r, _), _)| *r == root)
            .map(|(_, &handle)| handle)
            .collect();

        for binding_handle in bindings {
            if let Some(binding) = self.bindings.get_mut(binding_handle) {
                binding.restore_original_state();
            }
            self.bindings.remove_inactive_binding(binding_handle);
        }
    }

    /// `uncacheAction( clip, optionalRoot )`.
    pub fn uncache_action(&mut self, clip: &AnimationClip, optional_root: Option<RootId>) {
        if let Some(handle) = self.existing_action(clip, optional_root) {
            self.deactivate_action(handle);
            self.remove_inactive_action(handle);
        }
    }

    // ------------------------------------------- the AnimationAction surface
    //
    // Three puts these on the action, where `this._mixer` is reachable. Here
    // the mixer owns the action, so they take its handle.

    /// `AnimationAction.play()`.
    pub fn play(&mut self, handle: ActionHandle) -> ActionHandle {
        self.activate_action(handle);
        handle
    }

    /// `AnimationAction.stop()`.
    pub fn stop(&mut self, handle: ActionHandle) -> ActionHandle {
        self.deactivate_action(handle);
        self.reset(handle)
    }

    /// `AnimationAction.reset()`.
    pub fn reset(&mut self, handle: ActionHandle) -> ActionHandle {
        let control = &mut self.control;
        self.actions[handle.0]
            .as_mut()
            .expect("live action")
            .reset_(control);
        handle
    }

    /// `AnimationAction.isRunning()`.
    pub fn is_running(&self, handle: ActionHandle) -> bool {
        self.action(handle).is_running_locally() && self.is_active_action(handle)
    }

    /// `AnimationAction.isScheduled()`.
    pub fn is_scheduled(&self, handle: ActionHandle) -> bool {
        self.is_active_action(handle)
    }

    /// `AnimationAction.startAt( time )`.
    pub fn start_at(&mut self, handle: ActionHandle, time: f64) -> ActionHandle {
        self.action_mut(handle).start_at_(time);
        handle
    }

    /// `AnimationAction.setLoop( mode, repetitions )`.
    pub fn set_loop(
        &mut self,
        handle: ActionHandle,
        mode: LoopMode,
        repetitions: f64,
    ) -> ActionHandle {
        self.action_mut(handle).set_loop_(mode, repetitions);
        handle
    }

    /// `AnimationAction.setEffectiveWeight( weight )`.
    pub fn set_effective_weight(&mut self, handle: ActionHandle, weight: f64) -> ActionHandle {
        let control = &mut self.control;
        self.actions[handle.0]
            .as_mut()
            .expect("live action")
            .set_effective_weight_(weight, control);
        handle
    }

    /// `AnimationAction.getEffectiveWeight()`.
    pub fn get_effective_weight(&self, handle: ActionHandle) -> f64 {
        self.action(handle).get_effective_weight()
    }

    /// `AnimationAction.setEffectiveTimeScale( timeScale )`.
    pub fn set_effective_time_scale(
        &mut self,
        handle: ActionHandle,
        time_scale: f64,
    ) -> ActionHandle {
        let control = &mut self.control;
        self.actions[handle.0]
            .as_mut()
            .expect("live action")
            .set_effective_time_scale_(time_scale, control);
        handle
    }

    /// `AnimationAction.getEffectiveTimeScale()`.
    pub fn get_effective_time_scale(&self, handle: ActionHandle) -> f64 {
        self.action(handle).get_effective_time_scale()
    }

    /// `AnimationAction.setDuration( duration )`.
    pub fn set_duration(&mut self, handle: ActionHandle, duration: f64) -> ActionHandle {
        let control = &mut self.control;
        self.actions[handle.0]
            .as_mut()
            .expect("live action")
            .set_duration_(duration, control);
        handle
    }

    /// `AnimationAction.syncWith( action )`.
    pub fn sync_with(&mut self, handle: ActionHandle, other: ActionHandle) -> ActionHandle {
        let other_time = self.action(other).time;
        let other_time_scale = self.action(other).time_scale;

        let control = &mut self.control;
        self.actions[handle.0]
            .as_mut()
            .expect("live action")
            .sync_with_(other_time, other_time_scale, control);
        handle
    }

    /// `AnimationAction.halt( duration )`.
    pub fn halt(&mut self, handle: ActionHandle, duration: f64) -> ActionHandle {
        let now = self.time;
        let control = &mut self.control;
        self.actions[handle.0]
            .as_mut()
            .expect("live action")
            .halt_(duration, now, control);
        handle
    }

    /// `AnimationAction.warp( startTimeScale, endTimeScale, duration )`.
    pub fn warp(
        &mut self,
        handle: ActionHandle,
        start_time_scale: f64,
        end_time_scale: f64,
        duration: f64,
    ) -> ActionHandle {
        let now = self.time;
        let control = &mut self.control;
        self.actions[handle.0].as_mut().expect("live action").warp_(
            start_time_scale,
            end_time_scale,
            duration,
            now,
            control,
        );
        handle
    }

    /// `AnimationAction.stopWarping()`.
    pub fn stop_warping(&mut self, handle: ActionHandle) -> ActionHandle {
        let control = &mut self.control;
        self.actions[handle.0]
            .as_mut()
            .expect("live action")
            .stop_warping_(control);
        handle
    }

    /// `AnimationAction.stopFading()`.
    pub fn stop_fading(&mut self, handle: ActionHandle) -> ActionHandle {
        let control = &mut self.control;
        self.actions[handle.0]
            .as_mut()
            .expect("live action")
            .stop_fading_(control);
        handle
    }

    /// `AnimationAction.fadeIn( duration )`.
    pub fn fade_in(&mut self, handle: ActionHandle, duration: f64) -> ActionHandle {
        let now = self.time;
        let control = &mut self.control;
        self.actions[handle.0]
            .as_mut()
            .expect("live action")
            .fade_in_(duration, now, control);
        handle
    }

    /// `AnimationAction.fadeOut( duration )`.
    pub fn fade_out(&mut self, handle: ActionHandle, duration: f64) -> ActionHandle {
        let now = self.time;
        let control = &mut self.control;
        self.actions[handle.0]
            .as_mut()
            .expect("live action")
            .fade_out_(duration, now, control);
        handle
    }

    /// `AnimationAction.crossFadeFrom( fadeOutAction, duration, warp )`.
    pub fn cross_fade_from(
        &mut self,
        handle: ActionHandle,
        fade_out_action: ActionHandle,
        duration: f64,
        warp: bool,
    ) -> ActionHandle {
        self.fade_out(fade_out_action, duration);
        self.fade_in(handle, duration);

        if warp {
            let fade_in_duration = self.action(handle).get_clip().duration;
            let fade_out_duration = self.action(fade_out_action).get_clip().duration;

            let start_end_ratio = fade_out_duration / fade_in_duration;
            let end_start_ratio = fade_in_duration / fade_out_duration;

            let out_time_scale = self.action(fade_out_action).time_scale;
            let in_time_scale = self.action(handle).time_scale;
            self.action_mut(fade_out_action).restore_time_scale = Some(out_time_scale);
            self.action_mut(handle).restore_time_scale = Some(in_time_scale);

            self.warp(fade_out_action, 1.0, start_end_ratio, duration);
            self.warp(handle, end_start_ratio, 1.0, duration);
        }

        handle
    }

    /// `AnimationAction.crossFadeTo( fadeInAction, duration, warp )`.
    pub fn cross_fade_to(
        &mut self,
        handle: ActionHandle,
        fade_in_action: ActionHandle,
        duration: f64,
        warp: bool,
    ) -> ActionHandle {
        self.cross_fade_from(fade_in_action, handle, duration, warp)
    }

    /// `AnimationAction.getClip()`.
    pub fn get_clip(&self, handle: ActionHandle) -> &AnimationClip {
        self.action(handle).get_clip()
    }

    /// `AnimationAction.getRoot()` — `_localRoot || mixer._root`.
    pub fn get_action_root(&self, handle: ActionHandle) -> RootId {
        self.action_root(handle)
    }
}
