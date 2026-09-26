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
- **Lights, cameras and textures** take three.js's positional constructor
  arguments and expose the rest as fields or one-line setters, for the same
  reason.
- **Rust names throughout**, with the three.js name in the doc comment. This
  is already how the crate is written (`set_rotation`,
  `matrix_world_needs_update`, `is_mesh()`); recorded so it is not reopened.
- **`Scene` and the cameras own a `node` field** and are not `Node`s
  themselves. `Scene` adds `background`, `fog_node` and `override_material`;
  a camera adds its projection state. Both forward `add()`, `children()` and
  `update_matrix_world()` to their node, so both can sit inside the tree.

## 4. Surface hygiene

- `three_rs::testing` is `#[doc(hidden)]`. It holds the harness helpers
  (`write_png`, `three_js_dir`) that the examples, the viewer, the e2e tests
  and consumers' own graders use, so it stays public but out of the docs.
  The audit found no other accidental `pub`; the rest is the ported surface.
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

Loud:

| Field | On | Why |
|---|---|---|
| `env_map` | anything but Basic | The Basic flow's `BasicEnvironmentNode` is the only reader. Phong and Lambert wrap the same node in their own lighting model, which is not ported; a PBR material takes a PMREM through `pmrem_env` (or `scene.environment`) rather than a raw cube. |
| `pmrem_env` | anything but Standard / Physical | Only `PhysicalLightingModel` reads a PMREM. |
| `ao_map` | anything but Standard / Physical | `setupAmbientOcclusion()` is wired into the PBR flow only; Basic and Phong's indirect term does not multiply by it. |
| `anisotropy` | Physical, lit by a point, spot or directional light | The anisotropic `BRDF_GGX` (`V_GGX_SmithCorrelated_Anisotropic`, `D_GGX_Anisotropic`) is not ported, so the direct highlight would be the isotropic one. The indirect bent normal is ported, which is why an unlit anisotropic page such as `webgpu_loader_gltf_anisotropy` is quiet. |

Not fields, so nothing to be loud about: three.js properties the struct does
not have at all — `wireframe`, the `stencil*` family, `clippingPlanes`,
`sheenColorMap` / `sheenRoughnessMap`, `iridescence*`, `alphaTest` (the
scalar; `alpha_test_node` is the port's form), `polygonOffset*`, `dithering`.
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
arguments either. `docs/nodes.md` §36 has the ordering.

## Where each decision came from

Decision 6 came out of the compositor consumer and issue #62; the gaps it
closes were found by trying to build the compositor's scene against 0.1.2.

Decisions 1 and 2 came out of the plane-dendro consumer test and a
conversation about which renderers are multi-threaded and how. Decision 3 is
the port's founding discipline restated for the public API. The consumer
gaps that test produced (undocumented windowed path, silent frustum culling
of text, per-frame program rebuild) are filed as their own issues; they are
about missing pieces and performance, not about the shape above.
