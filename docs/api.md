# API design note

The decisions issue #14 asked for, settled 2026-09-13 after the first outside
consumer (a native app drawing dendrograms on two planes) built three rungs
against the published 0.1.0. Each decision records its reason so it is not
reopened per pull request. Breaking changes ship together in 0.2.0.

## 1. The scene stays single-threaded, by design

Nothing in the scene graph is `Send` or `Sync`, and that is the design rather
than a 0.x limitation. three.js's graph is single-threaded and
`WebGPURenderer` never touches an `Object3D` off the main thread; the port
inherits that shape, and it is the shape of every renderer built on the same
model. Renderers that work across threads fall into two camps: flat entity
stores with a scheduler (Bevy, Unity DOTS), or one owner thread with a copy of
the scene handed to the renderer (Unreal's proxies, Godot's servers, Filament,
rend3). Sharing a pointer tree behind locks is the third option, and
OpenSceneGraph is the cautionary tale; its successor VulkanSceneGraph went
back to a single owner. three-rs is in the second camp.

Parallelism belongs beside the scene, not inside it: asset decoding, layout,
the SDF rasteriser, and a git or filesystem scan all run on other threads and
hand finished data to the scene thread. `wgpu` itself is thread-safe. A
compositor drives the scene from one thread and feeds it messages.

`Rc`, not `Arc`; `RefCell`, not `Mutex`. The compiler refuses to let a `Node`
cross a thread, which is the guarantee wanted.

## 2. A scene object is a `Node`, a newtype over `Rc<RefCell<Object3D>>`

0.1.0 spells the handle out as a type alias, `pub type Node =
Rc<RefCell<Object3D>>`, with the tree operations on an `Object3DNode` trait.
Two costs showed up in the first consumer and in six of the fourteen examples:
every user has to discover `use three_rs::core::Object3DNode` before
`light.add(&mesh)` compiles, and rustdoc prints the plumbing in every
signature instead of a type called `Node`.

0.2.0 makes `Node` a newtype with the trait's methods inherent, and
`Deref<Target = RefCell<Object3D>>` so `borrow()` and `borrow_mut()` keep
working. Every call site stays the same text, so the examples, the QUnit
ports in `tests/`, and the line-for-line correspondence with three.js that
the pixel grader depends on are untouched. `WeakNode` gets the same
treatment; `Rc::ptr_eq` becomes `Node::ptr_eq`.

Rejected: an arena of indices, the shape a Rust reviewer expects and what the
d3-hierarchy port uses. `docs/scene-graph.md` gives the reasons and they
still hold: every three.js method would gain an arena argument, objects live
outside any scene (cameras, the instance-mesh dummy, loaded glTF trees before
they are added), `attach()` across parents stops being expressible, and the
QUnit ports have no arena to pass. The arena earns its cost only under
decision 1's first camp, which this crate is not in.

Users hold `Node` clones across frames; that is what every consumer does. The
parent link is `Weak`, the one place the port's ownership differs from
three.js: a child does not keep its parent alive, so a subtree removed and
not held is freed.

## 3. Keep three.js's shape wherever there is no real trade-off

- **Field access, not accessor layers.** `mesh.borrow_mut().position.y = -1.0`
  is the JavaScript with one extra call. A full getter and setter layer over
  `Object3D`'s public fields would hide the cell behind `update(|o| ..)`,
  which is `borrow_mut()` under another name, and would cost the ports their
  shape. Convenience setters are added one at a time where they clearly pay
  (`set_position`, light-specific passthroughs that avoid
  `light_mut().unwrap()`).
- **`Mesh::new(geometry, material)`** takes the material, as `new Mesh(
  geometry, material )` does. 0.1.0's `Mesh::new(geometry)` followed by
  `mesh.borrow_mut().mesh_mut().unwrap().material = Some(m)` is the ugliest
  line in every example and appears 22 times; `Line::new` and
  `LineSegments::new` already take the material. Breaking, 0.2.0.
- **Materials keep public fields and named constructors; no builders.**
  `MeshBasicNodeMaterial` has some 40 public fields and three.js's docs name
  them. Struct-update syntax already gives builder ergonomics:
  `MeshPhongNodeMaterial { shininess: 80.0, ..MeshPhongNodeMaterial::phong(c) }`.
  The named constructors (`phong`, `standard`, `line`, `sprite`) carry the
  per-kind defaults, which is where three.js's subclasses live.
- **Options structs are `#[non_exhaustive]`, built from `Default` or a
  constructor.** A struct that stands for one of three.js's options objects
  carries only the keys the ported pages read, so it gains a field whenever a
  rung needs one more, and with struct literals each of those was a break for
  every caller. That covers `PassOptions`, `RenderTargetOptions`,
  `RendererParameters` (one field today; `new WebGPURenderer()` takes a dozen),
  the blur `*Options`, `Billboarding`, `ExtrudeGeometryOptions`,
  `TextGeometryOptions`, `ReflectorParameters`, `OverrideNodes`, `Ktx2Support`,
  `RaycasterParams`, and sdf-text's `LayoutParams` and `BatchedTextOptions`.
  The fields stay public and `Default` holds three's defaults, so a caller
  writes `let mut options = PassOptions::default(); options.auto_clear_depth
  = false;`, the JavaScript object with its keys set one per line. That is the
  one form `#[non_exhaustive]` leaves open outside the crate: it forbids the
  literal and the `..Default::default()` update alike. A constructor is added
  only where a field has no default: `ComputeFlow::new( statements, count )`
  is three's `.compute( count )`, and `MaterialFlow::new( output, position )`
  the two nodes a hand-built flow must name. Structs callers only read — `Info`
  and its four count blocks, sdf-text's `TextRenderInfo` — are
  `#[non_exhaustive]` and nothing more. Materials keep the literal anyway.
  They are the construction surface of every example, where
  `..MeshPhongNodeMaterial::phong( c )` reads as the JavaScript does, and a
  field added to one is a breaking change taken at a minor release; that is
  the trade the bullet above made. Input events (`PointerEvent`, `WheelEvent`,
  `KeyEvent`) are DOM values rather than options and are left as literals too.
- **Lights, cameras and textures** take three.js's positional constructor
  arguments and expose the rest as fields or one-line setters, for the same
  reason.
  `LightProbe::new( sh, intensity )` returns a `Node`, as every other light
  does, and keeps its coefficients in the light's `sh` field. The addons that
  three.js writes as classes with static methods (`LightProbeGenerator`) are
  unit structs with associated functions, so the call reads
  `LightProbeGenerator::from_cube_texture( &cube )`. `CubeCamera` follows the
  cameras: it owns a `node` and its `render_target`, and its `update()` takes
  the renderer and the scene, as three's `update( renderer, scene )` does.
- **Rust names throughout**, with the three.js name in the doc comment. This
  is already how the crate is written (`set_rotation`,
  `matrix_world_needs_update`, `is_mesh()`); recorded so it is not reopened.
  It covers casing too (C-CASE): an acronym in a type, variant or trait name
  is one word, so three.js' `GLTFLoader`, `SRGBColorSpace` and
  `WebGPUCoordinateSystem` are `GltfLoader`, `ColorSpace::Srgb` and
  `CoordinateSystem::WebGpu`, as `HdrLoader`, `PmremGenerator`, `Ktx2Loader`,
  `MrtNode` and `FxaaNode` already were. Three kinds of name are not acronyms
  and keep their capitals: a dimension suffix (`Data3DTexture`,
  `TextureSource::Texture2D`), which a digit already splits and which would
  only get harder to read; an axis order (`EulerOrder::XYZ`), which is three
  one-letter axis names, as in glam's `EulerRot::XYZ`; and the glam-style
  vector types (`Type::UVec2`, `BVec3`), which are WGSL's `vec2<u32>` in the
  spelling Rust graphics code already uses. Error strings that quote
  three.js (`THREE.GLTFLoader: ...`) keep three's spelling.
- **One `ColorSpace`.** three.js' colour spaces are string constants shared by
  `ColorManagement`, `Color`, `Texture.colorSpace` and
  `renderer.outputColorSpace`, so the port has one enum for all four:
  `math::ColorSpace`, next to `ColorManagement`, re-exported at the crate root.
  `NoColorSpace` is a variant, as `''` is a constant, rather than `None` in an
  `Option`.
- **`Scene` and the cameras own a `node` field** and are not `Node`s
  themselves. `Scene` adds `background`, `fog_node` and `override_material`;
  a camera adds its projection state. Both forward `add()`, `children()` and
  `update_matrix_world()` to their node, so both can sit inside the tree.

## 4. Surface hygiene

- `three_rs::testing` is `#[doc(hidden)]`. It holds the harness helpers
  (`write_png`, `three_js_dir`) that the examples, the viewer, the e2e tests
  and consumers' own graders use, so it stays public but out of the docs.
  Decision 11 is the audit of everything else that was public.
- Only things that touch the filesystem or the GPU return `Result`: the
  loaders, `Renderer::new`, `render()`. Constructors, geometry builders and
  the scene-graph methods stay infallible, as they are in three.js. Issue #9
  did the work of replacing the remaining panics under this rule; the shape
  it settled on is decision 5.

## 5. Errors: one enum per crate, panics only on invariants

Decision 4's rule needed a shape to land in; issue #9 gave it one.

- **One error enum per crate**, `three_rs::Error` and `sdf_text::Error`, both
  `#[non_exhaustive]` and both implementing `std::error::Error`. A caller who
  wants to know what failed matches on variants that name it (`NoAdapter`,
  `UnsupportedTrackType`, `Image { path, reason }`); a caller who does not can
  `?` the lot into `Box<dyn Error>` or `anyhow`. Rejected: a per-module error
  type, which makes every loader call site declare a `From`, and a single
  `Error(String)`, which is a panic with extra steps. glTF is the one nested
  enum (`Error::Gltf(GltfError)`), because fifteen distinct failures inside
  one file format would otherwise crowd out the rest of the crate.
- **Fallible means reachable from a caller's mistake or from the machine.**
  Anything that reads a file, decodes an image, parses JSON or a track name,
  asks for an adapter or a device, or takes a texture type that may have no
  format, returns `Result`. That is the loaders, `Renderer::new` /
  `with_instance` / `read_canvas_pixels`, `RenderTarget::new_with_options`,
  `DepthTexture::set_type`, `AnimationClip` and `KeyframeTrack` parsing, and
  `VectorFont::parse`.
- **Panics are invariants, and say which one.** Every remaining panic in
  library code is a statement about the crate's own state — "the light list
  only holds lights", "prepare_canvas() has just created the canvas" — and its
  message starts `three-rs:` (or `sdf-text:`) and names it. If one of these
  fires it is a bug in the crate, never in the call. A caller can therefore
  read the rule as: handle the `Result`s, and do not write a `catch_unwind`.
- **Infallible stays infallible.** Constructors, geometry builders and the
  scene-graph methods do not return `Result`, as decision 3 says; where a
  value must be checked, the check is on the setter that takes it
  (`DepthTexture::set_type`) rather than on the accessor that would have
  panicked later, so the failure surfaces at the call that caused it.
- **`Error` is not the renderer's return channel for wrong pixels.** Nothing
  in this crate reports a rendering difference as an error; the grader does
  that. `Error` is for what did not happen at all.

## 6. The renderer can be embedded in a host that owns the GPU

Settled 2026-09-14, after the second outside consumer: a Smithay/wgpu Wayland
compositor that imports client dmabufs as `wgpu::Texture`s, draws its scene
with three-rs and scans out to DRM. Three additions, all at the boundary
between the renderer and a host that already has a device and already has
textures.

- **`Renderer::with_device( parameters, adapter, device, queue )`.** The host's
  device, adopted. `Renderer::new` and `with_instance` now end in it, so there
  is one construction path. A `wgpu::Texture` belongs to the device it was
  created on, so a renderer that always made its own device could never sample
  an imported buffer — the alternative, handing the renderer an `Instance` and
  letting it pick, is `with_instance`, and it does not solve this. `Device` and
  `Queue` are refcounted handles, so the host keeps its own clones and goes on
  using them.

  The one capability question this raises is `FLOAT32_FILTERABLE`, which
  sdf-text's `r32float` atlas needs. With an adopted device the adapter's
  feature set says nothing about what was actually enabled, so the renderer
  reads `device.features()` and asserts at the point of use; the doc comment
  tells a host that wants sdf-text to request it.

- **`Texture::external( gpu, color_space )`** — three.js's `ExternalTexture`.
  Size and format come off the `wgpu::Texture` so they cannot drift from it;
  `own_gpu = false`, so the renderer samples the handle and never re-uploads,
  re-creates or destroys it. The machinery already existed for render targets
  (`Texture::render_target` + `set_gpu`, and `ensure_texture_2d`'s `own_gpu`
  short-circuit); what was missing was a constructor that ties the four facts
  together, since `set_gpu` alone leaves size, format and colour space to be
  set by hand and silently wrong if they are not. The colour space must agree
  with the format's transfer function and that is asserted, because the GPU
  applies the transfer on sample and a mismatch is a wrong-looking frame rather
  than an error.

- **`Texture::set_data( data )` / `set_needs_update()` / `version()`** —
  `texture.needsUpdate = true`. A texture whose pixels change every frame at
  the same size and format (a screencast frame, an shm client buffer) had no
  path but clearing the GPU handle, which threw away the allocation, the mip
  chain and every bind group built from it. The renderer's texture cache is now
  keyed on `( id, version )`, exactly as the program cache is keyed on
  `( material.id, material.version )`, and a bumped version writes the new
  bytes into the texture that is already there.

  `set_data` bumps the version itself rather than waiting for a separate
  `set_needs_update()`. three.js needs the flag because its data array is
  mutated in place behind the texture's back; here the setter *is* the
  mutation, so requiring a second call would only add a silent-stale-frame
  footgun. `set_needs_update()` stays for forcing a re-upload, and the name
  keeps the three.js correspondence. `set_data` panics on the wrong byte count
  and on a texture the renderer does not own.

- **`InstancedBufferAttribute::array()` / `array_mut()` / `set_needs_update()`
  / `version()`** — `attribute.needsUpdate = true`, for `instanceMatrix` and
  `instanceColor` (#89). The renderer keeps one GPU buffer per attribute and
  writes it only when the version moves, so the array is no longer a public
  field: a write that bypassed the version would be a stale frame. The same
  reasoning as `set_data` puts the bump in the writers: `array_mut()` always
  bumps, and `set_matrix_at` / `set_color_at` bump when the value they write
  differs from the one there, so an animation loop that re-sets unchanged
  matrices uploads nothing. three.js' `setMatrixAt` needs the flag;
  here it does not.

## 7. A material field the port does not read says so

`MeshBasicNodeMaterial` is one struct for every kind (decision 3), so a field
can be set on a kind whose flow never looks at it. Issue #171 audited every
public field against what `materials::setup()` and the pipeline read, and
settled each one that was silently ignored: either it is implemented, or it is
loud. Loud means the renderer prints `three-rs: material <id> (<kind>): <field>
is set but not supported, and is ignored` once per material when it builds
the program, and `MeshBasicNodeMaterial::check_supported()` returns
`Err(Error::Unsupported { field, kind })` for an application that would rather
fail before its first frame. `unsupported_fields()` lists them all.

Implemented by the audit:

| Field | What reads it now |
|---|---|
| `depth_func` (new) | `RenderState` → `depthCompare`, as `_getDepthCompare()` maps `Material.depthFunc`. |
| `alpha_to_coverage` | the pipeline's `alphaToCoverageEnabled`, `&& sampleCount > 1` as in three. It used to be hard-wired off. |
| `emissive_node` on Phong, Lambert, Standard, Physical | `setupLighting()`'s `vec3( emissiveNode ?? materialEmissive )`. Only the Basic flow read it before. |
| `emissive_map` on Phong, Lambert | `MaterialNode.EMISSIVE`'s map multiply, as on Standard. |
| `clearcoat_map`, `clearcoat_roughness_map` (new) | `MaterialNode.CLEARCOAT` (× `.r`) and `.CLEARCOAT_ROUGHNESS` (× `.g`); `GLTFLoader` fills them from `clearcoatTexture` / `clearcoatRoughnessTexture`. |
| `clearcoat`, `clearcoat_roughness`, `clearcoat_normal_map` under a direct light | the direct clearcoat lobe in `PhysicalLightingModel.direct()`. The indirect lobe was already there; with a light in the scene the coat had no highlight. |
| `clearcoat_normal_scale` on a glTF primitive with no tangents | the derivative-tangent `.y` flip three applies to it as well as to `normalScale`. |
| `backdrop_node`, `backdrop_alpha_node` (new, #169) | `LightsNode.setup()`'s blend into `totalDiffuse` on Basic, Phong, Lambert, Toon, Standard, Physical and a custom `lighting_model`, and `setupLighting()`'s backdrop arm on an unlit material. A backdrop also puts the material in the transparent list. |

Loud:

| Field | On | Why |
|---|---|---|
| `env_map` | anything but Basic | The Basic flow's `BasicEnvironmentNode` is the only reader. Phong and Lambert wrap the same node in their own lighting model, which is not ported; a PBR material takes a PMREM through `pmrem_env` (or `scene.environment`) rather than a raw cube. |
| `pmrem_env` | anything but Standard / Physical | Only `PhysicalLightingModel` reads a PMREM. |
| `ao_map` | anything but Standard / Physical | `setupAmbientOcclusion()` is wired into the PBR flow only; Basic and Phong's indirect term does not multiply by it. |
| `backdrop_node` | Normal | `MeshNormalNodeMaterial`'s flow packs the normal straight into the output, with no lighting step for the blend to sit in. |
| `anisotropy` | Physical, lit by a point, spot or directional light | The anisotropic `BRDF_GGX` (`V_GGX_SmithCorrelated_Anisotropic`, `D_GGX_Anisotropic`) is not ported, so the direct highlight would be the isotropic one. The indirect bent normal is ported, which is why an unlit anisotropic page such as `webgpu_loader_gltf_anisotropy` is quiet. |

Not fields, so nothing to be loud about: three.js properties the struct does
not have at all — the `stencil*` family, `clippingPlanes`,
`sheenColorMap` / `sheenRoughnessMap`, `iridescence*`, `polygonOffset*`,
`dithering`. (`wireframe` and the scalar `alpha_test` are fields and are
honoured; `alpha_test_node` is the node form of the latter.)
Setting one is a compile error, which is louder than a log line. A field
three's own class for that kind lacks — `clearcoat` on a Standard material,
`shininess` on a Physical one — is ignored by three too and is not listed.
`scene.environment` is not applied to Phong or Lambert either, which is
three's behaviour: `NodeMaterial.setupEnvironment()`, which those kinds
inherit, reads only `material.envNode` / `material.envMap`.

## 8. Render-pipeline hooks take the renderer and nothing else

`RenderPipeline::on_before_render` and `on_after_render` take a
`Box<dyn FnMut(&mut Renderer)>`. A hook that has to move a camera captures
the camera itself, through `RenderCamera::set_view_offset` (issue #164). The
hook signature does not pass one in, because three's callbacks take no
arguments either. `docs/nodes.md` §37 has the ordering.

## 9. The node enum opens through `Node::Custom`

`nodes::Node` is a closed enum, and an exhaustive `match` over it in the
builder is what tells a rung it has added something the generator cannot
emit. Issue #155 settled how user code gets node types of its own anyway:
one more variant, `Node::Custom(Rc<dyn CustomNode>)`, rather than trait
objects everywhere or a registry of kinds. A `CustomNode` has a
`type_name`, a `node_type`, an `is_cacheable` flag and a `setup(
&NodeBuilder ) -> NodeRef` that **composes existing variants and never emits WGSL of its
own**; a node that needs a new statement shape becomes a variant in the
crate, where the dump gates see it. `tsl::custom( node )` wraps one.
`tsl::context( node, ContextValue )` and `tsl::isolate( node )` (and the
`.context()` / `.isolate()` methods) are three's `ContextNode` and
`IsolateNode`; `ContextValue` holds string-keyed node values, the addon half
of `BuildContext` (#155 decision 6). `NodeBuilder::context( key )` is how a
`setup` reads them. `docs/nodes.md` §45 has the semantics.

All of this shipped in 0.1.x (#155 decision 3). Strictly, three new variants
on a public enum break a caller that matches on `Node` exhaustively; no
published consumer does, since they build graphs and hand them to the
builder, so the change was taken as additive. 0.2.0 marks `Node`
`#[non_exhaustive]` (decision 11), so that later variants are additive by the
rules and not only in practice. The pre-`context` spellings stay:
`range_fog_factor_with_view_z` and `density_fog_factor_with_view_z` build the
same WGSL as the `.context( { getViewZ } )` form.

## 10. Passes render themselves; their explicit `render()` is removed

`PassNode::render`, `RttNode::render` and `BloomNode::render` were
`#[deprecated(since = "0.1.3")]` and are removed in 0.2.0. The renderer runs
a pass from `updateBefore()`, the first time in a frame a draw samples the
pass's texture, as three does (`docs/nodes.md` §57). A pass holds its scene
and camera through `pass( scene, camera )` or `set_scene` instead of
borrowing them per call. The three methods stayed for one release, forwarding
to that path while they were 0.1.x public API and callers used them: each one
marked the pass done for the frame and rendered it, so an unconverted caller
got the frame it got before, rendered once. Every caller in this tree is
converted; `mark_update_before` and the `NodeFrameState::mark` it used, which
existed only for the forward, went with them.

The pass types became `Rc` handles to make this work, and their setters
take `&self`. Code that held them by value still compiles, because a handle
derefs to its state. `controls_and_camera` hosts take the camera as
`cameras::CameraMut`, because an example that shares its camera with a pass
can lend it only as a `RefMut`.

## 11. Public means three.js public, or documented here

Settled for 0.2.0 by the audit #14 asked for, which walked every reachable
public item in `three-rs`, `sdf-text` and `three-rs-controls` against three.js
(or troika, for `sdf-text`) and against who outside the crate uses it. Each
item landed in one of three places, by one rule each:

- **Public**: a three.js class, its non-underscore methods and properties, an
  exported TSL function or constant, or something a decision in this file
  designs. This is kept even where nothing calls it yet, because the port
  mirrors three.js' public API on purpose and the next rung will want it.
- **`#[doc(hidden)]`**: not API, but the examples, the tests, the viewer, the
  web shell or `dump_wgsl` genuinely reach it. These are three's private
  members that the WGSL and pixel gates inspect (`quad_material`,
  `NodeProgram::vertex_buffers`, `materials::setup` and its contexts), the
  readbacks and cache counters the grader asserts on (`read_target_pixels`,
  `material_cache_len`), and the oracle hooks the decoder tests compare with
  three's own output (`GltfLoader::accessors`, `loaders::meshopt`,
  `loaders::draco`). Like `testing` (decision 4) they carry no compatibility
  promise.
- **`pub(crate)`**: everything else. Setup-context plumbing, per-light uniform
  helpers keyed on the renderer's light index, render-list and render-state
  internals, texture GPU handles, three's `_`-prefixed members, module-local
  helpers three.js does not export. Items that turned out to be unused
  anywhere were deleted rather than hidden.

A `pub mod` whose every item is re-exported from its parent is private now
(`animation::animation_action`, `extras::curve`, `math::interpolant`,
`three_rs::error` and the like), so each type has one path. The long paths
were only ever used by the QUnit ports, which now import the short ones.
Modules that are namespaces in three.js (`extras::shape_utils`,
`math::math_utils`) or that hold items not re-exported stay public.

`#[non_exhaustive]` goes on the enums that mirror an open-ended three.js
concept of which the port has only part: `nodes::Node` (decision 9) and its
`Builtin`, `UniformSource`, `TextureSource` and `BufferSource`; `MaterialKind`,
`Payload`, `LightKind`, `Background`, `ToneMapping`; the texture `TextureType`,
`Mapping`, `ColorSpace` and `DataTextureData`; KTX2's
`EngineFormat`; `TrackInterpolant`, which has no Bézier arm yet; and
`sdf_text::TextAlign`, which has no `justify`. Each one gains variants as rungs
land, and a variant is additive only if callers were made to write the `_`
arm first. Enums whose three.js set is closed and fully ported (`Blending`,
`Side`, `Wrapping`, `ShadowMapType`, `EulerOrder`) stay exhaustive, because a
caller matching them exhaustively is right to. The same reasoning puts the
attribute on the structs that stand for a partly ported options object or
counts block; decision 3 says how those are built.

The Rust API Guidelines checklist (#37, folded into #14) was the rubric, and
only the items that cannot be fixed later without a break were acted on; the
rest are follow-up issues. One more was: `CurveVector` (and so `BezierVector`
and `LineVector`) is sealed (C-SEALED). It is the bound the generic curves put
on their point type, implemented for `Vector2` and `Vector3` as three.js'
curves are, and sealing it is what lets it gain a method later. `RenderCamera`
is sealed the same way, for the same reason: it is the render path's slice of
`Camera`, implemented for `ArrayCamera`, `OrthographicCamera` and
`PerspectiveCamera` only, and the trait gains a method as the render path
needs one rather than growing a new one beside it. The traits
users are meant to implement (`Curve`, `CustomNode`, `LightingModel`,
`NodeUpdate`, `UvGenerator`, `BindingTarget`, `TargetResolver`) stay open. The `get_` prefixes (`get_world_position`,
`get_size`) stay: they are three.js' `getX( target )` methods, which compute
rather than return a field, so C-GETTER does not apply and decision 3's
correspondence does. Handles and ids into a crate-owned table
(`NodeRef`, `RootId`, `ActionHandle`, `GeometryId`, `MaterialId`, ...) are
opaque: their fields are private, so the only way to get one is the
constructor or accessor the owning type hands out, and a caller can never
forge one that names a slot the table never allocated.

## Where each decision came from

Decision 11 is the last of issue #14 (with #37).

Decision 10 is issue #162.


Decision 8 is #155's design note and its decisions 1, 2, 3 and 6, built in
#161.

Decision 6 came out of the compositor consumer and issue #62; the gaps it
closes were found by trying to build the compositor's scene against 0.1.2.

Decisions 1 and 2 came out of the plane-dendro consumer test and a
conversation about which renderers are multi-threaded and how. Decision 3 is
the port's founding discipline restated for the public API. The consumer
gaps that test produced (undocumented windowed path, silent frustum culling
of text, per-frame program rebuild) are filed as their own issues; they are
about missing pieces and performance, not about the shape above.
