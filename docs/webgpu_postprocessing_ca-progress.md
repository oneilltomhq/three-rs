# webgpu_postprocessing_ca

Branch `rung-room-environment`, cut from `origin/main` 2c96ac5. **PASS — 3 of
100000 pixels differ (0.003%), limit 0.1%.**

The rung is `RoomEnvironment` as a capability; `webgpu_postprocessing_ca` is
the gate. The plan and the environment survey are
`scouts/scouts/environment-family/ENVIRONMENT-FAMILY.md` (rows 4 and 5) and
`scouts/scouts/postprocessing-batch/PLAN.md`; the reference WGSL is
`scouts/scouts/environment-family/dump-postprocessing_ca/`.

## Why this example and not `webgpu_materials_alphahash`

Both were put through Three's own grader, twice each, under the GPU lock,
before any porting started. Only one of them grades on this machine:

| example | run 1 | run 2 |
|---|---|---|
| `webgpu_postprocessing_ca` | 0.0% | 0.0% |
| `webgpu_materials_alphahash` | 3.5% | 3.5% |

Three itself fails its own reference for `webgpu_materials_alphahash` here, so
it cannot gate anything, and the choice made itself. It is the cheaper gate
anyway: the whole lighting is `scene.environment = pmremGenerator.fromScene(
new RoomEnvironment(), 0.04 )`, so a wrong atlas is visible in the graded
frame, and the effect on top is one `Fn` with four texture samples.
Both pages' dumps are kept at
`scouts/scouts/environment-family/dump-postprocessing_ca/` and
`dump-materials_alphahash/`.

## What was added

| area | what |
|---|---|
| `src/environments/room_environment.rs` | `RoomEnvironment` — the room box, the six-instance `InstancedMesh`, the point light and the six emissive-only Lambert panels |
| `src/renderer/pmrem.rs` | `fromScene` takes three's `sigma` and runs `_blur( target, 0, 0, sigma )` before `_applyPMREM` |
| `src/nodes/display/chromatic_aberration.rs` | `chromatic_aberration()` — `ChromaticAberrationShader` as an `Fn` with a declared layout |
| `src/renderer/render_list.rs` | `Frustum.intersectsObject` fills in `InstancedMesh.boundingSphere` lazily (see below) |
| `tests/` | `room_environment.rs` + `tests/fixtures/room_environment.json`, the numeric gate; the rung's e2e test and its `rung!` row |
| `examples/` | `webgpu_postprocessing_ca.rs`, plus `room_box` / `room_boxes` / `room_panel` / `ca_rtt_quad` / `ca_render_pipeline_quad` in `dump_wgsl.rs` |

## What the pixels found

Two bugs, both invisible until the atlas was compared against three's own.

* **The frame came back black (16.6%).** `read_canvas_pixels` asked
  `prepare_canvas` for one sample, and `prepare_canvas` reallocated the whole
  canvas — including the resolved colour texture — on any sample-count change.
  This page is the first on the ladder with `antialias: true` *and* a
  `RenderPipeline`, so the canvas was at four samples and the readback handed
  back a freshly created, zero-filled texture. A readback must never be the
  thing that reallocates what the frame drew into. 16.6% → 0.8%.

  `webgpu_tsl_interoperability` hit the same wall from the other side and got
  there first: the readback on main now skips `prepare_canvas` entirely when a
  canvas already exists, which fixes this page too, so this rung carries no
  renderer change for it.
* **The remaining 0.8% was the whole `InstancedMesh` missing from four of the
  six PMREM faces.** `Frustum.intersectsObject( object )` in three.js reads
  `object.boundingSphere` when the object *has* the field, computing it on
  first use — `InstancedMesh` sets it to `null` in its constructor, and
  `computeBoundingSphere()` unions the geometry sphere pushed through every
  instance matrix. The port had `compute_bounding_sphere()` and never called
  it, so an instanced draw was culled against the one geometry sphere at the
  object's origin. In a `RoomEnvironment` that sphere is a unit box 3.5 below
  the cube camera: in front of the camera looking down (`-y` matched exactly),
  outside the 45° half-FOV of all four side faces. All six room boxes vanished
  from `+x`, `-x`, `+z` and `-z`.

With both fixed the 768×1024 atlas matches three's to half-float rounding —
**mean absolute difference 0.000001, maximum 0.001**, at both `sigma = 0` and
the page's `sigma = 0.04`, over all six faces and every mip level.

## What was ruled out along the way

Recorded because each cost a run and each looked plausible:

* **"MSAA + `PassNode` is broken in the port."** Two probe tests disproved it:
  MSAA render targets, a `PassNode` with MSAA, and sampling a pass into an MSAA
  destination all work. The bug was one layer further out, in the readback.
* **The blur's calibration.** A sigma sweep (0 → 877 pixels, 0.04 → 799,
  0.08 → 832, 0.15 → 1493) put the minimum at the page's own 0.04, and the
  unblurred atlas matched three's exactly, so `_blur` was never the suspect it
  looked like.
* **`RoomEnvironment` itself.** Every local matrix, every instance matrix, the
  light's four parameters and the eight materials' fields are gated against
  three's own construction at `EPS = 1e-12` (`tests/room_environment.rs`), so
  the scene was provably right while the render of it was wrong.
* **The effect.** `chromaticAberration()` samples green at `greenScale = 1.0`
  with zero offset, so the green channel of the graded image is exactly the
  scene render. The residual was on every environment-lit reflective shape in
  green too — the environment, not the aberration.

## `fullscreenPass`

This sitting found the same gap `webgpu_postprocessing_anamorphic` found:
`RenderContext.fullscreenPass = scene.isQuadMesh === true`, and
`Renderer.currentSamples` returns 0 for one, which is why three's own dump of a
post-processing page shows the final quad at `multisample { count: 1 }` while
the scene pass it samples is 4x. `Renderer::fullscreen_pass` landed with that
rung, and with it on this page the graded frame ends on a single-sample quad —
which, together with main's readback, is why this rung needs no renderer change
of its own for either half.

## WGSL

Against `dump-postprocessing_ca/`, and `docs/nodes.md` §22:

* **`m02`..`m07`**, the three room programs — statement for statement three's;
  varying order, `var` hoisting and duplicated zero-inits differ, all §8
  entries.
* **`m16`/`m17`**, the RTT quad — `main` byte-identical; the three helper
  functions are emitted in a different order.
* **`m18`/`m19`**, the `RenderPipeline` quad — the same four samples and the
  same returned swizzles; the port hoists two subexpressions three inlines and
  gives each sample's `.toVar()` a var of its own, twelve against six.

## What was left out

* **`OrbitControls`' auto-rotation.** `controls.autoRotate` is on and
  `update()` runs twice, moving the azimuth 5.1e-5 rad in total — 2e-3 world
  units at a radius of 42, a fiftieth of a pixel. What `update()` does that
  does matter, aiming the camera at `( 0, 0.5, 0 )`, the example does.
* **`PointsMaterial.size` / `sizeAttenuation`.** Not modelled, and that is
  three's behaviour rather than a gap: WebGPU supports only 1-pixel points, so
  `setupVertexSprite()` — the only reader of either — runs for a `Sprite` and
  never for a `Points`.
* **`RoomEnvironment.dispose()`.** Nothing on the ladder disposes an
  environment; `Drop` frees the same resources.
* **`webgpu_materials_alphahash`.** Graded and dumped, not ported. It is the
  natural next rung now that the environment under it is green, but it needs a
  reference Three itself can hit on this machine first.
