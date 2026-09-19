# webgpu_postprocessing_bloom

Branch `rung-gltf-bloom`, cut from `37aaca1` (the tip of
`rung-bloom-selective`, i.e. `origin/main` plus `BloomNode`).

**Green at 0 pixels of 100000**, steady frame 8.6 ms, 19 draw calls, 52085
triangles. The 21-row ladder is otherwise unchanged.

    depth_texture 0 / instance_mesh 60 / materials_basic 0 / rtt 1 /
    lights_phong 31 / morphtargets 0 / shadowmap 7 / lights_physical 4 /
    postprocessing_masking 18 / tsl_galaxy 40 / skinning 6 / mesh_batch 0 /
    compute_points 4 / radial_blur 7 / materials 44 / ssaa 0 /
    pmrem_cubemap 0 / bloom_selective 1 / lines_fat 0 / pmrem_test 27 /
    **bloom 0**

## What this rung actually is

The scout plan
(`scouts/scouts/webgpu_postprocessing_bloom/PLAN.md`) predates `BloomNode`,
which landed with `webgpu_postprocessing_bloom_selective` sitting 2. With the
bloom chain, `uniformArray`, const `array()` and the renderer save/restore all
in the tree, what was left of the plan is its **`GLTFLoader` half** — and that
is where the work went. The page is one glTF file and nothing else.

`PrimaryIonDrive.glb` is 20 nodes, 6 primitives, 3 materials, a `VEC4`
`COLOR_0` on every primitive, one `alphaMode: 'BLEND'` material, emissive
factors, `UNSIGNED_INT` indices and a node-TRS clip that `optimize()` shortens.
Of that, the loader already handled the container, the accessors, the node tree
and the skinned path (`webgpu_skinning`). It handled none of the non-skinned
mesh path.

## The oracle, before any GPU work

`tests/fixtures/webgpu_postprocessing_bloom/oracle.mjs` runs **three's own
`GLTFLoader`** in node and dumps what it parsed:
`tests/fixtures/webgpu_postprocessing_bloom/primaryiondrive.json`, 33 KB
because the bulk arrays are FNV-1a-32 hashes plus a `first` probe rather than
30 MB of floats. `tests/gltf_primary_ion_drive.rs` is seven tests against it:

| test | what it pins |
|---|---|
| `node_tree_matches_three` | 20 nodes, names, types, parents, the "node becomes its single mesh" rule |
| `transforms_match_three` | every node's TRS |
| `geometry_matches_three` | attribute item sizes, counts and hashes; the index array's *declared width* |
| `materials_match_three` | colour, metalness, roughness, emissive, opacity, side, transparent, depthWrite |
| `the_material_variant_flags_follow_the_geometry` | `assignFinalMaterial`'s clone: `(normalScale.y, vertexColors, flatShading)` per mesh |
| `animation_matches_three_before_and_after_optimize` | track names, interpolations, keyframe counts and value hashes, across `optimize()` |
| `world_matrices_at_t0_match_three` | every `matrixWorld` after `mixer.update( 0 )` |

Two of them — `transforms_match_three` and
`animation_matches_three_before_and_after_optimize` — were green against the
*unchanged* loader while the other five were red. That is what says the oracle
is measuring the loader rather than agreeing with it.

## What was added

| area | what |
|---|---|
| `src/loaders/gltf_loader.rs` | `createNodeMesh` for non-skinned primitives; `assignFinalMaterial`'s variant clone and cache; `alphaMode: 'BLEND'` → `transparent` + `depthWrite: false`; `emissiveFactor`; the index accessor keeps its declared width; an unnamed node gets the empty name rather than a generated one |
| `src/objects/mesh.rs` | `Mesh::of`, the payload alone, mirroring `SkinnedMesh::of` |
| `src/nodes/tsl.rs` | `vertex_color( item_size )` — `vertex_color_rgba` beside `vertex_color_rgb` |
| `src/materials/node_material.rs` | `SetupContext.vertex_color_size` |
| `src/renderer/mod.rs` | `vertex_color_size` from the geometry; `fullscreen_pass` and its `currentSamples` branch |
| `examples/webgpu_postprocessing_bloom.rs` | the example |
| `examples/dump_wgsl.rs` | `bloom_scene_constant1`, `bloom_scene_HoloFillDark`, `bloom_high_pass_single`, `bloom_render_pipeline_quad_reinhard` |
| `tests/e2e/main.rs` | the graded row and the ladder row |
| `docs/nodes.md` | §16 |
| `docs/postprocessing.md` | "MSAA and the output quad" |

## What the pixels found

**The image was black, and the loader was not why.** With the loader gated
green and the scene demonstrably drawing — `renderer.info()` said 19 calls and
52085 triangles, and a direct `renderer.render( scene, camera )` produced the
model — the `PassNode`'s target read back black. The difference from every
green postprocessing rung was one line of the page: `new WebGPURenderer( {
antialias: true } )`.

`Renderer.currentSamples` is 0 when `_currentRenderContext.fullscreenPass` is
set, which `_renderScene()` takes from `scene.isQuadMesh === true`. The port
only had the `needsFrameBufferTarget` branch. Every earlier rung's last draw
goes through the internal framebuffer target, so it landed on 0 that way and
the canvas MSAA path had never once been exercised; a `RenderPipeline` under
`antialias: true` is the first thing to reach it, and on this adapter a 4x
canvas attachment resolves to black. `docs/postprocessing.md` has the whole of
it. This is the scout's plan item 7 ("the pass store/resolve config"), which
this branch had written off as pixel-neutral — it is the entire image.

**`COLOR_0` is `VEC4` here.** `vertexColor()` widened a `vec3`
unconditionally. Three's `m00` takes `color : vec4<f32>` at `@location( 0 )`,
and after the fix so does the port's, attribute for attribute.

**`useDerivativeTangents` flips `normalScale.y` unconditionally.** The port had
gated it on `normal_map.is_some()`; three only asks whether the material has a
`normalScale`. `circle2_constant2_0` is the primitive with no `TANGENT`, and
its `(1, -1)` is what `the_material_variant_flags_follow_the_geometry` pins.

**`MaterialId::clone()` returns a fresh id on purpose**, so an
`assignFinalMaterial` cache cannot be gated on shared identity. It does not
need to be: the program cache is keyed on a hash of the generated WGSL
(`PipelineKey { program, state }`), not on material identity, so two clones of
one variant share a pipeline regardless. The test asserts the *flags* instead.

## WGSL

Against `scouts/scouts/webgpu_postprocessing_bloom/dump/`. No pre-existing
`dump_wgsl` section moved — checked by diffing the whole dump section by
section across the `vertex_color_size` change.

| dump | port section | result |
|---|---|---|
| `m00`, `m01` | `bloom_scene_constant1` | matches within §8's existing classes; attribute locations identical |
| `m02` | `bloom_scene_HoloFillDark` | the transparent, `FrontSide` path, same classes |
| `m04` | `bloom_high_pass_single` | identical but for one blank line |
| `m13` | `bloom_render_pipeline_quad_reinhard` | identical but for declaration order |

`m03`, `m05`–`m12` are byte-identical to the selective rung's sections and are
not dumped twice.

## Decisions a reviewer should look at

1. **`fullscreen_pass` is set around `render_output()`'s quad as well**, where
   it is provably redundant today (`needsFrameBufferTarget` is what routed the
   frame there). The alternative was to set it only where it changes something.
   It is three's own predicate, and leaving the second call site to coincidence
   is how the first one got missed.
2. **`vertex_color_size` in `SetupContext` rather than a material flag.** It is
   a property of the *geometry*, and `SetupContext` is the program cache key,
   so the two shapes cannot collide. The cost is a field on a struct that every
   call site has to fill; the shadow passes pass 0 with a comment saying why.
3. **`alphaMode: 'MASK'` is deliberately not ported**, with a comment in
   `build_material()`. The crate has `alpha_test_node`, but its WGSL is a
   literal where three's is the `materialAlphaTest` uniform, so porting `MASK`
   would put a divergence in the tree for a file that does not use it.
4. **The index accessor keeps its declared width.** Michelle's indices are
   `UNSIGNED_SHORT`, so `webgpu_skinning` cannot move — confirmed by regrading
   it after the loader commit (still 6 of 100000). `PrimaryIonDrive.glb`'s are
   `UNSIGNED_INT`, and truncating them to `u16` silently folds the geometry.

## Left out

* **`Material.name`.** A `&'static str` in the port, so a loaded material
  carries none. Nothing generated reads it; the dump sections select by mesh
  name. Changing the type is a wider change than this rung earns.
* **`alphaMode: 'MASK'`**, above.
* **`KHR_materials_*` extensions.** None are in this file.
* **`OrbitControls` updates.** The page constructs the controls and never
  updates them, so the camera pose is static and is spelled out in `init()`.
