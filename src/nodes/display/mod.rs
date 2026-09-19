//! Ports of `three.js/examples/jsm/tsl/display/` — the addon effect `Fn()`s a
//! `RenderPipeline`'s `outputNode` is built from.
//!
//! They live in the main crate rather than an `addons/` member because they are
//! node graphs, not scene-graph objects: an effect is a pure function of the
//! pass texture and a handful of uniforms, and it needs nothing from the
//! renderer that `src/nodes/tsl.rs` does not already export.
//!
//! [`bloom`] is the exception, and it is three.js' exception too: `BloomNode`
//! is a `TempNode` that owns eleven render targets and draws twelve quads of
//! its own in `updateBefore()`, so it reaches the renderer the way
//! [`SsaaPassNode`](crate::renderer::SsaaPassNode) does.

mod bloom;
mod radial_blur;
mod rtt;

pub use bloom::{bloom, luminosity_high_pass, BloomNode, HighPassInput};
pub use radial_blur::{radial_blur, RadialBlurOptions};
pub use rtt::{convert_to_texture, rtt, RttNode};
