//! Port of `three.js/src/nodes/` — the node system the WebGPU renderer builds
//! every material through. See `docs/nodes.md`.

pub mod batch;
pub mod builder;
pub mod code;
pub mod display;
pub mod lines;
pub mod materialx;
pub mod morph;
pub mod mrt;
pub mod node;
pub mod pmrem_node;
pub mod pmrem_utils;
pub mod skinning;
pub mod tsl;
pub mod wgsl;

pub use builder::{
    BindingDesc, ComputeFlow, ComputeProgram, MaterialFlow, NodeBuilder, NodeProgram, Stage,
    UniformMember, Visibility,
};
pub use mrt::{mrt, MrtNode, MrtValue};
pub use node::{
    BufferSource, Node, NodeRef, TextureSource, Type, UniformGroup, UniformSource, UpdateType,
};
