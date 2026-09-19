# Scout survey — the `webgpu_postprocessing_*` family

Scouted 2026-09-19 against vendor three.js r186 (`rung0/grader-flags.patch` applied), on this
machine (Intel Iris Xe, Mesa 25.3.6, Fedora 43). Port read at
`/home/tom/src/projects/three-rs/main` (rung 9 landed: `src/renderer/pass.rs`,
`src/renderer/render_pipeline.rs`, `docs/postprocessing.md`).

33 members. 7 are on Three's own `exceptionList` in `test/e2e/puppeteer.js`; of the remaining 26,
**20 grade 0.0% on this machine**, 2 sit on the 0.1% line, 4 fail outright.

Grader logs beside this file:
`e2e-runs-2026-09-19.log` (today, three runs) and `e2e-run7-rung9-candidates-2026-09-13.log`
(rung 0's 13th-September grades, reused for the twelve it already covered).

---

## 0. The one fact that shapes every plan in this family

`test/e2e/deterministic-injection.js` pins `performance.now()`/`Date.now()` to **0** and replaces
`requestAnimationFrame` with a one-shot that fires **exactly once**. So for every example here the
graded image is **frame 0 at t = 0**: no `Timer` delta, no `TWEEN` advance, no `OrbitControls`
damping motion, no `AnimationMixer` step. `Math.random` is the seeded
`x = sin(seed++) * 10000; x - floor(x)` sequence that `Renderer::skip_random_draws` already models.

This has a concrete consequence worth knowing before anyone picks a rung:
**`webgpu_postprocessing_transition` never runs its post-processing at all.** Its `render()` is

```js
if ( effectController.transition === 0 ) renderer.render( fxSceneB.scene, fxSceneB.camera );
```

and the TWEEN that would move `transition` off 0 has a 2000 ms delay against a frozen clock. Its
0.0% grade is a plain forward render of one instanced Phong scene — a fine cheap rung, but **not**
a post-processing rung, and `TransitionNode.js` would be dead code. Do not pick it for this family.

---

## 1. Family roster: exception-list status and grade

`E` = on Three's `exceptionList` (Three itself cannot grade it here). Grades from the logs above;
"run7" = 2026-09-13, "A/B/C" = today's three runs. Green (`Diff x%`) is a pass; `Error: Diff wrong
in x%` is a fail, which is why 0.1% appears in both columns — the grader rounds at the boundary.

| example | exception | grade(s) | verdict |
|---|---|---|---|
| `webgpu_postprocessing` | — | 0.1% **fail** (rung 0, runs 3 & 4) | rung 0 already dropped it |
| `webgpu_postprocessing_3dlut` | — | 0.0% (run7) | gradeable |
| `webgpu_postprocessing_afterimage` | — | 0.5% **fail** (run7) | not a rung |
| `webgpu_postprocessing_anamorphic` | — | 0.0% (run7) | gradeable |
| `webgpu_postprocessing_ao` | **E** (black screen) | — | not a rung |
| `webgpu_postprocessing_bloom` | — | 0.0% (run7) | gradeable |
| `webgpu_postprocessing_bloom_emissive` | — | 0.0% (A) | gradeable |
| **`webgpu_postprocessing_bloom_selective`** | — | **0.0% (A, C)** | **pick 3** |
| `webgpu_postprocessing_ca` | — | 0.0% (run7) | gradeable |
| `webgpu_postprocessing_difference` | — | 0.0% (A, C) | gradeable |
| `webgpu_postprocessing_direct` | — | 0.0% (run7, C) | gradeable |
| `webgpu_postprocessing_dof` | **E** (black screen) | — | not a rung |
| `webgpu_postprocessing_dof_basic` | — | 0.0% (A) | gradeable |
| `webgpu_postprocessing_fog` | — | 0.0% (A) | gradeable |
| `webgpu_postprocessing_fxaa` | — | 0.1% pass (run7, C) | **on the line — avoid** |
| `webgpu_postprocessing_godrays` | — | 0.0% (A) | gradeable |
| `webgpu_postprocessing_lensflare` | — | 0.0% (A) | gradeable |
| `webgpu_postprocessing_masking` | — | 0.0% (run7) | **DONE — rung 9** |
| `webgpu_postprocessing_motion_blur` | — | 0.0% (B) | gradeable |
| `webgpu_postprocessing_outline` | — | 0.0% (B) | gradeable |
| `webgpu_postprocessing_pixel` | — | 0.4% **fail** (run7) | not a rung |
| **`webgpu_postprocessing_radial_blur`** | — | **0.0% (B, C)** | **pick 1** |
| `webgpu_postprocessing_retro` | — | 1.5% **fail** (run7) | not a rung |
| `webgpu_postprocessing_smaa` | — | 0.3% **fail** (B) | not a rung |
| `webgpu_postprocessing_sobel` | — | 0.1% pass (run7, C) | **on the line — avoid** |
| **`webgpu_postprocessing_ssaa`** | — | **0.0% (B, C)** | **pick 2** |
| `webgpu_postprocessing_ssgi` | **E** (black screen) | — | not a rung |
| `webgpu_postprocessing_ssgi_ballpool` | **E** (black screen) | — | not a rung |
| `webgpu_postprocessing_ssr` | — | 0.1% **fail** (B) | not a rung |
| `webgpu_postprocessing_ssr_denoise` | **E** (needs more time) | — | not a rung |
| `webgpu_postprocessing_sss` | **E** (black screen) | — | not a rung |
| `webgpu_postprocessing_traa` | **E** (black screen) | — | not a rung |
| `webgpu_postprocessing_transition` | — | 0.0% (run7) | 0.0% but **runs no post-processing** (§0) |

`smaa` at 0.3% is the one genuinely surprising result: it is the cleanest *scene* in the family
(two boxes, one wireframe, one textured, no lights) but Three's own SMAA output does not reproduce
against its reference here. That kills what would otherwise have been the obvious cheap
multi-pass rung, and saves the ladder ~700 lines of `SMAANode.js`.

---

## 2. What each gradeable example imports and needs

Read from the HTML and the addon source. "MRT" = `mrt()` / multiple colour attachments on the
scene pass; "depth" = `pass.getTextureNode('depth')` / `getViewZNode()`; "history" =
`getPreviousTextureNode()`; "own RTs" = the effect node owns render targets and drives its own
`QuadMesh` passes in `updateBefore()`.

| example | display/pass nodes (`three/addons/tsl/display/*`) | notable `three/tsl` | MRT | depth | history | own RTs | other heavy deps |
|---|---|---|---|---|---|---|---|
| `direct` | *(none — `DirectRenderPipeline`)* | `output, saturation, uniform, vec4` | – | – | – | – | `DirectRenderPipeline`, NeutralToneMapping |
| `difference` | *(none)* | `pass, luminance, saturation` | – | – | **yes** | – | GIF decode, `Fog`, NeutralToneMapping |
| **`radial_blur`** | `radialBlur` (68) | `float, int, pass, uniform` | – | – | – | – | NeutralToneMapping, instance colour |
| `fxaa` | `fxaa` (364) | `pass, renderOutput` | – | – | – | – | – |
| `sobel` | `sobel` (168) | `pass, renderOutput` | – | – | – | – | PMREM + `RoomEnvironment`, glTF |
| `ca` | `chromaticAberration` (174) | `pass, renderOutput, uniform` | – | – | – | – | PMREM + `RoomEnvironment`, `Points`, `GridHelper` |
| `3dlut` | `lut3D` (109) | `texture3D, rotateUV, Fn, time, …` | – | – | – | – | `Texture3D`, LUTCube/3dl/Image loaders, glTF |
| **`ssaa`** | `ssaaPass` (344) | *(none directly)* | – | – | – | **yes** | additive-blend quad, `camera.setViewOffset` |
| **`bloom_selective`** | `bloom` (599) | `pass, mrt, output, float, uniform` | **yes** | – | – | **yes** | 11 RTs over 5 mip levels, `uniformArray` |
| `bloom` | `bloom` (599) | `pass` | – | – | – | **yes** | glTF + `AnimationMixer`, Reinhard, pass `config` |
| `bloom_emissive` | `bloom` (599) | `pass, mrt, output, emissive, vec4` | **yes** | – | – | **yes** | HDRLoader, glTF (DamagedHelmet), ACES |
| `anamorphic` | `bloom` (599) | `rtt, Loop, screenUV, viewportSize, instanceIndex, luminance` | – | – | – | **yes** | `rtt()`, compute-ish instancing, NeutralToneMapping |
| `lensflare` | `bloom`, `lensflare` (282), `gaussianBlur` (412) | `pass, mrt, output, emissive, uniform` | **yes** | – | – | **yes** | UltraHDRLoader, glTF, ACES |
| `outline` | `outline` (814) | `pass, uniform, time, oscSine` | – | – | – | **yes** | OBJLoader, `MeshLambertMaterial`, `SunLight(Node)` addon |
| `godrays` | `godrays` (618), `bilateralBlur` (377), `depthAwareBlend` (80) | `pass, uniform, float, int, color` | – | **yes** | – | **yes** | glTF |
| `fog` | `gaussianBlur` (412) | large `three/tsl` list | – | **yes** | – | **yes** | PLYLoader (Lucy100k), `SunLight(Node)` addon, ACES |
| `dof_basic` | `boxBlur` (65), `fxaa` (364) | `mix, pass, renderOutput, smoothstep, uniform, vec3` | – | **yes** (`getViewZNode`) | – | **yes** | DRACO + glTF, UltraHDRLoader, NeutralToneMapping |
| `motion_blur` | `motionBlur` (33) | `pass, mrt, output, velocity, screenUV, texture, uv` | **yes** | – | – | – | `velocity` node, glTF, `SunLight(Node)` addon |
| `transition` | `transition` (141) | `uniform, pass` | – | – | – | – | **never executes** (§0) |

---

## 3. Dependency table: node → who needs it → ported?

`src/` paths are in `/home/tom/src/projects/three-rs/main`.

### 3.1 Shared infrastructure (the real cost)

| piece | needed by | in the port? |
|---|---|---|
| `PassNode` + `RenderPipeline` + `QuadMesh` | everything with `pass()` | **have** — `src/renderer/pass.rs`, `render_pipeline.rs`, `src/objects/quad_mesh.rs`; `Renderer::render_quad` (`src/renderer/mod.rs:1278`) already lets a node draw its own quad into an RT |
| `NeutralToneMapping` | radial_blur, difference, direct, anamorphic, bloom_selective, dof_basic | **missing** — `ToneMapping` enum (`src/materials/mod.rs:80`) is `None`/`Reinhard`/`AcesFilmic` only. ~35 lines; the exact WGSL is dumped (see §4) |
| `LinearToneMapping` | sobel | missing (same enum, ~5 lines) |
| node-bounded `Loop` (`i < i32(uniform)`) + `Break` | radialBlur, fxaa, smaa, ssr, bloom (fixed bound only) | **partly** — `loop_statement(count: usize, …)` and `loop_n` take a Rust `usize`; a node bound is missing |
| `.toConst()` (a WGSL `let`, `nodeConstN`) | radialBlur, bloom comp, most addon `Fn`s | **missing** — `to_var` exists, `to_const` does not; small |
| `mrt()` / multiple colour attachments / `material.mrtNode` / `pass.setMRT` | bloom_selective, bloom_emissive, lensflare, motion_blur (+ excluded ao/ssgi/ssr/sss/traa) | **missing** — renderer, render target and `OutputType` struct all single-attachment today |
| effect node owning RTs + several quad materials | bloom, ssaa, gaussianBlur, godrays, outline, dof, anamorphic, lensflare, fog | **missing as a pattern**, but every primitive it needs is present (`RenderTarget::set_size`, `set_render_target`, `render_quad`) |
| additive blending on a quad + `premultipliedAlpha` + `depthCompare: always` | ssaa | **partly** — `src/materials/blending.rs` exists; check it covers `AdditiveBlending` |
| `InstancedMesh` per-instance colour (`setColorAt` → `nodeAttribute1` + `vInstanceColor`) | radial_blur, ssaa, transition | **missing** — `instance_color` is a field on `src/objects/instanced_mesh.rs:39` but nothing consumes it |
| `camera.setViewOffset` / `clearViewOffset` | ssaa | missing (`src/cameras/`) — ~30 lines in `makePerspective` |
| `uniformArray` (`array<vec4<f32>, 5>` in a uniform block) | bloom composite | missing |
| `copyTextureToTexture` (depth) | ssaa | missing; not observable in this example's image |
| pass `getPreviousTextureNode()` (history buffer) | difference, afterimage, traa | missing |
| pass depth / `getViewZNode` / `getLinearDepthNode` | fog, godrays, dof_basic | missing (the depth texture itself already exists per pass) |
| `velocity` node | motion_blur | missing (needs previous model-view matrices) |
| PMREM + `RoomEnvironment` | ca, sobel, ssr | missing, large |
| `Texture3D` + LUT loaders | 3dlut | missing |
| GIF decode | difference | missing (`Cargo.toml` has `png 0.17` + `zune-jpeg`, no gif) |
| `SunLight` / `SunLightNode` addon | fog, motion_blur, outline, pixel | missing |
| `AnimationMixer` + glTF animation | bloom | missing |
| `MeshLambertMaterial` | outline | missing (`MaterialKind` has Basic/Phong/Sprite/Standard) |

### 3.2 Already in the port and reusable as-is

`MeshStandardNodeMaterial` (`src/materials/physical.rs`, `dfg_lut.rs` — the dumps show the same
16×16 `rg16float` `DFG_LUT`), `MeshBasicNodeMaterial`, `MeshPhongNodeMaterial`, `flat_shading`,
`HemisphereLight`/`PointLight`/`AmbientLight`/`DirectionalLight`
(`src/lights/light_object.rs`), `tetrahedron_geometry`/`icosahedron_geometry`/`sphere_geometry`,
`InstancedMesh` matrices, `interleaved_gradient_noise` (`src/nodes/tsl.rs:2399`), `luminance`,
`saturation`, `mix`, `premultiply_alpha`/`unpremultiply_alpha`, `srgb_transfer_oetf`,
`render_output`, `Renderer::skip_random_draws`.

---

## 4. The picks, and why

Scored as *missing pieces per example unlocked*. Full plans in
`scouts/webgpu_postprocessing_<name>/PLAN.md`, each with its own dump.

1. **`webgpu_postprocessing_radial_blur`** — the whole effect is **one fragment shader**
   (`dump/m03_fragment_fragment_RenderPipeline.wgsl`, 60 statements) appended to the rung-9
   quad. Two render passes total. New surface: `NeutralToneMapping`, `to_const`, a node-bounded
   `Loop`, instance colour. Everything else is rung 1/5/9 material. Smallest possible step that
   is still genuinely a post-processing rung, and `NeutralToneMapping` + `to_const` +
   node-bounded `Loop` are prerequisites for six other members.

2. **`webgpu_postprocessing_ssaa`** — the cheapest example in the family that establishes the
   **"effect node owns render targets and drives quad passes"** pattern, and it does it as a
   `PassNode` subclass so rung 9's code is reused rather than duplicated. 26 passes, 3 pipelines,
   and its quad shader is 40 lines. New surface: additive-blend quad material,
   `camera.setViewOffset`, a second render target cloned from the pass's own. That pattern is the
   gate to bloom, gaussianBlur, godrays, outline, dof and anamorphic — nine examples.

3. **`webgpu_postprocessing_bloom_selective`** — the keystone. Its *scene* is 50
   `IcosahedronGeometry` spheres with `MeshBasicNodeMaterial` and **no lights at all**, so
   essentially all the work is the two things the family is blocked on: **MRT** (2 colour
   attachments, `material.mrtNode` and `pass.setMRT`) and **`BloomNode`** (11 render targets over
   5 mip levels, 7 quad materials, 12 quad passes). Unlocks `bloom`, `bloom_emissive`,
   `anamorphic`, `lensflare` (bloom) and `motion_blur` (MRT), and is the prerequisite for the
   currently-excluded ao/ssgi/ssr/sss/traa family should Three ever un-exclude them.
   **Honest estimate: two rungs**, split at MRT (see its PLAN.md §6).

---

## 5. Verdict — the order to take the family

Each step leaves the ladder green on its own.

| rung | example | why here | est. Rust |
|---|---|---|---|
| **next** | `radial_blur` | NeutralToneMapping + `to_const` + node-bounded `Loop` + instance colour, all inside one quad shader | ~450 |
| **+1** | `ssaa` | RT-owning effect node pattern, additive quad, view offset | ~550 |
| **+2** | `bloom_selective` (part A: MRT) | `mrt()`, `material.mrtNode`, `pass.setMRT`, 2-attachment `OutputType` | ~450 |
| **+3** | `bloom_selective` (part B: BloomNode) | the 5-level blur chain; lands the example green | ~600 |
| then, cheap | `bloom_emissive` | = MRT + bloom + ACES + HDRLoader + glTF | ~350 |
| then, cheap | `motion_blur` | = MRT + `velocity` + the `SunLight` addon | ~500 |
| then | `difference` | `getPreviousTextureNode` + GIF decode + `Fog` | ~300 |
| then | `direct` | `DirectRenderPipeline` — orthogonal to everything above | ~400 |
| later | `godrays`, `fog`, `dof_basic` | need pass depth / `getViewZNode` first | — |
| later | `outline`, `anamorphic`, `lensflare` | need the `SunLight` addon / `rtt()` / three effect nodes at once | — |
| **never (as rungs)** | `afterimage`, `pixel`, `retro`, `smaa`, `ssr`, plain `webgpu_postprocessing` | Three itself does not reproduce them here | — |
| **never** | `ao`, `dof`, `ssgi`, `ssgi_ballpool`, `sss`, `traa`, `ssr_denoise` | on Three's `exceptionList` | — |
| **not this family** | `transition` | 0.0% but never runs the pipeline (§0) | — |
| **avoid** | `fxaa`, `sobel` | 0.1%, on the pass/fail boundary; the port would have no headroom | — |
