# Scout — `webgpu_loader_gltf` (2026-09-19)

**Headline: `webgpu_loader_gltf` is not a rung. It grades 0.1%, not 0.0%, on two runs, and it is the
only example on the ladder shortlist that fetches its model over the public internet
(raw.githubusercontent.com) at render time. The recommendation is `webgpu_pmrem_cubemap` (0.0%),
which isolates the piece that is actually missing — PMREM / IBL — from assets that live in the
vendor tree.**

Artifacts next to this file:

| file | size | what |
|---|---|---|
| `dump-pmrem_cubemap/` | 510 KB | full dump of the **recommended** example: `dump.json`, 10 `.wgsl`, `actual_full.png`, `actual.jpg` |
| `dump/` | 830 KB | full dump of `webgpu_loader_gltf` itself (13 `.wgsl`), with one caveat below |
| `webgpu_pmrem_cubemap.jpg` | 27 KB | the grader's reference JPEG for the recommendation |
| `webgpu_loader_gltf.jpg` | 57 KB | the grader's reference JPEG for the assigned example |

---

## 1. Grade confirmation

```
cd ~/src/vendor/three.js && flock /run/user/1000/three-rs-gpu.lock npm run test-e2e-webgpu -- …
```

Run 1 (alone):

```
Diff 0.1% in file: webgpu_loader_gltf (5.2s)
TEST PASSED! 1 screenshots rendered correctly.
```

Run 2 (with its six siblings):

```
Diff 0.1% in file: webgpu_loader_gltf (3.8s)
Diff 0.0% in file: webgpu_loader_gltf_anisotropy (3.2s)
Diff 0.0% in file: webgpu_loader_gltf_compressed (3.1s)
Diff 0.0% in file: webgpu_loader_gltf_dispersion (3.1s)
Diff 0.0% in file: webgpu_loader_gltf_iridescence (3.3s)
Diff 0.0% in file: webgpu_loader_gltf_sheen (3.4s)
Diff 0.1% in file: webgpu_loader_gltf_transmission (3.4s)
TEST PASSED! 7 screenshots rendered correctly.
```

Run 3 (the PMREM family):

```
Diff 0.0% in file: webgpu_pmrem_cubemap (3.2s)
Diff 0.0% in file: webgpu_pmrem_equirectangular (3.2s)
Diff 0.0% in file: webgpu_pmrem_scene (3.1s)
Diff 0.0% in file: webgpu_pmrem_test (3.1s)
TEST PASSED! 4 screenshots rendered correctly.
```

`webgpu_loader_gltf` is **not** on `exceptionList` in `test/e2e/puppeteer.js` (nor is any example
named here). It passes, because the grader's bar is `maxDifferentPixels = 0.1` — but 0.1% *is* the
bar, so the port would start with literally zero headroom against Three's own number. Per the
brief that is a stop.

There is a second, harder reason to stop. The example does

```js
fetch( 'https://raw.githubusercontent.com/KhronosGroup/glTF-Sample-Assets/main/Models/model-index.json' )
```

and then loads `…/Models/DamagedHelmet/glTF-Binary/DamagedHelmet.glb` from the same host. The
grader's request interception (`page.on('request')` in `puppeteer.js`) only rewrites
`http://localhost:1234/build/*`; everything else goes out to the real network with
`await request.continue()`. So this example's graded frame depends on GitHub's availability,
GitHub's `model-index.json` contents *on the day*, and the Chrome profile's HTTP cache
(`userDataDir: './.puppeteer_profile'` is reused between runs). It is not a reproducible target
and it never can be one without patching the vendor tree.

Third: it needs `UltraHDRLoader` (755 lines: MPF container walk, XMP metadata, two embedded JPEGs,
gainmap recombination into half-float), the `Inspector` addon, and `renderer.compileAsync`.

### Siblings graded, and why none of them is the pick either

| example | diff | env source | model | extra material work |
|---|---|---|---|---|
| `_anisotropy` | 0.0% | UltraHDR 2k + `backgroundBlurriness 0.5` | AnisotropyBarnLamp.glb | KHR_materials_anisotropy (`D_GGX_Anisotropic`, `V_GGX_…_Anisotropic`, bent normals) |
| `_sheen` | 0.0% | UltraHDR 2k | SheenChair.glb | KHR_materials_sheen (`BRDF_Sheen`, sheen env LUT) + Inspector |
| `_iridescence` | 0.0% | **HDRLoader** (`venice_sunset_1k.hdr`, local) | IridescenceLamp.glb | KHR_materials_iridescence (thin-film) |
| `_dispersion` | 0.0% | **HDRLoader** (`pedestrian_overpass_1k.hdr`, local) | DispersionTest.glb | transmission + dispersion → a whole backdrop/transmission render pass |
| `_transmission` | 0.1% | UltraHDR 2k | IridescentDishWithOlives.glb | DRACO (wasm decoder) + transmission pass |
| `_compressed` | 0.0% | *no environment* | coffeemat.glb | KTX2/Basis transcode + meshopt (both wasm) |

Every sibling is the base example **plus** one more KHR extension BSDF. They are all strictly
larger than the thing they have in common. That common thing is the environment.

### The pick: `webgpu_pmrem_cubemap` (0.0%)

`examples/webgpu_pmrem_cubemap.html`, 0.0% on this machine, not on the exception list, every asset
local (`examples/textures/cube/pisaHDR/{px,nx,py,ny,pz,nz}.hdr`, ~220 KB each). It is the same
IBL machinery the whole `webgpu_loader_gltf*` family needs, with **nothing** else new: no glTF, no
UltraHDR, no Inspector, no `Math.random`, no animation, no extension BSDF, no tone-mapping mode the
port hasn't got. Its two sibling cousins `webgpu_pmrem_equirectangular` (0.0%) and
`webgpu_pmrem_scene` (0.0%) are the natural follow-ups and share the same code; see §7.

`webgpu_pmrem_test` also grades 0.0% but uses `THREE.PMREMGenerator` **directly** from the page
plus `MeshPhysicalMaterial` and the `Inspector`; it is a worse first target because it exposes the
generator as public API rather than through `EnvironmentNode`.

Everything from §2 on is about **`webgpu_pmrem_cubemap`**, with `webgpu_loader_gltf` kept as the
"what this unlocks" target in §7.

---

## 2. What the recommended example does

`examples/webgpu_pmrem_cubemap.html`, ~100 lines of script. Imports:

```js
import * as THREE from 'three/webgpu';
import { normalWorldGeometry, uniform, pmremTexture } from 'three/tsl';
import { HDRCubeTextureLoader } from 'three/addons/loaders/HDRCubeTextureLoader.js';
import { OrbitControls } from 'three/addons/controls/OrbitControls.js';
```

```js
camera = new THREE.PerspectiveCamera( 45, window.innerWidth / window.innerHeight, 0.25, 20 );
camera.position.set( 0, 0, 8 );
scene  = new THREE.Scene();
renderer = new THREE.WebGPURenderer( { antialias: true, forceWebGL: false } );
renderer.toneMapping = THREE.ACESFilmicToneMapping;   // exposure left at 1
await renderer.init();
const controls = new OrbitControls( camera, renderer.domElement );
controls.minDistance = 2; controls.maxDistance = 10; controls.update();

new HDRCubeTextureLoader()
    .setPath( './textures/cube/pisaHDR/' )
    .load( [ 'px.hdr', 'nx.hdr', 'py.hdr', 'ny.hdr', 'pz.hdr', 'nz.hdr' ], function ( map ) {

        scene.backgroundNode = pmremTexture( map, normalWorldGeometry, uniform( 0.5 ) );

        const geometry = new THREE.SphereGeometry( 0.4, 64, 64 );

        for ( let i = 0; i < 6; i ++ )
        for ( let j = 0; j < 5; j ++ ) {
            const material = new THREE.MeshPhysicalNodeMaterial( {
                roughness: i / 5, metalness: j / 4, envMap: map } );
            const mesh = new THREE.Mesh( geometry, material );
            mesh.position.x = i - 2.5;
            mesh.position.y = j - 2;
            scene.add( mesh );
        }

    } );
```

Facts that matter for the port:

* **30 spheres**, `SphereGeometry( 0.4, 64, 64 )` (one geometry, 30 materials), laid out
  x = −2.5 … 2.5, y = −2 … 2, at z = 0. `roughness = i/5` (0, .2, .4, .6, .8, 1),
  `metalness = j/4` (0, .25, .5, .75, 1). Everything else is `MeshPhysicalNodeMaterial` default:
  white colour, `ior 1.5`, `specularIntensity 1`, `specularColor` white, `envMapIntensity 1`, no
  maps, no clearcoat/sheen/iridescence/transmission/anisotropy.
* **No lights at all.** The whole image is IBL. That is what makes it a clean gate: if the
  environment is wrong the frame is black, and if the environment is *nearly* right the sphere
  grid shows it as a 6×5 roughness/metalness sweep.
* **Background** is `scene.backgroundNode = pmremTexture( map, normalWorldGeometry, uniform( 0.5 ) )`
  — the PMREM sampled along the world normal of the background sphere at a fixed roughness 0.5, so
  the backdrop is a blurred pisa. It is *not* `scene.background`; the port's rung-7
  `Background::Node` path is the right seam.
* **`OrbitControls`** is constructed and `update()`d once with the default target `(0,0,0)`;
  camera stays at `(0,0,8)` looking down −Z. `enableDamping` is false, there is no `autoRotate`,
  and the deterministic injection pins `performance.now()` to 0. **The camera pose is static and
  can be hard-coded in the Rust example.**
* **No `Math.random()` draws** before the frame: no `Inspector`, no `range()`, no instancing.
  `Renderer::skip_random_draws` stays at 0.
* Nothing branches on `window.TESTING`.
* Tone mapping: `ACESFilmicToneMapping`, exposure 1 — the port already has
  `ToneMapping::AcesFilmic` (rung 7) and `tone_mapping_exposure` (rung 8).
* Assets: six `.hdr` files, Radiance RGBE, 256×256 each (the dump's source cube texture is
  `256×256×6`), decoded on the CPU by `HDRLoader.parse` and uploaded as **half-float RGBA**
  (`HDRCubeTextureLoader.type = HalfFloatType`), `LinearSRGBColorSpace`, `generateMipmaps = false`.

---

## 3. Dump reading (`dump-pmrem_cubemap/`)

`flock … node tools/dump-webgpu.mjs webgpu_pmrem_cubemap --out …` — worked first time. It printed
one page error, `Failed to load resource: the server responded with a status of 404 (Not Found)`,
which is also printed for every other example I dumped today and does not correspond to any asset
the page uses; the frame is complete.

**10 shader modules, 5 render pipelines, 0 compute pipelines, 5 bind-group layouts, 8 textures,
1 sampler, 23 render passes, 23 submits** (one submit per pass).

| module pair | pipeline | used in |
|---|---|---|
| `m00/m01_PMREM_cubemap` (64 / 47 lines) | `renderPipeline_PMREM_cubemap_50` | 1 pass: source cube → cubeUV mip 0 |
| `m02/m03_PMREM_ggx` (65 / **354** lines) | `renderPipeline_PMREM_ggx_49` | **20 passes**, ping-pong between the two `PMREM.cubeUv` targets |
| `m04/m05_Background.material` (88 / **370** lines) | `renderPipeline_Background.material_47` | main pass, first draw |
| `m06/m07` (83 / **737** lines) | `renderPipeline_MeshPhysicalNodeMaterial_17` | main pass, 30 draws |
| `m08/m09_outputColorTransform` (62 / 103 lines) | `renderPipeline_outputColorTransform_51` | output pass, `draw(3)` |

`m08/m09` are byte-comparable to rung 8's `outputColorTransform_26.*-r186.wgsl` (103 / 62 lines) —
already ported.

**Chronological order** (from `dump.json`'s `order` log): the frame-buffer target + MSAA + depth
are created first, then pass 0 begins… and in the log the PMREM work is interleaved: the six
`writeTexture` uploads of the HDR faces, `createTexture PMREM.cubeUv` (768×1024), the cubemap
conversion pass, `createTexture PMREM.cubeUv` (second, 768×1024), then 20 GGX passes, *then* the
`Background.material` and `MeshPhysicalNodeMaterial` modules/pipelines, `DFG_LUT`, the
`depthBuffer`, the main pass (31 `drawIndexed`: background 5952 indices, then 30 spheres), and the
output pass. In port terms: **the PMREM chain must run, complete and submit before the scene pass
records its first draw.** That is `PMREMNode.updateBefore` → `_getPMREMFromTexture` →
`generator.fromCubemap`, driven from material setup, not from the app.

**Textures** (8):

| id | label | size | format | mips |
|---|---|---|---|---|
| t0 | *(frame-buffer target)* | 800×500 | `rgba16float` | 1 |
| t1 | `-msaa` | 800×500, `sampleCount 4` | `rgba16float` | 1 |
| t2 | *(scene depth)* | 800×500, `sampleCount 4` | `depth24plus` | 1 |
| t6 | `PMREM.cubeUv` | **768×1024** | `rgba16float` | 1 |
| t14 | *(the HDR cube)* | 256×256×**6** | `rgba16float` | 1 |
| t24 | `PMREM.cubeUv` | **768×1024** | `rgba16float` | 1 |
| t133 | `DFG_LUT` | 16×16 | `rg16float` | 1 |
| t229 | `depthBuffer` | 800×500, `sampleCount 1` | `depth24plus` | 1 |

The cubeUV atlas is a **2D** texture, 768 wide × 1024 tall, single mip — the "mips" are laid out
as tiles inside it, which is why the shader does the `getFace`/`getUV`/offset arithmetic by hand.
`768 = 3 × 256`; `1024 = 4 × 256` (the mip pyramid stacked vertically, plus the LOD_MIN extras at
x = 3 × 16 × …). `_generateCubeUVSize( 256 )` ⇒ `maxMip = log2(256) − 2 = 6`,
`texelHeight = 1/1024`, `texelWidth = 1/768`; those two land in the material's uniform block as
`nodeUniform16`/`nodeUniform17` and `maxMip` as `nodeUniform13`.

**Exactly one `GPUSampler` is ever created** for the whole page:

```
{ addressModeU/V/W: clamp-to-edge, magFilter: nearest, minFilter: nearest,
  mipmapFilter: nearest, lodMinClamp: 0, lodMaxClamp: 32, maxAnisotropy: 1 }
```

and it is bound for **both** the `DFG_LUT` and the `PMREM.cubeUv` in every sphere's object bind
group (bindings 1 and 3 of bgl 134 both resolve to sampler id 13). Note this differs from the rung
10 scout's dump, where the DFG LUT's sampler was mag/min linear — reproduce what *this* dump says
for *this* example and check the DFG sampler descriptor again if a rung-8-era assumption leaks in.
The cubeUV sample in the fragment is a single `textureSampleGrad( …, vec2(0), vec2(0) )` per mip
with the nearest sampler; there is no manual bilinear helper in r186's emitted WGSL.

**Bind-group layouts** (5):

| bgl | entries | who |
|---|---|---|
| 11 | `0: buffer (VERTEX\|FRAGMENT\|COMPUTE)` | group 0 (`bindGroup_render`) everywhere |
| 16 | `0: sampler(FRAG)`, `1: texture(FRAG, viewDimension cube)`, `2: buffer(7)` | PMREM cubemap-conversion material |
| 31 | `0: buffer(7)`, `1: sampler(FRAG)`, `2: texture(FRAG)` | PMREM GGX blur material |
| 134 | `0: buffer(7)`, `1: sampler`, `2: texture` (DFG_LUT), `3: sampler`, `4: texture` (cubeUV) | the 30 spheres |
| 237 | `0: sampler`, `1: texture`, `2: buffer(7)` | output pass |

**The sphere fragment (`m07`, 737 lines) against rung 8's physical fragment** (466 lines,
`scouts/rung8/MeshStandardMaterial_20.frag-r186.wgsl`): **+271 lines, and they are all IBL.** The
new material is three named helper functions plus one inlined block:

* `fn roughnessToMip( roughness ) -> f32` — the five-branch piecewise curve
  (`>= 0.8`, `>= 0.4`, `>= 0.305`, `>= 0.21`, else `-2 * log2( 1.16 * roughness )`),
* `fn getFace( direction ) -> f32`, `fn getUV( direction, face ) -> vec2<f32>`,
* the inlined cubeUV addressing (per mip): `max( 4 - mip, 0 )`, `exp2`, the
  `if face > 2 { uv.y += size; face -= 3 }` fold, the `+ ( 3.0 * 16.0 )` LOD_MIN column offset, the
  `4 * ( exp2( maxMip ) - size )` row offset, then `* texelWidth` / `* texelHeight`, then
  `textureSampleGrad`. Done **twice** and `mix`ed on `fract( mip )` when the mip is fractional,
  and then the whole thing is done **again** for the irradiance lobe at `roughnessToMip( 1.0 )`
  with `normalWorld` instead of the reflect vector.

The radiance direction is Three's roughness-biased reflection:

```wgsl
normalize( ( render.cameraWorldMatrix * vec4( normalize( mix(
    reflect( -positionViewDirection, normalView ), normalView,
    Roughness*Roughness*Roughness*Roughness ) ), 0.0 ) ).xyz )
```

then flipped through `object.nodeUniform14 * vec4( vec3( d.x, -d.y, d.z ), 1.0 )`
(`materialEnvRotation`, identity here, but it is a real `mat4` uniform in the object block and
must be emitted). `normalWorld` is `normalize( ( vec4( normalView, 0 ) * render.cameraViewMatrix ).xyz )`.
`iblIrradiance` is multiplied by `PI` in `EnvironmentNode`. `radiance`/`iblIrradiance` start at
`vec3(0)` and are `addAssign`ed, exactly the two accumulators
`src/materials/physical.rs` already declares and never writes to.

`m05_Background.material` (370 lines) is the same `roughnessToMip`/`getFace`/`getUV` trio over
`normalWorldGeometry` at a constant roughness 0.5 — i.e. once the material path works, the
background is the same node graph with a different normal and a uniform roughness.

`m03_PMREM_ggx` (354 lines) is the convolution kernel: GGX VNDF importance sampling,
`GGX_SAMPLES = 256`, with its own `getFace`/`getUV`. `m01_PMREM_cubemap` (47 lines) is the trivial
`textureSample( cube, direction )` blit that fills mip 0.

**Not captured by the dump tool:** the `order` log contains no `setViewport`/`setScissorRect` ops,
and `queue.writeTexture` records `size.width` as 0. `PMREMGenerator` renders each cubeUV tile with
a viewport into a shared target, so the rung worker must read the viewport rectangles out of
`src/renderers/common/extras/PMREMGenerator.js` (`_applyPMREM`, `_blur`, `_halfBlur`) rather than
out of `dump.json`. Worth a one-line fix to `tools/dump-webgpu.mjs` (hook
`GPURenderPassEncoder.setViewport/setScissorRect`) as step 0 of the rung — every future
render-target rung will want it.

### The `webgpu_loader_gltf` dump (`dump/`), for the record

13 modules, 8 render pipelines, 101 passes. It confirms the shape of the follow-on rung:

* `t16` 2048×1024 `rgba16float`, **12 mips** — the UltraHDR equirect, mipped with a new
  `mipmap-rgba16float-2d-array` pipeline.
* `t6` 1024×1024×**6** `rgba16float` + `t7` depth — `CubeMapNode` renders the equirect into a
  **cube render target** (6 passes, pipeline `renderPipeline_NodeMaterial_18`) because
  `scene.background` is an equirect texture; the background fragment then samples a
  `texture_cube<f32>` with `textureSampleLevel( …, render.nodeUniform5 )`, the LOD coming from
  `backgroundBlurriness`. So `scene.background = <equirect>` is **not** an equirect UV sample in
  the shader — it is a runtime equirect→cube bake.
* `t115`/`t128` `PMREM.cubeUv` 1536×2048 (the 2k source ⇒ bigger atlas), `PMREM_equirect` ×1 +
  `PMREM_ggx` ×22 passes.
* Five DamagedHelmet maps, all 2048², 12 mips: two `rgba8unorm-srgb` and three `rgba8unorm`.
* bgl 408 has **seven** sampler/texture pairs: the five glTF maps + cubeUV + DFG_LUT.
* **Caveat:** the dump reported
  `Async render pipeline creation failed (renderPipeline_Material_MR_21): Cannot read properties of null (reading 'module')`
  — an artifact of the tool's `createShaderModule` hook interacting with `compileAsync`, so the
  helmet is **missing from `dump/actual.jpg`**. The WGSL (`m11`/`m12_Material_MR`, 91 / 805 lines)
  and bgl 408 were still captured and are correct. Do not use `dump/actual.jpg` as a reference.

---

## 4. Gap list against the port

Port read at `main` @ `2cf90bd`. Ten e2e rungs green; `src/materials/physical.rs` already has
`PhysicalLightingModel` with `radiance` / `iblIrradiance` declared and documented as "with no
environment node nothing ever adds to them" (`physical.rs:269`, `:292`). The port has **no**
environment path of any kind for PBR: `grep -rn "environment\|pmrem"` finds only
`MeshBasicNodeMaterial.env_map` (rung 3's `BasicEnvironmentNode` over a `CubeTexture`).

### Have

| thing | Rust item |
|---|---|
| `MeshPhysicalNodeMaterial`, `ior`/`specularColor`/`specularIntensity`, `SpecularF90` | `src/materials/physical.rs`, `src/materials/mod.rs` (rung 8) |
| `DFG_LUT` 16×16 `rg16float` + `EnvironmentBRDF`/multiscattering | `src/materials/dfg_lut.rs`, `physical.rs` (rung 8) |
| `SphereGeometry(0.4,64,64)` | `src/geometries/sphere.rs` |
| ACESFilmic tone mapping + exposure | `RenderOutputNode` (rung 7/8) |
| `Background::Node` (a TSL graph as the background) | rung 7 |
| Render-to-texture, render targets, MSAA target + output pass | `src/renderer/render_target.rs`, `pass.rs` |
| `CubeTexture` upload, cube views, `textureSample` on cube | `src/textures/cube_texture.rs` (rung 3) |
| Mipmap blit pipeline | `src/renderer/mipmap.rs` |
| `Loop`/`If`/`IfVar` statement nodes, `u32`/`ivec2`, `textureSampleGrad`-class sampling | `src/nodes/` (rungs 6–8) |
| `OrthographicCamera`, `PerspectiveCamera`, `Node` tree, `RenderList` | core |

### Missing — owned by this rung

| # | thing | Three source | lands in | JS lines to read | rough Rust | kind |
|---|---|---|---|---|---|---|
| 1 | **RGBE (`.hdr`) decode** — `HDRLoader.parse`: the Radiance header, RLE-and-flat scanline decode, RGBE → half-float | `examples/jsm/loaders/HDRLoader.js` (463) | `src/loaders/hdr_loader.rs` (new) | 463 | ~350 | loader |
| 2 | **`HDRCubeTextureLoader`** — six faces into one `CubeTexture`, `HalfFloatType`, `LinearSRGBColorSpace`, `generateMipmaps = false`, `minFilter Linear` | `examples/jsm/loaders/HDRCubeTextureLoader.js` (164) | `src/loaders/hdr_cube_texture_loader.rs` (new) | 164 | ~120 | loader |
| 3 | **Half-float texture support** — `TextureType::HalfFloat`, `rgba16float`/`rg16float` upload paths, `f32 → f16` conversion on the CPU, non-sRGB sample type | `WebGPUTextureUtils` | `src/textures/texture.rs`, `src/renderer/` | ~150 | ~200 | renderer |
| 4 | **`RenderTarget` with a non-canvas colour format + viewport-scoped draws** — PMREM renders 21 passes into two 768×1024 `rgba16float` targets, each pass covering only a tile via `setViewport` | `RenderTarget`, `Renderer.setRenderTarget` | `src/renderer/render_target.rs`, `pass.rs` | ~200 | ~200 | renderer |
| 5 | **`PMREMGenerator`** — `_setSize`, `_allocateTargets`, `_textureToCubeUV`, `_applyPMREM`, `_blur`/`_halfBlur`, the lod mesh/geometry tables, the cubemap material, the GGX blur material and its uniforms | `src/renderers/common/extras/PMREMGenerator.js` (**951**) | `src/renderer/pmrem.rs` (new) | 951 | **~800** | renderer |
| 6 | **`PMREMUtils`** — `getFace`, `getUV`, `roughnessToMip`, `textureCubeUV`, `sphericalGaussianBlur`, `ggxConvolution` as TSL functions | `src/nodes/pmrem/PMREMUtils.js` (354) | `src/nodes/pmrem_utils.rs` (new) | 354 | ~450 | node system |
| 7 | **`PMREMNode`** — `pmremTexture( value, uvNode, levelNode )`, the per-renderer PMREM cache, `updateBefore` running the generator once, the `texelWidth`/`texelHeight`/`maxMip` uniforms, `materialEnvRotation` | `src/nodes/pmrem/PMREMNode.js` (428) | `src/nodes/pmrem_node.rs` (new) | 428 | ~350 | node system |
| 8 | **`EnvironmentNode`** — the radiance context (roughness-biased reflect, `cameraWorldMatrix`), the irradiance context (`normalWorld`, `× PI`), `materialEnvIntensity`, `addAssign` into `radiance`/`iblIrradiance` | `src/nodes/lighting/EnvironmentNode.js` (214) | `src/materials/environment.rs` (new) + a hook in `physical.rs` | 214 | ~250 | material |
| 9 | **`material.envMap` on a Standard/Physical material** and `scene.environment` | `NodeMaterial.setupEnvironment`, `Scene.environment` | `src/materials/mod.rs`, `src/objects/scene.rs` | ~60 | ~60 | material |
| 10 | **A "run this before the scene pass" hook** — `NodeUpdateType.RENDER` / `updateBefore` on a node, so the PMREM chain is generated during material setup and submitted before pass 0 records a draw | `NodeUpdateType`, `Renderer._nodes` | `src/renderer/mod.rs`, `src/nodes/node.rs` | ~80 | ~120 | renderer |
| 11 | **`normalWorldGeometry` TSL** (the background's uv node) | `src/nodes/accessors/Normal.js` | `src/nodes/tsl.rs` | ~30 | ~40 | node system |
| 12 | **The example + the e2e row** | — | `examples/webgpu_pmrem_cubemap.rs`, `tests/e2e/main.rs` | — | ~150 | — |

**Total ≈ 3 000 lines of Rust** over ~3 300 lines of JS read. Items 5–7 are two thirds of it.

**Paths every other example touches** (merge-conflict / regression risk, flag hard):

* **`src/renderer/mod.rs`** — item 10 puts a pre-pass hook into the render entry point, the same
  place rung 7's `render_shadows()` and rung 9's `PassNode` already sit, and the same place rungs
  10/11/12 are editing right now.
* **`src/textures/texture.rs`** — item 3 adds a `TextureType`/format axis to a struct that every
  material touches. Rung 10 is *also* editing this file (`Wrapping::Repeat`, `from_bytes`).
* **`src/materials/physical.rs`** — item 8 finally writes into `radiance`/`iblIrradiance`, which
  changes the emitted WGSL for **`webgpu_lights_physical` (rung 8)**. It must stay
  byte-identical when there is no environment: the two accumulators are already emitted as
  `vec3(0)` there, so the environment block has to be strictly conditional on
  `material.env_map.is_some() || scene.environment.is_some()`.
* **`src/renderer/render_target.rs`** — item 4 generalises formats/viewports; rung 9's
  `PassNode` and rung 7's shadow maps both read it.

### Explicitly NOT in this rung

Cube *render* targets (`CubeRenderTarget`, `CubeMapNode`), the equirect→cube bake,
`backgroundBlurriness`/`backgroundIntensity`, `PMREMGenerator.fromScene` (the `EXTRA_LODS` /
`sphericalGaussianBlur` half of the file), UltraHDR, and every KHR extension BSDF. See §7.

### Overlap with rung 10 (skinning) — read this before starting

`scouts/rung10/PLAN.md` §4 "What is missing — owned by rung 10" claims, and
`docs/gltf-progress.md` "Next, in order" item 3 repeats, the following, **all of which are
glTF-image work and none of which this rung needs**:

| rung 10 item | this rung |
|---|---|
| 5. PNG decode + `TextureLoader::from_bytes( &[u8], mime )` | **not needed** — the `.hdr` faces are separate files on disk; item 1 is an independent RGBE decoder |
| 6. `Wrapping::Repeat` | **not needed** — every sampler here is clamp-to-edge |
| 7. `GLTFLoader` material/texture wiring, KHR_materials_specular / _ior, the sRGB/linear split, `flipY`, `normalScale.y` | **not needed** — no glTF at all |
| 8. `LinearToneMapping` + `toneMappingExposure` | landed with rung 8; this example uses ACESFilmic |
| — | items 1–11 above are **untouched by rung 10** |

So the two rungs are disjoint. The one shared file is `src/textures/texture.rs` (rung 10 adds
`Wrapping::Repeat` and byte-decode; this rung adds `HalfFloat`/format), and they should be
sequenced or merged carefully rather than done twice. **Do not let the PMREM rung worker port
PNG-from-bytes or the glTF material wiring — rung 10 owns both.** Conversely, when rung 10 lands,
nothing in it gives this rung a head start beyond `Texture` gaining a byte-level constructor.

---

## 5. Gates beyond the pixel diff

The image is a good gate here (no lights ⇒ a broken environment is a black frame), but it is late.
In order of how early they fire:

1. **RGBE decode oracle (dump it now, it costs nothing).** `HDRLoader.parse` is pure CPU. Run
   Three's own parser under node on `examples/textures/cube/pisaHDR/px.hdr` and dump
   `{ width, height, type, data }` (the `Uint16Array` of half-floats) to JSON; assert the port's
   decoder bit-for-bit. This is the rung-10 `bones_t0.json` pattern. **I did not dump this** —
   it needs a five-line node script against `examples/jsm/loaders/HDRLoader.js` and no GPU, so the
   rung worker should do it as step 1. Expected format:
   `{ "file": "px.hdr", "width": 256, "height": 256, "data": [ <256*256*4 u16> ] }` (~1 MB per
   face; one face is enough).
2. **WGSL diff.** The five modules in `dump-pmrem_cubemap/` are the spec. `examples/dump_wgsl.rs`
   should grow blocks for `PMREM_cubemap`, `PMREM_ggx`, `Background.material` and the
   `MeshPhysicalNodeMaterial`, and be diffed line-for-line against `m01`, `m03`, `m05`, `m07`.
   `roughnessToMip`, `getFace` and `getUV` are emitted as *named functions* — three small,
   self-contained diffs that fire before any pixel is compared.
3. **PMREM readback.** `PMREM.cubeUv` is a plain 768×1024 `rgba16float` texture and the port can
   read it back (rung 4/9 already read render targets back). Two cheap assertions without an
   oracle: (a) mip 0's six 256² tiles equal the source cube faces after the conversion pass
   (the conversion is a pure resample), and (b) the mean of the last GGX level is within ~1e-3 of
   the mean of mip 0 (energy conservation). For a real oracle, add a `page.evaluate` readback of
   the cubeUV target to the dump tool and store it — worth doing once, it is the only number that
   separates "my GGX kernel is wrong" from "my addressing is wrong".
4. **`roughnessToMip` as a unit test.** It is a scalar function with five branches; table-test it
   against the WGSL at roughness 0, .05, .2, .21, .3, .305, .4, .5, .8, .9, 1.
5. **QUnit.** `test/unit/src/extras/PMREMGenerator.tests.js` does not exist in r186
   (`test/unit/three.source.unit.js` has no PMREM entry), so there is no upstream unit test to
   borrow. `HDRLoader` likewise has none.
6. **Regression gate:** rungs 5 and 8 must not move. `webgpu_lights_physical` at 4/100000 is the
   canary for item 8 leaking into the no-environment path.

---

## 6. Order of work

Each step leaves the ladder green (`cargo test --test e2e -- --nocapture --test-threads=1`).

0. **Two minutes of tooling:** hook `setViewport`/`setScissorRect` in
   `tools/dump-webgpu.mjs` and re-dump; fix the `writeTexture` size field while there.
1. **RGBE decode.** `src/loaders/hdr_loader.rs` + `hdr_cube_texture_loader.rs`, against the
   node-dumped oracle from §5.1. No GPU, no renderer changes. *Gate:* bit-exact decode of all six
   pisa faces.
2. **Half-float textures.** `TextureType::HalfFloat`, the `rgba16float` upload path, and a
   readback test that a uploaded half-float cube face round-trips. *Gate:* a unit test; ladder
   unchanged.
3. **Render targets with a format and a viewport.** Generalise `render_target.rs` to
   `rgba16float` + `setViewport`, with a 2-tile toy test. *Gate:* rungs 4 and 9 unchanged.
4. **`PMREMUtils` + the cubemap conversion pass.** Port `getFace`/`getUV`/`roughnessToMip` and the
   `PMREM_cubemap` material; run the one conversion pass into a 768×1024 target. *Gate:* `m01`'s
   WGSL matches; the readback of mip 0's six tiles equals the source faces.
5. **`PMREMGenerator._applyPMREM` + the GGX blur material.** The 20 ping-pong passes. *Gate:*
   `m03`'s 354 lines match line-for-line; the energy assertion of §5.3.
6. **`PMREMNode` + the pre-pass hook.** `pmremTexture(...)`, the per-renderer cache, `updateBefore`
   ordering so the 21 PMREM passes are submitted before the scene pass. *Gate:* the pass/submit
   order in the port's own dump matches `dump.json`'s `order`.
7. **`EnvironmentNode` into `physical.rs`.** *Gate:* `m07`'s 737 lines match, **and**
   `webgpu_lights_physical` is still 4/100000 and its WGSL still 466 lines.
8. **The background.** `scene.backgroundNode = pmremTexture( map, normalWorldGeometry, uniform(0.5) )`
   through rung 7's `Background::Node`. *Gate:* `m05` matches.
9. **`examples/webgpu_pmrem_cubemap.rs` + the e2e row.** *Gate:* the grader.

Steps 1–3 are independent of 4–8 and could be a first sitting on their own if the rung is split.

---

## 7. What this rung unlocks

This is the case for doing it. `scene.environment` / `envMap` under PMREM is the single most
reused addon-level feature in the WebGPU example set. Examples importing `HDRLoader`,
`UltraHDRLoader`, `HDRCubeTextureLoader` or setting `scene.environment`:
**56 `webgpu_*` examples** (`grep -lE "HDRLoader|UltraHDRLoader|HDRCubeTextureLoader|environment =" examples/webgpu_*.html | wc -l`).

What each further piece buys, cheapest first:

| next piece | cost on top of this rung | unlocks |
|---|---|---|
| nothing | — | `webgpu_pmrem_cubemap` |
| `PMREMGenerator.fromScene` (`EXTRA_LODS`, `sphericalGaussianBlur`, the cube camera) | ~250 lines | `webgpu_pmrem_scene` (0.0%) |
| `UltraHDRLoader` (MPF/XMP walk, two JPEG decodes, gainmap recombine) | ~600 lines | `webgpu_pmrem_equirectangular` (0.0%), and the equirect half of the gltf family |
| `CubeMapNode` + `CubeRenderTarget` (the equirect→cube bake) + `backgroundBlurriness`/`backgroundIntensity` | ~350 lines | `scene.background = <equirect>` — required by **every** `webgpu_loader_gltf*` example |
| rung 10's glTF material/texture wiring + PNG-from-bytes | *already rung 10's* | the glTF half |

So the ladder that reaches `webgpu_loader_gltf` is:

```
[this rung: PMREM + IBL + RGBE]  →  [rung 10: glTF images/materials]  →
[UltraHDR + CubeMapNode background]  →  webgpu_loader_gltf_iridescence / _sheen / _anisotropy …
```

`webgpu_loader_gltf` itself should probably **never** be a graded rung (network + 0.1%); its two
local-asset, 0.0% cousins `webgpu_loader_gltf_iridescence` (HDRLoader, no UltraHDR, no Inspector,
one extra BSDF) and `webgpu_loader_gltf_dispersion` (HDRLoader, but a transmission pass) are the
right destinations, and `_iridescence` is the cheaper of the two by a wide margin because it needs
no backdrop render.

Beyond the glTF family, the environment work is a precondition for `webgpu_clearcoat`,
`webgpu_materials_transmission`, `webgpu_reflection_roughness`, `webgpu_cubemap_adjustments`,
`webgpu_lights_sunlight`, `webgpu_parallax_uv`, `webgpu_materials_alphahash`, `webgpu_mrt`,
`webgpu_deferred` and the `webgpu_postprocessing_*` family's scenes.

---

## 8. Verdict

**`webgpu_loader_gltf`: not a rung.** 0.1% twice against Three's own reference on this machine
(the grader's exact pass threshold), and its model comes off the public internet at render time.
Do not put it on the ladder.

**`webgpu_pmrem_cubemap`: a rung, but a big one — two sittings, not one.** ~3 000 lines of Rust,
of which `PMREMGenerator` + `PMREMUtils` + `PMREMNode` are ~1 600. The clean split is
**steps 1–3 (RGBE decode, half-float textures, formatted/viewported render targets — ~800 lines,
no node-system work, gated entirely by CPU oracles and readbacks)** and **steps 4–9 (the PMREM
chain, `EnvironmentNode`, the background, the image — ~2 200 lines, gated by five WGSL diffs and
the grader)**. Step 1–3's gates need no GPU contention at all, which suits this machine.

Risks, in order:

1. **`physical.rs` regression.** Writing into `radiance`/`iblIrradiance` changes the shared
   physical material; rung 8's `webgpu_lights_physical` (4/100000) must stay byte-identical when
   no environment is present. Gate it with the WGSL line count before the pixels.
2. **The pre-pass ordering hook** (`updateBefore` / `NodeUpdateType.RENDER`) lands in
   `src/renderer/mod.rs`, which rungs 10, 11 and 12 are all editing today. Cut the branch from
   whatever tip has those merged, or expect the same 16-file merge rung 7 had.
3. **The cubeUV addressing is unforgiving and silent.** A wrong `texelWidth`, a wrong LOD_MIN
   column offset or a missed `-d.y` flip gives a plausible, wrong image. The readback of the
   cubeUV atlas (§5.3) is what separates the three failure modes; budget for building it.
