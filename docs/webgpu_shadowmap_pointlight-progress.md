# webgpu_shadowmap_pointlight

Branch `shadows-vsm`, issue #168. **PASS: 63 of 100000 pixels differ**
(0.063%, limit 0.1%). The steady frame is 5.0 ms in the viewer. The grader's
frame is 20 draw calls and 7421 triangles, and frames two and three build
nothing.

**The 63 pixels are the reference, not the port.** Three itself, rendered by
`tools/dump-webgpu.mjs` in Chrome on this machine, also differs from
`examples/screenshots/webgpu_shadowmap_pointlight.jpg` in 63 pixels. The
port's frame differs from that local Chrome frame in **0** pixels, measured
with the grader's own `compare()`. The differing pixels are on the edges of
the alpha-tested bands, where a driver's derivative and mip choice decides
which side of `alphaTest = 0.5` a texel lands.

## What was added

| area | what |
|---|---|
| `src/materials/mod.rs` | `MeshBasicNodeMaterial::{alpha_map, alpha_test}` |
| `src/materials/node_material.rs` | `MaterialNode.OPACITY` with an alpha map (`opacity * texture( alphaMap )`, narrowed by the assign); `alphaTest > 0` against `materialAlphaTest`; the shadow override inherits both |
| `src/nodes/{node,tsl}.rs`, `src/renderer/programs.rs` | the `materialAlphaTest` object uniform |
| `examples/` | `webgpu_shadowmap_pointlight.rs`, and the `pointlight_*` sections of `dump_wgsl.rs` |

The point shadows themselves (six-face cube depth, `PointShadowFilter`) were
already on the ladder. This rung exercises them for two lights at once, at a
128 map size with radius 10, through double-sided alpha-tested casters, with
the default `PCFShadowMap`.

## What the WGSL says

Three's dump has 11 modules. The override `ShadowMaterial` (`m04`) matches
statement for statement, apart from names: its alpha lines are asserted
verbatim in `tests/nodes_shadow_filters.rs`. The cage (`m06`) and the room
(`m08`) differ from three only in §8's classes: var vs `let` and numbering,
varying order, and `NORMAL_normalView`.

## The page's details

* `performance.now()` is 0, so the first light sits at `( 0, 6, 0 )` with no
  rotation. The second is at `time = 10000`, rotated by 10000 rad about x
  and z.
* `generateTexture()` fills only the canvas' bottom row, so each 2 × 2
  `CanvasTexture` is half transparent black. With `flipY`, `repeat( 1, 4.5 )`
  and `NearestFilter` magnification, this makes the bands.
* The bulbs' colours are multiplied by 200 and clip to white with no tone
  mapping.
