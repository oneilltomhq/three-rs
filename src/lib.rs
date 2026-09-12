//! `three-rs` — a port of three.js core + `WebGPURenderer` onto wgpu, grown one
//! rung at a time against three.js' own e2e image comparison.
//!
//! Every type here is a port of a specific three.js source file; the module doc
//! comments name it. Nothing is added that the current rung's example does not
//! use.

pub mod cameras;
pub mod core;
pub mod geometries;
pub mod materials;
pub mod math;
pub mod objects;
pub mod renderer;
pub mod testing;
pub mod textures;

pub use cameras::PerspectiveCamera;
pub use core::{BufferGeometry, Object3D};
pub use geometries::torus_knot_geometry;
pub use materials::{ColorNode, MeshBasicNodeMaterial};
pub use math::{Color, Euler, Matrix4, Quaternion, Vector3};
pub use objects::{Mesh, QuadMesh, Scene};
pub use renderer::{RenderTarget, Renderer, RendererParameters};
pub use textures::{DepthTexture, TextureType};
