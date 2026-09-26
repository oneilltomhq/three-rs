# `webgpu_loader_gltf_diffuse_roughness`

Status: **green.** 0 of 100000 pixels against three.js 5f610f5's own
`test/e2e/image.js`, threshold 0.1%. Intel Iris Xe, Mesa 25.3.6, wgpu on
Vulkan. Steady frame 8.6 ms (debug e2e ladder; not yet measured in release), 26 draw calls, 75713 triangles.

Khronos' `DiffuseRoughnessParameterSweep.glb` is a grid of spheres whose
`KHR_materials_diffuse_roughness.diffuseRoughnessFactor` runs from 0 to 1.
The page lights it with nothing but a PMREM of `RoomEnvironment`, which is
also the background at blurriness 0.5, and tone maps it with `NeutralToneMapping`.

## Reconciling with the plans

There is no scout `PLAN.md` for this rung. It was ported from
`~/src/vendor/three.js/examples/webgpu_loader_gltf_diffuse_roughness.html`
directly. Three's WGSL was dumped by `tools/dump-webgpu.mjs` into
`target/dumps/webgpu_loader_gltf_diffuse_roughness/` (not committed) and
diffed against `examples/dump_wgsl.rs`'s new `gltf_diffuse_roughness_on` and
`gltf_diffuse_roughness_zero` sections.

## What was added

| area | what |
| --- | --- |
| `src/materials/physical.rs` | `brdf_eon()`, `eon_directional_albedo()` and the `FON_*` terms, which is `BRDF_EON.js`; `Physical::diffuse_roughness` swaps them in at `direct()`, `indirect_diffuse()` and `indirect_specular()` |
| `src/materials/mod.rs`, `node_material.rs` | `diffuse_roughness`; `useDiffuseRoughness` and the `DiffuseRoughness` assignment in `setupVariants()` |
| `src/nodes/tsl.rs`, `node.rs`, `src/renderer/` | `material_diffuse_roughness()` (`UniformSource::MaterialDiffuseRoughness`), the `diffuse_roughness()` property |
| `src/loaders/gltf_loader.rs` | `KHR_materials_diffuse_roughness`'s factor, and promotion to physical |
| `src/core/buffer_geometry.rs` | `Index::set_x`, for the page's winding flip |
| `examples/` | `webgpu_loader_gltf_diffuse_roughness.rs`; `dump_wgsl`'s `dump_diffuse_roughness()` |

`docs/nodes.md` §54 is the long form.

## What the pixels found

**Nothing, which is itself the finding.** The page was at 0 pixels on the
first run, and it is also at 0 pixels with the EON lobe switched off. The
lobe moves the frame by up to 15 levels in 255 over about 73000 pixels,
and none of it crosses pixelmatch's 0.1 YIQ threshold. So the grader does
not check this rung's new code. Two other things do:

* The dump. `gltf_diffuse_roughness_on` is three's shader statement for
  statement, apart from the two cosmetic differences in §54.4.
* The error against three's own screenshot. Over the sphere grid, the mean
  absolute error against `expected.jpg` is 1.35 levels with EON and 3.60
  with Lambert. Over the whole frame it is 0.69 and 1.62.

## Not ported

* `diffuseRoughnessMap` (`diffuseRoughnessTexture` in the extension). The
  sweep does not use one.
