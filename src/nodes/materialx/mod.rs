//! Port of `three.js/src/nodes/materialx/` — the MaterialX node library three
//! ships auto-converted from MaterialX's own GLSL.
//!
//! [`mx_nodes`] is the public surface (`MaterialXNodes.js`, which is what TSL
//! re-exports); the other modules hold the converted functions behind it.

pub mod mx_nodes;
mod mx_noise;

pub use mx_nodes::*;
