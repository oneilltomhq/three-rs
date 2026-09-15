//! Camera controls: a helicopter over a ground that may be a plane or a
//! sphere.
//!
//! Not a port of anything in three.js — `OrbitControls` and friends live in
//! `examples/jsm`, not in the core this crate mirrors. The damping is a port of
//! yomotsu's `camera-controls`
//! ([`smooth_damp`](crate::math::math_utils::smooth_damp) and its two smooth
//! times); the pose model is this crate's own.
//!
//! ```no_run
//! use three_rs::controls::{Ground, Helicopter, Pose};
//! use three_rs::PerspectiveCamera;
//!
//! let mut heli = Helicopter::new(
//!     Ground::new(1e7),
//!     Pose { u: 0.0, v: -120.0, altitude: 40.0, yaw: 0.0, pitch: -0.26 },
//! );
//! let mut camera = PerspectiveCamera::new(60.0, 1.6, 1.0, 50_000.0);
//!
//! heli.move_ground(1.0, 0.0, 1.0 / 60.0);
//! if heli.update(1.0 / 60.0) {
//!     heli.apply(&mut camera);
//! }
//! ```

mod ground;
mod helicopter;

pub use ground::{Frame, Ground, MAX_RADIUS, MIN_RADIUS};
pub use helicopter::{
    Helicopter, Mode, Pane, Pose, DRAGGING_SMOOTH_TIME, MAX_ALTITUDE, MAX_PITCH, MIN_ALTITUDE,
    MIN_PITCH, REST_THRESHOLD, SMOOTH_TIME,
};
