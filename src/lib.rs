//! `three-rs` — a scene graph and a `wgpu`-based renderer, ported from
//! three.js.
//!
//! # Relationship to three.js
//!
//! `three-rs` is a line-for-line port of three.js' core scene graph
//! (`src/core`, `src/objects`, `src/math`, …) and its `WebGPURenderer`, grown
//! one rung at a time: each ported example's shaders come out of the same
//! node graph (see [`nodes`]) that three.js' own `WebGPURenderer` builds them
//! from, rather than being hand-written, and a rung counts as correct only
//! when it reproduces the pixels three.js' own end-to-end suite already
//! captured for that page (`three.js/test/e2e`). Every type here is a port of
//! a specific three.js source file; the module doc comments below and on
//! individual items name it, so three.js' own docs and source are the place
//! to go for semantics this crate's doc comments do not repeat.
//!
//! # Ownership
//!
//! The scene graph is single-threaded by design, not a 0.x limitation:
//! nothing in it is `Send` or `Sync`. three.js' graph is single-threaded too,
//! and `WebGPURenderer` never touches an `Object3D` off the main thread;
//! parallelism belongs beside the scene rather than inside it — asset
//! decoding, layout, rasterising and filesystem scans run on other threads
//! and hand finished data to the scene thread, `wgpu` itself is thread-safe,
//! and a compositor drives the scene from one thread and sends it messages.
//! See [decision 1 of the API design note][api-md].
//!
//! A scene object is a [`Node`] — a newtype over `Rc<RefCell<Object3D>>`, so
//! `borrow()` / `borrow_mut()` work as they would on the `RefCell` directly.
//! Users hold `Node` clones across frames, as every three-rs consumer does.
//! The one place this port's ownership differs from three.js: a child's
//! `parent` link is a `Weak`, not a strong reference, so a subtree that is
//! removed and not held anywhere else is freed rather than kept alive by its
//! own children. See [decision 2 of the API design note][api-md].
//!
//! [`testing`] stays `pub` — the examples, the viewer, the e2e tests, the
//! `sdf-text` crate's gates and consumers' own graders all use its
//! `write_png` and `three_js_dir` — but it is `#[doc(hidden)]`, since it is
//! not part of the renderer's API and carries no compatibility promise.
//!
//! # Errors
//!
//! Only what touches the filesystem or the GPU is fallible: the loaders,
//! [`Renderer::new`](renderer::Renderer::new) and
//! [`render()`](renderer::Renderer::render). Constructors, geometry builders
//! and scene-graph methods are infallible, as they are in three.js. A single
//! [`Error`], `#[non_exhaustive]`, covers this crate; every remaining panic in
//! library code is a statement about the crate's own invariants (never about
//! a caller's mistake) and its message starts `three-rs:`. See
//! [decision 5 of the API design note][api-md].
//!
//! # Modules
//!
//! - [`addons`] — ports of `three.js/examples/jsm/`, the tier above core that
//!   three.js ships separately.
//! - [`animation`] — ports of `three.js/src/animation`.
//! - [`cameras`] — ports of `three.js/src/cameras`.
//! - [`core`] — ports of `three.js/src/core`: `Object3D`, [`Node`],
//!   `BufferGeometry`, the raycaster.
//! - [`environments`] — ports of `three.js/examples/jsm/environments/`, scenes
//!   built for [`renderer::pmrem::PmremGenerator::from_scene`] rather than for
//!   the screen.
//! - [`extras`] — ports of `three.js/src/extras/`: curves, paths, shapes.
//! - [`geometries`] — ports of `three.js/src/geometries`.
//! - [`helpers`] — scene-graph objects that build their own geometry, ported
//!   from `three.js/src/helpers/`.
//! - [`io`] — the one seam through which the loaders read bytes: `std::fs`
//!   natively, a host-preloaded map in a browser.
//! - [`lights`] — ports of `three.js/src/lights`.
//! - [`loaders`] — ports of `three.js/src/loaders`.
//! - [`materials`] — ports of `three.js/src/materials/nodes`; under
//!   `WebGPURenderer` every material is a `NodeMaterial`.
//! - [`math`] — ports of `three.js/src/math`.
//! - [`nodes`] — the node system `WebGPURenderer` builds every material
//!   through, ported from `three.js/src/nodes/`. See
//!   [`docs/nodes.md`][nodes-md].
//! - [`objects`] — ports of `three.js/src/objects`, plus `Scene` and the
//!   renderer's `QuadMesh`.
//! - [`renderer`] — the renderer itself, ported from
//!   `three.js/src/renderers/common/Renderer.js` and the WebGPU backend.
//! - [`testing`] (hidden) — the e2e harness' deterministic clock, RNG and
//!   pixel comparison, kept public for consumers' own graders.
//! - [`textures`] — ports of `three.js/src/textures`.
//! - [`utils`] — ports of `three.js/examples/jsm/utils`, plus the clock seam
//!   that in three.js is just the browser.
//!
//! Design notes for individual subsystems live under `docs/` rather than in
//! doc comments — for example the [API design note][api-md], the
//! [scene-graph note][scene-graph-md] and the [node-system note][nodes-md].
//!
//! [api-md]: https://github.com/oneilltomhq/three-rs/blob/main/docs/api.md
//! [scene-graph-md]: https://github.com/oneilltomhq/three-rs/blob/main/docs/scene-graph.md
//! [nodes-md]: https://github.com/oneilltomhq/three-rs/blob/main/docs/nodes.md
//!
//! # Example
//!
//! Headless: create a renderer with no window, add a lit box to a scene,
//! render one frame and read the pixels back. `no_run` because CI's `test`
//! job has no GPU adapter to create a renderer against; the shape below is
//! exactly what every ported example's `main()` does.
//!
//! ```no_run
//! use std::rc::Rc;
//!
//! use three_rs::{
//!     box_geometry, AmbientLight, Color, DirectionalLight, Mesh, MeshBasicNodeMaterial,
//!     PerspectiveCamera, Renderer, RendererParameters, Scene,
//! };
//!
//! let mut renderer = Renderer::new(RendererParameters::default())?;
//! renderer.set_size(800.0, 500.0);
//!
//! let mut scene = Scene::new();
//! scene.add(&AmbientLight::new(Color::from_hex(0x404040), 1.0));
//! scene.add(&DirectionalLight::new(Color::from_hex(0xffffff), 1.0));
//!
//! let geometry = Rc::new(box_geometry(1.0, 1.0, 1.0, 1, 1, 1));
//! let material = MeshBasicNodeMaterial::new();
//! let mesh = Mesh::new(geometry, material);
//! scene.add(&mesh);
//!
//! let mut camera = PerspectiveCamera::new(60.0, 800.0 / 500.0, 0.1, 100.0);
//! camera.node.borrow_mut().position.z = 3.0;
//!
//! renderer.render(&mut scene, &mut camera);
//! let (width, height, pixels) = renderer.read_canvas_pixels()?;
//! assert_eq!(pixels.len(), (width * height * 4) as usize);
//! # Ok::<(), three_rs::Error>(())
//! ```

#![warn(missing_docs)]
pub mod addons;
pub mod animation;
pub mod cameras;
pub mod core;
pub mod environments;
pub(crate) mod error;
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

pub use cameras::{ArrayCamera, CubeCamera, OrthographicCamera, PerspectiveCamera, RenderCamera};
pub use core::{BufferGeometry, Intersection, Node, Object3D, Raycaster, Timer};
pub use environments::RoomEnvironment;
pub use error::{Error, GltfError};
pub use extras::{
    CatmullRomCurve3, CubicBezierCurve, CubicBezierCurve3, Curve, CurvePath, CurveType,
    CurveVector, EllipseCurve, FrenetFrames, LineCurve, LineCurve3, Path, QuadraticBezierCurve,
    QuadraticBezierCurve3, Shape, ShapePath, SplineCurve,
};
pub use geometries::{
    box_geometry, plane_geometry, quad_geometry, sphere_geometry, teapot_geometry,
    torus_knot_geometry,
};
pub use helpers::{CameraHelper, GridHelper};
pub use lights::{
    AmbientLight, DirectionalLight, HemisphereLight, Light, LightKind, LightObject, LightShadow,
    PointLight, ShadowCamera, SpotLight,
};
pub use loaders::{
    BufferGeometryLoader, CubeTextureLoader, HdrCubeTextureLoader, HdrLoader, Ktx2Loader,
    TextureLoader,
};
pub use materials::{
    Line2NodeMaterial, LineBasicNodeMaterial, MaterialKind, MeshBasicNodeMaterial,
    MeshLambertNodeMaterial, MeshNormalNodeMaterial, MeshPhongNodeMaterial,
    MeshPhysicalNodeMaterial, MeshStandardNodeMaterial, MeshToonNodeMaterial, PointsNodeMaterial,
    SpriteNodeMaterial, ToneMapping,
};
pub use math::{Color, ColorSpace, Euler, Matrix3, Matrix4, Quaternion, Vector2, Vector3};
pub use objects::{
    Background, Fog, FogExp2, Group, InstancedMesh, Line, LineSegments, Mesh, Payload, Points,
    QuadMesh, Scene, SceneFog, Sprite,
};
pub use renderer::{
    pass, BuildCounts, CameraRef, ComputeCounts, CubeRenderTarget, DirectRenderPipeline, Info,
    MemoryCounts, PassNode, RenderCounts, RenderPipeline, RenderTarget, Renderer,
    RendererParameters, SceneRef, SsaaPassNode, BACKENDS,
};
pub use textures::{
    CubeTexture, Data3DTexture, DepthTexture, Mapping, MinFilter, Texture, TextureFilter,
    TextureType,
};
