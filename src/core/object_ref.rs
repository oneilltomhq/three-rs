//! The scene-graph half of `three.js/src/core/Object3D.js`: the parent/children
//! tree and every method that needs it.
//!
//! See `docs/scene-graph.md` for why the tree is `Rc<RefCell<Object3D>>` with a
//! `Weak` parent rather than an arena.

use std::cell::RefCell;
use std::ops::Deref;
use std::rc::{Rc, Weak};

use crate::core::{ListenerHandle, Object3D, SceneEvent, SceneEventType};
use crate::math::{Matrix4, Quaternion, Vector3};

/// One node of the scene graph: three.js' `Object3D` *reference*, a newtype over
/// `Rc<RefCell<Object3D>>`.
///
/// Every inherent method here is the port of a three.js `Object3D` method that
/// reads or writes `parent`/`children`; the transform-only methods stay inherent
/// methods on [`Object3D`] itself. They live on the handle rather than on
/// `&mut Object3D` because every one of them needs to reach outside the object
/// it is called on — up to the parent or down into the children — which a
/// `&mut Object3D` cannot do.
///
/// `ObjectRef` derefs to `RefCell<Object3D>`, so `node.borrow()` and
/// `node.borrow_mut()` reach the object exactly as they did when `ObjectRef` was a
/// type alias.
#[derive(Clone)]
pub struct ObjectRef(Rc<RefCell<Object3D>>);

/// `Object3D.parent`, stored weakly so a child holding its parent does not keep
/// the parent alive (three.js' `parent` is a strong reference, but JS has a GC
/// and we do not).
#[derive(Clone, Default)]
pub struct WeakObjectRef(Weak<RefCell<Object3D>>);

impl Deref for ObjectRef {
    type Target = RefCell<Object3D>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl std::fmt::Debug for ObjectRef {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl std::fmt::Debug for WeakObjectRef {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl WeakObjectRef {
    /// `Weak::new()` — a `WeakObjectRef` that never upgrades.
    pub fn new() -> Self {
        Self(Weak::new())
    }

    /// The node, if it is still alive.
    pub fn upgrade(&self) -> Option<ObjectRef> {
        self.0.upgrade().map(ObjectRef)
    }
}

impl ObjectRef {
    /// `object`, moved into a fresh scene-graph node.
    pub fn new(object: Object3D) -> Self {
        Self(Rc::new(RefCell::new(object)))
    }

    /// `a === b`: whether two handles are the same object. Was `Rc::ptr_eq`.
    pub fn ptr_eq(a: &ObjectRef, b: &ObjectRef) -> bool {
        Rc::ptr_eq(&a.0, &b.0)
    }

    /// The weak handle `Object3D.parent` holds.
    pub fn downgrade(&self) -> WeakObjectRef {
        WeakObjectRef(Rc::downgrade(&self.0))
    }

    /// The address of the object, for a consumer keying a cache on identity.
    /// Note that an address is only unique among *live* nodes — see
    /// `docs/scene-graph.md`, "Identity and eviction"; `object.id` is the
    /// collision-free key the renderer itself uses.
    pub fn as_ptr(&self) -> *const RefCell<Object3D> {
        Rc::as_ptr(&self.0)
    }

    /// `Object3D.parent`, upgraded.
    pub fn parent(&self) -> Option<ObjectRef> {
        self.borrow()
            .parent
            .as_ref()
            .and_then(WeakObjectRef::upgrade)
    }

    /// `Object3D.children`, cloned (cheap: a `Vec` of `Rc`s). Cloning is what
    /// lets a traversal run without holding a borrow on the parent.
    pub fn children(&self) -> Vec<ObjectRef> {
        self.borrow().children.clone()
    }

    /// `Object3D.add( object )`.
    pub fn add(&self, object: &ObjectRef) -> &Self {
        // `Object3D.add: object can't be added as a child of itself.`
        if ObjectRef::ptr_eq(self, object) {
            return self;
        }

        object.remove_from_parent();
        object.borrow_mut().parent = Some(self.downgrade());
        self.borrow_mut().children.push(object.clone());

        object.dispatch_event(&SceneEvent::Added);
        self.dispatch_event(&SceneEvent::ChildAdded(object.clone()));

        self
    }

    /// `Object3D.remove( object )`.
    pub fn remove(&self, object: &ObjectRef) -> &Self {
        let index = self
            .borrow()
            .children
            .iter()
            .position(|child| ObjectRef::ptr_eq(child, object));

        if let Some(index) = index {
            object.borrow_mut().parent = None;
            self.borrow_mut().children.remove(index);

            object.dispatch_event(&SceneEvent::Removed);
            self.dispatch_event(&SceneEvent::ChildRemoved(object.clone()));
        }

        self
    }

    /// `Object3D.removeFromParent()`.
    pub fn remove_from_parent(&self) -> &Self {
        if let Some(parent) = self.parent() {
            parent.remove(self);
        }

        self
    }

    /// `Object3D.clear()`.
    pub fn clear(&self) -> &Self {
        for child in self.children() {
            self.remove(&child);
        }

        self
    }

    /// `Object3D.attach( object )`.
    pub fn attach(&self, object: &ObjectRef) -> &Self {
        // adds object as a child of this, while maintaining the object's world
        // transform
        self.update_world_matrix(true, false);

        let mut m1 = self.borrow().matrix_world;
        m1.invert();

        if let Some(object_parent) = object.parent() {
            object_parent.update_world_matrix(true, false);
            let parent_matrix_world = object_parent.borrow().matrix_world;
            m1.multiply(&parent_matrix_world);
        }

        object.apply_matrix4(&m1);

        object.remove_from_parent();
        object.borrow_mut().parent = Some(self.downgrade());
        self.borrow_mut().children.push(object.clone());

        object.update_world_matrix(false, true);

        object.dispatch_event(&SceneEvent::Added);
        self.dispatch_event(&SceneEvent::ChildAdded(object.clone()));

        self
    }

    /// `EventDispatcher.addEventListener( type, listener )`: `listener` is
    /// called with every event of type `ty` dispatched on this object, and
    /// with the object itself (three's `event.target`). The returned handle
    /// stands for the listener in [`remove_event_listener`] and
    /// [`has_event_listener`].
    ///
    /// [`remove_event_listener`]: Self::remove_event_listener
    /// [`has_event_listener`]: Self::has_event_listener
    pub fn add_event_listener(
        &self,
        ty: SceneEventType,
        listener: impl Fn(&SceneEvent, &ObjectRef) + 'static,
    ) -> ListenerHandle {
        self.borrow_mut().listeners.add(ty, Rc::new(listener))
    }

    /// `EventDispatcher.hasEventListener( type, listener )`.
    pub fn has_event_listener(&self, ty: SceneEventType, handle: ListenerHandle) -> bool {
        self.borrow().listeners.has(ty, handle)
    }

    /// `EventDispatcher.removeEventListener( type, listener )`. A handle that
    /// is not registered for `ty` is ignored, as three ignores an unknown
    /// listener. Safe to call from inside a listener, on itself or any other:
    /// a dispatch already under way still calls every listener it started
    /// with.
    pub fn remove_event_listener(&self, ty: SceneEventType, handle: ListenerHandle) {
        self.borrow_mut().listeners.remove(ty, handle);
    }

    /// `EventDispatcher.dispatchEvent( event )`: calls every listener of the
    /// event's type, in the order they were added, with this node as the
    /// target. The list is copied first and no borrow of the object is held
    /// during the calls, so a listener may borrow the node, change the tree
    /// (which dispatches again) or add and remove listeners.
    pub fn dispatch_event(&self, event: &SceneEvent) {
        let listeners = {
            let object = self.borrow();
            if object.listeners.is_empty() {
                return;
            }
            object.listeners.snapshot(event.event_type())
        };
        for listener in listeners {
            listener(event, self);
        }
    }

    /// `Object3D.getObjectById( id )`.
    pub fn get_object_by_id(&self, id: u32) -> Option<ObjectRef> {
        self.get_object_by_property(&|object| object.id == id)
    }

    /// `Object3D.getObjectByName( name )`.
    pub fn get_object_by_name(&self, name: &str) -> Option<ObjectRef> {
        self.get_object_by_property(&|object| object.name == name)
    }

    /// `Object3D.getObjectByProperty( name, value )`. Rust has no dynamic
    /// property lookup, so the property test is a predicate.
    pub fn get_object_by_property(&self, test: &dyn Fn(&Object3D) -> bool) -> Option<ObjectRef> {
        if test(&self.borrow()) {
            return Some(self.clone());
        }

        for child in self.children() {
            if let Some(object) = child.get_object_by_property(test) {
                return Some(object);
            }
        }

        None
    }

    /// `Object3D.getObjectsByProperty( name, value, result )`.
    pub fn get_objects_by_property(&self, test: &dyn Fn(&Object3D) -> bool) -> Vec<ObjectRef> {
        let mut result = Vec::new();

        if test(&self.borrow()) {
            result.push(self.clone());
        }

        for child in self.children() {
            result.extend(child.get_objects_by_property(test));
        }

        result
    }

    /// `Object3D.traverse( callback )`.
    pub fn traverse(&self, callback: &mut dyn FnMut(&ObjectRef)) {
        callback(self);

        for child in self.children() {
            child.traverse(callback);
        }
    }

    /// `Object3D.traverseVisible( callback )`.
    pub fn traverse_visible(&self, callback: &mut dyn FnMut(&ObjectRef)) {
        if !self.borrow().visible {
            return;
        }

        callback(self);

        for child in self.children() {
            child.traverse_visible(callback);
        }
    }

    /// `Object3D.traverseAncestors( callback )`.
    pub fn traverse_ancestors(&self, callback: &mut dyn FnMut(&ObjectRef)) {
        if let Some(parent) = self.parent() {
            callback(&parent);
            parent.traverse_ancestors(callback);
        }
    }

    /// `Object3D.updateMatrixWorld( force )`.
    pub fn update_matrix_world(&self, force: bool) {
        let force = update_own_matrix_world(self, force);

        // make sure descendants are updated if required
        for child in self.children() {
            child.update_matrix_world(force);
        }
    }

    /// `Object3D.updateWorldMatrix( updateParents, updateChildren )`.
    pub fn update_world_matrix(&self, update_parents: bool, update_children: bool) {
        if update_parents {
            if let Some(parent) = self.parent() {
                parent.update_world_matrix(true, false);
            }
        }

        let force = update_own_matrix_world(self, false);

        // make sure descendants are updated
        if update_children {
            for child in self.children() {
                update_children_world_matrix(&child, force);
            }
        }
    }

    /// `Object3D.getWorldPosition( target )`.
    pub fn get_world_position(&self) -> Vector3 {
        self.update_world_matrix(true, false);

        let mut target = Vector3::ZERO;
        target.set_from_matrix_position(&self.borrow().matrix_world);
        target
    }

    /// `Object3D.getWorldQuaternion( target )`.
    pub fn get_world_quaternion(&self) -> Quaternion {
        self.update_world_matrix(true, false);

        let mut position = Vector3::ZERO;
        let mut target = Quaternion::default();
        let mut scale = Vector3::ZERO;
        self.borrow()
            .matrix_world
            .decompose(&mut position, &mut target, &mut scale);
        target
    }

    /// `Object3D.getWorldScale( target )`.
    pub fn get_world_scale(&self) -> Vector3 {
        self.update_world_matrix(true, false);

        let mut position = Vector3::ZERO;
        let mut quaternion = Quaternion::default();
        let mut target = Vector3::ZERO;
        self.borrow()
            .matrix_world
            .decompose(&mut position, &mut quaternion, &mut target);
        target
    }

    /// `Object3D.getWorldDirection( target )`.
    pub fn get_world_direction(&self) -> Vector3 {
        self.update_world_matrix(true, false);

        let object = self.borrow();
        let e = &object.matrix_world.elements;
        let mut target = Vector3::new(e[8], e[9], e[10]);
        target.normalize();
        target
    }

    /// `Object3D.localToWorld( vector )`.
    pub fn local_to_world(&self, vector: &mut Vector3) {
        self.update_world_matrix(true, false);
        vector.apply_matrix4(&self.borrow().matrix_world);
    }

    /// `Object3D.worldToLocal( vector )`.
    pub fn world_to_local(&self, vector: &mut Vector3) {
        self.update_world_matrix(true, false);

        let mut m1 = self.borrow().matrix_world;
        m1.invert();
        vector.apply_matrix4(&m1);
    }

    /// `Object3D.lookAt( vector )`, parent rotation included.
    pub fn look_at(&self, target: &Vector3) {
        // This method does not support objects having non-uniformly-scaled
        // parent(s)
        let parent = self.parent();

        self.update_world_matrix(true, false);

        let mut position = Vector3::ZERO;
        position.set_from_matrix_position(&self.borrow().matrix_world);

        let mut m1 = Matrix4::identity();
        let up = self.borrow().up;

        if self.borrow().is_camera || self.borrow().is_light {
            m1.look_at(&position, target, &up);
        } else {
            m1.look_at(target, &position, &up);
        }

        self.borrow_mut().quaternion.set_from_rotation_matrix(&m1);

        if let Some(parent) = parent {
            m1.extract_rotation(&parent.borrow().matrix_world);
            let mut q1 = Quaternion::default();
            q1.set_from_rotation_matrix(&m1);
            q1.invert();
            self.borrow_mut().quaternion.premultiply(&q1);
        }

        self.borrow_mut().sync_rotation_from_quaternion();
    }

    /// `Object3D.applyMatrix4( matrix )` — the inherent method, reached through
    /// the cell so `attach()` can use it.
    pub fn apply_matrix4(&self, m: &Matrix4) {
        self.borrow_mut().apply_matrix4(m);
    }
}

/// The `updateWorldMatrix`/`updateMatrixWorld` body both methods share: compose
/// the local matrix, then multiply the parent's world matrix into it. Returns
/// the new `force` for the children.
fn update_own_matrix_world(node: &ObjectRef, force: bool) -> bool {
    if node.borrow().matrix_auto_update {
        node.borrow_mut().update_matrix();
    }

    // `this.matrix` *is* another object's `matrixWorld` (`CameraHelper`):
    // read it now, as the multiply below would read the shared object.
    let alias = node
        .borrow()
        .matrix_alias
        .as_ref()
        .and_then(WeakObjectRef::upgrade);
    if let Some(source) = alias {
        let matrix_world = source.borrow().matrix_world;
        node.borrow_mut().matrix = matrix_world;
    }

    let (needs_update, world_auto_update) = {
        let object = node.borrow();
        (
            object.matrix_world_needs_update,
            object.matrix_world_auto_update,
        )
    };

    if needs_update || force {
        if world_auto_update {
            let parent_matrix_world = node.parent().map(|parent| parent.borrow().matrix_world);
            let mut object = node.borrow_mut();
            match parent_matrix_world {
                None => object.matrix_world = object.matrix,
                Some(parent) => {
                    let matrix = object.matrix;
                    object.matrix_world.multiply_matrices(&parent, &matrix);
                }
            }
        }

        node.borrow_mut().matrix_world_needs_update = false;

        update_bind_matrix_inverse(node);

        return true;
    }

    // `SkinnedMesh.updateMatrixWorld()` calls `super.updateMatrixWorld()` and
    // then always recomputes `bindMatrixInverse`, whether or not the world
    // matrix was rebuilt.
    update_bind_matrix_inverse(node);

    force
}

/// The tail of `SkinnedMesh.updateMatrixWorld()`, for whichever nodes carry a
/// `SkinnedMesh` payload. In three.js this is an override on the subclass; here
/// the subclass is a payload variant, so the one tree walk runs it.
///
/// It matters that it runs *every* frame: in the default `AttachedBindMode`
/// `bindMatrixInverse` is `matrixWorld⁻¹`, and `GLTFLoader` binds with the
/// identity bind matrix, so everything the skin's own node contributes — a
/// glTF `Character` node's 0.01 scale and ±90° rotation, say — lives there and
/// nowhere else.
fn update_bind_matrix_inverse(node: &ObjectRef) {
    let mut object = node.borrow_mut();
    if !object.payload.is_skinned_mesh() {
        return;
    }
    let matrix_world = object.matrix_world;
    if let Some(skin) = object.payload.skinned_mesh_mut() {
        skin.update_bind_matrix_inverse(&matrix_world);
    }
}

/// `updateWorldMatrix( false, true, force )` — the recursive child half, which
/// differs from the public method only in that it threads `force` through.
fn update_children_world_matrix(node: &ObjectRef, force: bool) {
    let force = update_own_matrix_world(node, force);

    for child in node.children() {
        update_children_world_matrix(&child, force);
    }
}
