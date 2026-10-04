# `webgpu_oit` — weighted blended order-independent transparency

**Result: 0 of 100000 pixels different, limit 0.1%.**

The page renders an opaque torus knot among half-transparent planes and
spheres through `oitPass( scene, camera )` from
`examples/jsm/tsl/display/OITPassNode.js`. Opaque objects and transparents
that do not qualify go to the default pass. Qualifying transparents
(`transparent`, `NormalBlending`, no transmission, no backdrop) accumulate into
a two-attachment target that shares the default pass's depth. A full-screen
composite then mixes the weighted average over the beauty by the revealage.

## What was added

| Area | What | Why |
|---|---|---|
| `src/nodes/display/oit_pass.rs` | `oit_pass()`, `OitPassNode` | `OITPassNode` / `oitPass`, line by line |
| `src/materials/blending.rs` | public `BlendMode`, `BlendMode::new()`, `From<Blending>` | `new BlendMode( CustomBlending )` with its factors |
| `src/nodes/mrt.rs` | `set_blend_mode` taking a `BlendMode`, `set_clear_color` / `clear_color`, typed members | `MRTNode.setBlendMode()`, `setClearColor()`, `getOutputType()` |
| `src/renderer/render_target.rs` | `RenderTarget::set_texture_name( 0, … )` | `texture.name = 'accum'`, which `getBlendMode( textures[ 0 ].name )` reads |
| `src/renderer/mod.rs` | `Renderer.oit`, per-attachment MRT clears and blends | `setRenderObjectFunction()` for the two passes, and `RenderContext`'s per-attachment clear values |
| `examples/webgpu_oit.rs` | the page, its viewer entry and web shell entry | `webgpu_oit.html` |

`docs/nodes.md` §82 has the detail and the "Not ported" list.

## What the WGSL found

`tests/nodes_display_wgsl.rs` diffs four of the page's seven dumped modules
against three's: the composite (`m06`), the default pass's knot (`m01`) and
both sides of the accumulation material (`m03`, `m04`).

- The composite first came out with its `vec4( mix( … ), beauty.a )` hoisted
  into a var, where three repeats it inline for `.rgb` and `.a`. Three's
  `PassNode.isCacheable()` is `false`, so the port returns the composite
  through a non-cacheable `CustomNode`.
- The `revealage` member is an `f32` at `@location( 1 )` in three, because
  `getOutputType()` gives each MRT member its attachment's channel count. The
  port now does the same.
- Above `outgoingLight` the lit materials differ only in ways §8 already
  lists, so those gates compare from `outgoingLight` on, plus the back side's
  `normalViewGeometry * -1.0` on its own.

The port builds the same four render pipelines as three: the knot, the two
OIT sides (`depthWriteEnabled: false`, `One`/`One` and
`Zero`/`OneMinusSrcColor`) and the composite.

## What the pixels found

Nothing: 0 of 100000 pixels. A numpy diff of the port's frame against three's
`actual_full.png` has a maximum channel difference of 0. That is a pixel
comparison, not a byte comparison of the PNG files.

On the graded first frame the knot does not hide the transparents. The OIT
target is new, so `_renderScene()` clears the depth it shares on its first
render (§82.3). Three's reference shows the same thing.

## Draw-order independence

`tests/oit_frames.rs` (GPU; listed in `tests/gpu_only`) draws two overlapping
half-transparent quads (red in front, blue behind) and an opaque quad.

- With `renderOrder` forcing the wrong painter's order, the overlap is the
  weighted average ( ≈ `[165, 0, 165]` ).
- Swapping the order gives byte-identical OIT pixels.
- A plain `pass()` of the same scene changes with the order: `[188, 0, 0]`
  wrong-way round, `[188, 0, 137]` right-way round.
- On the first OIT frame the transparents show through the opaque quad, from
  the depth clear above. From the second frame on, the opaque quad covers
  them.

## Steady frame

2.2 ms (runs: 2.36, 2.00, 2.18), 12 draws, 12045 triangles, on Intel Iris Xe
(RPL-U), Mesa 25.3.6, Vulkan. `steady_frame_builds_nothing` has a
`rung!( webgpu_oit )` entry: after the first frame no pipeline or program is
built.
