//! Port of `three.js/examples/jsm/` — the addons, the tier above the core
//! library that three.js ships from a separate directory and that applications
//! import as `three/addons/…`.
//!
//! The crate keeps them in a module rather than a workspace crate (which is
//! what `scouts/webgpu_lines_fat/PLAN.md` §9 proposed) for one reason: the e2e
//! harness pulls each example into `tests/e2e/main.rs` with `#[path =
//! "../../examples/…"]`, so an example that lived in another crate would need
//! its own test binary and the ladder would stop being one command. Where an
//! addon needs no core change — `controls` — it is a workspace crate, and that
//! stays the preferred shape. See `README.md`, "Addons".
//!
//! [`controls::OrbitControls`] is the exception, and for the harness reason
//! above: 25 of the 39 graded examples create one in their `init()`, and an
//! example included by path cannot reach a crate that depends on this one. The
//! `three-rs-controls` workspace crate is a different addon — a map camera
//! over a ground, not a port of anything in three.js — and stays where it is.

pub mod controls;
pub mod geometry_utils;
pub mod lines;
