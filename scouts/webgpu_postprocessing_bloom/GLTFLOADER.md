# GLTFLoader as a capability (2026-09-19)

Companion to `PLAN.md` in this directory. What the port's glTF loader has, what it has not, and
which of Three's examples each missing piece is standing in front of. All counts are from r186's
`examples/` directory and `test/e2e/puppeteer.js`'s `exceptionList`, measured today; the scripts that
produced them are inlined below so they can be re-run.

---

## 1. The numbers

```
230   webgpu_*.html examples in r186
 59   of them import examples/jsm/loaders/GLTFLoader.js          (26%)
 12   of those 59 are on the grader's exceptionList              → not rungs, ever
 47   eligible: import GLTFLoader and are gradeable here
```

Of the 47 eligible:

| | count |
|---|---|
| need an environment loader (`HDRLoader` / `UltraHDRLoader` / `HDRCubeTextureLoader` / `RoomEnvironment` / `scene.environment`) | 24 |
| need **no** environment | 23 |
| blocked by a compression/packing extension (Draco, meshopt, KTX2/basisu, WebP, mesh quantization, GPU instancing) | 10 |
| **reachable with the core loader only** (no such extension found in their assets) | **37** |
| …of those, needing no environment either | **20** |

Caveat on the "37": twelve of the 47 build their model path with a template string
(`models/gltf/${name}.glb`) and their assets could not be resolved statically — `webgpu_compute_water`,
`webgpu_cubemap_adjustments`, `webgpu_custom_fog_background`, `webgpu_loader_gltf`,
`webgpu_loader_gltf_anisotropy`, `webgpu_loader_gltf_iridescence`, `webgpu_loader_gltf_sheen`,
`webgpu_loader_gltf_transmission`, `webgpu_mrt`, `webgpu_performance`,
`webgpu_postprocessing_bloom_emissive`, `webgpu_tonemapping`. All twelve need an environment anyway,
so none of them is a near-term target.

The twelve on the exception list, for the record: `webgpu_compute_rasterizer_ibl`,
`webgpu_cubemap_mix`, `webgpu_lightprobes_sponza`, `webgpu_materials_matcap`,
`webgpu_morphtargets_face`, `webgpu_postprocessing_ao`, `webgpu_postprocessing_ssr_denoise`,
`webgpu_postprocessing_sss`, `webgpu_shadowmap_progressive`, `webgpu_tsl_graph`,
`webgpu_vxgi_sponza`, `webgpu_water`.

Feature demand across the eligible examples' resolvable assets (an example counts once even if two of
its models use the feature):

```
 24  images (PNG/JPEG in the file)      12  KHR_materials_ior
 24  doubleSided                        11  KHR_materials_specular
 19  animations                          7  KHR_draco_mesh_compression
 17  skins                               4  KHR_materials_transmission
  6  TANGENT attribute                   3  KHR_materials_volume
  5  occlusionTexture                    2  KHR_mesh_quantization
  5  alphaMode: BLEND                    2  KHR_materials_unlit
  5  emissiveFactor ≠ 0                  2  EXT_texture_webp
  3  TEXCOORD_n, n > 0                   2  KHR_materials_variants
  3  morph targets                       1  KHR_texture_transform
  3  COLOR_0                             1  KHR_texture_basisu
  3  alphaMode: MASK                     1  EXT_meshopt_compression
  3  emissiveTexture                     1  KHR_materials_dispersion
  1  multi-primitive mesh                1  EXT_mesh_gpu_instancing
```

Two readings of that table. First, `KHR_materials_ior` + `KHR_materials_specular` lead the
extensions — and rung 10 already ported both, because Michelle uses them. Second, everything above
the extensions is *core loader*: images, doubleSided, animations, skins. The port has the skins and
the animations; the gap is in the ordinary, boring half.

Reproduce with:

```sh
cd ~/src/vendor/three.js/examples
grep -l GLTFLoader webgpu_*.html | wc -l
```

and the two python scripts recorded in this scout's session (per-example asset inspection walks each
`.glb`'s JSON chunk for `extensionsUsed`, `skins`, `images`, `primitives[].mode|targets|attributes`
and `materials[].alphaMode|doubleSided|emissiveFactor|occlusionTexture`).

---

## 2. What the port has today

`src/loaders/gltf_loader.rs`, 1 502 lines, landed by rung 10 (`docs/gltf-progress.md`,
`docs/rung10-progress.md`) and gated by `tests/gltf_loader.rs` — 6 tests checked to 1e-6 against
three.js' own `GLTFLoader` run under node.

| `GLTFLoader.js` piece | state |
|---|---|
| GLB container (header / JSON / BIN chunks), `.gltf` JSON, `data:` URIs incl. base64 | **done** |
| `loadBuffer` / `loadBufferView` / `getDependency` caching | **done** |
| `loadAccessor` — every component type, `normalized`, `byteStride` (interleaved), `sparse` | **done** |
| `loadGeometries` → `BufferGeometry`, the `ATTRIBUTES` rename table, index (u16 when it fits) | **done** |
| morph targets, `morphTargetsRelative = true`, `meshDef.extras.targetNames` | **done** |
| `loadNode` — the tree, `matrix` vs TRS, `createUniqueName`, `sanitizeNodeName` | **done** |
| `loadScene`, `gltf.scenes`, `gltf.scene` | **done** |
| `loadSkin` → `Skeleton` (`inverseBindMatrices`, joints as `Bone`s), `mesh.bind(skeleton, identity)` | **done** |
| `loadAnimation` → `AnimationClip` via `PATH_PROPERTIES`, `PropertyBinding` for the `Object3D` tree | **done** |
| `loadMaterial` — **parsed into `GltfMaterial` records**: baseColor/metallic/roughness factors and textures, normal + scale, occlusion, emissive, `alphaMode`, `alphaCutoff`, `doubleSided`, `extensions` | **done (parse)** |
| `KHR_materials_ior`, `KHR_materials_specular` | **done** |
| `build_material()` → `MeshStandard` / `MeshPhysical` with `map`/`normalMap`/`metalnessMap`/`roughnessMap`/`specularColorMap`, the `useDerivativeTangents` clone | **done, skinned primitives only** |
| `loadTexture` / `loadImageSource` — PNG and JPEG decode from the BIN chunk | **done** |

---

## 3. What is missing, and what each piece unlocks

Ordered by how many eligible examples it stands in front of. "Owned by" names the rung that should
take it; **this rung** is `webgpu_postprocessing_bloom` (see `PLAN.md`).

| # | missing piece | Three source | ≈ Rust | blocks | owned by |
|---|---|---|---|---|---|
| 1 | **`Payload::Mesh` for non-skinned primitives.** `gltf_loader.rs:665` skips every primitive without a `skin`, so `scene.add( gltf.scene )` draws nothing unless the file is a character. | `createNodeMesh` | ~130 | **all 47** | **this rung** |
| 2 | **`assignFinalMaterial`** — the variant clone and cache: `vertexColors` (3), `flatShading`, `useDerivativeTangents`, and sharing one Rust material between primitives that share a glTF material. | `GLTFLoader.assignFinalMaterial` | ~100 | **all 47** | **this rung** |
| 3 | **`alphaMode`** — `BLEND` → `transparent` + `depthWrite = false`; `MASK` → `alphaTest = alphaCutoff`. | `loadMaterial` | ~30 | 8 (5 BLEND + 3 MASK) | **this rung** |
| 4 | **`emissiveFactor` → `material.emissive`** (+ `emissiveIntensity`). | `loadMaterial` | ~25 | 5 | **this rung** |
| 5 | **`emissiveTexture`, `occlusionTexture`** wired onto the material (parsed today, never read). `aoMap` also needs `uv1`. | `loadMaterial` | ~60 | 5 + 3 | next glTF rung |
| 6 | **`TEXCOORD_n, n > 0`** — `texCoord` on a texture reference and the `uv1`/`uv2` attributes. | `assignTexture` | ~60 | 3 | next glTF rung |
| 7 | **Primitive dedup (`createPrimitiveKey`) + geometry `groups`** — one geometry shared between primitives, and the `<mesh>_<i>` child split. | `loadGeometries`, `loadMesh` | ~120 | 1 today, but it is a correctness trap everywhere | next glTF rung |
| 8 | **`KHR_texture_transform`** | `GLTFTextureTransformExtension` | ~80 | 1 | later |
| 9 | **`KHR_materials_unlit`** → `MeshBasicMaterial` | `GLTFMaterialsUnlitExtension` | ~40 | 2 | later |
| 10 | **`KHR_materials_transmission` + `_volume`** — needs a transmission backdrop render pass, not just loader work | two extensions + `MeshPhysicalMaterial` | ~150 loader + a renderer pass | 4 + 3 | a transmission rung |
| 11 | **`KHR_materials_variants`** | `GLTFMaterialsVariantsExtension` | ~80 | 2 | later |
| 12 | **`KHR_materials_dispersion`** | extension + BSDF | ~120 | 1 | with transmission |
| 13 | **`KHR_draco_mesh_compression`** — a Draco decoder (wasm in JS; a Rust decoder or a port) | `DRACOLoader` | large | **7** | its own rung |
| 14 | **`KHR_mesh_quantization`** | core dequantize | ~60 | 2 | with Draco |
| 15 | **`EXT_texture_webp`** — a WebP decoder | `GLTFTextureWebPExtension` + image decode | ~60 + a crate | 2 | later |
| 16 | **`KHR_texture_basisu` + `EXT_meshopt_compression`** — KTX2/Basis transcode and meshopt | `KTX2Loader`, meshopt | large | 1 | later |
| 17 | **`EXT_mesh_gpu_instancing`** | extension → `InstancedMesh` | ~100 | 1 | later |
| 18 | **`CUBICSPLINE` interpolation** — reduced to LINEAR today by dropping tangents; correct only for single-keyframe tracks | `GLTFCubicSplineInterpolant` | ~120 | 0 of the eligible assets use it | when something needs it |
| 19 | **glTF cameras and lights (`KHR_lights_punctual`)** | `loadCamera`, extension | ~120 | 0 eligible (assets carry camera *nodes* but the examples make their own camera) | when something needs it |
| 20 | **`mode` ≠ TRIANGLES (Points / Lines / strips / fans)**, `GLTFMeshStandardSGMaterial` | `loadMesh` | ~100 | 0 eligible | when something needs it |

Items 1–4 are ~285 lines of Rust and stand in front of **every one of the 47**. That is the whole
argument for doing them now, in the cheapest example that forces them.

---

## 4. The ladder this implies

```
[this rung: webgpu_postprocessing_bloom]
    items 1–4 — the core loader capability, gated by primaryiondrive.json
    + BloomNode, uniformArray, const array, pass store/resolve config
        ↓
[a second glTF rung: items 5–7]       ← emissive/occlusion maps, uv1, primitive dedup
    cheapest gate: webgpu_postprocessing_3dlut (coffeeMug, KHR_materials_unlit)
    or webgpu_compute_geometry (LeePerrySmith, needs rung 12's compute)
        ↓
[PMREM-B: environment]                ← unblocks the other 24
        ↓
[a transmission rung: item 10]        → webgpu_shadowmap_opacity (0.0%), webgpu_postprocessing_sobel
        ↓
[a Draco rung: item 13]               → 7 more, incl. webgpu_upscaling_fsr1 / _taau, webgpu_caustics
```

The three examples that become nearly free the moment this rung lands, needing no further loader
work at all (their assets use no extension and no image), are `webgpu_postprocessing_godrays`
(0.0%, needs `GodraysNode`), `webgpu_compute_geometry` (needs rung 12's storage buffers) and
`webgpu_instancing_morph` (needs the `SunLight` addon and an instanced morph texture).

The five Michelle/Soldier examples — `webgpu_backdrop`, `webgpu_backdrop_area`,
`webgpu_backdrop_water`, `webgpu_mrt_mask`, `webgpu_reflection_blurred` — already have everything
they need from the loader (rung 10 did it); each is blocked purely on one renderer feature
(`backdropNode`, MRT, `reflector()`). They are the best measure of how much of the loader work is
*already* paid for: five gradeable examples waiting on nothing glTF-shaped.
