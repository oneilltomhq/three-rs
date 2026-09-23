//! Ports of `three.js/examples/jsm/controls/`.
//!
//! One so far: [`OrbitControls`], which 159 of Three's 230 WebGPU example
//! pages create and 25 of the 39 graded ones do.

mod orbit_controls;

pub use orbit_controls::{
    Key, KeyEvent, MouseAction, MouseButton, MouseButtons, OrbitControls, PointerEvent, WheelDelta,
    WheelEvent,
};
