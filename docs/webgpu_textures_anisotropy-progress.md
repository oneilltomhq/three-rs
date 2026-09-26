# `webgpu_textures_anisotropy`

Status: **ported, not graded on this machine.** The port's frame is
pixel-identical to three.js 5f610f5's own frame for the page (max channel
difference 0 against `tools/dump-webgpu.mjs`' `actual_full.png`). But three
itself scores 9234 of 100000 pixels (9.2%) against its own
`examples/screenshots/webgpu_textures_anisotropy.jpg` here, so the port scores
the same 9234. Intel Iris Xe, Mesa 25.3.6, wgpu on Vulkan. The e2e test is
`#[ignore]`d with that reason, as `webgpu_instance_path`'s is, and the page
sits in the steady-frame strip, which checks that frames two and three build
and upload nothing.

The page draws two scenes into one frame through the scissor. Each has the
same crate-textured floor, repeated 512 times over 100 000 units. The left
half's sampler has `maxAnisotropy` 16 (`renderer.getMaxAnisotropy()`), the
right half's has 1. `autoClear` is off and one manual `renderer.clear()` comes
first.

## Grade first

| | pixels of 100000 |
| --- | --- |
| three.js 5f610f5 against its own reference JPEG | 9234 |
| this port against the same JPEG | 9234 |
| this port against three's frame on this machine | 0 (max channel difference 0) |

The differing pixels are all on the floor, in both halves, where the texture
is minified. The right half has no anisotropy at all, so the reference's
sampler differs in mip selection and filtering, not only in anisotropy. That
is the GPU the reference was rendered on. Nothing in the port can close the
gap without deriving from the reference.

Anisotropy is live in the port. With the left texture forced to
`anisotropy = 1`, 87 965 of the left half's 200 000 full-resolution pixels
move against three's frame, and none of the right half's do.

## What was added

| area | what |
| --- | --- |
| `src/renderer/bindings.rs` | `SamplerKey::of()` keeps a texture's `anisotropy` only when mag, min and mipmap filters are all linear, as `WebGPUTextureUtils.updateSampler()` does. wgpu rejects anything else. |
| `src/renderer/mod.rs` | `Renderer::get_max_anisotropy()`, `WebGPUCapabilities`' constant 16 |
| `src/testing.rs` | `composite_over_page()`: Chrome's source-over of a premultiplied canvas onto the page's `<body>` colour |
| `tools/dump-webgpu.mjs` | a stale `pageFile` line, left by a merge, that made every dump throw |
| `examples/` | `webgpu_textures_anisotropy.rs` |

`docs/nodes.md` §46 is the long form.

## What the pixels found

**The canvas is transparent.** The renderer's `alpha` defaults to true, so
its clear colour is `( 0, 0, 0, 0 )` (`Renderer._clearColor`, and three's
dump shows the clear pass with `a: 0`). The floor ends at the far plane,
25 000 units away, well below the horizon. Everything above that line, and the
two-pixel gutter `setScissor( 0, 0, SCREEN_WIDTH / 2 - 2, … )` leaves between
the halves, is the page's `body { background-color: #f1f1f1 }` showing
through. `page.screenshot()` grades the composite. The port grades the same
composite, using the page's own CSS colour (`PAGE_BACKGROUND`). Before this
page, every rung either covered its canvas or sat on `example.css`'s black
body, where the composite changes nothing.

No WGSL is new. `MeshPhongMaterial` with a `map` and linear `Fog` was already
ported, and the only new GPU state is the sampler descriptor.
