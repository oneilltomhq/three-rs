//! Port of `three.js/test/unit/src/objects/Group.tests.js`.
//!
//! `Extending` ports as "a `Group` *is* an `Object3D` node": three.js' `Group`
//! is a subclass with no state of its own, so here it is a constructor that
//! returns a `Node` with `type = 'Group'`.

use three_rs::core::{Object3DNode, Object3D};
use three_rs::objects::Group;

#[test]
fn instancing() {
    let object = Group::new();
    assert_eq!(object.children().len(), 0, "Can instantiate a Group.");
}

#[test]
fn extending() {
    // Every `Object3D` method is available on it, which is what the JS
    // `instanceof Object3D` assertion is checking for.
    let group = Group::new();
    let child = Object3D::new_node();
    group.add(&child);
    assert_eq!(group.children().len(), 1, "Group extends from Object3D");
}

#[test]
fn type_name() {
    let object = Group::new();
    assert_eq!(
        object.borrow().object_type,
        "Group",
        "Group.type should be Group"
    );
}

#[test]
fn is_group() {
    let object = Group::new();
    assert!(object.borrow().is_group, "Group.isGroup should be true");
}
