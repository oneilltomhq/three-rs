# Rung 7 scout — `webgpu_shadowmap`

Source: `~/src/vendor/three.js/examples/webgpu_shadowmap.html` (three r186dev, vendor @ 3d010ef).
Everything below was dumped from the real page on this machine with a temporary
`test/e2e/_dump_rung7.mjs` (puppeteer, same flags as the patched `puppeteer.js`:
`--use-angle=vulkan`, no `--disable-vulkan-surface`, profile `.puppeteer_profile_rung7`,
deterministic injection + `clean-page.js` + the `buildInjection` rewrites).
The script and its profile are deleted; the vendor tree ends at `M test/e2e/puppeteer.js`.

The dump hooked Chrome's WebGPU objects directly (`GPUDevice.createShaderModule` /
`createRenderPipeline` / `createBindGroupLayout` / `createTexture` / `createSampler`,
`GPUTexture.createView`, `GPUCommandEncoder.beginRenderPass` + `finish`,
`GPURenderPassEncoder.setPipeline/draw*/setViewport`, `GPUQueue.submit`), so the WGSL,
the layouts, the attachment formats, the draw order **and the submit order** are what
the driver actually received, not what Three's debug API reports.

Artifacts next to this file:

| file | what |
|---|---|
| `dump.json` | every module / pipeline / BGL / texture / pass / sampler / submit, raw |
| `m00..m15_*.{vert,frag}.wgsl` | the 8 programs (16 modules), in module-creation order; names are Three's shader-module labels, with the three unnamed Phong programs labelled by which object uses them |
| `webgpu_shadowmap.jpg` | the grader's reference screenshot (copied) |
| `scene.json` | empty — the example exports nothing on `window`; scene values come from the HTML |

---

## 1. The programs Three generates

Module label = `<stage>[_<program name>]`; pipeline label = `renderPipeline_<material.name||type>_<material.id>`.

| pipeline | label | modules | drawn in |
|---|---|---|---|
| 0 | `Background.material_23` | `m00/m01_Background.material` | main pass, first, 5952 indices (BackSide sphere) |
| 1 | `MeshPhongNodeMaterial_17` | `m02/m03_phong_pillars` (Three emits no program name) | main pass, the 4 pillars, 384 indices each |
| 2 | `ShadowMaterial_24` | `m04/m05` | both shadow passes, the pillars |
| 3 | `ShadowMaterial_24` | `m06/m07` | both shadow passes, the ground (6 indices) |
| 4 | `ShadowMaterial_24` | `m08/m09` | both shadow passes, the torus knot (36000 indices) |
| 5 | `MeshPhongNodeMaterial_22` | `m10/m11_phong_ground` | main pass, the ground |
| 6 | `MeshPhongNodeMaterial_18` | `m12/m13_phong_knot` | main pass, the torus knot (transparent, blended) |
| 7 | `outputColorTransform_26` | `m14/m15` | output pass, `draw(3)` |

**The pillars do not receive shadows.** `pillar*.receiveShadow` is never set, so pipeline 1
(`m02/m03_phong_pillars`) uses BGL 0 — no shadow textures, no `shadowMatrix`, no filter code,
a 8.6 KB fragment against the ground's 23.8 KB. Only the ground and the torus knot
(`receiveShadow = true`) get the two comparison samplers. So "shadow receiving" is a
per-material graph decision driven by `object.receiveShadow`, and the port needs two
different Phong programs from the same `MeshPhongNodeMaterial` settings.

**Three ShadowMaterial programs, one material.** `ShadowNode.updateShadow()` sets
`scene.overrideMaterial = this.getShadowMaterial()` (material id 24 for both lights), but the
override material's graph still pulls the *per-object* material's alpha chain, so the
generated program differs per object and the port must key the shadow program on
(shadow material, object material), not on the override material alone:

* pillars (`m05`): `DiffuseColor = vec4(0,0,0,1); DiffuseColor.w *= opacity;` — no varyings at all, `fn main()` takes no inputs.
* ground (`m07`): carries `v_positionWorld` **and a full `mx_fractal_noise_vec3` evaluation**, because the ground's `colorNode` alpha is reached: `DiffuseColor = vec4(vec3(0), 1.0 * <colorNode>.w)`. ~9.5 KB of MaterialX noise in a depth-only pass.
* torus knot (`m09`): `if ( ! ( mx_fractal_noise_float( positionLocal * 0.1, 3, 2.0, 0.5 ) * 1.0 > 0.0 ) ) { discard; }` — the `maskNode` crosses into the shadow pass, then the same black/opacity body. Input is `@location(0) positionLocal` (a varying of the *local* position, not world).

**Bind-group layouts** (group 0 = render/frame, group 1 = object/material):

* BGL 0 — `{binding 0, visibility VERTEX|FRAGMENT|COMPUTE(7), buffer}`. Used for both groups by pipelines 0–4 (background + shadow materials).
* BGL 1 — shadow-receiving Phong (pipelines 5, 6), group 1:
  `0: buffer(7)`, `1: sampler{type:"comparison"}`, `2: texture{sampleType:"depth"}`, `3: sampler{comparison}`, `4: texture{depth}` — both at `visibility: FRAGMENT`.
* BGL 2 — output pass group 1: `0: sampler{}`, `1: texture{}`, `2: buffer(7)`.

**Samplers created** (exactly two for the frame):
`{clamp-to-edge ×3, mag/min linear, mipmapFilter nearest, lodMinClamp 0, lodMaxClamp 32, compare: "less-equal", maxAnisotropy 1}` (shadow) and the same without `compare` (output pass blit).

**Pipeline state worth copying verbatim**

* main pass targets: `rgba16float`, `writeMask 15`, `multisample.count 4`; depth `depth24plus`, `depthCompare less-equal`.
  * background: `depthWriteEnabled false`, `depthCompare always`, `frontFace cw`, `cullMode back` (the rung-3 background-sphere trick).
  * Phong opaque: `frontFace ccw`, `cullMode back`, `depthWrite true`.
  * torus knot (`transparent: true`): same, plus blend `color: src-alpha / one-minus-src-alpha / add`, `alpha: one / one-minus-src-alpha / add`. Still `depthWrite true`, and still drawn **last** in the same pass.
* shadow pass pipelines: target `rgba8unorm` `count 1`, depth `depth24plus`, `frontFace cw`(!) `cullMode back`, `depthWrite true`, `depthCompare less-equal`. No `depthBias` anywhere — `shadow.bias` is 0 and is applied in the shader, not in pipeline state.
* output pass: target `rgba8unorm` count 1, and Three *does* attach a `depth24plus` depthStencil (`depthWrite true`, `less-equal`) — the port deliberately omits this (docs/nodes.md §7) and that stays fine.
* Vertex buffers: always one buffer per attribute, `arrayStride 12`, `float32x3`: `location 0 = position`, `location 1 = normal`. No uv, no tangent, no colour, anywhere. The shadow programs bind **only** `position`.

### Uniform block layouts (what the port must fill)

`renderStruct` (group 0) for the shadow-receiving Phong, in Three's emission order:
`cameraProjectionMatrix`, `cameraViewMatrix`, then, interleaved by the lights loop:
ambient `vec3` colour·intensity, spot `coneCos`/`penumbraCos` (`f32`,`f32`), spot `distance`/`decay`,
spot `positionView`(`vec3`), dir light colour (`vec3`), `cameraPosition`(vec3, as `nodeUniform9`),
spot target/position world pair (`vec3`,`vec3`), dir position/target pair (`vec3`,`vec3`),
fog colour (`vec3`), fog `near`, fog `far`,
then per shadow: `shadowMatrix` (`mat4x4`), `normalBias` (`f32`), `bias` (`f32`), `radius` (`f32`), `mapSize` (`vec2`), `intensity` (`f32`) — twice (spot then directional).
`objectStruct` (group 1): `modelWorldMatrix` (`mat4x4`), `opacity` (`f32`), `shininess` (`f32`), `specularColor` (`vec3`), `emissive` (`vec3`), `emissiveIntensity` (`f32`), `normalMatrix` (`mat3x3`).
Shadow material `objectStruct`: `{opacity: f32, modelWorldMatrix: mat4x4}` (the pillar/knot variants also carry it).

### The shadow filter, verbatim from `m11_phong_ground`

```wgsl
normalWorld  = normalize( ( vec4<f32>( normalView, 0.0 ) * render.cameraViewMatrix ).xyz );   // inverse-transpose via row-vector multiply
nodeVar9     = render.shadowMatrix * vec4<f32>( shadowPositionWorld + normalWorld * normalBias, 1.0 );
nodeVar10    = nodeVar9.xyz / nodeVar9.w;
shadowCoord  = vec3<f32>( c.x, 1.0 - c.y, c.z + bias );            // Y flip and bias added to z
if ( c.x >= 0 && c.x <= 1 && c.y >= 0 && c.y <= 1 && c.z <= 1 ) {
    phi = interleavedGradientNoise( fragCoord.xy ) * 6.28318530718;
    r   = radius * ( vec2<f32>(1.0,1.0) / mapSize ).x;
    // 5 taps, vogelDiskSample(i, 5, phi) * r, textureSampleCompare( depthTex, cmpSampler, uv, c.z )
    shadow = ( t0 + t1 + t2 + t3 + t4 ) * 0.2;
} else { shadow = 1.0; }
shadowFactor = mix( 1.0, shadow, shadowIntensity );
lightColor   = lightColor * vec3<f32>( shadowFactor );
```
`interleavedGradientNoise` and `vogelDiskSample` are emitted as WGSL `fn`s — see
`m11_phong_ground.frag.wgsl:485-508`; they come from `src/nodes/lighting/ShadowFilterNode.js`
(`PCFShadowFilter`, the default for `PCFShadowMap`) and `src/nodes/utils/…`/`LightUtils`.

Spot light, same shader: `smoothstep( coneCos, penumbraCos, dot( L, spotDirView ) )`,
then `if ( distance > 0 ) { … clamp(1 - (d/dist)^4)^2 / max(pow(d,decay),0.01) } else { 1/max(pow(d,decay),0.01) }`.
Phong direct: `diffuse = saturate(dot(N,L)) * lightColor * DiffuseColor.rgb * 1/π`;
specular = Schlick `exp2((-5.55473*VdotH - 6.98316)*VdotH)` F, `* 0.25`,
`((Shininess*0.5 + 1) * 1/π) * pow(saturate(dot(N,H)), Shininess)`.
Ambient: `irradiance += ambientColor` (no π), then `indirectDiffuse += vec4(irradiance,1) * (DiffuseColor * 1/π)`.

**Fog is the last statement of the scene fragment, in linear space, before tone mapping:**
`Output.rgb = mix( Output.rgb, fogColor, smoothstep( fogNear, fogFar, -v_positionView.z ) )`.
`fogColor` is the *linear* 0x222244 (`0.015996293, 0.015996293, 0.0578054`, same constant as the background).

### The output pass (`m15`)

`textureSample( frameBufferTex, sampler, fragCoord.xy / viewportSize )` → `unpremultiply`
(`if color.w == 0 → 0 else rgb/w`) → `acesFilmicToneMapping( rgb, exposure )` →
`sRGBTransferOETF` → `premultiply`. The ACES matrices are inlined; exposure is a
`render`-group `f32` (1.0). This is `RenderOutputNode` with tone mapping **and** colour
space in the output pass — exactly the structure the port already has, minus ACES.

---

## 2. Render-pass sequence for the one frame

`beginRenderPass` call order is *not* GPU order: Three begins the main pass first, and the
shadow passes are encoded on their own command encoders from inside the main pass's object
loop (`ShadowNode.updateBefore`). The `GPUQueue.submit` order (the only one that matters) is:

1. **shadow pass — spot light.** Color `ShadowMap` `rgba8unorm` 2048×2048 sampleCount 1, `clear` to `(0,0,0,0)`, store. Depth `ShadowDepthTexture` `depth24plus` 2048×2048, `depthLoadOp clear`, `clearValue 1.0`, store. `setViewport(0,0,2048,2048)`. Draws: pillars ×2 (pipeline 2, 384 each), ground (pipeline 3, 6), pillar (pipeline 2, 384), torus knot (pipeline 4, 36000).
2. **shadow pass — directional light.** Identical description against a *second* pair of `ShadowMap`/`ShadowDepthTexture` textures (same sizes/formats). Draws: pillar ×2, ground, pillar ×2, torus knot. (The per-pipeline draw grouping differs from pass 1 only because of the shadow camera's own frustum cull order.)
3. **main pass.** Color: `-msaa` `rgba16float` 800×500 sampleCount 4, `resolveTarget` = the single-sample `rgba16float` 800×500 (view labelled `colorAttachment_0`), `loadOp clear` `(0,0,0,0)`, store. Depth: `depth24plus` 800×500 **sampleCount 4**, `clear`, `clearValue 1.0`, store. Draws in order: background sphere, pillars (1,1), ground, pillars (1,1), torus knot.
4. **output pass.** Color: the canvas texture (`rgba8unorm` 800×500), `loadOp load`, store. Depth: a separate single-sample `depthBuffer` `depth24plus` 800×500, `load`/`store`. One `draw(3)` of the `outputColorTransform` pipeline.

Textures created for the frame: 8 — `rgba16float` resolve, `rgba16float ×4 msaa`, `depth24plus ×4`,
2 × (`ShadowMap rgba8unorm 2048²` + `ShadowDepthTexture depth24plus 2048²`), `depthBuffer depth24plus`.
Note the `ShadowMap` colour target is written but **never sampled**: all sampling is
`textureSampleCompare` on the `depth24plus` `ShadowDepthTexture`. It exists because
`ShadowNode.setupRenderTarget()` builds a `RenderTarget` and hangs a `DepthTexture` on it.

---

## 3. What is in the reference screenshot at t=0

`webgpu_shadowmap.jpg`, 400×250 (800×500 downscaled ×½ by `image.js`'s box `scale()`).
With `Date.now`/`performance.now` pinned to 0 and RAF firing once, `Timer.getDelta()` is 0,
so **none of the animation in `animate()` moves**: the torus knot keeps its identity
rotation, `dirGroup.rotation.y` stays 0, and `dirLight.position.z = 17 + sin(0)*5 = 17`
(so the directional light sits at its authored `(3, 12, 17)` inside an unrotated group).
`OrbitControls` only does `controls.update()` once with `target = (0,2,0)`, so the camera is
the authored `(0,10,20)` looking at `(0,2,0)`, fov 45, aspect 800/500 = 1.6, near 1, far 1000.
The image: a dark blue-violet (`0x222244`) background and a large ground plane (600×600 after
`scale×3`) covered in the grey-white `mx_fractal_noise_vec3(...).zzz * 0.2 + 0.5` mottle, warm
pink-lit from the spot at `(8,10,5)` and cool blue from the directional; four grey cylinders
(r 0.75, h 7) at `(±8, 3.5, ±8)` each throwing two shadows (a warm one away from the spot and a
cool one away from the dir light); in the middle, at `y = 3`, the torus knot (TorusKnot(25,8,75,80)
scaled 1/18) rendered with the noise `maskNode` so roughly half its surface is discarded — a
lacy, partly see-through knot, blended, also casting the same holed shadow. The far half of the
plane fades to the fog colour (fog 50→100 in view-space depth), and the whole thing is ACES
tone-mapped, so nothing clips to white. `Math.random` is never called by this example — no
seeded-random ordering risk at all (the only `Math.*` determinism that matters is `Date.now`).

---

## 4. Gap list against the port

Assumes rung 5 (`Light`, `PointLight`, `MeshPhongNodeMaterial`, `PhongLightingModel`,
`LightsNode`, `normalView`/`positionViewDirection` varyings, `irradiance`/`directDiffuse`/
`directSpecular` property plumbing) has landed; the port today has only
`src/lights/{light,point_light}.rs` on the `rung5` branch and nothing in `src/nodes`
for lighting.

### Lights (all new beyond rung 5's PointLight)

* `AmbientLight` + `AmbientLightNode` — `src/lights/AmbientLight.js`, `src/nodes/lighting/AmbientLightNode.js`. Emits `irradiance += lightColor` with no attenuation. (Rung 6 also wants this; whoever lands first.)
* `SpotLight` (`angle`, `penumbra`, `distance`, `decay`, `target`) — `src/lights/SpotLight.js`, `src/nodes/lighting/SpotLightNode.js`, plus `getDistanceAttenuation` from `src/nodes/lighting/LightUtils.js`. The cone term is `smoothstep(coneCos, penumbraCos, dot)`; `coneCos = cos(angle)`, `penumbraCos = cos(angle*(1-penumbra))`.
* `DirectionalLight` + `DirectionalLightNode` — `src/lights/DirectionalLight.js`, `src/nodes/lighting/DirectionalLightNode.js`. Direction comes from `lightTargetPosition - lightPosition` in **world** space, transformed by `cameraViewMatrix` then normalized (see `m11` `nodeVar26..29`) — the port needs the `lightTargetDirection`/`lightViewPosition` reference nodes from `src/nodes/accessors/Lights.js`.
* Light **intensity → colour** folding: Three uploads `color.multiplyScalar(intensity)` per light as one `vec3` uniform in the render group (`src/nodes/lighting/AnalyticLightNode.js`, `colorNode`/`baseColorNode`).
* `Object3D.target` for SpotLight/DirectionalLight (default target at origin, `updateMatrixWorld` on it) — `src/lights/*.js`. The dir light is inside a `Group` (`dirGroup`), so the light's *world* position must come from `matrixWorld`, not `position`: the port's scene-graph branch already gives this, but the renderer still walks a flat `Child` list (RUNGS.md) — **this rung is the first that genuinely nests and must fold `Child` into the tree.**
* `LightsNode` ordering: the uniform order in `renderStruct` follows the scene's light order (ambient, spot, dir) and the port must reproduce the per-light `setupDirectLight` sequence so the struct layout matches its own builder consistently (textual order need not match Three; see §7 of docs/nodes.md).

### Shadows (all new)

* `LightShadow` / `SpotLightShadow` / `DirectionalLightShadow` — `src/lights/LightShadow.js`, `SpotLightShadow.js`, `DirectionalLightShadow.js`: `mapSize`, `bias`, `normalBias`, `radius`, `intensity`, `blurSamples`, `camera`, `matrix`, `updateMatrices()`. `DirectionalLightShadow` needs `OrthographicCamera` (**not yet in the port** — only `PerspectiveCamera`; `src/cameras/OrthographicCamera.js`) with `left/right/top/bottom = ±17`, `near 0.1`, `far 500`. `SpotLightShadow.updateMatrices` overrides `camera.fov = MathUtils.RAD2DEG * 2 * angle * focus` (focus 1) and `aspect 1`, `near 8`, `far 200`.
* `LightShadow.matrix` = the `_shadowMatrix` biasing `[0.5,0,0,0.5; …]` ∘ `projectionMatrix` ∘ `matrixWorldInverse` — Three's WebGPU path builds it in `src/lights/LightShadow.js` `updateMatrices()` and the shader then does the `1 - y` flip itself (`m11`), so the port must use Three's exact matrix *and* the flip, not bake the flip into the matrix.
* `ShadowBaseNode` (`shadowPositionWorld` property, `setupShadowPosition` honouring `material.receivedShadowPositionNode`) — `src/nodes/lighting/ShadowBaseNode.js`.
* `ShadowNode` — `src/nodes/lighting/ShadowNode.js`: `setupRenderTarget` (RenderTarget + `DepthTexture` named `ShadowDepthTexture`, `compareFunction = LessEqualCompare`, `Linear` min/mag for PCF+texture-compare), `setupShadowCoord`, `setupShadowFilter`, `getShadowMaterial`, `getShadowRenderObjectFunction`, `updateShadow` (reset renderer state, `scene.overrideMaterial`, `setClearColor(0x000000, 0)`, `setRenderTarget(shadowMap)`), `updateBefore`.
* `PCFShadowFilter` + `interleavedGradientNoise` + `vogelDiskSample` — `src/nodes/lighting/ShadowFilterNode.js`. Only `PCFShadowFilter` is needed (`renderer.shadowMap.type` default `PCFShadowMap`); `BasicShadowFilter`/`PCFSoftShadowFilter`/`VSMShadowFilter`, `PointShadowNode` and the VSM blur materials can all be skipped.
* `renderer.shadowMap = { enabled, type }` — `src/renderers/common/Renderer.js`.
* `DepthTexture` with `compareFunction` → a wgpu **comparison sampler** and a `texture_depth_2d` binding: the port's texture/sampler layer has no `sampler_comparison` / `Depth` sample-type path (`src/textures/DepthTexture.js`, `src/renderers/webgpu/utils/WebGPUTextureUtils.js`).
* TSL/builder pieces the filter needs that rung 4's node system does not have: `textureSampleCompare` emission (`depthCompare()` in ShadowFilterNode), `If/Else` returning a value into a pre-declared `var` (the `nodeVar8` pattern — the port has `If` but check the else-assign shape), `Fn()` with **named struct inputs** (`Fn( ( { depthTexture, shadowCoord, shadow, depthLayer } ) => …)`), and `reference( 'radius', 'float', shadow ).setGroup( renderGroup )` — a uniform **referencing a JS object field**, which the port has no equivalent for (`src/nodes/accessors/ReferenceNode.js`, `src/nodes/core/UniformGroupNode.js` `renderGroup`).

### Nodes / TSL

* `mx_fractal_noise_float`, `mx_fractal_noise_vec3` and the whole MaterialX noise chain they pull in: `mx_perlin_noise_float/vec3`, `mx_gradient_float/vec3`, `mx_gradient_scale3d`, `mx_trilerp`, `mx_fade`, `mx_floor`, `mx_select`, `mx_negate_if`, `mx_hash_int`, `mx_hash_vec3`, `mx_rotl32`, `mx_bjfinal` — `src/nodes/materialx/lib/mx_noise.js` and `src/nodes/materialx/mx_noise.js`. **This is the single biggest code item of the rung** and it is bit-sensitive: integer hashing in `u32`, `i32` casts, `mx_floor` returning `i32`. Default args as dumped: `octaves 3, lacunarity 2.0, diminish 0.5`, and the result is `* 1.0` (the amplitude multiply is always emitted).
* `Fn()` with a `toVar()` body and `.xz.addAssign(...)` — swizzle-target assignment. The dump shows it lowered to `nodeVar0.x = nodeVar1[0]; nodeVar0.z = nodeVar1[1];` via a `vec2` temp, i.e. Three assigns through an indexed temp, not `v.xz = …`. Port needs `addAssign` on a swizzle (`src/nodes/core/Node.js` `.assign`, `src/nodes/math/OperatorNode.js`).
* `material.maskNode` → `if ( ! cond ) { discard; }` at the top of the fragment — `src/materials/nodes/NodeMaterial.js` (`setupDiscard`/`maskNode`), `src/nodes/core/Node.js` `discard`.
* `material.receivedShadowPositionNode` — `src/materials/nodes/NodeMaterial.js:289` + `ShadowBaseNode.setupShadowPosition`.
* `saturate()`, `greaterThan()`, `.toVar()`, `positionWorld`, `positionLocal`, `color()` — check which of these the rung-4 TSL layer already exposes; `positionWorld` as a *varying* (`v_positionWorld`) is needed by two of the shadow programs.
* `FogNode` / `RangeFogNode` — `src/nodes/fog/FogNode.js`, `RangeFogNode.js`, wired from `scene.fog` in `src/nodes/core/NodeLibrary.js` / `src/renderers/common/nodes/Nodes.js` (`getFogNode`). Applied as the last step of the material's `Output`, in linear space, using `-v_positionView.z`.
* `scene.backgroundNode = color( 0x222244 )` — the port's rung-3 background path uses `scene.background`; `backgroundNode` goes through `src/nodes/display/…`/`Nodes.js` `getBackgroundNode` and produces the same `Background.material` BackSide sphere (`m00/m01`), with `DiffuseColor = vec4(linear(0x222244),1) * backgroundIntensity`.
* `acesFilmicToneMapping` — `src/nodes/display/ToneMappingFunctions.js`; registered through `ToneMappingNode` / `NodeLibrary.addToneMapping` in `src/renderers/common/nodes/StandardNodeLibrary.js`. The port's `RenderOutputNode` equivalent currently hardcodes `NoToneMapping` (`src/materials/node_material.rs:204`). Also needs `renderer.toneMappingExposure` as a render-group uniform.
* `PhongLightingModel` specular half: `BRDF_BlinnPhong`, `F_Schlick`, `G_BlinnPhong_Implicit`, `D_BlinnPhong` — `src/nodes/functions/PhongLightingModel.js`, `src/nodes/functions/BSDF/{F_Schlick,BRDF_BlinnPhong}.js`. Rung 5 should bring these; if rung 5's PointLight path skipped `specularNode`/`shininess` uniforms, they are needed here (`specular 0x222222`, `shininess 0`→`max(x,1e-4)`).

### Renderer

* **`needs_frame_buffer_target` must stop being hardcoded `true`** (`port/src/renderer/mod.rs:1184`) — or rather, it must become the real predicate, because the shadow passes render to a plain `RenderTarget` with `autoClear`/clear colour `(0,0,0,0)` and **no** frame-buffer target and **no** output pass. Today a shadow pass would get an sRGB output pass bolted on. Three's predicate is in `src/renderers/common/Renderer.js` (`needsFrameBufferTarget` / `_renderOutput`): a frame-buffer target is needed when rendering to the canvas *and* (tone mapping ≠ None or output colour space ≠ Linear) or `samples > 0`.
* Renderer state save/restore around a nested render: `resetRendererAndSceneState` / `restoreRendererAndSceneState` in `src/renderers/common/RendererUtils.js` (renderTarget, activeCubeFace, clear colour/alpha, `scene.background`, `scene.backgroundNode`, `scene.overrideMaterial`, `toneMapping`, `renderObjectFunction`). The shadow pass must run with `scene.background*` and tone mapping *off* and the main camera's uniforms swapped for the shadow camera's — this is the trap that turns into "the shadow map contains the background sphere".
* `scene.overrideMaterial` (port has it from rung 1) must now compose with the per-object material for the alpha/mask chain — see the three `ShadowMaterial` programs.
* `renderer.setRenderObjectFunction` / per-object shadow filtering (`object.castShadow`) — `src/renderers/common/Renderer.js`, `ShadowNode.getShadowRenderObjectFunction`.
* Frustum culling against the **shadow camera** (`shadow.camera.layers`, `Frustum`): the per-pipeline draw grouping in the two shadow passes differs, which only happens if objects are sorted/culled per camera. The port's math2 branch already has `Frustum`.
* Render-target depth textures that are *sampled* afterwards (`usage` must include `TEXTURE_BINDING`), plus a second `RenderTarget` allocation path at 2048² — `port/src/renderer/render_target.rs` currently allocates one target and its msaa/depth pair.
* A bind group that mixes a uniform buffer, two comparison samplers and two depth textures in group 1 (BGL 1) — the port's `programs.rs` binding builder has only seen `{buffer}` and `{sampler, texture}`.
* `setViewport(0,0,2048,2048)` on the shadow pass (Three emits it explicitly; the port may rely on the attachment size).
* MSAA depth attachment at `sampleCount 4` already matches, and the canvas/output-pass structure already matches.

### Geometry / core

* `CylinderGeometry` (r 0.75/0.75, h 7, 32 segments → 384 indices) and `PlaneGeometry` and `TorusKnotGeometry` all exist on the merged geometries branch. Nothing new.
* `Object3D.clone()` for `pillar1.clone()` ×3 (`src/core/Object3D.js`) — cheap, but the pillars share one material instance (same pipeline), which the port's material identity must reflect; and `material.clone()` for `materialCustomShadow` must give a **new** material id so it gets its own pipeline.
* `Group` (exists), `scale.multiplyScalar`, `rotation.x`, all present.
* `Timer` (`src/misc/Timer.js`) — needed only to the extent that `getDelta()` is 0 at t=0; the ported example can skip it, since nothing moves.

---

## 5. Step order for the worker

Build in this order; each step has a visible, checkable consequence.

1. **Port the scene with no shadows and no fog and no tone mapping** — ambient + spot + dir (all `castShadow = false`), flat `colorNode` on the ground instead of the noise, opaque torus knot with no `maskNode`. Expect a recognisable but wrong image: right silhouettes, right lighting direction, no shadows, no mottle. This proves the new lights, the `Group`-nested directional light's world position, and the nested scene-graph walk.
2. **ACES + the `needs_frame_buffer_target` predicate.** Add `acesFilmicToneMapping` to the output pass and make the predicate real. Check against `m15`: unpremultiply → ACES → sRGB OETF → premultiply, in that order, with exposure 1. Getting this wrong is a global, flat colour error across the whole image — easy to spot, easy to misdiagnose later, so do it before shadows.
3. **Fog.** One `mix` at the end of the material fragment, in linear space, on `-v_positionView.z`. The far plane should go blue-violet and blend into the background seamlessly; if the background and the distant ground differ, the fog colour went through a colour-space conversion it should not have.
4. **Orthographic camera + `LightShadow` matrices, no rendering yet.** Unit-test `shadow.matrix` against Three's numbers for both lights (run a one-off node script against the vendor build; do not commit reference values derived from the image — matrices are API output, that is fine).
5. **The shadow pass, depth only.** Allocate the 2048² `rgba8unorm` + `depth24plus` pair, render the `ShadowMaterial` override with renderer state saved/restored, and dump the depth texture to a PNG by hand. Verify four cylinders and a knot are visible from each light's point of view before any sampling exists.
6. **Sample one shadow (the spot), unfiltered**, then add the 5-tap Vogel/IGN filter. Expect warm-side shadows to appear. Then the directional light. At this point the image should be close except for the noise.
7. **MaterialX noise.** Port `mx_noise` as its own module with its own unit tests against values pulled from the vendor build (`mx_fractal_noise_float/vec3` at a handful of positions) before wiring it in. Then: `planeMaterial.colorNode`, `receivedShadowPositionNode`, and the knot's `maskNode` discard (which changes both the main and the shadow programs).
8. **Transparency.** The knot's `transparent: true` blend state, drawn last in the same pass with `depthWrite true`. The port has "no transparent list" as a known rung-4 gap; here one object needs it.

### Traps

* **Comparison sampler, not a manual compare.** Binding is `sampler_comparison` + `texture_depth_2d` + `textureSampleCompare`, with `magFilter/minFilter: linear` so the hardware gives a 4-tap bilinear *comparison* per tap (20 effective taps). Using `textureSample` + a manual `<` gives hard, aliased shadows that will blow the 0.1% budget at every shadow edge. wgpu requires the depth texture's sampler to be declared comparison in the layout and the texture `sampleType: Depth`.
* The `ShadowMap` `rgba8unorm` colour target is written but never read. Do not try to encode depth into it, and do not skip it either if the port's RenderTarget needs a colour attachment — it is simply the `RenderTarget`'s texture.
* **Y flip and bias live in the shader**, after the perspective divide: `vec3(c.x, 1-c.y, c.z + bias)`. `bias` is 0 here and `normalBias` is 0, so both terms are no-ops for this example — but `normalBias` still appears in the shader as `shadowPositionWorld + normalWorld * normalBias`, and `normalWorld` is computed as `normalize((vec4(normalView,0) * cameraViewMatrix).xyz)` (row-vector multiply = inverse view rotation). Emitting `cameraViewMatrix * vec4(...)` instead silently rotates the bias and, more importantly, is the same mistake that will bite `normalWorld` elsewhere.
* **`radius` scaling uses only `.x` of `1/mapSize`** (`radius * (1/mapSize).x`). With a square 2048² map it does not matter; copy it anyway.
* `intensity` is applied as `mix(1.0, shadow, intensity)` — shadow intensity 1 here, but the `mix` must exist so the uniform slot exists.
* **The `if (inside frustum)` guard's else branch is `1.0`** (fully lit) and the guard includes `c.z <= 1.0` but *not* `c.z >= 0`. The 600×600 ground extends far outside both shadow frusta, so a wrong guard shows up as a huge square of shadow or a huge square of light on the plane — the most likely first-failure mode of step 6.
* **Tone mapping is in the output pass, not in the scene pass** (rung 2's finding again). Fog, however, is in the scene pass, in linear space, *before* tone mapping. Swapping either ordering changes every pixel.
* The shadow passes must run with no output pass, no tone mapping, no background, clear colour `(0,0,0,0)` and `depthClearValue 1.0`, and with the render-group camera uniforms pointing at the shadow camera. `resetRendererAndSceneState` exists in Three precisely because forgetting one of these is silent.
* Submit order is shadow, shadow, main, output even though Three *begins* the main pass first. Do the shadow passes first and submit them first; do not try to reproduce the interleaving.
* `frontFace: cw` on the shadow pipelines and on the background, `ccw` on the scene materials — the port already handles the background case; the shadow case is new.
* `Math.random` is never called here, so there is no seeded-random order to reproduce — but `Date.now() === 0` means `Timer.getDelta()` is 0 and **nothing** in `animate()` applies. A port that advances the knot's rotation by any nonzero delta fails everywhere at once.
* MaterialX noise is integer-hash-based: `mx_rotl32`/`mx_bjfinal` must be `u32` wrapping arithmetic (Rust `wrapping_*` if the port ever evaluates it on the CPU) and `mx_floor` returns `i32`. A one-bit difference in the hash changes the mottle pattern over the whole ground plane and over the knot's discard mask — this will read as "the noise is wrong", not "the shadow is wrong", so port it with its own tests first.
* The `transparent` knot is drawn in the same pass after the opaque objects with `depthWrite: true`; it is also in both shadow maps (with the discard), so its holes must match between the shadow and main programs — the same `maskNode` graph must be reachable from both.

---

## 6. HANDOFF-hard items (addons, loaders, assets)

Almost nothing, which is why this rung is reachable.

* **No external assets at all.** No textures, no models, no fonts, no JSON. Nothing is fetched.
* `OrbitControls` (`examples/jsm/controls/OrbitControls.js`) — an addon, but only `target.set(0,2,0)`, `minDistance`, `maxDistance`, `update()` are used, and at t=0 that reduces to "camera at (0,10,20) looking at (0,2,0)". Port the two lines into the example scene, do not port the addon.
* `Inspector` (`examples/jsm/inspector/Inspector.js`) — assigned to `renderer.inspector`. It is a debug overlay; `clean-page.js` hides `.three-inspector` and the grader's `buildInjection` force-disables `trackTimestamp` so it cannot even take timings. It contributes nothing to the framebuffer. Skip it entirely.
* `THREE.Timer` (`src/misc/Timer.js`, core not addon) — skip, delta is 0.
* `TeapotGeometry` is not needed here (that is rung 5).

## Director addendum (23:30)

Correction: the port does have `OrthographicCamera` (src/cameras/orthographic_camera.rs, added at rung 4 for the RTT quad). The LightShadow camera can use it as is.
