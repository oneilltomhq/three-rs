# Environment-family scout — the first environment rung after PMREM B: `webgpu_pmrem_test`

Scouted 2026-09-19. Vendor tree `~/src/vendor/three.js` @ r186 with
`rung0/grader-flags.patch` applied; nothing in it was modified. Port read at
`/home/tom/src/projects/three-rs/rung-pmrem-a` (stack tip `f949c0e`), read-only.
Assumes `rung-pmrem-b` (PMREMGenerator core + `fromCubemap` + cube-UV sampling
nodes + `webgpu_pmrem_cubemap` green) lands first; every "have (B)" below is a
claim about that branch, not about `main`.

**Headline: the pick is `webgpu_pmrem_test`, and `fromEquirectangular` is the
smallest possible delta on top of `fromCubemap` — one TSL `Fn` of six lines and
one extra `NodeMaterial`.** Everything else PMREM B has to build (the cube-UV
atlas, the 20-pass GGX chain, `textureCubeUV`, `PMREMNode`, `EnvironmentNode`,
`envMap` on `MeshPhysicalNodeMaterial`, ACESFilmic) is shared verbatim: the two
examples' dumps differ in exactly one shader module. The real new work in this
rung is not PMREM at all — it is the 2-D half-float **equirect source texture**:
`HDRLoader`'s `.load()` (not `.parse()`, which `rung-pmrem-a` already ported),
its `flipY = true`, and `scene.background` taking a cube-UV render-target
texture.

---

## 1. Grade confirmation

`webgpu_pmrem_test` is **not** on `exceptionList` in `test/e2e/puppeteer.js`.

```
cd ~/src/vendor/three.js && flock /run/user/1000/three-rs-gpu.lock \
  npm run test-e2e-webgpu -- webgpu_pmrem_scene webgpu_furnace_test \
     webgpu_instance_path webgpu_materials_envmaps \
     webgpu_materials_cubemap_mipmaps webgpu_pmrem_test
```

Two independent runs, full log in `e2e-env-candidates-2026-09-19.log`:

| example | run 1 | run 2 | exception list |
|---|---|---|---|
| **webgpu_pmrem_test** | **0.0%** | **0.0%** | no |
| webgpu_furnace_test | 0.0% | 0.0% | no |
| webgpu_pmrem_scene | 0.0% | 0.0% | no |
| webgpu_instance_path | 0.0% | 0.0% | no |
| webgpu_materials_envmaps | 0.0% | 0.0% | no |
| webgpu_materials_cubemap_mipmaps | 0.0% | 0.0% | no |

Six for six, twice, no retries needed. All five siblings are live options; §7
and `ENVIRONMENT-FAMILY.md` say why `webgpu_pmrem_test` goes first.

### 1.1 Why this one and not the `fromScene` pair

`fromScene` unlocks more examples (16 gradable ones use it, §7) but it is
strictly bigger: it renders a *scene* six times through the renderer's own
render path into viewport-tiled slices of the atlas, with `autoClear` false, a
`BackSide` background box, and — for `RoomEnvironment` — a `PointLight`, an
`InstancedMesh` and `MeshLambertMaterial`. `fromEquirectangular` renders one
36-vertex mesh with one new fragment shader. Do the cheap one first, land the
atlas-writing-from-a-2D-source path, then do `fromScene` against
`webgpu_furnace_test` (§6.8), whose env scene is a solid colour and nothing
else — the minimal `fromScene`.

---

## 2. What the example does

Source: `examples/webgpu_pmrem_test.html`, 150 lines. Imports `three` (which is
`Three.WebGPU.js` in this checkout), `OrbitControls`, `HDRLoader`, `Inspector`.

### 2.1 Setup

* `WebGPURenderer( { antialias: true } )`, `setPixelRatio( devicePixelRatio )`,
  `setSize( 800, 500 )` under the grader viewport, `setAnimationLoop( render )`.
* `renderer.toneMapping = ACESFilmicToneMapping`, `toneMappingExposure = 1`.
* `await renderer.init()`, then `createObjects()` from a promise chain.
* `Scene` with **no** `background` at init; `scene.background` is assigned the
  PMREM texture inside the loader callback.
* `PerspectiveCamera( 40, aspect, 1, 30 )` at `(0,0,16)`, then `updateCamera()`
  reinterprets the 40 as a **horizontal** FoV:
  `fov = 2*atan( tan( 40/2 * π/180 ) / aspect ) * 180/π`. At 800×500 (aspect
  1.6) that is `fov = 25.5115…`. Get this exactly right or every sphere is the
  wrong size.
* `OrbitControls` constructed, `minDistance`/`maxDistance` set, never moved
  (no `update()` call, no damping) — inert for the graded frame, as in rung 13.
* `DirectionalLight( 0xffffff, 0 )` — **intensity zero**, positioned via
  `setFromSphericalCoords( 100, -phi, π/2 - theta )` with
  `theta = 597.5 * π/512`, `phi = 213.5 * π/512`. It contributes nothing to the
  image but it **is** in the node graph and in the WGSL (§3.4). Do not optimise
  it away; the uniform buffer layout depends on it.
* `Inspector` + a GUI toggle; `clean-page.js` removes the panel and the
  `onChange` never fires, so `envMapIntensity` stays 1 and the light stays 0.

### 2.2 The environment

```js
new HDRLoader().setPath( 'textures/equirectangular/' )
    .load( 'spot1Lux.hdr', function ( texture ) {
        radianceMap = pmremGenerator.fromEquirectangular( texture ).texture;
        pmremGenerator.dispose();
        scene.background = radianceMap;
        …
    } );
const pmremGenerator = new THREE.PMREMGenerator( renderer );
pmremGenerator.compileEquirectangularShader();
```

* `spot1Lux.hdr` is **38 998 bytes**, 1024×512 Radiance RGBE — a black image
  with a single bright texel (27 490 nits) at `(597, 213)`. The whole asset is
  the size of a small PNG; there is no network cost and no glTF anywhere.
* `HDRLoader extends DataTextureLoader`, `this.type = HalfFloatType`. Its
  `parse()` returns `{ …, colorSpace: LinearSRGBColorSpace, minFilter: Linear,
  magFilter: Linear, generateMipmaps: false, flipY: true }`
  (`examples/jsm/loaders/HDRLoader.js:437-441`), and `DataTextureLoader`
  additionally sets `wrapS = wrapT = ClampToEdgeWrapping`, `anisotropy = 1`,
  `format`, `type`. **`flipY: true` is the one that has teeth** (§5.1).
  `texture.mapping` is left at `UVMapping`; `_fromTexture` takes the
  "equirectangular" branch for anything that is not a cube mapping, so nothing
  depends on the mapping constant here.
* `compileEquirectangularShader()` is called *before* the file arrives — under
  the grader it just builds `_equirectMaterial` early. The port can ignore it
  (it is a warm-up, and `_getEquirectMaterial( undefined )` then gets its
  texture assigned in `_textureToCubeUV`).
* `pmremGenerator.dispose()` runs immediately after `fromEquirectangular`; the
  returned render target survives.

### 2.3 The objects

`SphereGeometry( 0.4, 32, 32 )`, shared, and a 11×3 grid of
`MeshPhysicalMaterial`:

```js
for ( x = 0..10 ) for ( y = 0..2 )
  roughness: x/10,  metalness: y < 1 ? 1 : 0,
  color: y < 2 ? 0xffffff : 0x000000,
  envMap: radianceMap, envMapIntensity: 1
  mesh.position = ( x - 5, 1 - y, 0 )
```

33 meshes, 33 distinct materials, one geometry. Row y=0 metal white, row y=1
dielectric white, row y=2 dielectric black. No maps other than the envMap, no
transmission/clearcoat/sheen/iridescence, `ior` default 1.5.

### 2.4 Randomness and `window.TESTING`

**No `Math.random()` anywhere in the page, and no `window.TESTING` branch.**
`Renderer::skip_random_draws` is not needed. `performance.now` is pinned by
`deterministic-injection.js` but nothing in the page reads a clock — the frame
is static.

---

## 3. Dump reading

`flock … node tools/dump-webgpu.mjs webgpu_pmrem_test --out …/environment-family/dump-pmrem_test`
— ran first time; dump directory is 440 KB. (The tool printed one
`404 (Not Found)` page error; it is not an asset of this example — the HDR, the
shaders and the frame are all present and `actual.jpg` is the graded frame.)

**11 shader modules, 6 render pipelines, 0 compute, 25 render passes, 24
submits, 9 textures, 7 bind-group layouts, 827 recorded calls.**

### 3.1 Passes, in order

| pass | target | load | viewport | pipeline | draws |
|---|---|---|---|---|---|
| 1, 2 | temp `rgba16float` ↔ the 1024×512 source | clear | — | `mipmap-rgba16float-2d-array` | 1 each |
| 0 | `PMREM.cubeUv` 768×1024 | clear | `(0, 0, 768, 512)` | `PMREM_equirect` | 1 (36 verts) |
| 3–22 | `PMREM.cubeUv` / pingPong, alternating | **load** | see below | `PMREM_ggx` | 1 each |
| 23 | `-msaa` 800×500 + `depth24plus` | clear | — | `Background.material`, `MeshPhysicalMaterial` | **34** |
| 24 | swap-chain | load | — | `outputColorTransform` | 1 |

Chronology (the `order` log): passes **1 and 2 are submitted before pass 0**.
They are *not* mipmap generation — `generateMipmaps` is false on this texture.
They are `WebGPUTextureUtils._flipY()`, which borrows the mipmap blit pipeline
to render the source into a scratch texture and back, because
`texture.flipY === true` on the decoded HDR (`_copyBufferToTexture(…, flipY)`
→ `_flipY`). So the equirect image is **vertically flipped on the GPU before
anything samples it**. See §5.1.

GGX viewport ladder (`x, y, w, h`), each LOD written twice (render into
pingPong, copy back):

```
(0,512,384,256) ×2   (0,768,192,128) ×2   (0,896,96,64) ×2   (0,960,48,32) ×2
(48,960,48,32) ×2    (96,960,48,32) ×2    (144,960,48,32) ×2
(192,960,48,32) ×2   (240,960,48,32) ×2   (288,960,48,32) ×2
```

Ten LOD steps = 20 passes. `_setSize( image.width / 4 ) = 256` ⇒
`_lodMax = 8`, atlas `3·256 × 4·256 = 768×1024`, `_lodMeshes.length =
_lodMax - LOD_MIN + 1 + EXTRA_LODS = 11`. **Identical to the cubemap case**, so
if PMREM B's chain is right this part needs no work at all. Top-left viewport
origin, exactly as `docs/webgpu_pmrem_cubemap-progress.md` establishes.

### 3.2 Textures

| id | label | size | format |
|---|---|---|---|
| 0 / 1 / 2 | frame, `-msaa`, depth | 800×500 | rgba16float / rgba16float / depth24plus |
| 3 | `PMREM.cubeUv` | 768×1024 | rgba16float |
| 34 | `PMREM.cubeUv` (pingPong) | 768×1024 | rgba16float |
| 11 | the equirect source | 1024×512 | rgba16float |
| 17 | flipY scratch | (same desc) | rgba16float |
| 149 | `DFG_LUT` | 16×16 | rg16float |
| 254 | `depthBuffer` | 800×500 | depth24plus |

### 3.3 Bind-group layouts

| pipeline | group 0 | group 1 |
|---|---|---|
| `PMREM_equirect` | uniform (render) | sampler, `texture_2d`, uniform (object) |
| `PMREM_ggx` | uniform (render) | uniform, sampler, `texture_2d` |
| `Background.material` | uniform (render) | uniform, sampler, `texture_2d` |
| `MeshPhysicalMaterial` | uniform (render) | uniform, sampler+`texture_2d` (cubeUV), sampler+`texture_2d` (DFG LUT) |
| `outputColorTransform` | uniform (render) | sampler, `texture_2d`, uniform |

The physical material takes the PMREM as a plain **2-D** texture — cube-UV is
addressed arithmetically, there is no cube binding anywhere in this example.

### 3.4 The modules

`m00_vertex_fragment_mipmap.wgsl`, `m01/m02_*_PMREM_equirect.wgsl`,
`m03/m04_*_PMREM_ggx.wgsl`, `m05/m06_*_Background.material.wgsl`,
`m07_vertex_vertex.wgsl`, `m08_fragment_fragment.wgsl` (822 lines, the physical
material), `m09/m10_*_outputColorTransform.wgsl`.

The **entire** equirect fragment shader is:

```wgsl
nodeVar0 = normalize( nodeVarying4 );                        // outputDirection
nodeVar1 = textureSampleLevel( nodeUniform0, nodeUniform0_sampler,
    vec2<f32>( ( ( atan2( nodeVar0.z, nodeVar0.x ) * 0.15915494309189535 ) + 0.5 ),
               ( ( asin( clamp( nodeVar0.y, -1.0, 1.0 ) ) * 0.3183098861837907 ) + 0.5 ) ),
    0.0 );
output.color = nodeVar1;
```

That is `equirectUV( _outputDirection )` inlined, at explicit level 0. Its
vertex module is the same `outputDirection`-attribute + `modelViewMatrix` pair
the cubemap material uses — byte-identical apart from the varying index. The
cubemap counterpart (PMREM B) is the same shader with one `textureSampleLevel`
on a `texture_cube`. **This is the whole delta.**

In `m08_fragment_fragment.wgsl`: the directional light *is* present
(`render.nodeUniform14/15/16`, `nodeVar10 = pos - …`, `clamp(dot) * lightColor`
at lines 546-552) with its intensity-0 colour; `radiance` and `iblIrradiance`
are each accumulated as `… * object.nodeUniform23` — that `nodeUniform23` is
`envMapIntensity`. `irradiance` is zero (no ambient/hemisphere light).

---

## 4. Gap list against the port

"have (B)" = the PMREM-B branch is expected to provide it for
`webgpu_pmrem_cubemap`; "have (A)" = already on `rung-pmrem-a`.

| need | status | Three source | Rust landing site | size |
|---|---|---|---|---|
| cube-UV atlas target, pingPong, `_lodMeshes` (`outputDirection` attribute), `_setViewport`, `_applyPMREM` GGX chain | **have (B)** | `renderers/common/extras/PMREMGenerator.js` | `src/extras/pmrem_generator.rs` | 0 |
| `textureCubeUV` / `PMREMNode` / `EnvironmentNode` / material `envMap` | **have (B)** | `nodes/pmrem/*`, `nodes/lighting/EnvironmentNode.js` | `src/nodes/`, `src/materials/` | 0 |
| ACESFilmic tone mapping, `outputColorTransform`, MSAA, `DFG_LUT` | **have** | — | — | 0 |
| `HdrLoader::parse` (RGBE → half) + `DataUtils` | **have (A)** | `HDRLoader.js` | `src/loaders/hdr_loader.rs` | 0 |
| half-float 2-D `DataTexture` upload + `read_target_pixels_rgba16f` | **have (A)** | — | `src/textures/texture.rs` | 0 |
| render-target viewport, top-left origin | **have (A)** | — | `src/renderer/render_target.rs` | 0 |
| **`HDRLoader.load()`** — the `DataTextureLoader` wrapper: ClampToEdge wrap, Linear/Linear, `HalfFloatType`, `LinearSRGBColorSpace`, `generateMipmaps = false`, **`flipY = true`** | **missing** | `loaders/DataTextureLoader.js` (115-178, ~60 JS lines) + `HDRLoader.js:430-445` | `src/loaders/hdr_loader.rs` | ~50 Rust — loader |
| **`flipY` on a data-texture upload** | **missing** | `WebGPUTextureUtils._copyBufferToTexture` → `_flipY` | `src/renderer/mod.rs` (upload path) **or** CPU row-reverse in the loader | ~25 Rust — renderer, see §5.1 |
| **`equirectUV( dir )`** | **missing** | `nodes/utils/EquirectUV.js` (10 JS lines) | `src/nodes/tsl.rs` | ~15 Rust — node system |
| `atan2` / `asin` math ops | **missing** | — | `src/nodes/tsl.rs` (`math("atan2", …)` exists as a helper) | ~10 Rust — node system |
| `texture( map, uv, level )` on a 2-D texture with explicit level 0 | **probably missing** (`SampleMode` + `textureSampleLevel` exist in `builder.rs:1165`; `tsl::texture_uv` has no level arg) | `nodes/accessors/TextureNode.js` | `src/nodes/tsl.rs` | ~15 Rust — node system |
| **`_getEquirectMaterial()`** — `NodeMaterial`, `depthTest/Write` off, `NoBlending`, `fragmentNode = texture( env, equirectUV( outputDirection ), 0 )` | **missing** | `PMREMGenerator.js:942-948` | `src/extras/pmrem_generator.rs` | ~25 Rust — material |
| **`fromEquirectangular()` + the `_fromTexture` non-cube branch** (`_setSize( image.width / 4 )`, pick the equirect material, `_setViewport( 0, 0, 3·size, 2·size )`) | **missing** | `PMREMGenerator.js:205-230, 380-410, 553-590` | `src/extras/pmrem_generator.rs` | ~40 Rust — renderer/extras |
| **`scene.background = <cube-UV render-target texture>`** → `pmremTexture( background )` on the background sphere | **missing** (port's `Background` takes `Color`, `CubeTexture` or a node) | `renderers/common/nodes/NodeManager.js:713-717`, `Background.js:80-150` | `src/objects/scene.rs`, `src/materials/node_material.rs` | ~40 Rust — renderer |
| `MeshPhysicalMaterial` params used: `color`, `roughness`, `metalness`, `envMap`, `envMapIntensity` | **have (B)** for `envMap`; `envMapIntensity` (`materialEnvIntensity`, `nodeUniform23`) may be new | `nodes/accessors/MaterialProperties.js` | `src/materials/` | 0–15 Rust |
| `DirectionalLight` at intensity 0 in the graph | **have** | — | — | 0 |
| horizontal→vertical FoV conversion | example-local | — | `examples/webgpu_pmrem_test.rs` | ~5 |
| the example + e2e entry | — | — | `examples/webgpu_pmrem_test.rs`, `tests/e2e.rs` | ~120 |

**Total new Rust: roughly 250–350 lines**, of which maybe 90 are the example
itself. If PMREM B lands whole, this is a comfortable one-sitting rung — and
that is exactly why §6.8 hangs `webgpu_furnace_test`'s `fromScene` off the end
of it as a stretch.

### 4.1 Paths every other example touches

Two, and only two:

* **the texture-upload path** (`flipY` for buffer-sourced textures). Every
  ported example uploads textures. Whichever of the two options in §5.1 is
  chosen must leave `flipY == false` textures byte-identical — i.e. the flip
  must be gated on the flag, never applied unconditionally.
* **`Background`/`Scene`** gaining a third kind. `webgpu_materials_basic`,
  `webgpu_lights_physical` and the postprocessing rungs all set a background;
  adding a variant to the enum is a mechanical but wide edit.

`PMREMGenerator`, `equirectUV` and the equirect material are all additive and
cannot regress anything.

---

## 5. Traps, and gates beyond the pixel diff

### 5.1 `flipY` — the one that silently produces a plausible wrong image

`HDRLoader` sets `flipY: true`; `DataTextureLoader` copies it onto the texture;
the WebGPU backend honours it with two extra render passes *before* the PMREM
pass. If the port skips it, the environment is mirrored top-to-bottom — the
single bright texel of `spot1Lux.hdr` moves from above the spheres to below
them, every sphere still looks like a perfectly reasonable shiny sphere, and
the diff is large but the render "works". This is the equirect twin of the
cube-face orientation trap `rung-pmrem-a` documented.

Two implementations:

1. **CPU row-reverse at upload** when `texture.flip_y` is set, in the existing
   half-float upload. ~15 lines, no new pass, no new pipeline. The flip is an
   exact texel permutation, so the result is bit-identical to Three's blit
   (the blit samples texel centres of an equally sized target). This is a
   **divergence to record in `docs/nodes.md` §8** — the port would have 22
   passes where Three has 24, and two fewer submits.
2. **Port `_flipY`** (scratch texture + the mipmap blit pipeline, both
   directions). Matches the dump pass-for-pass; ~60 lines and a new pipeline
   the port does not otherwise need.

Recommendation: **(1)**, with the gate below making the choice safe.

**Gate (no image):** decode `spot1Lux.hdr` with the ported `HdrLoader`, upload
it as the loader's `.load()` would, and read it back with
`read_texture_pixels`/`read_target_pixels_rgba16f`. Assert that the one
non-black texel sits at `(597, 512 - 1 - 213) = (597, 298)` and that its value
is the RGBE-decoded 27 490-ish nits. One bright texel in a 1024×512 black image
is the perfect oracle: a missing flip, an off-by-one flip, or a row-stride bug
each move it somewhere provably wrong. Generate the expected value with a
`gen.mjs` beside the existing `tests/hdr/gen.mjs`, from the vendor's own
`HDRLoader.parse`, the way rung PMREM-A did.

### 5.2 The camera FoV

`updateCamera()` runs *before* `camera.position.set` and again on resize. At the
graded 800×500 the vertical FoV is `2·atan( tan( 20° ) / 1.6 )` = 25.5115…°, not
40°. Compute it the same way (degrees→radians→degrees) rather than hard-coding
a rounded number.

### 5.3 The intensity-0 directional light

It must exist in the light list so the physical fragment shader carries the
directional block and the render uniform buffer has `nodeUniform14/15/16`. A
port that drops zero-intensity lights would still render an identical image
here — but the WGSL diff, which is the port's primary gate, would not match.
Assert on the WGSL, not the pixels.

### 5.4 `compileEquirectangularShader()` and `dispose()`

`dispose()` is called before the first frame. Three keeps the returned render
target alive and only frees the generator's own scratch. A port that frees the
atlas in `Drop` renders black.

### 5.5 Explicit level 0

`_getEquirectMaterial` samples at level `0`, not with an implicit derivative
sample. With `generateMipmaps = false` and a `Linear` min filter the two happen
to agree here, but the WGSL must say `textureSampleLevel( …, 0.0 )` to match.

### 5.6 Gates the rung should ship

1. **WGSL fixture diff.** `examples/dump_wgsl.rs` for the equirect material
   against `dump-pmrem_test/m01_…` and `m02_…`; and the physical material
   against `m08_fragment_fragment.wgsl`. Copy only those into
   `tests/fixtures/webgpu_pmrem_test/`.
2. **The bright-texel readback** of §5.1.
3. **Atlas readback**: after `fromEquirectangular`, read back the 768×1024
   `rgba16float` atlas and assert the mip-0 face tiles are non-zero exactly
   where the source's bright texel projects, and that the 10 LOD tiles listed
   in §3.1 are non-zero. This is PMREM B's gate re-run through the new entry
   point and costs nothing extra.
4. **Pass census**: assert the render produces the expected pass/viewport
   sequence of §3.1 (minus the two flip passes if §5.1 option 1 is taken).
5. The e2e image gate against `examples/screenshots/webgpu_pmrem_test.jpg`
   (copied here as `webgpu_pmrem_test.jpg`, 12 KB).

---

## 6. Order of work

Each step leaves the ladder green.

1. **`equirectUV` + `atan2`/`asin` + `texture( map, uv, level )`.** Pure node
   system, no renderer change. Gate: a `dump_wgsl` unit test that prints the
   node and matches `m02_fragment_fragment_PMREM_equirect.wgsl`'s expression
   line for line. (~40 lines.)
2. **`HdrLoader::load()`** on top of the existing `parse()`: the
   `DataTextureLoader` property application, `flipY = true`. Gate: property
   assertions plus the existing `tests/hdr_loader.rs`. (~50 lines.)
3. **`flip_y` on the half-float 2-D upload** (§5.1 option 1). Gate: the
   bright-texel readback of §5.1, plus re-run the whole ladder to prove no
   `flip_y == false` texture moved. (~25 lines.)
4. **`_getEquirectMaterial` + `fromEquirectangular` + the `_fromTexture`
   non-cube branch.** Gate: the atlas readback (§5.6.3) and the pass census.
   (~65 lines.)
5. **`scene.background` accepting a cube-UV render-target texture**, routed to
   `pmremTexture( background )` with `backgroundBlurriness` as the level and
   `backgroundRotation` applied. Gate: the `Background.material` WGSL
   (`m06`). (~40 lines.)
6. **`envMapIntensity`** if PMREM B did not already need it (`nodeUniform23`).
7. **`examples/webgpu_pmrem_test.rs` + the e2e entry**: 33 materials, the
   horizontal-FoV camera, the intensity-0 directional light, ACESFilmic. Gate:
   the image. (~120 lines.)
8. **Stretch, only if steps 1–7 land early: `fromScene` via
   `webgpu_furnace_test`.** Its env scene is `new Scene()` with
   `background = new Color( 0xcccccc )` and *nothing else*, so `_sceneToCubeUV`
   reduces to: clear the atlas, build the `BackgroundBox`
   (`BoxGeometry` + `MeshBasicMaterial{ side: BackSide, depthWrite: false,
   depthTest: false }`, colour copied from the background), then for each of the
   six faces set the cube camera (`upSign = [1,1,1,1,-1,1]`,
   `forwardSign = [1,-1,1,-1,1,-1]`, fov 90, aspect 1, near/far 0.1/100),
   `_setViewport( col·256, i>2 ? 256 : 0, 256, 256 )` and render — with
   `autoClear` false so face *i* does not erase face *i-1*. That is
   `PMREMGenerator.js:448-545`, ~100 JS lines, maybe 120 Rust, and it needs
   only one renderer capability the port does not have: *render a scene into a
   viewport-restricted slice of a render target without clearing it*. The rest
   of `webgpu_furnace_test` is an 11×11 grid of `MeshPhysicalMaterial` over the
   same IBL path this rung already proves — and, because the environment is a
   uniform white furnace, it is the strongest numeric gate in the family
   (`ENVIRONMENT-FAMILY.md` §3). If it does not fit, it is the next rung.

---

## 7. What this rung unlocks

`fromEquirectangular` is the path *every* HDR-lit example takes, because
`scene.environment = <equirect texture>` funnels through
`NodeManager.updateEnvironment` → `EnvironmentNode` → `PMREMNode` →
`generator.fromEquirectangular` with no user-visible call. Off the exception
list and using a plain `.hdr` (no glTF, no UltraHDR):

| example | what else it needs |
|---|---|
| `webgpu_tsl_procedural_terrain` | `SunLight` addon, procedural TSL terrain |
| `webgpu_parallax_uv` | parallax UV nodes, 5 JPEG maps |
| `webgpu_tsl_wood` | `WoodNodeMaterial`, `FontLoader`/`TextGeometry` |
| `webgpu_cubemap_adjustments` | glTF (DamagedHelmet) |
| `webgpu_loader_gltf_dispersion` / `_iridescence` | glTF + those material extensions |
| `webgpu_tonemapping`, `webgpu_postprocessing_bloom_emissive`, `webgpu_loader_materialx` | glTF |
| `webgpu_materials_envmaps_groundprojected` | glTF + `GroundedSkybox` |

…and the **17 examples that use `UltraHDRLoader`** (`webgpu_loader_gltf`,
`webgpu_mrt`, `webgpu_deferred`, `webgpu_materials_transmission`,
`webgpu_performance`, `webgpu_reflection_roughness`, …) become "PMREM +
UltraHDR decode" rather than "PMREM + everything". None of them is reachable
until glTF-with-PBR and/or UltraHDR land; see `ENVIRONMENT-FAMILY.md`.

**On EXR.** The pick does **not** need an EXR loader, and neither does any
other gradable example in this family: the only `webgpu_*` user of `EXRLoader`
is `webgpu_materials_matcap`, which is on the exception list ("need more time
to render"). The loader that actually gates a third of the family is
`UltraHDRLoader` (JPEG + gain-map, `examples/jsm/loaders/UltraHDRLoader.js`),
not EXR. If EXR is ever needed, **port Three's `EXRLoader` rather than take a
crate**: the port's gate is bit-exactness against Three's decoder (the
`rung-pmrem-a` RGBE work is the precedent — Three's half-float conversion
*truncates*, and a crate that rounds would be wrong in a way no image diff
explains). The same argument applies to UltraHDR, where the JPEG gain-map maths
and the `HalfFloatType` output must match Three texel for texel; the JPEG
*container* decode is the one part worth delegating to a crate, since the port
already decodes JPEG for `TextureLoader`.

---

## 8. Verdict

**Rung-sized: one Opus worker, one sitting**, conditional on PMREM B landing
whole. ~250–350 lines of Rust, one new node (`equirectUV`), one new material,
one new generator entry point, one loader wrapper, one renderer flag (`flipY`).
The example is 150 lines with no randomness, no glTF, no animation, a 38 KB
asset, and a 0.0% grade twice.

The risk is not the size, it is that the rung is **too small to stand alone** if
PMREM B lands late or partial. Two mitigations, in order: if B has landed, take
step 8 (`fromScene` + `webgpu_furnace_test`) into the same sitting and land two
examples; if B has *not* landed, this plan is still the right next thing but its
"have (B)" column evaporates and it becomes half of a two-rung job with B.

Top three risks:

1. **`flipY`** (§5.1) — silently plausible wrong image; mitigated by the
   bright-texel readback, which is cheap and decisive.
2. **PMREM B's atlas not being exactly right.** This rung inherits the GGX
   chain wholesale and its image gate cannot distinguish "my equirect material
   is wrong" from "the inherited chain is wrong". Run the atlas readback
   (§5.6.3) against *both* entry points before touching the image.
3. **The `Background` widening** (§4.1) — the one edit that can regress other
   rungs; do it last (step 5), after the ladder is green.

Dependencies: **none on rungs 10/11/12** (no skinning, no BatchedMesh, no
compute, no Points). Hard dependency on `rung-pmrem-b`. Soft dependency on
`rung-materials` for `MeshPhysicalMaterial`'s parameter surface and on
`rung-ssaa`/`rung-lines-fat-a` only through the render-target viewport that
`rung-pmrem-a` already consumes.
