//! Ports of `three.js/examples/jsm/tsl/display/` — the addon effect `Fn()`s a
//! `RenderPipeline`'s `outputNode` is built from.
//!
//! They live in the main crate rather than an `addons/` member because they are
//! node graphs, not scene-graph objects: an effect is a pure function of the
//! pass texture and a handful of uniforms, and it needs nothing from the
//! renderer that `src/nodes/tsl.rs` does not already export.

mod radial_blur;

pub use radial_blur::{radial_blur, RadialBlurOptions};
