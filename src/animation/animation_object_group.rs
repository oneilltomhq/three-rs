//! Port of `three.js/src/animation/AnimationObjectGroup.js`, plus the group side
//! of `PropertyBinding.Composite`.
//!
//! # Membership / ownership design
//!
//! Three identifies a member by `object.uuid` and, for each subscribed path,
//! keeps one `PropertyBinding( object, path, parsedPath )` per member. This crate
//! has no `Object3D` yet, so a member is anything that can (a) name itself and
//! (b) resolve a parsed track name into a [`BindingTarget`] — i.e. the
//! [`GroupMember`] trait, which is [`TargetResolver`] plus a `uuid()`. The group
//! is generic over that type, so `Object3D` becomes a member with a one-line
//! `impl GroupMember for Object3D` once it exists.
//!
//! Members are held as `Rc<RefCell<M>>` ([`MemberRef`]) because ownership here is
//! genuinely shared in the way Three's object references are: the caller keeps
//! the object, several groups may contain it, and the per-path binding rows hold
//! it too (Three's `PropertyBinding.rootNode`). `RefCell` because resolving is
//! `&mut self` on [`TargetResolver`].
//!
//! Two pieces of group state are read from the outside in Three — `nCachedObjects_`
//! (read by `Composite`) and the per-path bindings array handed out by
//! `subscribe_` and mutated in place by `add` / `remove` / `uncache`. Both are
//! therefore shared handles rather than plain fields: `Rc<Cell<usize>>` and
//! [`BindingsForPath`] = `Rc<RefCell<Vec<Option<MemberBinding<M>>>>>`. A
//! subscriber keeps its row handle and sees the group's index shuffling live,
//! exactly as in JS; `unsubscribe_` drops the group's own handle so the row stops
//! being updated while the subscriber's clone stays valid and frozen.
//!
//! The `Option` in a row is JS's `undefined` hole: Three does not create bindings
//! for objects sitting in the cached region, so `bindingsForPath[ i ]` is a hole
//! for `i < nCachedObjects_` until that member is re-activated.
//!
//! [`Composite`] is the fan-out [`BindingTarget`]: `getValue` reads the first
//! active member, `setValue` writes every active member, as
//! `PropertyBinding.Composite` does. [`AnimationObjectGroup`] itself implements
//! [`TargetResolver`], so it can be handed to `AnimationMixer` as a root and a
//! resolved path yields one `Composite` over the members.

use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::rc::Rc;

use crate::animation::binding_target::{BindingTarget, TargetResolver};
use crate::animation::property_binding::ParsedTrackName;

/// `MathUtils.generateUUID()`.
///
/// Duplicated from `animation_clip.rs` (it is private there, and that file is
/// owned by another port); `uuid` is only ever an opaque identity.
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
        state ^= state >> 12;
        state ^= state << 25;
        state ^= state >> 27;
        state.wrapping_mul(0x2545_F491_4F6C_DD1D)
    };

    let mut bytes = [0u8; 16];
    for chunk in bytes.chunks_mut(8) {
        let v = next().to_le_bytes();
        chunk.copy_from_slice(&v[..chunk.len()]);
    }
    bytes[6] = (bytes[6] & 0x0f) | 0x40;
    bytes[8] = (bytes[8] & 0x3f) | 0x80;

    let hex: Vec<String> = bytes.iter().map(|b| format!("{b:02x}")).collect();
    format!(
        "{}-{}-{}-{}-{}",
        hex[0..4].concat(),
        hex[4..6].concat(),
        hex[6..8].concat(),
        hex[8..10].concat(),
        hex[10..16].concat()
    )
}

/// A group member: Three's `Object3D` seen only through what
/// `AnimationObjectGroup` needs of it — an identity and a way to resolve a
/// parsed track name.
pub trait GroupMember: TargetResolver {
    /// `object.uuid`.
    fn uuid(&self) -> String;
}

/// How a member is held: shared, because the caller, several groups and the
/// per-path binding rows all refer to the same object.
pub type MemberRef<M> = Rc<RefCell<M>>;

/// One row of `_bindings`: the bindings for one subscribed path, one slot per
/// member, `None` where Three leaves `undefined` (members in the cached region).
///
/// Shared so that a subscriber sees the group's shuffling live.
pub type BindingsForPath<M> = Rc<RefCell<Vec<Option<MemberBinding<M>>>>>;

/// One member's binding for one path: Three's
/// `new PropertyBinding( object, path, parsedPath )`.
///
/// Resolution is lazy, as Three's `bind()` is: the target is looked up on first
/// use and dropped by [`MemberBinding::unbind`].
pub struct MemberBinding<M: GroupMember> {
    member: MemberRef<M>,
    path: String,
    parsed_path: ParsedTrackName,
    target: Option<Box<dyn BindingTarget>>,
    resolve_failed: bool,
}

impl<M: GroupMember> MemberBinding<M> {
    fn new(member: MemberRef<M>, path: &str, parsed_path: &ParsedTrackName) -> Self {
        Self {
            member,
            path: path.to_string(),
            parsed_path: parsed_path.clone(),
            target: None,
            resolve_failed: false,
        }
    }

    /// `binding.path`.
    pub fn path(&self) -> &str {
        &self.path
    }

    /// `binding.parsedPath`.
    pub fn parsed_path(&self) -> &ParsedTrackName {
        &self.parsed_path
    }

    /// `binding.rootNode`.
    pub fn member(&self) -> &MemberRef<M> {
        &self.member
    }

    /// `binding.rootNode.uuid`.
    pub fn member_uuid(&self) -> String {
        self.member.borrow().uuid()
    }

    /// `bind()`: resolve the path against the member. Idempotent; a failed
    /// resolve is remembered so it is not retried every frame (Three logs once
    /// and installs the no-op `_getValue_unavailable`).
    pub fn bind(&mut self) -> bool {
        if self.target.is_some() {
            return true;
        }
        if self.resolve_failed {
            return false;
        }
        match self.member.borrow_mut().resolve(&self.parsed_path) {
            Some(target) => {
                self.target = Some(target);
                true
            }
            None => {
                self.resolve_failed = true;
                false
            }
        }
    }

    /// `unbind()`.
    pub fn unbind(&mut self) {
        self.target = None;
        self.resolve_failed = false;
    }

    /// `getValue( buffer, offset )`; a no-op if the path does not resolve.
    pub fn get_value(&mut self, buffer: &mut [f64], offset: usize) {
        if self.bind() {
            self.target.as_ref().unwrap().get_value(buffer, offset);
        }
    }

    /// `setValue( buffer, offset )`; a no-op if the path does not resolve.
    pub fn set_value(&mut self, buffer: &[f64], offset: usize) {
        if self.bind() {
            self.target.as_mut().unwrap().set_value(buffer, offset);
        }
    }

    /// `valueSize`, or 0 while unresolved.
    pub fn value_size(&mut self) -> usize {
        if self.bind() {
            self.target.as_ref().unwrap().value_size()
        } else {
            0
        }
    }
}

/// `PropertyBinding.Composite`: one [`BindingTarget`] fanning out over a group's
/// active members.
///
/// `getValue` reads the first active member (`_bindings[ nCachedObjects_ ]`),
/// `setValue` writes every active member — verbatim Three.
pub struct Composite<M: GroupMember> {
    bindings: BindingsForPath<M>,
    n_cached_objects: Rc<Cell<usize>>,
}

impl<M: GroupMember> Composite<M> {
    /// `new Composite( targetGroup, path, optionalParsedPath )`, after the group
    /// has handed out the row for that path.
    pub fn new(bindings: BindingsForPath<M>, n_cached_objects: Rc<Cell<usize>>) -> Self {
        Self {
            bindings,
            n_cached_objects,
        }
    }
}

impl<M: GroupMember + 'static> BindingTarget for Composite<M> {
    fn get_value(&self, buffer: &mut [f64], offset: usize) {
        let first = self.n_cached_objects.get();
        let mut bindings = self.bindings.borrow_mut();
        if let Some(Some(binding)) = bindings.get_mut(first) {
            binding.get_value(buffer, offset);
        }
    }

    fn set_value(&mut self, buffer: &[f64], offset: usize) {
        let first = self.n_cached_objects.get();
        let mut bindings = self.bindings.borrow_mut();
        let n = bindings.len();
        for i in first..n {
            if let Some(binding) = bindings[i].as_mut() {
                binding.set_value(buffer, offset);
            }
        }
    }

    fn value_size(&self) -> usize {
        let first = self.n_cached_objects.get();
        let mut bindings = self.bindings.borrow_mut();
        match bindings.get_mut(first) {
            Some(Some(binding)) => binding.value_size(),
            _ => 0,
        }
    }

    fn bind(&mut self) {
        let first = self.n_cached_objects.get();
        let mut bindings = self.bindings.borrow_mut();
        let n = bindings.len();
        for i in first..n {
            if let Some(binding) = bindings[i].as_mut() {
                binding.bind();
            }
        }
    }

    fn unbind(&mut self) {
        let first = self.n_cached_objects.get();
        let mut bindings = self.bindings.borrow_mut();
        let n = bindings.len();
        for i in first..n {
            if let Some(binding) = bindings[i].as_mut() {
                binding.unbind();
            }
        }
    }
}

/// `group.stats.objects`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StatsObjects {
    /// `stats.objects.total`.
    pub total: usize,
    /// `stats.objects.inUse`.
    pub in_use: usize,
}

/// `group.stats` — a snapshot, where Three uses live getters.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Stats {
    /// `stats.objects`.
    pub objects: StatsObjects,
    /// `stats.bindingsPerObject`.
    pub bindings_per_object: usize,
}

/// A group of objects that receives a shared animation state.
///
/// See the module documentation for the member/ownership design.
pub struct AnimationObjectGroup<M: GroupMember> {
    /// `uuid`.
    pub uuid: String,
    /// `_objects`: cached objects first, then the active ones.
    objects: Vec<MemberRef<M>>,
    /// `_indicesByUUID`.
    indices_by_uuid: HashMap<String, usize>,
    /// `nCachedObjects_`, the threshold; shared because `Composite` reads it.
    n_cached_objects: Rc<Cell<usize>>,
    /// `_paths`.
    paths: Vec<String>,
    /// `_parsedPaths`.
    parsed_paths: Vec<ParsedTrackName>,
    /// `_bindings`.
    bindings: Vec<BindingsForPath<M>>,
    /// `_bindingsIndicesByPath`.
    bindings_indices_by_path: HashMap<String, usize>,
}

impl<M: GroupMember> Default for AnimationObjectGroup<M> {
    fn default() -> Self {
        Self::new()
    }
}

impl<M: GroupMember> AnimationObjectGroup<M> {
    /// `new AnimationObjectGroup()`.
    pub fn new() -> Self {
        Self {
            uuid: generate_uuid(),
            objects: Vec::new(),
            indices_by_uuid: HashMap::new(),
            n_cached_objects: Rc::new(Cell::new(0)),
            paths: Vec::new(),
            parsed_paths: Vec::new(),
            bindings: Vec::new(),
            bindings_indices_by_path: HashMap::new(),
        }
    }

    /// `new AnimationObjectGroup( ...objects )` — the variadic constructor.
    pub fn with_members(members: &[MemberRef<M>]) -> Self {
        let mut group = Self::new();
        // The constructor does not go through `add`: it pushes the objects
        // straight into the (empty) active region and indexes them.
        for (i, member) in members.iter().enumerate() {
            group.indices_by_uuid.insert(member.borrow().uuid(), i);
            group.objects.push(Rc::clone(member));
        }
        group
    }

    /// `isAnimationObjectGroup`.
    pub fn is_animation_object_group(&self) -> bool {
        true
    }

    /// `nCachedObjects_`.
    pub fn n_cached_objects(&self) -> usize {
        self.n_cached_objects.get()
    }

    /// The shared `nCachedObjects_` cell, as `Composite` holds it.
    pub fn n_cached_objects_handle(&self) -> Rc<Cell<usize>> {
        Rc::clone(&self.n_cached_objects)
    }

    /// `_objects` (cached region first).
    pub fn objects(&self) -> &[MemberRef<M>] {
        &self.objects
    }

    /// `stats`.
    pub fn stats(&self) -> Stats {
        let total = self.objects.len();
        Stats {
            objects: StatsObjects {
                total,
                in_use: total - self.n_cached_objects.get(),
            },
            bindings_per_object: self.bindings.len(),
        }
    }

    /// `add( ...objects )`.
    pub fn add(&mut self, members: &[MemberRef<M>]) {
        let n_bindings = self.bindings.len();
        let mut known_object: Option<MemberRef<M>> = None;
        let mut n_objects = self.objects.len();
        let mut n_cached_objects = self.n_cached_objects.get();

        for member in members {
            let uuid = member.borrow().uuid();
            let index = self.indices_by_uuid.get(&uuid).copied();

            match index {
                None => {
                    // unknown object -> add it to the ACTIVE region
                    let index = n_objects;
                    n_objects += 1;
                    self.indices_by_uuid.insert(uuid, index);
                    self.objects.push(Rc::clone(member));

                    // accounting is done, now do the same for all bindings
                    for j in 0..n_bindings {
                        self.bindings[j].borrow_mut().push(Some(MemberBinding::new(
                            Rc::clone(member),
                            &self.paths[j],
                            &self.parsed_paths[j],
                        )));
                    }
                }
                Some(index) if index < n_cached_objects => {
                    known_object = Some(Rc::clone(&self.objects[index]));

                    // move existing object to the ACTIVE region
                    n_cached_objects -= 1;
                    let first_active_index = n_cached_objects;
                    let last_cached_object = Rc::clone(&self.objects[first_active_index]);

                    let last_cached_uuid = last_cached_object.borrow().uuid();
                    self.indices_by_uuid.insert(last_cached_uuid, index);
                    self.objects[index] = last_cached_object;

                    self.indices_by_uuid.insert(uuid, first_active_index);
                    self.objects[first_active_index] = Rc::clone(member);

                    // accounting is done, now do the same for all bindings
                    for j in 0..n_bindings {
                        let mut bindings_for_path = self.bindings[j].borrow_mut();

                        if index == first_active_index {
                            // JS aliases the same slot for `lastCached` and
                            // `binding`; the net effect is that the slot keeps
                            // its binding, or gets a fresh one if it was a hole.
                            let binding =
                                bindings_for_path[index].take().unwrap_or_else(|| {
                                    MemberBinding::new(
                                        Rc::clone(member),
                                        &self.paths[j],
                                        &self.parsed_paths[j],
                                    )
                                });
                            bindings_for_path[index] = Some(binding);
                            continue;
                        }

                        let last_cached = bindings_for_path[first_active_index].take();

                        let binding = bindings_for_path[index].take();
                        bindings_for_path[index] = last_cached;

                        // since we do not bother to create new bindings for
                        // objects that are cached, the binding may or may not
                        // exist
                        let binding = binding.unwrap_or_else(|| {
                            MemberBinding::new(
                                Rc::clone(member),
                                &self.paths[j],
                                &self.parsed_paths[j],
                            )
                        });

                        bindings_for_path[first_active_index] = Some(binding);
                    }
                }
                Some(index) => {
                    // Faithful to Three, quirk included: the comparison is
                    // against the *last* object moved out of the cached region,
                    // not against `member`, so re-adding an already-active
                    // object can reach this branch.
                    let same = known_object
                        .as_ref()
                        .is_some_and(|known| Rc::ptr_eq(&self.objects[index], known));
                    if !same && !Rc::ptr_eq(&self.objects[index], member) {
                        // `error( 'AnimationObjectGroup: Different objects with
                        // the same UUID detected. …' )`
                        eprintln!(
                            "AnimationObjectGroup: Different objects with the same UUID \
                             detected. Clean the caches or recreate your infrastructure \
                             when reloading scenes."
                        );
                    }
                    // else the object is already where we want it to be
                }
            }
        }

        self.n_cached_objects.set(n_cached_objects);
    }

    /// `add( object )`, for one member.
    pub fn add_one(&mut self, member: &MemberRef<M>) {
        self.add(std::slice::from_ref(member));
    }

    /// `remove( ...objects )`.
    pub fn remove(&mut self, members: &[MemberRef<M>]) {
        let n_bindings = self.bindings.len();
        let mut n_cached_objects = self.n_cached_objects.get();

        for member in members {
            let uuid = member.borrow().uuid();
            let index = self.indices_by_uuid.get(&uuid).copied();

            if let Some(index) = index {
                if index >= n_cached_objects {
                    // move existing object into the CACHED region
                    let last_cached_index = n_cached_objects;
                    n_cached_objects += 1;
                    let first_active_object = Rc::clone(&self.objects[last_cached_index]);

                    let first_active_uuid = first_active_object.borrow().uuid();
                    self.indices_by_uuid.insert(first_active_uuid, index);
                    self.objects[index] = first_active_object;

                    self.indices_by_uuid.insert(uuid, last_cached_index);
                    self.objects[last_cached_index] = Rc::clone(member);

                    // accounting is done, now do the same for all bindings
                    if index != last_cached_index {
                        for j in 0..n_bindings {
                            let mut bindings_for_path = self.bindings[j].borrow_mut();
                            let first_active = bindings_for_path[last_cached_index].take();
                            let binding = bindings_for_path[index].take();

                            bindings_for_path[index] = first_active;
                            bindings_for_path[last_cached_index] = binding;
                        }
                    }
                    // `index == lastCachedIndex` is JS's self-swap: a no-op.
                }
            }
        }

        self.n_cached_objects.set(n_cached_objects);
    }

    /// `remove( object )`, for one member.
    pub fn remove_one(&mut self, member: &MemberRef<M>) {
        self.remove(std::slice::from_ref(member));
    }

    /// `uncache( ...objects )`.
    pub fn uncache(&mut self, members: &[MemberRef<M>]) {
        let n_bindings = self.bindings.len();
        let mut n_cached_objects = self.n_cached_objects.get();
        let mut n_objects = self.objects.len();

        for member in members {
            let uuid = member.borrow().uuid();
            let index = self.indices_by_uuid.get(&uuid).copied();

            if let Some(index) = index {
                self.indices_by_uuid.remove(&uuid);

                if index < n_cached_objects {
                    // object is cached, shrink the CACHED region
                    n_cached_objects -= 1;
                    let first_active_index = n_cached_objects;
                    let last_cached_object = Rc::clone(&self.objects[first_active_index]);
                    n_objects -= 1;
                    let last_index = n_objects;
                    let last_object = Rc::clone(&self.objects[last_index]);

                    if index != first_active_index {
                        // last cached object takes this object's place
                        let last_cached_uuid = last_cached_object.borrow().uuid();
                        self.indices_by_uuid.insert(last_cached_uuid, index);
                    }

                    self.objects[index] = last_cached_object;

                    if first_active_index != last_index {
                        // last object goes to the activated slot and pop
                        let last_uuid = last_object.borrow().uuid();
                        self.indices_by_uuid.insert(last_uuid, first_active_index);
                    }

                    self.objects[first_active_index] = last_object;
                    self.objects.pop();

                    // accounting is done, now do the same for all bindings
                    for j in 0..n_bindings {
                        let mut bindings_for_path = self.bindings[j].borrow_mut();

                        if first_active_index == last_index {
                            // JS puts the same binding in both slots and then
                            // pops the duplicate away.
                            let v = bindings_for_path[first_active_index].take();
                            if index != first_active_index {
                                bindings_for_path[index] = v;
                            }
                            bindings_for_path.pop();
                            continue;
                        }

                        let last_cached = bindings_for_path[first_active_index].take();
                        let last = bindings_for_path[last_index].take();

                        // `index <= first_active_index < last_index`, so these
                        // writes are ordered as in JS.
                        bindings_for_path[index] = last_cached;
                        bindings_for_path[first_active_index] = last;
                        bindings_for_path.pop();
                    }
                } else {
                    // object is active, just swap with the last and pop
                    n_objects -= 1;
                    let last_index = n_objects;
                    let last_object = Rc::clone(&self.objects[last_index]);

                    if index != last_index {
                        let last_uuid = last_object.borrow().uuid();
                        self.indices_by_uuid.insert(last_uuid, index);
                    }

                    self.objects[index] = last_object;
                    self.objects.pop();

                    // accounting is done, now do the same for all bindings
                    for j in 0..n_bindings {
                        let mut bindings_for_path = self.bindings[j].borrow_mut();
                        let last = bindings_for_path[last_index].take();
                        bindings_for_path[index] = last;
                        bindings_for_path.pop();
                    }
                }
            }
        }

        self.n_cached_objects.set(n_cached_objects);
    }

    /// `uncache( object )`, for one member.
    pub fn uncache_one(&mut self, member: &MemberRef<M>) {
        self.uncache(std::slice::from_ref(member));
    }

    // Internal interface used by befriended PropertyBinding.Composite:

    /// `subscribe_( path, parsedPath )`: the bindings row for `path`, kept up to
    /// date by `add` / `remove` / `uncache` until `unsubscribe_`.
    #[allow(non_snake_case)]
    pub fn subscribe_(&mut self, path: &str, parsed_path: &ParsedTrackName) -> BindingsForPath<M> {
        if let Some(&index) = self.bindings_indices_by_path.get(path) {
            return Rc::clone(&self.bindings[index]);
        }

        let n_objects = self.objects.len();
        let n_cached_objects = self.n_cached_objects.get();
        let bindings_for_path: BindingsForPath<M> = Rc::new(RefCell::new(
            (0..n_objects).map(|_| None).collect::<Vec<_>>(),
        ));

        let index = self.bindings.len();
        self.bindings_indices_by_path.insert(path.to_string(), index);

        self.paths.push(path.to_string());
        self.parsed_paths.push(parsed_path.clone());
        self.bindings.push(Rc::clone(&bindings_for_path));

        {
            let mut row = bindings_for_path.borrow_mut();
            for i in n_cached_objects..n_objects {
                row[i] = Some(MemberBinding::new(
                    Rc::clone(&self.objects[i]),
                    path,
                    parsed_path,
                ));
            }
        }

        bindings_for_path
    }

    /// `unsubscribe_( path )`: forget a property path and stop updating the row
    /// previously obtained with [`Self::subscribe_`].
    #[allow(non_snake_case)]
    pub fn unsubscribe_(&mut self, path: &str) {
        let Some(index) = self.bindings_indices_by_path.remove(path) else {
            return;
        };

        let last_bindings_index = self.bindings.len() - 1;
        let last_bindings_path = self.paths[last_bindings_index].clone();

        self.bindings_indices_by_path
            .insert(last_bindings_path, index);

        let last_bindings = self.bindings.pop().expect("non-empty bindings");
        let last_parsed = self.parsed_paths.pop().expect("non-empty parsedPaths");
        let last_path = self.paths.pop().expect("non-empty paths");

        if index != last_bindings_index {
            self.bindings[index] = last_bindings;
            self.parsed_paths[index] = last_parsed;
            self.paths[index] = last_path;
        }
        // Three writes `indicesByPath[ lastBindingsPath ] = index` before the
        // swap, so unsubscribing the last path leaves that stale entry pointing
        // at the removed index; the `remove` above already dropped it here,
        // which is the same observable behaviour for every later lookup.
    }
}

/// A stable key for a parsed path, standing in for the raw track name that
/// `subscribe_` uses as its map key.
///
/// Divergence: `TargetResolver::resolve` receives only a [`ParsedTrackName`],
/// while Three's `Composite` is constructed with the original path string. Two
/// track names that parse identically therefore share one bindings row here.
fn parsed_path_key(parsed: &ParsedTrackName) -> String {
    format!(
        "{}/{}/{}/{}/{}",
        parsed.node_name.as_deref().unwrap_or(""),
        parsed.object_name.as_deref().unwrap_or(""),
        parsed.object_index.as_deref().unwrap_or(""),
        parsed.property_name,
        parsed.property_index.as_deref().unwrap_or("")
    )
}

impl<M: GroupMember + 'static> TargetResolver for AnimationObjectGroup<M> {
    /// `PropertyBinding.create( group, trackName )` for a group root: a
    /// [`Composite`] over the group's active members.
    fn resolve(&mut self, parsed: &ParsedTrackName) -> Option<Box<dyn BindingTarget>> {
        let key = parsed_path_key(parsed);
        let bindings = self.subscribe_(&key, parsed);
        Some(Box::new(Composite::new(
            bindings,
            self.n_cached_objects_handle(),
        )))
    }
}
