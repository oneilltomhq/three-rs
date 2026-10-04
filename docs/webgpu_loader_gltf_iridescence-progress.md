# `webgpu_loader_gltf_iridescence`

Status: **graded, 0 of 100000 pixels** against three's 0.1% limit. Steady
frame 5.2 ms, 5 draw calls, 20851 triangles, and nothing is built after the
first frame. Measured on Intel Iris Xe, Mesa, wgpu on Vulkan.

The page shows the Khronos Iridescence Lamp (`IridescenceLamp.glb`) under
`venice_sunset_1k.hdr`. The hdr is used twice: as `scene.environment`
through a PMREM, and unblurred as `scene.background` through a cube converted
from the equirect. The lamp has three glTF materials and no lights:

| material | kind | iridescence | IOR | thickness range |
| --- | --- | --- | --- | --- |
| lamp | standard | 0 | (1.3) | (`[ 100, 400 ]`) |
| transmission | physical | 1 | 1.67 | `[ 395, 405 ]` |
| iridescence | physical | 1 | 1.8 | `[ 485, 515 ]` |

Both iridescent materials read their thickness from the green channel of a
map. `tests/gltf_loader.rs::iridescence_lamp_materials` checks this table.

## Three's own score (step 0)

`npm run test-e2e-webgpu -- webgpu_loader_gltf_iridescence`, run twice under
the GPU lock: **Diff 0.0%, TEST PASSED** both times. So the page is graded.

## History: the stale branch

This page was first ported on `rung-gltf-iridescence` (issue #229), which
reached 1534 pixels and stopped there. That branch's notes put the residual
in the equirect-to-cube background, but then cleared the conversion by
measurement: doubling the cube face moved the count from 1534 to 1603. The
port onto main found two causes, neither of them in the background:

1. **The camera turns before the screenshot.** OrbitControls auto-rotates at
   `autoRotateSpeed = -0.5`. With no `deltaTime`, each `update()` turns
   `2π / 3600 · speed`. Three calls `update()` three times before the
   screenshot: once in `init()` after `autoRotate` is set, once in `init()`'s
   explicit `render()`, and once in the single animation frame the harness
   fires. The old port made none of these calls. The three turns add up to
   about 0.15°, which moves every high-contrast edge by under a pixel. That
   matches the old notes' "sub-pixel, edges only" signature. Swept on main:
   zero turns gives 1548 pixels, and three turns gives 0 once the second fix
   is in.
2. **`evalIridescence`'s loop vars were reset on every iteration.** `I` and
   `Cm` are `toVar()`s declared before the `Loop`. The port's var is written
   at its first read, which was inside the loop body, so the Airy sum lost
   its first term. The film then faded from the shade (656 pixels with the
   camera fixed). `eval_iridescence_body` now pushes both vars as statements
   ahead of the loop. `docs/nodes.md` §95 has the details.

## What was re-applied from the old branch, and what changed

* `KHR_materials_iridescence` in the glTF loader (`GltfIridescence`, plus
  the parsed but unread `iridescence_texture`, as in three).
* The four `Material` fields, their uniforms, the TSL accessors and the
  `Iridescence*` properties.
* `evalIridescence`, `evalSensitivity`, `Fresnel0ToIor` / `IorToFresnel0`,
  the `start()` branch, the iridescent F0 in `compute_multiscattering()` and
  `BRDF_GGX`'s `USE_IRIDESCENCE` blend, all in `src/materials/physical.rs`.
* Changed: the old branch added an `inclusive` flag to `Node::Loop`. Main
  already has `loop_options(…, "<=", …)`, so the loop uses that. The old
  `Schlick_to_F0` duplicate is dropped for main's `schlick_to_f0`. The mapped
  thickness reads one shared minimum uniform, where the old branch built two.

## Gates

* `nodes_display_wgsl::gltf_iridescence_lamp_matches_three`, against
  `tests/fixtures/nodes_display/webgpu_loader_gltf_iridescence_m14_lamp_iridescence.wgsl`.
  It compares `evalIridescence`, the three `Iridescence*` assigns and the
  metallic F0 `mix`.
* `gltf_loader::iridescence_lamp_materials`.
* The e2e rung `webgpu_loader_gltf_iridescence`, which is also on the
  `steady_frame_builds_nothing` list.

## Known differences

* The dielectric iridescent F0 is emitted as a temp, with its `mix` rebuilt
  for each of its two readers. Three emits one shared `mix`. The values are
  the same. This follows from the port's separate indirect-diffuse
  scattering pair (`docs/nodes.md` §8, §95.2).
* Not ported: the GUI. Dispersion and retroreflection stay off, and no
  material here uses them.
