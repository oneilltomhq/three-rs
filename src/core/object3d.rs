//! Port of `three.js/src/core/Object3D.js`. The parent/children tree and the
//! methods that need it live in [`crate::core::node`]; this file is the object
//! itself and its transform methods.

use std::cell::Cell;
use std::cell::RefCell;
use std::rc::Rc;

use crate::core::node::{Node, WeakNode};
use crate::math::{Euler, Matrix4, Quaternion, Vector3};

/// three.js' module-level `let _object3DId = 0`.
fn next_id() -> u32 {
    thread_local! {
        static OBJECT3D_ID: Cell<u32> = const { Cell::new(0) };
    }

    OBJECT3D_ID.with(|id| {
        let value = id.get();
        id.set(value + 1);
        value
    })
}

#[derive(Debug)]
pub struct Object3D {
    /// `Object3D.id` — a per-thread counter, three.js' `_object3DId ++`.
    pub id: u32,
    /// `Object3D.name`.
    pub name: String,
    /// `Object3D.type`. `'Object3D'` unless a subclass overrides it (`Group`
    /// sets `'Group'`).
    pub object_type: &'static str,
    /// `Object3D.parent`, weak: see `docs/scene-graph.md`.
    pub parent: Option<WeakNode>,
    /// `Object3D.children`.
    pub children: Vec<Node>,
    /// `Object3D.visible`.
    pub visible: bool,
    /// `Object3D.matrixAutoUpdate` (`Object3D.DEFAULT_MATRIX_AUTO_UPDATE`).
    pub matrix_auto_update: bool,
    /// `Object3D.matrixWorldAutoUpdate` (`DEFAULT_MATRIX_WORLD_AUTO_UPDATE`).
    pub matrix_world_auto_update: bool,
    /// `Object3D.matrixWorldNeedsUpdate`.
    pub matrix_world_needs_update: bool,
    /// `Object3D.isCamera` — `lookAt()` points a camera the other way round.
    pub is_camera: bool,
    /// `Object3D.isLight` — as `is_camera`.
    pub is_light: bool,
    /// `Object3D.isGroup`.
    pub is_group: bool,
    pub position: Vector3,
    /// Kept in sync with `quaternion` by [`Object3D::set_rotation`], the same way
    /// three.js' `Euler`/`Quaternion` `onChange` callbacks keep them in sync.
    pub rotation: Euler,
    pub quaternion: Quaternion,
    pub scale: Vector3,
    pub up: Vector3,
    pub matrix: Matrix4,
    pub matrix_world: Matrix4,
}

impl Default for Object3D {
    fn default() -> Self {
        Self {
            id: next_id(),
            name: String::new(),
            object_type: "Object3D",
            parent: None,
            children: Vec::new(),
            visible: true,
            matrix_auto_update: true,
            matrix_world_auto_update: true,
            matrix_world_needs_update: false,
            is_camera: false,
            is_light: false,
            is_group: false,
            position: Vector3::ZERO,
            rotation: Euler::default(),
            quaternion: Quaternion::default(),
            scale: Vector3::new(1.0, 1.0, 1.0),
            // `Object3D.DEFAULT_UP`
            up: Vector3::new(0.0, 1.0, 0.0),
            matrix: Matrix4::identity(),
            matrix_world: Matrix4::identity(),
        }
    }
}

/// `Object3D.copy( source, recursive = false )`, minus the tree: a clone gets a
/// fresh `id` and no parent or children, because a `Clone` that shared the
/// `Rc`s would give two objects the same child list.
impl Clone for Object3D {
    fn clone(&self) -> Self {
        Self {
            id: next_id(),
            name: self.name.clone(),
            object_type: self.object_type,
            parent: None,
            children: Vec::new(),
            visible: self.visible,
            matrix_auto_update: self.matrix_auto_update,
            matrix_world_auto_update: self.matrix_world_auto_update,
            matrix_world_needs_update: self.matrix_world_needs_update,
            is_camera: self.is_camera,
            is_light: self.is_light,
            is_group: self.is_group,
            position: self.position,
            rotation: self.rotation,
            quaternion: self.quaternion,
            scale: self.scale,
            up: self.up,
            matrix: self.matrix,
            matrix_world: self.matrix_world,
        }
    }
}

impl Object3D {
    /// A fresh `Object3D` as a scene-graph [`Node`].
    pub fn new_node() -> Node {
        Rc::new(RefCell::new(Self::default()))
    }

    /// This object, moved into a scene-graph [`Node`].
    pub fn into_node(self) -> Node {
        Rc::new(RefCell::new(self))
    }

    /// `object.rotation.set( x, y, z )` — the Euler `onChange` callback then
    /// refreshes the quaternion, which is what `updateMatrix()` composes from.
    pub fn set_rotation(&mut self, x: f64, y: f64, z: f64) {
        self.rotation.set(x, y, z);
        self.quaternion.set_from_euler(&self.rotation);
    }

    /// three.js' `Quaternion.onChange` callback: keeps `rotation` in step with
    /// `quaternion` (`rotation.setFromQuaternion( quaternion, rotation.order,
    /// false )`). Every method below that writes the quaternion calls this, the
    /// way three.js' property setters do.
    pub(crate) fn sync_rotation_from_quaternion(&mut self) {
        let (q, order) = (self.quaternion, self.rotation.order);
        self.rotation.set_from_quaternion(&q, order);
    }

    /// `Object3D.applyMatrix4()`.
    pub fn apply_matrix4(&mut self, m: &Matrix4) {
        self.update_matrix();

        self.matrix.premultiply(m);

        let matrix = self.matrix;
        let (mut position, mut quaternion, mut scale) =
            (self.position, self.quaternion, self.scale);
        matrix.decompose(&mut position, &mut quaternion, &mut scale);
        self.position = position;
        self.quaternion = quaternion;
        self.scale = scale;

        self.sync_rotation_from_quaternion();
    }

    /// `Object3D.applyQuaternion()`.
    pub fn apply_quaternion(&mut self, q: &Quaternion) {
        self.quaternion.premultiply(q);
        self.sync_rotation_from_quaternion();
    }

    /// `Object3D.setRotationFromAxisAngle()`.
    pub fn set_rotation_from_axis_angle(&mut self, axis: &Vector3, angle: f64) {
        // assumes axis is normalized
        self.quaternion.set_from_axis_angle(axis, angle);
        self.sync_rotation_from_quaternion();
    }

    /// `Object3D.setRotationFromEuler()`.
    pub fn set_rotation_from_euler(&mut self, euler: &Euler) {
        self.quaternion.set_from_euler(euler);
        self.sync_rotation_from_quaternion();
    }

    /// `Object3D.setRotationFromMatrix()` — the upper 3x3 of `m` must be a pure
    /// rotation (unscaled).
    pub fn set_rotation_from_matrix(&mut self, m: &Matrix4) {
        self.quaternion.set_from_rotation_matrix(m);
        self.sync_rotation_from_quaternion();
    }

    /// `Object3D.setRotationFromQuaternion()` — `q` is assumed normalized.
    pub fn set_rotation_from_quaternion(&mut self, q: &Quaternion) {
        self.quaternion.copy(q);
        self.sync_rotation_from_quaternion();
    }

    /// `Object3D.rotateOnAxis()` — rotates about an axis in object space.
    pub fn rotate_on_axis(&mut self, axis: &Vector3, angle: f64) {
        let mut q1 = Quaternion::default();
        q1.set_from_axis_angle(axis, angle);

        self.quaternion.multiply(&q1);
        self.sync_rotation_from_quaternion();
    }

    /// `Object3D.rotateOnWorldAxis()` — rotates about an axis in world space.
    /// Assumes the object has no rotated parent.
    pub fn rotate_on_world_axis(&mut self, axis: &Vector3, angle: f64) {
        let mut q1 = Quaternion::default();
        q1.set_from_axis_angle(axis, angle);

        self.quaternion.premultiply(&q1);
        self.sync_rotation_from_quaternion();
    }

    /// `Object3D.rotateX()`.
    pub fn rotate_x(&mut self, angle: f64) {
        self.rotate_on_axis(&Vector3::new(1.0, 0.0, 0.0), angle);
    }

    /// `Object3D.rotateY()`.
    pub fn rotate_y(&mut self, angle: f64) {
        self.rotate_on_axis(&Vector3::new(0.0, 1.0, 0.0), angle);
    }

    /// `Object3D.rotateZ()`.
    pub fn rotate_z(&mut self, angle: f64) {
        self.rotate_on_axis(&Vector3::new(0.0, 0.0, 1.0), angle);
    }

    /// `Object3D.translateOnAxis()` — translates along an axis in object space.
    pub fn translate_on_axis(&mut self, axis: &Vector3, distance: f64) {
        let mut v1 = *axis;
        v1.apply_quaternion(&self.quaternion);
        v1.multiply_scalar(distance);

        self.position.add(&v1);
    }

    /// `Object3D.translateX()`.
    pub fn translate_x(&mut self, distance: f64) {
        self.translate_on_axis(&Vector3::new(1.0, 0.0, 0.0), distance);
    }

    /// `Object3D.translateY()`.
    pub fn translate_y(&mut self, distance: f64) {
        self.translate_on_axis(&Vector3::new(0.0, 1.0, 0.0), distance);
    }

    /// `Object3D.translateZ()`.
    pub fn translate_z(&mut self, distance: f64) {
        self.translate_on_axis(&Vector3::new(0.0, 0.0, 1.0), distance);
    }

    /// `Object3D.localToWorld()`.
    pub fn local_to_world(&mut self, vector: &mut Vector3) {
        self.update_matrix_world(None);
        vector.apply_matrix4(&self.matrix_world);
    }

    /// `Object3D.worldToLocal()`.
    pub fn world_to_local(&mut self, vector: &mut Vector3) {
        self.update_matrix_world(None);

        let mut inverse = self.matrix_world;
        inverse.invert();
        vector.apply_matrix4(&inverse);
    }

    /// `Object3D.lookAt()` for a plain object: the object's +Z is pointed away
    /// from `target` (a camera or light points *at* it instead — see
    /// `PerspectiveCamera::look_at`).
    pub fn look_at(&mut self, target: &Vector3) {
        self.update_matrix_world(None);

        let mut position = Vector3::ZERO;
        position.set_from_matrix_position(&self.matrix_world);

        let mut m1 = Matrix4::identity();
        m1.look_at(target, &position, &self.up);

        self.quaternion.set_from_rotation_matrix(&m1);
        self.sync_rotation_from_quaternion();
    }

    /// `Object3D.getWorldPosition()`.
    pub fn get_world_position(&mut self) -> Vector3 {
        self.update_matrix_world(None);

        let mut target = Vector3::ZERO;
        target.set_from_matrix_position(&self.matrix_world);
        target
    }

    /// `Object3D.getWorldQuaternion()`.
    pub fn get_world_quaternion(&mut self) -> Quaternion {
        self.update_matrix_world(None);

        let mut position = Vector3::ZERO;
        let mut target = Quaternion::default();
        let mut scale = Vector3::ZERO;
        self.matrix_world
            .decompose(&mut position, &mut target, &mut scale);
        target
    }

    /// `Object3D.getWorldScale()`.
    pub fn get_world_scale(&mut self) -> Vector3 {
        self.update_matrix_world(None);

        let mut position = Vector3::ZERO;
        let mut quaternion = Quaternion::default();
        let mut target = Vector3::ZERO;
        self.matrix_world
            .decompose(&mut position, &mut quaternion, &mut target);
        target
    }

    /// `Object3D.getWorldDirection()` — the object's +Z axis in world space.
    pub fn get_world_direction(&mut self) -> Vector3 {
        self.update_matrix_world(None);

        let e = &self.matrix_world.elements;
        let mut target = Vector3::new(e[8], e[9], e[10]);
        target.normalize();
        target
    }

    /// `Object3D.updateMatrix()`.
    pub fn update_matrix(&mut self) {
        let (position, quaternion, scale) = (self.position, self.quaternion, self.scale);
        self.matrix.compose(&position, &quaternion, &scale);

        self.matrix_world_needs_update = true;
    }

    /// `Object3D.updateMatrixWorld()` for an object whose parent's world matrix
    /// is `parent_matrix_world` (`None` for a root).
    pub fn update_matrix_world(&mut self, parent_matrix_world: Option<&Matrix4>) {
        self.update_matrix();

        match parent_matrix_world {
            None => self.matrix_world = self.matrix,
            Some(parent) => {
                let matrix = self.matrix;
                self.matrix_world.multiply_matrices(parent, &matrix);
            }
        }
    }
}
