//! Port of `three.js/src/objects/Group.js`.

use crate::core::node::Node;
use crate::core::Object3D;

/// `Group` — an `Object3D` that exists only to hold children, with
/// `type = 'Group'` and `isGroup = true`. three.js implements it as a subclass
/// with no extra state, so here it is a constructor returning a [`Node`].
pub struct Group;

impl Group {
    /// `new Group()`.
    #[allow(clippy::new_ret_no_self)] // `new` mirrors three.js's constructor and returns a scene-graph `Node`, not `Self`; public API, not changing.
    pub fn new() -> Node {
        let object = Object3D {
            object_type: "Group",
            is_group: true,
            ..Default::default()
        };
        object.into_node()
    }
}
