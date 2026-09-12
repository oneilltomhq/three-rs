//! Port of `three.js/test/unit/src/animation/AnimationObjectGroup.tests.js`.
//!
//! Three's members are `new Object3D()`, which this crate does not have on this
//! branch; the suite only ever needs a member's `uuid` identity and the fact
//! that a `PropertyBinding` remembers it as `rootNode`, so members here are a
//! minimal [`Stub`] implementing `GroupMember` (a `uuid` plus a `TargetResolver`
//! that resolves every path to a `BufferTarget`).
//!
//! `assert.ok( x instanceof AnimationObjectGroup )` becomes a construction that
//! type-checks, as elsewhere in this port.

use std::cell::RefCell;
use std::rc::Rc;

use three_rs::animation::animation_object_group::{
    AnimationObjectGroup, BindingsForPath, GroupMember, MemberRef,
};
use three_rs::animation::binding_target::{BindingTarget, BufferTarget, TargetResolver};
use three_rs::animation::property_binding::{parse_track_name, ParsedTrackName};

/// Stand-in for `new Object3D()`: identity plus a resolver.
struct Stub {
    uuid: String,
}

impl Stub {
    fn new(uuid: &str) -> MemberRef<Self> {
        Rc::new(RefCell::new(Self {
            uuid: uuid.to_string(),
        }))
    }
}

impl TargetResolver for Stub {
    fn resolve(&mut self, _parsed: &ParsedTrackName) -> Option<Box<dyn BindingTarget>> {
        Some(Box::new(BufferTarget::new(vec![0.0, 0.0, 0.0])))
    }
}

impl GroupMember for Stub {
    fn uuid(&self) -> String {
        self.uuid.clone()
    }
}

const PATH_A: &str = "object.position";
const PATH_B: &str = "object.rotation";
const PATH_C: &str = "object.scale";

fn parsed(path: &str) -> ParsedTrackName {
    parse_track_name(path).expect("parses")
}

// INSTANCING
#[test]
fn instancing() {
    let _group_a: AnimationObjectGroup<Stub> = AnimationObjectGroup::new();
    // `assert.ok( groupA instanceof AnimationObjectGroup )` — it type-checks.
}

// PUBLIC
#[test]
fn is_animation_object_group() {
    let object: AnimationObjectGroup<Stub> = AnimationObjectGroup::new();
    assert!(
        object.is_animation_object_group(),
        "AnimationObjectGroup.isAnimationObjectGroup should be true"
    );
}

/// Three's inner `expect( testIndex, group, bindings, path, cached, roots )`.
#[track_caller]
fn expect(
    test_index: usize,
    group: &AnimationObjectGroup<Stub>,
    bindings: &BindingsForPath<Stub>,
    path: &str,
    cached: usize,
    roots: &[MemberRef<Stub>],
) {
    let row = bindings.borrow();
    let mut root_nodes: Vec<String> = Vec::new();
    let mut paths_ok = true;

    for i in group.n_cached_objects()..row.len() {
        let binding = row[i]
            .as_ref()
            .unwrap_or_else(|| panic!("{test_index}: active slot {i} has no binding"));
        if binding.path() != path {
            paths_ok = false;
        }
        root_nodes.push(binding.member_uuid());
    }

    let mut nodes_ok = true;
    for root in roots {
        if !root_nodes.contains(&root.borrow().uuid()) {
            nodes_ok = false;
        }
    }

    assert!(paths_ok, "{test_index} paths");
    assert!(nodes_ok, "{test_index} nodes");
    assert!(
        group.n_cached_objects() == cached,
        "{test_index} cache size (got {}, expected {cached})",
        group.n_cached_objects()
    );
    assert!(
        row.len() - group.n_cached_objects() == roots.len(),
        "{test_index} object count (got {}, expected {})",
        row.len() - group.n_cached_objects(),
        roots.len()
    );
}

/// A snapshot of a bindings row, for `assert.deepEqual( bindingsBC, copy )`.
fn snapshot(bindings: &BindingsForPath<Stub>) -> Vec<Option<(String, String)>> {
    bindings
        .borrow()
        .iter()
        .map(|slot| {
            slot.as_ref()
                .map(|b| (b.path().to_string(), b.member_uuid()))
        })
        .collect()
}

// OTHERS
#[test]
fn smoke_test() {
    let object_a = Stub::new("ObjectA");
    let object_b = Stub::new("ObjectB");
    let object_c = Stub::new("ObjectC");

    let parsed_path_a = parsed(PATH_A);
    let parsed_path_b = parsed(PATH_B);
    let parsed_path_c = parsed(PATH_C);

    // initial state

    let mut group_a: AnimationObjectGroup<Stub> = AnimationObjectGroup::new();
    // 'constructor (w/o args)'

    let bindings_aa = group_a.subscribe_(PATH_A, &parsed_path_a);
    expect(0, &group_a, &bindings_aa, PATH_A, 0, &[]);

    let mut group_b: AnimationObjectGroup<Stub> =
        AnimationObjectGroup::with_members(&[Rc::clone(&object_a), Rc::clone(&object_b)]);
    // 'constructor (with args)'

    let bindings_bb = group_b.subscribe_(PATH_B, &parsed_path_b);
    expect(
        1,
        &group_b,
        &bindings_bb,
        PATH_B,
        0,
        &[Rc::clone(&object_a), Rc::clone(&object_b)],
    );

    // add

    group_a.add(&[Rc::clone(&object_a), Rc::clone(&object_b)]);
    expect(
        2,
        &group_a,
        &bindings_aa,
        PATH_A,
        0,
        &[Rc::clone(&object_a), Rc::clone(&object_b)],
    );

    group_b.add(&[Rc::clone(&object_c)]);
    expect(
        3,
        &group_b,
        &bindings_bb,
        PATH_B,
        0,
        &[
            Rc::clone(&object_a),
            Rc::clone(&object_b),
            Rc::clone(&object_c),
        ],
    );

    // remove

    group_a.remove(&[Rc::clone(&object_a), Rc::clone(&object_c)]);
    expect(4, &group_a, &bindings_aa, PATH_A, 1, &[Rc::clone(&object_b)]);

    group_b.remove(&[
        Rc::clone(&object_a),
        Rc::clone(&object_b),
        Rc::clone(&object_c),
    ]);
    expect(5, &group_b, &bindings_bb, PATH_B, 3, &[]);

    // subscribe after re-add

    group_a.add(&[Rc::clone(&object_c)]);
    expect(
        6,
        &group_a,
        &bindings_aa,
        PATH_A,
        1,
        &[Rc::clone(&object_b), Rc::clone(&object_c)],
    );
    let bindings_ac = group_a.subscribe_(PATH_C, &parsed_path_c);
    expect(
        7,
        &group_a,
        &bindings_ac,
        PATH_C,
        1,
        &[Rc::clone(&object_b), Rc::clone(&object_c)],
    );

    // re-add after subscribe

    let bindings_bc = group_b.subscribe_(PATH_C, &parsed_path_c);
    group_b.add(&[Rc::clone(&object_a), Rc::clone(&object_b)]);
    expect(
        8,
        &group_b,
        &bindings_bb,
        PATH_B,
        1,
        &[Rc::clone(&object_a), Rc::clone(&object_b)],
    );

    // unsubscribe

    let copy_of_bindings_bc = snapshot(&bindings_bc);
    group_b.unsubscribe_(PATH_C);
    group_b.add(&[Rc::clone(&object_c)]);
    assert_eq!(
        snapshot(&bindings_bc),
        copy_of_bindings_bc,
        "no more update after unsubscribe"
    );

    // uncache active

    group_b.uncache(&[Rc::clone(&object_a)]);
    expect(
        9,
        &group_b,
        &bindings_bb,
        PATH_B,
        0,
        &[Rc::clone(&object_b), Rc::clone(&object_c)],
    );

    // uncache cached

    group_a.uncache(&[Rc::clone(&object_a)]);
    expect(
        10,
        &group_a,
        &bindings_ac,
        PATH_C,
        0,
        &[Rc::clone(&object_b), Rc::clone(&object_c)],
    );
}

// Beyond Three's suite: the `stats` getters and the `Composite` fan-out, neither
// of which Three's AnimationObjectGroup suite exercises (`stats` is read by the
// mixer, `Composite` lives in PropertyBinding.tests.js, which needs Object3D).
#[test]
fn stats() {
    let object_a = Stub::new("ObjectA");
    let object_b = Stub::new("ObjectB");

    let mut group: AnimationObjectGroup<Stub> =
        AnimationObjectGroup::with_members(&[Rc::clone(&object_a), Rc::clone(&object_b)]);
    let stats = group.stats();
    assert_eq!(stats.objects.total, 2, "stats.objects.total");
    assert_eq!(stats.objects.in_use, 2, "stats.objects.inUse");
    assert_eq!(stats.bindings_per_object, 0, "stats.bindingsPerObject");

    let _ = group.subscribe_(PATH_A, &parsed(PATH_A));
    assert_eq!(
        group.stats().bindings_per_object,
        1,
        "stats.bindingsPerObject after subscribe_"
    );

    group.remove(&[Rc::clone(&object_a)]);
    let stats = group.stats();
    assert_eq!(stats.objects.total, 2, "total is unchanged by remove");
    assert_eq!(stats.objects.in_use, 1, "inUse drops with the cached region");

    group.uncache(&[Rc::clone(&object_a)]);
    assert_eq!(group.stats().objects.total, 1, "uncache shrinks total");
}

#[test]
fn composite_fans_out_over_active_members() {
    let object_a = Stub::new("ObjectA");
    let object_b = Stub::new("ObjectB");

    let mut group: AnimationObjectGroup<Stub> =
        AnimationObjectGroup::with_members(&[Rc::clone(&object_a), Rc::clone(&object_b)]);

    // `PropertyBinding.create( group, path )` -> a Composite.
    let mut composite = group.resolve(&parsed(PATH_A)).expect("resolves");
    assert_eq!(composite.value_size(), 3, "valueSize of the first active");

    composite.set_value(&[1.0, 2.0, 3.0], 0);
    let mut out = [0.0; 3];
    composite.get_value(&mut out, 0);
    assert_eq!(out, [1.0, 2.0, 3.0], "getValue reads the first active member");

    // Removing a member moves it into the cached region, so the fan-out skips it;
    // the remaining active member is still written.
    group.remove(&[Rc::clone(&object_a)]);
    composite.set_value(&[4.0, 5.0, 6.0], 0);
    let mut out = [0.0; 3];
    composite.get_value(&mut out, 0);
    assert_eq!(out, [4.0, 5.0, 6.0], "still bound after remove");
}

// SKIPPED: nothing in Three's AnimationObjectGroup.tests.js needs `Object3D`
// beyond the `uuid` identity substituted above, so no assertion from that suite
// is left out.
