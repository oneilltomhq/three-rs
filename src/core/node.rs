//! The scene-graph half of `three.js/src/core/Object3D.js`: the parent/children
//! tree and every method that needs it.
//!
//! See `docs/scene-graph.md` for why the tree is `Rc<RefCell<Object3D>>` with a
//! `Weak` parent rather than an arena.

use std::cell::RefCell;
use std::rc::{Rc, Weak};

use crate::core::Object3D;
use crate::math::{Matrix4, Quaternion, Vector3};

/// One node of the scene graph: three.js' `Object3D` *reference*. Every method
/// on [`Object3DNode`] is the port of a three.js `Object3D` method that reads or
/// writes `parent`/`children`; the transform-only methods stay inherent methods
/// on [`Object3D`] itself.
pub type Node = Rc<RefCell<Object3D>>;

/// `Object3D.parent`, stored weakly so a child holding its parent does not keep
/// the parent alive (three.js' `parent` is a strong reference, but JS has a GC
/// and we do not).
pub type WeakNode = Weak<RefCell<Object3D>>;

/// The scene-graph methods of `Object3D`, on a [`Node`].
///
/// They live on `Rc<RefCell<Object3D>>` rather than on `&mut Object3D` because
/// every one of them needs to reach outside the object it is called on — up to
/// the parent or down into the children — which a `&mut Object3D` cannot do.
pub trait Object3DNode {
    /// `Object3D.parent`, upgraded.
    fn parent(&self) -> Option<Node>;
    /// `Object3D.children`, cloned (cheap: a `Vec` of `Rc`s). Cloning is what
    /// lets a traversal run without holding a borrow on the parent.
    fn children(&self) -> Vec<Node>;
    /// `Object3D.add( object )`.
    fn add(&self, object: &Node) -> &Self;
    /// `Object3D.remove( object )`.
    fn remove(&self, object: &Node) -> &Self;
    /// `Object3D.removeFromParent()`.
    fn remove_from_parent(&self) -> &Self;
    /// `Object3D.clear()`.
    fn clear(&self) -> &Self;
    /// `Object3D.attach( object )`.
    fn attach(&self, object: &Node) -> &Self;
    /// `Object3D.getObjectById( id )`.
    fn get_object_by_id(&self, id: u32) -> Option<Node>;
    /// `Object3D.getObjectByName( name )`.
    fn get_object_by_name(&self, name: &str) -> Option<Node>;
    /// `Object3D.getObjectByProperty( name, value )`. Rust has no dynamic
    /// property lookup, so the property test is a predicate.
    fn get_object_by_property(&self, test: &dyn Fn(&Object3D) -> bool) -> Option<Node>;
    /// `Object3D.getObjectsByProperty( name, value, result )`.
    fn get_objects_by_property(&self, test: &dyn Fn(&Object3D) -> bool) -> Vec<Node>;
    /// `Object3D.traverse( callback )`.
    fn traverse(&self, callback: &mut dyn FnMut(&Node));
    /// `Object3D.traverseVisible( callback )`.
    fn traverse_visible(&self, callback: &mut dyn FnMut(&Node));
    /// `Object3D.traverseAncestors( callback )`.
    fn traverse_ancestors(&self, callback: &mut dyn FnMut(&Node));
    /// `Object3D.updateMatrixWorld( force )`.
    fn update_matrix_world(&self, force: bool);
    /// `Object3D.updateWorldMatrix( updateParents, updateChildren )`.
    fn update_world_matrix(&self, update_parents: bool, update_children: bool);
    /// `Object3D.getWorldPosition( target )`.
    fn get_world_position(&self) -> Vector3;
    /// `Object3D.getWorldQuaternion( target )`.
    fn get_world_quaternion(&self) -> Quaternion;
    /// `Object3D.getWorldScale( target )`.
    fn get_world_scale(&self) -> Vector3;
    /// `Object3D.getWorldDirection( target )`.
    fn get_world_direction(&self) -> Vector3;
    /// `Object3D.localToWorld( vector )`.
    fn local_to_world(&self, vector: &mut Vector3);
    /// `Object3D.worldToLocal( vector )`.
    fn world_to_local(&self, vector: &mut Vector3);
    /// `Object3D.lookAt( vector )`, parent rotation included.
    fn look_at(&self, target: &Vector3);
    /// `Object3D.applyMatrix4( matrix )` — the inherent method, reached through
    /// the cell so `attach()` can use it.
    fn apply_matrix4(&self, m: &Matrix4);
}

/// The `updateWorldMatrix`/`updateMatrixWorld` body both methods share: compose
/// the local matrix, then multiply the parent's world matrix into it. Returns
/// the new `force` for the children.
fn update_own_matrix_world(node: &Node, force: bool) -> bool {
    if node.borrow().matrix_auto_update {
        node.borrow_mut().update_matrix();
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

        return true;
    }

    force
}

impl Object3DNode for Node {
    fn parent(&self) -> Option<Node> {
        self.borrow().parent.as_ref().and_then(Weak::upgrade)
    }

    fn children(&self) -> Vec<Node> {
        self.borrow().children.clone()
    }

    fn add(&self, object: &Node) -> &Self {
        // `Object3D.add: object can't be added as a child of itself.`
        if Rc::ptr_eq(self, object) {
            return self;
        }

        object.remove_from_parent();
        object.borrow_mut().parent = Some(Rc::downgrade(self));
        self.borrow_mut().children.push(object.clone());

        self
    }

    fn remove(&self, object: &Node) -> &Self {
        let index = self
            .borrow()
            .children
            .iter()
            .position(|child| Rc::ptr_eq(child, object));

        if let Some(index) = index {
            object.borrow_mut().parent = None;
            self.borrow_mut().children.remove(index);
        }

        self
    }

    fn remove_from_parent(&self) -> &Self {
        if let Some(parent) = self.parent() {
            parent.remove(self);
        }

        self
    }

    fn clear(&self) -> &Self {
        for child in self.children() {
            self.remove(&child);
        }

        self
    }

    fn attach(&self, object: &Node) -> &Self {
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
        object.borrow_mut().parent = Some(Rc::downgrade(self));
        self.borrow_mut().children.push(object.clone());

        object.update_world_matrix(false, true);

        self
    }

    fn get_object_by_id(&self, id: u32) -> Option<Node> {
        self.get_object_by_property(&|object| object.id == id)
    }

    fn get_object_by_name(&self, name: &str) -> Option<Node> {
        self.get_object_by_property(&|object| object.name == name)
    }

    fn get_object_by_property(&self, test: &dyn Fn(&Object3D) -> bool) -> Option<Node> {
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

    fn get_objects_by_property(&self, test: &dyn Fn(&Object3D) -> bool) -> Vec<Node> {
        let mut result = Vec::new();

        if test(&self.borrow()) {
            result.push(self.clone());
        }

        for child in self.children() {
            result.extend(child.get_objects_by_property(test));
        }

        result
    }

    fn traverse(&self, callback: &mut dyn FnMut(&Node)) {
        callback(self);

        for child in self.children() {
            child.traverse(callback);
        }
    }

    fn traverse_visible(&self, callback: &mut dyn FnMut(&Node)) {
        if !self.borrow().visible {
            return;
        }

        callback(self);

        for child in self.children() {
            child.traverse_visible(callback);
        }
    }

    fn traverse_ancestors(&self, callback: &mut dyn FnMut(&Node)) {
        if let Some(parent) = self.parent() {
            callback(&parent);
            parent.traverse_ancestors(callback);
        }
    }

    fn update_matrix_world(&self, force: bool) {
        let force = update_own_matrix_world(self, force);

        // make sure descendants are updated if required
        for child in self.children() {
            child.update_matrix_world(force);
        }
    }

    fn update_world_matrix(&self, update_parents: bool, update_children: bool) {
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

    fn get_world_position(&self) -> Vector3 {
        self.update_world_matrix(true, false);

        let mut target = Vector3::ZERO;
        target.set_from_matrix_position(&self.borrow().matrix_world);
        target
    }

    fn get_world_quaternion(&self) -> Quaternion {
        self.update_world_matrix(true, false);

        let mut position = Vector3::ZERO;
        let mut target = Quaternion::default();
        let mut scale = Vector3::ZERO;
        self.borrow()
            .matrix_world
            .decompose(&mut position, &mut target, &mut scale);
        target
    }

    fn get_world_scale(&self) -> Vector3 {
        self.update_world_matrix(true, false);

        let mut position = Vector3::ZERO;
        let mut quaternion = Quaternion::default();
        let mut target = Vector3::ZERO;
        self.borrow()
            .matrix_world
            .decompose(&mut position, &mut quaternion, &mut target);
        target
    }

    fn get_world_direction(&self) -> Vector3 {
        self.update_world_matrix(true, false);

        let object = self.borrow();
        let e = &object.matrix_world.elements;
        let mut target = Vector3::new(e[8], e[9], e[10]);
        target.normalize();
        target
    }

    fn local_to_world(&self, vector: &mut Vector3) {
        self.update_world_matrix(true, false);
        vector.apply_matrix4(&self.borrow().matrix_world);
    }

    fn world_to_local(&self, vector: &mut Vector3) {
        self.update_world_matrix(true, false);

        let mut m1 = self.borrow().matrix_world;
        m1.invert();
        vector.apply_matrix4(&m1);
    }

    fn look_at(&self, target: &Vector3) {
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

    fn apply_matrix4(&self, m: &Matrix4) {
        self.borrow_mut().apply_matrix4(m);
    }
}

/// `updateWorldMatrix( false, true, force )` — the recursive child half, which
/// differs from the public method only in that it threads `force` through.
fn update_children_world_matrix(node: &Node, force: bool) {
    let force = update_own_matrix_world(node, force);

    for child in node.children() {
        update_children_world_matrix(&child, force);
    }
}
