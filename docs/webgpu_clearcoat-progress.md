# `webgpu_clearcoat`

Status: **green.** 4 of 100000 pixels against three.js 5f610f5's own
`test/e2e/image.js`, threshold 0.1%. Intel Iris Xe, Mesa 25.3.6, wgpu on
Vulkan. Steady frame 2.7 ms, 6 draw calls, 17857 triangles.

Four `MeshPhysicalMaterial` spheres with `clearcoat = 1` — car paint, carbon
fibre, a golf ball and a red metal — in front of the Pisa HDR cube, which is
both the background and (PMREM-filtered) `scene.environment`, lit by one
`PointLight` of intensity 30. Issue #171.

## Reconciling with the plans

There is no scout `PLAN.md` for this rung. It was ported from
`~/src/vendor/three.js/examples/webgpu_clearcoat.html` directly, with three's
WGSL dumped by `tools/dump-webgpu.mjs` into `target/dumps/webgpu_clearcoat/`
(uncommitted, per the rules) and diffed against `examples/dump_wgsl.rs`'s new
`clearcoat_car_paint`, `clearcoat_fibers` and `clearcoat_golf` sections (three's
`m10`, `m12`, `m14`; golf and the red sphere share `m14`).

Issue #171 said the clearcoat terms were "never accumulated". That was half
right: the indirect lobe, the AO multiply and `finish()`'s `Fcc` blend were
already ported for the barn lamp (§26), and only `direct()`'s branch was
missing. It also named `webgpu_loader_gltf_sheen` and `webgpu_loader_gltf` for
a regrade. Neither asset has a clearcoat (`SheenChair`, `DamagedHelmet`) and
neither page has a light, so the direct lobe cannot move them, and the ladder
confirms they did not move. The barn lamp has a clearcoat but also no light.

## What was added

| area | what |
| --- | --- |
| `src/materials/physical.rs` | `direct()`'s clearcoat branch; `brdf_ggx_on()`, `BRDF_GGX` with its `roughness` and `normalView` arguments |
| `src/nodes/tsl.rs` | `clearcoat_normal_view()` falls back to the `NORMAL` sub-build's `normalView`, not the normal-mapped one |
| `src/materials/mod.rs`, `node_material.rs` | `clearcoat_map`, `clearcoat_roughness_map`; the rest of the issue's field audit (`docs/api.md` §7) |
| `src/loaders/gltf_loader.rs` | `clearcoatTexture`, `clearcoatRoughnessTexture`; the derivative-tangent flip of `clearcoatNormalScale.y` |
| `src/addons/textures/` | `FlakesTexture` |
| `examples/` | `webgpu_clearcoat.rs`; `dump_wgsl` sections `clearcoat_car_paint`, `_fibers`, `_golf` |

`docs/nodes.md` §28 is the long form.

## What the pixels found

**The coat was as bumpy as the base.** With the direct lobe in, the frame was
at 141 pixels, all of them on the carbon-fibre sphere: its coat reflected the
weave and its highlight was broken into sparkles where three's is one clean
dot. `MaterialNode.CLEARCOAT_NORMAL` with no `clearcoatNormalMap` is
`normalView` *read inside the `NORMAL` sub-build* — the geometric normal,
`NORMAL_normalView` in the dump — and the port had been handing the coat the
outer, normal-mapped `normalView`. One line in `tsl::clearcoat_normal_view`:
141 px → 4 px. The barn lamp never took that branch, because its coat has a
normal map of its own.

**`FlakesTexture` is the page's canvas.** It draws 20000 values from the seeded
`Math.random()`, after the inspector's five (the flakes are built in the HDR
loader's callback, which runs after `init()` has built the inspector). A probe
of the live page counted the draws and read the canvas back; the port's image
matches every flake interior and differs only on antialiased rims (mean 0.7
levels in 255). None of that survives `normalScale = 0.15`.
