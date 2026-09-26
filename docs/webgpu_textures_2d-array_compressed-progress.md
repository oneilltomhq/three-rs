# `webgpu_textures_2d-array_compressed`

Status: **green.** 0 of 100000 pixels against three.js' own
`test/e2e/image.js`, threshold 0.1%. Intel Iris Xe, Mesa 25.3.6, wgpu on
Vulkan. Steady frame 6.1 ms, 1 draw call, 2 triangles.

A 50 x 25 plane showing one layer of `textures/spiritedaway.ktx2`, a
six-layer UASTC array, 496 x 260, one level. Issue #172.

## Why this page and not `webgpu_loader_texture_ktx2`

Issue #172 names `webgpu_loader_texture_ktx2` as the rung to grade. It cannot
be graded at the threshold, by three or by the port. That page draws a label
for every sample into the DOM, and the reference screenshot captures the
labels; the WebGPU canvas is only part of the image. Three.js itself, at the
pinned commit and run through the same `test/e2e/image.js`, scores 2076 of
100000 against its own screenshot, and a canvas-only render — what any port
can produce — scores 3675. The threshold is 100. So this rung takes its
place: the same loader, three's own KTX2 file, an image that is all canvas.
Chrome scores 0 of 100000 on it.

## What was added

| area | what |
| --- | --- |
| `src/loaders/ktx2_loader.rs` | `KTX2Loader`: `detect_support`, the transcoder-target priority lists, the raw `FORMAT_MAP` / `TYPE_MAP` path, Zstandard, `parseColorSpace`, `getFormat` → `wgpu::TextureFormat` |
| `src/textures/compressed_texture.rs` | `Texture::compressed` / `compressed_array` (three's `CompressedTexture` / `CompressedArrayTexture`) |
| `src/textures/texture.rs` | `mipmaps`, `depth`, `premultiply_alpha`; `NearestMipmapNearest` |
| `src/renderer/mod.rs` | the device requests `TEXTURE_COMPRESSION_BC` / `_ETC2` / `_ASTC` when the adapter has them; `upload_mipmaps` (block-counted strides, every layer, no generated mips); a `D2Array` view for an array texture |
| `src/nodes/` | `TextureKind::Sampled2DArray`, `SampleMode::SampleLayer`, `tsl::texture_array` |
| `src/loaders/gltf_loader.rs` | `KHR_texture_basisu` |
| `tools/ktx2_reference.mjs`, `tests/ktx2_loader.rs` | the oracle: three's loader under node, compared byte for byte |

`docs/nodes.md` §29 has the divergences.

## WGSL

`examples/dump_wgsl.rs`' `textures_2d_array_compressed` against three's `m00`
/ `m01`: the same bindings (sampler, `texture_2d_array<f32>`, the object
struct with the layer, the opacity and the model matrix), the same statements.
Two differences, both in `docs/nodes.md` §8: the uniform / varying numbering,
and three's `let nodeVar0 = vec2<f32>( nodeVar0.x, 1.0 - nodeVar0.y )`
re-declaration of the flipped uv, which the port writes inline.

## What the pixels found

Nothing to chase: the first render scored 0. The rung was also run with the
loader forced to the uncompressed fallback (`Ktx2Loader::new()` without
`detect_support`), which is what a browser without `texture-compression-bc`
would get; that also scores 0.

The layer is `1`. The page sets `depthStep = 1` and adds `timer.getDelta() *
10` each frame; under the harness the clock is pinned, so the delta is 0 on
every frame including the steady ones.
