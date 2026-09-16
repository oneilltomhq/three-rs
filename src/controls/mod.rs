//! A camera over a ground, and the ground.
//!
//! [`Ground`] is a sphere with a distinguished point at the world origin and
//! arc-length coordinates around it; at its largest radius it is a plane to the
//! eye. [`MapControls`] drives a camera over it the way three.js'
//! `MapControls` drives one over a map — the ground is what you grab — with
//! `camera-controls`' `smoothDamp` on every field.
//!
//! ```no_run
//! use three_rs::cameras::PerspectiveCamera;
//! use three_rs::controls::{Ground, MapControls, Pose};
//!
//! let mut controls = MapControls::new(
//!     Ground::new(1e7),
//!     Pose {
//!         u: 0.0,
//!         v: 0.0,
//!         distance: 160.0,
//!         azimuth: 0.0,
//!         polar: 0.0,
//!     },
//! );
//!
//! let mut camera = PerspectiveCamera::new(60.0, 16.0 / 9.0, 1.0, 50_000.0);
//! controls.update(1.0 / 60.0);
//! controls.apply(&mut camera);
//! ```

mod ground;
mod map_controls;

pub use ground::{Frame, Ground, MAX_RADIUS, MIN_RADIUS};
pub use map_controls::{
    Damping, MapControls, Mode, Pane, Pose, DRAGGING_SMOOTH_TIME, MAX_DISTANCE, MAX_POLAR,
    MIN_DISTANCE, MIN_POLAR, PANE_LIFT, REST_THRESHOLD, SMOOTH_TIME, WHEEL_SMOOTH_TIME,
};
