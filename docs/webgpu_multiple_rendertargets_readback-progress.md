# `webgpu_multiple_rendertargets_readback`

Status: **green.** 22 of 100000 pixels against three.js 5f610f5's own
`test/e2e/image.js`, threshold 0.1%. Intel Iris Xe, Mesa 25.3.6, wgpu on
Vulkan. Steady frame 2.4 ms, 3 draw calls, 8194 triangles.

`webgpu_multiple_rendertargets` reached a second way. The MRT is set around
the scene render only (`setMRT( sceneMRT )` … `setMRT( null )`). The composite
is a `QuadMesh` over a bare `NodeMaterial`, so the canvas transform runs as
its own `outputColorTransform` pass. The page's Inspector also offers
`'diffuse'` and `'normal'`. These modes render into a 512² `readbackTarget`,
read one attachment back with `readRenderTargetPixelsAsync()` into a
`DataTexture`, and draw that texture instead. The graded frame is `'mrt'`.

## Reconciling with the plans

Ported from the page directly. The dump in
`target/dumps/webgpu_multiple_rendertargets_readback/` has the same `m01` /
`m02` as the first page's. Its `m04` is the quad, diffed against `dump_wgsl`'s
`multiple_rendertargets_readback_quad`. The readback target and the
`DataTexture` never reach the GPU in the graded frame, and three's dump
confirms the same: no 512² texture is created.

## What was added

| area | what |
| --- | --- |
| `src/renderer/mod.rs` | `read_render_target_pixels()`: `readRenderTargetPixelsAsync( rt, x, y, w, h, textureIndex )`, blocking |
| `src/textures/texture.rs` | `Texture::data_rgba8()`: `new DataTexture( Uint8Array, w, h )` |
| `examples/` | `webgpu_multiple_rendertargets_readback.rs` (with `Selection` for the dropdown); the `dump_wgsl` quad section |

## What the pixels found

The pixels found nothing new. The frame went green on its first run, at the
same 22 pixels as the first page. The only difference in the drawing is where
the canvas transform runs.

The steady frame did find something. The quad switched between the composite
and the readback material by `clone()`, and each clone mints a new
`MaterialId`. So every frame built one program, 2 buffers and 2 bind groups,
and `steady_frame_builds_nothing` failed. The example now owns both materials
and swaps them into the quad only when the selection changes (72d22f6).

The ungraded branch was run by hand (`THREE_RS_SELECTION=diffuse|normal`).
`'diffuse'` shows the knot read back through the `DataTexture`. `'normal'` is
black, because the page never names `readbackTarget`'s textures, so `normal`
is not written (§52.4).
