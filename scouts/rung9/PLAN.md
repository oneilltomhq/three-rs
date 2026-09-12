# Rung 9 scout — postprocessing

Scouted 2026-09-12. three.js @ 3d010ef (`r186dev`), grader as patched by rung 0.
Plain `webgpu_postprocessing` dropped at rung 0 (0.1% with Three itself).
Seven 0.0% candidates from `handoff/rung0/e2e-run7-rung9-candidates.log`.

**Recommendation: `webgpu_postprocessing_masking`. Second choice: `webgpu_postprocessing_direct`.**

Reference screenshots are next to this file as `ref_<name>.jpg` (copies of
`~/src/vendor/three.js/examples/screenshots/webgpu_postprocessing_<name>.jpg`).
Real-page dumps for the recommendation are in `dump/` and as
`scene_basic.{vert,frag}.wgsl`, `output_quad.{vert,frag}.wgsl`, `mipmap.wgsl`.

---

## 1. The seven candidates

### `_masking` — 135 lines — ref_masking.jpg  ★ RECOMMENDED
- **Scene:** three `Scene`s, one shared `PerspectiveCamera(50, …, 1, 1000)` at z=10.
  `baseScene` has **no meshes at all**, only `background = Color(0xe0e0e0)`.
  `maskScene1` = `Mesh(BoxGeometry(4,4,4))`, `maskScene2` = `Mesh(TorusGeometry(3,1,16,32))`,
  both with **no material argument** → Three's default `MeshBasicMaterial` (white),
  which under WebGPURenderer is a `MeshBasicNodeMaterial`.
- **Lights:** none. **Loaders:** `TextureLoader` ×2 (JPEG). **Addons:** `Inspector` only
  (GUI; the e2e injection/clean-page makes it invisible, and it emits no draws).
- **Assets:** `examples/textures/758px-Canestra_di_frutta_(Caravaggio).jpg` (758×600,
  `minFilter=LinearFilter`, `generateMipmaps=false`, `flipY=false`, SRGBColorSpace) and
  `examples/textures/2294472375_24a3b8ef46_o.jpg` (4096×2048, default filters →
  13 mip levels, `flipY=false`, SRGBColorSpace). Both already-supported formats
  (rung 3/4 decode JPEG).
- **Postprocessing nodes:** `pass()` ×3 (`src/nodes/display/PassNode.js`), `.a` on two of
  them (swizzle on the pass's `PassTextureNode`), `texture()`
  (`src/nodes/accessors/TextureNode.js`), `.mix()`. Output goes through
  `RenderPipeline` (`src/renderers/common/RenderPipeline.js`; `PostProcessing.js` is now
  just a deprecated subclass of it) with the **default** `outputColorTransform = true`, so
  `RenderOutputNode` (`src/nodes/display/RenderOutputNode.js`) is appended, plus
  `PremultiplyAlphaFunctions.js` (unpremultiply/premultiply) and
  `ColorSpaceFunctions.js` (`sRGBTransferOETF`). **No addon display node at all.**
- **Programs:** **2** (plus Three's own raw `mipmap` blit, which the port already has):
  one `MeshBasicNodeMaterial` vert+frag shared by box *and* torus, and one
  `RenderPipeline` quad vert+frag. **Render passes: 4** (1 clear-only + 2 scene + 1 canvas),
  plus 12 mipmap blits for the 4096×2048 JPEG.
- **Determinism:** no `Math.random`. `animate()` uses `performance.now()*0.001 + 6000`
  and the injection pins `performance.now` to 0, so `time === 6000` exactly. Pure
  trig on a constant — the safest animation state of the seven.

### `_direct` — 128 lines — ref_direct.jpg  ☆ SECOND CHOICE
- **Scene:** 100 `Mesh(SphereGeometry(1,4,4), MeshPhongMaterial{color: random, flatShading:true})`
  under one `Object3D` (so a real nested transform), `AmbientLight(0xcccccc)`,
  `DirectionalLight(0xffffff, 3)`. `background = Color(0x000000)`.
  `renderer.toneMapping = NeutralToneMapping`.
- **Loaders/addons/assets:** none (except `Inspector`).
- **Postprocessing nodes:** `output` (`src/nodes/core/PropertyNode.js`:
  `nodeImmutable( PropertyNode, 'output', 'Output' )`), `saturation`
  (`src/nodes/display/ColorAdjustment.js`), `uniform`, `vec4`.
- **Mechanism: `DirectRenderPipeline` (`src/renderers/common/DirectRenderPipeline.js`),
  not `RenderPipeline`.** There is **no `pass()`, no intermediate render target and no
  output quad** — the `outputNode` is spliced into each scene material's own fragment
  output. So it is the *smallest* example but it does **not** force the PostProcessing
  core the rung is supposed to buy. Also needs `flatShading` (a new NodeMaterial flag),
  `DirectionalLight`, `NeutralToneMapping` and 500 `Math.random()` draws in a strict order.
- **Programs:** 1 scene program (all 100 meshes share it) + the output splice inside it.

### `_bloom` — 194 lines — ref_bloom.jpg
- **Scene:** `GLTFLoader` → `models/gltf/PrimaryIonDrive.glb`, `AnimationMixer` playing
  `gltf.animations[0].optimize()`, `Timer`, `AmbientLight`, `PointLight(100)` parented to
  the camera, `OrbitControls`. `antialias: true` (MSAA) with an explicit
  `storeMultisampledColorBuffer/…/resolveDepthBuffer` config on `pass()`.
  `ReinhardToneMapping`.
- **Postprocessing:** `pass()` + `getTextureNode('output')` (MRT-name access) +
  `bloom()` from `examples/jsm/tsl/display/BloomNode.js`, which is a 5-level
  mip pyramid: high-pass + 5 downsample + 5 upsample/blur passes
  (it pulls `GaussianBlurNode`-style separable blur internally) → ~12 extra passes
  and ~7 extra programs.
- **Verdict:** depends on rung 10's `GLTFLoader` + the animation merge, adds MSAA resolve
  policy, emissive PBR and a whole mip-pyramid effect. Most representative of later
  needs, wrong first step.

### `_anamorphic` — 202 lines — ref_anamorphic.jpg
- **Scene:** `InstancedMesh(SphereGeometry(0.1,32,32), MeshBasicNodeMaterial, 200)` with
  per-instance matrix + colour from `Math.random`, `scene.backgroundNode` as a TSL `Fn`
  using `screenUV`, `positionNode` animating with `time` + `instanceIndex`,
  `OrbitControls`, `NeutralToneMapping`, `antialias: true`.
- **Postprocessing:** `pass()`, `bloom()` (addon) with `setResolutionScale(0.25)` and a
  **replaced `highPassFn`** using `rtt()` (`src/nodes/utils/RTTNode.js`), `Loop()` with a
  uniform bound (80 iterations!), `viewportSize`, `luminance`, `MirroredRepeatWrapping`.
- **Verdict:** the heaviest node-graph of the seven. `rtt()` inside a function, dynamic
  `Loop`, mirrored wrapping, resolution-scaled targets. No.

### `_ca` — 350 lines — ref_ca.jpg
- **Scene:** `PMREMGenerator` + `RoomEnvironment` addon (a whole cube-render +
  spherical-harmonics path), `GridHelper`, `Points` + `PointsMaterial` with a hand-built
  `BufferGeometry`, ~8 geometry types, many `MeshStandardMaterial`s, `Group`s, `Timer`,
  `OrbitControls`, `antialias`.
- **Postprocessing:** `pass()`, `renderOutput()` (`RenderOutputNode.js`) with
  `outputColorTransform = false`, `chromaticAberration()` from
  `examples/jsm/tsl/display/ChromaticAberrationNode.js`.
- **Verdict:** the postprocessing part is cheap; the *scene* is the most expensive of the
  seven (PMREM + RoomEnvironment + Points). Rung 0 already flagged sub-pixel `Points`
  coverage as the `webgpu_camera` killer. No.

### `_3dlut` — 279 lines — ref_3dlut.jpg
- **Scene:** `GLTFLoader` → `models/gltf/coffeeMug.glb` (baked map, `anisotropy = 8`),
  a `PlaneGeometry(1,1,16,64)` smoke mesh with a full custom TSL `positionNode`
  (twist via `rotateUV`, wind) and `colorNode` (noise, `smoothstep` edge fades),
  `textures/noises/perlin/128x128.png` with `RepeatWrapping`, `OrbitControls`, `antialias`.
- **Postprocessing:** `pass()`, `renderOutput()`, `outputColorTransform = false`,
  `texture3D()` (`src/nodes/accessors/Texture3DNode.js`), `lut3D()` from
  `examples/jsm/tsl/display/Lut3DNode.js`.
- **Loaders:** `LUTCubeLoader`, `LUT3dlLoader`, `LUTImageLoader` — **three** addon loaders,
  nine LUT assets (`examples/luts/*.CUBE`, `Presetpro-Cinematic.3dl`, `NeutralLUT.png`,
  `B&WLUT.png`, `NightLUT.png`) all loaded before first frame, plus a 3D texture type the
  port has never had. No.

### `_transition` — 282 lines — ref_transition.jpg
- **Scene:** two `FXScene`s, each an `InstancedMesh` (Box / Icosahedron, 20×20×20 grid-ish
  count) with per-instance colour, `AmbientLight(3)` + `DirectionalLight(3)`,
  `MeshPhongNodeMaterial{flatShading}`, its own camera, `Timer`,
  **`TWEEN` addon** (`examples/jsm/libs/tween.module.js`) driving the transition value.
- **Postprocessing:** `pass()` ×2, `TextureNode` built by hand, `transition()` from
  `examples/jsm/tsl/display/TransitionNode.js`, 6 `textures/transition/transition*.png`.
- **Verdict:** close to `_masking` in shape (two passes mixed by a texture) but adds
  TWEEN time-dependence, two cameras, instancing, Phong, flatShading and 6 PNGs. A good
  *follow-up* to `_masking`, not the first.

### Recommendation rationale
`_masking` is the only candidate that forces the whole `pass()` + `RenderPipeline` +
`RenderOutputNode` core and **nothing else**: no lights, no addon display node, no new
geometry (Box and Torus are both already in `port/src/geometries`), no new material
(default `MeshBasicNodeMaterial`, which rung 1 already drives), no `Math.random`, and a
frozen animation time. Everything it adds is reusable by every later postprocessing
example: multiple passes per frame, a pass's colour *and* alpha, half-float intermediate
targets, a fullscreen triangle quad program, and the sRGB output transform moving out of
the scene pass into the pipeline's own quad. It is also the *only* candidate whose three
scenes share one program, so the shader-generation surface is two programs total.

`_direct` is second because it is genuinely tiny and would land `DirectRenderPipeline`,
`output` and `saturation`, but it buys no `pass()` and no render target, so it would not
discharge the rung's stated purpose; keep it as the fallback if `_masking`'s alpha
handling turns out to be a rabbit hole.

---

## 2. `_masking` — the real page, dumped

Dumped with `test/e2e/_dump_rung9.mjs` (temporary, deleted; modelled on
`test/e2e/puppeteer.js` — `--use-angle=vulkan`, no `--disable-vulkan-surface`,
`userDataDir: ./.puppeteer_profile_rung9`, port 1241, viewport 800×500, the e2e
`deterministic-injection.js` + `clean-page.js`, builds patched the same way).
Raw JSON in `dump/` (`pipelines.json`, `layouts.json`, `passes.json`, `textures.json`,
`actual_full.png`).

**5 shader modules, 3 pipelines, 2 bind-group layouts, 16 render passes, 9 textures.**

### Programs
| file | label | entry | notes |
|---|---|---|---|
| `scene_basic.vert.wgsl` | `vertex` | `main` | one vertex buffer, stride 12, `float32x3` position only (no uv, no normal — `MeshBasicNodeMaterial` with a plain colour needs neither) |
| `scene_basic.frag.wgsl` | `fragment` | `main` | target `rgba16float`, writeMask 15 |
| `output_quad.vert.wgsl` | `vertex_RenderPipeline` | `main` | one vertex buffer, stride 8, `float32x2` **uv**; clip position is built from `array<f32,3>(-1,-1,3)[vertexIndex]` / `(3,-1,-1)[vertexIndex]` — the fullscreen-triangle `QuadMesh` (draw 3 vertices, non-indexed) |
| `output_quad.frag.wgsl` | `fragment_RenderPipeline` | `main` | target **`rgba8unorm`** (not `-srgb`), writeMask 15 |
| `mipmap.wgsl` | `mipmap` | `main_2d_array` | Three's own raw WGSL blit; `port/src/renderer/shaders/mipmap.wgsl` already matches |

`scene_basic.frag.wgsl` is worth noting for how little it is: **no fragment inputs at
all** (no uv, no normal, no varyings), one `objectStruct` at `@binding(0) @group(1)` holding
`nodeUniform0: vec3<f32>` (diffuse), `nodeUniform1: f32` (opacity) and
`nodeUniform4: mat4x4<f32>`, and the body
`DiffuseColor = vec4(diffuse,1); DiffuseColor.a *= opacity; DiffuseColor.a = 1.0;
output.color = max( DiffuseColor, vec4(0) )`. That is the whole "two white meshes" half of
the example — the port's rung-1/4 `MeshBasicNodeMaterial` path should already emit this
modulo the binding layout.

Both render pipelines: `topology triangle-list`, `frontFace ccw`, `cullMode back`,
`depthStencil { depth24plus, depthWriteEnabled: true, depthCompare: "less-equal" }`,
`multisample count 1` (no MSAA: `_masking` does not pass `antialias`).
Note the canvas pipeline **also** carries a depth attachment and back-face culling.

### Bind groups
- Scene program: two groups, each a single `binding 0, visibility 7 (VERT|FRAG|COMP), buffer{}`
  (the usual render-uniform group + object group).
- Output quad program, one group:
  `0 sampler / 1 texture_2d<f32>` (base pass colour),
  `2 sampler / 3 texture_2d<f32>` (`texture1` JPEG),
  `4 var<uniform> object { nodeUniform2: mat3x3, nodeUniform5: mat3x3 }` — the two
  **texture matrices** for the loaded textures, interleaved *between* the texture bindings,
  `5/6` (mask1 pass colour), `7/8` (`texture2` JPEG), `9/10` (mask2 pass colour).
  Binding indices are assigned in node-visit order, with the uniform buffer taking the
  next free slot when the first non-texture uniform appears — the port's binding
  allocator has to reproduce that interleave exactly or the layouts will not match.

### Fragment body (the whole effect)
```
nodeVar0 = textureSample( basePassColor, uv )
nodeVar2 = textureSample( texture1, ( texMat1 * vec3(uv,1) ).xy )
nodeVar3 = textureSample( mask1PassColor, uv )
nodeVar5 = textureSample( texture2, ( texMat2 * vec3(uv,1) ).xy )
nodeVar6 = textureSample( mask2PassColor, uv )
nodeVar8 = mix( mix( base, tex1, mask1.a ), tex2, mask2.a )
nodeVar9 = unpremultiplyAlpha( vec4( nodeVar8.rgb, clamp( nodeVar8.a, 0, 1 ) ) )
output.color = premultiplyAlpha( vec4( sRGBTransferOETF( nodeVar9.rgb ), nodeVar9.a ) )
```
So `RenderOutputNode` with `NoToneMapping` emits exactly
**unpremultiply → sRGB OETF → premultiply**, guarded by `if (color.a == 0) return vec4(0)`
in the unpremultiply helper (`PremultiplyAlphaFunctions.js`). `sRGBTransferOETF` uses the
`pow(c, 0.41666)` form with `mix(…, …, vec3(c <= 0.0031308))` — note the `0.41666`
constant, not `1/2.4`-derived, and the `mix` on a bool-vector, which WGSL accepts via the
`vec3<f32>(bvec3)` cast Three emits.

### Per-frame pass sequence (observed `beginRenderPass` order)
| seq | target | size | format | samples | load/store | clear | depth | draws |
|---|---|---|---|---|---|---|---|---|
| 0 | **canvas** | 800×500 | `rgba8unorm` | 1 | clear/store | 0,0,0,0 | depth24plus clear/store | `renderPipeline_RenderPipeline_19`, `draw(3,1,0,0)` |
| 1 | RT "output" #1 (base) | 800×500 | `rgba16float` | 1 | clear/store | **0.7454042095350284** ×3, a=1 | own depth24plus | none (baseScene has no meshes) |
| 2 | RT "output" #2 (mask1) | 800×500 | `rgba16float` | 1 | clear/store | 0,0,0,**0** | own depth24plus | basic program, `drawIndexed(36,1,0,0,0)` (box) |
| 3 | RT "output" #3 (mask2) | 800×500 | `rgba16float` | 1 | clear/store | 0,0,0,**0** | own depth24plus | basic program, `drawIndexed(3072,1,0,0,0)` (torus) |
| 4–15 | `texture2` mips 1..12 | 4096×2048 ↓ | `rgba8unorm-srgb` | 1 | clear/store | — | none | mipmap blit |

- **Beware:** that is the order the `beginRenderPass` *calls* happen in, not necessarily
  GPU execution order — each is a separate command encoder/submit, and the three scene
  passes must execute before the canvas pass for the frame to be correct. The port should
  simply run passes → canvas; do not read pass 0 appearing first as "Three draws the quad
  first".
- `0.7454042095350284` is `0xe0/255 = 0.8784…` through sRGB→linear: the background clear
  colour is converted on the CPU because the target is linear `rgba16float`. The two mask
  scenes have `background === null` → clear to **transparent** `(0,0,0,0)`; that zero alpha
  is what `.a` reads as the mask, so getting the clear-alpha right *is* the example.
- No `setViewport`/`setScissorRect` calls at all; no MRT (single colour attachment per pass);
  no depth-texture sampling.
- Textures: 3 × `rgba16float` colour + 3 × `depth24plus` (`label: "depth"`, usage 23 =
  `COPY_SRC|TEXTURE_BINDING|RENDER_ATTACHMENT`), `texture1` 758×600 `rgba8unorm-srgb`
  `mipLevelCount 1`, `texture2` 4096×2048 `rgba8unorm-srgb` `mipLevelCount 13`.
  Each `pass()` allocates its **own** depth texture — they are not shared.

---

## 3. Gap list against `port/src`

`port/src/nodes` is `{builder, mod, node, tsl, wgsl}.rs`; `port/src/renderer` is
`{mipmap, mod, present, programs, render_target}.rs` + `shaders/`. Rungs 1–4 are in;
5–8 are assumed landed before this rung. Missing, by Three source file:

**Pipeline / renderer**
- `RenderPipeline` — `src/renderers/common/RenderPipeline.js` (the `outputNode` +
  `needsUpdate` + `render()` object the example constructs).
  It owns `outputNode`, `outputColorTransform` (default `true`), `needsUpdate`, a
  `QuadMesh` whose `material.fragmentNode` is set to
  `renderOutput( outputNode, renderer.toneMapping, renderer.outputColorSpace )`, and
  `render()` → `this._quadMesh.render( renderer )`. `PostProcessing.js` is only a
  deprecated alias subclass — port `RenderPipeline`, not `PostProcessing`.
- `QuadMesh` — `src/renderers/common/QuadMesh.js`. Port has a quad for rung 1/4 but not
  Three's `QuadMesh` with its own `NodeMaterial` and `render(renderer)` entry.
- The fullscreen-**triangle** vertex trick (`array<f32,3>[vertexIndex]`) rather than a
  two-triangle quad — `src/renderers/common/QuadMesh.js` geometry, and the `vertexIndex`
  builtin plus indexed-`array` constructor in the WGSL emitter (`port/src/nodes/wgsl.rs`).
- `RenderContext` / `RenderContexts` — `src/renderers/common/RenderContext.js`,
  `RenderContexts.js`: one render context **per render target**, so three independent
  contexts alive in one frame. The port currently has a single implicit context.
- `Renderer.setRenderTarget` / `_renderScene` nesting —
  `src/renderers/common/Renderer.js`: rendering scene→RT while a higher-level render is
  in flight, and `renderer.getDrawingBufferSize()` feeding pass target sizes.
- `Renderer.needsFrameBufferTarget` / `_renderOutput` must now be **false/skipped** for the
  scene passes and the conversion must move to the pipeline quad. Rung 2's note
  (`needs_frame_buffer_target` hardcoded) is exactly the thing to unpick here.
- Clear-colour/alpha per render target, including `background === null` → `(0,0,0,0)` —
  `src/renderers/common/Background.js` + `Renderer.js` `_clearColor`/`Color4.js`.
  Port must also do the sRGB→linear clear conversion for float targets (rung 2 knows this).
- Per-target depth texture allocation and `depthBuffer`/`stencilBuffer` flags —
  `src/core/RenderTarget.js`, `src/renderers/common/Textures.js`.
- `HalfFloatType` render targets (`rgba16float`) as the *pass* default —
  `src/nodes/display/PassNode.js` (`this._textureType = HalfFloatType`), `src/constants.js`.

**Nodes**
- `PassNode` + `PassTextureNode` + `PassMultipleTextureNode` + `pass()` —
  `src/nodes/display/PassNode.js`. Needs: `renderTarget` ownership, `setSize`,
  `updateBeforeType = NodeUpdateType.FRAME`, `updateBefore()` doing the nested render,
  `getTextureNode(name = 'output')` / `getTexture(name)` (the name→texture map, where
  `'output'` is `renderTarget.texture.name` and `'depth'` the `DepthTexture`), `.a` swizzle
  on the resulting texture node, and the `toInspector()` no-op.
  Construction is `new RenderTarget( w, h, { type: HalfFloatType, ...options } )` with
  `texture.name = 'output'` and a `DepthTexture` named `'depth'` whenever
  `options.depthBuffer !== false` (PassNode.js:246-256) — that is exactly the three
  `rgba16float` + `depth24plus` pairs seen in the dump.
  **Not needed for `_masking`:** `setMRT`/`getMRT`/`MRTNode`
  (`src/nodes/core/MRTNode.js`), `getPreviousTexture*` (`PassMultipleTextureNode`),
  `getViewZNode`/`getLinearDepthNode` and `PassNode.DEPTH` scope — stub them or leave
  them out; the first example that needs them is an AO/TRAA one, not this rung.
- `RenderOutputNode` + `renderOutput()` — `src/nodes/display/RenderOutputNode.js`
  (tone mapping + colour space + premultiply ordering; `_masking` is the
  `NoToneMapping` + `SRGBTransferOETF` path).
- `ColorSpaceNode` + `sRGBTransferOETF` — `src/nodes/display/ColorSpaceNode.js`,
  `src/nodes/display/ColorSpaceFunctions.js`. Emit the `0.41666` form verbatim.
- `premultiplyAlpha` / `unpremultiplyAlpha` —
  `src/nodes/display/PremultiplyAlphaFunctions.js` (and the `a == 0` early-out).
- `ScreenNode` (`screenUV`, `viewportSize`, `screenCoordinate`) —
  `src/nodes/display/ScreenNode.js`. `_masking` itself does not use it, but `PassNode`'s
  sizing and every later example do; cheap to land now.
- `texture()` with a **texture matrix** uniform (`mat3x3` in the object group) —
  `src/nodes/accessors/TextureNode.js` + `src/nodes/accessors/UVNode.js`.
  Rung 4 has `texture()`; the `uvNode == null` → `texture.matrix * vec3(uv,1)` path with
  `flipY = false` and `Texture.updateMatrix()`/`matrixAutoUpdate` may not be in yet
  (`src/textures/Texture.js`).
- `mix()` as a method on a node (`maskAlpha.mix(a, b)` = `mix(a, b, maskAlpha)`) and the
  `.a`/`.w` swizzle on a texture-sample result — `src/nodes/math/MathNode.js`,
  `src/nodes/core/Node.js` swizzle proxy.
- `NodeMaterial` for the quad: no geometry attributes but `uv`, `depthTest`/`depthWrite`
  defaults that still produce a depth attachment — `src/materials/nodes/NodeMaterial.js`.
- `diagnostic( off, derivative_uniformity )` + `var<private> output : OutputStruct`
  preamble for fragment stages that sample inside helper fns — `src/nodes/core/NodeBuilder.js`
  / `src/renderers/webgpu/nodes/WGSLNodeBuilder.js`. (Already present in rung 4 output? verify.)
- Binding-index allocation that interleaves the object uniform buffer between texture
  pairs — `src/renderers/webgpu/utils/WebGPUBindingUtils.js` +
  `src/renderers/common/Bindings.js`.

**Render-target features not needed by `_masking`** (record as *not* required, so the
worker does not build them): MRT / multiple colour attachments, depth-texture *reads*,
viewport/scissor, mip chains on render targets, MSAA resolve. The only mip chain is the
ordinary texture-upload one the port already has (`port/src/renderer/mipmap.rs`).

---

## 4. Step order for the rung worker

1. Port `QuadMesh` + the fullscreen-triangle vertex path and confirm the emitter produces
   `output_quad.vert.wgsl` byte-for-byte (`array<f32,3>(…)[ vertexIndex ]`, uv varying at
   `@location(0)`). Cheapest possible check that `vertexIndex` and indexed-array
   constructors work.
2. `RenderTarget`/`RenderContext` multiplicity: allocate a `rgba16float` colour +
   `depth24plus` depth per target, render the existing rung-1 scene into one, blit it to
   the canvas with the quad. Keep rungs 1–8 green through this (they currently rely on the
   single hardcoded frame-buffer target).
3. `PassNode` with `updateBefore` nested rendering: `pass(scene, camera)` → one RT, one
   `PassTextureNode`. Then `getTextureNode('output')` and `.a`.
4. `RenderPipeline`/`PostProcessing` with `outputNode`, and **move** the sRGB output
   transform: delete the scene-pass output conversion for the postprocessing path and
   emit `RenderOutputNode` into the quad fragment instead. Diff against
   `output_quad.frag.wgsl`.
5. Background/clear per target: `0xe0e0e0` → linear `0.7454042095350284` for the base
   pass, `(0,0,0,0)` for the two mask passes.
6. Load the two JPEGs with `flipY = false` + `generateMipmaps = false` on the first, then
   wire `texture()`'s `mat3x3` texture matrix and the `mix` chain. Diff the fragment.
7. Run the grader. Expect 0 or a handful of pixels; the JPEG-decode residue from rung 4
   (`zune-jpeg` ≤3/channel vs libjpeg-turbo) applies to both textures here and is the most
   likely source of any non-zero count.

### Traps
- **Where sRGB goes.** Rungs 1–4 convert in a dedicated output pass
  (`output_color_transform.wgsl` lineage). Under `RenderPipeline` the conversion belongs
  *only* in the quad fragment, and the canvas format is plain `rgba8unorm` (no
  `-srgb` view doing it for free). Doing it twice, or leaving it in the scene pass, is the
  classic silent-wrong-output failure on this stack.
- **Premultiply sandwich.** `RenderOutputNode` wraps the colour in
  unpremultiply → OETF → premultiply *even when nothing is transparent*. With `alpha == 1`
  it is identity, but the unpremultiply helper's `a == 0 → vec4(0)` branch matters for the
  mask-scene pixels that survive into `mix`. Implement the branch, do not "optimise" it.
- **Clear alpha is the effect.** `maskScene1/2` have `background === null`, so their pass
  clears to alpha 0 and the box/torus write alpha 1. If the port clears RT alpha to 1
  (a very easy default) the whole image becomes the second JPEG and the diff will be ~100%
  — but it will not error.
- **Half-float intermediates.** `rgba16float`, not `rgba8unorm`. Values round-trip through
  f16; matching Three means writing linear values to f16 and converting on the quad, not
  storing sRGB 8-bit.
- **Texture filtering.** `texture1` is `LinearFilter` with **no mipmaps** (so
  `mipmapFilter` must not be set to a level the one-level texture cannot serve);
  `texture2` is default `LinearMipmapLinearFilter` with 13 levels, and its mip chain is
  Three's own per-level bilinear box blit — already ported at rung 3, reuse it unchanged.
  Both are `flipY = false`, which is *not* Three's default; the texture matrix in the
  `object` uniform block is where that shows up.
- **`Math.random` order.** `_masking` uses none — a deliberate advantage. (`_direct`,
  `_anamorphic`, `_transition` all do, and would need the f64 `sin(seed++)` sequence
  reproduced draw-for-draw.)
- **First-frame state.** `animate()` runs once with `performance.now() === 0`, so
  `time = 6000` exactly; `box.position.x = cos(4000)`, `box.position.y = sin(6000)`,
  `box.rotation.x = 6000`, `box.rotation.y = 3000`, torus likewise with `cos(6000)` /
  `sin(4000)`. Compute these in f64 and do **not** wrap the angles; `Euler`→`Quaternion`
  on a 6000 rad angle is where a subtle f32 divergence would enter.
- **Pass order vs. capture order.** See the table note in §2: the canvas
  `beginRenderPass` is issued first in the dump but the scene passes are separate submits
  that must land first.
- **The depth attachment on the quad pipeline.** Three still attaches a depth texture and
  `cullMode: back` to the canvas pass. Harmless for a fullscreen triangle wound ccw, but
  if the port winds the triangle the other way, back-face culling drops it and the canvas
  stays at the clear colour — silently.

---

## 5. What the HANDOFF rules make hard

- **Addons:** `_masking` needs **none**. Its only addon import is
  `examples/jsm/inspector/Inspector.js`, which is a GUI overlay — the e2e
  `clean-page.js` hides it and it issues no draws, so the ported scene simply omits it
  (the same way earlier rungs omit `OrbitControls` beyond its initial `lookAt`; here there
  is no `OrbitControls` at all). Every other candidate needs at least one
  `examples/jsm/tsl/display/*` node, and `_3dlut` needs three loaders plus a 3D texture.
- **Loaders:** two JPEGs through `TextureLoader`. Both formats the port already decodes
  (rung 3/4). Keep rung 4's `zune-jpeg` residue note in mind as the expected diff floor.
- **Assets** (all already in the vendor checkout, nothing to fetch):
  - `~/src/vendor/three.js/examples/textures/758px-Canestra_di_frutta_(Caravaggio).jpg`
  - `~/src/vendor/three.js/examples/textures/2294472375_24a3b8ef46_o.jpg`
  - reference: `~/src/vendor/three.js/examples/screenshots/webgpu_postprocessing_masking.jpg`
  - Note the parentheses in the first filename — quote it in any shell/`include_bytes!` path.
- For the second choice `_direct`: no assets at all, but it needs `DirectRenderPipeline`,
  `flatShading` on a Phong node material, `NeutralToneMapping`
  (`src/nodes/display/ToneMappingFunctions.js`) and the 500-call `Math.random` order.
- For reference, the asset cost of the ones *not* recommended:
  `models/gltf/PrimaryIonDrive.glb` (`_bloom`), `models/gltf/coffeeMug.glb` +
  `luts/{Bourbon 64,Chemical 168,Clayton 33,Cubicle 99,Remy 24}.CUBE` +
  `luts/Presetpro-Cinematic.3dl` + `luts/{NeutralLUT,B&WLUT,NightLUT}.png` +
  `textures/noises/perlin/128x128.png` (`_3dlut`),
  `textures/transition/transition1..6.png` + `examples/jsm/libs/tween.module.js`
  (`_transition`), `examples/jsm/environments/RoomEnvironment.js` (`_ca`).

---

## Cleanup

`test/e2e/_dump_rung9.mjs` and `.puppeteer_profile_rung9/` deleted; the vendor tree is
back to `M test/e2e/puppeteer.js` only.
