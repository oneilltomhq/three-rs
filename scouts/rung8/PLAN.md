# Rung 8 scout — the PBR rung

Scouted 2026-09-12. Vendor tree `~/src/vendor/three.js` @ 3d010ef (r186dev),
grader flags patch applied (`M test/e2e/puppeteer.js` only).

**Recommendation: `webgpu_lights_physical`.** `webgpu_materials` contains no
PBR at all — see §1.2. It is a TSL-breadth example, not a PBR example, and
belongs (if anywhere) beside rung 13, not here.

---

## 1. The two candidates

### 1.1 `webgpu_lights_physical`

Source: `~/src/vendor/three.js/examples/webgpu_lights_physical.html`, ~110
lines of scene code. Reference: `webgpu_lights_physical.jpg` (next to this
file). Rung 0 graded it **0.0%**.

**What the page builds**

Camera: `PerspectiveCamera( 50, 800/500, 0.1, 100 )` at `(-4, 2, 4)`.
`OrbitControls` is constructed (its constructor's `update()` re-aims the
camera at the origin — same situation as rung 1) and never touched again.

Lights (two, no ambient):

| light | construction | value at t=0 |
|---|---|---|
| `PointLight` | `( 0xffee88, 1, 100, 2 )`, `position (0,2,0)`, `castShadow = true` | `animate()` sets `power = 400` → `intensity = 400 / (4π) = 31.830988618379067`; `distance = 100`, `decay = 2`; `position.y = cos(0)*0.75 + 1.25 = 2.0` (unchanged) |
| `HemisphereLight` | `( 0xddeeff, 0x0f0e0d, 0.02 )` | `animate()` sets `intensity = 0.0001` (`hemiLuminousIrradiances[0]`) — effectively black, but it is still compiled into every fragment shader |

Materials — four `MeshStandardMaterial`, i.e. four
`MeshStandardNodeMaterial` under WebGPURenderer:

| material | mesh | parameters | maps |
|---|---|---|---|
| `bulbMat` | `SphereGeometry(0.02,16,8)`, child **of the light** (so it inherits the light's world transform) | `color 0x000000`, `emissive 0xffffee`, `emissiveIntensity` set each frame to `intensity / 0.02² = 79577.47154594767` | none |
| `floorMat` | `PlaneGeometry(20,20)`, `rotation.x = -π/2`, `receiveShadow` | `roughness .8`, `metalness .2`, `color 0xffffff`, `bumpScale 1` | `hardwood2_diffuse.jpg` (**SRGBColorSpace**), `hardwood2_bump.jpg` (linear), `hardwood2_roughness.jpg` (linear) — all `RepeatWrapping`, `repeat (10,24)`, `anisotropy 4` |
| `cubeMat` | `BoxGeometry(.5,.5,.5)` ×3 at `(-0.5,.25,-1)`, `(0,.25,-5)`, `(7,.25,0)`, all `castShadow` | `roughness .7`, `metalness .2`, `bumpScale 1` | `brick_diffuse.jpg` (**sRGB**), `brick_bump.jpg` (linear), `RepeatWrapping`, `repeat (1,1)`, `anisotropy 4` |
| `ballMat` | `SphereGeometry(.25,32,32)` at `(1,.25,1)`, `rotation.y = π`, `castShadow` | `roughness .5`, `metalness 1.0` | `planets/earth_atmos_2048.jpg` (**sRGB**), `planets/earth_specular_2048.jpg` as `metalnessMap` — the example marks this one **sRGB too**, which is arguably wrong but must be reproduced bit-for-bit |

No env map, no `scene.background` (renders over the clear colour), no
`scene.environment`, **no PMREM**, no cube textures, no HDR/EXR.

Renderer: `WebGPURenderer()` — **`antialias` is not set**, so `sampleCount = 1`
throughout (confirmed in the dump; simpler than rung 2's MSAA path).
`shadowMap.enabled = true`, `toneMapping = ReinhardToneMapping`,
`toneMappingExposure = 0.68^5 = 0.14539336640000004`.

Addons: `OrbitControls` (construction-time `update()` only) and `Inspector`
(GUI only, no pixels). `Math.random` is **never called** — one whole class of
determinism trap is absent.

**Programs Three compiles** (dumped, `dumps/webgpu_lights_physical.dump.json`):
13 shader modules, **8 render pipelines**, 5 bind-group layouts:

| pipeline | what it is | files here |
|---|---|---|
| `renderPipeline_MeshStandardMaterial_17` | `bulbMat` (emissive only, + DFG LUT) | `MeshStandardMaterial_17.{vert,frag}.wgsl` |
| `renderPipeline_MeshStandardMaterial_18` | `floorMat` — the big one: map + bump + roughnessMap + **point shadow** | `MeshStandardMaterial_18.*` |
| `renderPipeline_MeshStandardMaterial_19` | `ballMat` (map + metalnessMap) | `MeshStandardMaterial_19.*` |
| `renderPipeline_MeshStandardMaterial_20` | `cubeMat` (map + bump) | `MeshStandardMaterial_20.*` |
| `renderPipeline_ShadowMaterial_24` | the shadow-map depth pass material (one pipeline, shared by all casters) | `ShadowMaterial_24.*` |
| `renderPipeline_outputColorTransform_26` | the output pass: premultiply-undo → Reinhard → sRGB OETF | `outputColorTransform_26.*` |
| `mipmap-rgba8unorm-srgb-2d-array`, `mipmap-rgba8unorm-2d-array` | Three's raw-WGSL mipmap blit, already ported (`src/renderer/mipmap.rs`) | `mipmap_*.{vert,frag}.wgsl` |

So **five new node-generated programs**, of which four are the same
`MeshStandardNodeMaterial` graph with different map sets — the increment is
one lighting model plus one material, instantiated four ways.

GPU textures created (`dumps/textures.json`): the rgba16float 800×500 frame
target + depth24plus, the 16×16 `DFG_LUT` (rg16float), six sRGB/linear
rgba8unorm 2D maps with full mip chains, and
`PointShadowMap` rgba8unorm 512×512×**6 layers** + `PointShadowDepthTexture`
depth24plus 512×512×6, sampled as `texture_depth_cube`.

**The reference image at t=0.** A near-black room. A wide hardwood floor
fills the lower two-thirds, running from a vanishing point at upper centre
down to the bottom edge; the planks are lit by a single warm point light
hanging at `y = 2` above and slightly behind the camera-facing side, so the
brightest area is a soft elliptical pool just left of centre, with a long
specular streak from the roughness/bump interplay running down the middle of
the floor towards the bottom edge. Three small brick cubes sit on the floor:
one large and clearly lit at centre-left (that's `boxMesh` at `(-0.5,.25,-1)`,
nearest the light, with a visible soft cube shadow cast to its right), one
small and dim at far left (`boxMesh2` at `(0,.25,-5)`), one small at far right
(`boxMesh3` at `(7,.25,0)`), both far enough away that inverse-square falloff
drops them to a dim brown. Right of centre, a small dark metallic sphere
(`ballMesh`, metalness 1.0, earth diffuse × earth-specular metalness map)
shows a bright highlight on its upper left and a dark grounded contact
shadow. The top third of the image is essentially pure black: the bulb mesh
itself is at `y = 2` and outside the frustum, so no emissive blob appears.
Everything above the floor horizon is the untouched clear colour. Most of the
image's pixel budget is therefore in the smooth, low-contrast falloff over
the floor — which is good news for the 0.1% threshold (no thin high-contrast
geometry edges of the kind that sank `webgpu_camera`), and bad news in that
a slightly wrong falloff exponent or tone-mapping exposure moves *every*
floor pixel at once.

### 1.2 `webgpu_materials`

Source: `examples/webgpu_materials.html`. Reference: `webgpu_materials.jpg`
(next to this file). Rung 0 graded it 0.0%.

**There is no PBR in this example.** It builds 17 materials, of which 16 are
`MeshBasicNodeMaterial` with a `colorNode` and one is `MeshNormalMaterial`,
all on a `TeapotGeometry(50,18)`, plus a `GridHelper` (`LineBasicMaterial`).
Dumped program count: **18 render pipelines** (16 `MeshBasicNodeMaterial`, 1
`LineBasicMaterial`, 1 `MeshNormalMaterial`) + 1 `outputColorTransform` + 1
mipmap blit. No lights whatsoever, no shadow, no tone mapping, no env map.
Textures: `uv_grid_opengl.jpg`, `alphaMap.jpg` (both `RepeatWrapping`).

What it *would* force is breadth in TSL, not depth in lighting:
`positionLocal`/`positionWorld`/`normalLocal`/`normalWorld`/`normalView`,
`opacityNode` + `transparent` (a sorted transparent list — the port does not
have one), `alphaTestNode`, `cameraProjectionMatrix` as a node, user `Fn()`
with and without inputs, **`wgslFn` with an includes list** (raw-WGSL
injection), `triplanarTexture`, `screenUV.flipY()`, `oscSine`, and `Loop()`
with a `toVar()` accumulator. It also needs `GridHelper` and `TeapotGeometry`
(the latter already ported), `Math.random` three times per mesh in creation
order (17 meshes × 3 draws, order-sensitive), and a camera position driven by
`Date.now()` (`cos(0)*1000 = 1000`, `z = 0`) plus one `+= 0.01 / 0.005`
rotation step applied before the single render.

**Recommendation and why.** Pick `webgpu_lights_physical`.

1. It is the only one of the two that is the PBR rung. `webgpu_materials`
   would leave `MeshStandardNodeMaterial`, `PhysicalLightingModel`, BRDF_GGX
   and the DFG LUT entirely unported, and the ladder has no other slot for
   them.
2. The increment on top of rung 7 is small and almost entirely *reusable*:
   one new lighting model file, one new material, one new light type
   (`HemisphereLight`), one new shadow variant (point/cube). Everything else —
   the output pass, tone mapping, the shadow infrastructure, `normalView`,
   texture nodes, mip generation — rung 7 and rungs 2–4 already deliver.
   Four of the five new programs are the *same* graph with different map
   sets, so one correct `MeshStandardNodeMaterial` buys all four.
3. Its determinism surface is unusually clean: **no `Math.random`**, no
   `scene.background`, no MSAA, no addon geometry, no loader beyond
   `TextureLoader` on plain JPEGs that rungs 3 and 5 already decode.
4. `webgpu_materials` is much wider and much shallower: 18 distinct programs,
   a transparent+alphaTest sorted draw list the renderer lacks, `wgslFn`
   raw-WGSL splicing, and `Loop`/`toVar` control flow in the node builder.
   That is a rung's worth of *node-builder* work with zero lighting payoff,
   and it duplicates ground rung 13 (`webgpu_tsl_galaxy`, pure TSL) will
   cover anyway. It is a good **reorder candidate for a later TSL rung**, not
   rung 8.
5. Its reference screenshot also carries alpha-mapped overlay glyphs on the
   left teapots and a full-frame thin-line `GridHelper` — thin high-contrast
   lines are exactly the failure class that dropped `webgpu_camera` at rung
   0, and it only scored 0.0% because Three was grading itself.

**Assets it needs** (all already in the vendor tree, all plain JPEG):

```
~/src/vendor/three.js/examples/textures/hardwood2_diffuse.jpg      sRGB
~/src/vendor/three.js/examples/textures/hardwood2_bump.jpg         linear
~/src/vendor/three.js/examples/textures/hardwood2_roughness.jpg    linear
~/src/vendor/three.js/examples/textures/brick_diffuse.jpg          sRGB
~/src/vendor/three.js/examples/textures/brick_bump.jpg             linear
~/src/vendor/three.js/examples/textures/planets/earth_atmos_2048.jpg      sRGB
~/src/vendor/three.js/examples/textures/planets/earth_specular_2048.jpg   sRGB (as metalnessMap — reproduce the example's choice)
```

Plus the 16×16 RG16F DFG LUT, which is a literal `Uint16Array` in Three's own
source (`src/nodes/functions/BSDF/DFGLUT.js`) — port the array as source
data, it is not derived from any reference image.

---

## 2. The dumps

Produced with a temporary `~/src/vendor/three.js/test/e2e/_dump_rung8.mjs`
(deleted; profile dir `.puppeteer_profile_rung8` deleted), modelled on
`puppeteer.js`: same flags with `--use-angle=vulkan` and no
`--disable-vulkan-surface`, same `buildInjection` rewrites of
`build/three.webgpu.js`, same `deterministic-injection.js` +
`clean-page.js` + `networkidle` + single-RAF sequence. It hooks
`GPUDevice.prototype.{createShaderModule,createBindGroupLayout,createPipelineLayout,createRenderPipeline*,createTexture}`
so nothing depends on Three's debug API, and captures every pipeline actually
built, in creation order.

Files next to this plan:

```
MeshStandardMaterial_17.{vert,frag}.wgsl  + .layout.txt   bulbMat
MeshStandardMaterial_18.{vert,frag}.wgsl  + .layout.txt   floorMat  (the reference program)
MeshStandardMaterial_19.{vert,frag}.wgsl  + .layout.txt   ballMat
MeshStandardMaterial_20.{vert,frag}.wgsl  + .layout.txt   cubeMat
ShadowMaterial_24.{vert,frag}.wgsl        + .layout.txt   shadow-map depth pass
outputColorTransform_26.{vert,frag}.wgsl  + .layout.txt   Reinhard + sRGB OETF
mipmap_rgba8unorm_2d_array.*, mipmap_rgba8unorm_srgb_2d_array.*   (already ported)
dumps/webgpu_lights_physical.dump.json    full capture incl. bind-group layouts
dumps/textures.json                       every GPUTexture created
dumps/webgpu_lights_physical.actual.png   Three's own 800×500 frame
```

`.layout.txt` carries the primitive/depthStencil/multisample/target state and
the full `@group` entry list per pipeline. Shape confirms rung 4's model:
`@group(0)` = render group (one uniform buffer, visibility 7), `@group(1)` =
object group (uniform buffer at binding 0, then sampler/texture pairs from
binding 1 up). Colour target is `rgba16float` (the internal frame target),
depth `depth24plus` / `less-equal`, `cullMode: back`, `sampleCount 1`.
Vertex buffers arrive as three separate buffers: `uv` (loc 0, float32x2),
`normal` (loc 1, float32x3), `position` (loc 2, float32x3) — note **uv first**.

---

## 3. Gap list

Against `port/src/nodes/{builder,node,tsl,wgsl}.rs`,
`port/src/materials/node_material.rs` and `port/src/renderer/*`, assuming
rung 5 (PointLight + `MeshPhongNodeMaterial` + normalMap + specular) and
rung 7 (shadow maps, Fog, ACES tone mapping, `Fn()`) have landed.

### 3.1 Lighting model — `src/nodes/functions/PhysicalLightingModel.js`

Everything below is one file in Three and should be one module in the port.

- `PhysicalLightingModel.direct()` — the `directDiffuse` / `directSpecular`
  accumulation seen in `MeshStandardMaterial_18.frag.wgsl`.
- `PhysicalLightingModel.indirectDiffuse()` / `indirectSpecular()` /
  `ambientOcclusion()` — the `singleScatteringDielectric` /
  `multiScatteringDielectric` / `singleScatteringMetallic` /
  `multiScatteringMetallic` / `radiance` / `iblIrradiance` block. It is
  emitted **even with no env map** (radiance and iblIrradiance are literal
  zero vec3s), so the port must emit the same dead algebra or prove the
  folded result is identical.
- `computeMultiscattering()` → `multiScatteringCompensation` (the
  `1/(dfg.x+dfg.y) - 1` energy-compensation factor applied to
  `directSpecular`).
- The specular AO term at the end: `pow(saturate(dotNV + AO), exp2(-16*rough + 1)) - 1 + AO`.

### 3.2 BRDF / BSDF — `src/nodes/functions/BSDF/`

- `V_GGX_SmithCorrelated.js` — emitted verbatim as a WGSL `fn`.
- `D_GGX.js` — emitted verbatim as a WGSL `fn`.
- `BRDF_GGX.js` — the caller; inlines `F_Schlick` (`exp2((-5.55473*x - 6.98316)*x)`).
- `F_Schlick.js` — inlined, not emitted as a function.
- `BRDF_Lambert.js` — the `* 0.3183098861837907` (1/π) term.
- **`DFGLUT.js`** — `DFGApprox` is *not* used on this build. Three ships a
  hard-coded 16×16 `Uint16Array` RG16F table and samples it at
  `(roughness, saturate(dotNV))`. Needs: a `DataTexture` path, `RGFormat`,
  `HalfFloatType`, `LinearFilter` min+mag, `ClampToEdgeWrapping`,
  `generateMipmaps = false`, and `rg16float` texture upload in the backend.
  This is the single largest *renderer* gap in the rung.
- `EnvironmentBRDF.js` — referenced by the indirect path; with no env map its
  contribution folds to zero, but the node graph still walks it.
- Not used here (skip): `BRDF_Sheen`, `LTC`, `D_GGX_Anisotropic`,
  `V_GGX_SmithCorrelated_Anisotropic`, `Schlick_to_F0`. **No sheen, no
  clearcoat, no iridescence, no transmission** — `MeshPhysicalNodeMaterial`
  is not needed at all; `MeshStandardNodeMaterial` is enough.

### 3.3 Material — `src/materials/nodes/MeshStandardNodeMaterial.js`

- `MeshStandardNodeMaterial` itself (`setupLightingModel` →
  `PhysicalLightingModel`, `setupSpecular`, `setupVariants`).
- `src/nodes/core/PropertyNode.js` — the named properties the dump shows as
  `var<private>`: `DiffuseColor`, `Metalness`, `Roughness`, `SpecularColor`,
  `SpecularColorBlended`, `SpecularF90`, `DiffuseContribution`,
  `EmissiveColor`, `Output`, `normalView`, `normalViewGeometry`,
  `normalWorld`, `positionViewDirection`, `shadowPositionWorld`,
  `directDiffuse`, `directSpecular`, `indirectDiffuse`, `indirectSpecular`,
  `irradiance`, `iblIrradiance`, `radiance`, `ambientOcclusion`,
  `totalDiffuse`, `totalSpecular`, `outgoingLight`.
  `SpecularColorBlended = mix(vec3(0.04), DiffuseColor.rgb, Metalness)` and
  `DiffuseContribution = DiffuseColor.rgb * (1 - metalness)` are the r186
  shape — do not port the older `diffuseColor *= 1-metalness` form.
- `src/nodes/functions/material/getRoughness.js` and
  `getGeometryRoughness.js` — `min(max(roughness*roughnessMap.g, 0.0525) + geometryRoughness, 1)`
  where `geometryRoughness = max3(max(abs(dFdx(normalViewGeometry)), abs(-dFdy(normalViewGeometry))))`.
  Needs `dpdx`/`dpdy` on a `vec3` varying in the port's WGSL emitter.
- `src/nodes/accessors/MaterialNode.js` — `materialColor`, `materialOpacity`,
  `materialMetalness`, `materialRoughness`, `materialEmissive`,
  `materialEmissiveIntensity`, `materialBumpScale`, and the
  `materialReference` uniforms behind them.
- `src/nodes/accessors/BumpMapNode.js` — **new**. Three samples the bump map
  three times (`uv`, `uv + dFdx(uv)`, `uv + -dFdy(uv)`), builds
  `dHdxy = vec2(hx-h, hy-h) * bumpScale`, then perturbs with a
  dFdx/dFdy-of-`positionView` frame and `f32(isFront)*2-1`. Distinct from
  rung 5's `normalMap` path; both live in `NormalMapNode.js`/`BumpMapNode.js`.
- `src/nodes/accessors/TextureNode.js` with a **uv transform matrix**: each
  map contributes a `mat3x3` object uniform and the uv is
  `(uvTransform * vec3(uv,1)).xy`. Rungs 3/5 may have gotten away with
  identity; here `repeat (10,24)` and `(1,1)` make it load-bearing, as does
  `Texture.matrixAutoUpdate` / `updateMatrix` in `src/textures/Texture.js`.
- `RepeatWrapping` in the sampler (`addressModeU/V = repeat`) — rung 4 noted
  the port is ClampToEdge-only. **Blocking.**
- `anisotropy = 4` on five textures → `maxAnisotropy` in the wgpu
  `SamplerDescriptor`, and the mip filter must be linear.

### 3.4 Lights — `src/nodes/lighting/`

- `PointLightNode.js` + `LightUtils.js` `getDistanceAttenuation` — the
  `if (cutoffDistance > 0)` two-branch falloff with `decay = 2` and the
  `saturate(1 - (d/cutoff)^4)²` window. Rung 5 delivers the node; confirm it
  emits the **cutoffDistance branch** (rung 5's light has `distance` default
  0 and may have folded the branch away).
- `HemisphereLightNode.js` — **new**: `mix(groundColor, skyColor, dot(normalWorld, normalize(lightDirection))*0.5 + 0.5)`,
  contributing to `irradiance`. Three uniforms: sky colour, ground colour,
  light-direction vec3 in view/world space.
- `LightsNode.js` — the multi-light accumulation order (point first, then
  hemisphere into `irradiance`) and the render-group light uniform packing.
- `src/lights/PointLight.js` `power` getter/setter (`intensity = power/(4π)`)
  and `src/lights/HemisphereLight.js`. Porting the `power` accessor matters:
  the example sets `power`, not `intensity`.
- `AnalyticLightNode.js` / `LightingContextNode.js` — the
  `directDiffuse`/`directSpecular`/`irradiance` context the lighting model
  writes into.

### 3.5 Shadows — the biggest single new piece

- `src/nodes/lighting/PointShadowNode.js` — **cube shadow**, new even after
  rung 7's spot+directional. What the dump shows:
  - a 512×512×**6-layer** `depth24plus` texture viewed as
    `texture_depth_cube`, plus a 512×512×6 `rgba8unorm` colour target
    (`PointShadowMap`) that is written but never read;
  - six render passes with the light's cube view matrices;
  - sampling with `textureSampleCompare` and a **negated Y**:
    `vec3(d.x, -d.y, d.z)`;
  - the distance-to-depth remap
    `((near + (-dist)) * far) / ((far - near) * (-dist)) + bias`, and the
    early-out `if (maxAbs - far <= 0 && maxAbs - near >= 0)`;
  - normal-offset: the sampled position is
    `shadowPositionWorld + normalWorld * normalBias`.
- `src/nodes/lighting/ShadowFilterNode.js` — `vogelDiskSample( i, 5, phi )`
  and `interleavedGradientNoise( fragCoord.xy )`, five taps averaged ×0.2,
  radius `radius / mapSize.x`. Both are emitted as WGSL `fn`s. Rung 7's
  filter is the 2D variant of the same file; the port must have the
  point/cube branch too.
- `src/nodes/lighting/ShadowBaseNode.js` / `ShadowNode.js` — the
  `shadowPositionWorld` property and `mix(1, shadow, shadowIntensity)`.
- `sampler_comparison` + `texture_depth_cube` binding types in the port's
  bind-group layout builder, and a `CompareFunction` sampler in wgpu.
- The shadow-pass material: the dump labels it `ShadowMaterial_24` and builds
  **one** pipeline for all casters — a depth-only pass whose fragment shader
  only evaluates `DiffuseColor.a` (map alpha × opacity) for alpha discard.
  It binds the object's diffuse map, so the port needs per-object bind groups
  against one shared pipeline.
- Renderer: `shadowMap.enabled`, a shadow render list, per-face viewport /
  layer render passes, and `depthCompare` for the shadow pass.

### 3.6 Renderer / backend gaps (`port/src/renderer`)

- `rg16float` texture format + half-float `DataTexture` upload (DFG LUT).
- 2D-array depth textures with a **cube** texture view
  (`wgpu::TextureViewDimension::Cube`) and `usage` including
  `RENDER_ATTACHMENT | TEXTURE_BINDING`.
- Comparison samplers (`compare: Some(CompareFunction::LessEqual)`).
- `SamplerDescriptor` address modes `Repeat` (rung 4 gap: ClampToEdge only)
  and `anisotropy_clamp`.
- `ReinhardToneMapping` in the output pass
  (`src/nodes/display/ToneMappingNode.js` + `ToneMappingFunctions.js`) —
  rung 7 brings ACES; Reinhard is a second, simpler entry in the same table:
  `clamp(c*exposure / (c*exposure + 1), 0, 1)`. `toneMappingExposure` is a
  render-group uniform.
- The output pass also emits the un-premultiply guard
  `if (color.a == 0) → vec4(0) else color.rgb/color.a` ahead of tone mapping
  — confirm rung 2/4's `outputColorTransform` matches.
- Object-group `mat3x3` uniform packing (uv transforms and the normal matrix)
  — WGSL `mat3x3<f32>` is three `vec4`-aligned columns; getting the stride
  wrong is a silent-wrong-output failure on this stack.
- Multiple textures per material (up to **five** samplers + textures in one
  bind group here, incl. the comparison pair) — check the port's group-1
  binding allocator ordering matches the dump (`sampler` then `texture`,
  ascending, after the uniform buffer at binding 0).
- `src/lights/…` world-matrix propagation: `bulbMat`'s mesh is a **child of
  the PointLight**, so the renderer must walk nested children. Rung notes say
  the renderer still walks a flat `Child` list and "folding `Child` into the
  tree is left for the first rung that nests" — **this is that rung.**

---

## 4. Step order for the rung worker

1. **Scene graph first.** Make the renderer walk the real `Object3D` tree
   (`scene-graph` branch landed the tree; the renderer still uses the flat
   list). The bulb mesh is a child of the light and will render at the origin
   otherwise. Cheap, and it unblocks everything after it.
2. **Sampler + texture plumbing**: `RepeatWrapping`, `anisotropy`, the
   per-texture uv `mat3x3` transform from `Texture.repeat/offset/center/rotation`.
   Verify against rung 3/5's still-green e2e before touching lighting.
3. **`MeshStandardNodeMaterial` with no lights and no shadow.** Get
   `DiffuseColor`/`Metalness`/`Roughness`/`SpecularColorBlended`/`EmissiveColor`
   and the DFG LUT sampling out, and diff the generated WGSL against
   `MeshStandardMaterial_17.frag.wgsl` (bulbMat is the minimal program — no
   maps, no shadow). Getting 17 byte-comparable is the fastest real signal in
   this rung.
4. **DFG LUT**: port the `Uint16Array` verbatim, add `rg16float` upload, add
   the `DataTexture` path. Check the sampled values against Three by dumping
   `dfg` for a couple of (roughness, dotNV) pairs.
5. **`PhysicalLightingModel` direct path** + `PointLightNode` with
   `cutoffDistance`/`decay`. Target: `MeshStandardMaterial_20`
   (cubeMat: map + bump, no shadow, no roughnessMap).
6. **`BumpMapNode`** — three-tap dHdxy and the dFdx/dFdy frame. Hardest thing
   to get right by inspection; compare WGSL literally.
7. **`HemisphereLightNode`** + the indirect/multiscattering block. Target:
   `MeshStandardMaterial_19` (ballMat, metalness 1.0 — the metallic branch of
   multiscattering only shows up here).
8. **Point/cube shadows** last: six-face render, cube depth view, comparison
   sampler, `vogelDiskSample` filter. Target: `MeshStandardMaterial_18`.
9. **Reinhard tone mapping + exposure** in the output pass, then grade.

Grade after 3, 5, 7 and 9 with the diff image, not with draw counts.

## 5. Traps

- **No PMREM, no env map prefiltering, no HDR/EXR.** The indirect block is
  still emitted with zero radiance — resist the urge to delete it, and check
  the folded result rather than assuming.
- **`Math.random` is never called.** Nothing to replicate. But `Date.now()`
  is: `time = 0` → `bulbLight.position.y = cos(0)*0.75 + 1.25 = 2.0`, which
  coincidentally equals the initial `position.set(0,2,0)`. Do not "simplify"
  by dropping the animate assignment without checking the arithmetic.
- **First-frame state is the whole test.** `animate()` runs once *before* the
  render and mutates four things: `toneMappingExposure = 0.68^5`,
  `bulbLight.power = 400`, `bulbMat.emissiveIntensity = 79577.47154594767`,
  `hemiLight.intensity = 0.0001`. All four differ from the constructor
  values. `params.shadows !== previousShadowMap` is `true !== false` on the
  first frame, so all three materials get `needsUpdate = true` — irrelevant
  for the port, but it is why Three recompiles.
- **Texture colour spaces are mixed and one is arguably wrong.**
  `hardwood2_diffuse`, `brick_diffuse`, `earth_atmos` are sRGB → upload as
  `rgba8unorm-srgb` and let the GPU decode (rung 3's finding). `earth_specular`
  is *also* marked sRGB even though it is a metalness map — reproduce it.
  `hardwood2_bump`, `hardwood2_roughness`, `brick_bump` are **linear** →
  `rgba8unorm`. The dump confirms: three srgb mip chains, three linear ones,
  and two different mipmap pipelines (`mipmap-rgba8unorm-srgb-2d-array` and
  `mipmap-rgba8unorm-2d-array`).
- **Mip generation must match rung 3's box-blit exactly** for six textures
  with 11–12 mip levels each. The floor at `repeat (10,24)` under a grazing
  camera is sampling deep into the mip chain over most of the image; a mip
  that is off by a rounding step will move a large fraction of pixels at
  once.
- **JPEG decode residue.** Rung 4 logged `zune-jpeg` vs `libjpeg-turbo`
  differing by ≤3/channel. Seven JPEGs here, and the floor is a large
  low-contrast gradient — this is the rung where that residue is most likely
  to compound. Watch it; do not fudge it.
- **No MSAA.** `antialias` is not passed. If the port hardcoded
  `sampleCount 4` for the frame target anywhere after rung 2, it must become
  conditional.
- **`needs_frame_buffer_target` is hardcoded** (rung 4 gap) — fine here,
  since there *is* a frame target + output pass, but the exposure uniform has
  to reach it.
- **Half-float target already exists** (`rgba16float` frame target from rung
  2); the new half-float need is the `rg16float` LUT, which is a *sampled*
  texture, not a render target.
- **`f32(isFront)` in the bump frame** needs `@builtin(front_facing)` as a
  fragment input; the floor is single-sided and rotated, so the sign matters.
- **Very large emissiveIntensity (79577)** in an rgba16float target: the bulb
  mesh is off-screen so it never lands in the image, but a wrong clamp or an
  f16 overflow in the frame target would show up as `inf`/black. Half-float
  max is 65504 — `emissive(0xffffee) * 79577` **overflows f16** on the G and
  B channels. Three is storing `inf` there and it is off-screen; if the port
  clips the geometry differently it will surface. Do not "fix" it.

## 6. What the HANDOFF rules make hard

- **Addons**: only two, both harmless. `OrbitControls` is used for its
  constructor-time `update()` only — port that as a plain `camera.lookAt(0,0,0)`
  equivalent (same as rung 1 did), not as a controls implementation.
  `Inspector` (`examples/jsm/inspector/Inspector.js`) is GUI-only and
  contributes no pixels; skip it entirely. The grader's `clean-page.js`
  removes the DOM anyway.
- **Loaders**: `TextureLoader` on seven JPEGs. No new loader class; rungs 3
  and 5 already decode JPEG. No `BufferGeometryLoader`, no GLTF, no RGBELoader.
- **HDR/EXR**: none. This rung needs no `.hdr`, no `.exr`, no cube map, no
  PMREM — a real advantage over any env-map-based PBR example.
- **Geometry**: `SphereGeometry`, `PlaneGeometry`, `BoxGeometry` — all in the
  merged `geometries` branch. No addon geometry (unlike rung 5's Teapot).
- **The DFG LUT array** is the one piece of bulk constant data to copy. It
  comes from `src/nodes/functions/BSDF/DFGLUT.js`, which is Three's own
  source, not the reference image — porting it is in the same class as
  porting a geometry constant, and it is the only way to match Three's
  specular exactly.

## 7. Vendor tree state

`_dump_rung8.mjs` and `.puppeteer_profile_rung8/` were deleted after the
dumps were taken. `~/src/vendor/three.js` ends at `M test/e2e/puppeteer.js`
(the rung 0 flags patch) and nothing else.

(At the time of writing, `test/e2e/_dump_rung7.mjs`, `_dump_rung9.mjs` and
their profile dirs were also present — those belong to the rung 7 and rung 9
scouts running in parallel and were left alone.)
