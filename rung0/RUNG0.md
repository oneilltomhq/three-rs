# Rung 0 — grader calibration (2026-09-12)

Machine: Fedora 43, Intel Iris Xe (RPL-U), Mesa 25.3.6, Chrome 152.0.7977.54
via puppeteer 25, node 24.13. three.js @ 3d010ef, `npm ci` clean (npm-ci.log).

## What the stock grader does here

`npm run test-e2e-webgpu` as shipped renders every WebGPU example black
(e2e-run1.log, e2e-run2-visible.log). Two causes, both silent:

1. It forces `VK_DRIVER_FILES=…/lvp_icd.x86_64.json` (lavapipe). Dawn in
   Chrome 152 refuses that adapter and hands out SwiftShader instead, which
   then throws `OperationError: Instance dropped in popErrorScope` and
   presents nothing. Some examples notice and fall back to WebGL2, which is
   why four of them "passed" in run 1.
2. `--disable-vulkan-surface` blanks WebGPU canvas capture in headless
   Chrome 152 even on the real Intel adapter. A raw clear-to-red canvas
   probe (_probe5.mjs) is 0% non-black with the flag and 100% without.

Fix (grader-flags.patch, applied to ~/src/vendor/three.js, comparator
untouched): drop the lavapipe env override, replace `--disable-vulkan-surface`
with `--use-angle=vulkan`. Adapter is then `intel/gen-12lp`.

## Ladder results with the fixed grader (e2e-run3, repeated in run4)

| example | diff | verdict |
|---|---|---|
| webgpu_camera | 0.9% ×4 runs | DROP — sub-pixel coverage of the Points star field and wireframe lines (same class as upstream's webgpu_generator_city exclusion) |
| webgpu_instance_mesh | 0.1% (rounded, passes) | keep, thin margin |
| webgpu_materials_basic | 0.0% | pass |
| webgpu_rtt | 0.0% | pass |
| webgpu_lights_phong | 0.0% | pass |
| webgpu_morphtargets | 0.0% | pass |
| webgpu_shadowmap | 0.0% | pass |
| webgpu_lights_physical | 0.0% | pass |
| webgpu_materials | 0.0% | pass |
| webgpu_postprocessing | 0.1% ×4 runs, FAIL | DROP — sits on the 0.1% line; pick another postprocessing_* for rung 9 |
| webgpu_skinning | 0.0% | pass |
| webgpu_mesh_batch | 0.0% | pass |
| webgpu_compute_points | 0.0% | pass |
| webgpu_tsl_galaxy | 0.0% | pass |

## Rung 1 replacement

Graded 28 further examples (e2e-run5, e2e-run6). Chosen: **webgpu_depth_texture**
(0.0%, 99 lines, no lights, no addons beyond OrbitControls' initial lookAt).
Forces scene graph, PerspectiveCamera, Mesh, TorusKnotGeometry,
MeshBasicNodeMaterial as overrideMaterial, scene.background, deterministic
Math.random, RenderTarget + FloatType DepthTexture, QuadMesh, texture() node.

Other 0.0% passers on file for later reorders: webgpu_camera_array,
webgpu_clipping, webgpu_equirectangular, webgpu_instance_uniform,
webgpu_materials_lightmap, webgpu_multiple_elements, webgpu_occlusion,
webgpu_textures_2d-array, webgpu_procedural_texture, webgpu_cubemap_adjustments,
webgpu_geometry_loft, webgpu_instance_sprites, webgpu_lines_fat,
webgpu_materials_toon, webgpu_sprites.

## Rung 9 candidates (e2e-run7, graded 2026-09-12 22:30)

0.0% and eligible: webgpu_postprocessing_3dlut, _anamorphic, _bloom, _ca,
_direct, _masking, _transition. Marginal (0.1% rounded, pass): _fxaa, _sobel.
Fail: _afterimage 0.5%, _pixel 0.4%, _retro 1.5%. Director picks from the
0.0% set when rung 9 comes; _direct is the likely simplest (pass() + a
single display node), _bloom the most representative of later needs.
