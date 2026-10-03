# `webgpu_lightprobe_cubecamera`

Status: **green.** 0 of 100000 pixels against three.js' own
`test/e2e/image.js`, threshold 0.1%. Intel Iris Xe, Mesa 25.3.6, wgpu on
Vulkan. Steady frame 1.6 ms, 3 draw calls, 2945 triangles; the steady frame
builds nothing. Not graded in the browser (see below).

The pisa background, rendered once into a 256² RGBA8 cube render target by a
`CubeCamera` at the origin, read back, and projected by
`LightProbeGenerator.fromCubeRenderTarget()`. A radius-5 `LightProbeHelper`
draws the result.

## What was added

| area | what |
| --- | --- |
| `src/cameras/cube_camera.rs` | `CubeCamera`: six `PerspectiveCamera( -90, 1, near, far )` children, `update( renderer, scene )` |
| `src/addons/lights.rs` | `from_cube_render_target`, with three's WebGPU (`flip = 1`) face table |
| `src/renderer/mod.rs` | one face of a cube read back as bytes (`RGBA8` or `RGBA16F`) |
| `tests/renderer_cube_camera.rs` | GPU: the probe from a captured cube agrees with the probe from the same cube's PNGs |

`docs/nodes.md` §60.3 and §60.4 are the long form, `webgpu_lightprobe`'s note
covers the probe itself.

## What the pixels could not see

As on `webgpu_lightprobe`, a wrong sign in the face table stays under the
colour threshold. `tests/renderer_cube_camera.rs` captures the pisa
background with a `CubeCamera` and projects it with
`from_cube_render_target()`. It then compares the result with
`from_cube_texture()` on the PNGs, coefficient by coefficient, to 1% of the
DC term. A swapped axis or sign in either the cube camera's orientations or
the render-target face table fails it.

## Divergences

- `CubeCamera` renders each face into a 2-D target and copies it into the
  cube's layer. three binds the layer as the attachment. `activeMipmapLevel`
  is always 0, and the cube camera's `layers` are not copied onto its
  children.
- **In the browser too.** `fromCubeRenderTarget()` is async in three, and so
  is the port's `from_cube_render_target_async()`: the example's `init()`
  awaits it, the web shell awaits `init()` (the row's trailing `await`), and
  the native callers block on it for free. Issue #261 was the synchronous
  shell that kept this rung in `tools/web_gate.skip`.
