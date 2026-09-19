# `webgpu_loader_gltf_sheen`

Status: **green.** 3 of 100000 pixels against three.js r186's own
`test/e2e/image.js`, threshold 0.1%. Intel Iris Xe, Mesa 25.3.6, wgpu 30.0.1 on
Vulkan.

The page is §23's environment — the UltraHDR `royal_esplanade` map as both
`scene.background` (a 512² cube) and `scene.environment` (a PMREM) — with
SheenChair.glb in front of it and **no lights at all**: the page's one
`DirectionalLight` is commented out upstream. Everything visible is indirect.

## Grade first

Three's own frame for this page, captured through `tools/dump-webgpu.mjs`
(which pins the harness exactly as the grader does) and compared against
`examples/screenshots/webgpu_loader_gltf_sheen.jpg` with three's unmodified
`Image.compare( …, 0.1 )`:

| | pixels of 100000 |
| --- | --- |
| three.js r186 against its own reference JPEG | 3 (0.003%) |
| this port against the same JPEG | 3 (0.003%) |

The port is exactly at three's own number. That is the tightest result on the
ladder so far, and it is not luck: the asset is small, the frame is entirely
indirect lighting out of a PMREM the port already matched at
`webgpu_pmrem_equirectangular`, and there is no animation or randomness in the
page for the two engines to disagree about.

Unlike `dump-gltf`, this dump's image is sound — the model comes from the
checkout's own `examples/models/`, so the chair is in three's captured frame.

## Reconciling with the plans

No scout plan exists for this page. It was ported from
`~/src/vendor/three.js/examples/webgpu_loader_gltf_sheen.html` directly, against
three's WGSL dumped into `target/dumps/gltf_sheen/` (uncommitted, per the
rules). The fabric is `m09`/`m10`; `m11`–`m16` are the label, the wood and the
metal; `m03`/`m04` the background; `m05`–`m08` the PMREM; `m17`/`m18` the
output transform. Only `m10` is new work — the other six programs are shapes
§23 already landed.

## What was added

| area | what |
| --- | --- |
| `src/materials/physical.rs` | `D_Charlie` and `V_Neubelt` as real WGSL `fn`s; `BRDF_Sheen` and `IBLSheenBRDF` inlined, as three does; `Physical::sheen` and the five hook branches (`start` / `direct` / `indirect_diffuse` / `indirect_specular` / `ambient_occlusion`) plus `Physical::finish()` |
| `src/materials/mod.rs` | `Material::sheen` (`f64`, the `useSheen` switch); `Material::sheen` → `sheen_color` renamed to three's name |
| `src/materials/node_material.rs` | `setup_standard` emits `Sheen` / `SheenRoughness` when `sheen > 0` on a physical material, and calls `Physical::finish()` after `outgoingLight` |
| `src/nodes/tsl.rs` | `material_sheen()`, `material_sheen_color()`, `material_sheen_roughness()`; the `sheen` / `sheenRoughness` / `sheenSpecularDirect` / `sheenSpecularIndirect` properties; `uv1()`; `texture()` now resolves its default uv through the map's channel |
| `src/nodes/node.rs` | three `UniformSource` variants for the sheen uniforms, in the object group |
| `src/renderer/programs.rs`, `src/renderer/mod.rs` | their uniform values, read off the material per object |
| `src/textures/texture.rs` | `Texture::channel` / `set_channel` (`Texture.channel`, `uv` or `uv1`); `set_matrix` (`KHR_texture_transform`'s rotated case); `clone_texture` |
| `src/loaders/gltf_loader.rs` | `KHR_materials_sheen`; `KHR_texture_transform` and `texCoord` through one `assign_texture()` for every map; promotion to `MeshPhysicalNodeMaterial` on `ior` / `specular` / `sheen` |
| `examples/webgpu_loader_gltf_sheen.rs` | the port |
| `examples/dump_wgsl.rs` | a `loader_gltf_sheen_fabric` section — the fabric built by hand, with its transforms, its `uv1` occlusion map and its sheen factors |
| `tests/e2e/main.rs` | the graded test and the `rung!()` line |

`docs/nodes.md` §25 is the reference for all of it.

## What the pixels found

**Nothing.** The example passed the grader on its first GPU run, at 3 pixels.
What the *diff against the dump* found, before any pixels, was the work:

* **`sheen` is a float, and the float is the switch.**
  `MeshPhysicalNodeMaterial.useSheen` is `this.sheen > 0`, and `Sheen` is
  `sheenColor * sheen` **in the shader** (`m10`: `Sheen = ( object.nodeUniform14
  * vec3<f32>( object.nodeUniform15 ) )`). The port's `Material` had only a
  `sheen: Color`, which is three's `sheenColor` under three's *old* name; both
  are now present under three's current names. Folding the multiply into the
  uniform would have generated one uniform where three generates two, and would
  have broken the page's GUI slider.
* **The energy compensation in `indirectSpecular()` is one value applied
  twice.** Three builds `sheenEnergyComp` once and `mulAssign`s it onto
  `indirectSpecular` and `indirectDiffuse`; that requires both to be `toVar`s,
  which is only true in the sheen branch. Building it twice would have given
  the same pixels and a dump that no longer lines up.
* **The `iblIrradiance` sheen term comes *first* in `indirectSpecular()`**,
  before the four `*ScatteringDielectric/Metallic` accumulations — the only
  ordering in the whole model that is not where a reader would guess.
* **glTF's `T * R * S` is not `Texture.updateMatrix()`'s `T * S * R`.** The
  wood's two maps are rotated, so the transform cannot go through
  `updateMatrix()` at all. `Texture::set_matrix` writes it, exactly as three's
  `GLTFTextureTransformExtension` does with `matrixAutoUpdate = false`.
  §25.4.
* **A transform or a `texCoord` belongs to the texture *reference*, not the
  texture.** The same image is referenced by several materials with different
  transforms, so `assign_texture()` clones the cached `Texture` whenever either
  is present. Without the clone the last material parsed would have set the uv
  matrix for all of them.

None of these show as a small diff: each one is the whole chair or a whole map
in the wrong place.

## What was ruled out

* **Making `BRDF_Sheen` a WGSL `fn`.** `D_Charlie` and `V_Neubelt` carry
  `setLayout` in three and become functions; `BRDF_Sheen` and `IBLSheenBRDF` do
  not and are inlined at every call. The port follows the `setLayout` split
  rather than its own taste, so the dump stays comparable.
* **Skipping `brdf_sheen()` because this page cannot reach it.** With no lights
  the direct lobe is dead code here and no dump pins it. It is implemented from
  `BRDF_Sheen.js` anyway and its doc comment says so — a lighting model that is
  half-ported is a trap for the next sheen page, which will have a light.
* **Loosening `SheenRoughness`'s clamp.** `clamp( x, 0.0001, 1.0 )` is three's,
  and the lower bound is load-bearing: `D_Charlie`'s `invAlpha` is `1 / r²`.
* **`KHR_materials_variants`.** SheenChair carries the extension; the page never
  selects a variant, so the loader ignores it exactly as three's default does.
* **`scene.backgroundBlurriness`.** `//scene.backgroundBlurriness = 1; // @TODO:
  Needs PMREM` is commented out upstream. The skybox is the sharp cube.
* **Promoting every glTF material to physical.** Only the fabric is physical in
  three's output; the other three compile the standard material's
  `SpecularColor = vec3<f32>( 0.04, 0.04, 0.04 )`. Promotion is per extension.
  §25.6.

## What was left out

* **`Texture.channel` beyond 1.** `set_channel` asserts. glTF allows more uv
  sets; the node system would need `uv2`, `uv3` … and nothing on the ladder has
  them.
* **`KHR_texture_transform`'s interaction with `Texture.center`.** The port
  writes the glTF matrix directly for the rotated case, so `center` never
  participates. A page that animated a rotated glTF texture would need the real
  `matrixAutoUpdate = false` flag.
* **The GUI.** `renderer.inspector.createParameters( 'SheenChair_fabric' )`
  adds one `sheen` slider over `[ 0, 1 ]`. The graded frame is the untouched
  asset value (1), which is what the example renders.
* **`OrbitControls`.** `enableDamping` with no pointer events moves nothing, so
  the example calls `camera.look_at( 0, 0.35, 0 )` directly — the same
  emulation every earlier rung uses.
* **The sheen *direct* path's pixels.** Ported, unexercised. The first ladder
  page with a light and a sheen material grades it.

## Ladder after the change

Unchanged, all 43 e2e tests green: depth_texture 0, instance_mesh 60,
materials_basic 0, rtt 1, lights_phong 31, morphtargets 0, shadowmap 7,
lights_physical 4, postprocessing_masking 18, tsl_galaxy 40, loader_gltf 59,
mrt 87, custom_fog_background 57, and this page 3.
