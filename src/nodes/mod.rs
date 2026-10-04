//! Port of `three.js/src/nodes/` — the node system the WebGPU renderer builds
//! every material through. See `docs/nodes.md`.

pub(crate) mod alpha_hash;
pub mod batch;
pub mod builder;
pub mod clipping;
pub mod code;
pub mod display;
pub(crate) mod frame;
pub mod lines;
pub mod materialx;
pub mod morph;
pub(crate) mod mrt;
pub mod node;
pub mod pmrem_node;
pub mod pmrem_utils;
pub mod reflector_node;
pub mod skinning;
pub mod tsl;
pub mod velocity;
pub mod wgsl;

pub use builder::{
    BindingDesc, ComputeFlow, ComputeProgram, MaterialFlow, NodeBuilder, NodeProgram,
    UniformMember, Visibility,
};
pub(crate) use frame::UpdateNode;
pub use frame::{NodeFrameState, NodeUpdate, NodeUpdateType};
pub use mrt::{get_texture_index, mrt, MrtNode};
pub use node::{
    BufferSource, ContextValue, CustomNode, DebugCallback, DebugInfo, Node, NodeFrame, NodeRef,
    Object3DScope, StorageAccess, TextureSource, Type, UniformGroup, UniformSource, UpdateType,
};
