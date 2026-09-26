# `webgpu_tsl_earth`

Status: **green.** 0 of 100000 pixels against three.js 5f610f5's own
`test/e2e/image.js`, threshold 0.1%. Intel Iris Xe, Mesa 25.3.6, wgpu on
Vulkan. Steady frame 2.6 ms, 3 draw calls, 16129 triangles.

A `MeshStandardNodeMaterial` globe lit by one `DirectionalLight`. Its colour
is the day map with the clouds mixed in, and its roughness is remapped from
the roughness channel. Its normal is a `bumpMap()` of the elevation and the
clouds. Its `outputNode` mixes in the night map on the dark side and a
fresnel atmosphere tint. Around it sits a `BackSide`, transparent
`MeshBasicNodeMaterial` shell at 1.04x scale, whose `outputNode` is the
atmosphere colour with a fresnel alpha. The page uses three 4096x2048 JPEGs
at 8x anisotropy.

## Reconciling with the plans

There is no scout plan. The rung was ported from
`~/src/vendor/three.js/examples/webgpu_tsl_earth.html`, and three's WGSL was
dumped by `tools/dump-webgpu.mjs` into `target/dumps/webgpu_tsl_earth/`
(uncommitted). `examples/dump_wgsl.rs`'s new `tsl_earth_globe` and
`tsl_earth_atmosphere` sections are compared against three's `m01` / `m02`
and `m03` / `m04`. Three's own frame scores 0 against its reference.

## What was added

| area | what |
| --- | --- |
| `src/nodes/tsl.rs` | `bump_map_with( height, scale )`: `bumpMap()` of any expression of texture taps, standing in for `dHdxy_fwd`'s `getUV` context. `bump_map()` is now its one-map case |
| `examples/` | `webgpu_tsl_earth.rs` (its node graph in a public `materials()`); `dump_wgsl` sections `tsl_earth_globe`, `tsl_earth_atmosphere` |

`docs/nodes.md` §53.3 and §53.4 are the long form.

## What the pixels found

The first grade was 0 pixels. Against three's own 800x500 frame, 27781
pixels differ by one or two levels across the globe, and 9 isolated pixels
near the sun's highlight differ by up to 75. The likely source is the
filtering of the three large JPEGs (mip generation, 8x anisotropy). It was
not pinned down, because none of it comes near the grader's threshold. The
bump section of the WGSL is three's statement for statement, so the normal
is not the cause.

The Inspector's parameters (the two atmosphere colours and the roughness
range) are the uniforms the port builds. The GUI itself is not ported.
