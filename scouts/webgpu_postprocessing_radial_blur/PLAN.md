# Rung scout — `webgpu_postprocessing_radial_blur`

Scouted 2026-09-19. Example: `~/src/vendor/three.js/examples/webgpu_postprocessing_radial_blur.html`
(vendor r186, `rung0/grader-flags.patch` applied). Family context and the order this sits in:
`scouts/postprocessing-family/SURVEY.md`.

Files next to this plan:

| file | what |
|---|---|
| `dump/dump.json` | every module / bind-group layout / pipeline layout / render pipeline / buffer / texture / view / sampler / bind group the driver was asked to create, both render passes with their command streams, and the 65-entry chronological `order` log |
| `dump/m00_vertex_vertex.wgsl`, `dump/m01_fragment_fragment.wgsl` | the one scene program (`MeshStandardNodeMaterial`, instanced, flat-shaded) |
| `dump/m02_vertex_vertex_RenderPipeline.wgsl`, `dump/m03_fragment_fragment_RenderPipeline.wgsl` | the quad — **the whole effect is in the fragment one** |
| `dump/actual_full.png` | Three's own 800×500 deterministic frame |
| `dump/actual.jpg` | the same frame through `test/e2e/image.js` `scale()` — byte-for-byte what the grader compares |
| `reference-r186.jpg` | `examples/screenshots/webgpu_postprocessing_radial_blur.jpg` |

Dump: `tools/dump-webgpu.mjs` from the `tools-dump` worktree, under the GPU lock. It reported one
404 (`Failed to load resource`) on the page, as it does for every example in this family — the
Inspector's optional asset; it does not affect the frame.

**Result: 4 shader modules, 2 render pipelines, 3 bind-group layouts, 0 compute pipelines,
4 textures, 1 sampler, 9 buffers, 2 render passes, 2 draw calls.**

---

## 1. Grade confirmation — 0.0%, twice

```
Diff 0.0% in file: webgpu_postprocessing_radial_blur (3.0s)   # run B, 2026-09-19
Diff 0.0% in file: webgpu_postprocessing_radial_blur (3.7s)   # run C, 2026-09-19
```

Logs: `scouts/postprocessing-family/e2e-runs-2026-09-19.log`. **Not** on the `exceptionList` in
`test/e2e/puppeteer.js`. No sibling substitution needed.

---

## 2. What the page does

`WebGPURenderer()` — **no `antialias`**, so `renderer.samples === 0` and the pass target is
single-sampled (`dump.json` texture 4: `rgba16float`, 800×500, `sampleCount: 1`).
`setPixelRatio(devicePixelRatio)` → 1, `setSize(800, 500)`.
`renderer.toneMapping = THREE.NeutralToneMapping`.
`renderer.inspector = new Inspector()` plus one `createParameters` and six `gui.add` calls — UI
only, no draws, no random.

### 2.1 Scene

* `PerspectiveCamera( 45, 800/500, 0.1, 200 )`, `position.z = 50`, never moved (no controls).
* `scene.background = new Color( 0x000000 )` → clear `(0,0,0,1)` with the forced alpha of 1 that
  `Background.update()` applies for a `Color` background (`docs/postprocessing.md`).
* `HemisphereLight( 0xffffff, 0x8d8d8d )` (default intensity 1) at `(0, 1000, 0)`.
* `PointLight( 0xffffff, 1000 )` at the origin.
* One `Group` holding one `InstancedMesh( TetrahedronGeometry(), MeshStandardMaterial({ flatShading: true }), 100 )`.
  `MeshStandardMaterial` defaults: white, `roughness 1`, `metalness 0`.
  The group's `rotation.y` is advanced by `delta * 0.1` in `animate()` — and `delta` is **0**
  under the frozen clock, so the graded frame is the untouched `init()` state.

### 2.2 `Math.random` draws before the frame — **700**

Seven per instance, in this order, for `i = 0 .. 99`:

```js
dummy.position.x = Math.random() * 50 - 25;   // 1
dummy.position.y = Math.random() * 50 - 25;   // 2
dummy.position.z = Math.random() * 50 - 25;   // 3
// center guard: no draw
dummy.scale.setScalar( Math.random() * 2 + 1 );   // 4
dummy.rotation.x = Math.random() * Math.PI;   // 5
dummy.rotation.y = Math.random() * Math.PI;   // 6
dummy.rotation.z = Math.random() * Math.PI;   // 7
```

The centre guard (`if center.length() < 6` → normalize and scale to 6, mutating x and y) draws
nothing but **must be reproduced**; it moves instances off the origin, which is exactly where the
radial blur's centre is, so getting it wrong is highly visible.

`col.setHSL( 0.55 + (i/100) * 0.15, 1, 0.2 )` is deterministic — no draw. It goes through
`setColorAt`, and the dump confirms it reaches the shader (§3).

`Renderer::skip_random_draws` is not needed here *if* the port makes the same 700 draws in the
same order; there is no random consumed by anything but this loop.

### 2.3 The TSL graph

```js
const scenePass = pass( scene, camera );
const weightUniform   = uniform( float( 0.9 ) );
const decayUniform    = uniform( float( 0.95 ) );
const exposureUniform = uniform( int( 5 ) );
const countUniform    = uniform( int( 32 ) );
renderPipeline.outputNode = radialBlur( scenePass,
    { weight: weightUniform, decay: decayUniform, count: countUniform, exposure: exposureUniform } );
```

`renderPipeline.outputColorTransform` is left **true**, so the quad's fragment is
`renderOutput( radialBlur(...), NeutralToneMapping, sRGB )`.

`examples/jsm/tsl/display/radialBlur.js` is **68 lines** and is a single `Fn` — no render targets,
no extra passes. It uses `convertToTexture`, `interleavedGradientNoise( screenCoordinate )`,
`.toConst()`, a node-bounded `Loop`, `.toVar()`, `addAssign`/`mulAssign`/`divAssign`, and `mix`.
`options.premultipliedAlpha` is false here, so both `premultiplyAlpha` branches are dead.

Note the dumped uniform types: `count` and `exposure` are written as `int(...)` in the page but
both land in the object block as **`f32`** (`i32( object.nodeUniform1 )` is the loop bound). Port
them as float uniforms.

---

## 3. Dump reading

### 3.1 Passes — only two

| # | encoder | attachments | load | draws |
|---|---|---|---|---|
| 0 | `renderContext_0` | canvas (texture 2), `depth24plus` | clear `(0,0,0,0)` | 1 — `renderPipeline_RenderPipeline_18`, `draw(3, 1, 0, 0)` |
| 1 | `renderContext_1` | `output` (texture 4, `rgba16float` 800×500), `depth` (texture 5, `depth24plus`) | clear | 1 — `renderPipeline_MeshStandardMaterial_17`, indexed, 100 instances |

Recording order is the rung-9 order (`docs/postprocessing.md`, "Divergence: who fires
updateBefore"): the canvas pass's descriptor is created first, the pass render executes and is
submitted first. Nothing else — no mipmap chain, no output pass (`outputColorTransform` keeps the
transform inside the quad shader, so `needsFrameBufferTarget` is false).

Textures: `depthBuffer` (canvas depth), `output` `rgba16float`, `depth` `depth24plus`, and
`DFG_LUT` `rg16float` 16×16 — the port already generates that one in `src/materials/dfg_lut.rs`.

### 3.2 The scene program

`m00_vertex_vertex.wgsl` / `m01_fragment_fragment.wgsl` — the rung-7-ish physical material the port
already builds, plus **two instanced things**:

* `NodeBuffer_1367 : array<mat4x4<f32>, 100>` at `@binding(3) @group(1)` — `instanceMatrix` as a
  *uniform array*, indexed by `instanceIndex`; the port's `instance_matrix(count)` already does
  this (`src/nodes/tsl.rs:1882`).
* `@location(2) nodeAttribute1 : vec3<f32>` with `stepMode: "instance"`, `arrayStride 12`,
  carried to the fragment stage as `@location(2) vInstanceColor : vec3<f32>`. **This is
  `setColorAt` and the port does not have it.**

Vertex buffers: `position` (stride 12), `normal` (stride 12), instance colour (stride 12,
`stepMode: instance`). No uv — the material has no map.

### 3.3 The quad program — the whole effect

`m02_vertex_vertex_RenderPipeline.wgsl` is the rung-9 full-screen triangle verbatim
(`array<f32,3>(-1,-1,3)[vertexIndex]`, uv attribute at stride 8, `draw(3,1,0,0)`).

`m03_fragment_fragment_RenderPipeline.wgsl` is 60 statements and contains, in order:

* four helper `fn`s: `interleavedGradientNoise`, `fn1` (= `unpremultiplyAlpha`),
  `neutralToneMapping`, `sRGBTransferOETF`, `fn0` (= `premultiplyAlpha`);
* the pass texture at `@binding(0,1) @group(1)`, sampled with the **raw `uv()` varying** and no
  texture matrix — `radialBlur` reads `textureNode.uvNode || uv()`, and a `PassTextureNode` has
  `setUpdateMatrix(false)` (the rung-9 asymmetry, `docs/postprocessing.md`);
* `objectStruct { nodeUniform1: count, nodeUniform2: weight, nodeUniform3: decay, nodeUniform4: exposure }`
  — all `f32`, in that declaration order;
* `renderStruct { nodeUniform5 : f32 }` at `@binding(0) @group(0)` — `toneMappingExposure`;
* **two `let` bindings**, `nodeConst0` (the base tap) and `nodeConst1` (the per-step offset) —
  `.toConst()`, which the port does not emit today;
* a **node-bounded** `for ( var i : i32 = 0; i < i32( object.nodeUniform1 ); i ++ )` with four
  statements in the body, ending in `nodeVar3 = nodeVar3 * object.nodeUniform3` (the decay);
* the composite `mix( blur, base * 2, 0.5 )`, then `renderOutput`'s
  clamp → unpremultiply → `neutralToneMapping` → `sRGBTransferOETF` → premultiply sandwich.

The `neutralToneMapping` body in the dump is the exact WGSL the port must emit; copy it from
`dump/m03_fragment_fragment_RenderPipeline.wgsl:78-110` and check it against
`three.js/src/nodes/display/ToneMappingFunctions.js` (`neutralToneMapping`) so the node graph, not
a hand-written string, produces it.

**Surprise worth naming:** `nodeVar0` (the running sample uv) is declared as a module-scope
`var<private> nodeVar0 : vec2<f32>` and *reassigned* inside the loop; `nodeVar2` (the accumulator)
likewise. That is `.toVar()` on a `vec2( textureNode.uvNode || uv() )` and on `vec4()`. The port's
`to_var` already produces module-scope `var<private>`, so this matches — but the loop body must
mutate the *same* var, i.e. `add_assign` inside `loop_statement`'s body, which the port has.

---

## 4. Gap list against the port

Port read at `/home/tom/src/projects/three-rs/main`.

| what the example needs | status | where it lands / what to port |
|---|---|---|
| `pass( scene, camera )`, `RenderPipeline`, full-screen quad | **have** | `src/renderer/pass.rs`, `src/renderer/render_pipeline.rs` (rung 9) |
| clear `(0,0,0,1)` from a `Color` background | **have** | `Background::update` (rung 9 fixed the alpha semantics) |
| `PerspectiveCamera(45, 1.6, 0.1, 200)` | **have** | `src/cameras/perspective_camera.rs` |
| `HemisphereLight`, `PointLight` | **have** | `src/lights/light_object.rs` (`LightKind::Hemisphere`) |
| `TetrahedronGeometry()` | **have** | `src/geometries/polyhedron.rs::tetrahedron_geometry` |
| `MeshStandardMaterial` + `flatShading` + DFG LUT | **have** | `src/materials/physical.rs`, `dfg_lut.rs`, `flat_shading` (`src/materials/mod.rs:159`) |
| `InstancedMesh` matrices (uniform `array<mat4x4,100>`) | **have** | `src/nodes/tsl.rs:1882 instance_matrix` |
| `interleavedGradientNoise` | **have** | `src/nodes/tsl.rs:2399` — diff the emitted `fn` against the dump |
| `premultiplyAlpha` / `unpremultiplyAlpha` / `sRGBTransferOETF` / `renderOutput` | **have** | `src/materials/node_material.rs:468` |
| `mix`, `clamp`, `fract`, `dot`, `float` uniforms | **have** | `src/nodes/tsl.rs` |
| **`InstancedMesh.setColorAt` → instance-colour attribute + `vInstanceColor` varying** | **missing** | renderer + node-system. `src/objects/instanced_mesh.rs` has the `instance_color` field (line 39) but nothing reads it. Needs: a third vertex buffer with `stepMode: instance`, an `attribute("nodeAttribute1", Vec3)` at location 2, a varying, and `DiffuseColor *= vInstanceColor` in `setupDiffuseColor`. Three source: `src/objects/InstancedMesh.js` + `NodeMaterial.setupDiffuseColor`'s `instanceColor` branch. ~40 lines JS to read, **~120 lines Rust** across `src/renderer/mod.rs`, `src/objects/instanced_mesh.rs`, `src/materials/node_material.rs`. **Touches the shared vertex-buffer path — regression risk for `webgpu_instance_mesh` (rung 2); re-grade it.** |
| **`NeutralToneMapping`** | **missing** | `ToneMapping` enum (`src/materials/mod.rs:80`) + a `neutral_tone_mapping(color, exposure)` in `src/nodes/tsl.rs` beside `reinhard_tone_mapping`/`aces_filmic_tone_mapping`, and the arm in `render_output`. Three source: `src/nodes/display/ToneMappingFunctions.js` (~35 lines JS). **~60 lines Rust.** The exact target WGSL is in the dump. |
| **`.toConst()`** (a WGSL `let nodeConstN = …`) | **missing** | node-system. `to_var` exists (`src/nodes/tsl.rs:240`); `to_const` is the same shape with a `let` emission and its own `nodeConstN` counter. Three source: `Node.toConst` / `NodeBuilder.getConst`. **~50 lines Rust** in `src/nodes/tsl.rs` + `src/nodes/builder.rs`. **Touches the shared statement emitter — every other rung's WGSL must be re-diffed after.** |
| **node-bounded `Loop`** (`i < i32(uniform)`) | **missing** | node-system. `loop_statement(count: usize, …)` and `loop_n` (`src/nodes/tsl.rs:2036`, `:2310`) take a Rust `usize` and emit a literal bound. Needs a variant taking a `NodeRef` bound and emitting `i32( <expr> )`. Three source: `src/nodes/utils/LoopNode.js` (the `start`/`end`/`type`/`condition` object form). **~80 lines Rust.** |
| `col.setHSL` | check | `src/math/Color` — confirm `set_hsl` exists; 20 lines if not |

Everything else in the file is already on the ladder. Nothing here depends on rungs 10, 11 or 12.

**Estimated new Rust: ~430 lines**, plus the example (~180) and its `tests/e2e/main.rs` entry.

---

## 5. Gates beyond the pixel diff

1. **WGSL identity, both programs.** Add two `examples/dump_wgsl.rs` entries the way rung 9 did
   (`docs/rung9-progress.md` step 1): `radial_blur_quad` built as
   `render_output( radial_blur( to_var(None, texture_uv(&rt_texture, uv())), … ), Neutral )`, and
   `radial_blur_scene` for the instanced flat-shaded standard material. Diff them against
   `dump/m03_fragment_fragment_RenderPipeline.wgsl` and `dump/m0{0,1}_*.wgsl`. Expected
   divergences are only the `docs/nodes.md` §7 set (banner, `// codes` body order, uniform
   numbering). **This alone proves `to_const`, the node-bounded loop and `neutralToneMapping`
   before a single pixel is compared** — do it first.
2. **`neutralToneMapping` as a unit test.** Three has QUnit coverage under
   `test/unit/src/nodes/display/` — check for a `ToneMappingNode` case; failing that, assert the
   ported function against a handful of values computed from
   `src/nodes/display/ToneMappingFunctions.js` by hand (the piecewise points 0.08 and 0.76 are the
   ones worth pinning).
3. **Pass-descriptor trace.** Rung 9 verified its four passes with a temporary `eprintln!` in
   `Renderer::draw`; do the same and check it against §3.1's two-row table — format, size,
   `sampleCount: 1`, clear values, draw counts (1 indexed × 100 instances, then `draw(3,1,0,0)`).
4. **Instance-colour oracle, dumpable now.** The 100 `setColorAt` colours are deterministic
   (`setHSL(0.55 + i/100*0.15, 1, 0.2)`), and the 100 matrices come from the 700-draw sequence.
   Dump both from the page with a small `page.evaluate` (`mesh.instanceColor.array`,
   `mesh.instanceMatrix.array`) into `instances_t0.json` — 100 × (16 + 3) floats — and assert the
   port's buffers against it. That separates "my random sequence is wrong" from "my shader is
   wrong", which is the failure mode most likely to eat the sitting.
5. **Re-grade rung 2.** `webgpu_instance_mesh` shares the instanced vertex-buffer path.

---

## 6. Order of work

Each step leaves the ladder green.

1. `NeutralToneMapping`: the enum arm, `neutral_tone_mapping()`, the `render_output` match arm.
   Gate: a `dump_wgsl` entry emitting the dumped `neutralToneMapping` body byte-for-byte. No
   existing rung changes (none uses it).
2. `to_const`. Gate: every existing `dump_wgsl` entry is unchanged (nothing calls it yet), and a
   new entry emits `let nodeConst0 = …`. Re-run the full e2e ladder — this touches the emitter.
3. Node-bounded `Loop`. Gate: a `dump_wgsl` entry emitting
   `for ( var i : i32 = 0; i < i32( object.nodeUniformN ); i ++ )`. Existing loops unchanged.
4. Instance colour end to end. Gate: `webgpu_instance_mesh` (rung 2) still green, and the
   `instances_t0.json` oracle from §5.4 matches.
5. `radial_blur()` in a new `src/nodes/display/radial_blur.rs` (or `src/nodes/tsl.rs` if the
   project prefers one module). Gate: quad WGSL identical to §3.3.
6. `examples/webgpu_postprocessing_radial_blur.rs` + the `tests/e2e/main.rs` entry, modelled on
   `examples/webgpu_postprocessing_masking.rs`. Gate: the pass trace of §5.3, then the pixel diff.
7. Full ladder re-run, plus `docs/postprocessing.md` gaining a "single-pass effect nodes" section.

---

## 7. What this rung unlocks

* `NeutralToneMapping` is needed by **`difference`, `direct`, `anamorphic`, `bloom_selective`,
  `dof_basic`** — five more members, all 0.0%.
* `to_const` and the node-bounded `Loop` are used by essentially every addon `Fn` in
  `examples/jsm/tsl/display/` (`grep -l 'toConst\|Loop(' examples/jsm/tsl/display/*.js` hits 20 of
  the 47 files), including `fxaa`, `boxBlur`, `gaussianBlur`, `bilateralBlur`, `sobel`,
  `chromaticAberration` and `BloomNode`.
* Instance colour unlocks the scenes of **`ssaa`** and **`transition`**, and removes a dead field.
* After this rung, `webgpu_postprocessing_fxaa` and `_sobel` are "swap the `Fn`" — but both grade
  0.1%, on the boundary, so they are not worth taking (SURVEY §5).

## 8. Verdict

**Rung-sized: one Opus worker, one sitting.** ~430 lines of new Rust, none of it architectural —
three small node-system additions (`to_const`, node-bounded `Loop`, a tone-mapping function), one
renderer addition (instance colour) and a 68-line effect that is fully specified by a dumped
60-statement shader. Two render passes, two draws. The rung-9 machinery carries the rest.

Top three risks, in order:

1. **`to_const` perturbs every other rung's WGSL.** It adds a counter and a statement kind to the
   shared emitter. Mitigated by step 2's gate (re-diff all `dump_wgsl` entries and re-run the
   ladder before anything else is built on it).
2. **Instance colour touches the shared vertex-buffer path**, which every mesh example uses. Rung
   2 (`webgpu_instance_mesh`) is the canary; its score already moved once at rung 9.
3. **The 700-draw random sequence plus the centre guard.** Any off-by-one puts 100 tetrahedra in
   the wrong places, which the radial blur's origin-centred streaks make maximally visible. The
   §5.4 oracle is the cheap defence; dump it before writing the scene.

No dependency on rungs 10 (skinning), 11 (BatchedMesh) or 12 (compute/storage/Points).
