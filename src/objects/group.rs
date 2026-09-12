//! Port of `three.js/src/objects/Group.js`.

use crate::core::node::Node;
use crate::core::Object3D;

/// `Group` — an `Object3D` that exists only to hold children, with
/// `type = 'Group'` and `isGroup = true`. three.js implements it as a subclass
/// with no extra state, so here it is a constructor returning a [`Node`].
pub struct Group;

impl Group {
    /// `new Group()`.
    pub fn new() -> Node {
        let mut object = Object3D::default();
        object.object_type = "Group";
        object.is_group = true;
        object.into_node()
    }
}
