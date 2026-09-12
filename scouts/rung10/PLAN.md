# Rung 10 scout — `webgpu_skinning`

Source: `~/src/vendor/three.js/examples/webgpu_skinning.html` (three **r186**, vendor @ `148ef33`
with `rung0/grader-flags.patch` applied).

Everything below was dumped from the real page on this machine with a temporary
`test/e2e/_dump_rung10.mjs` (puppeteer, same flags as the patched `puppeteer.js`:
`--use-angle=vulkan`, *no* `--disable-vulkan-surface`, profile `.puppeteer_profile_rung10`,
`deterministic-injection.js` + `clean-page.js` + the `buildInjection` rewrites, `networkidle0`
→ 2 s network idle → one RAF). The script and its profile are deleted; `git status` in the vendor
tree shows `M test/e2e/puppeteer.js` plus the *other* concurrent scouts' files
(`_dump_rung11/12/13.mjs`, `_probe_rung13.mjs`, `.puppeteer_profile_rung11/12/13/`) — none of them mine.

The dump hooks Chrome's WebGPU objects directly (`GPUDevice.createShaderModule` /
`createRenderPipeline` / `createBindGroupLayout` / `createBindGroup` / `createPipelineLayout` /
`createTexture` / `createSampler`, `GPUTexture.createView`, `GPUCommandEncoder.beginRenderPass`
+ `finish`, `GPURenderPassEncoder.setPipeline/setBindGroup/setVertexBuffer/setIndexBuffer/draw*`,
`GPUQueue.submit/writeBuffer/writeTexture`, `GPUAdapter.requestDevice`), so the WGSL, the layouts,
the attachment formats, the draw order **and the submit order** are what the driver actually
received. It also wraps `Renderer.prototype.render` (via a tail appended to the intercepted
`build/three.webgpu.js`) to capture the scene at the pinned frame.

Artifacts next to this file:

| file | what |
|---|---|
| `e2e-skinning-r186.log` | Three's own e2e on the four skinning examples |
| `dump.json` | every module / pipeline / BGL / bind group / texture / view / sampler / pass / submit / writeBuffer / writeTexture, raw |
| `m00..m06_*-r186.wgsl` | the 7 shader modules in creation order, named by Three's own module label |
| `scene.json` | the whole scene at the pinned frame: renderer, camera, lights, every node's `matrixWorld`, the skin, the material and all five textures |
| `bones_t0.json` | the unit-test oracle: `bindMatrix`, `bindMatrixInverse`, 65 `boneInverses`, the 65 flattened `boneMatrices` that get uploaded, and every bone's local TRS + `matrixWorld` at the pinned time |
| `webgpu_skinning.jpg` | the grader's reference screenshot (copy of `examples/screenshots/webgpu_skinning.jpg`) |
| `actual_full-r186.png` | Three's own 800×500 frame, undownscaled |
| `actual-r186.jpg` | the same, through `image.js`'s `scale(1/2)` at quality 95 — byte-for-byte what the grader compares |

---

## 1. The example is 0.0% at r186

```
cd ~/src/vendor/three.js
npm run test-e2e-webgpu -- webgpu_skinning webgpu_skinning_instancing \
    webgpu_skinning_instancing_individual webgpu_skinning_points
```

```
Diff 0.0% in file: webgpu_skinning (4.0s)
Diff 0.0% in file: webgpu_skinning_instancing (3.8s)
Diff 0.0% in file: webgpu_skinning_instancing_individual (4.9s)
Diff 0.0% in file: webgpu_skinning_points (3.7s)
TEST PASSED! 4 screenshots rendered correctly.
```

`webgpu_skinning` is **0.0%**, so the ladder entry stands and no alternative is needed.
(The other three are also 0.0% and stay on file as later reorders: `_instancing` and
`_instancing_individual` add `InstancedMesh` + per-instance `AnimationMixer`s on the same
`Michelle.glb`; `_points` swaps in `PointsNodeMaterial` and `computeSkinning()` — a compute pass,
which is rung 12 territory.)

---

## 2. The page

`examples/webgpu_skinning.html`, 129 lines. Imports `three/webgpu`, `color`/`screenUV` from
`three/tsl`, and `GLTFLoader` from `three/addons/loaders/GLTFLoader.js`. Nothing else.

```js
camera = new THREE.PerspectiveCamera( 50, window.innerWidth / window.innerHeight, 0.01, 100 );
camera.position.set( 1, 2, 3 );
scene  = new THREE.Scene();
scene.backgroundNode = screenUV.y.mix( color( 0x66bbff ), color( 0x4466ff ) );
camera.lookAt( 0, 1, 0 );
timer = new THREE.Timer(); timer.connect( document );
const light = new THREE.PointLight( 0xffffff, 1, 100 ); light.power = 2500;
camera.add( light ); scene.add( camera );
scene.add( new THREE.AmbientLight( 0x4466ff, 1 ) );
new GLTFLoader().load( 'models/gltf/Michelle.glb', gltf => {
    mixer = new THREE.AnimationMixer( gltf.scene );
    mixer.clipAction( gltf.animations[ 0 ] ).play();
    scene.add( gltf.scene );
} );
renderer = new THREE.WebGPURenderer( { antialias: true } );
renderer.toneMapping = THREE.LinearToneMapping;
renderer.toneMappingExposure = 0.4;
```

**Model.** `examples/models/gltf/Michelle.glb`, 3 276 888 bytes, Blender glTF I/O v3.4.50.
One scene, root node 66 `Character` (rotation `[0.70710688829422, 0, 0, 0.7071066498756409]`,
uniform scale `0.009999999776482582`), one mesh node 65 `Ch03` (`mesh 0`, `skin 0`), 65 joints.
One primitive: `POSITION`/`TEXCOORD_0`/`NORMAL` (16 340 each, `float32`), `JOINTS_0`
(`VEC4 UNSIGNED_BYTE`), `WEIGHTS_0` (`VEC4 float32`), 84 318 `UNSIGNED_SHORT` indices,
material 0. **No morph targets, no tangents.**

**Clip.** `gltf.animations[ 0 ]` = **`SambaDance`**, 195 channels / 195 samplers, all
`interpolation: "LINEAR"`, paths `translation` / `rotation` / `scale` only, 547 keyframes per
track, input range `[0.03333333333333333, 18.233333333333334]` (accessor 7). The second clip
`TPose` (2 keyframes) is never played.

**The pinned frame.** The injection makes `performance.now()`, `Date.now()` and
`Date.prototype.getTime()` all return `0`, and RAF fires exactly once. `THREE.Timer`
(`src/core/Timer.js`) takes `this._startTime = performance.now()` in its constructor and
`update()` computes `_delta = ( performance.now() - _previousTime ) * _timescale`, so
**`timer.getDelta()` is exactly `0`** and `animate()` reduces to

```js
mixer.update( 0 );          // action.time stays 0
renderer.render( scene, camera );
```

So **the mixer time is exactly t = 0 s**, not the bind pose: `action.play()` has already
activated the action, `mixer.update( 0 )` still runs one accumulate/apply cycle at clip time 0
with weight 1, and every bone's `position`/`quaternion`/`scale` is overwritten with the clip's
value at 0. Because the first keyframe of every `SambaDance` track is at **t = 0.0333 s**, the
`LinearInterpolant` clamps below the first key and every track returns its **first keyframe
value verbatim**. That is the pose in `bones_t0.json` and in the reference image.

**Lights** (`scene.json`):

* `PointLight`, colour `0xffffff`, `power = 2500` → **`intensity = 2500 / (4π) = 198.94367886486918`**,
  `distance = 100`, `decay = 2`. It is a **child of the camera**, and the camera is a child of the
  scene, so its world matrix is the camera's: world position `(1, 2, 3)`.
* `AmbientLight`, colour `0x4466ff` (`4482815`), `intensity = 1`.
* No directional/spot/hemisphere light, **no shadows**, no environment, no IBL.

**Camera.** `PerspectiveCamera( 50, 800/500 = 1.6, 0.01, 100 )` at `(1, 2, 3)`,
`lookAt( 0, 1, 0 )` → quaternion `[-0.1505711418828577, 0.15830765731779026,
0.024434332593631203, 0.9755357401230101]`. `projectionMatrix` and `matrixWorldInverse` are in
`scene.json` to 17 digits, for a direct unit test.

**Background.** No `scene.background`; `scene.backgroundNode = screenUV.y.mix( color( 0x66bbff ),
color( 0x4466ff ) )`. `screenUV` is **not** Y-flipped under WGSL (`ScreenNode.generate()` only
flips when `builder.isFlipY()`, which is a WebGL/GLSL thing), so the gradient is
`fragCoord.y / height`: `0x66bbff` at the **top**, `0x4466ff` at the **bottom**. Verified against
`actual_full-r186.png`: row 0 is `(65, 123, 170)`, row 499 is `(42, 65, 170)`, which is exactly
linear `0x66bbff = (0.13287, 0.49693, 1.0)` and `0x4466ff = (0.05781, 0.13287, 1.0)`, each
`× 0.4` (LinearToneMapping exposure) then sRGB-encoded.

**No fog, no grid, no helpers, no `OrbitControls`, no `Math.random`.** `renderer.inspector` is not
set on this example (`Inspector` is not even imported), so unlike rungs 7/9 there is nothing to
hide. The only addon is `GLTFLoader`.

**What the frame looks like** (`webgpu_skinning.jpg`, 400×250): a vertical blue gradient, light
steel-blue at the top, saturated blue at the bottom; the Mixamo "Michelle" character centred and
slightly left, roughly 60 % of the frame height, mid-samba: weight on the right leg, left knee
bent and raised, right arm flung out to her right at shoulder height, left arm down, torso twisted
toward the camera, both bunches of hair swung out. Dark skin, grey crop top, yellow
harem trousers with cyan stripes, dark trainers. Lit almost entirely by the camera-mounted point
light (so there is no visible shadow side), with the blue ambient filling the silhouette edges.

---

## 3. What Three renders

### 3.1 Modules, pipelines, layouts

7 shader modules, 5 pipelines, 3 bind-group layouts, 9 textures, 4 samplers, 38 render passes,
6 queue submits.

| pipeline | label | modules | where |
|---|---|---|---|
| p0 | `renderPipeline_Background.material_19` | `m00/m01_Background.material` | main pass, first, `drawIndexed(5952)` |
| p1 | `mipmap-rgba8unorm-srgb-2d-array` | `m02_mipmap` | mip blits for the two sRGB textures |
| p2 | `mipmap-rgba8unorm-2d-array` | `m02_mipmap` | mip blits for the two linear textures |
| p3 | `renderPipeline_Ch03_Body_18` | `m03/m04_Ch03_Body` | main pass, second, `drawIndexed(84318)` |
| p4 | `renderPipeline_outputColorTransform_21` | `m05/m06` | output pass, `draw(3)` |

`m02_mipmap` is Three's own raw-WGSL blit — the port already has it (`src/renderer/mipmap.rs`).
Note the mip passes issue their draws through **render bundles**
(`WebGPUTexturePassUtils._mipmapRunBundles` → `passEncoder.executeBundles`), which is why
`dump.json`'s `passes[1..36].cmds` are empty; it is not a missing hook.

**Bind-group layouts**

* `bgl0` — `{ binding 0, visibility 7 (VERTEX|FRAGMENT|COMPUTE), buffer }`. Used for *both*
  groups of the background pipeline and for group 0 of everything.
* `bgl1` — the skinned material's group 1:
  `0: buffer(7)`, then five `sampler`/`texture` pairs at `visibility 2 (FRAGMENT)` —
  `1/2` map, `3/4` metalness+roughness, `5/6` specularColor, `7/8` **DFG_LUT**, `9/10` normalMap —
  and finally **`11: buffer, visibility 1 (VERTEX only)` = the bone-matrix uniform array.**
* `bgl2` — output pass group 1: `0: sampler`, `1: texture`, `2: buffer(7)`.

**Samplers** (4, all cached by descriptor):

| id | descriptor | used by |
|---|---|---|
| s0 | `addressMode u/v repeat`, w clamp, mag/min/mipmap **linear**, lod 0–32, aniso 1 | the four glTF textures |
| s1 | `{ minFilter: linear }` | mipmap blit |
| s2 | `{ minFilter: nearest }` | mipmap blit |
| s3 | all clamp-to-edge, mag/min linear, **mipmapFilter nearest**, lod 0–32 | DFG_LUT **and** the output-pass blit (same descriptor → one sampler) |

**Textures** (9):

| id | label | size | format | mips |
|---|---|---|---|---|
| t0 | *(frame-buffer target)* | 800×500 | `rgba16float` | 1 |
| t1 | `-msaa` | 800×500 ×4 | `rgba16float` | 1 |
| t2 | *(depth)* | 800×500 ×4 | `depth24plus` | 1 |
| t3 | `Ch03_1001_Diffuse` | 512×512 | `rgba8unorm-srgb` | 10 |
| t4 | `Ch03_1001_Glossiness` | 512×512 | `rgba8unorm` | 10 |
| t5 | `Image` (the specular-colour map) | 512×512 | `rgba8unorm-srgb` | 10 |
| t6 | `DFG_LUT` | 16×16 | `rg16float` | 1 |
| t7 | `Ch03_1001_Normal` | 512×512 | `rgba8unorm` | 10 |
| t8 | `depthBuffer` | 800×500 | `depth24plus` | 1 |

`DFG_LUT` is the only `writeTexture` (1024 bytes, `bytesPerRow 64`).

**Pipeline state to copy verbatim**

* Main pass targets `rgba16float` `writeMask 15` `multisample.count 4`; depth `depth24plus`,
  `depthCompare less-equal`.
  * Background: `depthWriteEnabled false`, `depthCompare **always**`, `frontFace cw`,
    `cullMode back` — the rung-3 background-sphere trick, unchanged.
  * `Ch03_Body`: `frontFace ccw`, **`cullMode none`** (the glTF material is `doubleSided: true`
    → `side: DoubleSide`), `depthWriteEnabled true`, `depthCompare less-equal`, no blending
    (`transparent: false`).
* Output pass: target `rgba8unorm` count 1, `frontFace ccw`, `cullMode back`, and Three attaches
  a `depth24plus` depthStencil (the port deliberately omits this, `docs/nodes.md` §7 — still fine).
* **Vertex buffers: one buffer per attribute, in this order**

  | slot / `@location` | attribute | stride | format |
  |---|---|---|---|
  | 0 | `position` | 12 | `float32x3` |
  | 1 | `skinIndex` | **16** | **`uint32x4`** |
  | 2 | `skinWeight` | 16 | `float32x4` |
  | 3 | `normal` | 12 | `float32x3` |
  | 4 | `uv` | 8 | `float32x2` |

  The order is the node graph's request order, not the geometry's: `NodeMaterial.setupPosition()`
  runs `skinning()` first (position → skinIndex → skinWeight), then the normal chain, then the
  material's `map` pulls `uv`.

  `skinIndex` reaches the GPU as **`Uint32Array`**, not the glTF `Uint8Array`:
  `WebGPUAttributeUtils.createAttribute` (`src/renderers/webgpu/utils/WebGPUAttributeUtils.js:82-104`)
  rewrites `bufferAttribute.array` in place for any non-normalized `Int8/Int16/Uint8/Uint16`
  attribute, and does the same to the index buffer (`UNSIGNED_SHORT` → `uint32`, with
  `0xffff → 0xffffffff` primitive-restart remapping — harmless here, 16 340 vertices).

### 3.2 The skinning vertex path, module by module (`m03_vertex_Ch03_Body-r186.wgsl`)

Source: `src/nodes/accessors/Skinning.js` (the file is **`Skinning.js`**, not `SkinningNode.js` —
r186 replaced the node class with a TSL `Fn`), called from
`src/materials/nodes/NodeMaterial.js` `setupPosition()`.

```wgsl
struct NodeBuffer_1225Struct { value : array< mat4x4<f32>, 65 > };
@binding( 11 ) @group( 1 ) var<uniform> NodeBuffer_1225 : NodeBuffer_1225Struct;
```

**Uniform buffer, not a bone texture.** `getBoneMatricesNode()` picks
`referenceBuffer( 'skeleton.boneMatrices', 'mat4', bones.length )` whenever
`bones.length * 16 * 4 <= builder.getUniformBufferLimit()`. Here that is
`65 * 64 = 4160` bytes against `maxUniformBufferBindingSize = 65536` on this adapter
(`intel / gen-12lp`), so the texture path (`skeleton.computeBoneTexture()`,
`getBoneTextureMatrices`, `textureSize`/`load`) is **never taken** and
`skeleton.boneTexture` stays `null` (confirmed in `scene.json`). The upload is one
`queue.writeBuffer` of 4160 bytes into a buffer labelled
`bindingBufferundefined_UniformBuffer_0_(vertex)`.

Position:

```wgsl
positionLocal = position;
nodeVar0 = ( object.nodeUniform2 * vec4<f32>( positionLocal, 1.0 ) );        // bindMatrix * position
positionLocal = ( object.nodeUniform0 * (                                    // bindMatrixInverse * ...
      ( ( skinWeight.x * B[ skinIndex.x ] ) * nodeVar0 )
    + ( ( skinWeight.y * B[ skinIndex.y ] ) * nodeVar0 )
    + ( ( skinWeight.z * B[ skinIndex.z ] ) * nodeVar0 )
    + ( ( skinWeight.w * B[ skinIndex.w ] ) * nodeVar0 ) ) ).xyz;
```

Note the association: Three emits `(w * M) * v`, i.e. the weight scales the **matrix** first.
`nodeVar0` is a `vec4` temp (`bindMatrix.mul( position )` is hoisted once and reused by all four
terms). `bindMatrix` = `object.nodeUniform2`, `bindMatrixInverse` = `object.nodeUniform0`.

Normal (`getSkinnedNormalAndTangent`, reached because the geometry has a `normal` attribute; the
tangent branch is *not* reached, there is no `tangent`):

```wgsl
normalLocal = normal;
normalLocal = mat3x3<f32>(
    ( ( bindMatrixInverse * ( w.x*B[i.x] + w.y*B[i.y] + w.z*B[i.z] + w.w*B[i.w] ) ) * bindMatrix )[0].xyz,
    ( ...same... )[1].xyz,
    ( ...same... )[2].xyz ) * normalLocal;
```

Three does **not** CSE the skin matrix here — it re-emits the full
`bindMatrixInverse * Σ(w·B) * bindMatrix` product three times, once per column. The port may CSE
it (pixel-neutral, same arithmetic), but note the different association from the position path:
here the weights multiply the matrices and the **sum** is formed first, then
`bindMatrixInverse * skinMatrix * bindMatrix`, then `mat3()` of it.

The rest of the vertex stage is the ordinary NodeMaterial flow:

```wgsl
varyings.nodeVarying6       = uv;
varyings.v_normalViewGeometry = normalize( ( cameraViewMatrix * vec4( normalMatrix * normalLocal, 0.0 ) ).xyz );
modelViewMatrix             = cameraViewMatrix * modelWorldMatrix;
varyings.v_positionView     = ( modelViewMatrix * vec4( positionLocal, 1.0 ) ).xyz;
varyings.v_positionViewDirection = -v_positionView;
builtinClipSpace            = cameraProjectionMatrix * vec4( v_positionView, 1.0 );
```

**`bindMatrix` and `bindMatrixInverse` at this frame** (`bones_t0.json`):

```
bindMatrix        = identity                                   // GLTFLoader binds with the identity
bindMatrixInverse = [ 100.00000223517424, 0, 0, 0,
                      0, -3.0294629646984445e-05, -100.00000565802227, 0,
                      0,  100.00000565802227,     -3.0294629646984445e-05, 0,
                      0, 0, 0, 1 ]                              // = inverse( skinnedMesh.matrixWorld )
```

`bindMode` is `attached`, so `SkinnedMesh.updateMatrixWorld()` recomputes
`bindMatrixInverse = matrixWorld⁻¹` **every frame** — that is where the `Character` node's
0.01 scale and the ±90° X rotation live. This is the trap the gltf worker already flagged
(`port/docs/gltf-progress.md`, "Two things that will bite the rung worker").

### 3.3 The material (`m04_fragment_Ch03_Body-r186.wgsl`, 20 KB)

**It is a `MeshPhysicalNodeMaterial`, not `MeshStandardNodeMaterial.`** The GLB declares
`extensionsUsed: [ "KHR_materials_specular", "KHR_materials_ior" ]`, and `GLTFLoader`'s
`KHR_MATERIALS_SPECULAR` / `KHR_MATERIALS_IOR` extension handlers switch the material class to
`MeshPhysicalMaterial`. `scene.json`:

```
type MeshPhysicalMaterial, name "Ch03_Body", id 18, side 2 (DoubleSide), transparent false,
color 0xffffff, opacity 1, metalness 0.5, roughness 1,
ior 1.4500000476837158, specularIntensity 1, specularColor 0xffffff,
clearcoat 0, sheen 0, iridescence 0, transmission 0, anisotropy 0, dispersion 0,
emissive 0x000000, emissiveIntensity 1, flatShading false, vertexColors false, alphaTest 0,
normalScale ( 1, -1 ), normalMapType TangentSpaceNormalMap
map              Ch03_1001_Diffuse    512² srgb  flipY false wrap 1000/1000 mag Linear min LinearMipmapLinear
normalMap        Ch03_1001_Normal     512² ""    (linear)
roughnessMap     Ch03_1001_Glossiness 512² ""    (linear)
metalnessMap     Ch03_1001_Glossiness 512² ""    (the same texture object)
specularColorMap Image                512² srgb
```

`normalScale.y = -1` is **not** in the glTF: `GLTFLoader.js:3564` clones the material and flips
`normalScale.y` when the geometry has a `normalMap` but no `tangent` attribute
(`useDerivativeTangents`). That clone is why the material id in the pipeline label is 18 while the
uncloned original is 17. Getting this wrong flips the lighting on every bump.

`objectStruct` (group 1, binding 0, **608 bytes**), in Three's emission order:

| member | type | value |
|---|---|---|
| `nodeUniform0` | `mat4x4` | `bindMatrixInverse` |
| `nodeUniform2` | `mat4x4` | `bindMatrix` |
| `nodeUniform3` | `vec3` | material `color` |
| `nodeUniform5` | `mat3x3` | `map` uv transform |
| `nodeUniform6` | `f32` | `opacity` |
| `nodeUniform7` | `f32` | `metalness` |
| `nodeUniform9` | `mat3x3` | `metalnessMap` uv transform |
| `nodeUniform10` | `f32` | `roughness` |
| `nodeUniform11` | `mat3x3` | `roughnessMap` uv transform |
| `nodeUniform13` | `mat3x3` | `normalMatrix` |
| `nodeUniform14` | `f32` | `ior` |
| `nodeUniform15` | `vec3` | `specularColor` |
| `nodeUniform17` | `mat3x3` | `specularColorMap` uv transform |
| `nodeUniform18` | `f32` | `specularIntensity` |
| `nodeUniform19` | `vec3` | `emissive` |
| `nodeUniform20` | `f32` | `emissiveIntensity` |
| `nodeUniform22` | `mat4x4` | `modelWorldMatrix` |
| `nodeUniform24` | `mat3x3` | `normalMap` uv transform |
| `nodeUniform25` | `vec2` | `normalScale` |

`renderStruct` (group 0, binding 0, **192 bytes**): `cameraProjectionMatrix`, `cameraViewMatrix`,
`nodeUniform27: vec3` (point-light colour × intensity), `nodeUniform28: f32` (light `distance` =
100), `nodeUniform29: f32` (`decay` = 2), `nodeUniform30: vec3` (ambient colour × intensity),
`nodeUniform26: vec3` (the point light's **view-space** position, appended after the per-light
triples — the same ordering rung 5 found).

The fragment, in order:

1. `DiffuseColor = vec4( color, 1 ) * textureSample( map )`; `.w *= opacity`; `.w = 1.0`.
2. `Metalness = metalness * roughnessMap.b` — note **`.z`**, the metalness channel of the ORM map.
3. `normalViewGeometry = normalize( v_normalViewGeometry )`;
   `Roughness = min( max( roughness * roughnessMap.g, 0.0525 ) + max3( abs(dpdx(N)), abs(-dpdy(N)) ), 1.0 )`
   — the geometric-roughness term is always emitted.
4. `IOR = ior`; `f0 = ((IOR-1)/(IOR+1))`;
   `SpecularColor = min( vec3(f0*f0) * ( specularColor * specularColorMap.rgb ), 1 ) * specularIntensity`.
   **This is the whole `MeshPhysicalNodeMaterial` delta over rung 8's Standard material**, where
   `SpecularColor` is the constant `vec3(0.04)` and `SpecularF90` is the constant `1.0`.
5. `SpecularColorBlended = mix( SpecularColor, DiffuseColor.rgb, Metalness )`;
   `SpecularF90 = mix( specularIntensity, 1.0, Metalness )`;
   `DiffuseContribution = DiffuseColor.rgb * ( 1 - metalness*roughnessMap.b )`.
6. Emissive, then the **derivative TBN**: `faceDirection = f32(isFront)*2-1`,
   `cross(-dpdy(v_positionView), N)`, `dpdx(uv)`, `cross(N, dpdx(v_positionView))`, `-dpdy(uv)`,
   the `if ( det == 0 ) { 0 } else { inverseSqrt(det) }` guard, `mat3(T, B, N)`; then
   `normalView = normalize( TBN * vec3( (2*normalMap-1).xy * normalScale, (2*normalMap-1).z ) )`.
7. `dfg = textureSample( DFG_LUT, s3, vec2( Roughness, saturate( dot(N,V) ) ) ).xy`;
   `multiScatteringCompensation = SpecularColorBlended * (1/(dfg.x+dfg.y) - 1) + 1`.
8. **The one point light**: `L = lightViewPosition - v_positionView`;
   `if ( distance > 0 ) { d = length(L); t = d/distance; att = (1/max(pow(d,decay),0.01)) * saturate(1 - t⁴)² }`
   `else { att = 1/max(pow(length(L),decay),0.01) }`; `irradiance = saturate(dot(N,L̂)) * colour * att`.
   Diffuse: `irradiance * DiffuseContribution * (1/π) * ( 1 - F_Schlick(SpecularColor, SpecularF90, VdotH) )`.
   Specular: `F_Schlick(SpecularColorBlended, 1.0, VdotH) * V_GGX_SmithCorrelated(α, NdotL, NdotV)
   * D_GGX(α, NdotH) * irradiance * multiScatteringCompensation`, with `α = Roughness²`.
   `V_GGX_SmithCorrelated` and `D_GGX` are the only emitted `fn`s.
9. **Ambient**: `irradiance += ambientColour` (no π), then the full
   single/multi-scattering dielectric+metallic chain from
   `src/nodes/functions/PhysicalLightingModel.js` — all of it emitted even though
   `radiance` and `iblIrradiance` are hard `vec3(0)` (no env map). `ambientOcclusion = 1.0`, and
   the specular-AO term (`exp2(-(1 - Roughness*-16))`, `pow`, `saturate`) is still emitted.
10. `Output = max( vec4( totalDiffuse + totalSpecular + EmissiveColor, DiffuseColor.w ), 0 )`.

### 3.4 The output pass (`m06`)

```wgsl
nodeVar0 = textureSample( frameBufferTex, s3, fragCoord.xy / viewportSize );
nodeVar1 = unpremultiply( vec4( nodeVar0.xyz, clamp( nodeVar0.w, 0, 1 ) ) );
nodeVar2 = vec4( linearToneMapping( nodeVar1.xyz, exposure ), nodeVar1.w );
output.color = premultiply( vec4( sRGBTransferOETF( nodeVar2.xyz ), nodeVar2.w ) );
```

`linearToneMapping( c, e ) = clamp( c * e, 0, 1 )` with `exposure = 0.4` as a render-group `f32`.
This is the port's existing `RenderOutputNode` plus one new tone-mapping function — the cheapest
tone mapper on the ladder (rung 7 needs ACES; this one is two lines).

### 3.5 Pass and submit order

`beginRenderPass` order is not GPU order; the `GPUQueue.submit` order is:

1. 9 mip-blit passes into `Ch03_1001_Diffuse` (one `executeBundles` per level 1..9), encoder `mipmapEncoder`.
2. 9 mip-blit passes into `Ch03_1001_Glossiness`.
3. 9 mip-blit passes into `Image`.
4. 9 mip-blit passes into `Ch03_1001_Normal`.
5. **main pass** (`renderContext_0`): colour `-msaa` `rgba16float` 800×500 ×4, `resolveTarget` the
   single-sample `rgba16float`, `loadOp clear` `(0,0,0,0)`; depth `depth24plus` 800×500 **×4**,
   `clear` to 1.0. Draws, in order:
   `p0` background (`setBindGroup 0/1`, index `uint32`, 1 vertex buffer, `drawIndexed(5952, 1)`),
   then `p3` `Ch03_Body` (`setBindGroup 0/1`, index `uint32`, **five** vertex buffers,
   `drawIndexed(84318, 1)`).
6. **output pass** (`renderContext_1`): colour = the canvas texture `rgba8unorm` 800×500,
   `loadOp load`; depth `depthBuffer` `depth24plus` 800×500, `load`/`store`. One
   `p4` `draw(3, 1)`.

Only two draws in the scene pass. No shadow pass, no second render target.

---

## 4. Gap list against the port

`port` is at `cdf834a` (treewalk merged). 767 tests, e2e 4/4 (0 / 45 / 0 / 1).

### What already exists

* **`GLTFLoader`** (`src/loaders/gltf_loader.rs`): GLB container, accessors, primitives →
  `BufferGeometry`, nodes → `Object3D` tree, skins → `Skeleton`, animations → `AnimationClip`,
  materials/textures/images as data records with the raw bytes. 7 tests against three's own
  loader (`tests/gltf/samples/sample.mjs`), Michelle included.
* **`Bone` / `Skeleton` / `SkinnedMesh`** (`src/objects/{bone,skeleton,skinned_mesh}.rs`):
  `bind`, `pose`, `normalizeSkinWeights`, `update_matrix_world`, `apply_bone_transform`,
  `Skeleton::update()` → `bone_matrices: Vec<f32>`, `bone_texture_size()`.
* **`AnimationMixer`** and the whole `src/animation` family, with
  `src/animation/object3d_target.rs` binding `.position` / `.quaternion` / `.scale` /
  `.morphTargetInfluences` through `SceneResolver` / `NodeTarget`. `LinearInterpolant` included.
* **The tree walk**: `Payload::{Mesh, InstancedMesh}`, `renderer::project_object`,
  `RenderList` with three's comparators, lights collected into `RenderList.lights`
  (`src/renderer/render_list.rs`).
* **The node system** (`docs/nodes.md`): `NodeBuilder` emitting both WGSL stages plus bindings,
  render/object uniform groups, `TextureSource`/`SamplerBinding`, `RenderOutputNode`,
  the mipmap blit (`src/renderer/mipmap.rs`), MSAA `rgba16float` frame-buffer target + output pass.
* `png = "0.17"` is already a dependency (used for writing); `zune-jpeg` for decode.

### What is missing — owned by rung 10

1. **`Payload::SkinnedMesh`.** `SkinnedMesh` still owns its own `Node`
   (`src/objects/skinned_mesh.rs:28`), so `project_object` never sees it.
   Add the variant (`src/objects/payload.rs`), move `geometry` / `material` / `skeleton` /
   `bind_mode` / `bind_matrix` / `bind_matrix_inverse` into it, make `Payload::is_mesh()` true for
   it (three's `SkinnedMesh extends Mesh`), and make `SkinnedMesh::new` return a `Node`.
   Frustum culling must use `SkinnedMesh.boundingSphere` (three's `SkinnedMesh.raycast`/cull path),
   not the geometry's — though at 800×500 with the whole model on screen nothing culls either way.
2. **The skinning node** — `src/nodes/accessors/Skinning.js` ported into the port's `setup_position`
   hook, which `docs/nodes.md` §6 already names as the insertion point (it is where
   `instanced_mesh()` already goes). Needs: `attribute( 'skinIndex', 'uvec4' )` (a **u32** vertex
   attribute — new; every attribute in the port so far is `f32`), `reference( 'bindMatrix' )` /
   `reference( 'bindMatrixInverse' )` as *object-group* mat4 uniforms, and the
   `referenceBuffer( 'skeleton.boneMatrices', 'mat4', n )` binding.
3. **`array<mat4x4<f32>, N>` uniform binding.** A new `Binding` kind: a `var<uniform>` struct whose
   single member is a fixed-size `mat4` array, at its own binding index, **`visibility: VERTEX`
   only**, updated per object. `UniformSource::BoneMatrices(skeleton)` writing
   `Skeleton::bone_matrices` (already a `Vec<f32>` of `16 * bones`). Choose the uniform path
   unconditionally for this rung — `65 * 64 = 4160 ≤ 65536` — and leave `computeBoneTexture()`
   for the day a skeleton exceeds the limit.
4. **`skinIndex` / index widening.** The port's loader emits `Uint8` skin indices and `u16`
   indices. Either widen both to `u32` on upload exactly as
   `WebGPUAttributeUtils.createAttribute` does (recommended: identical pipeline descriptors to
   the dump), or declare `uint8x4` and keep `vec4<u32>` in WGSL. Do **not** mix.
5. **PNG decode + texture-from-bytes.** `TextureLoader` takes a path and decodes only JPEG
   (`src/loaders/texture_loader.rs:33`). Four 512×512 8-bit RGB PNGs come out of the GLB as
   `bufferView` bytes; the loader needs a `from_bytes( &[u8], mime )` entry point using the `png`
   crate. (Colour type 2 = RGB, no alpha — expand to RGBA.)
6. **`RepeatWrapping`.** `Wrapping` (`src/textures/texture.rs:14`) has only `ClampToEdge`.
   The glTF sampler has no `wrapS`/`wrapT`, so three's default `RepeatWrapping` applies to all four
   textures, and the sampler descriptor in the dump is `repeat`/`repeat`. (The UVs are all inside
   `[0,1]` on this model, so this is likely pixel-neutral — but the sampler must still be right or
   the *mip* footprint at the UV-island seams changes.)
7. **`GLTFLoader` material/texture wiring**: turn `GltfMaterial` + `GltfImage` records into a real
   material with five texture slots, honouring `KHR_materials_specular`
   (`specularColorTexture`, `specularColorFactor`, `specularFactor`) and `KHR_materials_ior`
   (`ior: 1.45`), the sRGB/linear colour-space split (`map` and `specularColorMap` sRGB;
   `normalMap`, `roughnessMap`, `metalnessMap` linear), `flipY = false`, and the
   `useDerivativeTangents → normalScale.y *= -1` clone.
8. **`LinearToneMapping`** in `RenderOutputNode` (`src/materials/node_material.rs:204` hardcodes
   `NoToneMapping`) plus `renderer.toneMappingExposure` as a render-group `f32`.
9. **`backgroundNode`.** The port's rung-3 background path takes `scene.background`
   (a `Color`/`CubeTexture`). This example sets `scene.backgroundNode` to a TSL graph, so the
   background material's `colorNode` becomes `screenUV.y.mix( color(a), color(b) )` —
   needs `screenUV` (= `fragCoord.xy / viewportSize`, **no Y flip**) and `mix` on colours.
   Note the background vertex shader here also carries the orthographic branch
   (`if cameraProjectionMatrix[3][3] == 1.0`), which rung 3's dump did not have.
10. **The example + the e2e entry**: `examples/webgpu_skinning.rs` and a row in `tests/e2e/main.rs`.

### What rung 10 needs from rung 8 (PBR)

Rung 8 (`webgpu_lights_physical`) owns `MeshStandardNodeMaterial` + `PhysicalLightingModel`.
Rung 10 needs **all** of it and a little more. From rung 8, unchanged:

* `MeshStandardNodeMaterial`: `metalness`/`roughness`/`metalnessMap`/`roughnessMap`
  (`.b` and `.g` of the same texture), `normalMap` + the derivative-TBN normal node,
  `emissive`/`emissiveIntensity`, the geometric-roughness `dpdx/dpdy` term and the
  `max( …, 0.0525 )` clamp.
* `PhysicalLightingModel`: `F_Schlick`, `V_GGX_SmithCorrelated`, `D_GGX`, `BRDF_Lambert`,
  `computeMultiscattering` and the single/multi scattering dielectric+metallic chain, the
  specular-AO term.
* **The DFG LUT**: the 16×16 `rg16float` `DataTexture` from
  `src/nodes/functions/BSDF/DFGLUT.js` (a literal `Uint16Array` in three's own source — copy it,
  it is not reference-image-derived), `rg16float` upload, and a clamp-to-edge linear sampler.
* `AmbientLight` + `AmbientLightNode`, `PointLight` + `PointLightNode` +
  `getDistanceAttenuation` (rung 5 brings the last two; rung 8 brings ambient).

Rung 10 adds on top, and this is the whole `MeshPhysicalNodeMaterial` surface it needs:

* `ior` → `f0 = ((ior-1)/(ior+1))²`,
* `specularColor` × `specularColorMap`, `specularIntensity`,
* `SpecularColor = min( f0 * specularColor·map, 1 ) * specularIntensity`,
* `SpecularF90 = mix( specularIntensity, 1, metalness )` (Standard hardcodes `1.0`),
* `side: DoubleSide` → `cullMode: none`.

Everything else in `MeshPhysicalMaterial` — clearcoat, sheen, iridescence, transmission,
anisotropy, dispersion — is 0 here and **must not be emitted**; the dump shows no trace of any of
them, so they are dead code for this rung.

If rung 8 has not landed when rung 10 starts, rung 10 cannot be done first: the fragment shader is
95 % rung 8's. **Order 8 → 10 is a hard dependency**, unlike 7 → 10.

### What the mixer must produce, and how to unit-test it

At the pinned frame the renderer must do, in this order (the order
`tests/gltf_loader.rs::michelle_mixer_at_zero` already uses):

```
mixer.update( 0.0 )                      // writes bone .position/.quaternion/.scale
scene.update_matrix_world( true )        // bone matrixWorld, and the SkinnedMesh's
skinned_mesh.update_matrix_world( true ) // AttachedBindMode: bindMatrixInverse = matrixWorld⁻¹
skeleton.update()                        // boneMatrices[i] = boneMatrix[i] * boneInverses[i]
upload bone_matrices                     // 4160 bytes → the vertex-visible uniform array
```

`bones_t0.json` is the oracle, taken from the page at exactly that moment:

* `bones[]` — all 65, each with `name`, `parent`, local `position`/`quaternion`/`scale` **after**
  `mixer.update(0)`, and `matrixWorld`.
* `boneInverses[]` — the 65 `inverseBindMatrices` from the GLB.
* `boneMatrices` — the 4160-byte flattened array as uploaded, 65 × 16 f32.
* `bindMatrix` (identity) and `bindMatrixInverse`.
* `skinnedMeshMatrixWorld`.

First row for a smoke check: `mixamorigHips.matrixWorld` starts
`[0.008227340933105275, -0.00018857241280558921, -0.005681135218087022, 0, …]` and its
translation is `(-0.0011792862151722705, 0.9886072747501711, -0.0011999657255460897)`;
`boneMatrices[0..16]` starts `[0.8227341175079346, -0.018857240676879883, -0.5681135058403015, 0, …]`
(note the ×100 relative to `matrixWorld` — the bind-space scale).

Three new tests, all CPU-only, none touching the GPU:

1. `michelle_bones_world_at_zero` — every one of the 65 bone `matrixWorld`s against
   `bones_t0.json` to 1e-6. (`tests/gltf_loader.rs` already checks two of them; widen it.)
2. `michelle_bone_matrices_at_zero` — the full 4160-element `Skeleton::bone_matrices` against
   `bones_t0.json`, plus `bind_matrix_inverse`, after the five-step order above.
3. `michelle_skinned_vertex_at_zero` — apply the WGSL skinning arithmetic on the CPU for a handful
   of vertices and compare against a node-side evaluation. (Optional; 1 and 2 pin everything that
   matters, and `apply_bone_transform` is already tested.)

Failing 1 but passing the existing `michelle_mixer_at_zero` means the clamp-below-first-keyframe
behaviour differs; failing 2 with 1 green means `bindMatrixInverse` or the
`boneMatrix * boneInverse` order.

---

## 5. Traps

* **The material is Physical, not Standard.** `KHR_materials_specular` + `KHR_materials_ior` in
  the GLB silently promote the class. Porting the glTF material as `MeshStandardNodeMaterial`
  gives `SpecularColor = 0.04` instead of `((1.45-1)/(1.45+1))² = 0.03373…` modulated by the
  512² `Image` specular-colour texture — a small, uniform, everywhere error that reads as
  "the lighting is slightly off" rather than as a missing feature.
* **`normalScale.y = -1`.** Applied by `GLTFLoader`, not by the asset, because the geometry has a
  normal map and no tangents. Wrong sign = the bump lighting inverts over the whole model.
* **`mixer.update( 0 )` is not the bind pose.** It applies the clip. And because every track's
  first keyframe is at 0.0333 s while the mixer is at 0 s, the interpolant **clamps** — a port
  that extrapolates below the first key, or that treats "no keyframe at or before t" as
  "leave the bone alone", gets a different (and much more T-pose-like) silhouette.
* **`bindMatrixInverse` must be recomputed every frame.** `bindMode` is `attached`; the
  `Character` node carries `scale = 0.01` and a ±90° X rotation, so a stale or identity
  `bindMatrixInverse` scales the whole skin by 100 and lies it on its back. Already flagged in
  `port/docs/gltf-progress.md`.
* **Bone count vs the uniform limit.** 65 bones → 4160 bytes, comfortably under
  `maxUniformBufferBindingSize = 65536` on this adapter, so Three takes the **uniform-array**
  path and never allocates a bone texture. Do not implement the bone-texture path for this rung;
  do gate on the limit so the fallback is a known hole rather than a silent wrong answer. Also
  note `array<mat4x4<f32>, 65>` has a **stride of 64 bytes** with no padding (mat4 align 16), so
  the `Vec<f32>` maps 1:1.
* **`skinIndex` is an integer attribute.** `vec4<u32>` in WGSL, `uint32x4` in the pipeline,
  stride 16, after Three's in-place `Uint8Array → Uint32Array` rewrite. Declaring it `float32x4`
  and converting in the shader works numerically but changes the vertex buffer layout and the
  attribute is then subject to f32 rounding above 2²⁴ (irrelevant at 65 bones, but it is the
  wrong shape).
* **Skinning precision.** The weights sum to 1 in the asset (no `normalizeSkinWeights` call in the
  example), and the whole chain is f32 in the shader. Do the sum in the same association Three
  uses — `(w·M)·v` summed left-to-right for position, `Σ(w·M)` then
  `bindMatrixInverse · Σ · bindMatrix` for the normal. Re-associating changes the last bits and
  can move a silhouette pixel, which at 84 318 indices against a 100-pixel budget is affordable
  but not free.
* **Draw order.** Background first (`depthCompare always`, `depthWrite false`, `frontFace cw`),
  then the one opaque mesh. Nothing is transparent, so the port's missing transparent list is not
  exercised here.
* **`cullMode: none`.** `doubleSided: true` in the glTF. With back-face culling on, the inside of
  the hair bunches and the trouser cuffs punch holes.
* **Texture colour spaces.** `map` and `specularColorMap` are `rgba8unorm-srgb` (the GPU does the
  sRGB→linear decode, no colour-space node — the rung-3 finding); `normalMap`, `roughnessMap`,
  `metalnessMap` are plain `rgba8unorm`. `roughnessMap` and `metalnessMap` are **the same texture
  object** and appear once in the bind group (bindings 3/4), sampled twice with two different
  (identical) uv-transform matrices. `flipY = false` on all four.
* **Mipmaps.** All four are 512² with 10 levels, generated by Three's own blit before the frame.
  The port already has `mipmap.rs`; the only new thing is that these are `2d` (the existing path
  is `2d-array`, and Three uses `main_2d_array` here too — same module).
* **`screenUV` has no Y flip under WGSL.** Flipping it swaps the gradient top-to-bottom, which is
  a ~100 % pixel diff on the background and trivially visible, but it is the kind of thing that
  gets "fixed" the wrong way when the model looks right.
* **`Math.random` is never called.** No seeded-random ordering to reproduce. `Date.now()` = 0 is
  the only determinism that bites, via `Timer.getDelta() === 0`.
* **Morph targets: none.** `geometry.morphAttributes` is empty and `morphTargetInfluences` is
  `null` on the `SkinnedMesh`. Rung 6's morph path is not exercised.
* **No `Inspector` on this example** — unlike rungs 7 and 9, nothing to hide, nothing to skip.
* The GLB has **no** Draco, no meshopt, no KHR_texture_transform, no KHR_materials_emissive_strength,
  no cameras and no lights. `KHR_materials_specular` and `KHR_materials_ior` are the only two
  extensions and neither is in `extensionsRequired`.

---

## 6. Step ladder

Each step has a gate that fails loudly.

1. **`Payload::SkinnedMesh` + the pose, drawn with the existing basic material.**
   Move `SkinnedMesh`'s geometry/skeleton into a `Payload` variant, make the walk draw it,
   render Michelle at t = 0 with `MeshBasicNodeMaterial` and **no skinning node** (bind pose,
   positions straight from the attribute).
   *Gate:* the existing e2e stays at 0 / 45 / 0 / 1, and a new
   `examples/webgpu_skinning.rs` produces a recognisable human-shaped white silhouette on the
   blue gradient, roughly centred. It will be in a T-pose and slightly the wrong shape — that is
   expected.

2. **The mixer at t = 0, CPU only.**
   Wire `AnimationMixer` + `SceneResolver` into the example, run the five-step order, and add the
   two new unit tests against `bones_t0.json`.
   *Gate:* `michelle_bones_world_at_zero` and `michelle_bone_matrices_at_zero` green to 1e-6.
   No pixels move yet (nothing consumes `bone_matrices`).

3. **The skinning node.**
   `u32` vertex attributes, the `array<mat4x4, N>` vertex-visible uniform binding, and
   `skinning()` inside `setup_position`. Dump the port's generated vertex WGSL
   (`examples/dump_wgsl.rs`) and diff it statement-by-statement against
   `m03_vertex_Ch03_Body-r186.wgsl`.
   *Gate:* the silhouette snaps from T-pose to the samba pose — compare against
   `webgpu_skinning.jpg` by eye; the arms and the raised knee are unmistakable. Still untextured
   and unlit.

4. **Textures: PNG-from-bytes, `RepeatWrapping`, the five slots.**
   Decode the four PNGs out of the GLB, upload with the right colour spaces and 10 mip levels,
   and put `map` on a `MeshBasicNodeMaterial` first.
   *Gate:* the character is correctly textured — dark skin, grey top, yellow trousers with cyan
   stripes — at flat full brightness. Mips: check the 512² textures report 10 levels and that
   the trouser stripes do not alias.

5. **The material and the lights** (needs rung 8's `PhysicalLightingModel` + DFG LUT).
   `MeshPhysicalNodeMaterial` with ior/specularColor/specularIntensity, the normal map with
   `normalScale = (1,-1)`, metalness/roughness from the ORM map, the camera-child `PointLight`
   (`power 2500`, `distance 100`, `decay 2`) and the `AmbientLight`.
   *Gate:* diff the port's fragment WGSL against `m04_fragment_Ch03_Body-r186.wgsl` program-wide
   before looking at pixels (rung 5's discipline). Then the image should be close but flat/bright,
   because tone mapping is still `NoToneMapping`.

6. **`LinearToneMapping` at exposure 0.4 + the `backgroundNode` gradient, then the diff.**
   *Gate:* the background gradient reads `(65, 123, 170)` at the top row and `(42, 65, 170)` at
   the bottom of the 800×500 frame — that is `actual_full-r186.png`'s exact values and it pins
   tone mapping, exposure, `screenUV` orientation and the sRGB OETF in one check. Then
   `cargo test --test e2e -- --nocapture --test-threads=1` against
   `examples/screenshots/webgpu_skinning.jpg`, target < 100 of 100 000 pixels.

---

## 7. HANDOFF-hard items

* **One asset**: `examples/models/gltf/Michelle.glb` (3.2 MB). Already read by the port's
  `tests/gltf_loader.rs`, so the path plumbing exists.
* **One addon**: `GLTFLoader`, already ported (`src/loaders/gltf_loader.rs`) except for
  images/textures and extensions.
* **`THREE.Timer`** (`src/core/Timer.js`) — skip; delta is 0, the ported example calls
  `mixer.update( 0.0 )` directly.
* No `OrbitControls`, no `Inspector`, no `TeapotGeometry`, no second model, no env map, no HDR.
* **The DFG LUT array** (`src/nodes/functions/BSDF/DFGLUT.js`) is the only bulk constant to copy,
  and it is rung 8's to copy.
