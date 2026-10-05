//! Port of `three.js/src/objects/Group.js`.

use crate::core::object_ref::ObjectRef;
use crate::core::Object3D;

/// `Group` — an `Object3D` that exists only to hold children, with
/// `type = 'Group'` and `isGroup = true`. three.js implements it as a subclass
/// with no extra state, so here it is a constructor returning a [`ObjectRef`].
pub struct Group;

impl Group {
    /// `new Group()`.
    #[allow(clippy::new_ret_no_self)] // `new` mirrors three.js's constructor and returns a scene-graph `ObjectRef`, not `Self`; public API, not changing.
    pub fn new() -> ObjectRef {
        let object = Object3D {
            object_type: "Group",
            is_group: true,
            ..Default::default()
        };
        object.into_node()
    }
}
