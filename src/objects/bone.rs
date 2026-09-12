//! Port of `three.js/src/objects/Bone.js`.
//!
//! `Bone` adds nothing to `Object3D` but `isBone` and `type = 'Bone'`. The
//! scene graph stores its payload as [`Object3D`], which has no `is_bone`
//! field, so the marker is `object_type == "Bone"`; [`is_bone`] is the test.

use crate::core::{Node, Object3D};

/// `class Bone extends Object3D`.
pub struct Bone;

impl Bone {
    /// `new Bone()`, as a scene-graph [`Node`].
    pub fn new() -> Node {
        let mut object = Object3D::default();
        object.object_type = "Bone";
        object.into_node()
    }

    /// `new Bone()` with a name, which is how `GLTFLoader` builds them.
    pub fn named(name: &str) -> Node {
        let node = Self::new();
        node.borrow_mut().name = name.to_string();
        node
    }
}

/// `object.isBone`.
pub fn is_bone(object: &Object3D) -> bool {
    object.object_type == "Bone"
}
