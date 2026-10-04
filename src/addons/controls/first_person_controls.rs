//! Port of `three.js/examples/jsm/controls/FirstPersonControls.js`.

use super::{KeyCode, MouseButton};
use crate::core::Node;
use crate::math::math_utils::{clamp, deg_to_rad, lerp, map_linear, rad_to_deg};
use crate::math::{Spherical, Vector3};

/// Walk and look around: WASD / arrows move in the ground plane, R / F move
/// up and down, and a drag turns the view, easing in and out.
///
/// A port of `examples/jsm/controls/FirstPersonControls.js` as of the pinned
/// 5f610f5 (r187dev): the same fields with the same defaults, and the same
/// `update( delta )` — the world-axis key velocity from the camera's yaw, the
/// look-direction pointer velocity, the shared `dampingFactor` lerp on both,
/// the latitude clamp to ±85° and the spherical `lookAt` the camera ends each
/// frame with.
///
/// The damped, world-axis version landed in r186: #33874 added the eased
/// velocities and separate key / pointer move sources, f88964a made the keys
/// world-axis aligned, and #34485 restored R / F. Already before that, the
/// look came from the pointer's offset from where the drag *started* (not
/// from the element's centre), so the element's size is not read anywhere and
/// there is no `activeLook`; `handleResize()` has been a deprecated no-op
/// since r184.
///
/// # Input, as calls
///
/// The JS class is also its own DOM listener set. Here the host calls
/// [`pointer_down`](Self::pointer_down), [`pointer_move`](Self::pointer_move),
/// [`pointer_up`](Self::pointer_up), [`touch_start`](Self::touch_start),
/// [`touch_end`](Self::touch_end), [`key_down`](Self::key_down) and
/// [`key_up`](Self::key_up) with the fields the JS handlers read. Pointer
/// coordinates only ever enter as differences from the pointer-down position,
/// so any origin works as long as it is the same for the whole drag.
///
/// # The object is an argument
///
/// As with [`OrbitControls`](super::OrbitControls), the camera belongs to the
/// application, so [`new`](Self::new), [`look_at`](Self::look_at) and
/// [`update`](Self::update) take it rather than the controls holding it. They
/// take its [`Node`] (`&camera.node` for a
/// [`PerspectiveCamera`](crate::cameras::PerspectiveCamera)), because the JS
/// only needs an `Object3D`, and [`Node::look_at`] is three's
/// `Object3D.lookAt()` in full: world position, parent rotation, and the
/// camera/non-camera flip. The input methods only change the controls' state,
/// as in the JS, where only `update()` and `lookAt()` touch `this.object`.
///
/// # What is left out
///
/// - The `Controls` base class's `connect()` / `disconnect()` / `dispose()`,
///   and the `contextmenu` suppression: they are DOM listener bookkeeping.
/// - `handleResize()`, a deprecated no-op since r184.
pub struct FirstPersonControls {
    /// `controls.enabled` (from the `Controls` base class). When `false`,
    /// [`update`](Self::update) does nothing; input is still recorded, as in
    /// the JS.
    pub enabled: bool,

    /// The movement speed. Default 1.
    pub movement_speed: f64,
    /// The look-around speed. Default 0.005.
    pub look_speed: f64,
    /// How quickly the movement and look velocities catch up with the input.
    /// Lower is heavier; 1 disables the easing. Default 0.1.
    pub damping_factor: f64,
    /// Whether dragging vertically looks up and down. Default `true`.
    pub look_vertical: bool,
    /// Whether the camera moves forward on its own. Default `false`.
    pub auto_forward: bool,
    /// Whether the camera's height adds to its forward speed, through
    /// [`height_coef`](Self::height_coef), [`height_min`](Self::height_min) and
    /// [`height_max`](Self::height_max). Default `false`.
    pub height_speed: bool,
    /// How much faster the camera moves forward near `height_max`. Default 1.
    pub height_coef: f64,
    /// Lower height for the speed adjustment. Default 0.
    pub height_min: f64,
    /// Upper height for the speed adjustment. Default 1.
    pub height_max: f64,
    /// Whether the vertical look is mapped into
    /// [`vertical_min`](Self::vertical_min)..[`vertical_max`](Self::vertical_max).
    /// Default `false`.
    pub constrain_vertical: bool,
    /// Lower polar limit, radians in `[0, PI]`. Default 0.
    pub vertical_min: f64,
    /// Upper polar limit, radians in `[0, PI]`. Default PI.
    pub vertical_max: f64,

    /// `mouseDragOn`, which the JS marks read-only.
    mouse_drag_on: bool,

    velocity: Vector3,

    pointer_x: f64,
    pointer_y: f64,
    pointer_down_x: f64,
    pointer_down_y: f64,
    pointer_count: i32,

    // Forward / backward come from keys and the pointer, tracked per source so
    // they don't clobber: while a forward / backward key is held, a click only
    // looks.
    key_forward: bool,
    key_backward: bool,
    pointer_forward: bool,
    pointer_backward: bool,
    move_left: bool,
    move_right: bool,
    move_up: bool,
    move_down: bool,

    /// Degrees.
    lat: f64,
    /// Degrees.
    lon: f64,
    lon_velocity: f64,
    lat_velocity: f64,
}

impl FirstPersonControls {
    /// `new FirstPersonControls( camera )`: the defaults, with the latitude
    /// and longitude read off the camera's current orientation
    /// (`_setOrientation()`), so the first `update()` does not snap the view.
    pub fn new(object: &Node) -> Self {
        let mut controls = Self {
            enabled: true,

            movement_speed: 1.0,
            look_speed: 0.005,
            damping_factor: 0.1,
            look_vertical: true,
            auto_forward: false,
            height_speed: false,
            height_coef: 1.0,
            height_min: 0.0,
            height_max: 1.0,
            constrain_vertical: false,
            vertical_min: 0.0,
            vertical_max: std::f64::consts::PI,

            mouse_drag_on: false,

            velocity: Vector3::ZERO,

            pointer_x: 0.0,
            pointer_y: 0.0,
            pointer_down_x: 0.0,
            pointer_down_y: 0.0,
            pointer_count: 0,

            key_forward: false,
            key_backward: false,
            pointer_forward: false,
            pointer_backward: false,
            move_left: false,
            move_right: false,
            move_up: false,
            move_down: false,

            lat: 0.0,
            lon: 0.0,
            lon_velocity: 0.0,
            lat_velocity: 0.0,
        };
        controls.set_orientation(object);
        controls
    }

    /// `mouseDragOn`: whether a pointer is down.
    pub fn mouse_drag_on(&self) -> bool {
        self.mouse_drag_on
    }

    /// `lookAt( target )`: turns the object to face the world-space `target`
    /// and re-reads the latitude and longitude from it.
    pub fn look_at(&mut self, object: &Node, target: &Vector3) -> &mut Self {
        object.look_at(target);
        self.set_orientation(object);
        self
    }

    /// `update( delta )`, `delta` in seconds.
    pub fn update(&mut self, object: &Node, delta: f64) {
        if !self.enabled {
            return;
        }

        let one = |held: bool| -> f64 {
            if held {
                1.0
            } else {
                0.0
            }
        };

        let mut drive = one(self.key_forward) - one(self.key_backward);
        let mut look_move = one(self.pointer_forward) - one(self.pointer_backward);

        if self.auto_forward && drive == 0.0 && look_move == 0.0 {
            look_move = 1.0;
        }

        // faster forward movement the higher the camera is
        let (position, quaternion) = {
            let object = object.borrow();
            (object.position, object.quaternion)
        };

        let mut forward_speed = self.movement_speed;

        if self.height_speed {
            let y = clamp(position.y, self.height_min, self.height_max);
            forward_speed += (y - self.height_min) * self.height_coef;
        }

        // target velocity in world space: keys are world axis aligned, moving
        // in the XZ plane from the camera's yaw only (R / F along world Y),
        // while pointer and touch input moves along the look direction
        let yaw = deg_to_rad(self.lon);
        let sin_yaw = yaw.sin();
        let cos_yaw = yaw.cos();

        let mut strafe = one(self.move_right) - one(self.move_left);
        let mut climb = one(self.move_up) - one(self.move_down);

        // normalize combined key input so diagonal movement isn't faster
        let key_scale = 1.0 / 1.0_f64.max((strafe * strafe + climb * climb + drive * drive).sqrt());

        strafe *= self.movement_speed * key_scale;
        climb *= self.movement_speed * key_scale;
        drive *= (if drive > 0.0 {
            forward_speed
        } else {
            self.movement_speed
        }) * key_scale;

        let mut target_velocity = Vector3::new(
            sin_yaw * drive - cos_yaw * strafe,
            climb,
            cos_yaw * drive + sin_yaw * strafe,
        );

        if look_move != 0.0 {
            let mut look_direction = Vector3::new(0.0, 0.0, -1.0);
            look_direction.apply_quaternion(&quaternion);
            let speed = if look_move > 0.0 {
                forward_speed
            } else {
                self.movement_speed
            };
            target_velocity.add_scaled_vector(&look_direction, look_move * speed);
        }

        // ease toward the target velocity for smooth acceleration and
        // deceleration
        self.velocity.lerp(&target_velocity, self.damping_factor);

        let position = {
            let mut object = object.borrow_mut();
            object.position.add_scaled_vector(&self.velocity, delta);
            object.position
        };

        let vertical_look_ratio = if self.constrain_vertical {
            std::f64::consts::PI / (self.vertical_max - self.vertical_min)
        } else {
            1.0
        };

        // target look velocity, zero when not dragging so the view eases to a
        // stop
        let target_lon = if self.mouse_drag_on {
            -self.pointer_x * self.look_speed
        } else {
            0.0
        };
        let target_lat = if self.mouse_drag_on && self.look_vertical {
            -self.pointer_y * self.look_speed * vertical_look_ratio
        } else {
            0.0
        };

        self.lon_velocity = lerp(self.lon_velocity, target_lon, self.damping_factor);
        self.lat_velocity = lerp(self.lat_velocity, target_lat, self.damping_factor);

        self.lon += self.lon_velocity * delta;
        self.lat += self.lat_velocity * delta;

        self.lat = clamp(self.lat, -85.0, 85.0);

        let mut phi = deg_to_rad(90.0 - self.lat);
        let theta = deg_to_rad(self.lon);

        if self.constrain_vertical {
            phi = map_linear(
                phi,
                0.0,
                std::f64::consts::PI,
                self.vertical_min,
                self.vertical_max,
            );
        }

        let mut target_position = Vector3::ZERO;
        target_position
            .set_from_spherical_coords(1.0, phi, theta)
            .add(&position);

        object.look_at(&target_position);
    }

    /// `onPointerDown` for a mouse or pen: the left button moves forward and
    /// the right backward (unless a forward / backward key is held, when the
    /// click only looks), and every button starts a look drag at `(x, y)`.
    pub fn pointer_down(&mut self, button: MouseButton, x: f64, y: f64) {
        self.pointer_count += 1;

        if !self.key_forward && !self.key_backward {
            match button {
                MouseButton::Left => self.pointer_forward = true,
                MouseButton::Right => self.pointer_backward = true,
                MouseButton::Middle | MouseButton::Other => {}
            }
        }

        self.start_drag(x, y);
    }

    /// `onPointerDown` for `pointerType === 'touch'`: one finger moves
    /// forward, two or more backward.
    pub fn touch_start(&mut self, x: f64, y: f64) {
        self.pointer_count += 1;

        self.pointer_forward = self.pointer_count == 1;
        self.pointer_backward = self.pointer_count >= 2;

        self.start_drag(x, y);
    }

    fn start_drag(&mut self, x: f64, y: f64) {
        self.pointer_down_x = x;
        self.pointer_down_y = y;

        self.pointer_x = 0.0;
        self.pointer_y = 0.0;

        self.mouse_drag_on = true;
    }

    /// `onPointerUp` (and `pointercancel`) for a mouse or pen.
    pub fn pointer_up(&mut self, button: MouseButton) {
        if !self.mouse_drag_on {
            return;
        }

        self.pointer_count -= 1;

        match button {
            MouseButton::Left => self.pointer_forward = false,
            MouseButton::Right => self.pointer_backward = false,
            MouseButton::Middle | MouseButton::Other => {}
        }

        self.end_drag();
    }

    /// `onPointerUp` (and `pointercancel`) for `pointerType === 'touch'`.
    pub fn touch_end(&mut self) {
        if !self.mouse_drag_on {
            return;
        }

        self.pointer_count -= 1;

        self.pointer_forward = self.pointer_count == 1;
        self.pointer_backward = false;

        self.end_drag();
    }

    fn end_drag(&mut self) {
        self.pointer_x = 0.0;
        self.pointer_y = 0.0;

        if self.pointer_count == 0 {
            self.mouse_drag_on = false;
        }
    }

    /// `onPointerMove`: the drag's offset from where it started, which sets
    /// the look velocity. Ignored with no pointer down.
    pub fn pointer_move(&mut self, x: f64, y: f64) {
        if !self.mouse_drag_on {
            return;
        }

        self.pointer_x = x - self.pointer_down_x;
        self.pointer_y = y - self.pointer_down_y;
    }

    /// `onKeyDown`: W / ArrowUp forward, S / ArrowDown back, A / ArrowLeft
    /// and D / ArrowRight strafe, R up, F down.
    pub fn key_down(&mut self, key: KeyCode) {
        self.set_key(key, true);
    }

    /// `onKeyUp`.
    pub fn key_up(&mut self, key: KeyCode) {
        self.set_key(key, false);
    }

    fn set_key(&mut self, key: KeyCode, held: bool) {
        match key {
            KeyCode::ArrowUp | KeyCode::KeyW => self.key_forward = held,
            KeyCode::ArrowLeft | KeyCode::KeyA => self.move_left = held,
            KeyCode::ArrowDown | KeyCode::KeyS => self.key_backward = held,
            KeyCode::ArrowRight | KeyCode::KeyD => self.move_right = held,
            KeyCode::KeyR => self.move_up = held,
            KeyCode::KeyF => self.move_down = held,
            _ => {}
        }
    }

    /// `_setOrientation()`: the latitude and longitude, in degrees, of the
    /// camera's look direction.
    fn set_orientation(&mut self, object: &Node) {
        let quaternion = object.borrow().quaternion;

        let mut look_direction = Vector3::new(0.0, 0.0, -1.0);
        look_direction.apply_quaternion(&quaternion);
        let mut spherical = Spherical::default();
        spherical.set_from_vector3(&look_direction);

        self.lat = 90.0 - rad_to_deg(spherical.phi);
        self.lon = rad_to_deg(spherical.theta);
    }
}
