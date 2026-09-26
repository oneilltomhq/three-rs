# `webgpu_deferred`

2026-09-25: regraded against three.js 5f610f5 (the cube PMREM of 2f80402, #146): 0 of 100000 pixels (was 54), with the background now through `CubeMapNode` as the page has it.

Status: **green.** 54 of 100000 pixels against three.js r186's own
`test/e2e/image.js`, threshold 0.1%. Intel Iris Xe, Mesa 25.3.6, wgpu 30.0.1 on
Vulkan.

The page is a small deferred renderer built out of `PassNode`s: a teapot into a
three-attachment G-buffer with the scene's lighting switched off, a full-screen
resolve quad that runs three's ordinary physical lighting flow against the
G-buffer through `overrideNodes()`, a transparent pass that shares the opaque
pass's depth attachment, and a composite.

## Grade first

Three's own frame for this page, captured through `tools/dump-webgpu.mjs` (which
pins the harness exactly as the grader does) and compared against
`examples/screenshots/webgpu_deferred.jpg` with three's unmodified
`Image.compare( …, 0.1 )`:

| | pixels of 100000 |
| --- | --- |
| three.js r186 against its own reference JPEG | 0 (0.000%) |
| this port against the same JPEG | 54 (0.054%) |

Unlike `webgpu_loader_gltf` (59) or `webgpu_custom_fog_background` (57), the
reference for this page is exact on this GPU, so all 54 pixels are the port's.
They are single-pixel runs along the **edges of the transparent planes** — the
lower-left plane's rim (x 0–28, y ≈ 160) and four short runs on the upper plane
edges — with nothing in the teapot's interior, nothing in the resolved
background, and nothing on the light spheres. That is a half-covered pixel
blended from a slightly different plane-edge coverage, which is what the
interleaved back/front `DoubleSide` split makes sensitive; the largest channel
delta is 82 of 255 on a plane edge and the mean over the 54 is 41.

## Reconciling with the plans

No scout plan exists for this page. It was ported from
`~/src/vendor/three.js/examples/webgpu_deferred.html` directly, against three's
WGSL dumped into `target/dumps/webgpu_deferred/` (uncommitted, per the rules):
`m11`/`m12` are the G-buffer pair, `m13`/`m14` the resolve quad, `m15`–`m17`
the transparent planes (one vertex, one fragment per side) and `m18`/`m19` the
composite.

Three pieces of the page were already on the ladder and needed nothing:

* **UltraHDR as background *and* `scene.environment`** — §21 and §23.1
  (`webgpu_pmrem_equirectangular`, `webgpu_loader_gltf`).
* **MRT as a pass property**, including the material-dependent
  `MrtValue::Deferred` shim — §23.2 (`webgpu_mrt`).
* **`PassNode::depth_texture()`** — §24.1
  (`webgpu_custom_fog_background`); this rung passes the texture *into* a
  second pass rather than sampling it.

## What was added

| area | what |
| --- | --- |
| `src/renderer/mod.rs` | `Renderer::opaque` / `transparent` / `lighting_enabled` / `camera_layers`; the background gated on `opaque`; the transparent `DoubleSide` two-draw split with `BACK_SIDE` / `FRONT_SIDE` material-key variants; the `depthInitialized` first-frame depth clear |
| `src/renderer/pass.rs` | `PassOptions::depth_texture` / `auto_clear_depth`; `PassNode::set_layers` / `set_opaque` / `set_transparent` / `set_lighting_enabled`; the borrowed-depth resize path |
| `src/renderer/render_target.rs` | `depth_initialized()` / `set_depth_initialized()` / `depth_buffer()`; `set_size_keeping_depth()` |
| `src/materials/mod.rs` | `MeshBasicNodeMaterial::metalness_node` / `roughness_node` / `depth_node` / `context_overrides` |
| `src/materials/node_material.rs` | `SetupContext::lighting_disabled` and the `setupLighting()` gate it feeds; `SetupContext::geometry_missing_normal`; the material's metalness/roughness overrides; `MaterialFlow::depth` |
| `src/materials/physical.rs` | `getGeometryRoughness()` is `float( 0 )` for a geometry with no normal attribute |
| `src/nodes/tsl.rs` | `OverrideNodes` and `with_override_nodes()` — `overrideNodes( [ [ positionView, … ], … ] )` |
| `src/nodes/builder.rs` | `MaterialFlow::depth` generated first, and the `@builtin( frag_depth )` output struct |
| `examples/webgpu_deferred.rs` | the port |
| `examples/dump_wgsl.rs` | `deferred_gbuffer` and `deferred_resolve` sections |
| `tests/e2e/main.rs` | the graded test (with the shared-depth assertion) and the `rung!()` line |

`docs/nodes.md` §27 is the reference for all of it.

## What the pixels found

Four things, in the order the grader gave them up.

**4914 pixels — five of the six planes were missing.** `pivot.rotation.y =
angle` sets the Euler without touching the quaternion; three syncs the two
through `Euler.onChange`. The port needs `set_rotation( 0, angle, 0 )`. Every
plane was drawn — all six were simply in the same place.

**557 pixels — one plane was occluded by the teapot's spout and should not have
been.** This is the `renderTargetData.depthInitialized` quirk in §27.4: three
clears the depth once, on the first render into a target that has one, when
`autoClearDepth` is false. The flag is per *render target*, so the transparent
pass wipes the shared depth on frame one although the texture is the opaque
pass's. Reproduced on purpose; the graded frame is frame one.

**167 pixels — the light spheres and the planes were slightly the wrong hue.**
`Color.setHSL( h, s, l )` defaults to `ColorManagement.workingColorSpace`,
which is linear-sRGB. The eight light colours are built that way.

**41 programs rebuilt every frame**, caught by `steady_frame_builds_nothing`
rather than by a diff. The `DoubleSide` split clones the material to set
`side`, and `MaterialId::clone()` mints a *fresh* id — that is how two
materials built from one stay two materials — so the program cache key changed
every frame. The key is now taken from the original material and carries the
side as a variant.

What the *dump diff* found, before any pixels: the G-buffer fragment was
emitting a full (all-zero) indirect-lighting chain that three does not emit at
all, because `lighting: false` drops the environment along with the lights.
§27.1 has the gate. It cost no pixels — an MRT material never reads `Output` —
but it bound the PMREM textures and the BRDF LUT to a program that could not
use them.

## What was ruled out

* **`RenderList.transparentDoublePass`.** Three has a second `DoubleSide`
  path that batches every back side before every front side. It is gated on
  `material.transmission > 0`, which no material here has, and its draw order
  is visibly different with six overlapping planes. The per-object interleaved
  split in `_renderObjectDirect()` is the one this page takes.
* **Giving the transparent pass its own depth texture and copying.** The page's
  whole point is one attachment written by two passes; a copy would need a
  blit three does not do and would still need the `depthInitialized` clear to
  land in the same place.
* **Clearing depth per texture rather than per target.** Tidier, and wrong: the
  second pass's clear is exactly what the reference frame shows.
* **A general `ContextNode` for `overrideNodes()`.** Same reasoning as §24.3 —
  this port's TSL is eager. The three overrides the page uses are named fields
  on the material; an arbitrary `[ node, node ]` list would need node identity
  in the memo keys for nothing the ladder asks for.
* **Lighting as an object on the renderer.** Three's `renderer.lighting` owns
  the lights node; here it is a bool on the renderer and on the pass, because
  the port builds the lights per draw from the render list.

## What was left out

* **`OrbitControls`.** `camera.lookAt( 0, 0, -0.2 )`, as on every earlier rung;
  no pointer events reach the harness.
* **`onWindowResize`.** The harness never resizes. The borrowed-depth resize
  path exists and is exercised by the pass's first `render()` only.
* **A custom depth output name.** `PassNode::view_z_node` and the depth texture
  are still `'depth'`-only (§24, "What was left out").
* **`MrtNode::set_deferred` for anything but a material-dependent `vec4`.** The
  two deferred members here are `vec4( positionView, metalness )` and
  `vec4( normalView, roughness )`; §23.2 has the shim's shape.

## Ladder after the change

Unchanged, all 43 e2e tests green: depth_texture 0, instance_mesh 60,
materials_basic 0, rtt 1, lights_phong 31, morphtargets 0, shadowmap 7,
lights_physical 4, postprocessing_masking 18, tsl_galaxy 40, loader_gltf 59,
mrt 87, custom_fog_background 57, and this page 54.
