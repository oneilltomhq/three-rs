# `webgpu_instance_uniform` — one uniform, twelve objects

**Result: 13 of 100000 pixels different (0.013%), limit 0.1%.** Passed on the
first render, with no iteration against the reference image.

Twelve `TeapotGeometry( 50, 18 )` meshes over a `GridHelper( 1000, 40 )`,
sharing a node graph whose only per-mesh input is a `vec3` uniform resolved
from the object about to be drawn. The teapots sample a cube map along the
reflect vector and add the uniform into both the diffuse and the emissive
term.

## Deltas from the brief and the scout note

The `webgpu_materials` scout listed this page as "`GridHelper` +
`TeapotGeometry` + `MeshStandardNodeMaterial` (rung 8) + per-instance
uniforms". Read against the source, two of those are wrong:

* The material is a **`MeshBasicNodeMaterial`**, not a Standard one. There is
  no PBR, no light and no tone mapping on the page.
* There is a `CubeTextureLoader` load of `textures/cube/SwedishRoyalCastle/`
  and a `cubeTexture()` node, which the note does not mention. The loader and
  the cube-map mipmap chain landed with the env-map work already on main, so
  this cost nothing.

`GridHelper`, `TeapotGeometry`, `LineSegments` and `LineBasicNodeMaterial` all
exist since the `webgpu_materials` rung; the brief's "may or may not exist" is
resolved — they do, and the grid's two modules here are byte-identical to that
rung's `m13`/`m14`.

Re-graded in three's own harness before porting: `Diff 0.0% in file:
webgpu_instance_uniform (3.1s)`, stock threshold, `--webgpu`, under the GPU
lock.

## What was added

| Area | What | Why |
|---|---|---|
| `src/nodes/node.rs` | `UniformSource::ObjectUpdate`, `ObjectUpdate`, `ObjectUpdateFn` | `Node.updateType = NodeUpdateType.OBJECT` over a `uniform()` |
| `src/nodes/tsl.rs` | `uniform_object( ty, \|object\| … )` | the public spelling of it |
| `src/renderer/programs.rs` | `UniformContext::object`, the `ObjectUpdate` arm of `bytes()` | `frame.object`, read where three reads it — as the object's buffer is written |
| `src/renderer/mod.rs` | `Renderable::object` | `renderItem.object`, carried to the draw; `None` for the background, quad, PMREM and output passes |
| `src/materials/mod.rs` | `emissive_node` | `NodeMaterial.emissiveNode` |
| `src/materials/node_material.rs` | the EMISSIVE tail of `setupLighting()` on the **unlit** branch | `MeshBasicMaterial` has no `emissive` colour, so the node is the only way in |

`examples/webgpu_instance_uniform.rs`, its `[[example]]` entry, its e2e test,
its row in `steady_frame_builds_nothing`, an `instance_uniform_teapot` section
in `examples/dump_wgsl.rs` and `tests/nodes_object_uniform.rs`.

Nothing was needed for the cube map: `cubeTexture( map )` with no uv is
`materialEnvRotation.mul( vec4( reflectVector, 1 ) )` through the existing
`tsl::cube_texture()`, which is what `MeshBasicNodeMaterial.envMap` already
builds. The example composes it rather than adding a third spelling.

## What the pixels found

Nothing. The WGSL matched three's `m01` / `m02` statement for statement before
the example was run, and the first frame graded 13 px. The residue is the
cube-map JPEG decode and the MSAA resolve, the same two sources every textured
rung carries.

## What the WGSL found

One thing, and it was about counting rather than about pixels: the port builds
the teapot program **twelve times** where three builds it once. Three's page
hands one `Material` object to all twelve meshes; a `MeshBasicNodeMaterial` is
a value here and `Material.clone()` takes a fresh `MaterialId`, exactly as
`Material.clone()` does in three, so each mesh owns its own material and each
gets its own `NodeBuilder::build()`. All twelve generate the same WGSL, so the
program cache holds three entries and the pipeline cache three, and the GPU
sees what three's does. `tests/e2e/main.rs` pins `( 14 built, 3 pipelines, 3
resident )` so a change to material identity surfaces there rather than as an
unexplained frame time. `docs/nodes.md` §19.

## What was ruled out

* **A `color` field on `Object3D`.** The page hangs an ad-hoc `.color` on each
  `Mesh`; `Object3D` is a struct, and inventing a property for one example is
  API invented for a page rather than ported from three. `uniform_object`
  hands the callback the `Object3D` instead and the example answers from a
  `HashMap` keyed on `Object3D.id` — the same one uniform, the same twelve
  values, storage moved from the object to the closure.
* **A second `cube_texture` spelling.** `cubeTexture( tex )` with the default
  uv is three lines of composition over what `envMap` already uses; a
  `cube_texture_env()` wrapper would have been a second name for one graph.
* **Caching the per-object value.** `ObjectUpdate` compares and hashes by
  pointer identity, like `SettableValue`: a value that moves between draws must
  never reach a program or pipeline cache key.
  `tests/nodes_object_uniform.rs::two_per_object_uniforms_are_two_uniforms`
  pins it.
* **`OrbitControls`.** The page constructs one with `minDistance 400` /
  `maxDistance 2000`; its constructor's `update()` re-derives the camera
  position from the spherical offset to the origin and points it there. The
  camera sits at 1216.55 from the target, inside the clamp, and no pointer
  event ever fires, so all that survives into the graded frame is
  `camera.lookAt( 0, 0, 0 )`.
* **The inspector's `Math.random()` draws.** Unlike `webgpu_tsl_galaxy`, this
  page never calls `createParameters()`, so the inspector draws none and the
  harness' seeded sequence is four draws per mesh — the colour, then the three
  Euler angles — and nothing else.

## WGSL

`cargo run --release --example dump_wgsl` emits an `instance_uniform_teapot`
section. It matches `dump-instance_uniform/m01` and `m02` statement for
statement; the differences are the ones `docs/nodes.md` §8 already lists
(generated uniform and var numbering, render-struct member order, the absent
`VERTEX_` sub-build temps). The grid is `materials_grid`, already diffed on the
`webgpu_materials` rung.

## Gates

* `cargo fmt --check` — clean.
* `cargo clippy --release --all-targets -- -D warnings` — clean.
* `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps` — clean.
* `cargo test --release --lib` — 60 passed.
* e2e, release, serial, under the GPU lock — 33 tests, all green. The full
  ladder, unchanged: compute_points 4, depth_texture 0, furnace_test 0,
  instance_mesh 60, **instance_uniform 13**, lights_phong 31, lights_physical
  4, lines_fat 0, materials 44, materials_basic 0, mesh_batch 0, morphtargets
  0, pmrem_cubemap 0, pmrem_scene 0, pmrem_test 27, postprocessing_anamorphic
  2, postprocessing_bloom 0, postprocessing_bloom_selective 1,
  postprocessing_difference 13, postprocessing_direct 21,
  postprocessing_masking 18, postprocessing_radial_blur 7, rtt 1, shadowmap 7,
  skinning 6, ssaa 0, tsl_galaxy 40 — all of 100000, limit 100. Steady frame
  6.03 ms, ceiling 100 ms, and `programs` / `pipelines` / `buffers` all zero on
  frames two and three.
