//! `three-rs` — a port of three.js core + `WebGPURenderer` onto wgpu, grown one
//! rung at a time against three.js' own e2e image comparison.
//!
//! Every type here is a port of a specific three.js source file; the module doc
//! comments name it. Nothing is added that the current rung's example does not
//! use.

pub mod addons;
pub mod animation;
pub mod cameras;
pub mod core;
pub mod environments;
pub mod error;
pub mod extras;
pub mod geometries;
pub mod helpers;
/// The one seam through which the loaders read bytes: `std::fs` natively, a
/// host-preloaded map in a browser (issue #128).
pub mod io;
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
pub mod utils;

pub use cameras::{OrthographicCamera, PerspectiveCamera, RenderCamera};
pub use core::{BufferGeometry, Node, Object3D, Timer};
pub use environments::RoomEnvironment;
pub use error::{Error, GltfError};
pub use extras::{CatmullRomCurve3, Curve, CurveType, FrenetFrames};
pub use geometries::{
    box_geometry, plane_geometry, quad_geometry, sphere_geometry, teapot_geometry,
    torus_knot_geometry,
};
pub use helpers::GridHelper;
pub use lights::{
    AmbientLight, DirectionalLight, HemisphereLight, Light, LightKind, LightObject, LightShadow,
    PointLight, ShadowCamera, SpotLight,
};
pub use loaders::{
    BufferGeometryLoader, CubeTextureLoader, HdrCubeTextureLoader, HdrLoader, TextureLoader,
};
pub use materials::{
    Line2NodeMaterial, LineBasicNodeMaterial, MaterialKind, MeshBasicNodeMaterial,
    MeshLambertNodeMaterial, MeshNormalNodeMaterial, MeshPhongNodeMaterial,
    MeshPhysicalNodeMaterial, MeshStandardNodeMaterial, PointsNodeMaterial, ToneMapping,
};
pub use math::{Color, Euler, Matrix3, Matrix4, Quaternion, Vector2, Vector3};
pub use objects::{
    Background, Fog, FogExp2, Group, InstancedMesh, Line, LineSegments, Mesh, Payload, Points,
    QuadMesh, Scene, SceneFog,
};
pub use renderer::{
    BuildCounts, ComputeCounts, DirectRenderPipeline, Info, MemoryCounts, PassNode, RenderCounts,
    RenderPipeline, RenderTarget, Renderer, RendererParameters, SsaaPassNode, BACKENDS,
};
pub use textures::{
    ColorSpace, CubeTexture, DepthTexture, Mapping, MinFilter, Texture, TextureFilter, TextureType,
};
