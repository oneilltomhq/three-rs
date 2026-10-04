//! Ports of `three.js/examples/jsm/controls/`.
//!
//! [`OrbitControls`], which 159 of Three's 230 WebGPU example pages create
//! and 25 of the 39 graded ones do, [`FirstPersonControls`],
//! [`FlyControls`] and [`TransformControls`]. See `docs/controls.md`.

mod first_person_controls;
mod fly_controls;
mod key_code;
mod orbit_controls;
mod transform_controls;

pub use first_person_controls::FirstPersonControls;
pub use fly_controls::{FlyControls, MoveState};
pub use key_code::KeyCode;

pub use orbit_controls::{
    Key, KeyEvent, MouseAction, MouseButton, MouseButtons, OrbitControls, PointerEvent, WheelDelta,
    WheelEvent,
};
pub use transform_controls::{
    Axis, Mode, Pointer, PointerType, Space, TransformCamera, TransformControls,
    TransformControlsEvent, TransformPointerEvent,
};
