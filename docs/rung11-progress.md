# Rung 11 — `webgpu_mesh_batch`

**Result: 0 of 100000 pixels different, limit 0.1%.** The ladder is unmoved:
depth_texture 0 / instance_mesh 60 / materials_basic 0 / rtt 1 / lights_phong
31 / morphtargets 0 / shadowmap 7 / lights_physical 4 /
postprocessing_masking 18 / tsl_galaxy 40.

Under `WebGPURenderer` this page uses **no multi-draw, no indirect draws and no
storage buffers**. Three issues 453 ordinary `drawIndexed()` calls in one pass,
against one pipeline and one bind group, varying only `(indexCount, firstIndex,
firstInstance)`. Everything per-instance lives in three `DataTexture`s read
with `textureLoad`. `renderer.info()` reports `calls 454` — the 453 sub-draws
plus the output-colour-transform quad.

## The scout plan versus the tree (step 1)

The plan (2026-09-13) predates 0.2.0. The deltas:

| The plan says | The tree has |
|---|---|
| `Object3D` handles, `Mesh::new( geometry )` | `Node` newtype over `Rc<RefCell<Object3D>>`; `Mesh::new( geometry, material )`. `BatchedMesh::new()` returns a `Node` whose `Payload` is the batch, mirroring three.js' constructor. |
| "add an instanced-attribute upload path" | PR #90's instanced attribute cache already uploads one attribute per array. This rung needs none of it: `BatchedMesh` has no instanced attributes, only data textures. |
| "count the 453 draws by hand" | `renderer.info()` exists; `info.record_draw` is called per sub-draw, so the 453 land in `RenderCounts::calls`. |
| `Renderer::skip_random_draws` for the Inspector's five `Math.random()` draws | Kept in the example instead: this rung has no `range()` fills, so the example owns a `DeterministicRandom` and `skip( INSPECTOR_RANDOM_DRAWS )`. `skip_random_draws` stays a rung-13 (`range()`) mechanism. |
| Per-instance visibility as a third texture | r186 has no visibility texture; visibility is a CPU flag that decides whether an instance reaches the draw list. |

## What was added

| Area | What | Where |
|---|---|---|
| Textures | `DataTexture` — CPU-owned `f32`/`u32` image, versioned, uploaded once and re-uploaded on `set_needs_update()` | `src/textures/data_texture.rs` |
| Objects | `BatchedMesh`: geometry ranges in one `BufferGeometry`, `add_geometry` / `set_geometry_at` / `add_instance` / `set_matrix_at` / `set_color_at`, the three textures, the per-instance frustum cull and the sorted draw list | `src/objects/batched_mesh.rs` |
| Objects | `Payload::BatchedMesh`, `SubDraw`, `BatchCamera`, `SortContext`, `MultiDrawItem`, `CustomSort` | `src/objects/{payload,batched_mesh}.rs` |
| Utils | `radix_sort()` — the addons' hybrid radix sort (`examples/jsm/utils/SortUtils.js`) and `to_uint32()` (JS `>>> 0`) | `src/utils/sort_utils.rs` |
| Nodes | `batch()` / `batch_color()` / `batch_indirect_index()` — `src/nodes/accessors/Batch.js` | `src/nodes/batch.rs` |
| Nodes | `SampleMode::LoadTexel`, `TextureKind::{FloatData2D, Uint2D}`, `Node::TextureSize`, `Node::VaryingProperty`, `tsl::{modulo, ivec2, texture_size, texture_load_texel, varying_property}` | `src/nodes/{node,builder,wgsl,tsl}.rs` |
| Materials | `material.output_node` and `MaterialFlow::output_node` — `NodeMaterial.setup()`'s custom-output branch | `src/materials/{mod,node_material}.rs` |
| Renderer | `Renderable::sub_draws`, the `on_before_render()` pass, data-texture upload and binding, per-sub-draw `record_draw` | `src/renderer/{mod,programs}.rs` |
| Testing | `Strip::assert_steady_uploading()` — a steady frame that legitimately uploads N textures | `src/testing/strip.rs` |

## What the pixels found

Nothing: the example was green on the first render. The work that would
otherwise have shown up as wrong pixels was found by comparing the port's state
against the scout's `dump.json` before any GPU run. Three state gates now live
in `tests/objects_batched_mesh.rs`, and all three were bit-exact against the
dump's `writeTexture` payloads once the bug below was fixed:

* the matrices texture (48×48 `rgba32float`, 9216 floats) — identical;
* the colours texture (23×23 `rgba32float`) — identical, including the alpha of
  1 that `Color.toArray()` never writes;
* the indirect texture — all 453 entries in the same order.

### The one bug: three.js sorts the first frame from the wrong origin

`BatchedMesh.onBeforeRender()` computes each range's depth as

```js
const z = _temp.subVectors( _sphere.center, _vector ).dot( _forward );
```

where `_vector` is meant to be the camera position in the batch's local frame.
But `_vector` is a module-level scratch that `getBoundingBoxAt()` and
`getBoundingSphereAt()` also use, and those compute lazily — so the *first*
call in the loop overwrites `_vector` with the last vertex of the geometry
whose bounds it just computed, and every depth from then on is measured from
that vertex instead of from the camera. On this page that shifts every depth by
about `-30` (the camera distance), which changes nothing about the *set* of
visible instances but rotates the sorted order: ranges that were 0–30 units
deep go negative, wrap through `>>> 0` to the top of the `uint32` range and
sort last. Instances 0, 1 and 2 each get a slightly different `_vector`, since
each is the first to touch its geometry, which is why instance 1 sits one place
out of order in Three's own list.

The graded frame is the first frame, so the port reproduces this exactly:
`BatchedMesh` carries a `shared_vector` field that `bounding_box_at()` /
`bounding_sphere_at()` overwrite the same way, and the sort reads it. From the
second frame on the bounds are cached, `shared_vector` keeps the camera
position and the depths are the ones the code intends — in the port as in
three.js. This is deliberate quirk reproduction; it is commented at both
sites.

## What was ruled out

* **Multi-draw / indirect draws.** `chromium-experimental-multi-draw-indirect`
  is in the adapter's feature list in the dump, but the pass contains 453 plain
  `drawIndexed()` commands. Nothing in the backend needed to change beyond a
  list of `SubDraw`s on the `Renderable`.
* **Storage buffers.** None in the dump; the bind group is one uniform buffer
  and three sampled textures.
* **A second pipeline.** The output colour transform pass was already ported.
* **`Renderer::skip_random_draws`.** See the delta table.
* **A new comparator or threshold.** Three's own, unmodified.

## What was left out

* `BatchedMesh`'s optimisation/compaction API (`optimize()`, `deleteGeometry()`,
  `deleteInstance()`, `setGeometryIdAt()`), its `BufferGeometry` growth path
  and `copy()` / `toJSON()`.
* `onBeforeShadow()` — a batch that casts a shadow re-sorts against the shadow
  camera. No rung has a shadow-casting batch.
* The wireframe multiplier in `onBeforeRender()` (`multiDrawMultiplier`), and
  `ArrayCamera` frustums.
* The default sorts (`sortOpaque` / `sortTransparent`) are ported but untested
  by the ladder: this example always installs its own `sortFunction`.
* `reversedDepth`, which r186 passes to `setFromProjectionMatrix()` and which
  the port's `Frustum` does not model (always `false` here).
