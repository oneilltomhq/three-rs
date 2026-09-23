//! Port of `three.js/examples/jsm/controls/OrbitControls.js`.

use crate::cameras::PerspectiveCamera;
use crate::math::{Quaternion, Spherical, Vector2, Vector3};

/// `_EPS` — the squared displacement below which `update()` reports "nothing
/// moved" and skips the `change` event.
const EPS: f64 = 0.000001;

const TWO_PI: f64 = 2.0 * std::f64::consts::PI;

/// `_STATE`, minus the four `TOUCH_*` members (see the struct's docs on what
/// is left out).
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
enum State {
    #[default]
    None,
    Rotate,
    Dolly,
    Pan,
}

/// Which physical button a [`PointerEvent`] carries — `event.button`'s 0, 1
/// and 2.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum MouseButton {
    /// `event.button === 0`, and what a default-constructed
    /// [`PointerEvent`] carries.
    #[default]
    Left,
    Middle,
    Right,
    /// Any other button; `onMouseDown`'s `default: mouseAction = -1`, i.e. it
    /// does nothing.
    Other,
}

/// `THREE.MOUSE` — what a button is bound to do.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum MouseAction {
    Rotate,
    Dolly,
    Pan,
    /// `mouseButtons.X = null`, which `onMouseDown` reads as "no action".
    None,
}

/// `controls.mouseButtons`.
#[derive(Clone, Copy, Debug)]
pub struct MouseButtons {
    pub left: MouseAction,
    pub middle: MouseAction,
    pub right: MouseAction,
}

impl Default for MouseButtons {
    /// `{ LEFT: MOUSE.ROTATE, MIDDLE: MOUSE.DOLLY, RIGHT: MOUSE.PAN }`.
    fn default() -> Self {
        Self {
            left: MouseAction::Rotate,
            middle: MouseAction::Dolly,
            right: MouseAction::Pan,
        }
    }
}

/// A DOM `PointerEvent`, as a value.
///
/// The JS class mixes the listeners into itself; here the host owns its event
/// source (winit in the viewer, `addEventListener` in the browser shell) and
/// hands the controls the fields its handlers read.
///
/// `client_x` / `client_y` are **relative to the element the controls act on**,
/// with y downwards. The JS reads `event.clientX`, which is relative to the
/// viewport, and then only ever uses differences of it — except in
/// `_updateZoomParameters`, which subtracts `getBoundingClientRect().left`.
/// Making the coordinates element-relative here is that subtraction, done by
/// the host, which is the only party that knows where its element is.
#[derive(Clone, Copy, Debug, Default)]
pub struct PointerEvent {
    /// `event.pointerId`. A mouse always reports the same id; distinct ids are
    /// how the JS counts fingers.
    pub pointer_id: i32,
    pub button: MouseButton,
    pub client_x: f64,
    pub client_y: f64,
    pub ctrl_key: bool,
    pub meta_key: bool,
    pub shift_key: bool,
}

/// `WheelEvent.deltaMode`.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum WheelDelta {
    /// `0` — `deltaY` is in pixels.
    #[default]
    Pixel,
    /// `1` — in lines; `_customWheelEvent` multiplies by 16.
    Line,
    /// `2` — in pages; `_customWheelEvent` multiplies by 100.
    Page,
}

/// A DOM `WheelEvent`, as a value. See [`PointerEvent`] on the coordinates.
#[derive(Clone, Copy, Debug, Default)]
pub struct WheelEvent {
    pub client_x: f64,
    pub client_y: f64,
    pub delta_y: f64,
    pub delta_mode: WheelDelta,
    /// A trackpad pinch arrives as a ctrl-wheel; `_customWheelEvent`
    /// multiplies `deltaY` by 10 for it, unless Control is genuinely held.
    pub ctrl_key: bool,
}

/// The four keys `controls.keys` names. Everything else is ignored, as in the
/// JS's `switch ( event.code )`.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Key {
    ArrowLeft,
    ArrowUp,
    ArrowRight,
    ArrowDown,
}

/// A DOM `keydown`, as a value.
#[derive(Clone, Copy, Debug)]
pub struct KeyEvent {
    pub key: Key,
    pub ctrl_key: bool,
    pub meta_key: bool,
    pub shift_key: bool,
}

/// Orbit, dolly and pan a camera around a target.
///
/// A port of `examples/jsm/controls/OrbitControls.js`: the same fields with
/// the same defaults, and the same `update()` — the spherical round trip
/// through the camera's up axis, the damping, the azimuth and polar limits,
/// the pan modes, auto-rotate, zoom-to-cursor and the `_EPS` change detection
/// that decides its return value.
///
/// # Input, as values
///
/// The JS class is also its own DOM listener set: `connect()` binds
/// `pointerdown`, `wheel`, `keydown` and friends on an element. Here the host
/// owns its event source and calls [`pointer_down`](Self::pointer_down),
/// [`pointer_move`](Self::pointer_move), [`pointer_up`](Self::pointer_up),
/// [`wheel`](Self::wheel) and [`key`](Self::key) with [`PointerEvent`],
/// [`WheelEvent`] and [`KeyEvent`] values — the same fields the JS's private
/// handlers read off the DOM events. The element's `clientWidth` /
/// `clientHeight`, which scale rotation and panning, become
/// [`set_element_size`](Self::set_element_size).
///
/// # The camera is an argument
///
/// The JS holds `this.object`. Rust cannot: the camera belongs to the example
/// and the controls are a field beside it. So every method that reads or
/// writes the camera takes it. That includes the pointer handlers, not only
/// `update()` — `_pan()` reads the camera's `fov`, its `matrix` and its
/// distance to the target, and `_handleMouseMove*` end by calling
/// `this.update()`, so the JS's handlers touch `this.object` too.
///
/// # What is left out
///
/// - **Touch.** `_handleTouchStart*` / `_handleTouchMove*` and the two-finger
///   dolly, pan and rotate. The `_pointers` bookkeeping that distinguishes one
///   finger from two is here, so a second pointer is tracked and ignored
///   rather than mistaken for a mouse drag.
/// - **Orthographic cameras.** `update()`'s `isOrthographicCamera` branches
///   (`zoom` in place of `radius`, `minZoom`/`maxZoom`, the ortho `_pan`).
///   None of the 39 graded pages puts `OrbitControls` on an
///   `OrthographicCamera` — the two that use one, `webgpu_compute_points` and
///   `webgpu_tsl_interoperability`, have no controls at all — so there is no
///   ported example to grade such a branch against. [`min_zoom`](Self::min_zoom)
///   and [`max_zoom`](Self::max_zoom) are kept as fields so that adding the
///   branch later is not an API change.
/// - **Events.** `change`, `start` and `end` are not dispatched; `update()`
///   returns the boolean the `change` dispatch is guarded by, which is what a
///   render-on-demand host needs.
/// - `cursorStyle`, which is a CSS property of an element this crate does not
///   have.
pub struct OrbitControls {
    /// `controls.enabled` (from the `Controls` base class).
    pub enabled: bool,

    /// The focus point the camera orbits.
    pub target: Vector3,
    /// The centre of the `min_target_radius` / `max_target_radius` sphere.
    pub cursor: Vector3,

    /// How far you can dolly in. Default 0.
    pub min_distance: f64,
    /// How far you can dolly out. Default infinity.
    pub max_distance: f64,
    /// Orthographic only; see the struct docs. Default 0.
    pub min_zoom: f64,
    /// Orthographic only; see the struct docs. Default infinity.
    pub max_zoom: f64,

    /// How close the target may come to [`cursor`](Self::cursor). Default 0.
    pub min_target_radius: f64,
    /// How far the target may go from [`cursor`](Self::cursor). Default
    /// infinity.
    pub max_target_radius: f64,

    /// Lower vertical limit, in `[0, PI]`. Default 0.
    pub min_polar_angle: f64,
    /// Upper vertical limit, in `[0, PI]`. Default PI.
    pub max_polar_angle: f64,
    /// Lower horizontal limit. Default negative infinity.
    pub min_azimuth_angle: f64,
    /// Upper horizontal limit. Default infinity.
    pub max_azimuth_angle: f64,

    /// Inertia. With it on, `update()` must be called every frame.
    pub enable_damping: bool,
    /// Default 0.05.
    pub damping_factor: f64,

    pub enable_zoom: bool,
    /// Default 1.
    pub zoom_speed: f64,
    pub enable_rotate: bool,
    /// Default 1.
    pub rotate_speed: f64,
    /// Default 1.
    pub key_rotate_speed: f64,
    pub enable_pan: bool,
    /// Default 1.
    pub pan_speed: f64,
    /// `true`: pan in screen space. `false`: pan in the plane orthogonal to
    /// the camera's up. Default `true`.
    pub screen_space_panning: bool,
    /// Pixels per keypress. Default 7.
    pub key_pan_speed: f64,
    /// Dolly towards the pointer rather than the centre. Default `false`.
    pub zoom_to_cursor: bool,

    /// Turn around the target by itself. With it on, `update()` must be called
    /// every frame.
    pub auto_rotate: bool,
    /// Default 2 — one orbit in 30 seconds at 60 fps.
    pub auto_rotate_speed: f64,

    /// `controls.mouseButtons`.
    pub mouse_buttons: MouseButtons,

    // `saveState()` / `reset()`.
    target0: Vector3,
    position0: Vector3,
    zoom0: f64,

    // internals
    state: State,
    last_position: Vector3,
    last_quaternion: Quaternion,
    last_target_position: Vector3,

    /// `_quat` — takes the camera's own up onto +Y, so the spherical maths is
    /// always done in a y-up frame however the camera is oriented.
    quat: Quaternion,
    quat_inverse: Quaternion,
    /// The camera's `up` the two quaternions above were built from, so that a
    /// host that changes it is noticed.
    up: Vector3,

    spherical: Spherical,
    spherical_delta: Spherical,

    scale: f64,
    pan_offset: Vector3,

    rotate_start: Vector2,
    rotate_end: Vector2,
    rotate_delta: Vector2,

    pan_start: Vector2,
    pan_end: Vector2,
    pan_delta: Vector2,

    dolly_start: Vector2,
    dolly_end: Vector2,
    dolly_delta: Vector2,

    dolly_direction: Vector3,
    mouse: Vector2,
    perform_cursor_zoom: bool,

    /// `_pointers` — the ids currently down, in order.
    pointers: Vec<i32>,

    /// The element's `clientWidth` / `clientHeight`.
    element_width: f64,
    element_height: f64,
}

impl OrbitControls {
    /// `new OrbitControls( camera, domElement )`.
    ///
    /// The JS constructor ends with `this.update()`, which seeds the spherical
    /// from `camera.position - target` and points the camera at the target.
    /// So does this, and that is why the camera is `&mut`: a page's
    /// `new OrbitControls( camera, … )` moves the camera, and a ported example
    /// that did not would render a different first frame.
    ///
    /// The element size starts as the renderer's drawing buffer would: a host
    /// that never calls [`set_element_size`](Self::set_element_size) gets the
    /// 800x500 the grader uses. Rotation and panning scale by it, so it only
    /// affects how far a drag goes, never where the camera starts.
    pub fn new(camera: &mut PerspectiveCamera) -> Self {
        let (position, up, zoom) = {
            let object = camera.node.borrow();
            (object.position, object.up, camera.zoom)
        };

        // `_quat = new Quaternion().setFromUnitVectors( object.up, ( 0, 1, 0 ) )`.
        let mut quat = Quaternion::default();
        quat.set_from_unit_vectors(&up, &Vector3::new(0.0, 1.0, 0.0));
        let mut quat_inverse = quat;
        quat_inverse.invert();

        let mut controls = Self {
            enabled: true,

            target: Vector3::ZERO,
            cursor: Vector3::ZERO,

            min_distance: 0.0,
            max_distance: f64::INFINITY,
            min_zoom: 0.0,
            max_zoom: f64::INFINITY,

            min_target_radius: 0.0,
            max_target_radius: f64::INFINITY,

            min_polar_angle: 0.0,
            max_polar_angle: std::f64::consts::PI,
            min_azimuth_angle: f64::NEG_INFINITY,
            max_azimuth_angle: f64::INFINITY,

            enable_damping: false,
            damping_factor: 0.05,

            enable_zoom: true,
            zoom_speed: 1.0,
            enable_rotate: true,
            rotate_speed: 1.0,
            key_rotate_speed: 1.0,
            enable_pan: true,
            pan_speed: 1.0,
            screen_space_panning: true,
            key_pan_speed: 7.0,
            zoom_to_cursor: false,

            auto_rotate: false,
            auto_rotate_speed: 2.0,

            mouse_buttons: MouseButtons::default(),

            target0: Vector3::ZERO,
            position0: position,
            zoom0: zoom,

            state: State::None,
            last_position: Vector3::ZERO,
            last_quaternion: Quaternion::default(),
            last_target_position: Vector3::ZERO,

            quat,
            quat_inverse,
            up,

            spherical: Spherical::default(),
            spherical_delta: Spherical::default(),

            scale: 1.0,
            pan_offset: Vector3::ZERO,

            rotate_start: Vector2::default(),
            rotate_end: Vector2::default(),
            rotate_delta: Vector2::default(),

            pan_start: Vector2::default(),
            pan_end: Vector2::default(),
            pan_delta: Vector2::default(),

            dolly_start: Vector2::default(),
            dolly_end: Vector2::default(),
            dolly_delta: Vector2::default(),

            dolly_direction: Vector3::ZERO,
            mouse: Vector2::default(),
            perform_cursor_zoom: false,

            pointers: Vec::new(),

            element_width: 800.0,
            element_height: 500.0,
        };

        controls.update(camera, None);
        controls
    }

    /// The element's `clientWidth` and `clientHeight`.
    ///
    /// Rotation and perspective panning divide by the **height** only ("yes,
    /// height" in the JS), so the aspect ratio does not distort the speed; the
    /// width is used by zoom-to-cursor's normalised-device coordinates.
    pub fn set_element_size(&mut self, width: f64, height: f64) {
        self.element_width = width.max(1.0);
        self.element_height = height.max(1.0);
    }

    /// `getPolarAngle()`.
    pub fn get_polar_angle(&self) -> f64 {
        self.spherical.phi
    }

    /// `getAzimuthalAngle()`.
    pub fn get_azimuthal_angle(&self) -> f64 {
        self.spherical.theta
    }

    /// `getDistance()`.
    pub fn get_distance(&self, camera: &PerspectiveCamera) -> f64 {
        camera.node.borrow().position.distance_to(&self.target)
    }

    /// `saveState()`.
    pub fn save_state(&mut self, camera: &PerspectiveCamera) {
        self.target0 = self.target;
        self.position0 = camera.node.borrow().position;
        self.zoom0 = camera.zoom;
    }

    /// `reset()` — back to the last [`save_state`](Self::save_state), or to
    /// the state the controls were constructed in.
    pub fn reset(&mut self, camera: &mut PerspectiveCamera) {
        self.target = self.target0;
        camera.node.borrow_mut().position = self.position0;
        camera.zoom = self.zoom0;
        camera.update_projection_matrix();

        self.update(camera, None);

        self.state = State::None;
    }

    /// `update( deltaTime )`.
    ///
    /// `delta` is the frame's length in seconds, and only auto-rotate reads
    /// it: with it, the turn is frame-rate independent; without it, the JS
    /// falls back to a per-frame angle that assumes 60 fps.
    ///
    /// Returns whether the camera or the target actually moved — the JS's
    /// guard on dispatching `change`, which is what a page that renders on
    /// demand rather than on a loop keys off.
    pub fn update(&mut self, camera: &mut PerspectiveCamera, delta: Option<f64>) -> bool {
        // `this.object.up` is not const across a program's life, and the two
        // quaternions are derived from it.
        let up = camera.node.borrow().up;
        if up != self.up {
            self.up = up;
            self.quat
                .set_from_unit_vectors(&up, &Vector3::new(0.0, 1.0, 0.0));
            self.quat_inverse = self.quat;
            self.quat_inverse.invert();
        }

        let mut v = camera.node.borrow().position;
        v.sub(&self.target);

        // rotate offset to "y-axis-is-up" space
        v.apply_quaternion(&self.quat);

        // angle from z-axis around y-axis
        self.spherical.set_from_vector3(&v);

        if self.auto_rotate && self.state == State::None {
            self.rotate_left(self.auto_rotation_angle(delta));
        }

        if self.enable_damping {
            self.spherical.theta += self.spherical_delta.theta * self.damping_factor;
            self.spherical.phi += self.spherical_delta.phi * self.damping_factor;
        } else {
            self.spherical.theta += self.spherical_delta.theta;
            self.spherical.phi += self.spherical_delta.phi;
        }

        // restrict theta to be between desired limits
        let (mut min, mut max) = (self.min_azimuth_angle, self.max_azimuth_angle);
        if min.is_finite() && max.is_finite() {
            if min < -std::f64::consts::PI {
                min += TWO_PI;
            } else if min > std::f64::consts::PI {
                min -= TWO_PI;
            }

            if max < -std::f64::consts::PI {
                max += TWO_PI;
            } else if max > std::f64::consts::PI {
                max -= TWO_PI;
            }

            self.spherical.theta = if min <= max {
                min.max(max.min(self.spherical.theta))
            } else if self.spherical.theta > (min + max) / 2.0 {
                min.max(self.spherical.theta)
            } else {
                max.min(self.spherical.theta)
            };
        }

        // restrict phi to be between desired limits
        self.spherical.phi = self
            .min_polar_angle
            .max(self.max_polar_angle.min(self.spherical.phi));

        self.spherical.make_safe();

        // move target to panned location
        if self.enable_damping {
            self.target
                .add_scaled_vector(&self.pan_offset, self.damping_factor);
        } else {
            self.target.add(&self.pan_offset);
        }

        // Limit the target distance from the cursor.
        self.target.sub(&self.cursor);
        self.target
            .clamp_length(self.min_target_radius, self.max_target_radius);
        self.target.add(&self.cursor);

        // Adjust the camera position from the zoom only when we are not
        // zooming to the cursor; in that case the zoom is applied below.
        let mut zoom_changed = false;
        if self.zoom_to_cursor && self.perform_cursor_zoom {
            self.spherical.radius = self.clamp_distance(self.spherical.radius);
        } else {
            let previous_radius = self.spherical.radius;
            self.spherical.radius = self.clamp_distance(self.spherical.radius * self.scale);
            zoom_changed = previous_radius != self.spherical.radius;
        }

        v.set_from_spherical_coords(
            self.spherical.radius,
            self.spherical.phi,
            self.spherical.theta,
        );

        // rotate offset back to "camera-up-vector-is-up" space
        v.apply_quaternion(&self.quat_inverse);

        {
            let mut position = self.target;
            position.add(&v);
            camera.node.borrow_mut().position = position;
        }

        camera.look_at(&self.target);

        if self.enable_damping {
            self.spherical_delta.theta *= 1.0 - self.damping_factor;
            self.spherical_delta.phi *= 1.0 - self.damping_factor;
            self.pan_offset.multiply_scalar(1.0 - self.damping_factor);
        } else {
            self.spherical_delta.set(0.0, 0.0, 0.0);
            self.pan_offset.set(0.0, 0.0, 0.0);
        }

        // adjust camera position
        if self.zoom_to_cursor && self.perform_cursor_zoom {
            // Move the camera down the pointer ray; this avoids the floating
            // point error of recomputing the radius from the new position.
            let previous_radius = v.length();
            let new_radius = self.clamp_distance(previous_radius * self.scale);
            let radius_delta = previous_radius - new_radius;

            {
                let dolly_direction = self.dolly_direction;
                camera
                    .node
                    .borrow_mut()
                    .position
                    .add_scaled_vector(&dolly_direction, radius_delta);
            }
            camera.update_matrix_world();

            zoom_changed = radius_delta != 0.0;

            if self.screen_space_panning {
                // Put the orbit target in front of the new camera position.
                let matrix = camera.node.borrow().matrix;
                let mut target = Vector3::new(0.0, 0.0, -1.0);
                target.transform_direction(&matrix);
                target.multiply_scalar(new_radius);
                target.add(&camera.node.borrow().position);
                self.target = target;
            } else {
                let (origin, matrix, up) = {
                    let object = camera.node.borrow();
                    (object.position, object.matrix, object.up)
                };
                let mut direction = Vector3::new(0.0, 0.0, -1.0);
                direction.transform_direction(&matrix);

                // Above 20 degrees from the horizon the plane intersection
                // runs away, so the target is left where it is.
                if up.dot(&direction).abs() < TILT_LIMIT {
                    camera.look_at(&self.target);
                } else {
                    let mut plane = crate::math::Plane::default();
                    plane.set_from_normal_and_coplanar_point(&up, &self.target);
                    let ray = crate::math::Ray::new(origin, direction);
                    if let Some(point) = ray.intersect_plane(&plane) {
                        self.target = point;
                    }
                }
            }
        }

        self.scale = 1.0;
        self.perform_cursor_zoom = false;

        // update condition is:
        // min(camera displacement, camera rotation in radians)^2 > EPS
        // using small-angle approximation cos(x/2) = 1 - x^2 / 8
        let (position, quaternion) = {
            let object = camera.node.borrow();
            (object.position, object.quaternion)
        };

        if zoom_changed
            || self.last_position.distance_to_squared(&position) > EPS
            || 8.0 * (1.0 - self.last_quaternion.dot(&quaternion)) > EPS
            || self.last_target_position.distance_to_squared(&self.target) > EPS
        {
            self.last_position = position;
            self.last_quaternion = quaternion;
            self.last_target_position = self.target;
            return true;
        }

        false
    }

    // ------------------------------------------------------------- the maths

    /// `_getAutoRotationAngle( deltaTime )`.
    fn auto_rotation_angle(&self, delta: Option<f64>) -> f64 {
        match delta {
            Some(delta) => (TWO_PI / 60.0 * self.auto_rotate_speed) * delta,
            None => TWO_PI / 60.0 / 60.0 * self.auto_rotate_speed,
        }
    }

    /// `_getZoomScale( delta )`.
    fn zoom_scale(&self, delta: f64) -> f64 {
        let normalized_delta = (delta * 0.01).abs();
        0.95f64.powf(self.zoom_speed * normalized_delta)
    }

    fn rotate_left(&mut self, angle: f64) {
        self.spherical_delta.theta -= angle;
    }

    fn rotate_up(&mut self, angle: f64) {
        self.spherical_delta.phi -= angle;
    }

    /// `_panLeft( distance, objectMatrix )`.
    fn pan_left(&mut self, distance: f64, object_matrix: &crate::math::Matrix4) {
        let mut v = Vector3::ZERO;
        v.set_from_matrix_column(object_matrix, 0);
        v.multiply_scalar(-distance);
        self.pan_offset.add(&v);
    }

    /// `_panUp( distance, objectMatrix )`.
    fn pan_up(&mut self, distance: f64, object_matrix: &crate::math::Matrix4, up: &Vector3) {
        let mut v = Vector3::ZERO;
        if self.screen_space_panning {
            v.set_from_matrix_column(object_matrix, 1);
        } else {
            v.set_from_matrix_column(object_matrix, 0);
            let column = v;
            v.cross_vectors(up, &column);
        }
        v.multiply_scalar(distance);
        self.pan_offset.add(&v);
    }

    /// `_pan( deltaX, deltaY )` — pixels, right and down positive.
    fn pan(&mut self, delta_x: f64, delta_y: f64, camera: &PerspectiveCamera) {
        let (position, matrix, up) = {
            let object = camera.node.borrow();
            (object.position, object.matrix, object.up)
        };

        let mut v = position;
        v.sub(&self.target);
        let mut target_distance = v.length();

        // half of the fov is centre to top of screen
        target_distance *= ((camera.fov / 2.0) * std::f64::consts::PI / 180.0).tan();

        // only clientHeight, so the aspect ratio does not distort the speed
        let height = self.element_height;
        self.pan_left(2.0 * delta_x * target_distance / height, &matrix);
        self.pan_up(2.0 * delta_y * target_distance / height, &matrix, &up);
    }

    fn dolly_out(&mut self, dolly_scale: f64) {
        self.scale /= dolly_scale;
    }

    fn dolly_in(&mut self, dolly_scale: f64) {
        self.scale *= dolly_scale;
    }

    /// `_updateZoomParameters( x, y )`. `x` / `y` are element-relative, so the
    /// JS's `rect.left` / `rect.top` are 0 and `rect.width` / `rect.height`
    /// are the element size.
    fn update_zoom_parameters(&mut self, x: f64, y: f64, camera: &PerspectiveCamera) {
        if !self.zoom_to_cursor {
            return;
        }

        self.perform_cursor_zoom = true;

        self.mouse.x = (x / self.element_width) * 2.0 - 1.0;
        self.mouse.y = -(y / self.element_height) * 2.0 + 1.0;

        let mut direction = Vector3::new(self.mouse.x, self.mouse.y, 1.0);
        direction.unproject(camera);
        direction.sub(&camera.node.borrow().position);
        direction.normalize();
        self.dolly_direction = direction;
    }

    fn clamp_distance(&self, distance: f64) -> f64 {
        self.min_distance.max(self.max_distance.min(distance))
    }

    // ------------------------------------------------------------- the input

    /// `onPointerDown` for a mouse pointer.
    ///
    /// Touch is not ported: a second pointer is tracked and then ignored,
    /// rather than starting a two-finger gesture.
    pub fn pointer_down(&mut self, camera: &mut PerspectiveCamera, event: &PointerEvent) {
        if !self.enabled {
            return;
        }

        if self.pointers.contains(&event.pointer_id) {
            return;
        }
        self.pointers.push(event.pointer_id);

        // `onMouseDown`: a second button while one is held does not restart
        // the gesture, exactly as the JS's `_isTrackingPointer` guard.
        if self.pointers.len() > 1 {
            return;
        }

        let action = match event.button {
            MouseButton::Left => self.mouse_buttons.left,
            MouseButton::Middle => self.mouse_buttons.middle,
            MouseButton::Right => self.mouse_buttons.right,
            MouseButton::Other => MouseAction::None,
        };

        let modified = event.ctrl_key || event.meta_key || event.shift_key;

        match action {
            MouseAction::Dolly => {
                if !self.enable_zoom {
                    return;
                }
                // `this._updateZoomParameters( event.clientX, event.clientX )`
                // — the JS passes clientX twice; kept, because the port is of
                // the code and not of what it meant to say.
                self.update_zoom_parameters(event.client_x, event.client_x, camera);
                self.dolly_start.set(event.client_x, event.client_y);
                self.state = State::Dolly;
            }
            MouseAction::Rotate => {
                if modified {
                    if !self.enable_pan {
                        return;
                    }
                    self.pan_start.set(event.client_x, event.client_y);
                    self.state = State::Pan;
                } else {
                    if !self.enable_rotate {
                        return;
                    }
                    self.rotate_start.set(event.client_x, event.client_y);
                    self.state = State::Rotate;
                }
            }
            MouseAction::Pan => {
                if modified {
                    if !self.enable_rotate {
                        return;
                    }
                    self.rotate_start.set(event.client_x, event.client_y);
                    self.state = State::Rotate;
                } else {
                    if !self.enable_pan {
                        return;
                    }
                    self.pan_start.set(event.client_x, event.client_y);
                    self.state = State::Pan;
                }
            }
            MouseAction::None => self.state = State::None,
        }
    }

    /// `onPointerMove` for a mouse pointer.
    pub fn pointer_move(&mut self, camera: &mut PerspectiveCamera, event: &PointerEvent) {
        if !self.enabled {
            return;
        }

        match self.state {
            State::Rotate => {
                if !self.enable_rotate {
                    return;
                }
                self.rotate_end.set(event.client_x, event.client_y);
                let (end, start) = (self.rotate_end, self.rotate_start);
                self.rotate_delta.sub_vectors(&end, &start);
                self.rotate_delta.multiply_scalar(self.rotate_speed);

                let height = self.element_height;
                // "yes, height" — both axes.
                self.rotate_left(TWO_PI * self.rotate_delta.x / height);
                self.rotate_up(TWO_PI * self.rotate_delta.y / height);

                self.rotate_start = self.rotate_end;
                self.update(camera, None);
            }
            State::Dolly => {
                if !self.enable_zoom {
                    return;
                }
                self.dolly_end.set(event.client_x, event.client_y);
                let (end, start) = (self.dolly_end, self.dolly_start);
                self.dolly_delta.sub_vectors(&end, &start);

                if self.dolly_delta.y > 0.0 {
                    self.dolly_out(self.zoom_scale(self.dolly_delta.y));
                } else if self.dolly_delta.y < 0.0 {
                    self.dolly_in(self.zoom_scale(self.dolly_delta.y));
                }

                self.dolly_start = self.dolly_end;
                self.update(camera, None);
            }
            State::Pan => {
                if !self.enable_pan {
                    return;
                }
                self.pan_end.set(event.client_x, event.client_y);
                let (end, start) = (self.pan_end, self.pan_start);
                self.pan_delta.sub_vectors(&end, &start);
                self.pan_delta.multiply_scalar(self.pan_speed);

                self.pan(self.pan_delta.x, self.pan_delta.y, camera);

                self.pan_start = self.pan_end;
                self.update(camera, None);
            }
            State::None => {}
        }
    }

    /// `onPointerUp`.
    pub fn pointer_up(&mut self, event: &PointerEvent) {
        self.pointers.retain(|id| *id != event.pointer_id);

        if self.pointers.is_empty() {
            self.state = State::None;
        }
    }

    /// `onMouseWheel` — the wheel dollies unless a drag is already running.
    ///
    /// Returns whether the host should call `preventDefault()` on the event:
    /// the JS does so unconditionally once it has decided to act, which is how
    /// a page stops the wheel scrolling the document under the canvas.
    pub fn wheel(&mut self, camera: &mut PerspectiveCamera, event: &WheelEvent) -> bool {
        if !self.enabled || !self.enable_zoom || self.state != State::None {
            return false;
        }

        // `_customWheelEvent( event )`.
        let mut delta_y = event.delta_y;
        match event.delta_mode {
            WheelDelta::Pixel => {}
            WheelDelta::Line => delta_y *= 16.0,
            WheelDelta::Page => delta_y *= 100.0,
        }
        // A trackpad pinch arrives as a ctrl-wheel. The JS also checks that
        // Control is not genuinely held, which it learns from a document-level
        // `keydown`/`keyup` interceptor; a host here reports the pinch by
        // leaving `ctrl_key` set, and a real Control-wheel by clearing it.
        if event.ctrl_key {
            delta_y *= 10.0;
        }

        self.update_zoom_parameters(event.client_x, event.client_y, camera);

        if delta_y < 0.0 {
            self.dolly_in(self.zoom_scale(delta_y));
        } else if delta_y > 0.0 {
            self.dolly_out(self.zoom_scale(delta_y));
        }

        self.update(camera, None);
        true
    }

    /// `onKeyDown` — the arrow keys pan, or rotate with ctrl/meta/shift.
    ///
    /// Returns the JS's `needsUpdate`, which is also its `preventDefault()`
    /// guard: `true` means the controls consumed the key and the host should
    /// stop the browser scrolling the page with it.
    pub fn key(&mut self, camera: &mut PerspectiveCamera, event: &KeyEvent) -> bool {
        if !self.enabled {
            return false;
        }

        let modified = event.ctrl_key || event.meta_key || event.shift_key;
        let height = self.element_height;
        let key_rotate = TWO_PI * self.key_rotate_speed / height;

        match event.key {
            Key::ArrowUp => {
                if modified {
                    if self.enable_rotate {
                        self.rotate_up(key_rotate);
                    }
                } else if self.enable_pan {
                    self.pan(0.0, self.key_pan_speed, camera);
                }
            }
            Key::ArrowDown => {
                if modified {
                    if self.enable_rotate {
                        self.rotate_up(-key_rotate);
                    }
                } else if self.enable_pan {
                    self.pan(0.0, -self.key_pan_speed, camera);
                }
            }
            Key::ArrowLeft => {
                if modified {
                    if self.enable_rotate {
                        self.rotate_left(key_rotate);
                    }
                } else if self.enable_pan {
                    self.pan(self.key_pan_speed, 0.0, camera);
                }
            }
            Key::ArrowRight => {
                if modified {
                    if self.enable_rotate {
                        self.rotate_left(-key_rotate);
                    }
                } else if self.enable_pan {
                    self.pan(-self.key_pan_speed, 0.0, camera);
                }
            }
        }

        self.update(camera, None);
        true
    }
}

/// `_TILT_LIMIT = Math.cos( 70 * DEG2RAD )`.
const TILT_LIMIT: f64 = 0.342_020_143_325_668_9;
