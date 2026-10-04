# `webgpu_ocean`

Status: **green.** 0 of 100000 pixels against three.js 5f610f5's own
`test/e2e/image.js`, threshold 0.1%. Intel Iris Xe, Mesa 25.3.6, wgpu on
Vulkan. Steady frame 4.5 ms (release), 19 draw calls, 75 triangles.

A `WaterMesh` on a 10 000² plane, a `SkyMesh` scaled to 10 000 with the sun
2° above the horizon, and a 30-unit `MeshStandardNodeMaterial` cube (roughness
0, metalness 0) lit only by `scene.environment`, a PMREM of the sky alone.
The frame goes through a `RenderPipeline`: the scene pass plus a faint bloom
(threshold 0, strength 0.1, radius 0), then ACES Filmic at exposure 0.1. Time
is pinned to 0, so the normal map has not scrolled and the cube sits at
`y = 5` unrotated.

## Reconciling with the plans

There is no scout plan. The rung was ported from
`~/src/vendor/three.js/examples/webgpu_ocean.html` and
`examples/jsm/objects/WaterMesh.js`. Three's WGSL was dumped by
`tools/dump-webgpu.mjs` into `target/dumps/webgpu_ocean/` (uncommitted). The
water's two stages (`m09`, `m10`) are committed verbatim as
`tests/fixtures/webgpu_ocean/` and gated by `tests/nodes_water_wgsl.rs`. The
sky is `webgpu_sky`'s `SkyMesh`, and the bloom is `webgpu_postprocessing_bloom`'s.
The mirror is `webgpu_mirror`'s `reflector()`.

## What was added

| area | what |
| --- | --- |
| `src/addons/objects/` | `WaterMesh` and `WaterMeshOptions`: the mesh and its six uniforms as `SettableValue`s, the material, and the inline `getNoise` `Fn()` |
| `src/nodes/reflector_node.rs`, `src/renderer/reflector.rs` | `ReflectorNode::add_target_on_setup()`, which adds the mirror's target to an object just before the reflector's first update |
| `examples/` | `webgpu_ocean.rs`; a `dump_wgsl` section `water` |
| `tests/` | `nodes_water_wgsl.rs` against three's `m09` / `m10`; the e2e rung, which also checks the scene's child order after `updateSun()` |

`docs/nodes.md` §76 is the long form.

## What the pixels found

- **32.7 %: the water stood upright.** `water.rotation.x = -π/2` did
  nothing, because the port's `Object3D::rotation` is not synced into the
  quaternion. The page now calls `set_rotation`, for the cube too.
- **19.9 %: the water was too bright.** The port's water reflected the
  sunset and the cube; three's graded frame shows dark water. Upstream adds
  the mirror's target to the mesh inside the `colorNode` `Fn()`, so the add
  happens during the first render, after `updateMatrixWorld()`. That frame's
  mirror therefore uses an identity world matrix, the plane `z = 0`. A
  scratch copy of three's page that rendered a second frame (after
  `nodeFrame.update()`) showed the bright water the port had drawn.
  `add_target_on_setup` reproduces upstream's timing, and the frame went to
  0 pixels.

## Divergences that do not move a pixel

- The WGSL differences in §76.2: the parenthesised `flipX`, and the output
  clamp as a `var` rather than a `let`.
- `SkyMesh.showSunDisc` is a `bool` uniform here and a `u32` in three's dump
  (§59.2).

## What is not ported

- `Water2Mesh` and `webgpu_water` are a separate port: `docs/webgpu_water-progress.md` and §83.
- `renderer.inspector` and its parameters panel draw nothing into the
  canvas. `Parameters` holds the GUI's values (elevation, azimuth, exposure),
  and `update_sun()` is its callback.
- `water.resolutionScale` is read at construction, not at first build
  (§76.1).
