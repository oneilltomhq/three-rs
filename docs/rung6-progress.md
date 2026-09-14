# Rung 6 — `webgpu_morphtargets`

`examples/webgpu_morphtargets.html`: a 32×32×32-segment box with two morph
targets (a sphere and a twist) under a `MeshPhongNodeMaterial` with
`flatShading: true`, an `AmbientLight` and a `PointLight` parented to the
camera.

Result: **0 of 100000 pixels different**.

## What the graded frame actually needs

`initGUI()` only registers Inspector sliders, and no `onChange` fires under the
harness' single deterministic RAF, so **both influences are 0 in the graded
frame** — the box is still a box, and the `If( influence.notEqual( 0 ) )` body
never executes. What the frame does exercise is everything around the loop: the
extra bind-group entries, the vertex-only visibility, `base = 1 - Σ influences`
(1 here) and the flat-shaded Phong lighting.

The morph path itself is exercised off the graded path instead: the
`MORPH_INFLUENCES` environment hook on the example drives the sliders
(`MORPH_INFLUENCES=1,0` renders the sphere, `0,1` the twist, `0.5,0.5` the
blend — all three checked by eye), and `tests/nodes_morph.rs` pins `getEntry()`'s
packing.

## What was built

- **`PerspectiveCamera` owns a scene-graph `Node`** (commit `14860a5`). The page
  does `scene.add( camera )` and parents the point light to the camera, which
  the old bare-`Object3D` camera could not express. `docs/scene-graph.md`
  predicted this; `OrthographicCamera` is untouched.
- **`AmbientLight`, `sortLights()` and lazy lighting-context vars** (`8c48acf`).
  `LightsNode.setupLightsNode()` sorts by `light.id`, so the ambient light —
  created first but reached second by the tree walk, since the point light hangs
  off the camera — emits first, as the dump shows. `AmbientLightNode` is
  `context.irradiance.addAssign( colorNode )`; the accumulators are lazy
  `toVar()`s, so their declarations interleave with their first use exactly as
  Three's do. Both changes are textually neutral for rungs 3 and 5 apart from
  `directSpecular` moving to its first use (which is what Three does) and
  pixel-neutral for every existing rung.
- **flatShading** (`7af16f5`). `normalViewGeometry` becomes `normalFlat` —
  `normalize( cross( dpdx( positionView ), - dpdy( positionView ) ) )` — so the
  vertex stage drops the `normal` attribute, the normal matrix uniform and the
  `v_normalViewGeometry` varying.
- **Morph targets** (`3f24661`). `getEntry()` packs `morphAttributes.position`
  into one `rgba32float` `DataArrayTexture` (here 4096 × 2, 2 layers, from 6534
  positions); `morphReference()` emits `positionLocal *= base` and the
  `Loop( 2 )`. New node plumbing: `Loop` / `If` statements, an `i32` loop index,
  `ivec2`, `textureLoad` on a `texture_2d_array` with a layer, the influences
  `uniformArray` and the `base` uniform.

## Bugs the dumps caught

- **`normalView`'s cache key ignored the flat-shading flag.** The port keys
  `normalView` on (sub-build layer, material normal node) to stand in for
  Three's per-build `nodeData`; with `flatShading` a third input appeared, and
  without it in the key the morph material reused the *previous* material's
  interpolated normal — visible in the dump as a stray `v_normalViewGeometry`
  varying and a normal-matrix uniform in a shader that should have neither.
  Only the multi-material `dump_wgsl` run shows this; a single-material process
  (the example, the e2e test) cannot.

Nothing was found only by the grader: the image was correct the first time the
tree built, because the graded frame's morph influences are 0.

## Ruled out along the way

- **`rgba32float` filtering.** The first bind-group layout declared the morph
  texture `filterable: true`, which wgpu rejects without the
  `FLOAT32_FILTERABLE` feature. The shader only ever `textureLoad`s it, and
  Three's own layout has no sampler for it at all, so the entry is
  `filterable: false` with `viewDimension: 2d-array`.
- **Traversal order as light order.** With the camera in the tree, traversal
  order is no longer creation order; matching the dump needed `sortLights()`,
  not a re-ordering of the walk.

## Notes for the next rungs

- `SetupContext` now carries `morph: Option<MorphEntry>` and `lights:
  Vec<LightKind>`; `Renderable` carries `morph_influences` / `morph_base`.
- `Mesh::morph_target_influences` exists (`updateMorphTargets()` sizes it from
  `geometry.morphAttributes`), but nothing animates it yet — an example that
  morphs per frame needs the influences re-uploaded per draw, which the existing
  per-item `UniformContext` already supports.
- `morphAttributes.normal` / `.color` are not ported: `getEntry()` hard-codes
  `vertexDataCount = 1` and `morphReference()` skips the `normalLocal` half.
- The lighting-context accumulators are lazy now, so a new lighting model that
  reads one before assigning it gets the `toVar()` at its first use, which is
  what Three does.
