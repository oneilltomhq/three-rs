# Changelog

All notable changes to `three-rs` are documented here. `sdf-text` and
`three-rs-controls`, the other crates in this workspace, version separately and
have their own sections after the release they ship with. The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

## [Unreleased]

## [0.2.0] - 2026-09-29

Ships with `sdf-text` 0.2.0 and the first release of `three-rs-controls`
(0.1.0); their sections follow this one.

### Changed

Breaking changes, settled in [`docs/api.md`](docs/api.md) after 0.1.0's first
outside consumers and batched together for the 0.2.0 release:

- **`Node` is a newtype**, not a `pub type Node = Rc<RefCell<Object3D>>` alias,
  with the tree operations (`add`, `children`, `matrix_world`, ...) inherent
  instead of living on a separately-imported `Object3DNode` trait. Call sites
  are unchanged; `use three_rs::core::Object3DNode` is no longer needed.
  `WeakNode` gets the same treatment and `Rc::ptr_eq` becomes `Node::ptr_eq`.
  In the same change, **`Mesh::new` takes the material**:
  `Mesh::new(geometry, material)`, matching `new Mesh(geometry, material)`,
  instead of `Mesh::new(geometry)` followed by setting `.material` by hand.
  (#72)
- **Fallible entry points return `Result`.** Loaders, `Renderer::new`,
  `Renderer::with_instance`, `Renderer::read_canvas_pixels`,
  `RenderTarget::new_with_options`, `DepthTexture::set_type`,
  `AnimationClip`/`KeyframeTrack` parsing and `VectorFont::parse` return
  `three_rs::Error` (or `sdf_text::Error`) instead of panicking. Both error
  enums are `#[non_exhaustive]`. Constructors, geometry builders and the rest
  of the scene-graph API stay infallible. (#74)
- **`Renderer::with_device(parameters, adapter, device, queue)`** lets a host
  that already owns a `wgpu::Device` and `Queue` hand them to the renderer
  instead of the renderer creating its own; `Renderer::new` and
  `with_instance` now end in it. `Texture::external(gpu, color_space)` wraps
  a `wgpu::Texture` the renderer never re-uploads or destroys, and
  `Texture::set_data` / `set_needs_update` / `version` let a texture whose
  pixels change every frame (a screencast, a shared-memory client buffer)
  keep its GPU allocation instead of being recreated. (#64)
- **Passes render themselves.** `PassNode`, `RttNode` and `BloomNode` are now
  `Rc` handles with `&self` setters; give one its scene and camera with
  `pass(scene, camera)` or `set_scene`, and the renderer runs it from its
  `updateBefore()` hook the first time a frame samples the pass's texture,
  the way three.js does. Their explicit `render()` methods are deprecated
  (see *Removed*). (#206)
- **A scene's or material's environment is an `Environment` enum** —
  `Environment::Pmrem(PmremHandle)` or `Environment::Node(EnvironmentNode)` —
  instead of a raw PMREM handle, so an environment can be a generated cube or
  a graph of `pmremTexture()` reads. (#208)
- **Options structs are `#[non_exhaustive]`**, so a field added later is not
  a break: `PassOptions`, `RenderTargetOptions`, `RendererParameters`,
  `GaussianBlurOptions`, `BoxBlurOptions`, `HashBlurOptions`,
  `RadialBlurOptions`, `Billboarding`, `ExtrudeGeometryOptions`,
  `TextGeometryOptions`, `ReflectorParameters`, `OverrideNodes`,
  `Ktx2Support`, `RaycasterParams`, and sdf-text's `LayoutParams` and
  `BatchedTextOptions`. Replace `RendererParameters { antialias: true }` (and
  `S { a, ..S::default() }`) with `let mut p = RendererParameters::default();
  p.antialias = true;`. `ComputeFlow` literals become `ComputeFlow::new(
  statements, count)`, with `workgroup_size`, `name` and `on_init` set as
  fields afterwards; `MaterialFlow` literals become `MaterialFlow::new(output,
  position)`. `Info`, `RenderCounts`, `BuildCounts`, `MemoryCounts`,
  `ComputeCounts` and sdf-text's `TextRenderInfo` are `#[non_exhaustive]` too:
  read their fields, and destructure them with `..`. (`docs/api.md` decision
  3; #217)
- **One `ColorSpace`.** `math::ColorSpace` and `textures::ColorSpace` are
  merged into `three_rs::math::ColorSpace` (also `three_rs::ColorSpace`),
  with the variants `NoColorSpace`, `Srgb` and `LinearSrgb`. Replace
  `textures::ColorSpace` with `math::ColorSpace`. `ColorManagement::convert`,
  `working_to_color_space`, `color_space_to_working` and `get_transfer` take a
  `ColorSpace` instead of an `Option<ColorSpace>`: write
  `ColorSpace::NoColorSpace` where you passed `None`, and drop the `Some(..)`.
  The HDR, cube HDR, Ultra HDR and KTX2 loaders now tag linear textures
  `LinearSrgb` rather than `NoColorSpace`, as three.js does. Both sample the
  same way. (#216)
- **Acronyms in public names use Rust casing** (API Guidelines C-CASE), with
  the three.js name in the doc comment:
  `ColorSpace::SRGB` becomes `Srgb`, `ColorSpace::LinearSRGB` becomes `LinearSrgb`,
  `GLTFLoader` becomes `GltfLoader`, and
  `CoordinateSystem::WebGL`/`WebGPU` become `WebGl`/`WebGpu`.
  Dimension suffixes (`Data3DTexture`), axis orders (`EulerOrder::XYZ`) and
  glam-style vector types (`Type::UVec2`) are unchanged; see
  [`docs/api.md`](docs/api.md) decision 3. (#216)
- **The public surface was audited** (`docs/api.md` decision 11, #213).
  About 290 items that were public only for the crate's own tests and
  examples are now `pub(crate)`, 79 more are `#[doc(hidden)]` (the grader
  readbacks, the `draco`/`meshopt` loader modules), and about 25 dead items
  are gone, among them `Renderer::program_builds` and `ColorManagement`'s
  deprecated helpers. Items re-exported one level up lost their long paths:
  import them from the parent module. `CurveVector` is sealed. 18 enums are
  `#[non_exhaustive]`, `nodes::Node` among them, so match them with a `_`
  arm.
- **`RenderCamera` is sealed**, matching `CurveVector`: it cannot be
  implemented for a type outside this crate. Nothing that legitimately used
  it as a trait object or bound needs to change. (#218)
- **`NodeRef`, `RootId` and `ActionHandle` are opaque handles**: their tuple
  field is private. `NodeRef::node()` (or `as_rc()` for the `Rc<Node>`
  itself) replaces reading `.0`; `RootId::MIXER_ROOT` replaces
  `RootId(0)`. `ActionHandle` and `RootId` are otherwise unchanged — get one
  from the `AnimationMixer` that owns it. (#218)

Other changes:

- The `rust-version` in `Cargo.toml` is documented as 1.90, which is what the
  lockfile already needed. (#73)

### Added

- **glTF loading.** `GltfLoader` for `.gltf`/`.glb` (#119), with
  `KHR_materials_sheen` (#122), `KHR_materials_anisotropy` (#124),
  `KHR_draco_mesh_compression` via `draco-core` (#150), `KHR_texture_basisu`
  / KTX2 textures (#176), `EXT_meshopt_compression` (#182),
  `EXT_texture_webp` (#183, AVIF evaluated and left out), and
  `KHR_materials_diffuse_roughness` with the EON diffuse lobe (#204). A required
  extension the loader cannot read is now a load error rather than a
  silently wrong scene (#125), and every glTF texture reference is a
  `GltfTextureRef` carrying its own sampler and wrap state (#127).
- **Post-processing.** SSAA (#100), selective bloom and `uniformArray()`
  (#108), a previous-frame texture and pass `getOutput` hook (#109),
  anamorphic flare (`rtt()`, node-valued loop bounds, mirrored-repeat wrap)
  (#111), chromatic aberration (#118), a node library covering GaussianBlur,
  Sobel, DotScreen, RGBShift, Transition, AfterImage, Pixelation, hashBlur
  and boxBlur (#148), FXAA (#197) and halftone (#209).
- **PMREM environment lighting.** HDR and half-float texture loading and
  readback (#102); `PMREMGenerator` ports `from_cubemap` (#105),
  `from_equirectangular` (#107, plus `UltraHDRLoader`, #117), `from_scene`
  (#110, #112) and the furnace-test path (#110), each gated against three's
  own numbers; `RoomEnvironment` (#118); an LDR cubemap upload path for
  `webgpu_materials_envmaps` and `_cubemap_mipmaps` (#114).
- **Compute.** Compute kernels and storage buffers (#96); indirect
  draw/dispatch, atomics and workgroup memory (#178); `Data3DTexture` and
  storage textures (#175).
- **Rendering.** Multiple render targets and MRT readback (#106, #207);
  `Renderer::copy_texture_to_texture` and in-place partial texture updates
  (#202); Basic and VSM shadow filters with `filterNode`/`shadowNode` hooks
  (#177); occlusion queries and `isOccluded` (#190); `ArrayCamera` and
  user-defined `LightingModel`s (#191); `reflector()`, a planar mirror node
  (#208); layer-based render filtering (#203); `MeshToonNodeMaterial` and a
  toon outline pass (#193); `BatchedMesh` drawn as one `drawIndexed` per
  range (#95); fat lines (`Line2NodeMaterial` and the `lines` addon) with
  viewport/scissor state and `CatmullRomCurve3` (#101, #104); `Raycaster`,
  `Sprite` and instanced sprites (#149, #192); skinned and instanced points
  (#199); an `alphaHash` blend mode and multi-material geometry groups
  (#201); `textureGrad`/`textureGather` sampling (#188); per-object uniform
  buffers and `wgslFn` varyings (#116); `wgslFn` and a MaterialX node
  function library for TSL (#98, #151); the direct clearcoat lighting lobe,
  with every other unsupported material field now warning once (or, via
  `check_supported()`/`unsupported_fields()`, failing before the first
  frame) instead of being silently ignored (#181).
- **Nodes.** `Node::Custom(Rc<dyn CustomNode>)` for user node types that
  compose existing variants, `tsl::custom()`, and `tsl::context()` /
  `tsl::isolate()` (`.context()` / `.isolate()`) for three's `ContextNode`
  and `IsolateNode`, sharing one parent-chained cache during a build (#185,
  #189, #194).
- **Geometry, math and cameras.** `Shape`, `Path` and the curve family
  (`CurvePath`, Béziers, ellipses/arcs, splines) with `ShapeUtils`/Earcut
  triangulation (#180); `Fog` and `FogExp2` behind `scene.fog` (#145);
  `Box3::set_from_object` and a camera-fit helper (#79); `project`/
  `unproject` on any camera, and `crossed` (#77); `OrthographicCamera::
  look_at`, matching the perspective camera (#83); `RenderCamera::
  set_view_offset` and render-pipeline before/after hooks (#186); mutable
  geometry attributes and render-target readback (#81); a cached geometry
  bounding box and sphere (#136).
- **Controls.** `OrbitControls`, in the native viewer and the browser
  (#130); map-style controls over a curved ground (#86).
- **Browser.** The graded examples run in the browser on wasm32 + WebGPU,
  animated on a clock (#129, #130), with a live gallery published to GitHub
  Pages (#131).
- **Diagnostics.** `Renderer::info()` reports per-frame draw-call and
  triangle counts (#76).
- **Documentation.** Every public item in `three-rs`, `sdf-text` and
  `three-rs-controls` has a doc comment naming the three.js (or lib3) item it
  ports, each crate has a crate-level overview with an example, and
  `#![warn(missing_docs)]` keeps it that way (#215, #219–#225). A
  `CHANGELOG.md`, a code of conduct and issue templates were added (#212).
- 69 more three.js examples graded green since 0.1.2, for 79 in total — see
  the README's gallery table for the full list.

### Fixed

- `texture()` reads an unfilterable map with `textureLoad` instead of a
  filtering sample. (#121)
- An instanced attribute uploads once per array instead of once per draw.
  (#90)
- The renderer keeps uniform buffers, views, samplers and bind groups across
  frames instead of rebuilding them every frame. (#138)
- The texture and view caches are swept by liveness, closing a leak on
  long-running scenes. (#174)
- `map` is read on every material path, and `vertexColors` is honoured.
  (#78)
- Non-skinned glTF materials read a `vec4` `COLOR_0`, matching skinned ones.
  (#113)

### Removed

- **`PassNode::render`, `RttNode::render` and `BloomNode::render()`**,
  deprecated since the passes-render-themselves change above, are removed.
  Give a pass its scene and camera and the renderer runs it automatically.
- `d3-hierarchy` moved out of this workspace into its own repository; its
  examples and gates no longer live here. The SDF examples and gates moved
  into `sdf-text` in the same change. (#66)

## sdf-text 0.2.0 - 2026-09-29

Breaking, because of the `Result` and `Option` signatures below; it depends on
`three-rs` 0.2.

### Changed

- `sdf_text::Error`, a `#[non_exhaustive]` crate-level error enum, replaces
  the remaining panics on fallible paths, mirroring `three-rs::Error`.
- **`BatchedText::add_text` returns `Option<usize>`** (`None` at capacity)
  and **`Text::member_id` returns `Option<usize>`** (`None` when the member
  has not joined a batch), replacing the `-1`-sentinel `i64` both used to
  return. (#218)

### Added

- `VectorFont::measure()`, for laying out text before there is an object to
  measure. (#75)
- A refilled `BatchedText`'s newly-added glyphs draw without waiting for a
  full rebuild. (#85)
- Batch members honour their own full `matrix_world`, and a batch's bounds
  come from its members rather than its node. (#75)
- Roboto ships inside the package so the example runs standalone. (#75)

## three-rs-controls 0.1.0 - 2026-09-29

### Added

- First release: `MapControls`, a map camera over a ground with curvature,
  in the spirit of three.js' `MapControls` and damped the way camera-controls
  damps, for `three-rs` 0.2. (#86, #92)

## [0.1.2] - 2026-09-13

### Fixed

- Geometry, attribute and texture caches are keyed by never-reused ids
  instead of `Rc` address, closing a freed-address ABA bug a two-plane
  dendrogram scene hit. (#58)

_Ships alongside the 0.1.1 fix, both already on `main` before this tag._

## [0.1.1] - 2026-09-13

### Fixed

- Program cache keys are bindings hashed by identity and shape instead of
  their `Debug` string, so a frame with a large texture no longer formats
  its pixels into a cache key. (#56)

## [0.1.0] - 2026-09-13

### Added

- First crates.io release: three.js core and math (`Object3D` scene graph,
  `BufferGeometry`, cameras, `Vector`/`Matrix`/`Quaternion`/`Euler`/`Color`),
  the `WebGPURenderer` port on wgpu with a TSL-generated node system, the
  core materials, lights and shadow maps, and the first graded examples.

[Unreleased]: https://github.com/oneilltomhq/three-rs/compare/v0.2.0...HEAD
[0.2.0]: https://github.com/oneilltomhq/three-rs/compare/v0.1.2...v0.2.0
[0.1.2]: https://github.com/oneilltomhq/three-rs/compare/v0.1.1...v0.1.2
[0.1.1]: https://github.com/oneilltomhq/three-rs/compare/v0.1.0...v0.1.1
[0.1.0]: https://github.com/oneilltomhq/three-rs/releases/tag/v0.1.0
