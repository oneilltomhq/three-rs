# `webgpu_loader_gltf_transmission`

Status: **green.** 6 of 100000 pixels against three.js' reference
(`examples/screenshots/webgpu_loader_gltf_transmission.jpg`), threshold 0.1%.
Intel Iris Xe, Mesa 25.3.6, Vulkan. Steady frame 2.9 ms, 6 draw calls, 26433
triangles. Issue #230. `docs/nodes.md` §94 is the long form.

`IridescentDishWithOlives.glb` under the same `royal_esplanade_2k.hdr.jpg`
PMREM as the anisotropy rung (`scene.background` and `scene.environment`,
`backgroundBlurriness = 0.35`), ACES at exposure 1, a 45° camera at
`( 0, 0.4, 0.7 )` orbiting `( 0, 0.1, 0 )`, and one animation clip. Four
materials: two transmissive glasses stacked over each other (`glassDish`,
`glassCover`), the opaque `olives`, and the `alphaMode: MASK` `goldLeaf`.

## three.js against its own reference

Measured first, before any Rust, with
`npm run test-e2e-webgpu -- webgpu_loader_gltf_transmission` in the pinned
checkout, twice: **passes both times**, `Diff 0.0%` (under 0.05% of pixels;
the harness prints one decimal). So the page is graded.

## History: the old branch

The page was first ported on `rung-gltf-transmission` (tip `2f3f53d`,
branched 19 Sep from `8ec1d5b`) and could not run: every primitive in the
asset is `KHR_draco_mesh_compression`, listed in `extensionsRequired`, and
nothing decoded Draco. Before a guard existed the file loaded "successfully"
into four meshes with the right counts and every value zero, because a Draco
primitive's accessors have no `bufferView` and a `bufferView`-less accessor is
a legal zero-filled one.

Carried forward onto main (branch `gltf-transmission-page`), file by file:

| old branch | on main | here |
| --- | --- | --- |
| `GltfError::UnsupportedRequiredExtension`, `SUPPORTED_EXTENSIONS`, `check_required_extensions()` (`d3dccfe`) | landed as #125 (`4a67156`) | dropped |
| `draco_required_is_an_error` in `tests/gltf_loader.rs` | contradicts main, whose `SUPPORTED_EXTENSIONS` has `KHR_draco_mesh_compression` | dropped |
| the Draco blocker | the pure-Rust decoder (`src/loaders/draco`, #139 / #150) | the asset decodes with real positions |
| `alphaMode: MASK` left unwired | `out.alpha_test = alpha_cutoff`, emitted as the `materialAlphaTest` uniform | gated against three's dump (below) |
| `examples/webgpu_loader_gltf_transmission.rs` | — | re-applied, brought up to the sheen/anisotropy page shape |
| `docs/nodes.md` §28, this doc | — | renumbered to §94, rewritten |

## What changed in the example against the old branch

* **`OrbitControls` instead of a `lookAt`.** The old branch replaced the
  controls with `camera.look_at()`, reasoning that the graded auto-rotation
  delta is 0. It is not: the page calls `controls.update()` *without* a delta,
  and `_getAutoRotationAngle( null )` is the frame-count branch,
  `2π / 60 / 60 * autoRotateSpeed` per call, which the pinned clock does not
  stop. The page calls it once in `init()` and once per frame, through
  `enableDamping`'s `dampingFactor`, so the graded camera is turned by two
  damped steps of `-0.75` speed. The port now builds the controls with the
  page's settings and calls `update( None )` in the same two places.
* **A `Timer`.** `mixer.update( timer.getDelta() )` as the page does it; the
  first delta is 0 under the pinned clock, so the graded pose is the clip's
  first keyframe, as in `webgpu_skinning`.
* `resize()`, `controls()`, `controls_and_camera()` and `pin_time` in `main()`,
  the shape every graded page has for the viewer and the browser shell.

## Gates

* The rung: `webgpu_loader_gltf_transmission` in `tests/e2e/main.rs`, and on
  the `steady_frame_builds_nothing` list (frames two and three build,
  compile and upload nothing; the mixer at delta 0 and the auto-rotating
  camera write uniforms only).
* `alphaMode: MASK`: `gltf_transmission_gold_leaf_alpha_test_matches_three`
  in `tests/nodes_display_wgsl.rs`, against three's `goldLeaf` fragment
  module from this page's dump
  (`tests/fixtures/nodes_display/webgpu_loader_gltf_transmission_m12_gold_leaf.wgsl`):
  the base-colour map times the `VEC4` `COLOR_0`, the opacity, the discard
  against `object.nodeUniformN` (the uniform, not a literal) and the opaque
  `DiffuseColor.w = 1.0`.

No new material or TSL code was needed: the transmission path, the
`KHR_materials_*` extensions this asset uses, vertex colours, the Draco
decoder and the MASK uniform were all already on main. What this page adds is
the first graded frame of each of them on a glTF.

## Measurements

| what | pixels of 100000 |
| --- | --- |
| three.js against its reference, run 1 | 0.0% (pass) |
| three.js against its reference, run 2 | 0.0% (pass) |
| the port, first run on current main | 6 |

The steady-frame time is the e2e grader's (frames 2..4: 3.38, 2.91, 2.90
ms). `viewer --headless --frames 40` measured 6 to 12 ms for this page and
13 ms for the anisotropy page (README: 3.2 ms) during the same session,
with sibling agents building on the machine, so the grader's quieter number
is the one in the README table.

## What remains

Nothing for the rung. What it does not test, and stays open elsewhere:

* `KHR_materials_iridescence`: despite the asset's name it is not in the
  file (#229 is the iridescence rung).
* Double-pass transmission: neither glass is `doubleSided`.
* `attenuationDistance`: the volume extension here leaves it at infinity.
