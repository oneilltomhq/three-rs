# Rung scout — `webgpu_postprocessing_bloom_selective`

Scouted 2026-09-19. Example:
`~/src/vendor/three.js/examples/webgpu_postprocessing_bloom_selective.html` (vendor r186,
`rung0/grader-flags.patch` applied). Family context: `scouts/postprocessing-family/SURVEY.md`.

**Take `webgpu_postprocessing_ssaa` first.** This example needs the "effect node owns render
targets and drives quad passes" pattern that `ssaa` establishes; without it, this is three rungs,
not two.

Files next to this plan:

| file | what |
|---|---|
| `dump/dump.json` | all 514 chronological calls, 14 render passes with command streams, 9 pipelines, 73 bind groups |
| `dump/m00_vertex_vertex.wgsl`, `dump/m01_fragment_fragment.wgsl` | the scene program — **note the two-location `OutputType`: this is MRT** |
| `dump/m02..m03_*_Bloom_highPass.wgsl` | the luminosity high-pass quad |
| `dump/m04..m09_*_Bloom_separable.wgsl` | one vertex + **five** fragment modules, one per mip level |
| `dump/m10..m11_*_Bloom_comp.wgsl` | the composite quad |
| `dump/m12_fragment_fragment_RenderPipeline.wgsl` | the final quad (`outputPass.add(bloomPass).renderOutput()`) |
| `dump/actual_full.png`, `dump/actual.jpg` | Three's deterministic frame, raw and grader-scaled |
| `reference-r186.jpg` | `examples/screenshots/webgpu_postprocessing_bloom_selective.jpg` |

**Result: 13 shader modules, 9 render pipelines, 5 bind-group layouts, 0 compute pipelines,
15 textures, 1 sampler, 62 buffers, 73 bind groups, 14 render passes, 63 draws.**

---

## 1. Grade confirmation — 0.0%, twice

```
Diff 0.0% in file: webgpu_postprocessing_bloom_selective (3.0s)   # run A, 2026-09-19
Diff 0.0% in file: webgpu_postprocessing_bloom_selective (3.1s)   # run C, 2026-09-19
```

Logs: `scouts/postprocessing-family/e2e-runs-2026-09-19.log`. **Not** on the `exceptionList`.

---

## 2. What the page does

The reason this is the right entry point for MRT and bloom: **the scene is almost nothing.**

`WebGPURenderer()` — no `antialias`. `setPixelRatio(1)`, `setSize(800,500)`.
`renderer.toneMapping = THREE.NeutralToneMapping`. `renderer.inspector` + GUI only. The
`pointerdown` raycaster and `window.onresize` never fire. `animate()` is one
`renderPipeline.render()` — no time, no motion, nothing to freeze.

### 2.1 Scene

* `PerspectiveCamera( 40, 800/500, 1, 200 )` at `(0, 0, 20)`, `lookAt( 0, 0, 0 )`.
* **No `scene.background`** → the pass targets clear to the renderer clear colour, `(0,0,0,0)`
  (the `alpha: true` default, `docs/postprocessing.md`).
* **No lights at all.**
* 50 × `Mesh( IcosahedronGeometry( 1, 15 ), MeshBasicNodeMaterial({ color }) )`, each with its own
  material carrying `material.mrtNode = mrt({ bloomIntensity: uniform( bloomIntensity ) })`.
  The dump shows 50 separate `drawIndexed` calls in one pass — 50 materials, but **one shared
  pipeline** (`renderPipeline_MeshBasicNodeMaterial_44`), 50 object bind groups.

`IcosahedronGeometry( 1, 15 )` is 20 × 16² = 5120 triangles; 50 of them is ~256k triangles.

### 2.2 `Math.random` draws before the frame — **450**

Nine per sphere, `i = 0 .. 49`, in this exact order (JS evaluates `setHSL`'s arguments
left-to-right, so the two inside it are draws 1 and 2):

```js
color.setHSL( Math.random(), 0.7, Math.random() * 0.2 + 0.05 );        // 1, 2
const bloomIntensity = Math.random() > 0.5 ? 1 : 0;                     // 3
sphere.position.x = Math.random() * 10 - 5;                             // 4
sphere.position.y = Math.random() * 10 - 5;                             // 5
sphere.position.z = Math.random() * 10 - 5;                             // 6
sphere.position.normalize().multiplyScalar( Math.random() * 4.0 + 2.0 );// 7
sphere.scale.setScalar( Math.random() * Math.random() + 0.5 );          // 8, 9
```

Draw 3 decides, per sphere, whether it blooms. Getting the sequence off by one changes *which*
spheres glow — the most visible possible failure, and the reason for the oracle in §5.4.

### 2.3 The TSL graph

```js
const scenePass = pass( scene, camera );
scenePass.setMRT( mrt( { output, bloomIntensity: float( 0 ) } ) );   // pass-level default

const outputPass         = scenePass.getTextureNode();                // attachment 0
const bloomIntensityPass = scenePass.getTextureNode( 'bloomIntensity' ); // attachment 1
const bloomPass          = bloom( outputPass.mul( bloomIntensityPass ) );

renderPipeline.outputColorTransform = false;
renderPipeline.outputNode = outputPass.add( bloomPass ).renderOutput();
```

Two levels of MRT: the **pass** declares `{ output, bloomIntensity: float(0) }` as the default,
and each **material** overrides `bloomIntensity` with its own `uniform(0 or 1)`. The dumped scene
fragment writes `@location(0) m0` and `@location(1) m1`.

`outputColorTransform` is **false** and the transform is applied explicitly by `.renderOutput()`
on the output node, so the quad's fragment ends with the neutral-tone-mapping sandwich anyway
(`dump/m12_*.wgsl` contains `neutralToneMapping` verbatim — the same function `radial_blur` needs).

`examples/jsm/tsl/display/BloomNode.js` is **599 lines**, of which ~200 are jsdoc.

---

## 3. Dump reading

### 3.1 Textures — 15

| id | label | format | size |
|---|---|---|---|
| 0 | `depthBuffer` | `depth24plus` | 800×500 |
| 4 | `output` | `rgba16float` | 800×500 |
| **5** | `bloomIntensity` | `rgba16float` | 800×500 |
| 6 | `depth` | `depth24plus` | 800×500 |
| 118 | `UnrealBloomPass.bright` | `rgba16float` | 400×250 |
| 132/143 | `.h0` / `.v0` | `rgba16float` | 400×250 |
| 147/156 | `.h1` / `.v1` | `rgba16float` | 200×125 |
| 160/169 | `.h2` / `.v2` | `rgba16float` | 100×62 |
| 173/182 | `.h3` / `.v3` | `rgba16float` | 50×31 |
| 186/195 | `.h4` / `.v4` | `rgba16float` | 25×15 |

Eleven bloom targets, all `depthBuffer: false` (the pipelines have `depthStencil: null`). The
sizes are `Math.round( w/2 )` then halved four times with rounding — note 100→**62** and 50→**31**,
not 50 and 25. Reproduce the rounding, not the division.

### 3.2 The 14 passes

```
 0  renderContext_0  canvas + depthBuffer                       clear   draw(3,1,0,0)   RenderPipeline
 1  renderContext_1  output (4) + bloomIntensity (5) + depth    clear   50 × drawIndexed  MeshBasicNodeMaterial
 2  renderContext_2  UnrealBloomPass.bright                     clear   draw(3,1,0,0)   Bloom_highPass
 3  renderContext_2  UnrealBloomPass.h0                         clear   draw            Bloom_separable_70
 4  renderContext_2  UnrealBloomPass.v0                         clear   draw            Bloom_separable_70
 5  renderContext_2  UnrealBloomPass.h1                         clear   draw            Bloom_separable_71
 6  renderContext_2  UnrealBloomPass.v1                         clear   draw            Bloom_separable_71
 7  …  h2 / v2 (…_72), h3 / v3 (…_73), h4 / v4 (…_74)
13  renderContext_2  UnrealBloomPass.h0                         clear   draw            Bloom_comp
```

Pass 1 is the only multi-attachment pass and the only one with depth. Passes 2–13 have **no depth
attachment at all**. The composite writes back into `h0`, which is then what
`passTexture( this, _renderTargetsHorizontal[0].texture )` hands the final quad.

Each `Bloom_separable_7N` pipeline is used for both the horizontal and the vertical pass of its
mip — the `direction` uniform (`vec2(1,0)` / `vec2(0,1)`) and `colorTexture` are swapped between
them, which is why there are 73 bind groups for 9 pipelines.

### 3.3 The scene program — MRT

`dump/m01_fragment_fragment.wgsl`:

```wgsl
struct OutputType {
	@location( 0 ) m0 : vec4<f32>,
	@location( 1 ) m1 : vec4<f32>,
};
var<private> output : OutputType;
…
@fragment fn main() -> OutputType { … }
```

Compare with every other example in the port, which emits
`struct OutputStruct { @location(0) color: vec4<f32> }`. **The struct name changes too**
(`OutputType` vs `OutputStruct`) — that is Three's MRT branch in `NodeBuilder`, not an accident;
match it.

The material has no uv, no map, no lights: `DiffuseColor = vec4( object.nodeUniform0, 1.0 )` and
the two outputs.

### 3.4 The bloom quads

* **`Bloom_highPass`** (`dump/m03_*.wgsl`) — 5 statements. Samples two textures with the raw uv
  (no texture matrix: both are pass textures), multiplies them, and
  `mix( vec4(0), c, smoothstep( threshold, threshold + smoothWidth, dot( c.rgb, vec3(0.2126, 0.7152, 0.0722) ) ) )`.
  Defaults: `threshold` 0, `smoothWidth` 0.01, `strength` 1, `radius` 0.
* **`Bloom_separable`** (`dump/m05..m09`) — one module per mip, differing only in the two baked
  coefficient arrays. Mip 0 (kernel radius 6):

  ```wgsl
  nodeVar1 = nodeVar0.xyz * vec3<f32>( 0.19947 );           // centerWeight
  for ( var i : i32 = 0; i < 3; i ++ ) {                     // literal bound, 3 bilinear taps
      nodeVar2 = ( object.nodeUniform3 * object.nodeUniform4 ) *
                 vec2<f32>( array<f32,3>( 1.40733340004593, 3.294214972162989, 5.0 )[ i ] );
      … += ( s1.xyz + s2.xyz ) *
           vec3<f32>( array<f32,3>( 0.2970163278514283, 0.09175375661117716, 0.008764100149861079 )[ i ] );
  }
  ```

  The offsets/weights come from `_getSeparableBlurMaterial`'s CPU-side Gaussian
  (`0.39894 * exp(-0.5 i²/σ²) / σ`, σ = radius/3) merged into bilinear taps. Kernel radii are
  `[6, 10, 14, 18, 22]` → `[3, 5, 7, 9, 11]` taps. **Compute them the same way; do not paste the
  dumped floats** — but diff against them.
  Three `mat3x3` texture matrices appear in the object block because this material uses
  `texture( null )` (matrix on), unlike the pass textures.
* **`Bloom_comp`** (`dump/m11_*.wgsl`) — five texture samples, `array<f32,5>(1,0.8,0.6,0.4,0.2)`
  as a `var<private>` array, a `NodeBuffer_1297 : array<vec4<f32>, 5>` uniform (that is
  `uniformArray( bloomTintColors )`), and `fn4( factor, radius ) = mix( factor, 1.2 - factor, radius )`
  (`lerpBloomFactor`). One long additive expression, then `* strength`.

### 3.5 Shared-context detail

`BloomNode.setup()` does `context( builder.getSharedContext() )` and assigns it as each quad
material's `contextNode`. That is what keeps these quads out of the outer graph's uniform
numbering; in the port the equivalent is simply building each quad material's fragment node in its
own `NodeBuilder` pass, which is already how `RenderPipeline` builds its quad.

---

## 4. Gap list against the port

| what the example needs | status | where it lands / what to port |
|---|---|---|
| `PassNode`, `RenderPipeline`, quad, `render_quad`, `set_render_target` | **have** | rung 9 |
| `MeshBasicNodeMaterial` + `color`, `IcosahedronGeometry`, `PerspectiveCamera` | **have** | rung 1 |
| clear `(0,0,0,0)` with no background | **have** | rung 9 |
| `smoothstep`, `dot`, `mix`, `luminance` coefficients, `uniform` | **have** | `src/nodes/tsl.rs` |
| fixed-bound `Loop` + `const_array` + `element_node` | **have** | `src/nodes/tsl.rs:2036`, `:207`, `:1070` — the separable blur needs exactly these |
| `texture()` with its `mat3x3` matrix, and pass textures without | **have** | rung 9's documented asymmetry |
| **`NeutralToneMapping`** | **missing** | see `radial_blur` PLAN §4 — free if that rung landed |
| **effect node owning RTs + several quad materials + `Renderer::clear()`** | **missing** | see `ssaa` PLAN §4 — free if that rung landed |
| **`mrt()` / `material.mrtNode` / `pass.setMRT()`** | **missing** | **the big one.** Three source: `src/nodes/core/MRTNode.js` (~150 lines), `NodeBuilder`'s `OutputType` branch, `RenderTarget`'s multi-texture support, and `Renderer`'s multi-attachment pass setup. Lands in `src/nodes/`, `src/renderer/render_target.rs`, `src/renderer/mod.rs`, `src/materials/node_material.rs`. **~450 lines Rust.** **Touches the fragment-output path every material uses** — the single largest regression risk in this family. |
| **`getTextureNode( name )` on a pass** | **missing** | `src/renderer/pass.rs` — resolve an MRT name to the right attachment texture. ~60 lines |
| **`uniformArray`** (`array<vec4<f32>, 5>` in a uniform block) | **missing** | `src/nodes/tsl.rs` + `src/renderer/programs.rs` layout. Three source: `src/nodes/accessors/UniformArrayNode.js`. ~120 lines |
| **`BloomNode`** | **missing** | new `src/nodes/display/bloom.rs`. 599 lines JS (~400 without jsdoc) → **~500 lines Rust**: 11 RTs, the `setSize` rounding, 7 quad materials, the CPU-side Gaussian, the composite. |
| `RenderTarget` with `depth_buffer: false` | check | `RenderTargetOptions` has `depth_buffer` (`src/renderer/pass.rs` uses it); confirm a `None` depth attachment reaches the pass descriptor |
| `.renderOutput()` as a method on a node (vs `RenderPipeline`'s wrapper) | **partly** | `render_output()` is a free function (`src/materials/node_material.rs:468`); here it must be applied to the output node while `output_color_transform` is false — which `RenderPipeline` already supports |

**Estimated new Rust: ~1050 lines** on top of `ssaa` + `radial_blur`. That is why this is **two
rungs**, split at §6.

No dependency on rungs 10, 11 or 12.

---

## 5. Gates beyond the pixel diff

1. **MRT WGSL identity, before any bloom code.** A `dump_wgsl` entry for a `MeshBasicNodeMaterial`
   with `mrt_node = mrt({ output, bloomIntensity: uniform(1) })`, diffed against
   `dump/m01_fragment_fragment.wgsl` — including the `OutputType` struct name and the
   `@location(0) m0` / `@location(1) m1` member names.
2. **The MRT pass descriptor.** Extend rung 9's `eprintln!` trace: pass 1 must have **two**
   `rgba16float` colour attachments plus `depth24plus`, and passes 2–13 exactly one colour
   attachment and **no depth**.
3. **Gaussian coefficients as a unit test.** Compute offsets/weights for kernel radii
   `[6,10,14,18,22]` and assert mip 0 equals
   `offsets [1.40733340004593, 3.294214972162989, 5.0]`,
   `weights [0.2970163278514283, 0.09175375661117716, 0.008764100149861079]`,
   `centerWeight 0.19947` — read straight off `dump/m05_fragment_fragment_Bloom_separable.wgsl`.
   Do the same for the other four from `m06..m09`. Pure arithmetic, no GPU.
4. **The 50 spheres oracle, dumpable now.** `page.evaluate` over `scene.children` for
   `[position.toArray(), scale.x, material.color.getHex(), material.mrtNode.get('bloomIntensity').value]`
   into `spheres_t0.json` — 50 rows. This pins the 450-draw sequence *and* which spheres bloom,
   which is the failure this example is most likely to die on.
5. **Mip-size table as a unit test**: 800×500 → 400×250 → 200×125 → 100×62 → 50×31 → 25×15.
6. **Full ladder re-run** after the MRT change — it touches every material's fragment output.

---

## 6. Order of work — two rungs

### Rung A — MRT (leaves the ladder green without any bloom)

1. `mrt()` node, `material.mrt_node`, `PassNode::set_mrt`, multi-texture `RenderTarget`,
   multi-attachment pass setup, the `OutputType` struct branch in the builder.
2. `PassNode::texture_node( name )`.
3. Gate: §5.1 WGSL identity, §5.2 descriptor trace, and **the whole existing ladder unchanged**
   (every rung still emits `OutputStruct { color }` when no MRT is set).
4. Optional but cheap: land `webgpu_postprocessing_motion_blur`'s MRT half here as a second
   consumer, or simply stop and let rung B use it.

### Rung B — `BloomNode` and the example

5. `uniformArray`. Gate: a `dump_wgsl` entry emitting `array<vec4<f32>, 5>`.
6. The three quad materials in isolation, each with its own WGSL gate against
   `dump/m03`, `dump/m05..m09`, `dump/m11`. Do the separable one second — it carries §5.3.
7. `BloomNode`'s 11 render targets, `set_size` rounding (§5.5) and the 12-pass `render()`.
   Gate: the §3.2 pass table, all 14 rows.
8. `examples/webgpu_postprocessing_bloom_selective.rs` + the `tests/e2e/main.rs` entry, with the
   §5.4 oracle asserted before the pixel diff. Watch `STEADY_FRAME_CEILING`: 256k triangles plus
   12 quad passes per frame.
9. `docs/postprocessing.md` gains MRT and bloom sections.

---

## 7. What this rung unlocks

* **MRT** → `bloom_emissive`, `lensflare`, `motion_blur` (three more 0.0% members), and is the
  prerequisite for the currently-excluded `ao`/`ssgi`/`ssr`/`sss`/`traa` group should Three ever
  un-exclude them.
* **`BloomNode`** → `bloom` (add glTF + `AnimationMixer` + Reinhard), `bloom_emissive` (add
  HDRLoader + glTF + ACES), `anamorphic` (add `rtt()`), `lensflare` (add `LensflareNode` +
  `GaussianBlurNode`).
* **`uniformArray`** is used by `GodraysNode`, `SSAONode`, `GTAONode` and `SSRNode`.
* `NeutralToneMapping` is shared with `radial_blur`, `difference`, `direct`, `anamorphic`,
  `dof_basic`.

Counting only the 0.0% members: this lands 1 and makes 4 more cheap — the best ratio in the
family, which is why it is worth two rungs.

## 8. Verdict

**Two rungs**, split at MRT as in §6. Not one: ~1050 lines of new Rust, and the MRT half alone
rewrites the fragment-output path that every existing rung depends on — that deserves its own
green ladder before `BloomNode`'s 11 render targets are stacked on it.

Top three risks:

1. **MRT changes the fragment output for every material.** The struct name itself changes
   (`OutputStruct` → `OutputType`). A regression here is a regression on all ten existing rungs.
   Rung A's gate is the whole ladder, not this example.
2. **The 450-draw random sequence decides which spheres bloom.** An off-by-one is a visually
   dramatic, hard-to-attribute failure. Dump `spheres_t0.json` (§5.4) *before* writing the scene.
3. **The Gaussian coefficients and the mip rounding.** Five kernels, five sizes, and `100 → 62`
   rather than `50`. Both are CPU-side arithmetic with dumped ground truth (§5.3, §5.5) — test
   them before touching the GPU.

Dependencies: **`ssaa` first** (RT-owning effect nodes, `Renderer::clear()`), and `radial_blur`
before that (`NeutralToneMapping`). No dependency on rungs 10, 11 or 12.
