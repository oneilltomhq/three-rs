//! Port of `three.js/test/unit/src/core/EventDispatcher.tests.js`, onto the
//! typed [`SceneEvent`] the scene graph dispatches (`docs/api.md` decision
//! 12).
//!
//! three's tests use an `'anyType'` string and a bare `EventDispatcher`; the
//! port's dispatcher is the [`ObjectRef`] and its event types are closed, so they
//! use `SceneEventType::Added`, and `dispatch_event` is called directly with
//! no tree change behind it. The listener `{}` three passes stands for a
//! function's identity, which is the [`ListenerHandle`] here. Past the four
//! QUnit cases are the copy-then-call cases three's `dispatchEvent()`
//! comments on ("Make a copy, in case listeners are removed while
//! iterating"), which it has no test for.
//!
//! Skipped: `Instancing` (there is no separate dispatcher to make), and the
//! `_listeners` shape checks (`_listeners === undefined` by default, an empty
//! array left behind once the last listener goes), which inspect a private
//! field.

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use three_rs::core::{ListenerHandle, Object3D, ObjectRef, SceneEvent, SceneEventType};

const ANY_TYPE: SceneEventType = SceneEventType::Added;
const ANOTHER_TYPE: SceneEventType = SceneEventType::Removed;

fn counter() -> (Rc<Cell<u32>>, impl Fn(&SceneEvent, &ObjectRef) + 'static) {
    let count = Rc::new(Cell::new(0));
    let inner = count.clone();
    (count, move |_: &SceneEvent, _: &ObjectRef| {
        inner.set(inner.get() + 1)
    })
}

#[test]
fn add_event_listener() {
    let event_dispatcher = Object3D::new_node();

    let (_, listener) = counter();
    let handle = event_dispatcher.add_event_listener(ANY_TYPE, listener);
    assert!(
        event_dispatcher.has_event_listener(ANY_TYPE, handle),
        "listener with unknown type was added"
    );

    // "can't add one listener twice to same type": a closure is moved in, so
    // the same one cannot be added again; a second closure is a second
    // listener with its own handle.
    let (_, other) = counter();
    let second = event_dispatcher.add_event_listener(ANY_TYPE, other);
    assert_ne!(handle, second, "every listener has its own handle");
    assert!(
        event_dispatcher.has_event_listener(ANY_TYPE, handle),
        "listener is still there"
    );
}

#[test]
fn has_event_listener() {
    let event_dispatcher = Object3D::new_node();

    let (_, listener) = counter();
    let handle = event_dispatcher.add_event_listener(ANY_TYPE, listener);

    assert!(
        event_dispatcher.has_event_listener(ANY_TYPE, handle),
        "listener was found"
    );
    assert!(
        !event_dispatcher.has_event_listener(ANOTHER_TYPE, handle),
        "listener was not found which is good"
    );
}

#[test]
fn remove_event_listener() {
    let event_dispatcher = Object3D::new_node();

    let (count, listener) = counter();
    let handle = event_dispatcher.add_event_listener(ANY_TYPE, listener);
    assert!(
        event_dispatcher.has_event_listener(ANY_TYPE, handle),
        "if a listener was added, it is there"
    );

    event_dispatcher.remove_event_listener(ANY_TYPE, handle);
    assert!(
        !event_dispatcher.has_event_listener(ANY_TYPE, handle),
        "listener was deleted"
    );
    event_dispatcher.dispatch_event(&SceneEvent::Added);
    assert_eq!(count.get(), 0, "a deleted listener is not called");

    // "unknown types will be ignored"
    let (_, listener) = counter();
    let handle = event_dispatcher.add_event_listener(ANY_TYPE, listener);
    event_dispatcher.remove_event_listener(ANOTHER_TYPE, handle);
    assert!(
        event_dispatcher.has_event_listener(ANY_TYPE, handle),
        "removing under another type leaves the listener alone"
    );

    // "undefined listeners are ignored": the nearest thing is a handle that
    // is no longer registered.
    event_dispatcher.remove_event_listener(ANY_TYPE, handle);
    event_dispatcher.remove_event_listener(ANY_TYPE, handle);
    assert!(
        !event_dispatcher.has_event_listener(ANY_TYPE, handle),
        "removing twice is harmless"
    );
}

#[test]
fn dispatch_event() {
    let event_dispatcher = Object3D::new_node();

    let (call_count, listener) = counter();
    event_dispatcher.add_event_listener(ANY_TYPE, listener);
    assert_eq!(call_count.get(), 0, "no event, no call");

    event_dispatcher.dispatch_event(&SceneEvent::Added);
    assert_eq!(call_count.get(), 1, "one event, one call");

    event_dispatcher.dispatch_event(&SceneEvent::Added);
    assert_eq!(call_count.get(), 2, "two events, two calls");

    event_dispatcher.dispatch_event(&SceneEvent::Removed);
    assert_eq!(call_count.get(), 2, "an event of another type, no call");
}

#[test]
fn dispatch_event_passes_the_event_and_its_target() {
    let event_dispatcher = Object3D::new_node();
    let child = Object3D::new_node();

    /// `event.type`, `event.child` and `event.target`.
    type Seen = (SceneEventType, Option<ObjectRef>, ObjectRef);
    let seen: Rc<RefCell<Vec<Seen>>> = Rc::default();
    let log = seen.clone();
    event_dispatcher.add_event_listener(SceneEventType::ChildAdded, move |event, target| {
        log.borrow_mut()
            .push((event.event_type(), event.child().cloned(), target.clone()));
    });

    event_dispatcher.dispatch_event(&SceneEvent::ChildAdded(child.clone()));

    let seen = seen.borrow();
    assert_eq!(seen.len(), 1);
    assert_eq!(seen[0].0, SceneEventType::ChildAdded, "event.type");
    assert!(
        ObjectRef::ptr_eq(seen[0].1.as_ref().unwrap(), &child),
        "event.child"
    );
    assert!(
        ObjectRef::ptr_eq(&seen[0].2, &event_dispatcher),
        "event.target"
    );
}

#[test]
fn listeners_are_called_in_the_order_they_were_added() {
    let event_dispatcher = Object3D::new_node();
    let order: Rc<RefCell<Vec<u32>>> = Rc::default();
    for i in 0..3 {
        let order = order.clone();
        event_dispatcher.add_event_listener(ANY_TYPE, move |_, _| order.borrow_mut().push(i));
    }
    event_dispatcher.dispatch_event(&SceneEvent::Added);
    assert_eq!(*order.borrow(), vec![0, 1, 2]);
}

#[test]
fn a_listener_can_remove_itself() {
    let event_dispatcher = Object3D::new_node();

    let own: Rc<Cell<Option<ListenerHandle>>> = Rc::default();
    let calls = Rc::new(Cell::new(0));
    let (own_inner, calls_inner) = (own.clone(), calls.clone());
    let handle = event_dispatcher.add_event_listener(ANY_TYPE, move |_, target| {
        calls_inner.set(calls_inner.get() + 1);
        target.remove_event_listener(ANY_TYPE, own_inner.get().unwrap());
    });
    own.set(Some(handle));
    let (after, after_listener) = counter();
    event_dispatcher.add_event_listener(ANY_TYPE, after_listener);

    event_dispatcher.dispatch_event(&SceneEvent::Added);
    assert_eq!(calls.get(), 1, "called once");
    assert_eq!(after.get(), 1, "the listener after it is still called");
    assert!(!event_dispatcher.has_event_listener(ANY_TYPE, handle));

    event_dispatcher.dispatch_event(&SceneEvent::Added);
    assert_eq!(calls.get(), 1, "not called once it removed itself");
    assert_eq!(after.get(), 2);
}

#[test]
fn a_listener_removed_during_dispatch_is_still_called_by_that_dispatch() {
    // `listenerArray.slice( 0 )`: the dispatch calls the listeners it started
    // with.
    let event_dispatcher = Object3D::new_node();

    let victim: Rc<Cell<Option<ListenerHandle>>> = Rc::default();
    let victim_inner = victim.clone();
    event_dispatcher.add_event_listener(ANY_TYPE, move |_, target| {
        if let Some(handle) = victim_inner.get() {
            target.remove_event_listener(ANY_TYPE, handle);
        }
    });
    let (count, listener) = counter();
    victim.set(Some(
        event_dispatcher.add_event_listener(ANY_TYPE, listener),
    ));

    event_dispatcher.dispatch_event(&SceneEvent::Added);
    assert_eq!(count.get(), 1, "called by the dispatch it was removed in");

    event_dispatcher.dispatch_event(&SceneEvent::Added);
    assert_eq!(count.get(), 1, "not called by the next");
}

#[test]
fn a_listener_added_during_dispatch_waits_for_the_next() {
    let event_dispatcher = Object3D::new_node();

    let (count, listener) = counter();
    let pending = RefCell::new(Some(listener));
    event_dispatcher.add_event_listener(ANY_TYPE, move |_, target| {
        if let Some(listener) = pending.borrow_mut().take() {
            target.add_event_listener(ANY_TYPE, listener);
        }
    });

    event_dispatcher.dispatch_event(&SceneEvent::Added);
    assert_eq!(count.get(), 0, "not called by the dispatch that added it");

    event_dispatcher.dispatch_event(&SceneEvent::Added);
    assert_eq!(count.get(), 1, "called by the next");
}

#[test]
fn a_listener_can_borrow_its_target() {
    let event_dispatcher = Object3D::new_node();
    event_dispatcher.add_event_listener(ANY_TYPE, |_, target| {
        target.borrow_mut().name = "seen".to_string();
    });
    event_dispatcher.dispatch_event(&SceneEvent::Added);
    assert_eq!(event_dispatcher.borrow().name, "seen");
}
