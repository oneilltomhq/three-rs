# `webgpu_lights_custom`

Status: **ported, not graded.** Three.js itself fails its own reference for
this page on this machine: three's frame differs from
`webgpu_lights_custom.jpg` in 416 of 100000 pixels (0.416%, over the 0.1%
limit). The port's full-resolution frame is pixel-identical to three's (0
differing pixels against `tools/dump-webgpu.mjs`' `actual_full.png`). The
e2e test is `#[ignore]`d with that reason, following the
`webgpu_instance_path` precedent. The page stays in the steady-frame ladder
(`rung!`).

Half a million `PointsNodeMaterial` points are lit by three `PointLight`s
through a user-defined `LightingModel`. Its `direct()` adds each light's
colour straight into `directDiffuse`. Each light carries a small sphere whose
`NodeMaterial` sets `lightsNode = lights()` and so ignores the scene lights.

## What was added

| area | what |
| --- | --- |
| `src/materials/lighting_model.rs` | the `LightingModel` trait (`start` / `direct` / `indirect` / `finish`, three's defaults), `LightingBuilder`, `ReflectedLight`, `DirectLightData`, and `LightsNode.setup()`'s tail |
| `src/materials/mod.rs`, `node_material.rs` | the material's `lighting_model`, read by the kinds with no model of their own, as `LightingContextNode.setup()` does |
| `examples/` | `webgpu_lights_custom.rs`; `dump_wgsl` sections `lights_custom_points` and `lights_custom_sphere` (three's `m05` and `m01`) |

`docs/nodes.md` §36 has the full design.

## What the pixels found

The frame is made of 1-pixel points, drawn with MSAA. On the Iris Xe (Mesa,
Vulkan), our frame and three's are the same, and both miss the reference by
416 pixels. Under SwiftShader, three's frame matches the reference at 0
pixels. So the reference records SwiftShader's rasterisation of 1-pixel
points, and no correct port can match it on this GPU. Loosening the grader
or copying anything from the screenshot is not allowed, so the rung stays
ungraded until the ladder runs on SwiftShader, or on a GPU that covers points
the same way SwiftShader does.
