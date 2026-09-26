# `webgpu_textures_partialupdate`

Status: **green.** 0 of 100000 pixels against three.js 5f610f5's own
`test/e2e/image.js`, threshold 0.1%. Intel Iris Xe, Mesa 25.3.6, wgpu on
Vulkan. Steady frame 9.29 ms (debug), 2 draw calls, 3 triangles.

A 2 x 2 plane showing `Carbon.png`, which the page patches every tenth of a
second with `renderer.copyTextureToTexture( dataTexture, diffuseMap, null,
position )`: a 32 x 32 `DataTexture` of one random colour, copied to a random
multiple of 32 texels.

## Reconciling with the plans

There is no scout `PLAN.md` for this rung. It was ported from
`~/src/vendor/three.js/examples/webgpu_textures_partialupdate.html` directly,
with three's WGSL dumped by `tools/dump-webgpu.mjs` into
`target/dumps/webgpu_textures_partialupdate/` (uncommitted). The dump has two
passes, the scene and the output transform, and one `MeshBasicMaterial` with a
`map`. Nothing in the node graph is new, so `dump_wgsl` has no new section.

## What was added

| area | what |
| --- | --- |
| `src/renderer/mod.rs` | `Renderer::copy_texture_to_texture`; `COPY_SRC` on every uploaded texture, as three's `createTexture()` gives it |
| `src/textures/texture.rs` | `Texture::data_rgba8`, `new DataTexture( Uint8Array, w, h )` |
| `examples/` | `webgpu_textures_partialupdate.rs` |
| `tests/e2e/main.rs` | the rung, plus a check of the copy itself |

`docs/nodes.md` §56 is the long form.

## What the pixels found

**The graded frame never copies.** Under the pinned clock
`timer.getElapsed()` is 0 on every frame, so three's page never reaches its
first `copyTextureToTexture` before the screenshot, and the port does not
either. The first run was at 0 pixels.

**So the copy is checked separately.** After the steady frames, the test pins
the clock at 150 ms. `animate()` then makes exactly one copy from the seeded
`Math.random()` sequence, and the next frame is compared with the frame
before it. The copy lands at texel `( 32, 288 )` in colour `( 2, 213, 60 )`.
The changed pixels span x 243..=265 and y 205..=227, and three's semantics
predict x 243.8..266.1 and y 205.4..227.7. That is the block counted up from
the bottom of the image, because `Carbon.png` was uploaded with `flipY`. The
block's centre shows the copied bytes. `target/e2e/webgpu_textures_partialupdate/patched.png`
is that frame.
