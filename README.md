# three-rs

[![CI](https://github.com/oneilltomhq/three-rs/actions/workflows/ci.yml/badge.svg)](https://github.com/oneilltomhq/three-rs/actions/workflows/ci.yml)

A port of [three.js](https://github.com/mrdoob/three.js) core and its
`WebGPURenderer` to Rust on [wgpu](https://github.com/gfx-rs/wgpu), graded by
Three's own end-to-end pixel comparison: each ported example renders headless
and is diffed against the reference screenshot Three ships for that example,
using Three's unmodified `test/e2e/image.js` comparator.

Two things distinguish it from other three.js-shaped Rust crates (including
the similarly named `threers`): the shaders are not hand-written but generated
from Three's node graph (TSL) by a port of Three's `NodeBuilder`, the way
`WebGPURenderer` does it, so Three's own WGSL dumps are the reference; and
correctness is judged by Three's own examples and reference screenshots, not
by scenes written for the port.

**Status: early, working, incomplete.** The `webgpu_*` examples in the gallery
below pass the grader; the vast majority of Three's 600-odd examples have not been
attempted. The API follows Three's object model but is not stable. Vulkan on
Linux is the only backend that has been run.

## What is ported

- **Math and core.** Vector/Matrix/Quaternion/Euler/Color and the geometric
  helpers (Box, Sphere, Plane, Ray, Frustum, Triangle, ...), `Object3D` scene
  graph, `BufferGeometry` with named attributes, groups, draw ranges and morph
  attributes, layers, cameras. Verified against Three's QUnit tests.
- **Geometries.** Box, Plane, Cylinder, Cone, Torus, the polyhedra, Circle,
  Ring, Lathe, Capsule, plus the Teapot and RoundedBox addons, bit-exact
  against samples generated from Three.
- **Node system (TSL).** A `NodeBuilder` that generates both WGSL stages and the
  bind group layout from a node graph, following Three's `nodes/` and
  `renderers/webgpu/nodes/`. Generated WGSL is kept structurally identical to
  Three's dumps; deliberate divergences are listed in `docs/nodes.md`.
- **Materials.** `MeshBasic`, `MeshPhong`, `MeshStandard` (physical lighting
  model with DFG LUT), `Sprite`, `LineBasic`, and the shadow-pass material.
  Bump maps, env maps, blending modes, tone mapping.
- **Lights and shadows.** Ambient, Hemisphere, Point, Spot, Directional;
  planar and cube shadow maps with Three's Vogel-disk filter.
- **Renderer.** Render lists, instancing, morph targets, render targets,
  MSAA, the linear-to-sRGB output pass, `PassNode` post-processing, mipmaps,
  cube textures, line topology, viewport / scissor / `clearDepth` and
  `autoClear`.
- **Addons.** `src/addons/` holds the `three/addons/…` tier that the graded
  examples import: `lines` (`LineSegmentsGeometry`, `LineGeometry`,
  `LineSegments2`, `Line2` — fat lines, with `Line2NodeMaterial` in core beside
  them, as three.js ships it), `geometry_utils`, and `controls::OrbitControls`,
  a port of the JS class graded against the JS class itself. An addon that
  needs nothing from core would be a workspace crate instead — `addons/controls`
  is one — and that stays the preferred shape; these live in the root crate
  because the e2e harness pulls examples in with `#[path = "../../examples/…"]`,
  and an example in another crate would need its own test binary.
- **Loaders.** glTF/GLB (all accessor types, skins, animations, KHR specular
  and ior), textures (PNG, JPEG), cube textures.
- **Animation.** Interpolants, keyframe tracks, clips, `PropertyMixer`,
  `AnimationAction` and `AnimationMixer`.
- **Workspace crate.** `sdf-text`: signed-distance-field text rendering with
  a `BatchedText` object (ttf-parser outlines, analytic rasteriser), and the
  SDF text examples with their gates. Its `d33_treemap_labels` example lays
  its treemap out with [d3-hierarchy](https://github.com/oneilltomhq/d3-hierarchy),
  a port of d3-hierarchy 3.1.2 that began in this repository.

## Examples graded green

<!-- gallery:start -->
| | | | |
| --- | --- | --- | --- |
| [<img src="https://raw.githubusercontent.com/oneilltomhq/three-rs/main/docs/gallery/webgpu_depth_texture.jpg" alt="webgpu_depth_texture" width="200">](https://oneilltomhq.github.io/three-rs/?example=webgpu_depth_texture) | [<img src="https://raw.githubusercontent.com/oneilltomhq/three-rs/main/docs/gallery/webgpu_instance_mesh.jpg" alt="webgpu_instance_mesh" width="200">](https://oneilltomhq.github.io/three-rs/?example=webgpu_instance_mesh) | [<img src="https://raw.githubusercontent.com/oneilltomhq/three-rs/main/docs/gallery/webgpu_materials_basic.jpg" alt="webgpu_materials_basic" width="200">](https://oneilltomhq.github.io/three-rs/?example=webgpu_materials_basic) | [<img src="https://raw.githubusercontent.com/oneilltomhq/three-rs/main/docs/gallery/webgpu_rtt.jpg" alt="webgpu_rtt" width="200">](https://oneilltomhq.github.io/three-rs/?example=webgpu_rtt) |
| [`webgpu_depth_texture`](https://github.com/oneilltomhq/three-rs/blob/main/examples/webgpu_depth_texture.rs) | [`webgpu_instance_mesh`](https://github.com/oneilltomhq/three-rs/blob/main/examples/webgpu_instance_mesh.rs) | [`webgpu_materials_basic`](https://github.com/oneilltomhq/three-rs/blob/main/examples/webgpu_materials_basic.rs) | [`webgpu_rtt`](https://github.com/oneilltomhq/three-rs/blob/main/examples/webgpu_rtt.rs) |
| [<img src="https://raw.githubusercontent.com/oneilltomhq/three-rs/main/docs/gallery/webgpu_lights_phong.jpg" alt="webgpu_lights_phong" width="200">](https://oneilltomhq.github.io/three-rs/?example=webgpu_lights_phong) | [<img src="https://raw.githubusercontent.com/oneilltomhq/three-rs/main/docs/gallery/webgpu_morphtargets.jpg" alt="webgpu_morphtargets" width="200">](https://oneilltomhq.github.io/three-rs/?example=webgpu_morphtargets) | [<img src="https://raw.githubusercontent.com/oneilltomhq/three-rs/main/docs/gallery/webgpu_shadowmap.jpg" alt="webgpu_shadowmap" width="200">](https://oneilltomhq.github.io/three-rs/?example=webgpu_shadowmap) | [<img src="https://raw.githubusercontent.com/oneilltomhq/three-rs/main/docs/gallery/webgpu_lights_physical.jpg" alt="webgpu_lights_physical" width="200">](https://oneilltomhq.github.io/three-rs/?example=webgpu_lights_physical) |
| [`webgpu_lights_phong`](https://github.com/oneilltomhq/three-rs/blob/main/examples/webgpu_lights_phong.rs) · [notes](https://github.com/oneilltomhq/three-rs/blob/main/docs/rung5-progress.md) | [`webgpu_morphtargets`](https://github.com/oneilltomhq/three-rs/blob/main/examples/webgpu_morphtargets.rs) · [notes](https://github.com/oneilltomhq/three-rs/blob/main/docs/rung6-progress.md) | [`webgpu_shadowmap`](https://github.com/oneilltomhq/three-rs/blob/main/examples/webgpu_shadowmap.rs) · [notes](https://github.com/oneilltomhq/three-rs/blob/main/docs/rung7-progress.md) | [`webgpu_lights_physical`](https://github.com/oneilltomhq/three-rs/blob/main/examples/webgpu_lights_physical.rs) · [notes](https://github.com/oneilltomhq/three-rs/blob/main/docs/rung8-progress.md) |
| [<img src="https://raw.githubusercontent.com/oneilltomhq/three-rs/main/docs/gallery/webgpu_postprocessing_masking.jpg" alt="webgpu_postprocessing_masking" width="200">](https://oneilltomhq.github.io/three-rs/?example=webgpu_postprocessing_masking) | [<img src="https://raw.githubusercontent.com/oneilltomhq/three-rs/main/docs/gallery/webgpu_tsl_galaxy.jpg" alt="webgpu_tsl_galaxy" width="200">](https://oneilltomhq.github.io/three-rs/?example=webgpu_tsl_galaxy) | [<img src="https://raw.githubusercontent.com/oneilltomhq/three-rs/main/docs/gallery/webgpu_skinning.jpg" alt="webgpu_skinning" width="200">](https://oneilltomhq.github.io/three-rs/?example=webgpu_skinning) | [<img src="https://raw.githubusercontent.com/oneilltomhq/three-rs/main/docs/gallery/webgpu_mesh_batch.jpg" alt="webgpu_mesh_batch" width="200">](https://oneilltomhq.github.io/three-rs/?example=webgpu_mesh_batch) |
| [`webgpu_postprocessing_masking`](https://github.com/oneilltomhq/three-rs/blob/main/examples/webgpu_postprocessing_masking.rs) · [notes](https://github.com/oneilltomhq/three-rs/blob/main/docs/rung9-progress.md) | [`webgpu_tsl_galaxy`](https://github.com/oneilltomhq/three-rs/blob/main/examples/webgpu_tsl_galaxy.rs) · [notes](https://github.com/oneilltomhq/three-rs/blob/main/docs/rung13-progress.md) | [`webgpu_skinning`](https://github.com/oneilltomhq/three-rs/blob/main/examples/webgpu_skinning.rs) · [notes](https://github.com/oneilltomhq/three-rs/blob/main/docs/rung10-progress.md) | [`webgpu_mesh_batch`](https://github.com/oneilltomhq/three-rs/blob/main/examples/webgpu_mesh_batch.rs) · [notes](https://github.com/oneilltomhq/three-rs/blob/main/docs/rung11-progress.md) |
| [<img src="https://raw.githubusercontent.com/oneilltomhq/three-rs/main/docs/gallery/webgpu_postprocessing_radial_blur.jpg" alt="webgpu_postprocessing_radial_blur" width="200">](https://oneilltomhq.github.io/three-rs/?example=webgpu_postprocessing_radial_blur) | [<img src="https://raw.githubusercontent.com/oneilltomhq/three-rs/main/docs/gallery/webgpu_materials.jpg" alt="webgpu_materials" width="200">](https://oneilltomhq.github.io/three-rs/?example=webgpu_materials) | [<img src="https://raw.githubusercontent.com/oneilltomhq/three-rs/main/docs/gallery/webgpu_postprocessing_ssaa.jpg" alt="webgpu_postprocessing_ssaa" width="200">](https://oneilltomhq.github.io/three-rs/?example=webgpu_postprocessing_ssaa) | [<img src="https://raw.githubusercontent.com/oneilltomhq/three-rs/main/docs/gallery/webgpu_pmrem_cubemap.jpg" alt="webgpu_pmrem_cubemap" width="200">](https://oneilltomhq.github.io/three-rs/?example=webgpu_pmrem_cubemap) |
| [`webgpu_postprocessing_radial_blur`](https://github.com/oneilltomhq/three-rs/blob/main/examples/webgpu_postprocessing_radial_blur.rs) · [notes](https://github.com/oneilltomhq/three-rs/blob/main/docs/webgpu_postprocessing_radial_blur-progress.md) | [`webgpu_materials`](https://github.com/oneilltomhq/three-rs/blob/main/examples/webgpu_materials.rs) · [notes](https://github.com/oneilltomhq/three-rs/blob/main/docs/webgpu_materials-progress.md) | [`webgpu_postprocessing_ssaa`](https://github.com/oneilltomhq/three-rs/blob/main/examples/webgpu_postprocessing_ssaa.rs) · [notes](https://github.com/oneilltomhq/three-rs/blob/main/docs/webgpu_postprocessing_ssaa-progress.md) | [`webgpu_pmrem_cubemap`](https://github.com/oneilltomhq/three-rs/blob/main/examples/webgpu_pmrem_cubemap.rs) · [notes](https://github.com/oneilltomhq/three-rs/blob/main/docs/webgpu_pmrem_cubemap-progress.md) |
| [<img src="https://raw.githubusercontent.com/oneilltomhq/three-rs/main/docs/gallery/webgpu_postprocessing_bloom_selective.jpg" alt="webgpu_postprocessing_bloom_selective" width="200">](https://oneilltomhq.github.io/three-rs/?example=webgpu_postprocessing_bloom_selective) | [<img src="https://raw.githubusercontent.com/oneilltomhq/three-rs/main/docs/gallery/webgpu_compute_points.jpg" alt="webgpu_compute_points" width="200">](https://oneilltomhq.github.io/three-rs/?example=webgpu_compute_points) | [<img src="https://raw.githubusercontent.com/oneilltomhq/three-rs/main/docs/gallery/webgpu_lines_fat.jpg" alt="webgpu_lines_fat" width="200">](https://oneilltomhq.github.io/three-rs/?example=webgpu_lines_fat) | [<img src="https://raw.githubusercontent.com/oneilltomhq/three-rs/main/docs/gallery/webgpu_pmrem_test.jpg" alt="webgpu_pmrem_test" width="200">](https://oneilltomhq.github.io/three-rs/?example=webgpu_pmrem_test) |
| [`webgpu_postprocessing_bloom_selective`](https://github.com/oneilltomhq/three-rs/blob/main/examples/webgpu_postprocessing_bloom_selective.rs) · [notes](https://github.com/oneilltomhq/three-rs/blob/main/docs/webgpu_postprocessing_bloom_selective-progress.md) | [`webgpu_compute_points`](https://github.com/oneilltomhq/three-rs/blob/main/examples/webgpu_compute_points.rs) · [notes](https://github.com/oneilltomhq/three-rs/blob/main/docs/rung12-progress.md) | [`webgpu_lines_fat`](https://github.com/oneilltomhq/three-rs/blob/main/examples/webgpu_lines_fat.rs) · [notes](https://github.com/oneilltomhq/three-rs/blob/main/docs/webgpu_lines_fat-progress.md) | [`webgpu_pmrem_test`](https://github.com/oneilltomhq/three-rs/blob/main/examples/webgpu_pmrem_test.rs) · [notes](https://github.com/oneilltomhq/three-rs/blob/main/docs/webgpu_pmrem_test-progress.md) |
| [<img src="https://raw.githubusercontent.com/oneilltomhq/three-rs/main/docs/gallery/webgpu_postprocessing_difference.jpg" alt="webgpu_postprocessing_difference" width="200">](https://oneilltomhq.github.io/three-rs/?example=webgpu_postprocessing_difference) | [<img src="https://raw.githubusercontent.com/oneilltomhq/three-rs/main/docs/gallery/webgpu_postprocessing_direct.jpg" alt="webgpu_postprocessing_direct" width="200">](https://oneilltomhq.github.io/three-rs/?example=webgpu_postprocessing_direct) | [<img src="https://raw.githubusercontent.com/oneilltomhq/three-rs/main/docs/gallery/webgpu_furnace_test.jpg" alt="webgpu_furnace_test" width="200">](https://oneilltomhq.github.io/three-rs/?example=webgpu_furnace_test) | [<img src="https://raw.githubusercontent.com/oneilltomhq/three-rs/main/docs/gallery/webgpu_postprocessing_anamorphic.jpg" alt="webgpu_postprocessing_anamorphic" width="200">](https://oneilltomhq.github.io/three-rs/?example=webgpu_postprocessing_anamorphic) |
| [`webgpu_postprocessing_difference`](https://github.com/oneilltomhq/three-rs/blob/main/examples/webgpu_postprocessing_difference.rs) · [notes](https://github.com/oneilltomhq/three-rs/blob/main/docs/webgpu_postprocessing_difference-progress.md) | [`webgpu_postprocessing_direct`](https://github.com/oneilltomhq/three-rs/blob/main/examples/webgpu_postprocessing_direct.rs) · [notes](https://github.com/oneilltomhq/three-rs/blob/main/docs/webgpu_postprocessing_direct-progress.md) | [`webgpu_furnace_test`](https://github.com/oneilltomhq/three-rs/blob/main/examples/webgpu_furnace_test.rs) · [notes](https://github.com/oneilltomhq/three-rs/blob/main/docs/webgpu_furnace_test-progress.md) | [`webgpu_postprocessing_anamorphic`](https://github.com/oneilltomhq/three-rs/blob/main/examples/webgpu_postprocessing_anamorphic.rs) · [notes](https://github.com/oneilltomhq/three-rs/blob/main/docs/webgpu_postprocessing_anamorphic-progress.md) |
| [<img src="https://raw.githubusercontent.com/oneilltomhq/three-rs/main/docs/gallery/webgpu_pmrem_scene.jpg" alt="webgpu_pmrem_scene" width="200">](https://oneilltomhq.github.io/three-rs/?example=webgpu_pmrem_scene) | [<img src="https://raw.githubusercontent.com/oneilltomhq/three-rs/main/docs/gallery/webgpu_postprocessing_bloom.jpg" alt="webgpu_postprocessing_bloom" width="200">](https://oneilltomhq.github.io/three-rs/?example=webgpu_postprocessing_bloom) | [<img src="https://raw.githubusercontent.com/oneilltomhq/three-rs/main/docs/gallery/webgpu_materials_envmaps.jpg" alt="webgpu_materials_envmaps" width="200">](https://oneilltomhq.github.io/three-rs/?example=webgpu_materials_envmaps) | [<img src="https://raw.githubusercontent.com/oneilltomhq/three-rs/main/docs/gallery/webgpu_materials_cubemap_mipmaps.jpg" alt="webgpu_materials_cubemap_mipmaps" width="200">](https://oneilltomhq.github.io/three-rs/?example=webgpu_materials_cubemap_mipmaps) |
| [`webgpu_pmrem_scene`](https://github.com/oneilltomhq/three-rs/blob/main/examples/webgpu_pmrem_scene.rs) · [notes](https://github.com/oneilltomhq/three-rs/blob/main/docs/webgpu_pmrem_scene-progress.md) | [`webgpu_postprocessing_bloom`](https://github.com/oneilltomhq/three-rs/blob/main/examples/webgpu_postprocessing_bloom.rs) · [notes](https://github.com/oneilltomhq/three-rs/blob/main/docs/webgpu_postprocessing_bloom-progress.md) | [`webgpu_materials_envmaps`](https://github.com/oneilltomhq/three-rs/blob/main/examples/webgpu_materials_envmaps.rs) · [notes](https://github.com/oneilltomhq/three-rs/blob/main/docs/webgpu_materials_envmaps-progress.md) | [`webgpu_materials_cubemap_mipmaps`](https://github.com/oneilltomhq/three-rs/blob/main/examples/webgpu_materials_cubemap_mipmaps.rs) · [notes](https://github.com/oneilltomhq/three-rs/blob/main/docs/webgpu_materials_envmaps-progress.md) |
| [<img src="https://raw.githubusercontent.com/oneilltomhq/three-rs/main/docs/gallery/webgpu_postprocessing_bloom_emissive.jpg" alt="webgpu_postprocessing_bloom_emissive" width="200">](https://oneilltomhq.github.io/three-rs/?example=webgpu_postprocessing_bloom_emissive) | [<img src="https://raw.githubusercontent.com/oneilltomhq/three-rs/main/docs/gallery/webgpu_instance_uniform.jpg" alt="webgpu_instance_uniform" width="200">](https://oneilltomhq.github.io/three-rs/?example=webgpu_instance_uniform) | [<img src="https://raw.githubusercontent.com/oneilltomhq/three-rs/main/docs/gallery/webgpu_tsl_interoperability.jpg" alt="webgpu_tsl_interoperability" width="200">](https://oneilltomhq.github.io/three-rs/?example=webgpu_tsl_interoperability) | [<img src="https://raw.githubusercontent.com/oneilltomhq/three-rs/main/docs/gallery/webgpu_pmrem_equirectangular.jpg" alt="webgpu_pmrem_equirectangular" width="200">](https://oneilltomhq.github.io/three-rs/?example=webgpu_pmrem_equirectangular) |
| [`webgpu_postprocessing_bloom_emissive`](https://github.com/oneilltomhq/three-rs/blob/main/examples/webgpu_postprocessing_bloom_emissive.rs) · [notes](https://github.com/oneilltomhq/three-rs/blob/main/docs/webgpu_postprocessing_bloom_emissive-progress.md) | [`webgpu_instance_uniform`](https://github.com/oneilltomhq/three-rs/blob/main/examples/webgpu_instance_uniform.rs) · [notes](https://github.com/oneilltomhq/three-rs/blob/main/docs/webgpu_instance_uniform-progress.md) | [`webgpu_tsl_interoperability`](https://github.com/oneilltomhq/three-rs/blob/main/examples/webgpu_tsl_interoperability.rs) · [notes](https://github.com/oneilltomhq/three-rs/blob/main/docs/webgpu_tsl_interoperability-progress.md) | [`webgpu_pmrem_equirectangular`](https://github.com/oneilltomhq/three-rs/blob/main/examples/webgpu_pmrem_equirectangular.rs) · [notes](https://github.com/oneilltomhq/three-rs/blob/main/docs/webgpu_pmrem_equirectangular-progress.md) |
| [<img src="https://raw.githubusercontent.com/oneilltomhq/three-rs/main/docs/gallery/webgpu_postprocessing_ca.jpg" alt="webgpu_postprocessing_ca" width="200">](https://oneilltomhq.github.io/three-rs/?example=webgpu_postprocessing_ca) | [<img src="https://raw.githubusercontent.com/oneilltomhq/three-rs/main/docs/gallery/webgpu_loader_gltf.jpg" alt="webgpu_loader_gltf" width="200">](https://oneilltomhq.github.io/three-rs/?example=webgpu_loader_gltf) | [<img src="https://raw.githubusercontent.com/oneilltomhq/three-rs/main/docs/gallery/webgpu_mrt.jpg" alt="webgpu_mrt" width="200">](https://oneilltomhq.github.io/three-rs/?example=webgpu_mrt) | [<img src="https://raw.githubusercontent.com/oneilltomhq/three-rs/main/docs/gallery/webgpu_custom_fog_background.jpg" alt="webgpu_custom_fog_background" width="200">](https://oneilltomhq.github.io/three-rs/?example=webgpu_custom_fog_background) |
| [`webgpu_postprocessing_ca`](https://github.com/oneilltomhq/three-rs/blob/main/examples/webgpu_postprocessing_ca.rs) · [notes](https://github.com/oneilltomhq/three-rs/blob/main/docs/webgpu_postprocessing_ca-progress.md) | [`webgpu_loader_gltf`](https://github.com/oneilltomhq/three-rs/blob/main/examples/webgpu_loader_gltf.rs) · [notes](https://github.com/oneilltomhq/three-rs/blob/main/docs/webgpu_loader_gltf-progress.md) | [`webgpu_mrt`](https://github.com/oneilltomhq/three-rs/blob/main/examples/webgpu_mrt.rs) · [notes](https://github.com/oneilltomhq/three-rs/blob/main/docs/webgpu_loader_gltf-progress.md) | [`webgpu_custom_fog_background`](https://github.com/oneilltomhq/three-rs/blob/main/examples/webgpu_custom_fog_background.rs) · [notes](https://github.com/oneilltomhq/three-rs/blob/main/docs/webgpu_custom_fog_background-progress.md) |
| [<img src="https://raw.githubusercontent.com/oneilltomhq/three-rs/main/docs/gallery/webgpu_loader_gltf_sheen.jpg" alt="webgpu_loader_gltf_sheen" width="200">](https://oneilltomhq.github.io/three-rs/?example=webgpu_loader_gltf_sheen) | [<img src="https://raw.githubusercontent.com/oneilltomhq/three-rs/main/docs/gallery/webgpu_deferred.jpg" alt="webgpu_deferred" width="200">](https://oneilltomhq.github.io/three-rs/?example=webgpu_deferred) | [<img src="https://raw.githubusercontent.com/oneilltomhq/three-rs/main/docs/gallery/webgpu_loader_gltf_anisotropy.jpg" alt="webgpu_loader_gltf_anisotropy" width="200">](https://oneilltomhq.github.io/three-rs/?example=webgpu_loader_gltf_anisotropy) | [<img src="https://raw.githubusercontent.com/oneilltomhq/three-rs/main/docs/gallery/webgpu_materials_texture_manualmipmap.jpg" alt="webgpu_materials_texture_manualmipmap" width="200">](https://oneilltomhq.github.io/three-rs/?example=webgpu_materials_texture_manualmipmap) |
| [`webgpu_loader_gltf_sheen`](https://github.com/oneilltomhq/three-rs/blob/main/examples/webgpu_loader_gltf_sheen.rs) · [notes](https://github.com/oneilltomhq/three-rs/blob/main/docs/webgpu_loader_gltf_sheen-progress.md) | [`webgpu_deferred`](https://github.com/oneilltomhq/three-rs/blob/main/examples/webgpu_deferred.rs) · [notes](https://github.com/oneilltomhq/three-rs/blob/main/docs/webgpu_deferred-progress.md) | [`webgpu_loader_gltf_anisotropy`](https://github.com/oneilltomhq/three-rs/blob/main/examples/webgpu_loader_gltf_anisotropy.rs) · [notes](https://github.com/oneilltomhq/three-rs/blob/main/docs/webgpu_loader_gltf_anisotropy-progress.md) | [`webgpu_materials_texture_manualmipmap`](https://github.com/oneilltomhq/three-rs/blob/main/examples/webgpu_materials_texture_manualmipmap.rs) · [notes](https://github.com/oneilltomhq/three-rs/blob/main/docs/webgpu_materials_texture_manualmipmap-progress.md) |

<sub>Our own rendered frames, one per graded example. Each thumbnail opens the example running in your browser on WebGPU ([all of them](https://oneilltomhq.github.io/three-rs/)); the caption links the ported source. See [`docs/gallery.md`](https://github.com/oneilltomhq/three-rs/blob/main/docs/gallery.md).</sub>
<!-- gallery:end -->

Every one of these also runs in a browser, on the browser's own WebGPU, from
the same `init()`: the graded frame first, then the page's own animation loop,
with drag to orbit, wheel to dolly and right-drag to pan. See
[`web/README.md`](https://github.com/oneilltomhq/three-rs/blob/main/web/README.md)
for how to build and serve the shell, and [issue #128](https://github.com/oneilltomhq/three-rs/issues/128)
for where that is going. The table's `browser` column is CI's `web-gate` job:
the same example in headless Chrome on software WebGPU, graded against the
same screenshots at the same threshold (see
[Grading in the browser](https://github.com/oneilltomhq/three-rs/blob/main/web/README.md#grading-in-the-browser)).

| Three example | different pixels (of 100000) | steady frame (ms) | draw calls | triangles | browser |
|---|---|---|---|---|---|
| webgpu_depth_texture | 0 | 11.0 | 43 | 671746 | yes |
| webgpu_instance_mesh | 60 (Three itself scores 60 against the same JPEG) | 9.3 | 2 | 967001 | yes |
| webgpu_materials_basic | 0 | 16.1 | 118 | 113345 | yes |
| webgpu_rtt | 1 | 2.3 | 3 | 14 | yes |
| webgpu_lights_phong | 31 | 4.3 | 5 | 62001 | yes |
| webgpu_morphtargets | 0 | 2.8 | 2 | 12289 | yes |
| webgpu_shadowmap | 7 | 7.7 | 19 | 39399 | yes |
| webgpu_lights_physical | 4 | 4.2 | 11 | 4267 | yes |
| webgpu_postprocessing_masking | 18 | 1.0 | 3 | 1037 | yes |
| webgpu_tsl_galaxy | 40 | 5.3 | 2 | 40001 | yes |
| webgpu_skinning | 6 | 3.0 | 3 | 30091 | yes |
| webgpu_mesh_batch | 0 | 2.5 | 454 | 44341 | yes |
| webgpu_postprocessing_radial_blur | 7 | 3.9 | 2 | 401 | yes |
| webgpu_materials | 44 | 7.3 | 19 | 350065 | yes |
| webgpu_postprocessing_ssaa | 0 | 11.9 | 17 | 2119689 | yes |
| webgpu_pmrem_cubemap | 31 | 8.2 | 32 | 243905 | yes |
| webgpu_postprocessing_bloom_selective | 1 | 16.5 | 63 | 256013 | yes |
| webgpu_compute_points | 4 (see below) | 10.7 | 2 | 1 + 300000 points | yes |
| webgpu_lines_fat | 0 | 3.8 | 6 | 11191 | yes |
| webgpu_pmrem_test | 0 | 6.1 | 35 | 67457 | yes |
| webgpu_postprocessing_difference | 13 | 1.5 | 2 | 13 | yes |
| webgpu_postprocessing_direct | 21 | 4.9 | 93 | 4192 | yes |
| webgpu_furnace_test | 0 | 7.1 | 122 | 116161 | yes |
| webgpu_postprocessing_anamorphic | 2 | 8.1 | 16 | 398798 | yes |
| webgpu_pmrem_scene | 0 | 3.8 | 9 | 58433 | yes |
| webgpu_postprocessing_bloom | 0 | 8.6 | 19 | 52085 | yes |
| webgpu_materials_envmaps | 0 | 2.1 | 3 | 7105 | yes |
| webgpu_materials_cubemap_mipmaps | 1 | 2.6 | 3 | 65025 | yes |
| webgpu_postprocessing_bloom_emissive | 0 | 5.1 | 15 | 17449 | yes |
| webgpu_instance_uniform | 13 | 6.0 | 14 | 247105 | yes |
| webgpu_tsl_interoperability | 0 | 2.2 | 2 | 4 | yes |
| webgpu_pmrem_equirectangular | 0 | 5.9 | 32 | 243905 | yes |
| webgpu_postprocessing_ca | 0 | 3.4 | 23 | 5658 (+ 42 lines) | yes |
| webgpu_loader_gltf | 22 | 1.9 | 3 | 17437 | yes |
| webgpu_mrt | 10 | 2.8 | 3 | 17437 | yes |
| webgpu_custom_fog_background | 7 | 2.0 | 2 | 15453 | yes |
| webgpu_loader_gltf_sheen | 28 | 3.1 | 6 | 41921 | yes |
| webgpu_deferred | 0 | 2.8 | 25 | 26378 | yes |
| webgpu_loader_gltf_anisotropy | 94 | 3.2 | 5 | 12188 | yes |
| webgpu_materials_texture_manualmipmap | 81 | 2.5 | 11 | 19 | yes |

`webgpu_compute_points` is graded like the rest and its 4 pixels mean less
than the rest: its frame is black apart from a 2x2 block at the centre, so
Three's comparator would pass it at 0.0% even if the compute passes never ran.
That rung is gated on the WGSL its kernels compile to and on reading the
storage buffers back — `docs/rung12-progress.md` says why and what the tests
assert.

Measured on Intel Iris Xe, Mesa 25.3.6, Fedora 43, against three.js 5f610f5
(past r186, for the cube PMREM of 2f80402; the pin becomes the r187 tag once
upstream tags it).
Other GPUs and drivers will land somewhere else on the pass threshold; the
threshold is Three's own (0.1% of pixels).

The steady frame is the whole cost of a frame once the first has built and
uploaded everything — the example's `animate()` through to the GPU finishing
it, at 800x500, mean of 30 frames after a 10-frame warm-up in a release build
(`viewer <example> --headless --frames 40`, below). The same machine caveat
applies. The e2e grader renders three more frames after the graded one and
fails a rung whose steady frame is over a ceiling set well above these
(`STEADY_FRAME_CEILING` in `tests/e2e/main.rs`), so a per-frame cost the single
graded frame cannot see fails the ladder.

The last two columns are `renderer.info()` for the whole graded frame — draw
calls and the triangles they drew, instancing multiplied in (`webgpu_rtt`'s 14
is a cube and two full-screen quads; `webgpu_instance_mesh`'s million is one
instanced draw). Unlike the time, these are exact: the same grader asserts
that frames two and three of every rung compile, build and upload *nothing*,
as an equality, which is what catches a regression that re-uploads a live
geometry every frame while staying well under a time ceiling. `Info` has the
rest of the counts (pipelines, geometries, attribute buffers, textures, and
the resident totals).

## Building

(For how to contribute, what is in scope, and the gates a change has to pass,
see `CONTRIBUTING.md`.)

Requires a Rust toolchain (1.90 or newer) and a Vulkan driver. `wgpu` comes
from crates.io, pinned to `30.0.1` in `Cargo.toml`; nothing else is unusual.

```sh
cargo build --release
cargo test -p sdf-text --lib                # sdf-text unit tests; no GPU
cargo test -p three-rs --lib                # three-rs unit tests; no GPU
```

The rest of `cargo test --workspace` needs a GPU (the renderer tests and the
SDF text gates) and, for the e2e grader, the three.js checkout described
next. The e2e tests serialise themselves on the one GPU; no `--test-threads`
flag is needed for them, but `sdf-text`'s three GPU gates still want
`-- --test-threads=1`.

### The viewer

```sh
cargo run --release --bin viewer -- --list
cargo run --release --bin viewer -- webgpu_lights_physical
cargo run --release --bin viewer -- 8                        # the same, by key
cargo run --release --bin viewer -- shadowmap --headless --frames 40
```

Opens the named example in a window (winit, tested on Wayland). All 39 graded
examples are there, and each one animates, orbits, dollies and pans through
its *own* `animate()`, `resize()` and `OrbitControls` — the viewer drives the
example, it does not restate it. `--list` prints the examples with their keys,
and a key stands in for the name on the command line; in the window, `[` and
`]` step to the previous and next example, because 39 of them do not fit in
the 36 single keys a keyboard has. The window prints one line a second with
the frame rate and the steady-state render time (mean and max over the last
60 frames, after a 10-frame warm-up):

```text
webgpu_lights_phong — 1000x625 — 59.9 fps — render mean 1.61 ms max 1.79 ms (last 60 frames, after 10 warm-up)
```

`--headless --frames N` renders N frames with no window and reports the same
numbers for the whole frame, CPU and GPU, which is how the table above was
measured; add `--screenshot out.png` to keep the last frame. It also writes
`target/e2e/<example>/strip.png`: three more frames tiled left to right at
1:1, each with its draw calls and its build counts in the gutter under it, so
the count ladder sits beside the time one. The e2e harness writes the same
strip per rung (`steady-strip.png`) and one for the geometry-mutation case.

### Addons

three.js keeps its controls, loaders and helpers out of core, in
`examples/jsm/` — exported as `three/addons/*`, importing from `three` like any
user, and without core's stability promise. `addons/` is the same tier here:
workspace crates that depend on `three-rs` and are not ports of anything in
three.js' `src/`.

`three_rs::addons::controls::OrbitControls` is the exception to that rule, and
it is in the root crate rather than a workspace one: 27 of the 39 graded pages
create an `OrbitControls`, and an example pulled in by `#[path]` cannot reach a
crate that depends on `three-rs`. It is a port of
`examples/jsm/controls/OrbitControls.js` — the same state, the same defaults,
the same `update()` — but input-agnostic: it takes `pointer_down`,
`pointer_move`, `pointer_up`, `wheel` and `key` as plain values, so the same
controls serve a winit window, a DOM canvas and a test. It is graded the way
everything else here is, against the original: `tools/orbit_controls_reference.mjs`
runs the JS class under node over twelve scripted scenarios and prints the
camera it ends with, and `tests/addons_orbit_controls.rs` replays the same
scenarios and asserts every number to 1e-9.

`addons/controls/` is a different addon: `three-rs-controls`, a `MapControls` in the spirit of
three.js' addon of that name, over a `Ground` — a sphere whose north pole is the
world origin, so `R = 1e7` is a plane and a small R a planet, with no separate
case. The orbit target is a point *on* the ground, the camera never rolls —
`up` is the ground normal — and every field is damped with `smooth_damp`, a port
of camera-controls' `smoothDamp`.

`cargo run --release -p three-rs-controls --bin heli` is the demo, with its key
map on screen: **left-drag** grabs the ground,
**right-drag** (or **ctrl**-drag) orbits, the **wheel** zooms to the pointer,
the **arrows** pan, **[** **]** curl the ground and **P** flattens it, **Home**
resets, **Tab** is the overview (click a pane to drop onto it), **Esc** quits.

```sh
cargo run --release -p three-rs-controls --bin heli -- \
    --headless addons/controls/shots/heli-sphere-high.png \
    --radius 300 --pose 0,0,800,0,20 [--overview] [--size 1600x1000] [--no-legend]
```

### Running the examples and the e2e grader

The examples load their textures and models from Three's own `examples/`
directory, and the grader reads Three's reference screenshots and runs its
comparator under node. So they need a three.js checkout:

```sh
git clone https://github.com/mrdoob/three.js ~/src/vendor/three.js
git -C ~/src/vendor/three.js checkout 5f610f5   # the r187 tag, once upstream tags it
(cd ~/src/vendor/three.js && npm ci)
export THREE_JS_DIR=~/src/vendor/three.js     # this is the default location
cargo test --test e2e -- --nocapture
```

Each e2e test writes `actual.png`, the reference, and a diff strip under
`target/e2e/<example>/`. The grader is unmodified; `rung0/grader-flags.patch`
is only needed to run Three's *own* Chrome-based e2e suite on Linux with a
real Vulkan adapter, which is how the reference numbers were calibrated
(`rung0/RUNG0.md`).

One more checkout is optional:

| env var | default | needed by |
|---|---|---|
| `D3_GALLERY_DIR` | `~/src/vendor/d3-gallery` | `sdf-text`'s `d33_treemap_labels` example (its `flare.json`) |

Tests that need a checkout that is missing fail on the open with the path
they looked for.

## Layout

```
src/            the three-rs crate, mirroring three.js's src/ tree
  math core cameras geometries lights loaders materials objects textures animation
  nodes/        TSL nodes and the WGSL NodeBuilder
  renderer/     the wgpu backend: pipelines, bindings, passes, shadows, present
  bin/viewer.rs
examples/       one file per ported Three example, also compiled into tests/e2e
tests/          Three's QUnit tests ported per module, plus the e2e harness
sdf-text/       workspace crate: SDF text and BatchedText, with its examples and gates
addons/         workspace crates in the role of three.js' examples/jsm: not ports
  controls/     three-rs-controls — the map camera, its ground, and the heli demo
docs/           design notes per subsystem and per-example progress logs
rung0/          how the grader was calibrated
tools/          dump-webgpu.mjs, the rung dump hook (see docs/dumping.md)
```

`docs/nodes.md` and `docs/scene-graph.md` are the two to read first: how the
node system maps onto Three's, and how the `Rc<RefCell<Object3D>>` scene graph
replaces Three's prototype tree. `docs/dumping.md` says how a rung captures
the WGSL and GPU descriptors it ports against.

## How it was built

Example by example. Each "rung" takes one Three example, dumps the WGSL Three
generates for it (`tools/dump-webgpu.mjs`; see `docs/dumping.md`), ports whatever the example needs until the generated WGSL
matches and the grader passes, then merges. The port was directed and largely
written by Claude agents with the pixel diff as the ground truth; the
per-rung progress notes in `docs/` record what each rung found, including the
places where Three's own output was reproduced bug-for-bug and the places
where it deliberately was not.

## License

MIT. See `LICENSE`, which also carries the three.js (MIT) notice this port
derives from.
