//! `three-rs` — a port of three.js core + `WebGPURenderer` onto wgpu, grown one
//! rung at a time against three.js' own e2e image comparison.
//!
//! Every type here is a port of a specific three.js source file; the module doc
//! comments name it. Nothing is added that the current rung's example does not
//! use.

pub mod cameras;
pub mod core;
pub mod geometries;
#[cfg(not(feature = "nodes-only"))]
pub mod loaders;
pub mod materials;
pub mod math;
pub mod nodes;
#[cfg(not(feature = "nodes-only"))]
pub mod objects;
#[cfg(not(feature = "nodes-only"))]
pub mod renderer;
pub mod testing;
pub mod textures;

pub use cameras::PerspectiveCamera;
pub use core::{BufferGeometry, Object3D};
pub use geometries::{sphere_geometry, torus_knot_geometry};
#[cfg(not(feature = "nodes-only"))]
pub use loaders::{BufferGeometryLoader, CubeTextureLoader};
pub use materials::MeshBasicNodeMaterial;
pub use math::{Color, Euler, Matrix3, Matrix4, Quaternion, Vector2, Vector3};
#[cfg(not(feature = "nodes-only"))]
pub use objects::{Background, Child, InstancedMesh, Mesh, QuadMesh, Scene};
#[cfg(not(feature = "nodes-only"))]
pub use renderer::{RenderTarget, Renderer, RendererParameters};
pub use textures::{ColorSpace, CubeTexture, DepthTexture, Mapping, Texture, TextureType};
