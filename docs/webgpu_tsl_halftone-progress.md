# `webgpu_tsl_halftone`

Status: **green.** 93 of 100000 pixels against three.js 5f610f5's own
`test/e2e/image.js`, threshold 0.1%. Intel Iris Xe, Mesa 25.3.6, wgpu on
Vulkan. Steady frame 3.1 ms, 4 draw calls, 44363 triangles.

Two TSL halftone dot screens (a purple shade and a cyan highlight) are mixed
into every material's `output` through `material.outputNode = halftones(
output )`. The page applies them to a default `MeshStandardNodeMaterial` on a
torus knot and a sphere, and to every material of the skinned `Michelle.glb`
in its bind pose. The scene has one ambient and one directional light.

## Reconciling with the plans

There is no scout plan. The rung was ported from
`~/src/vendor/three.js/examples/webgpu_tsl_halftone.html`, and three's WGSL
was dumped by `tools/dump-webgpu.mjs` into `target/dumps/webgpu_tsl_halftone/`
(uncommitted). `examples/dump_wgsl.rs`'s new `tsl_halftone_default` and
`tsl_halftone_body` sections are compared against three's `m00` / `m01` and
`m03` / `m04` (`Ch03_Body`).

Three itself scores 93 pixels against its own reference JPEG on this
machine. The port's 800x500 frame is pixel-identical to three's own frame
(`actual_full.png`, 0 differing pixels, max channel difference 0), so it
scores the same 93.

## What was added

| area | what |
| --- | --- |
| `src/nodes/tsl.rs` | `resolve_fn_call()`: runs an argument-less inline `Fn()` call's body in the current material's context. `resolve_fog_factor()` now delegates to it |
| `src/materials/node_material.rs` | a deferred `outputNode` is resolved in `setup_inner`, inside the material's normal / side / tangent context |
| `examples/` | `webgpu_tsl_halftone.rs`; `dump_wgsl` sections `tsl_halftone_default`, `tsl_halftone_body` |

`docs/nodes.md` §53 is the long form.

## What the pixels found

**Michelle's halftone read the wrong normal.** The first grade was 134 pixels,
all of them on Michelle (hair buns, and scattered pixels on the body). The
torus and sphere matched three's frame exactly. The body's fragment shader
showed why: just before `normalWorld`, the port re-assigned
`normalView = normalViewGeometry`, while three's reads the normal-mapped,
`DoubleSide`-negated `normalView` the lighting had used. The halftone graph
was built eagerly in `init()`, with no material in scope, so
`normal_world()`'s cache key had no normal map and a front side. Three's `Fn`
body runs lazily inside each material's build. With `halftones` held as an
argument-less inline call and run during `NodeMaterial` setup (the seam
`scene.fogNode` already used), the rung went from 134 px to 93 px, and the
frame is identical to three's.

The ladder did not move. No other rung has a deferred `outputNode`, and
`resolve_fog_factor`'s behaviour is unchanged.
