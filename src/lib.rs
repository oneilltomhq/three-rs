//! `three-rs` — a port of three.js core + `WebGPURenderer` onto wgpu, grown one
//! rung at a time against three.js' own e2e image comparison.
//!
//! Every type here is a port of a specific three.js source file; the module doc
//! comments name it. Nothing is added that the current rung's example does not
//! use.

pub mod animation;
pub mod cameras;
pub mod core;
pub mod geometries;
pub mod lights;
pub mod loaders;
pub mod materials;
pub mod math;
pub mod nodes;
pub mod objects;
pub mod renderer;
/// Hidden from the rendered docs on purpose (#40). It stays `pub` because the
/// examples, the viewer, the e2e tests, the `sdf-text` crate's gates and
/// consumers' own graders use it; it is not part of the renderer's API and
/// carries no compatibility promise.
#[doc(hidden)]
pub mod testing;
pub mod textures;

pub use cameras::{OrthographicCamera, PerspectiveCamera, RenderCamera};
pub use core::{BufferGeometry, Node, Object3D};
pub use geometries::{
    box_geometry, plane_geometry, quad_geometry, sphere_geometry, teapot_geometry,
    torus_knot_geometry,
};
pub use lights::{
    AmbientLight, DirectionalLight, HemisphereLight, Light, LightKind, LightObject, LightShadow,
    PointLight, ShadowCamera, SpotLight,
};
pub use loaders::{BufferGeometryLoader, CubeTextureLoader, TextureLoader};
pub use materials::{
    LineBasicNodeMaterial, MaterialKind, MeshBasicNodeMaterial, MeshPhongNodeMaterial,
    MeshPhysicalNodeMaterial, MeshStandardNodeMaterial, ToneMapping,
};
pub use math::{Color, Euler, Matrix3, Matrix4, Quaternion, Vector2, Vector3};
pub use objects::{
    Background, Group, InstancedMesh, Line, LineSegments, Mesh, Payload, QuadMesh, Scene,
};
pub use renderer::{PassNode, RenderPipeline, RenderTarget, Renderer, RendererParameters};
pub use textures::{
    ColorSpace, CubeTexture, DepthTexture, Mapping, MinFilter, Texture, TextureFilter, TextureType,
};
