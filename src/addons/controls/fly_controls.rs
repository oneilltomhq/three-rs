//! Port of `three.js/examples/jsm/controls/FlyControls.js`.

use super::{KeyCode, MouseButton};
use crate::core::ObjectRef;
use crate::math::{Quaternion, Vector3};

/// `_EPS` — the change below which `update()` reports "nothing moved" and
/// skips the `change` event.
const EPS: f64 = 0.000001;

/// `_moveState`: which way the controls are being driven, one entry per
/// direction. Keys set an entry to 1 or 0; the pointer writes fractional
/// [`yaw_left`](Self::yaw_left) and [`pitch_down`](Self::pitch_down) in
/// `[-1, 1]` from its position in the element.
///
/// The vectors [`FlyControls::update`] reads are derived from this by
/// [`FlyControls::update_movement_vector`] and
/// [`FlyControls::update_rotation_vector`]; a host that writes it directly
/// calls those after, as the JS handlers do.
#[derive(Clone, Copy, PartialEq, Debug, Default)]
pub struct MoveState {
    /// R.
    pub up: f64,
    /// F.
    pub down: f64,
    /// A.
    pub left: f64,
    /// D.
    pub right: f64,
    /// W, or the left button.
    pub forward: f64,
    /// S, or the right button.
    pub back: f64,
    /// ArrowUp.
    pub pitch_up: f64,
    /// ArrowDown, or the pointer below the element's centre.
    pub pitch_down: f64,
    /// ArrowLeft, or the pointer left of the element's centre.
    pub yaw_left: f64,
    /// ArrowRight.
    pub yaw_right: f64,
    /// Q.
    pub roll_left: f64,
    /// E.
    pub roll_right: f64,
}

/// Fly a camera in its own frame: WASD / R / F translate along the camera's
/// axes, the arrows pitch and yaw, Q / E roll, and the pointer steers by its
/// offset from the element's centre.
///
/// A port of `examples/jsm/controls/FlyControls.js` as of the pinned 5f610f5
/// (r187dev): the same fields with the same defaults, the same `_moveState`,
/// `_updateMovementVector()` / `_updateRotationVector()`, and the same
/// `update( delta )` — three `translateX/Y/Z` calls, then a post-multiplied,
/// normalised small-angle quaternion, then the `_EPS` change detection that
/// decides its return value.
///
/// # Input, as calls
///
/// The JS class is its own DOM listener set; here the host calls
/// [`key_down`](Self::key_down), [`key_up`](Self::key_up),
/// [`pointer_down`](Self::pointer_down), [`pointer_move`](Self::pointer_move),
/// [`pointer_up`](Self::pointer_up) and
/// [`pointer_cancel`](Self::pointer_cancel). The element's `offsetWidth` /
/// `offsetHeight`, which `onPointerMove` normalises the pointer by, become
/// [`set_element_size`](Self::set_element_size); pointer coordinates are
/// **element-relative** (`pageX - offsetLeft`, `pageY - offsetTop`), y down,
/// which is the subtraction `_getContainerDimensions()` exists for, done by
/// the host.
///
/// # The object is an argument
///
/// As with [`OrbitControls`](super::OrbitControls), the camera belongs to the
/// application. Only [`update`](Self::update) touches it, so only it takes
/// it, as a [`ObjectRef`] (`&camera.node` for a
/// [`PerspectiveCamera`](crate::cameras::PerspectiveCamera)) because the JS
/// only needs an `Object3D`.
///
/// # What is left out
///
/// - The `change` event. [`update`](Self::update) returns the boolean the JS
///   dispatches it on.
/// - `connect()` / `disconnect()` / `dispose()` and the `contextmenu`
///   suppression, which are DOM listener bookkeeping, and the
///   `domElement === document` branch of `_getContainerDimensions()` (the
///   host passes whichever size it means).
pub struct FlyControls {
    /// `controls.enabled` (from the `Controls` base class). When `false`,
    /// input is ignored and [`update`](Self::update) does nothing.
    pub enabled: bool,

    /// The movement speed. Default 1.
    pub movement_speed: f64,
    /// The rotation speed. Default 0.005.
    pub roll_speed: f64,
    /// When `true`, the pointer only steers while a button is held. Default
    /// `false`.
    pub drag_to_look: bool,
    /// When `true`, the camera moves forward unless S is held. Default
    /// `false`.
    pub auto_forward: bool,
    /// `movementSpeedMultiplier`: Shift sets it to 0.1 and releasing Shift
    /// sets it to 1, but `update()` never reads it, so neither does this. The
    /// JS constructor never assigns it, so there it is `undefined` until Shift
    /// is first pressed; here it starts at 1, the value releasing Shift
    /// leaves, since an `f64` has no `undefined`.
    pub movement_speed_multiplier: f64,

    /// `_moveState`. Call [`update_movement_vector`](Self::update_movement_vector)
    /// and [`update_rotation_vector`](Self::update_rotation_vector) after
    /// writing it directly.
    pub move_state: MoveState,

    move_vector: Vector3,
    rotation_vector: Vector3,
    last_quaternion: Quaternion,
    last_position: Vector3,
    /// Buttons down, with `drag_to_look`.
    status: i32,

    /// The element's `offsetWidth` / `offsetHeight`.
    element_width: f64,
    element_height: f64,
}

impl Default for FlyControls {
    fn default() -> Self {
        Self::new()
    }
}

impl FlyControls {
    /// `new FlyControls( object )`. The JS constructor does not read the
    /// object, so this does not take it.
    ///
    /// The element size starts at 800x500; call
    /// [`set_element_size`](Self::set_element_size) with the real one.
    pub fn new() -> Self {
        Self {
            enabled: true,

            movement_speed: 1.0,
            roll_speed: 0.005,
            drag_to_look: false,
            auto_forward: false,
            movement_speed_multiplier: 1.0,

            move_state: MoveState::default(),

            move_vector: Vector3::ZERO,
            rotation_vector: Vector3::ZERO,
            last_quaternion: Quaternion::default(),
            last_position: Vector3::ZERO,
            status: 0,

            element_width: 800.0,
            element_height: 500.0,
        }
    }

    /// The element's `offsetWidth` and `offsetHeight`.
    ///
    /// Each is clamped to at least 1. The JS divides by half of each, so a
    /// zero-sized (hidden or collapsed) element would turn the pointer's
    /// steer into `±Infinity` or `NaN` there; here it stays finite.
    pub fn set_element_size(&mut self, width: f64, height: f64) {
        self.element_width = width.max(1.0);
        self.element_height = height.max(1.0);
    }

    /// `_moveVector`: the translation direction, in the camera's frame.
    pub fn move_vector(&self) -> Vector3 {
        self.move_vector
    }

    /// `_rotationVector`: pitch, yaw and roll rates about the camera's X, Y
    /// and Z.
    pub fn rotation_vector(&self) -> Vector3 {
        self.rotation_vector
    }

    /// `update( delta )`, `delta` in seconds.
    ///
    /// Returns whether the camera moved by more than `_EPS` since the last
    /// time it did — the JS's guard on dispatching `change`.
    pub fn update(&mut self, object: &ObjectRef, delta: f64) -> bool {
        if !self.enabled {
            return false;
        }

        let mut object = object.borrow_mut();

        let move_mult = delta * self.movement_speed;
        let rot_mult = delta * self.roll_speed;

        object.translate_x(self.move_vector.x * move_mult);
        object.translate_y(self.move_vector.y * move_mult);
        object.translate_z(self.move_vector.z * move_mult);

        let mut tmp_quaternion = Quaternion::default();
        tmp_quaternion
            .set(
                self.rotation_vector.x * rot_mult,
                self.rotation_vector.y * rot_mult,
                self.rotation_vector.z * rot_mult,
                1.0,
            )
            .normalize();
        object.quaternion.multiply(&tmp_quaternion);
        object.sync_rotation_from_quaternion();

        if self.last_position.distance_to_squared(&object.position) > EPS
            || 8.0 * (1.0 - self.last_quaternion.dot(&object.quaternion)) > EPS
        {
            self.last_quaternion = object.quaternion;
            self.last_position = object.position;
            true
        } else {
            false
        }
    }

    /// `_updateMovementVector()`.
    pub fn update_movement_vector(&mut self) {
        let state = &self.move_state;
        let forward = if state.forward != 0.0 || (self.auto_forward && state.back == 0.0) {
            1.0
        } else {
            0.0
        };

        self.move_vector.x = -state.left + state.right;
        self.move_vector.y = -state.down + state.up;
        self.move_vector.z = -forward + state.back;
    }

    /// `_updateRotationVector()`.
    pub fn update_rotation_vector(&mut self) {
        let state = &self.move_state;
        self.rotation_vector.x = -state.pitch_down + state.pitch_up;
        self.rotation_vector.y = -state.yaw_right + state.yaw_left;
        self.rotation_vector.z = -state.roll_right + state.roll_left;
    }

    /// `onKeyDown`. Ignored while Alt is held, as in the JS.
    pub fn key_down(&mut self, key: KeyCode, alt_key: bool) {
        if alt_key || !self.enabled {
            return;
        }

        self.set_key(key, 1.0);
    }

    /// `onKeyUp`.
    pub fn key_up(&mut self, key: KeyCode) {
        if !self.enabled {
            return;
        }

        self.set_key(key, 0.0);
    }

    fn set_key(&mut self, key: KeyCode, value: f64) {
        let state = &mut self.move_state;
        match key {
            KeyCode::ShiftLeft | KeyCode::ShiftRight => {
                self.movement_speed_multiplier = if value == 1.0 { 0.1 } else { 1.0 };
            }

            KeyCode::KeyW => state.forward = value,
            KeyCode::KeyS => state.back = value,

            KeyCode::KeyA => state.left = value,
            KeyCode::KeyD => state.right = value,

            KeyCode::KeyR => state.up = value,
            KeyCode::KeyF => state.down = value,

            KeyCode::ArrowUp => state.pitch_up = value,
            KeyCode::ArrowDown => state.pitch_down = value,

            KeyCode::ArrowLeft => state.yaw_left = value,
            KeyCode::ArrowRight => state.yaw_right = value,

            KeyCode::KeyQ => state.roll_left = value,
            KeyCode::KeyE => state.roll_right = value,

            KeyCode::Other => {}
        }

        self.update_movement_vector();
        self.update_rotation_vector();
    }

    /// `onPointerDown`: with `drag_to_look`, starts steering; otherwise the
    /// left button flies forward and the right backward.
    pub fn pointer_down(&mut self, button: MouseButton) {
        if !self.enabled {
            return;
        }

        if self.drag_to_look {
            self.status += 1;
        } else {
            match button {
                MouseButton::Left => self.move_state.forward = 1.0,
                MouseButton::Right => self.move_state.back = 1.0,
                MouseButton::Middle | MouseButton::Other => {}
            }

            self.update_movement_vector();
        }
    }

    /// `onPointerMove`, at element-relative `(x, y)`: yaw and pitch by the
    /// pointer's offset from the element's centre, in half-sizes. Ignored with
    /// `drag_to_look` and no button down.
    pub fn pointer_move(&mut self, x: f64, y: f64) {
        if !self.enabled {
            return;
        }

        if !self.drag_to_look || self.status > 0 {
            let half_width = self.element_width / 2.0;
            let half_height = self.element_height / 2.0;

            self.move_state.yaw_left = -(x - half_width) / half_width;
            self.move_state.pitch_down = (y - half_height) / half_height;

            self.update_rotation_vector();
        }
    }

    /// `onPointerUp`.
    pub fn pointer_up(&mut self, button: MouseButton) {
        if !self.enabled {
            return;
        }

        if self.drag_to_look {
            self.status -= 1;

            self.move_state.yaw_left = 0.0;
            self.move_state.pitch_down = 0.0;
        } else {
            match button {
                MouseButton::Left => self.move_state.forward = 0.0,
                MouseButton::Right => self.move_state.back = 0.0,
                MouseButton::Middle | MouseButton::Other => {}
            }

            self.update_movement_vector();
        }

        self.update_rotation_vector();
    }

    /// `onPointerCancel`.
    pub fn pointer_cancel(&mut self) {
        if !self.enabled {
            return;
        }

        if self.drag_to_look {
            self.status = 0;

            self.move_state.yaw_left = 0.0;
            self.move_state.pitch_down = 0.0;
        } else {
            self.move_state.forward = 0.0;
            self.move_state.back = 0.0;

            self.update_movement_vector();
        }

        self.update_rotation_vector();
    }
}
