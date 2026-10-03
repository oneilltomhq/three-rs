# `webgpu_refraction`

Status: **ported, not graded on this machine.** Three.js 5f610f5 scores 344 of
100000 pixels (0.34%) against its own
`examples/screenshots/webgpu_refraction.jpg` here (`tools/dump-webgpu.mjs`'
`actual.jpg`), over the 0.1% limit. The port scores 336 against the same
JPEG. Against three's own frame on this machine (`actual_full.png`), 905
pixels differ at all and 13 by more than 2 of 255. Intel Iris Xe, Mesa 25.3.6,
wgpu on Vulkan. The e2e test is `#[ignore]`d with that reason, as
`webgpu_postprocessing_afterimage`'s is. The page sits in the steady-frame
strip, and frames two and three build and upload nothing (8 draw calls, 33
triangles).

`webgpu_mirror`'s box of five Phong walls and a flat-shaded icosahedron, cut
in half by a vertical plane that refracts. The plane is a transparent
`MeshBasicNodeMaterial` whose `backdropNode` is
`viewportSharedTexture( viewportSafeUV( screenUV + offset ) )`, with the
offset taken from a checkerboard normal map.

## Grade first

| | pixels of 100000 |
| --- | --- |
| three.js 5f610f5 against its own reference JPEG | 344 |
| this port against the same JPEG | 336 |
| this port against three's frame on this machine, > 2 of 255 | 13 (max 198) |

The 13 pixels lie on the refracted checker edges. There, the normal-map
offset puts the read texel on one side or the other of a wall edge, so a
rounding difference in the offset flips the texel. The 892 pixels that differ
by 1 or 2 are the same rounding spread over the plane.

## What was added

| area | what |
| --- | --- |
| `src/renderer/screen_reads.rs` | the framebuffer copies: colour into the shared `FramebufferTexture`, depth into the shared `DepthTexture` |
| `src/nodes/display/viewport_texture.rs` | `viewportSharedTexture`, `viewportTexture`, `viewportDepthTexture`, `viewportLinearDepth`, `viewportSafeUV` |
| `src/materials/` | `backdropNode` / `backdropAlphaNode`; a backdrop material goes in the transparent list |
| `examples/` | `webgpu_refraction.rs`; `dump_wgsl`'s `dump_refraction()` |
| `tests/nodes_display_wgsl.rs` | `refraction_backdrop_matches_three`: the refractor's fragment body against three's `m06` |

`docs/nodes.md` §58 is the long form.

## The dump

The refractor's fragment WGSL matches three's `m06` apart from variable
names and declaration order. That includes the `BasicLightingModel` sum the
backdrop mixes into, which starts with three's `indirectDiffuse = vec4( 0 ).xyz`
store, and the `viewportSafeUV` depth test against the copied depth texture.
