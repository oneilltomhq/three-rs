# `webgpu_water`

Status: **ported, ungraded.** Three.js lists the page in its e2e exception
list (`'webgpu_water', // 1 min`), so it has no rung. The water's WGSL is
gated by `tests/nodes_water_wgsl.rs` and its frames by
`tests/water2_frames.rs`. Scored informally with
`three_rs::testing::compare` (three's `test/e2e/image.js` comparator,
threshold 0.1%), the port's first frame differs from
`examples/screenshots/webgpu_water.jpg` in 6 of 100000 pixels (0.006%). It
differs from a local dump of three.js 5f610f5's page in 0 pixels. Intel Iris
Xe, Mesa 25.3.6, wgpu on Vulkan. Steady frame 4.0 ms (release, the viewer
headless), 29 draw calls, 48 560 triangles.

"The Night Pool" (`models/gltf/pool.glb`, Draco with WebP textures) is
scaled to a tenth. A `Water2Mesh` fills its basin 0.2 above the floor, and
four dark grey planes surround it. `moonless_golf_2k.hdr.jpg` is both the
background and `scene.environment`. The frame goes through a
`RenderPipeline`: an MRT scene pass of `output` and `emissive`, then
`bloom( emissive, 2 )` added to the beauty, then `renderOutput()` (ACES
Filmic, exposure 0.5), then FXAA.

## Reconciling with the plans

There is no scout plan. The port follows
`~/src/vendor/three.js/examples/webgpu_water.html` and
`examples/jsm/objects/Water2Mesh.js`. `tools/dump-webgpu.mjs` dumped three's
WGSL into `target/dumps/webgpu_water/` (uncommitted). The water's two stages
(`m18`, `m19`) are committed verbatim in `tests/fixtures/webgpu_water/`. No
page builds the flow-map branch, so `tools/dump-pages/water2_flow_map.html`
builds one, and its dump's `m03` / `m04` are committed beside them. The
mirror is `webgpu_mirror`'s `reflector()`, with `webgpu_ocean`'s deferred
target add. The refraction is §61's screen read. Bloom and FXAA are the
existing display nodes.

## What was added

| area | what |
| --- | --- |
| `src/addons/objects/` | `Water2Mesh` and `Water2MeshOptions`: the mesh, its six uniforms as `SettableValue`s, and `WaterNode` as a `CustomNode` whose `updateBefore` runs `updateFlow( deltaTime )` |
| `src/loaders/gltf_loader.rs` | `transmissionTexture` into `transmission_map`; `occlusionTexture.strength` into `ao_map_intensity` |
| `src/materials/` | `transmission_map` (three's `transmissionMap`), whose red channel multiplies `transmission` |
| `examples/` | `webgpu_water.rs`; a `dump_wgsl` section `water2`; a viewer entry after Ocean |
| `tests/` | `nodes_water_wgsl.rs` against both branches' dumps; `water2_frames.rs`, which steps `flowConfig` through a wrap and a reset and checks the refraction and the tint on the GPU |
| `tools/dump-pages/` | `water2_flow_map.html` |

`docs/nodes.md` §83 is the long form.

## What the pixels found

- **The water was too dark.** Its mean colour was (18, 20, 19) against
  three's (62, 70, 63). Three's graded frame has no reflection: the
  reflector's first frame mirrors about `z = 0`, and three culls it. The
  water there is the tinted refraction of the pool's interior, and that
  interior was too dark. `pool.glb`'s `SPWallsFloorStairs` sets
  `occlusionTexture.strength` to 0, which three applies as
  `aoMapIntensity = 0`. The loader ignored the strength, so the AO map
  darkened the walls and floor. With the strength wired, the mean is
  (62, 70, 65) and the frame scores 0.006% against the screenshot.
- **The loader also needed `transmissionTexture`.** The `Clutter`
  material is `transmission: 1` times a texture whose red channel is 0
  almost everywhere, so only the light beams and circles transmit. Without
  the map, all of the clutter would transmit. The loader now reads it.

## Divergences that do not move a pixel

- `screenUV`, which the water reads three times, is one `nodeVarN` in the
  port and inline in three (§8). The WGSL gate inlines it.
- The parenthesised `flipX` and the output clamp as a `var`, as for
  `webgpu_ocean` (§76.2).
- The Ultra HDR environment loads synchronously before the first frame.
  Upstream's `load()` callback lands asynchronously, and three's screenshot
  shows it already applied.
- The graph and the mirror are built at construction. The mirror's target is
  still added at the first render, as upstream does.

## What is not ported

- `renderer.inspector` and its "Water" panel. Its four controls are the
  example's `set_color`, `set_scale`, `set_flow_x` and `set_flow_y`.
- Rebuilds, swappable `TextureNode` maps, optional normal maps, `color` as a
  CSS string, and `isWater` (§83.4).
