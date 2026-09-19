# Scout — `webgpu_postprocessing_bloom` (2026-09-19)

**Subject: GLTFLoader as a capability, and the first example to gate it.**

**Headline: the pick is `webgpu_postprocessing_bloom` — 0.0% on two runs, off the exception list,
one local 5.8 MB `.glb` and nothing else on disk. It is the cheapest example on the ladder whose
*whole scene* is a glTF file: 20 nodes, 6 primitives, 3 materials, vertex colours, an alpha-blended
material, emissive factors and a node-TRS animation clip, none of which the port's loader wires up
today. The price is one addon post node (`BloomNode`, 599 lines JS) and two small TSL node types
(`uniformArray`, const `array`). ~1 300 lines of Rust, one sitting.**

Artifacts next to this file:

| file | size | what |
|---|---|---|
| `dump/` | 460 KB | full dump: `dump.json`, 14 `.wgsl`, `actual_full.png`, `actual.jpg` |
| `primaryiondrive.json` | 33 KB | **the numeric oracle** — Three's own `GLTFLoader` parse of `PrimaryIonDrive.glb` (see §5.1) |
| `oracle.mjs` | 5 KB | the node script that produced it; re-runnable, no GPU, no network |
| `webgpu_postprocessing_bloom.jpg` | 27 KB | the grader's reference JPEG |

See also `GLTFLOADER.md` in this directory for the capability note (what the loader unlocks, with
counts).

---

## 1. Grade confirmation

```
cd ~/src/vendor/three.js && flock /run/user/1000/three-rs-gpu.lock npm run test-e2e-webgpu -- …
```

Run 1 (six candidates):

```
Diff 0.0% in file: webgpu_backdrop (3.1s)
Diff 0.0% in file: webgpu_mrt_mask (3.0s)
Diff 0.0% in file: webgpu_postprocessing_bloom (3.1s)
Diff 0.0% in file: webgpu_postprocessing_godrays (3.1s)
Diff 0.0% in file: webgpu_shadowmap_opacity (3.2s)
Diff 0.1% in file: webgpu_tsl_halftone (3.1s)
TEST PASSED! 6 screenshots rendered correctly.
```

Run 2 (four of them again):

```
Diff 0.0% in file: webgpu_postprocessing_bloom (3.3s)
Diff 0.0% in file: webgpu_postprocessing_godrays (3.1s)
Diff 0.0% in file: webgpu_shadowmap_opacity (3.1s)
Diff 0.1% in file: webgpu_tsl_halftone (3.0s)
TEST PASSED! 4 screenshots rendered correctly.
```

`webgpu_postprocessing_bloom` is **0.0% twice** and is **not** on `exceptionList` in
`test/e2e/puppeteer.js`. Its only asset is `examples/models/gltf/PrimaryIonDrive.glb`, a local file;
the page makes no network request of its own. (Twelve of the 59 `webgpu_*` GLTFLoader examples *are*
on the exception list — see `GLTFLOADER.md` §3 — and `webgpu_tsl_halftone` grades at the threshold,
so both are off the table by the brief's rule.)

### Why not the other candidates

| candidate | diff | glTF exercised | what else it would cost |
|---|---|---|---|
| **`webgpu_postprocessing_bloom`** | **0.0 / 0.0** | **20 nodes, 6 prims, 3 mats, COLOR_0, TANGENT, alphaMode BLEND, emissive, doubleSided, node-TRS clip** | `BloomNode` (599 JS), `uniformArray`, const `array` |
| `webgpu_shadowmap_opacity` | 0.0 / 0.0 | DragonAttenuation: 2 prims, 2 JPEG maps | KHR_materials_transmission + _volume, a transmission backdrop pass, `shadowMap.transmitted`, `castShadowNode`, AgX tone mapping — renderer-core work, larger than the loader work |
| `webgpu_postprocessing_godrays` | 0.0 / 0.0 | godrays_demo: 2 prims, **1 material, no textures, no animation** | `GodraysNode` + `BilateralBlurNode` + `depthAwareBlend`; barely gates the loader at all |
| `webgpu_backdrop` | 0.0 | Michelle — the path rung 10 already covers | `backdropNode` / `viewportSharedTexture` (a shared-viewport colour copy), 8 new TSL ops |
| `webgpu_mrt_mask` | 0.0 | Michelle — already covered | MRT in `PassNode`, `GaussianBlurNode` |
| `webgpu_tsl_halftone` | **0.1 / 0.1** | Michelle — already covered | at the grader's threshold: **stop** |
| `webgpu_animation_retargeting` | not graded | Michelle + Soldier (2 skins, 7 UV sets) | `reflector()` (a planar reflection render pass), `SkeletonHelper`, `SkeletonUtils.retargetClip` (496 JS) |
| `webgpu_postprocessing_3dlut` | not graded | coffeeMug: KHR_materials_unlit, 1 JPEG | 3-D textures (`texture3D`) — a new texture dimension — plus three LUT loaders |

Everything from §2 on is about **`webgpu_postprocessing_bloom`**.

---

## 2. What the example does

`examples/webgpu_postprocessing_bloom.html`, ~110 lines of script. Imports:

```js
import * as THREE from 'three/webgpu';
import { pass } from 'three/tsl';
import { bloom } from 'three/addons/tsl/display/BloomNode.js';
import { Inspector } from 'three/addons/inspector/Inspector.js';
import { OrbitControls } from 'three/addons/controls/OrbitControls.js';
import { GLTFLoader } from 'three/addons/loaders/GLTFLoader.js';
```

The scene:

```js
const scene = new THREE.Scene();                       // no background node, no scene.background
camera = new THREE.PerspectiveCamera( 40, w / h, 1, 100 );
camera.position.set( - 5, 2.5, - 3.5 );
scene.add( camera );
scene.add( new THREE.AmbientLight( 0xcccccc ) );       // intensity 1
const pointLight = new THREE.PointLight( 0xffffff, 100 );
camera.add( pointLight );                              // a child of the camera, at its origin

const gltf = await new GLTFLoader().loadAsync( 'models/gltf/PrimaryIonDrive.glb' );
scene.add( gltf.scene );
mixer = new THREE.AnimationMixer( gltf.scene );
mixer.clipAction( gltf.animations[ 0 ].optimize() ).play();

renderer = new THREE.WebGPURenderer( { antialias: true } );
renderer.toneMapping = THREE.ReinhardToneMapping;      // exposure left at 1
renderer.inspector = new Inspector();

renderPipeline = new THREE.RenderPipeline( renderer );
const config = { storeMultisampledColorBuffer: false, storeMultisampledDepthBuffer: false,
    storeMultisampledStencilBuffer: false, resolveColorBuffer: true,
    resolveDepthBuffer: false, resolveStencilBuffer: false };
const scenePass      = pass( scene, camera, config );
const scenePassColor = scenePass.getTextureNode( 'output' ).toInspector( 'Color' );
const bloomPass      = bloom( scenePassColor ).toInspector( 'Bloom' );   // strength 1, radius 0, threshold 0
renderPipeline.outputNode = scenePassColor.add( bloomPass );

const controls = new OrbitControls( camera, renderer.domElement );  // target (0,0,0), constructor update()
// animate(): timer.update(); mixer.update( timer.getDelta() ); renderPipeline.render();
```

Facts that matter for the port:

* **Camera pose is static.** `OrbitControls` is constructed (which calls `update()`, i.e.
  `lookAt(0,0,0)`) and is *never* updated in `animate()`. The Rust example hard-codes
  `position (−5, 2.5, −3.5)`, `lookAt(0,0,0)`, fov 40, near 1, far 100.
* **The point light is a child of the camera**, so `camera.matrixWorld` has to be current before the
  light is collected. `scene.add( camera )` puts it in the tree, which is enough.
* **`timer.getDelta()` is 0 on the graded frame.** The harness pins `performance.now()` to 0, so
  `mixer.update( 0 )` — the clip at t = 0. The oracle's `atT0` block is exactly that state.
* **`clip.optimize()` is not a no-op.** Two of the four tracks drop from 879 to 690 keyframes
  (`circle1.quaternion`, `circle2.quaternion`); the other two are unchanged. The port has
  `AnimationClip::optimize()` (`src/animation/animation_clip.rs:368`) — it must be called, and the
  oracle records the track shapes both ways so a mistake here is caught before the pixels.
* **No `Math.random()` draws.** `grep -c Math.random` is 0 in both `jsm/inspector/Inspector.js` and
  `jsm/libs/lil-gui.module.min.js`. `Renderer::skip_random_draws` stays at 0.
* Nothing branches on `window.TESTING`. `toInspector()` is a no-op for rendering.
* Tone mapping `Reinhard`, exposure 1 (`ToneMapping::Reinhard` exists, rung 7/8).

### The asset: `models/gltf/PrimaryIonDrive.glb`, 5 800 408 bytes

Read out of the GLB's JSON chunk and out of the oracle:

* `extensionsUsed: []`, `extensionsRequired: []` — **no KHR extension at all**, no Draco, no
  meshopt, no KTX2. This is the whole point of the pick: it is pure core-loader work.
* **0 textures, 0 images, 0 samplers, 0 skins, 0 cameras.** No PNG/JPEG decode is on the critical
  path, which keeps the rung away from `src/textures/texture.rs`.
* 19 glTF nodes → 20 `Object3D`s (`OSG_Scene` group + 19). The tree, from the oracle:

  ```
  OSG_Scene (Group)
   └ RootNode_(gltf_orientation_matrix)      matrix [1,0,0,0, 0,0,-1,0, 0,1,0,0, 0,0,0,1]
     └ RootNode_(model_correction_matrix)
       └ f10517d4966d42c99c9bc47c460a132ffbx
         └ ""                                (an unnamed node)
           └ RootNode
             ├ circle   ├ Mesh circle_constant1_0      (constant1,     57600 idx, 33080 pos)
             │          └ Mesh circle_HoloFillDark_0   (HoloFillDark,   4800 idx,  3200 pos)
             ├ geo1     ├ Mesh geo1_constant1_0        (constant1,     75522 idx, 45022 pos)
             │          └ Mesh geo1_HoloFillDark_0     (HoloFillDark,   5430 idx,  2208 pos)
             ├ cam1     └ "" (unnamed)
             ├ circle1  └ Mesh circle1_constant2_0     (constant2,     10320 idx,  6880 pos)
             ├ circle2  └ Mesh circle2_constant2_0     (constant2,      2544 idx,  1408 pos)
             └ cam2     └ "" (unnamed)
  ```

  Two unnamed nodes and a six-deep chain of pure-transform `Object3D`s: this exercises
  `createUniqueName` / `sanitizeNodeName` and the matrix-vs-TRS branch, both of which rung 10 ported.
* **Attributes per primitive:** `POSITION` (VEC3 f32, bufferView stride **12**), `NORMAL` (VEC3 f32,
  stride 12), `COLOR_0` (**VEC4** f32, bufferView stride **16**), `TANGENT` (VEC4 f32, stride 16 —
  present on five of the six primitives, absent on `circle2_constant2_0`), `indices` SCALAR
  **UNSIGNED_INT** (5125 → `Uint32Array`; rung 10's "u16 when it fits" path must *not* narrow these,
  75 522 indices over 45 022 vertices is fine for u16 by count but the dump confirms `uint32`).
  Two `bufferView`s carry `byteStride`, so the interleaved accessor path rung 10 ported is used.
* **Three materials**, all `MeshStandardMaterial` after `loadMaterial` (no `ior`/`specular`
  extension, so *not* physical — note the port's `build_material` currently picks physical only when
  `ior`/`specular` is present, which is the right branch here):

  | name | color (linear) | metalness | roughness | emissive | side | transparent | opacity | depthWrite |
  |---|---|---|---|---|---|---|---|---|
  | `constant1` | 1, 1, 1 | 0 | 0.6 | 0.018924884 ×3 | `DoubleSide` | false | 1 | true |
  | `constant2` | 0.970067979, 0.321488876, 0 | 0 | 0.6 | 0.322027439, 0.290558949, 0 | `DoubleSide` | false | 1 | true |
  | `HoloFillDark` | 0, 0, 0 | 0.9224466463 | 0.7029344512 | 0, 0, 0 | `FrontSide` | **true** | **0.8092606707** | **false** |

  `HoloFillDark` is `alphaMode: "BLEND"`, which is where `transparent = true` **and**
  `depthWrite = false` come from (`GLTFLoader.loadMaterial`). All three get
  **`vertexColors = true`** from `assignFinalMaterial`, because every primitive has `COLOR_0`.

---

## 3. Dump reading (`dump/`)

`flock … node tools/dump-webgpu.mjs webgpu_postprocessing_bloom --out …` — worked first time. It
printed the same spurious `404 (Not Found)` page error every example produces today; the frame in
`dump/actual.jpg` is complete and matches the reference.

**14 shader modules, 11 render pipelines, 0 compute pipelines, 5 bind-group layouts, 16 textures,
1 sampler, 14 render passes, 14 submits** (one submit per pass).

| module(s) | lines | pipeline(s) | used in |
|---|---|---|---|
| `m00_vertex_constant1` | 82 | — | the vertex stage of **all six** scene draws |
| `m01_fragment_constant1` | 445 | `renderPipeline_constant1_21`, `renderPipeline_constant2_24` | 4 draws |
| `m02_fragment_HoloFillDark` | 443 | `renderPipeline_HoloFillDark_22` | 2 draws |
| `m03/m04_Bloom_highPass` | 42 / 45 | `renderPipeline_Bloom_highPass_26` | 1 pass |
| `m05_vertex_Bloom_separable` + `m06…m10` | 42 / 63 each | `…_separable_27…31` | 10 passes (H+V per mip) |
| `m11/m12_Bloom_comp` | 42 / 84 | `renderPipeline_Bloom_comp_32` | 1 pass |
| `m13_fragment_RenderPipeline` | 109 | `renderPipeline_RenderPipeline_25` | the canvas pass, `draw(3)` |

**Pass / submit order** (`dump.json`'s `order`): the canvas pass's encoder is opened first (`n=1`)
and ends last (`n=329`), exactly the `RenderPipeline` shape rung 9 documented in
`docs/postprocessing.md`. Between them, in order: pass 1 (the scene, 6 `drawIndexed`), pass 2
(high pass), passes 3–12 (H, V per mip, five mips), pass 13 (composite into `UnrealBloomPass.h0`),
then the canvas pass.

**Scene pass draw order** (pass 1, with the pipeline each draw was on):

| # | indexCount | pipeline | mesh |
|---|---|---|---|
| 1 | 75522 | `constant1_21` | `geo1_constant1_0` |
| 2 | 57600 | `constant1_21` | `circle_constant1_0` |
| 3 | 10320 | `constant1_21` | `circle1_constant2_0` |
| 4 | 2544 | `constant2_24` | `circle2_constant2_0` |
| 5 | 5430 | `HoloFillDark_22` | `geo1_HoloFillDark_0` |
| 6 | 4800 | `HoloFillDark_22` | `circle_HoloFillDark_0` |

The four opaque draws come first, then the two transparent ones — the port's `render_list.rs`
opaque/transparent split. Note the **pipeline naming is a red herring**: `circle1_constant2_0` draws
on the pipeline *labelled* `constant1_21` because `constant1` and `constant2` compile to byte-identical
WGSL (only uniform *values* differ) and Three caches the program; `circle2_constant2_0` gets its own
pipeline only because it has no `TANGENT` attribute. Do not try to reproduce the labels.

**Vertex buffer layout** — important, and easy to get backwards:

```
buffer 0: arrayStride 16, location 0 = float32x4   ← color   (COLOR_0, VEC4)
buffer 1: arrayStride 12, location 1 = float32x3   ← normal
buffer 2: arrayStride 12, location 2 = float32x3   ← position
```

`TANGENT` is parsed but **never bound**: no material here has a normal map, so the builder never
reads it. Locations follow the order the attributes are first touched in the generated vertex code
(`varyings.nodeVarying6 = color;` then `normalLocal = normal;` then `positionLocal = position;`),
not the glTF attribute order.

**Textures** (16):

| id | label | size | format | mips | samples |
|---|---|---|---|---|---|
| t0 | `depthBuffer` | 800×500 | `depth24plus` | 1 | 1 |
| t4 | `output` | 800×500 | `rgba16float` | 1 | 1 |
| t5 | `output-msaa` | 800×500 | `rgba16float` | 1 | **4** |
| t6 | `depth` | 800×500 | `depth24plus` | 1 | **4** |
| t19 | `DFG_LUT` | 16×16 | `rg16float` | 1 | 1 |
| t68 | `UnrealBloomPass.bright` | 400×250 | `rgba16float` | 1 | 1 |
| t80/t90 | `UnrealBloomPass.h0` / `.v0` | 400×250 | `rgba16float` | 1 | 1 |
| t94/t103 | `.h1` / `.v1` | 200×125 | `rgba16float` | 1 | 1 |
| t107/t116 | `.h2` / `.v2` | **100×62** | `rgba16float` | 1 | 1 |
| t120/t129 | `.h3` / `.v3` | **50×31** | `rgba16float` | 1 | 1 |
| t133/t142 | `.h4` / `.v4` | **25×15** | `rgba16float` | 1 | 1 |

The mip chain is `Math.floor( x / 2 )` at each step from 400×250 — 250→125→**62**→**31**→**15**, not
powers of two. Getting one of those off by one moves every blur tap.

**The scene pass's store ops come from the example's `config`:**

```
colorAttachments[0]: view = t5 (msaa), resolveTarget = t4, loadOp clear, storeOp "discard"
depthStencilAttachment: view = t6, depthLoadOp clear, depthStoreOp "discard"
```

`storeMultisampledColorBuffer: false` / `storeMultisampledDepthBuffer: false` becomes
`storeOp: "discard"` on both attachments while `resolveColorBuffer: true` keeps the resolve target.
The port's `PassNode` stores today; this is a new knob (gap item 7).

**One `GPUSampler` for the whole page**, the usual default:
`clamp-to-edge ×3, mag/min/mipmap nearest, lodMinClamp 0, lodMaxClamp 32, maxAnisotropy 1`.

**Bind-group layouts** (5):

| bgl | entries | who |
|---|---|---|
| 15 | `0: buffer(VERTEX\|FRAGMENT\|COMPUTE)` | group 0 (`bindGroup_render`) |
| 20 | `0: buffer(7)`, `1: sampler(FRAG)`, `2: texture(FRAG)` | the three scene materials (the texture is the `DFG_LUT`) |
| 72 | `0: sampler`, `1: texture`, `2: buffer` | high pass / separable blur quads |
| 148 | `0: buffer`, **`1: buffer(FRAG)`**, then five `sampler`+`texture` pairs | the composite quad — binding 1 is `uniformArray( bloomTintColors )` |
| 158 | `0: sampler`, `1: texture`, `2: sampler`, `3: texture` | the canvas quad (scene colour + bloom) |

**The scene fragment (`m01`, 445 lines) against rung 8's physical fragment** (466 lines): it is
*smaller*, because there is no ior/specular block. The deltas from what the port emits today are two,
both one line:

```wgsl
DiffuseColor  = ( vec4<f32>( object.nodeUniform0, 1.0 ) * nodeVarying6 );   // ← vertex colour
EmissiveColor = ( object.nodeUniform6 * vec3<f32>( object.nodeUniform7 ) ); // ← emissive × intensity
…
nodeVar135 = max( vec4<f32>( ( outgoingLight + EmissiveColor ), DiffuseColor.w ), vec4<f32>( 0.0 ) );
```

`m02` (HoloFillDark) differs from `m01` in exactly three lines, which the rung worker can use as a
free check on the transparent/`FrontSide` path:

```
< @builtin( front_facing ) isFront : bool          (m01, DoubleSide)
< DiffuseColor.w = 0.8092606707;                   (m02 only — opacity < 1)
< NORMAL_normalView = ( normalViewGeometry * … faceDirection … )   vs   = normalViewGeometry
```

`m06`'s separable blur is the one to read closely: the Gaussian taps are **baked const arrays**
(`array<f32,3>( 1.407…, 3.294…, 5.0 )` offsets and `array<f32,3>( 0.297…, 0.0917…, 0.00876… )`
weights for mip 0), emitted inline inside a `for ( var i : i32 = 0; i < 3; i ++ )`. The offsets come
from merging adjacent taps into bilinear fetches — but the page's single sampler is **nearest**, so
that "bilinear" merge lands on nearest taps; reproduce the arithmetic, not the intent. The five mips
use `kernelSizeArray = [ 6, 10, 14, 18, 22 ]` → 3, 5, 7, 9, 11 loop iterations respectively
(`m06`…`m10`, 63 lines each — they differ only in the two const arrays and the loop bound).

`m12` (composite) sums five `lerpBloomFactor( bloomFactors[i], radius ) * vec4( tint[i], 1 ) * blur[i]`
terms and multiplies by `strength`; `bloomFactors` is an inline `array<f32,5>( 1.0, 0.8, 0.6, 0.4, 0.2 )`
and `bloomTintColors` is a real **uniform buffer** (`NodeBuffer_1191.value[ i ].xyz`) of five vec4s.

`m13` is `renderOutput`: `textureSample(scene) + textureSample(bloom)` → unpremultiply →
`reinhardToneMapping( …, render.nodeUniform2 )` → `sRGBTransferOETF` → premultiply, into an
`rgba8unorm` canvas target. Structurally identical to rung 9's `RenderPipeline` fragment with
`ToneMapping::Reinhard` selected.

**Not captured by the dump tool:** `setViewport` / `setScissorRect` are still not hooked (the
PMREM-A scout asked for this too), and `queue.writeTexture` records `size.width` as 0. Neither
matters here — every pass covers its whole target — but the hook is still worth the two minutes.

---

## 4. Gap list against the port

Port read at `rung-pmrem-a` @ `f949c0e` (rungs through PMREM-A: 16 graded examples,
`docs/webgpu_pmrem_cubemap-progress.md` for the current tip).

### Have

| thing | Rust item |
|---|---|
| GLB container, buffers/bufferViews/accessors (incl. `byteStride`, sparse, every component type), node tree, `createUniqueName`/`sanitizeNodeName`, matrix-vs-TRS | `src/loaders/gltf_loader.rs` (1 502 lines, rung 10) |
| `materials`/`textures`/`images` parsed into `GltfMaterial` / `GltfTexture` / `GltfImage` records — **including `alphaMode`, `alphaCutoff`, `doubleSided`, `emissiveFactor`, `occlusionTexture`, `emissiveTexture`** | `gltf_loader.rs:155`, `load_materials()` |
| `build_material()` → `MeshStandard` / `MeshPhysical`, `map`/`normalMap`/`metalnessMap`/`roughnessMap`/`specularColorMap`, the `useDerivativeTangents` clone | `gltf_loader.rs:1191` |
| glTF animations → `AnimationClip` via `PATH_PROPERTIES`; `AnimationMixer`, `SceneResolver`, `NodeTarget` for `.position`/`.quaternion`/`.scale` | `src/animation/`, `src/animation/object3d_target.rs` |
| `AnimationClip::optimize()` and `KeyframeTrack::optimize()` | `src/animation/animation_clip.rs:368`, `keyframe_track.rs:477` |
| `MeshStandardNodeMaterial` with the physical lighting model + `DFG_LUT`, `vertexColors`, `emissive` | `src/materials/`, `node_material.rs` |
| `AmbientLight`, `PointLight` (incl. as a child of the camera) | `src/lights/` |
| Transparent/opaque sort, `depthWrite`, blending, `Side::Double` + `faceDirection` | `src/renderer/render_list.rs`, `src/materials/blending.rs`, rung 10's `negateOnBackSide` |
| `ToneMapping::Reinhard` + exposure | rung 7/8 |
| `PassNode` (`rgba16float` target + own depth), `RenderPipeline`, `QuadMesh`, `renderOutput` | `src/renderer/pass.rs`, `render_pipeline.rs`, rung 9 |
| Half-float render targets, MSAA + resolve, render-target *viewport*, `read_target_pixels_rgba16f()` | `src/renderer/render_target.rs`, PMREM-A |
| `luminance`, `smoothstep`, `mix`, `dot`, `loop_statement` / `loop_n`, `if_statement`, `texture()` with its `mat3x3` uv transform, `PassTextureNode` | `src/nodes/tsl.rs`, `src/nodes/display/` |
| A multi-target multi-pass display node as precedent | `src/renderer/ssaa_pass.rs` (383 lines), `src/nodes/display/radial_blur.rs` (125) |

### Missing — owned by this rung

| # | thing | Three source | lands in | JS to read | rough Rust | kind |
|---|---|---|---|---|---|---|
| 1 | **Non-skinned glTF primitives get a `Payload::Mesh`.** Today `Gltf` builds a material and a payload **only** for primitives with a `skin` (`gltf_loader.rs:665`, `let Some(skin) = primitive.skin else { continue };`); everything else stays a bare `GltfPrimitive` record, so `scene.add( gltf.scene )` draws nothing. This is *the* capability gap. | `GLTFParser.loadMesh` / `createNodeMesh` | `src/loaders/gltf_loader.rs` | ~120 | ~130 | loader |
| 2 | **`assignFinalMaterial`** — the material-variant clone: `vertexColors` when the geometry has `color`, `flatShading` when it has no `normal`, `useDerivativeTangents` when it has no `tangent`; and the per-variant material cache so two primitives sharing a glTF material share one Rust material. | `GLTFLoader.assignFinalMaterial` | `src/loaders/gltf_loader.rs` | ~70 | ~100 | loader |
| 3 | **`alphaMode` wiring** — `BLEND` → `transparent = true`, `depthWrite = false`; `MASK` → `alphaTest = alphaCutoff`; and `baseColorFactor[3]` → `opacity` (already set). Parsed into `GltfMaterial` today, never read. | `GLTFParser.loadMaterial` | `src/loaders/gltf_loader.rs` | ~25 | ~30 | loader |
| 4 | **`emissiveFactor` → `material.emissive`** and `emissiveIntensity` on the built material (the field exists on `Material`, the loader never sets it). | same | `src/loaders/gltf_loader.rs`, `src/materials/mod.rs` | ~15 | ~25 | loader/material |
| 5 | **`uniformArray`** — a fragment-visible uniform **buffer** of `vec4`s, addressed as `NodeBuffer_N.value[ i ].xyz`, its own bgl entry (bgl 148 binding 1). The port has no array-uniform node. | `src/nodes/accessors/UniformArrayNode.js` (~250) | `src/nodes/tsl.rs`, `src/nodes/node.rs`, `src/nodes/builder.rs` | 250 | ~180 | node system |
| 6 | **Const `array()` node** — `array< f32, N >( … )[ i ]`, indexed by a loop variable, emitted inline. Needed three times (blur offsets, blur weights, `bloomFactors`). | `src/nodes/utils/ArrayNode.js` (~150) | `src/nodes/tsl.rs`, `src/nodes/wgsl.rs` | 150 | ~90 | node system |
| 7 | **Pass store/resolve config** — `storeMultisampledColorBuffer` / `…DepthBuffer` / `resolveColorBuffer` on `pass( scene, camera, config )` → `storeOp: "discard"` on both attachments with the resolve kept. | `PassNode`, `RenderContext` | `src/renderer/pass.rs`, `src/renderer/render_target.rs` | ~60 | ~60 | renderer |
| 8 | **`BloomNode`** — 11 `rgba16float` targets on a floored ÷2 chain, the high-pass material, five separable-blur materials with baked Gaussian tap tables, the composite material, and a `updateBefore` that renders 12 quad passes into them before the canvas pass records. | `examples/jsm/tsl/display/BloomNode.js` (599, ~350 non-doc) | `src/nodes/display/bloom.rs` (new) | 599 | ~450 | addon / display node |
| 9 | **`RendererUtils.resetRendererState` / `restoreRendererState`** — Bloom's `updateBefore` saves and restores the renderer's target, tone mapping, colour space, scene/camera and MRT around its 12 passes. Rung 9 does the equivalent by hand in `RenderPipeline::render`; this needs it as a reusable piece. | `src/renderers/common/RendererUtils.js` (~130) | `src/renderer/mod.rs` | 130 | ~90 | renderer |
| 10 | **The example + the e2e row + the WGSL dump blocks** | — | `examples/webgpu_postprocessing_bloom.rs`, `tests/e2e/main.rs`, `examples/dump_wgsl.rs`, `src/bin/viewer.rs` | — | ~180 | — |

**Total ≈ 1 335 lines of Rust** over ~1 300 lines of JS read. Items 1–4 (the loader capability, ~285
lines) are the point of the rung; items 5–9 (~780) are the price of admission; item 10 is the gate.

**Paths every other example touches** (merge-conflict / regression risk, flag hard):

* **`src/loaders/gltf_loader.rs`** — items 1–4. Rung 10's `webgpu_skinning` (6/100000) runs through
  the same `build_material` and the same primitive loop. Item 2 in particular changes the material
  *variant* logic that produced rung 10's `normalScale.y = -1`; if `assignFinalMaterial` is written
  as a rewrite rather than an extension, Michelle's lighting inverts silently. **Gate `webgpu_skinning`
  before and after every commit in items 1–4.**
* **`src/renderer/pass.rs`** — item 7 adds a config to the node rung 9, radial-blur and SSAA all use.
* **`src/renderer/mod.rs`** — item 9 factors the save/restore that `RenderPipeline::render` and
  `SsaaPass` both do today; it is the same entry point PMREM-B will want for its pre-pass hook.
* **`src/nodes/builder.rs` / `wgsl.rs`** — items 5 and 6 add two emission forms. Any change to
  binding numbering there moves the WGSL of *every* rung.

### Explicitly NOT in this rung

Textures from glTF images beyond what rung 10 has (this asset has none), `KHR_texture_transform`,
`texCoord > 0`, primitive dedup via `createPrimitiveKey`, geometry `groups`, `Points`/`Line`
primitive modes, glTF cameras and lights, `GLTFMeshStandardSGMaterial`, `CUBICSPLINE`, Draco,
meshopt, KTX2, and every `KHR_materials_*` BSDF. None is reachable from this asset; see
`GLTFLOADER.md` for which example forces each one.

---

## 5. Gates beyond the pixel diff

In order of how early they fire.

### 5.1 The glTF oracle — `primaryiondrive.json` (dumped, 33 KB)

This is the numeric gate the brief asks for: Three's own `GLTFLoader` parse of the asset, recorded
from node with no GPU and no network by `oracle.mjs` in this directory. Format:

```jsonc
{
  "file": "models/gltf/PrimaryIonDrive.glb",
  "asset": { … },                       // gltf.asset verbatim
  "sceneName": "OSG_Scene",
  "sceneChildren": [ "RootNode_(gltf_orientation_matrix)" ],
  "nodeCount": 20,
  "nodes": [                            // scene.traverse() order — the order itself is asserted
    { "name": "…", "type": "Group|Object3D|Mesh", "parent": "…",
      "visible": true, "castShadow": false, "receiveShadow": false, "renderOrder": 0,
      "position": [x,y,z], "quaternion": [x,y,z,w], "scale": [x,y,z],
      "matrix": [ 16 ], "matrixWorld": [ 16 ],          // 9-dp rounded
      "geometry": {                                      // Mesh only
        "name": …, "groups": [], "drawRange": {…}, "morphAttributes": {},
        "attributes": {
          "color":    { "itemSize": 4, "count": 33080, "normalized": false,
                        "array": "Float32Array", "hash": <FNV-1a-32 over the bytes>,
                        "first": [ 8 values ] },
          "tangent": …, "normal": …, "position": …,
          "index":    { "count": 57600, "array": "Uint32Array", "hash": …, "first": [ 6 ] }
        } },
      "material": { "name","type","side","transparent","opacity","depthWrite","alphaTest",
                    "blending","vertexColors","flatShading","toneMapped","color","metalness",
                    "roughness","emissive","emissiveIntensity","ior","specularIntensity",
                    "specularColor","normalScale",
                    "map|normalMap|roughnessMap|metalnessMap|emissiveMap|aoMap|specularColorMap":
                        null | { colorSpace, flipY, wrapS, wrapT, magFilter, minFilter, channel } } }
  ],
  "animations":          [ { name, duration, blendMode, tracks: [ { name, type, valueSize, times,
                             interpolation, timesHash, valuesHash, time0, timeN, values0 } ] } ],
  "animationsOptimized": [ … same shape, after clip.optimize() … ],
  "atT0":                [ { name, matrixWorld } × 20 ]   // mixer on the optimized clip, update(0)
}
```

`hash` is FNV-1a-32 over the raw bytes of the typed array — the same "hash plus probes" trick
PMREM-A's `tests/hdr/oracle.json` uses, so the file stays 33 KB instead of 30 MB and a failure still
says *where* via `first`. The obvious Rust test file is `tests/gltf_primary_ion_drive.rs`, alongside
rung 10's `tests/gltf_loader.rs`, asserting in this order:

1. `nodeCount`, then the traverse-order list of `(name, type, parent)` — catches the node tree and
   the unnamed-node naming before anything else.
2. Per node: `position`/`quaternion`/`scale` and `matrix`/`matrixWorld` to 1e-6.
3. Per mesh: every attribute's `itemSize`/`count`/array type and its byte hash; the index type
   (**`Uint32Array`, not narrowed**) and hash.
4. Per mesh: the whole material record — this is what items 2–4 are gated on. `vertexColors: true`
   on all three, `transparent`/`depthWrite`/`opacity` on `HoloFillDark`, `side: 2` on the other two,
   `emissive` on `constant1`/`constant2`.
5. `animations` vs `animationsOptimized`: 4 tracks, `circle1`/`circle2` dropping 879 → 690 times.
6. `atT0`: the 20 `matrixWorld`s after `mixer.update( 0 )` on the *optimized* clip.

**Re-run it with `node oracle.mjs > primaryiondrive.json` from this directory.** It reads
`GLTFLoader.js` and `three.module.js` straight out of the vendor tree by absolute path and writes
nothing there.

### 5.2 WGSL diff

`examples/dump_wgsl.rs` should grow four blocks — `bloom_scene` (the `constant1` pair),
`bloom_highpass`, `bloom_separable` (mip 0) and `bloom_comp` — diffed line-for-line against `m00/m01`,
`m03/m04`, `m05/m06`, `m11/m12`. `m01` at 445 lines against rung 8's 466 is a near-subset, so the
vertex-colour and emissive lines are a three-line diff, and `m02` vs `m01` is a free check on the
transparent/`FrontSide` path (§3). The blur's two const tap arrays are literal numbers in the dump
and must match to the digit.

### 5.3 Readbacks

Every bloom target is a plain `rgba16float` texture and PMREM-A already landed
`read_target_pixels_rgba16f()`. Three cheap assertions without an oracle:

1. With `threshold = 0` and `smoothWidth = 0.01`, the high-pass output equals the scene colour
   scaled by `smoothstep( 0, 0.01, luminance )` — i.e. for any texel whose luminance is > 0.01 it is
   *exactly* the input. Read back `UnrealBloomPass.bright` and compare against a readback of the
   scene pass target.
2. Each blur target's mean is within ~1e-3 of its input's mean (the tap weights sum to
   `centerWeight + 2·Σw` ≈ 1 by construction). Catches a wrong kernel table.
3. The mip chain's sizes: 400×250, 200×125, 100×62, 50×31, 25×15. Assert them as integers before
   rendering anything — the `floor` is the single most likely silent error.

### 5.4 Unit tests

* The Gaussian tap table is a pure function of `kernelRadius`
  (`sigma = r/3`, `c[i] = 0.39894·exp(−0.5·i²/σ²)/σ`, then the pairwise merge). Table-test the five
  radii `[6, 10, 14, 18, 22]` against the numbers in `m06`…`m10`. No GPU.
* `lerpBloomFactor( factor, radius ) = mix( factor, 1.2 − factor, radius )` — five values at
  `radius = 0` (the example's setting) is a two-line test.
* `AnimationClip::optimize()` against `animationsOptimized` — the port already has the function;
  this asset is the first thing on the ladder that calls it.

### 5.5 QUnit

`test/unit/three.source.unit.js` has no `BloomNode` and no `GLTFLoader` entry in r186, so there is no
upstream unit test to borrow for either half.

### 5.6 Regression gate

**`webgpu_skinning` (6/100000) is the canary for items 1–4** and must be run after every loader
commit. `webgpu_postprocessing_masking` (18), `webgpu_postprocessing_radial_blur` (7) and
`webgpu_postprocessing_ssaa` (0) are the canaries for items 7 and 9.

---

## 6. Order of work

Each step leaves the ladder green (`cargo test --test e2e -- --nocapture --test-threads=1`).

0. **Two minutes of tooling.** Hook `setViewport`/`setScissorRect` in `tools/dump-webgpu.mjs` and fix
   the `writeTexture` size field. (Asked for by the PMREM-A scout too; do it once.)
1. **The oracle test.** Copy `oracle.mjs`/`primaryiondrive.json` into `tests/gltf/` and write
   `tests/gltf_primary_ion_drive.rs` asserting §5.1 items 1–3 and 5 — the parts that already pass.
   *Gate:* green with no library change, which proves the oracle is right before it is used to judge
   new code.
2. **Items 1–4: the loader capability.** `Payload::Mesh` for non-skinned primitives,
   `assignFinalMaterial`'s variant clone and cache, `alphaMode`, `emissive`. *Gate:* §5.1 item 4 (the
   full material records) turns green, **and `webgpu_skinning` is still 6/100000 with its WGSL
   unchanged.** No renderer work yet.
3. **Items 5–6: `uniformArray` and const `array`.** Two node types, gated by WGSL emission unit
   tests (`array< f32, 3 >( … )[ i ]` inside a `for`, and `NodeBuffer_N.value[ i ]` with its bgl
   entry). *Gate:* unit tests; ladder unchanged.
4. **Item 7 + item 9: pass config and renderer save/restore.** `pass( scene, camera, config )` with
   `storeOp: discard`, and `reset/restoreRendererState` factored out of `RenderPipeline::render`.
   *Gate:* `webgpu_postprocessing_masking`, `_radial_blur` and `_ssaa` all unchanged.
5. **Item 8a: the high-pass and the target chain.** Allocate the 11 targets, assert their sizes
   (§5.3.3), run the one high-pass quad. *Gate:* `m04`'s 45 lines match; readback §5.3.1.
6. **Item 8b: the separable blur.** The tap tables (§5.4), the five materials, the ten passes.
   *Gate:* `m06`…`m10` match line-for-line; readback §5.3.2.
7. **Item 8c: the composite + `updateBefore` ordering.** *Gate:* `m12` matches and the port's own
   dump shows 14 passes / 14 submits in `dump.json`'s order.
8. **Item 10: the scene.** `examples/webgpu_postprocessing_bloom.rs` — the glTF, the two lights, the
   camera-parented point light, `Reinhard`, `mixer.update( 0 )` on the optimized clip. *Gate:*
   `m00/m01/m02` match; §5.1 item 6 (`atT0`) matches; then the grader.

Steps 1–2 are the loader rung and are independent of 3–7; if the sitting runs long, stopping after
step 2 still lands the capability with a numeric gate and no image.

---

## 7. What this rung unlocks

See `GLTFLOADER.md` for the full table. In summary: 59 of Three's 230 `webgpu_*` examples import
`GLTFLoader`, 47 of them off the exception list. This rung lands the core loader path (non-skinned
meshes, material variants, alphaMode, emissive) that **all 47** need, plus `uniformArray`, const
`array` and the pass store/resolve config.

The immediate cheap follow-ups, once this lands:

| next example | extra cost on top of this rung | what it adds |
|---|---|---|
| `webgpu_postprocessing_godrays` (0.0%) | `GodraysNode` + `BilateralBlurNode` + `depthAwareBlend` | a depth-aware post chain |
| `webgpu_portal` (not graded) | `pass()` of a second scene, `mx_worley`/`mx_fractal_noise`, transparent quad | nested scene passes |
| `webgpu_tsl_halftone` | — (but it grades 0.1%: **never a rung**) | — |
| `webgpu_shadowmap_opacity` (0.0%) | KHR_materials_transmission/_volume, a transmission pass, AgX | the transmission family |
| `webgpu_mrt_mask` (0.0%) | MRT in `PassNode`, `GaussianBlurNode` | MRT — which also makes *selective* bloom work |

`BloomNode` itself is reused directly by `webgpu_postprocessing_bloom_emissive`,
`webgpu_postprocessing_lensflare` and `webgpu_volume_caustics` (all of which additionally need the
environment work PMREM-B is doing, or Draco).

---

## 8. Verdict

**`webgpu_postprocessing_bloom`: a rung, and a well-shaped one — one Opus worker, one sitting.**
~1 335 lines of Rust, of which the actual subject (the loader capability) is ~285 and gated entirely
by a 33 KB CPU oracle that needs no GPU. The remaining ~1 050 lines are a self-contained display node
plus two node types, gated by four WGSL diffs and three readbacks. The natural split, if it runs
long, is after step 2.

It is the right *first* gate for GLTFLoader because it is the only 0.0%, local-asset, exception-list-free
example whose entire scene comes out of a glTF file with **no extension, no compression, no texture and
no environment** — every other candidate pays for a second capability before the loader is exercised at
all.

Risks, in order:

1. **`assignFinalMaterial` regressing rung 10.** Items 1–4 rewrite the path that produced
   `webgpu_skinning`'s 6/100000, including the `useDerivativeTangents` clone whose absence inverts
   Michelle's lighting *silently*. Extend the existing function; do not replace it; run
   `webgpu_skinning` after every commit in step 2.
2. **The blur mip chain's `floor`.** 400×250 → 200×125 → 100×62 → 50×31 → 25×15. A `ceil`, a
   power-of-two rounding, or a `>> 1` on an odd number gives a plausible, slightly-wrong glow and
   nothing else complains. Assert the sizes as integers (§5.3.3) before rendering.
3. **`uniformArray`'s bind-group slot.** It is binding **1** of bgl 148, between the object uniform
   buffer and the first sampler, `FRAGMENT`-visible only. Getting the slot or the visibility wrong
   shifts every later binding in that layout and the failure shows up as a wrong texture, not as an
   error. Gate it on the bgl in `dump.json`, not on the image.
