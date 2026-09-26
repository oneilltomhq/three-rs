# `webgpu_texturegather`

Status: **green.** 16 of 100000 pixels against three.js 5f610f5's own
`test/e2e/image.js`, threshold 0.1% (limit 100). Intel Iris Xe, Mesa 25.3.6,
wgpu on Vulkan. Steady frame 1.8 ms (release, `viewer --headless`), 4 draw
calls, 6 triangles. Frames 2 and 3 build nothing.

`init()` renders a red `MeshStandardNodeMaterial` cube, lit by a directional
and an ambient light, once into a 100 x 100 render target with a
`DepthTexture`. A unit plane under an orthographic camera then reads the
target back. The top half is `textureGather( 0, … )` of the colour target,
tiled ten times through `RepeatWrapping`. The bottom half is
`textureGatherCompare` of the depth against 1. Both taps carry an `ivec2( 0,
7 )` texel offset. As in `webgpu_texturegrad`, the page's WebGPU and WebGL
canvases are the two halves of one canvas.

## Reconciling with the plans

There is no scout `PLAN.md`. It was ported from
`~/src/vendor/three.js/examples/webgpu_texturegather.html`, with three's WGSL
dumped into `target/dumps/webgpu_texturegather/` (uncommitted). The colour
path is the `m04` fragment.

## What was added

| area | what |
| --- | --- |
| `src/nodes/node.rs`, `builder.rs` | `SampleMode::Gather { component, offset }` and `SampleMode::GatherCompare { compare, offset }`, emitted as `generateTextureGather()` / `generateTextureGatherCompare()` write them, including three's `vec4<f32>( … )` cast of an `UnsignedInt` depth texture's compare |
| `src/nodes/tsl.rs` | `texture_gather( map, uv, component, offset )`, `depth_texture_gather_compare( depth, uv, compare, offset )` |
| `examples/` | `webgpu_texturegather.rs`; a `dump_wgsl` section `texturegather` |
| `tests/` | `nodes_texture_wgsl.rs::texturegather_fragment_matches_three` against three's verbatim `m04` |

`docs/nodes.md` §44 is the long form.

## What the pixels found

The first frame graded 16 pixels, all of them at the corners of the
depth-compare pentagon in the left half. There the four gathered depth texels
straddle the cube's silhouette, and a last-bit difference in the rasterised
depth flips one comparison. The page asks the target for `generateMipmaps:
true` and a `LinearMipmapLinearFilter`, but `textureGather` reads mip 0 and
ignores filters, so the port leaves both out.
