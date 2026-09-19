# Rung scout — `webgpu_postprocessing_ssaa`

Scouted 2026-09-19. Example: `~/src/vendor/three.js/examples/webgpu_postprocessing_ssaa.html`
(vendor r186, `rung0/grader-flags.patch` applied). Family context and the order this sits in:
`scouts/postprocessing-family/SURVEY.md`. **Take `webgpu_postprocessing_radial_blur` first** — not
because this rung needs it, but because both need instance colour and that is the only overlap.

Files next to this plan:

| file | what |
|---|---|
| `dump/dump.json` | all 276 chronological calls, 26 render passes with their command streams, every descriptor |
| `dump/m00_vertex_vertex.wgsl`, `dump/m01_fragment_fragment.wgsl` | the scene program (`MeshStandardNodeMaterial`, instanced, 3 point lights + ambient) |
| `dump/m02_vertex_vertex_SSAA.wgsl`, `dump/m03_fragment_fragment_SSAA.wgsl` | **the accumulation quad — 40 lines** |
| `dump/m04_vertex_vertex_RenderPipeline.wgsl`, `dump/m05_fragment_fragment_RenderPipeline.wgsl` | the final quad (`renderOutput` only) |
| `dump/actual_full.png`, `dump/actual.jpg` | Three's deterministic frame, raw and grader-scaled |
| `reference-r186.jpg` | `examples/screenshots/webgpu_postprocessing_ssaa.jpg` |

**Result: 6 shader modules, 3 render pipelines, 4 bind-group layouts, 0 compute pipelines,
6 textures, 1 sampler, 9 buffers, 7 bind groups, 26 render passes, 27 submits, 17 draws.**

---

## 1. Grade confirmation — 0.0%, twice

```
Diff 0.0% in file: webgpu_postprocessing_ssaa (3.0s)   # run B, 2026-09-19
Diff 0.0% in file: webgpu_postprocessing_ssaa (3.0s)   # run C, 2026-09-19
```

Logs: `scouts/postprocessing-family/e2e-runs-2026-09-19.log`. **Not** on the `exceptionList`.

---

## 2. What the page does

`WebGPURenderer()` — no `antialias`. `setPixelRatio(1)`, `setSize(800, 500)`. **No
`renderer.toneMapping` is set**, so it stays `NoToneMapping` and the final quad's `renderOutput` is
just the clamp/unpremultiply → sRGB OETF → premultiply sandwich (`dump/m05_*.wgsl` confirms: no
tone-mapping `fn` at all). `renderer.inspector` + five `gui.add` — UI only.

`animate()` each frame does `renderer.setClearColor( 0x000000, 1.0 )` (from `params.clearColor`
`'black'`, `clearAlpha` 1), `ssaaRenderPass.sampleLevel = 3`, `camera.view.offsetX = 0`, then
`renderPipeline.render()`. `mesh.rotation` is advanced by `delta`, which is 0.

### 2.1 Scene

* `PerspectiveCamera( 65, 800/500, 3, 10 )`, `position.z = 7`, and — important —
  **`camera.setViewOffset( 800, 500, 0, 0, 800, 500 )`** is called in `init()`. So `camera.view`
  is *enabled* with a zero offset before SSAA ever jitters it, and `SSAAPassNode` copies that
  enabled view as its base (`originalViewOffset.enabled` is true, so the restore path at the end
  is the `setViewOffset` branch, not `clearViewOffset`).
* No `scene.background` → the pass target clears to the renderer clear colour, `(0,0,0,1)`
  (`dump.json` pass 1 `clearValue`), because the page set alpha to 1.
* Lights: `PointLight( 0xefffef, 500 )` at `(-10, -10, 10)`, `PointLight( 0xffefef, 500 )` at
  `(-10, 10, 10)`, `PointLight( 0xefefff, 500 )` at `(10, -10, 10)`, `AmbientLight( 0xffffff, 0.2 )`.
* One `InstancedMesh( SphereGeometry( 3, 48, 24 ), new MeshStandardMaterial(), 120 )` — default
  white, `roughness 1`, `metalness 0` — with per-instance matrices **and** per-instance colours.
  An empty `Group` is added to the scene and never used.

### 2.2 `Math.random` draws before the frame — **960**

Eight per instance, `i = 0 .. 119`:

```js
dummy.position.x = Math.random() * 4 - 2;     // 1
dummy.position.y = Math.random() * 4 - 2;     // 2
dummy.position.z = Math.random() * 4 - 2;     // 3
dummy.rotation.x = Math.random();             // 4
dummy.rotation.y = Math.random();             // 5
dummy.rotation.z = Math.random();             // 6
dummy.scale.setScalar( Math.random() * 0.2 + 0.05 );  // 7
color.setHSL( Math.random(), 1.0, 0.3 );      // 8
```

(`dummy` is a `new THREE.Mesh()`, not an `Object3D` — same matrix composition.) `setMatrixAt` is
called *before* `setColorAt`, but the random draws happen in the order above.

### 2.3 The TSL graph

```js
renderPipeline = new THREE.RenderPipeline( renderer );
ssaaRenderPass = ssaaPass( scene, camera );
renderPipeline.outputNode = ssaaRenderPass.getTextureNode();
```

`outputColorTransform` stays true. `examples/jsm/tsl/display/SSAAPassNode.js` is **344 lines** and
is a `PassNode` **subclass** — `super( PassNode.COLOR, scene, camera, { samples: 0 } )`. Its entire
behaviour lives in `updateBefore()` plus a `setup()` that builds one extra quad material.

`sampleLevel = 3` → `_JitterVectors[3]`, **8** offsets:
`[1,-3] [-1,3] [5,1] [-3,-5] [-5,5] [-7,-1] [3,7] [7,-7]`, each scaled by `0.0625` when handed to
`setViewOffset`. `unbiased` is true, so

```
baseSampleWeight = 1/8 = 0.125
sampleWeight[i]  = 0.125 + (1/32) * ( -0.5 + (i + 0.5) / 8 )
```

giving, for i = 0..7: `0.111328125, 0.1171875, 0.123046875, 0.12890625, 0.134765625, 0.140625,
0.146484375, 0.15234375`. **Those eight constants are the whole numerical content of the effect**
— put them in the plan's test, not in a comment.

---

## 3. Dump reading

### 3.1 Textures

| id | label | format | size | samples |
|---|---|---|---|---|
| 0 | `depthBuffer` | `depth24plus` | 800×500 | 1 |
| **4** | `output` | `rgba16float` | 800×500 | 1 | ← `_sampleRenderTarget` (first used) |
| 5 | `depth` | `depth24plus` | 800×500 | 1 | ← its depth |
| 17 | `DFG_LUT` | `rg16float` | 16×16 | 1 |
| **27** | `output` | `rgba16float` | 800×500 | 1 | ← `PassNode.renderTarget` (the accumulator) |
| 28 | `depth` | `depth24plus` | 800×500 | 1 |

GPU textures are created on first *use*, so the sample target (rendered into first) gets the lower
id even though `_sampleRenderTarget = this.renderTarget.clone()` happens second. Both are
`samples: 1` — the `{ samples: 0 }` the subclass passes to `super()`.

### 3.2 The 26 passes

```
 0  renderContext_0   canvas (tex 2) + depthBuffer      clear (0,0,0,1)   draw(3,1,0,0)  RenderPipeline
 1  clear             tex  4 + depth                    clear             (no draws)
 2  renderContext_2   tex  4 + depth                    load              drawIndexed ×1, 120 instances   ← sample 0
 3  clear             tex 27 + depth                    clear             (no draws)                       ← i === 0 only
 4  renderContext_2   tex 27 + depth                    load              draw(3,1,0,0)  SSAA               ← accumulate 0
 5  clear             tex  4                            clear
 6  renderContext_2   tex  4                            load              drawIndexed    ← sample 1
 7  renderContext_2   tex 27                            load              draw SSAA      ← accumulate 1
 …   (clear tex 4, scene, accumulate) × 6 more
25  renderContext_2   tex 27                            load              draw SSAA      ← accumulate 7
```

So: **1 canvas pass, 8 sample-target clears, 8 scene renders, 1 accumulator clear, 8 accumulation
quads = 26.** Each is its own command encoder and its own submit (27 `queue.submit`, 27
`commandEncoder.finish`). The canvas pass's descriptor is recorded first and executes/submits
first — the rung-9 recording-order divergence, unchanged.

The accumulator clear happens **inside** the `i === 0` iteration, *after* the first scene render
and *before* the first accumulation quad — not at the top of `updateBefore`. Three does it with
`setClearColor( 0x000000, 0.0 ); clear(); setClearColor( back )`, but the dumped `clearValue` is
`(0,0,0,1)`; the `clear` pass descriptor carries the renderer's *current* value and the
alpha-0 clear is applied by the pass itself. Reproduce the dumped descriptor, not the JS reading.

**`copyTextureToTexture( sampleRT.depthTexture, renderTarget.depthTexture )` at the end of
`updateBefore` does not appear anywhere in the 276-call `order` log.** Nothing in this example
reads the pass's depth, so it is safe to skip — but say so in `docs/postprocessing.md` rather than
silently omitting it.

### 3.3 The three pipelines

| pipeline | targets | depth | vertex buffers |
|---|---|---|---|
| `renderPipeline_MeshStandardMaterial_17` | `rgba16float`, no blend | `depth24plus` | position(12), normal(12), instance colour(12, `stepMode: instance`) — **no uv** |
| **`renderPipeline_SSAA_21`** | `rgba16float`, **blend `one/one/add` for both colour and alpha** | `depth24plus`, `depthWriteEnabled: false`, `depthCompare: "always"` | uv(8) only |
| `renderPipeline_RenderPipeline_19` | `rgba8unorm` (canvas), no blend | `depth24plus` | uv(8) only |

The SSAA blend is exactly what `src/materials/blending.rs:185` already produces for
`Blending::Additive` **with** `premultiplied_alpha` — `set_blend(One, One, One, One)`. The
`depthCompare: always` + `depthWriteEnabled: false` pair is `depthTest: false, depthWrite: false`
on a `NodeMaterial`.

### 3.4 The accumulation quad — 40 lines

`dump/m03_fragment_fragment_SSAA.wgsl`:

```wgsl
struct objectStruct { nodeUniform1 : mat3x3<f32>, nodeUniform2 : f32 };
…
nodeVar0 = textureSample( nodeUniform0, nodeUniform0_sampler,
                          ( object.nodeUniform1 * vec3<f32>( nodeVarying0, 1.0 ) ).xy );
output.color = fn0( fn1( ( nodeVar0 * vec4<f32>( object.nodeUniform2 ) ) ) );
```

`fn1` is `unpremultiplyAlpha`, `fn0` is `premultiplyAlpha` — the latter comes from
`premultipliedAlpha: true` on the material, which `NodeMaterial` applies to the output. Two
uniforms: `nodeUniform1` is the **texture matrix** (a `mat3x3`, because the quad uses
`texture( sampleRT.texture )` — *not* `passTexture`, so `setUpdateMatrix` is left on, unlike the
rung-9 pass texture) and `nodeUniform2` is `sampleWeight`.

That asymmetry is the one subtle thing in the shader and it is the same one
`docs/postprocessing.md` already documents from the other direction.

### 3.5 The final quad

`dump/m05_fragment_fragment_RenderPipeline.wgsl` samples the accumulator with the **raw `uv()`
varying** (it *is* a `passTexture`) and applies `renderOutput` with no tone mapping. Nine lines of
flow. Nothing new.

---

## 4. Gap list against the port

| what the example needs | status | where it lands / what to port |
|---|---|---|
| `PassNode` with its own `rgba16float` RT + `depth` | **have** | `src/renderer/pass.rs` |
| `RenderPipeline` + full-screen triangle + `render_output` | **have** | `src/renderer/render_pipeline.rs` |
| `Renderer::render_quad( &QuadMesh )` into an arbitrary RT | **have** | `src/renderer/mod.rs:1278` + `set_render_target` (`:649`) — this is why the rung is cheap |
| `PerspectiveCamera::set_view_offset` / `clear_view_offset` | **have** | `src/cameras/perspective_camera.rs:135`, `:160` |
| `Blending::Additive` + `premultiplied_alpha` → `one/one/add` | **have** | `src/materials/blending.rs:185` |
| `depth_test: false` / `depth_write: false` on a material | **have** | `src/materials/mod.rs` |
| `SphereGeometry( 3, 48, 24 )`, `PointLight`, `AmbientLight`, `MeshStandardMaterial` | **have** | `src/geometries/sphere.rs`, `src/lights/`, `src/materials/physical.rs` |
| `premultiply_alpha` / `unpremultiply_alpha` | **have** | `src/nodes/tsl.rs:2269`, `:2283` |
| `texture( rt.texture )` **with** its `mat3x3` texture matrix, on a quad | **have** | `src/nodes/tsl.rs:1745 texture` (rung 9 dumps show the same interleave) |
| **`InstancedMesh.setColorAt` → instance-colour attribute + varying** | **missing** | same gap as `radial_blur`; see that PLAN §4. ~120 lines. If `radial_blur` lands first this is free. |
| **`RenderTarget::clone()`** (a second RT with the same descriptor, own textures) | **missing** | `src/renderer/render_target.rs` — ~40 lines |
| **`Renderer::clear()` as a standalone pass** (a `beginRenderPass` with `loadOp: clear` and no draws) | **missing** | `src/renderer/mod.rs`. The dump shows 9 of these. Needs `autoClear = false` semantics too, since the scene renders use `loadOp: load`. ~80 lines, **touches the shared pass setup — the regression risk of this rung.** |
| **`Renderer::set_clear_color( color, alpha )`** at runtime | check | `_clearColor` exists (rung 9); a public setter + the save/restore around the accumulator clear. ~30 lines |
| **`SSAAPassNode`** itself | **missing** | new `src/renderer/ssaa_pass.rs` (or `src/nodes/display/`). Port of `examples/jsm/tsl/display/SSAAPassNode.js` — 344 lines JS, of which ~120 are jsdoc and ~50 are `_JitterVectors`. **~280 lines Rust**: the six jitter tables (only level 3 is exercised; port all six), the weight formula, the 8-iteration loop, the second quad material, and the same explicit-`render()` ownership divergence rung 9 already established for `PassNode`. |
| `copyTextureToTexture` for depth | **missing, and skippable** | not in the trace; document the omission |

**Estimated new Rust: ~550 lines** (~430 if `radial_blur` landed first), plus the example (~200)
and its `tests/e2e/main.rs` entry.

Nothing depends on rungs 10, 11 or 12.

---

## 5. Gates beyond the pixel diff

1. **The eight sample weights**, as a plain unit test against the constants in §2.3. They are the
   only floating-point content of the effect and a transcription error in `unbiased` would show up
   as a uniform brightness shift the JPEG comparison might still pass.
2. **WGSL identity for the accumulation quad.** A `dump_wgsl` entry building
   `unpremultiply_alpha( texture(&sample_rt.texture).mul(sample_weight) )` on a material with
   `premultiplied_alpha = true`, diffed against `dump/m03_fragment_fragment_SSAA.wgsl`. 40 lines —
   this is the cheapest possible proof that the texture-matrix/`passTexture` asymmetry is right.
3. **The 26-pass trace.** Extend rung 9's temporary `eprintln!` in `Renderer::draw` to also print
   clear-only passes, and diff the sequence against §3.2 — target texture id, `loadOp`, draw
   count. Getting the accumulator's clear at the right point (inside iteration 0, after the first
   scene render) is the single most likely bug and this trace catches it instantly.
4. **View-offset oracle, dumpable now.** `page.evaluate` the eight `camera.projectionMatrix`
   values Three computes for the eight jittered `setViewOffset` calls (hook `updateBefore` or
   recompute from `_JitterVectors[3]`), into `jitter_projections_t0.json` — 8 × 16 floats — and
   assert the port's matrices against it. Cheap and it isolates camera maths from blending maths.
5. **Instance oracle**, as in `radial_blur` PLAN §5.4: `instanceMatrix.array` (120×16) and
   `instanceColor.array` (120×3) from the page.
6. **Re-grade the whole ladder** after the `clear()`/`autoClear` change.

---

## 6. Order of work

1. `RenderTarget::clone()`. Gate: existing rungs unchanged.
2. `Renderer::clear()` as its own pass + `autoClear` + public `set_clear_color`. Gate: **full
   ladder re-run** — this touches every pass's `loadOp`. Nothing yet calls `clear()` explicitly, so
   the ladder must be bit-identical.
3. Instance colour (skip if `radial_blur` already landed it). Gate: `webgpu_instance_mesh` green.
4. The SSAA quad material and its WGSL gate (§5.2), standalone, before any multi-pass logic.
5. `SsaaPassNode`: the jitter tables, the weight formula (§5.1 test), and `render()` doing the
   8× loop. Gate: the §5.3 pass trace matches all 26 rows.
6. `examples/webgpu_postprocessing_ssaa.rs` + the `tests/e2e/main.rs` entry. Gate: pixel diff.
   Note the steady-frame budget in `tests/e2e/main.rs` — **this example renders the scene eight
   times per frame** (120 spheres of `SphereGeometry(3,48,24)` ≈ 2300 triangles each); confirm it
   fits under `STEADY_FRAME_CEILING` (600 ms debug / 100 ms release) and, if not, say so rather
   than raising the ceiling.
7. `docs/postprocessing.md` gains an "effect nodes that own render targets" section, including the
   skipped depth copy.

---

## 7. What this rung unlocks

The RT-owning-effect-node pattern plus `Renderer::clear()`/`autoClear` is what
`examples/jsm/tsl/display/` is built on. After it, these become "port the node, not the
machinery":

* `BloomNode` → **`bloom_selective`, `bloom`, `bloom_emissive`, `anamorphic`, `lensflare`**
  (though bloom also needs MRT — see that plan)
* `GaussianBlurNode` → **`fog`, `lensflare`**
* `boxBlur` → **`dof_basic`**
* `GodraysNode` + `BilateralBlurNode` → **`godrays`**
* `OutlineNode` → **`outline`**

That is nine of the twenty gradeable members. It is also the pattern every future
`three/addons/tsl/display` port will reuse.

## 8. Verdict

**Rung-sized: one Opus worker, one sitting** — provided `radial_blur` goes first (it carries
instance colour). ~550 lines of new Rust, ~430 if it follows `radial_blur`. The effect's shader is
40 lines and every GPU primitive it needs already exists in the port; the work is orchestration,
and the 26-row pass table in §3.2 specifies it completely.

Top three risks:

1. **`Renderer::clear()` / `autoClear` touches the shared pass setup.** Every existing rung's
   `loadOp` and clear value flows through it. Step 2's full-ladder gate is mandatory, and rung 9's
   clear-alpha history (`docs/rung9-progress.md`, "The one behaviour change to an earlier rung")
   shows how easily that number moves.
2. **The accumulator's clear position.** It sits inside iteration 0, between the first scene render
   and the first accumulation draw, and its dumped `clearValue` is `(0,0,0,1)` even though the JS
   reads `setClearColor(0x000000, 0.0)`. Read the dump, not the JS.
3. **Performance.** Eight full scene renders of 120 high-tessellation spheres per frame against
   `STEADY_FRAME_CEILING`. If it does not fit, this is a two-rung example, not a failed one.

No dependency on rungs 10, 11 or 12.
