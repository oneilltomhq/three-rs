//! Port of `three.js/src/nodes/core/NodeFrame.js` and the `NodeUpdateType`
//! half of `constants.js`: the renderer-owned clock, and the three update
//! phases a node can take part in. See `docs/nodes.md` §57.
//!
//! Three's `NodeFrame` is one object. It holds the clock (`time`,
//! `deltaTime`, `frameId`, `renderId`), the three update maps, and, while an
//! update runs, the render object being drawn. The port splits it along the
//! borrow it needs:
//!
//! * [`NodeFrameState`] is the renderer's: the clock and the maps. The
//!   renderer owns one and reads it back through
//!   [`Renderer::node_frame`](crate::renderer::Renderer::node_frame).
//! * [`NodeFrame`](crate::nodes::NodeFrame) is what an object-update uniform
//!   is handed: the render object and the occlusion results, borrowed for one
//!   draw, plus a copy of the clock (§39).
//!
//! A node takes part in the phases through [`NodeUpdate`]. The builder
//! collects every such node a material reaches into its
//! [`NodeProgram`](crate::nodes::NodeProgram), which is cached with the
//! material, so a steady frame walks three short lists and no graph.

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::{Rc, Weak};

use crate::renderer::Renderer;

/// `NodeUpdateType`: how often a phase runs for one node.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum NodeUpdateType {
    /// `NONE`: never.
    #[default]
    None,
    /// `FRAME`: at most once per frame, however many draws reach the node.
    Frame,
    /// `RENDER`: at most once per `render()` call.
    Render,
    /// `OBJECT`: every time a draw that reaches the node is made.
    Object,
}

/// The update half of three's `Node`: `updateBeforeType` / `updateType` /
/// `updateAfterType` and the three methods they gate.
///
/// Three hands each method the `NodeFrame`, and the node reads
/// `frame.renderer` off it. The port hands the renderer itself, and the clock
/// is [`renderer.node_frame()`](Renderer::node_frame).
///
/// A method returns `false` for "this did not count", three's `=== false`:
/// the guard is left as it was, so the phase runs again at the next chance.
pub trait NodeUpdate {
    /// `getUpdateBeforeType()`.
    fn update_before_type(&self) -> NodeUpdateType {
        NodeUpdateType::None
    }
    /// `getUpdateType()`.
    fn update_type(&self) -> NodeUpdateType {
        NodeUpdateType::None
    }
    /// `getUpdateAfterType()`.
    fn update_after_type(&self) -> NodeUpdateType {
        NodeUpdateType::None
    }
    /// `updateBefore( frame )`: before the draw that reaches the node. A pass
    /// renders its scene here.
    fn update_before(&self, _renderer: &mut Renderer) -> bool {
        true
    }
    /// `update( frame )`: while the draw's bindings are updated.
    fn update(&self, _renderer: &mut Renderer) -> bool {
        true
    }
    /// `updateAfter( frame )`: after the draw.
    fn update_after(&self, _renderer: &mut Renderer) -> bool {
        true
    }
    /// Three's `PassNode.setup()` sets `renderTarget.samples = renderer.samples`
    /// while the graph that reaches the pass is analyzed — before any pass
    /// renders. That is what lets a composite bind the pass's depth texture
    /// as `texture_depth_multisampled_2d` on the very build that discovers
    /// it. The port has no per-node `setup()` hook (`docs/nodes.md` §57.2),
    /// so [`sync_before_build`](Self::sync_before_build) stands in for it: it
    /// runs on every node with a live texture registration, right before
    /// [`Renderer`] builds a material's program,
    /// which is the earliest point the builder can see a pass's textures.
    /// [`PassNode`](crate::PassNode) is the only node that overrides it
    /// (`docs/nodes.md` §57.5); the default is a no-op.
    fn sync_before_build(&self, _samples: u32) {}
}

/// One entry of three's `updateBeforeNodes` / `updateNodes` /
/// `updateAfterNodes`: the node, and `node.updateReference( frame )`, the
/// identity its guard is kept under.
#[derive(Clone)]
pub(crate) struct UpdateNode {
    pub(crate) reference: usize,
    pub(crate) node: Rc<dyn NodeUpdate>,
}

impl UpdateNode {
    /// An entry for `node`, guarded under its own address.
    pub fn new<T: NodeUpdate + 'static>(node: Rc<T>) -> Self {
        Self {
            reference: Rc::as_ptr(&node) as *const u8 as usize,
            node,
        }
    }

    /// An entry guarded under `reference`, for a node that is a wrapper
    /// around the thing that has the identity (a `CustomNode`, a
    /// `ComputeFlow`).
    pub(crate) fn with_reference(reference: usize, node: Rc<dyn NodeUpdate>) -> Self {
        Self { reference, node }
    }
}

impl std::fmt::Debug for UpdateNode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("UpdateNode")
            .field(&(self.reference as *const u8))
            .finish()
    }
}

/// Which of the three maps a guard lives in.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) enum UpdatePhase {
    Before,
    Update,
    After,
}

/// `{ renderId, frameId }`, one per node per phase (`_getMaps()`).
#[derive(Clone, Copy, Debug, Default)]
struct Stamp {
    frame_id: u64,
    render_id: u64,
    /// The frame the entry was last claimed in, for [`NodeFrameState::sweep`].
    touched: u64,
}

/// A phase that [`NodeFrameState::claim`] let through. Hand it back to
/// [`NodeFrameState::settle`] with what the method returned.
#[must_use]
#[derive(Debug)]
pub(crate) struct Claim {
    phase: UpdatePhase,
    reference: usize,
    ty: NodeUpdateType,
    previous: Stamp,
}

/// Three's `NodeFrame`, the renderer's half: the clock and the update maps.
#[derive(Debug, Default)]
pub struct NodeFrameState {
    /// `frameId`: incremented by [`update`](Self::update), once per frame.
    pub frame_id: u64,
    /// `renderId`: which `render()` call is running. Three sets it to
    /// `info.calls` and puts the previous value back when a nested render
    /// returns, so a pass rendered from inside a draw does not change the
    /// outer render's id.
    pub render_id: u64,
    /// `time`, in seconds — what the TSL `time` node reads.
    pub time: f64,
    /// `deltaTime` — the last update's step.
    pub delta_time: f64,
    /// `lastTime` — `undefined` until the first frame, which is what makes
    /// that frame's delta 0 whatever the clock says.
    last_time: Option<f64>,
    /// The count `render_id` is taken from: never reset, unlike `info.calls`.
    renders: u64,
    /// Whether a frame has been opened by a render and not yet closed by a
    /// render to the screen. See [`begin_render`](Self::begin_render).
    open: bool,
    /// `updateBeforeMap` / `updateMap` / `updateAfterMap`, keyed on the
    /// update reference.
    maps: HashMap<(UpdatePhase, usize), Stamp>,
}

impl NodeFrameState {
    /// `NodeFrame.update()`:
    ///
    /// ```js
    /// this.frameId ++;
    /// if ( this.lastTime === undefined ) this.lastTime = performance.now();
    /// this.deltaTime = ( performance.now() - this.lastTime ) / 1000;
    /// this.lastTime = performance.now();
    /// this.time += this.deltaTime;
    /// ```
    ///
    /// `now` is in milliseconds, as `performance.now()` is.
    pub fn update(&mut self, now: f64) {
        self.frame_id += 1;
        let last = *self.last_time.get_or_insert(now);
        self.delta_time = (now - last) / 1000.0;
        self.time += self.delta_time;
        self.last_time = Some(now);
    }

    /// The start of a `render()` or `render_quad()`: open a frame if none is
    /// open (three's animation loop calls `update()` once per display frame,
    /// before any render of it), then take a new render id. Returns the id
    /// to put back with [`end_render`](Self::end_render).
    pub(crate) fn begin_render(&mut self, now: f64) -> u64 {
        self.open(now);
        self.renders += 1;
        std::mem::replace(&mut self.render_id, self.renders)
    }

    /// Open a frame if none is open, as the first render of one does.
    pub(crate) fn open(&mut self, now: f64) {
        if !self.open {
            self.update(now);
            self.open = true;
        }
    }

    /// The end of a render: put the outer render's id back, and close the
    /// frame if this render drew to the screen. The port has no animation
    /// loop to mark frames, and a render to the screen is the last thing a
    /// frame does, so that is where one ends. Renders into a target — a
    /// pass's scene, a bloom's quads — belong to the frame they are part of.
    pub(crate) fn end_render(&mut self, previous_render_id: u64, to_screen: bool) {
        self.render_id = previous_render_id;
        if to_screen {
            self.open = false;
        }
    }

    /// The guard half of `updateBeforeNode()` / `updateNode()` /
    /// `updateAfterNode()`: `Some` if the phase should run now.
    ///
    /// For `updateBefore` three writes the stamp first and puts the old one
    /// back if the method returns `false`; for the other two it writes the
    /// stamp only once the method has returned something else. Either way
    /// the stamp the map ends up with is the same, except while the method
    /// runs: a pass's nested render that reaches the same node again finds
    /// its before-guard already taken, which is what stops a pass sampling
    /// itself from recursing. [`settle`](Self::settle) does the rest.
    pub(crate) fn claim(
        &mut self,
        phase: UpdatePhase,
        reference: usize,
        ty: NodeUpdateType,
    ) -> Option<Claim> {
        let (frame_id, render_id) = (self.frame_id, self.render_id);
        let stamp = self.maps.entry((phase, reference)).or_default();
        stamp.touched = frame_id;
        let previous = *stamp;
        match ty {
            NodeUpdateType::None => return None,
            NodeUpdateType::Frame if stamp.frame_id == frame_id => return None,
            NodeUpdateType::Render if stamp.render_id == render_id => return None,
            NodeUpdateType::Frame if phase == UpdatePhase::Before => stamp.frame_id = frame_id,
            NodeUpdateType::Render if phase == UpdatePhase::Before => stamp.render_id = render_id,
            _ => {}
        }
        Some(Claim {
            phase,
            reference,
            ty,
            previous,
        })
    }

    /// The rest of a claimed phase, given what the method returned.
    pub(crate) fn settle(&mut self, claim: Claim, counted: bool) {
        let (frame_id, render_id) = (self.frame_id, self.render_id);
        let Some(stamp) = self.maps.get_mut(&(claim.phase, claim.reference)) else {
            return;
        };
        match (claim.phase, counted) {
            (UpdatePhase::Before, false) => *stamp = claim.previous,
            (UpdatePhase::Before, true) => {}
            (_, false) => {}
            (_, true) => match claim.ty {
                NodeUpdateType::Frame => stamp.frame_id = frame_id,
                NodeUpdateType::Render => stamp.render_id = render_id,
                NodeUpdateType::None | NodeUpdateType::Object => {}
            },
        }
    }

    /// Drop the stamps no node has touched for `grace` frames, so a consumer
    /// that makes new passes every frame does not grow the maps.
    pub(crate) fn sweep(&mut self, grace: u64) {
        let cutoff = self.frame_id.saturating_sub(grace);
        self.maps.retain(|_, stamp| stamp.touched >= cutoff);
    }
}

/// A [`TEXTURE_UPDATES`] entry: the node's update reference, and the node.
type TextureUpdate = (usize, Weak<dyn NodeUpdate>);

thread_local! {
    /// Which node fills a texture from its `updateBefore()`, by texture id:
    /// `PassTextureNode.passNode`, and `RTTNode` being the `TextureNode` of
    /// its own target. The port's texture node is a bare
    /// [`Node::Texture`](crate::nodes::Node::Texture) over the texture, so the
    /// link sits beside the texture instead of on the node. Weak: the pass
    /// is kept alive by whoever made it, as the example's `App` does.
    static TEXTURE_UPDATES: RefCell<HashMap<usize, TextureUpdate>> =
        RefCell::new(HashMap::new());
}

/// Mark `texture_id` as filled by `node`'s `updateBefore()`: any material
/// that binds the texture has `node` among its update-before nodes.
pub(crate) fn register_texture_update<T: NodeUpdate + 'static>(texture_id: usize, node: &Rc<T>) {
    let reference = Rc::as_ptr(node) as *const u8 as usize;
    let node: Rc<dyn NodeUpdate> = node.clone();
    let weak = Rc::downgrade(&node);
    TEXTURE_UPDATES.with(|map| {
        let mut map = map.borrow_mut();
        map.retain(|_, (_, weak)| weak.strong_count() > 0);
        map.insert(texture_id, (reference, weak));
    });
}

/// The node that fills `texture_id`, if one is registered and alive.
pub(crate) fn texture_update(texture_id: usize) -> Option<UpdateNode> {
    TEXTURE_UPDATES.with(|map| {
        let (reference, weak) = map.borrow().get(&texture_id)?.clone();
        Some(UpdateNode::with_reference(reference, weak.upgrade()?))
    })
}

/// [`NodeUpdate::sync_before_build`] on every live registered node, ahead of
/// a material build. Cheap and idempotent (`RenderTarget::set_samples`
/// returns early once a target is already at `samples`), so calling it on
/// every cache miss — the only time a build runs — costs nothing on a
/// steady frame.
pub(crate) fn sync_before_build(samples: u32) {
    TEXTURE_UPDATES.with(|map| {
        for (_, weak) in map.borrow().values() {
            if let Some(node) = weak.upgrade() {
                node.sync_before_build(samples);
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A node's guard across two frames, each of two renders, each of two
    /// draws of the node: the phases run in three's order, `FRAME` once a
    /// frame, `RENDER` once a render and `OBJECT` every draw.
    #[test]
    fn two_frames_run_the_phases_in_order_behind_their_guards() {
        let mut frame = NodeFrameState::default();
        let mut log: Vec<(u64, u64, &str)> = Vec::new();

        // One node per update type in each phase, keyed apart.
        let nodes = [
            (1usize, NodeUpdateType::Frame, "frame"),
            (2, NodeUpdateType::Render, "render"),
            (3, NodeUpdateType::Object, "object"),
        ];

        for _frame in 0..2 {
            for render in 0..2 {
                let to_screen = render == 1;
                let previous = frame.begin_render(0.0);
                for _draw in 0..2 {
                    // `_renderObjectDirect`: updateBefore, update, draw,
                    // updateAfter.
                    for (phase, name) in [
                        (UpdatePhase::Before, "before"),
                        (UpdatePhase::Update, "update"),
                    ] {
                        for (reference, ty, kind) in nodes {
                            if let Some(claim) = frame.claim(phase, reference, ty) {
                                log.push((frame.frame_id, frame.render_id, name));
                                log.push((frame.frame_id, frame.render_id, kind));
                                frame.settle(claim, true);
                            }
                        }
                    }
                    log.push((frame.frame_id, frame.render_id, "draw"));
                    for (reference, ty, kind) in nodes {
                        if let Some(claim) = frame.claim(UpdatePhase::After, reference, ty) {
                            log.push((frame.frame_id, frame.render_id, "after"));
                            log.push((frame.frame_id, frame.render_id, kind));
                            frame.settle(claim, true);
                        }
                    }
                }
                frame.end_render(previous, to_screen);
            }
        }

        let count = |f: u64, what: &str, kind: &str| {
            log.windows(2)
                .filter(|w| w[0].0 == f && w[0].2 == what && w[1].2 == kind)
                .count()
        };
        for f in [1, 2] {
            for what in ["before", "update", "after"] {
                assert_eq!(count(f, what, "frame"), 1, "{what} FRAME in frame {f}");
                assert_eq!(count(f, what, "render"), 2, "{what} RENDER in frame {f}");
                assert_eq!(count(f, what, "object"), 4, "{what} OBJECT in frame {f}");
            }
        }
        // Two frames, not four: the render into a target does not close one.
        assert_eq!(frame.frame_id, 2);

        // Order within the first draw of the first frame.
        let first: Vec<&str> = log
            .iter()
            .take_while(|entry| entry.2 != "draw")
            .map(|entry| entry.2)
            .collect();
        assert_eq!(
            first,
            [
                "before", "frame", "before", "render", "before", "object", "update", "frame",
                "update", "render", "update", "object"
            ]
        );
        let after_draw = log.iter().position(|e| e.2 == "draw").unwrap();
        assert_eq!(log[after_draw + 1].2, "after");
    }

    /// `=== false` leaves the guard as it was, so the phase runs again at the
    /// next draw of the same frame; a nested claim of a before-phase that is
    /// running is refused.
    #[test]
    fn a_phase_that_does_not_count_runs_again() {
        let mut frame = NodeFrameState::default();
        let previous = frame.begin_render(0.0);

        let claim = frame
            .claim(UpdatePhase::Before, 7, NodeUpdateType::Frame)
            .expect("first claim");
        // Re-entered while it runs: refused.
        assert!(frame
            .claim(UpdatePhase::Before, 7, NodeUpdateType::Frame)
            .is_none());
        frame.settle(claim, false);
        let claim = frame
            .claim(UpdatePhase::Before, 7, NodeUpdateType::Frame)
            .expect("runs again after a false");
        frame.settle(claim, true);
        assert!(frame
            .claim(UpdatePhase::Before, 7, NodeUpdateType::Frame)
            .is_none());

        let claim = frame
            .claim(UpdatePhase::After, 8, NodeUpdateType::Frame)
            .unwrap();
        frame.settle(claim, false);
        assert!(frame
            .claim(UpdatePhase::After, 8, NodeUpdateType::Frame)
            .is_some());
        frame.end_render(previous, true);
    }

    /// A render nested inside another gets its own id and hands the outer
    /// one back, as three's `renderId = previousRenderId` does.
    #[test]
    fn a_nested_render_restores_the_render_id() {
        let mut frame = NodeFrameState::default();
        let outer = frame.begin_render(0.0);
        let outer_id = frame.render_id;
        let inner = frame.begin_render(0.0);
        assert_ne!(frame.render_id, outer_id);
        frame.end_render(inner, false);
        assert_eq!(frame.render_id, outer_id);
        frame.end_render(outer, true);
        assert_eq!(frame.frame_id, 1);
    }
}
