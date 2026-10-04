//! Port of `three.js/src/objects/ClippingGroup.js`.

use crate::core::node::Node;
use crate::core::Object3D;
use crate::math::Plane;
use crate::objects::Payload;

/// `ClippingGroup` — a `Group` whose clipping planes clip every descendant,
/// which is how `WebGPURenderer` encodes clipping in the scene graph
/// (`Material.clippingPlanes` and `renderer.clippingPlanes` are WebGL-only).
///
/// The renderer's walk keeps a clipping context per group it enters
/// (`ClippingContext.getGroupContext()`): a nested group's planes are added to
/// its parent's, as an intersection when `clip_intersection` is set and as a
/// union otherwise. The planes are world-space and read every frame, so moving
/// one needs nothing; changing how many there are rebuilds the programs below
/// the group, as a new cache key does in three.js.
///
/// The node is an `Object3D` with `type = 'ClippingGroup'` and `isGroup =
/// true`; this state is its [`Payload::ClippingGroup`].
#[derive(Clone, Debug)]
pub struct ClippingGroup {
    /// `ClippingGroup.clippingPlanes`, world-space.
    pub clipping_planes: Vec<Plane>,
    /// `ClippingGroup.enabled` — with it off the group clips nothing and its
    /// descendants keep the context they would have had without it.
    pub enabled: bool,
    /// `ClippingGroup.clipIntersection` — clip only what is behind *all* of
    /// the planes, rather than what is behind any one of them.
    pub clip_intersection: bool,
    /// `ClippingGroup.clipShadows` — whether the planes also clip the
    /// descendants' shadow-map draws.
    pub clip_shadows: bool,
}

impl Default for ClippingGroup {
    fn default() -> Self {
        Self {
            clipping_planes: Vec::new(),
            enabled: true,
            clip_intersection: false,
            clip_shadows: false,
        }
    }
}

impl ClippingGroup {
    /// `new ClippingGroup()`, as a scene-graph [`Node`], with no planes.
    #[allow(clippy::new_ret_no_self)] // `new` mirrors three.js's constructor and returns a scene-graph `Node`, not `Self`; public API, not changing.
    pub fn new() -> Node {
        Self::of(Self::default())
    }

    /// A `ClippingGroup` node holding `group`.
    pub fn of(group: Self) -> Node {
        let mut object = Object3D {
            object_type: "ClippingGroup",
            is_group: true,
            ..Default::default()
        };
        object.payload = Payload::ClippingGroup(group);
        object.into_node()
    }
}
