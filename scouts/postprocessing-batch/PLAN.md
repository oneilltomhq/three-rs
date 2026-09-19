# Scout plan — a **batch** rung of post-processing examples

Scouted 2026-09-19 against vendor three.js r186 (`rung0/grader-flags.patch`), on this machine
(Intel Iris Xe, Mesa 25.3.6, Fedora 43). Port read at
`/home/tom/src/projects/three-rs/rung-pmrem-a` (stack tip `f949c0e`: rungs 0–13, radial blur,
ssaa, materials, fat lines A, PMREM A). The in-flight `rung-bloom-selective` worker (MRT +
`BloomNode` + `pass` MRT outputs) is **assumed landed** throughout; at the time of writing that
branch has `src/nodes/mrt.rs` and edits to `render_target.rs` / `node_material.rs` in the working
tree and no `BloomNode` yet.

Starting point: `scouts/postprocessing-family/SURVEY.md`. This plan does not re-grade what the
survey graded twice; it re-graded the single-run rows it picks from.

Dumps beside this file: `dump-difference/`, `dump-direct/`, `dump-bloom/`, `dump-anamorphic/`,
`dump-ca/`, `dump-motion_blur/` (each: `dump.json`, every `mNN_*.wgsl`, `actual_full.png`,
`actual.jpg`). All six were captured with
`tools-dump/tools/dump-webgpu.mjs <example> --out …` under the GPU lock. Every dump printed one
`404 (Not Found)` page error (the same resource in all six, present in dumps of examples that
already grade 0.0%); no dump failed.

---

## 0. The batch, and why these four

Constraint from the brief: small, self-contained effect nodes; nothing beyond what the port has
**plus bloom**; no video, no controls-dependent state, no network.

In the survey's "gradeable" column, that pool is much smaller than the candidate list suggests.
Everything except four members needs a loader or an environment the port does not have:

| considered | blocker | verdict |
|---|---|---|
| `dof_basic` | DRACO + glTF + `UltraHDRLoader`, pass depth | not this batch |
| `3dlut` | glTF + 3 LUT loaders + `Texture3D`/`texture3D` | not this batch |
| `godrays` | glTF, pass depth, 3 effect nodes (618+377+80 JS lines) | own rung |
| `fog` | `PLYLoader` + Lucy100k, `SunLight` addon, pass depth, `gaussianBlur` | own rung |
| `lensflare` | `UltraHDRLoader` + glTF + three effect nodes | own rung |
| `outline` | `OBJLoader` (955), `OutlineNode` (814), `SunLight` addon, `MeshLambertMaterial` | own rung |
| `motion_blur` | dumped below (§6.2): MRT is the easy half; `SunLight` **shadow map**, skinned glTF | own rung |
| `ca` | dumped below (§6.1): `PMREMGenerator.fromScene( RoomEnvironment )` — the *second* PMREM sitting, plus `Points`, `GridHelper`, `MeshLambertMaterial` | own rung, after PMREM B |
| `bloom_emissive` | needs PMREM-from-equirect for `scene.environment` | after PMREM B |

What is left, and what this plan covers, in the order to do them:

| # | example | grade (runs) | new Rust, honest | depends on the bloom worker? |
|---|---|---|---|---|
| 1 | `webgpu_postprocessing_difference` | 0.0% (survey A, C) | ~420 (180 of it a GIF decoder) | no |
| 2 | `webgpu_postprocessing_bloom` | **0.0% ×2 today** | ~300 | **yes** |
| 3 | `webgpu_postprocessing_direct` | 0.0% (survey run7, C) | ~330 | no |
| 4 | `webgpu_postprocessing_anamorphic` | **0.0% ×2 today** | ~430 | **yes** |

Today's two grader runs, both under the lock:

```
Diff 0.0% in file: webgpu_postprocessing_anamorphic (3.4s)   Diff 0.0% in file: webgpu_postprocessing_ca (3.8s)
Diff 0.0% in file: webgpu_postprocessing_anamorphic (3.2s)   Diff 0.0% in file: webgpu_postprocessing_ca (5.8s)
Diff 0.0% in file: webgpu_postprocessing_bloom (3.5s)        Diff 0.0% in file: webgpu_postprocessing_motion_blur (3.4s)
Diff 0.0% in file: webgpu_postprocessing_bloom (3.5s)        Diff 0.0% in file: webgpu_postprocessing_motion_blur (3.4s)
```

None of the six is on Three's `exceptionList`.

Frame-0 facts from the survey's §0 apply to all four: `performance.now`/`Date.now` are 0, one RAF,
`OrbitControls` never moves, `Math.random` is the seeded sequence `Renderer::skip_random_draws`
models. None of the four branches on `window.TESTING` (`grep -n TESTING` finds nothing in any of
the four HTML files).

---

## 1. `webgpu_postprocessing_difference` — the cheapest, and the only one nothing else blocks

**Grade** 0.0% twice (survey runs A and C). Not on the exception list.

### What the example does

`examples/webgpu_postprocessing_difference.html`, 141 lines. One `BoxGeometry` with
`MeshBasicMaterial({ map: crate.gif })`, `scene.background = Color(0x0487e2)`,
`scene.fog = new Fog( 0x0487e2, 7, 25 )`, `renderer.toneMapping = NeutralToneMapping`,
no lights, no `antialias`. `Math.random` is never called. `OrbitControls` with damping, which at
frame 0 is a no-op. `mesh.rotation.y += timer.getDelta() * 5 * speed` is `+= 0` at frame 0.

The effect is not an addon at all — it is four `three/tsl` calls on the pass:

```js
const scenePass = pass( scene, camera );
const currentTexture  = scenePass.getTextureNode();
const previousTexture = scenePass.getPreviousTextureNode();
const frameDiff = previousTexture.sub( currentTexture ).abs();
const saturationAmount = luminance( frameDiff ).mul( 1000 ).clamp( 0, 3 );
renderPipeline.outputNode = saturation( currentTexture, saturationAmount );
```

### Dump reading (`dump-difference/`)

5 modules, 3 render pipelines, 10 passes, 3 submits, 2 draws, 4 samplers, 9 buffers.

* `m00_vertex_fragment_mipmap.wgsl` — the mipmap blit, 8 passes over texture 16
  (256×256 `rgba8unorm-srgb`, 9 levels: `crate.gif` is a real GIF, `version 89a, 256 x 256`).
* `m01/m02_*_fragment.wgsl` — the box: `MeshBasicMaterial` with a `mat3x3` uv transform in the
  object block and the linear-fog mix
  `mix( Output.xyz, fogColor, smoothstep( near, far, -v_positionView.z ) )`.
* `m03/m04_*_RenderPipeline.wgsl` — the quad. Two texture bindings, then one line:
  `mix( vec3( dot( cur.xyz, LUM ) ), cur.xyz, clamp( dot( abs( prev - cur ), vec4( LUM, 1.0 ) ) * 1000, 0, 3 ) )`,
  then `unpremultiply → neutralToneMapping → sRGBTransferOETF → premultiply`, all inline (no
  second output pass, as rung 9 established).
* Textures: `4 output rgba16float` (the scene pass target) and `59 output rgba16float`, which
  **no pass ever writes**.

Two details worth pinning, both visible only in `dump.json`:

1. **`dot( vec4, vec3 )` pads with 1.0.** `luminance()` is `dot( color, vec3( 0.2126, … ) )` and
   here `color` is a **vec4**, so TSL widens the coefficient vector and the dumped code is
   `dot( abs( prev - cur ), vec4( vec3( .2126, .7152, .0722 ), 1.0 ) )` — the alpha difference is
   weighted **1.0**. The port's `luminance` (`src/nodes/tsl.rs:2617`) is `dot(color, vec3(...))`
   with no such conversion; a vec3-only reading changes the answer wherever alpha differs.
2. **The "previous" texture is zero at frame 0.** `PassNode.updateBefore()` calls
   `toggleTexture()` *before* rendering (`PassNode.js:861`), so the first frame renders into the
   clone and the node named "previous" points at the never-rendered original. `dump.json` has two
   quad bind groups: 63 (previous = texture 4) built and dropped, and 64 — the one pass 0 actually
   binds — with `nodeUniform0 = texture 4` (the rendered target) and `nodeUniform1 = texture 59`
   (untouched, i.e. WebGPU's zero-initialised contents). So the graded frame's `frameDiff` is
   `|0 − current| = current`, and the whole image is a strongly-saturated version of the box.
   A port that renders the previous buffer, or swaps in the other order, changes the image.

### Gap list

| thing | status |
|---|---|
| `pass()` / `RenderPipeline` / quad | **have** — `src/renderer/pass.rs`, `render_pipeline.rs` |
| `NeutralToneMapping` in the quad | **have** — `ToneMapping::Neutral`, `render_pipeline.rs:64` |
| `saturation`, `mix`, `clamp`, `abs` | **have** — `src/nodes/tsl.rs` |
| `luminance` of a **vec4** | **missing (small)** — either a vec4 overload or, better, three's own conversion rule in `dot`: `src/nodes/tsl.rs`, ~15 lines. Node-system work, touches a shared path — cover it with a WGSL fixture test |
| `getPreviousTextureNode()` + `toggleTexture()` | **missing** — `src/nodes/display/PassNode.js:630–716, 861` → `src/renderer/pass.rs`: a second `RenderTarget`-backed texture per output name, a texture node bound to it, swap at the top of `PassNode::render`. ~70 lines Rust, renderer work, self-contained |
| `scene.fog` → `fogNode` (linear) | **have** — `fog()` / `range_fog_factor()` (`src/nodes/tsl.rs:592–616`), `Scene::fog_node`, `Material::fog` |
| `MeshBasicMaterial` + `map` + uv transform | **have** (rung 1/6) |
| mipmap generation for a loaded texture | **have** — `src/renderer/mipmap.rs` (the dump's 8 blits) |
| sRGB texture + `rgba8unorm-srgb` | **have** — `Texture::color_space` |
| **GIF decode** | **missing** — `src/loaders/texture_loader.rs` only does JPEG (`load`) and PNG/JPEG (`from_bytes`). `crate.gif` is GIF89a, 256×256, palette + LZW. Either add the `gif` crate to `Cargo.toml` (~30 lines of glue) or hand-roll the decoder (~180 lines). Loader work, isolated |

**Size: ~420 lines Rust** (≈240 without the decoder), plus a `webgpu_postprocessing_difference.rs`
example. Nothing here is blocked by the bloom worker.

---

## 2. `webgpu_postprocessing_bloom` — the plain bloom example, cheap the moment `BloomNode` lands

**Grade** 0.0% twice today. Not on the exception list.

### What the example does

194 lines. `PrimaryIonDrive.glb` + `AnimationMixer` playing `gltf.animations[0].optimize()` (the
clip is named `Main`, 4 channels), `AmbientLight(0xcccccc)`, a `PointLight(0xffffff, 100)` **added
to the camera** (so it moves with it and its position comes from the camera's world matrix),
`renderer.toneMapping = ReinhardToneMapping`, `antialias: true`. No `Math.random`, no `TESTING`.

Post-processing is three lines:

```js
const scenePass = pass( scene, camera, config );
const scenePassColor = scenePass.getTextureNode( 'output' );
renderPipeline.outputNode = scenePassColor.add( bloom( scenePassColor ) );
```

`config` is the MSAA store/resolve hint (`storeMultisampledColorBuffer: false`,
`resolveColorBuffer: true`, …). It is an optimisation, not a visual: it decides `storeOp` on the
multisampled attachment.

The glb (inspected with `node`, no extensions, **no images**): 6 meshes, 19 nodes, 37 accessors,
3 materials — `constant1` (doubleSided, emissive 0.0189³, metallic 0, rough 0.6), `constant2`
(doubleSided, emissive (0.322, 0.291, 0), base colour orange), `HoloFillDark`
(**`alphaMode: BLEND`**, base alpha 0.809, metallic 0.92).

### Dump reading (`dump-bloom/`)

14 modules, 11 render pipelines, 14 passes, 14 submits, 19 draws, 1 sampler.

* Pass 0 — the canvas quad (recorded first, as rung 9 documented). Pass 1 — the scene: 6 draws
  over 3 pipelines named after the glb materials, into `5 output-msaa rgba16float sampleCount 4`
  resolving to `4 output`, depth `6 depth 4×`.
* Passes 2–13 — the bloom chain at the default `_resolutionScale = 0.5`:
  `UnrealBloomPass.bright` 400×250, then `h0/v0` 400×250, `h1/v1` 200×125, `h2/v2` 100×62,
  `h3/v3` 50×31, `h4/v4` 25×15, and a final `Bloom_comp` back into `h0`. Twelve quad draws, one
  submit each.
* The material shaders are the physical ones the port already emits: the single sampler is the
  16×16 `rg16float` `DFG_LUT` (`textureSample( …, vec2( Roughness, clamp( dotNV … ) ) )`) — i.e.
  `src/materials/dfg_lut.rs` unchanged.

### Gap list

| thing | status |
|---|---|
| `BloomNode` (11 RTs, 7 quad materials, 12 passes) | **from the bloom worker** — if `rung-bloom-selective` lands, this example needs no bloom code at all beyond `setResolutionScale` staying at its 0.5 default |
| MSAA pass target + resolve | **have** — `RenderTarget::samples`, `Renderer::samples`, `src/renderer/mod.rs:3776` |
| pass `config` store/resolve flags | **missing (cosmetic)** — only changes `storeOp`/`resolveTarget` on the multisampled attachment; ~25 lines in `src/renderer/pass.rs` + `render_target.rs`, and the dump is the oracle |
| `ReinhardToneMapping` | **have** — `ToneMapping::Reinhard` |
| glTF geometry/nodes/animation | **have** — `src/loaders/gltf_loader.rs` (rung 10), `AnimationMixer`, `AnimationClip::optimize` (`src/animation/animation_clip.rs:368`) |
| glTF **materials wired for non-skinned meshes** | **missing** — rung 10 deliberately attached materials only to skinned meshes (`docs/rung10-progress.md`). `GltfMaterial` already parses `emissiveFactor`, `alphaMode`, `doubleSided` (`gltf_loader.rs:955–1019`); what is missing is building a `MeshStandardNodeMaterial` from it for every primitive and walking the node tree into `Mesh`es. ~120 lines, loader work |
| `alphaMode: BLEND` → `transparent` + normal blending + transparent sort | **partly** — `Material::transparent`/`opacity` exist and `src/materials/blending.rs` is there; the render list's transparent pass ordering is the risk. Check `src/renderer/render_list.rs` |
| `PointLight` as a child of the camera | **have** if the light's world matrix is taken from the tree (`src/lights/light_object.rs`); worth asserting — a light parented to the camera is the one lighting fact this example has |
| `AmbientLight` | **have** |

**Size: ~300 lines Rust.** Entirely gated on the bloom worker; do it immediately after that branch
merges, as the cheapest proof that `BloomNode` is right in a *second* scene.

---

## 3. `webgpu_postprocessing_direct` — one pass, no render target, a renderer feature

**Grade** 0.0% (survey run7 and run C). Not on the exception list.

### What the example does

128 lines, **no addon at all**. 100 `Mesh`es sharing a `SphereGeometry( 1, 4, 4 )`, each with its
own `MeshPhongMaterial({ color: Math.random() * 0xffffff, flatShading: true })`,
`AmbientLight(0xcccccc)` + `DirectionalLight(0xffffff, 3)` at (1,1,1),
`scene.background = Color(0x000000)`, `renderer.toneMapping = NeutralToneMapping`, no
`antialias`, no controls.

`Math.random` draws, in order, **9 per mesh × 100 = 900**: colour, `position.set` ×3,
`position.multiplyScalar` ×1, `rotation.set` ×3, `scale.setScalar` ×1. `object.rotation` is
advanced in `animate()` *before* the render, so the graded frame has
`object.rotation = (0.005, 0.01, 0)`, not zero.

The post-processing:

```js
const saturationFactor = uniform( 0 );
renderPipeline = new THREE.DirectRenderPipeline( renderer );
renderPipeline.outputNode = vec4( saturation( output.rgb, saturationFactor ), output.a );
renderPipeline.render( scene, camera );
```

`saturationFactor` is **0**, so the graded image is fully desaturated — a greyscale field of
flat-shaded spheres. That is a gift: it means the numeric path through `saturation` is exercised
but the colours are luminances, and a wrong `output` hook is immediately visible.

### Dump reading (`dump-direct/`)

4 modules, 2 render pipelines, **1 pass, 1 submit, 93 draws** (101 objects, the rest frustum
culled), 1 bind-group layout, 99 buffers, **0 samplers**. The only texture is the 800×500
`depthBuffer`: there is no intermediate colour target anywhere. That is the whole point of
`DirectRenderPipeline` — no `pass()`, no quad, no output blit.

The output transform is inlined at the end of **every material's fragment shader**
(`m03_fragment_fragment.wgsl`, and identically in `m01_…_Background.material.wgsl`):

```wgsl
Output = nodeVar25;
Output = nodeVar25;                       // <- output.assign( materialOutputNode )
nodeVar26 = vec4( max( mix( vec3( dot( Output.xyz, LUM ) ), Output.xyz, object.nodeUniform13 ), vec3( 0 ) ), Output.w );
nodeVar27 = fn1( vec4( nodeVar26.xyz, clamp( nodeVar26.w, 0, 1 ) ) );   // unpremultiplyAlpha
nodeVar28 = vec4( neutralToneMapping( nodeVar27.xyz, render.nodeUniform14 ), nodeVar27.w );
output.color = fn0( vec4( sRGBTransferOETF( nodeVar28.xyz ), nodeVar28.w ) );   // premultiplyAlpha
```

The doubled `Output = nodeVar25;` is `DirectRenderPipeline._updateContext`'s
`output.assign( materialOutputNode )` landing on top of `NodeMaterial.setupOutput`'s own assign —
reproduce it, it is free and it is what the WGSL diff will compare.

The solid background is **not** the usual background path: `_getBackgroundNode()` replaces
`scene.background` with `uniform( color )` for the duration of the render so the background quad's
fragment gets the same inline transform (hence a `Background.material` pipeline with a full
tone-mapping tail, and `DiffuseColor = vec4( object.nodeUniform0, 1 ) * vec4( render.nodeUniform1 )`
— colour × `backgroundIntensity`).

### Gap list

| thing | status |
|---|---|
| `output` node (the material's own output, assignable) | **missing** — `three.js/src/nodes/core/OutputStructNode`-adjacent `output` in `TSL.js`; in the port the `Output` var already exists inside `src/materials/node_material.rs` (`Output = …` in every dump). Exposing it as a TSL node is ~30 lines |
| `renderer.contextNode` + a `getOutput( materialOutputNode, builder )` hook | **missing** — `three.js/src/renderers/common/DirectRenderPipeline.js` (233 lines) + `RenderPipeline._updateContext`. In the port: a `Option<Rc<dyn Fn(NodeRef) -> NodeRef>>` on the renderer consulted by `node_material::setup_output`. ~60 lines, **but it edits the one function every material goes through** — the merge-conflict and regression risk of this batch |
| `DirectRenderPipeline` itself | **missing** — new `src/renderer/direct_render_pipeline.rs`, ~120 lines: save/neutralise `tone_mapping` + output colour space, swap in the background uniform node, render, restore. Structurally the same as the existing `RenderPipeline::render` |
| background colour as a `uniform()` node | **have** — `materials::background_node_color_node` (`src/renderer/mod.rs:1006`) already takes a node background; needs the `Color → uniform node` substitution (~20 lines) |
| `saturation`, `uniform`, `vec4`, `NeutralToneMapping` | **have** |
| `MeshPhongMaterial` + `flatShading` + per-mesh materials | **have** (rung 4/6) |
| `AmbientLight` + `DirectionalLight` | **have** |
| `SphereGeometry( 1, 4, 4 )` | **have** |
| 900 seeded `Math.random` draws | **have** — `Renderer::skip_random_draws`; count and order are above |

**Size: ~330 lines Rust.** Independent of the bloom worker. The risk is concentrated in one shared
function, so land it with a WGSL fixture for one untouched material (e.g. rung 6's) proving the
hook is inert when no `DirectRenderPipeline` is in play.

---

## 4. `webgpu_postprocessing_anamorphic` — bloom with a replaced high-pass, and `rtt()`

**Grade** 0.0% twice today. Not on the exception list.

### What the example does

202 lines, no assets. One `InstancedMesh` of 200 `SphereGeometry( 0.1, 32, 32 )` with
`MeshBasicNodeMaterial`, `setMatrixAt` + `setColorAt` per instance
(**`Math.random` draws: 4 per instance × 200 = 800**, in the order x, y, z, colour), a
`positionNode` that bobs each instance with `time` (0 at frame 0, so the bob is
`sin( instanceIndex * 0.5 * 0.5 )` — *not* zero), `scene.backgroundNode` = a TSL `Fn` mixing
`0x111111` → black by `screenUV.distance( 0.5 ).mul( 2 )`, `NeutralToneMapping`, `antialias: true`.

The effect is `BloomNode` with two things swapped:

```js
const bloomPass = bloom( scenePass.getTextureNode(), intensity, radius, threshold );
bloomPass.setResolutionScale( 0.25 );
bloomPass.highPassFn = Fn( ( { input, threshold, smoothWidth } ) => { … } );
renderPipeline.outputNode = scenePass.add( bloomPass.mul( tintColor ) );
```

The replacement high-pass is the interesting part: it builds a bright-pass **`rtt()`** node
(`mix( vec4(0), input, smoothstep( threshold, threshold + smoothWidth, luminance( input.rgb ) ) )`
rendered to its own target with `MirroredRepeatWrapping`), then a horizontal `Loop` from
`-samples/2` to `+samples/2` (samples = 80) sampling that target at `uv.x + (1/viewportSize.x) * i * 4`
with a `pow( 1 - |i| / halfSamples, 2 )` weight, divided by `samples/3`.

### Dump reading (`dump-anamorphic/`)

18 modules, 11 render pipelines, 15 passes, 15 submits, 16 draws, 24 buffers, **2 samplers**, 17
textures.

* Pass 1 — the scene: background (`drawIndexed 5952`, one instance) + the instanced mesh
  (`drawIndexed 5952, instanceCount 200`) into `5 output-msaa` 4× resolving to `4 output`.
* Pass 3 (`renderPipeline_RTT_28`, `m04/m05_*_RTT.wgsl`) — the bright-pass `rtt()`, a **full-size
  800×500 `rgba16float`** target (texture 37) with its own `depth24plus` (38). One triangle.
* Pass 2 (`renderPipeline_Bloom_highPass_20`, `m06/m07`) — the custom high pass into
  `35 UnrealBloomPass.bright` **200×125** (= `floor( 800 × 0.25 )`, `floor( 500 × 0.25 )`), with
  an explicit `setViewport( 0, 0, 200, 125 )`. Its WGSL is the node-bounded loop with a negated
  start:
  `for ( var i : i32 = i32( ( - nodeVar1 ) ); i < i32( nodeVar1 ); i ++ )`, `nodeVar1 = object.nodeUniform0 / 2.0`.
* Passes 4–13 — the standard five-level separable blur, 200×125 → 100×62 → 50×31 → 25×15 → 12×7,
  and pass 14 the composite back into `h0`.
* Submission order is worth copying: scene, **then the RTT**, then the high pass, even though the
  high-pass pass descriptor is *begun* first (`beginRenderPass` n=57 vs n=61, submits at n=79 then
  n=100). Same recording-vs-execution split rung 9 documented.

**The two samplers are both `nearest` / `clamp-to-edge` on all three address modes.** The
`MirroredRepeatWrapping` the page hands `rtt()` does **not** reach the GPU sampler, and neither
does linear filtering. The high-pass reads `uv.x ± 4 · i / width` with `i` up to ±40, i.e. up to
±0.2 outside [0,1], and those reads are **clamped**, not mirrored. A port that "correctly"
implements mirrored wrap here renders a different image.

### Gap list

| thing | status |
|---|---|
| `BloomNode` + its 11 RTs | **from the bloom worker** |
| `setResolutionScale( 0.25 )` + `setViewport` on the bright target | mostly the worker's; the viewport itself is **have** — `RenderTarget::set_viewport` (fat-lines A) |
| **`bloomPass.highPassFn` is replaceable** | design constraint on the worker's `BloomNode`: the high-pass material's fragment node must be a settable closure, not hard-coded. **Tell the bloom worker now** — retrofitting is worse than building it in |
| **`rtt()` / `convertToTexture`** | **missing** — `three.js/src/nodes/utils/RTTNode.js` (~200 JS lines). A node that owns a `RenderTarget`, renders its own quad in `updateBefore`, and is sampled like a texture. Every primitive exists (`Renderer::render_quad`, `RenderTarget`, the ssaa pattern). ~150 lines Rust in `src/nodes/display/rtt.rs`. **Shared**: `ca`, `dof_basic` and every `convertToTexture( <non-texture> )` need the same node |
| `Loop` with node `start` **and** `end`, start negated | **partly** — radial blur landed a node-bounded `end`; a node `start` is ~20 lines |
| `viewportSize`, `screenUV`, `uv()`, `smoothstep`, `pow`, `luminance` (vec3 here) | **have** |
| `scene.backgroundNode` = a TSL `Fn` | **have** — `background_node_color_node` |
| `InstancedMesh` + `setColorAt` (`vInstanceColor`) | **have** — landed with radial blur (`src/materials/node_material.rs:147`) |
| `positionNode` with `time` + `instanceIndex` | **have** — `instance_index()` (`src/nodes/tsl.rs:1360`), `time` |
| MSAA pass target | **have** |
| sampler state for RT textures = nearest/clamp | **check** — the dump says nearest/clamp; if the port's render-target textures default to linear, this example diverges |

**Size: ~430 lines Rust** (≈280 of it `rtt()`, which the next three examples after this batch also
want). Gated on the bloom worker.

---

## 5. Shared gaps (two or more of the four need these)

| shared piece | needed by | size |
|---|---|---|
| `BloomNode` with a **replaceable `highPassFn`** and a working `setResolutionScale` | `bloom`, `anamorphic` | the worker's; the replaceability is a request to make now |
| The nearest/clamp sampler reading for render-target textures | `anamorphic`, `bloom`, `difference` (all three sample RTs) | 0 if already true; the dumps are the oracle |
| Pass MSAA store/resolve `config` | `bloom`, `anamorphic` (both `antialias: true`) | ~25 lines, one implementation |
| `dot( vec4, vec3 )` widening with `w = 1.0` | `difference` now; every `luminance( vec4 )` later | ~15 lines, node-system, shared path |
| `rtt()` / `convertToTexture` | `anamorphic` now; `ca`, `dof_basic`, `fog` next | ~150 lines |
| A material-output hook on the renderer (`contextNode.getOutput`) | `direct` only in this batch, but it is the mechanism `DirectRenderPipeline` and every "apply the output transform in-material" variant uses | ~60 lines on a shared path |
| Nothing in this batch needs: a Gaussian blur node, pass **depth** outputs, `Texture3D`/`texture3D`, velocity, `SunLight`, PMREM | — | — |

Two pieces two-or-more *later* examples want and this batch deliberately leaves alone: pass
depth/`getViewZNode` (fog, godrays, dof_basic) and the `SunLight` addon (fog, motion_blur,
outline, pixel).

---

## 6. The two that were dumped and dropped (evidence, so nobody re-scouts them)

### 6.1 `ca` — `dump-ca/`

0.0% twice today, but: 20 modules, **14 pipelines, 33 passes, 33 submits, 66 draws**. Passes 1–7
render `RoomEnvironment` into `0 PMREM.cubeUv` 768×1024 `rgba16float` (a
`MeshLambertMaterial` pipeline and two `MeshStandardMaterial` ones, 8–10 instances per face),
passes 8–29 are `PMREM_blur` + eleven `PMREM_ggx` ping-pongs between two cubeUv atlases. That is
exactly the second PMREM sitting (`docs/webgpu_pmrem_cubemap-progress.md` steps 4–9) plus
`RoomEnvironment` (a hand-built scene) plus `MeshLambertMaterial`. The scene itself then adds
`PointsMaterial` (200 particles) and `LineBasicMaterial` (`GridHelper`), 8 geometry types, and one
`RTT` pass because `chromaticAberration( renderOutput( scenePass ) )` runs its input through
`convertToTexture`. `ChromaticAberrationNode.js` is only 174 lines and would be ~60 lines of Rust;
everything expensive about this example is the environment. **Own rung, after PMREM B.**

### 6.2 `motion_blur` — `dump-motion_blur/`

0.0% twice today. 15 modules, 8 pipelines, 24 passes (10 of them mipmap blits for two floor
textures), 7 submits. Pass 1 is the MRT pass — **two colour attachments, `4 output` and
`5 velocity`, both `rgba16float`** — which the bloom worker's MRT makes cheap. The expensive half
is the `SunLight` addon: `10 SunShadowMap` 2048×1024 `rgba8unorm` + `11 SunShadowDepthTexture`,
two `ShadowMaterial` passes, plus the skinned `Xbot.glb`. `MotionBlur.js` is 33 lines; the
`velocity` node (previous model-view matrices per object) is the real node work. **Own rung.**

---

## 7. Gates beyond the pixel diff

* **WGSL diff, per example.** `examples/dump_wgsl.rs` against the named modules:
  `dump-difference/m04_fragment_fragment_RenderPipeline.wgsl` (the whole effect is those four
  lines), `dump-direct/m03_fragment_fragment.wgsl` **and**
  `dump-direct/m01_fragment_fragment_Background.material.wgsl` (the inline output tail, including
  the doubled `Output =` assign), `dump-anamorphic/m07_fragment_fragment_Bloom_highPass.wgsl` (the
  negated node loop bound) and `m05_fragment_fragment_RTT.wgsl`, `dump-bloom/m12_…_Bloom_comp.wgsl`.
* **A `dot( vec4, vec3 )` unit test** — the one-line assertion that the emitted code is
  `vec4( vec3( … ), 1.0 )`, not `vec3( … )`. Cheap, and it is the silent-wrongness of §1.
* **A previous-texture test without the GPU**: assert that after one `PassNode::render` the
  "current" node names the target that was rendered and the "previous" node names the other one,
  and that the other one was never bound as a colour attachment. `dump-difference/dump.json`'s
  bind groups 63/64 are the oracle (see §1).
* **A pass/submit-count test** per example, read straight from the dumps: difference 2 render
  passes + 8 mipmap blits / 3 submits / 2 draws; direct **1 pass, 1 submit, 93 draws, 0 samplers**;
  bloom 14 passes / 14 submits / 19 draws; anamorphic 15 passes / 15 submits / 16 draws. `direct`'s
  is the strongest single assertion in this batch — if any intermediate colour target appears, the
  direct pipeline is wrong regardless of the pixels.
* **No numeric oracle was dumped** beyond the above; none of these four has a
  bones-matrix-shaped quantity. The GIF decode should be gated the way rung 10's loaders were: a
  small `tests/` fixture comparing the decoded 256×256 RGBA against a `node`-side decode of
  `crate.gif`, generated without modifying the vendor tree.

---

## 8. Order of work (each step leaves the ladder green)

1. **`difference`**, and inside it, in order: `dot( vec4, vec3 )` widening + its fixture; the GIF
   decode + its fixture; `getPreviousTextureNode`/`toggleTexture` in `pass.rs`; the example; the
   grade. Nothing here depends on the bloom worker, so start here even if that branch is late.
2. **`bloom`** — requires the bloom branch merged. Wire glTF materials for non-skinned meshes,
   check the transparent sort, add the pass `config` flags, then the example.
3. **`direct`** — `output` node, the `getOutput` hook (+ an inertness fixture on an existing
   rung's material), `DirectRenderPipeline`, the background-uniform substitution, then the example.
4. **`anamorphic`** — `rtt()` (the big piece), the node `start` on `Loop`, the `highPassFn`
   replacement, then the example.

Steps 1 and 3 are independent of steps 2 and 4; if the bloom worker slips, the sitting is still
two examples and ~750 lines.

---

## 9. What this batch unlocks

* `rtt()` (step 4) is the last blocker `ca` has apart from PMREM B, and `dof_basic`, `fog` and
  every future `convertToTexture( … )` need it.
* glTF materials on non-skinned meshes (step 2) is the piece `bloom_emissive`, `godrays`,
  `motion_blur`, `3dlut`, `lensflare` and `dof_basic` all need, i.e. six more members.
* `getPreviousTextureNode` (step 1) is what `afterimage` and `traa` need — both currently ungradeable
  here, so it buys nothing in the family today, but `difference` is the only example that proves it.
* A second and third scene through `BloomNode` (steps 2 and 4) is the cheapest insurance the bloom
  rung has; `bloom_selective` alone would leave the resolution scale, the high-pass hook and the
  non-MRT path untested.

---

## 10. Verdict

**Rung-sized as a batch of three for one Opus worker in one sitting; four is the stretch.**

Honest arithmetic: `difference` ~420, `bloom` ~300, `direct` ~330, `anamorphic` ~430 → ~1480 lines
of Rust plus four examples. That is more than one sitting at this ladder's demonstrated pace
(rung 9 was ~600, ssaa ~550). The realistic call:

* **Do 1 + 2 + 3** (`difference`, `bloom`, `direct`) — ~1050 lines, three examples green, and
  only one of them waits on the bloom worker.
* **`anamorphic` is the fourth, and it is really an `rtt()` rung** wearing a bloom costume. If the
  sitting runs long, drop it and it becomes the cheap next rung — with `ca` right behind it once
  PMREM B lands.

Top three risks, in order:

1. **The `getOutput` hook in `node_material`** (`direct`) touches the function every material in
   the port goes through. Fixture an untouched rung's WGSL before and after.
2. **The bloom worker's `BloomNode` shape.** Both `bloom` and `anamorphic` need `highPassFn`
   replaceable and `setResolutionScale` real; if it lands hard-coded to `bloom_selective`'s use,
   two of these four examples grow a refactor.
3. **glTF materials for non-skinned meshes** (`bloom`) is the only piece here whose size is
   genuinely uncertain — transparency sorting and `doubleSided` interact with the render list, a
   path every rung uses.
