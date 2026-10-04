//! The scene graph's events and render callbacks: three.js'
//! `EventDispatcher` as it is used on an `Object3D`, and the
//! `onBeforeRender` / `onAfterRender` callbacks.
//!
//! three.js dispatches by string (`'added'`, `'childadded'`, ...) with an
//! untyped event object. The port has a closed [`SceneEvent`] enum instead,
//! because the four events an `Object3D` dispatches are the whole set and
//! each carries known data; `docs/api.md` decision 12 says why there is no
//! string-keyed dispatcher. The listener list lives on the [`Object3D`] and
//! the methods that add, remove and dispatch are on [`Node`], next to the
//! tree methods that dispatch.

use std::cell::Cell;
use std::fmt;
use std::rc::Rc;

use crate::cameras::RenderCamera;
use crate::core::{Group, Node};
use crate::objects::Scene;
use crate::renderer::{RenderTarget, Renderer};

#[cfg(doc)]
use crate::core::Object3D;

/// An event an `Object3D` dispatches to its listeners, the `event` argument
/// of three.js' `dispatchEvent( event )`.
///
/// The set is closed: these are the four events three's `Object3D` dispatches
/// from `add()`, `remove()` and `attach()`. three's `event.target` is the
/// second argument every listener receives, not a field here.
#[derive(Clone, Debug)]
pub enum SceneEvent {
    /// `{ type: 'added' }` — dispatched on an object once it has a new parent.
    Added,
    /// `{ type: 'removed' }` — dispatched on an object once it has lost its
    /// parent.
    Removed,
    /// `{ type: 'childadded', child }` — dispatched on the parent after
    /// [`Added`](Self::Added) has been dispatched on the child.
    ChildAdded(Node),
    /// `{ type: 'childremoved', child }` — dispatched on the old parent after
    /// [`Removed`](Self::Removed) has been dispatched on the child.
    ChildRemoved(Node),
}

/// The `type` string of a [`SceneEvent`], which is what a listener is
/// registered for: three's `addEventListener( 'added', ... )` is
/// `add_event_listener(SceneEventType::Added, ...)`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SceneEventType {
    /// `'added'`.
    Added,
    /// `'removed'`.
    Removed,
    /// `'childadded'`.
    ChildAdded,
    /// `'childremoved'`.
    ChildRemoved,
}

impl SceneEvent {
    /// `event.type`.
    pub fn event_type(&self) -> SceneEventType {
        match self {
            Self::Added => SceneEventType::Added,
            Self::Removed => SceneEventType::Removed,
            Self::ChildAdded(_) => SceneEventType::ChildAdded,
            Self::ChildRemoved(_) => SceneEventType::ChildRemoved,
        }
    }

    /// `event.child` — the child of a [`ChildAdded`](Self::ChildAdded) or
    /// [`ChildRemoved`](Self::ChildRemoved) event; `None` for the others.
    pub fn child(&self) -> Option<&Node> {
        match self {
            Self::ChildAdded(child) | Self::ChildRemoved(child) => Some(child),
            Self::Added | Self::Removed => None,
        }
    }
}

/// What [`Node::add_event_listener`] returns, and what
/// [`Node::remove_event_listener`] and [`Node::has_event_listener`] take in
/// place of three's listener function.
///
/// A JS function has an identity a Rust closure lacks, so the handle stands
/// for it. Every handle is distinct, which is also why three's "can't add one
/// listener twice" check has nothing to compare: a closure is moved into the
/// list and cannot be added a second time. Opaque, per decision 11 of
/// `docs/api.md`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ListenerHandle(u64);

impl ListenerHandle {
    fn next() -> Self {
        thread_local! {
            static NEXT: Cell<u64> = const { Cell::new(0) };
        }
        NEXT.with(|next| {
            let value = next.get();
            next.set(value + 1);
            Self(value)
        })
    }
}

/// A listener: called with the event and three's `event.target`, the node the
/// event was dispatched on.
///
/// `Fn` rather than `FnMut` because dispatch is re-entrant: a listener that
/// adds a child dispatches again from inside its own call, which may reach
/// the same listener. Keep mutable state in a `Cell` or `RefCell`. Getting the
/// target as an argument, rather than capturing the node, is what keeps a
/// listener from holding its own node alive.
pub type SceneEventListener = Rc<dyn Fn(&SceneEvent, &Node)>;

/// three's `object._listeners`: the listeners of one object, in the order
/// they were added. Opaque — it is a public field of [`Object3D`] only so
/// `Object3D { .., ..Default::default() }` keeps working outside the crate;
/// the methods that use it are on [`Node`].
#[derive(Clone, Default)]
pub struct SceneEventListeners {
    entries: Vec<(SceneEventType, ListenerHandle, SceneEventListener)>,
}

impl fmt::Debug for SceneEventListeners {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_list()
            .entries(self.entries.iter().map(|(ty, handle, _)| (ty, handle)))
            .finish()
    }
}

impl SceneEventListeners {
    pub(crate) fn add(
        &mut self,
        ty: SceneEventType,
        listener: SceneEventListener,
    ) -> ListenerHandle {
        let handle = ListenerHandle::next();
        self.entries.push((ty, handle, listener));
        handle
    }

    pub(crate) fn has(&self, ty: SceneEventType, handle: ListenerHandle) -> bool {
        self.entries
            .iter()
            .any(|(t, h, _)| *t == ty && *h == handle)
    }

    pub(crate) fn remove(&mut self, ty: SceneEventType, handle: ListenerHandle) {
        if let Some(index) = self
            .entries
            .iter()
            .position(|(t, h, _)| *t == ty && *h == handle)
        {
            self.entries.remove(index);
        }
    }

    /// `listenerArray.slice( 0 )`: the listeners of `ty`, copied out so the
    /// caller can drop its borrow of the object before calling them.
    pub(crate) fn snapshot(&self, ty: SceneEventType) -> Vec<SceneEventListener> {
        self.entries
            .iter()
            .filter(|(t, _, _)| *t == ty)
            .map(|(_, _, listener)| listener.clone())
            .collect()
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

/// `object.onBeforeRender` / `object.onAfterRender`, as a boxed closure in an
/// `Option` field of [`Object3D`].
///
/// three passes `( renderer, scene, camera, geometry, material, group )` with
/// the object as `this`. Here the first argument is the object's own [`Node`],
/// not borrowed, so the hook can borrow it (or any other node) as it likes;
/// the geometry and material are on its payload. The renderer is shared:
/// the hook runs inside a render, and the port records every draw of a pass
/// after all of its before-hooks have run, so a hook cannot draw. `group` is
/// the `geometry.groups` entry the draw uses, `None` for a single-material
/// object. See `docs/api.md` decision 12.
///
/// The renderer takes the hook out of the field for the call and puts it back
/// afterwards unless the hook stored a new one there, so a hook can replace
/// itself but not clear itself: clearing the field from inside the call is
/// undone. Clear it outside a render instead.
pub type ObjectRenderHook =
    Box<dyn FnMut(&Node, &Renderer, &Scene, &dyn RenderCamera, Option<&Group>)>;

/// `scene.onBeforeRender` / `scene.onAfterRender`, as a boxed closure in an
/// `Option` field of [`Scene`].
///
/// three passes `( renderer, scene, camera, renderTarget )`, where
/// `renderTarget` is the target the scene is drawn into: the internal
/// framebuffer target when one is used, else the renderer's render target, so
/// `None` means the canvas. `Fn`, because the renderer reaches the scene
/// shared — a pass or a reflector renders it again from inside its own render
/// — so keep mutable state in a `Cell` or `RefCell`.
pub type SceneRenderHook = Box<dyn Fn(&Renderer, &Scene, &dyn RenderCamera, Option<&RenderTarget>)>;
