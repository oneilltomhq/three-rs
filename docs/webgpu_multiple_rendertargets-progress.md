# `webgpu_multiple_rendertargets`

Status: **green.** 22 of 100000 pixels against three.js 5f610f5's own
`test/e2e/image.js`, threshold 0.1%. Intel Iris Xe, Mesa 25.3.6, wgpu on
Vulkan. Steady frame 2.2 ms, 2 draw calls, 8193 triangles.

A hardwood-textured torus knot drawn once into a two-attachment
`RenderTarget` (`{ count: 2 }`, `NearestFilter`), with `output` and
`normal: normalWorld` set on the renderer through `renderer.setMRT()`. A
`RenderPipeline` then shows `output` on the left half of the canvas and
`normal` on the right.

## Reconciling with the plans

There is no scout `PLAN.md` for this rung. It was ported from
`~/src/vendor/three.js/examples/webgpu_multiple_rendertargets.html` directly.
Three's WGSL was dumped by `tools/dump-webgpu.mjs` into
`target/dumps/webgpu_multiple_rendertargets/`, which is not committed, per the
rules. The dump was diffed against `examples/dump_wgsl.rs`'s new
`multiple_rendertargets_knot` and `multiple_rendertargets_composite` sections
(three's `m02` and `m04_fragment_fragment_RenderPipeline`).

## What was added

| area | what |
| --- | --- |
| `src/renderer/render_target.rs` | `set_count()` (`{ count }`) and `set_texture_name()` (`textures[ i ].name`) |
| `src/renderer/mod.rs` | extra MRT attachments clear to `( 0, 0, 0, 1 )`, as `WebGPUBackend.beginRender()` does |
| `examples/` | `webgpu_multiple_rendertargets.rs`; the two `dump_wgsl` sections |

`docs/nodes.md` §52 is the long form.

## What the pixels found

**The second attachment's clear colour.** The first run was at 42120 pixels:
the whole right half, grey where three's is black. Three clears attachment 0
to the renderer's clear colour and every other attachment to opaque black.
The port gave every attachment the scene's `0x222222`. No earlier MRT rung
could see this. `webgpu_mrt` has a skybox over every pixel, and the other MRT
passes clear to black anyway or only read where there is geometry. The full
ladder did not move. With the fix, the frame is at 22 pixels, scattered along
the knot's silhouette.
