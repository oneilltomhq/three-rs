//! Ports of `three.js/test/unit/src/geometries/*.tests.js`.
//!
//! Laid out as one integration-test binary with the per-geometry modules under
//! `tests/geometry/` so no new `[[test]]` target is needed in `Cargo.toml`.

#[path = "geometry/support.rs"]
mod support;

#[path = "geometry/batch1.rs"]
mod batch1;

#[path = "geometry/teapot.rs"]
mod teapot;
