# Re-pin to r186 (2026-09-12)

Vendor tree `~/src/vendor/three.js` moved from `3d010ef` (r186dev, `three@0.185.0`)
to the **r186** tag: commit **`148ef33`** (annotated tag object `819fadd`),
`three@0.186.0`. Tree now at `148ef33` with `grader-flags.patch` applied and
nothing else modified (`git status` = `M test/e2e/puppeteer.js`).

`grader-flags.patch` **applied unchanged** (`git apply --check` clean); no
hand-port, no regeneration needed. `npm ci` re-run because `package-lock.json`
moved (log: scratch only, exit 0, 224 packages, 0 vulnerabilities).

## What changed in the grader (3d010ef → 148ef33)

`git diff 3d010ef 148ef33 --stat -- test/e2e examples/screenshots package.json`:

- **`test/e2e/image.js` — unchanged.** The comparator, the 0.1% pass rate, the
  threshold-0.1 RGB-distance rule and `scale()` are all byte-identical. The
  Rust harness's copy of the rules stands.
- **`test/e2e/deterministic-injection.js` — rewritten in two places.**
  (a) It now guards on `globalThis._e2eInjected` and installs `Math.random` /
  the frozen clock on `globalThis` rather than `window`, and returns early when
  there is no `window` — so it also runs inside Workers. Same seed
  (`PI/4`), same `sin(seed++) * 10000` recurrence, same `now() === 0`, same
  single-RAF behaviour. Nothing for the port to change.
  (b) Video determinism was rewritten (playbackRate 0, `requestVideoFrameCallback`,
  a new `window._videosReady()` that `puppeteer.js` now waits on). Irrelevant to
  the ladder — no ladder example has a video.
- **`test/e2e/puppeteer.js`**: the deterministic injection is now *prepended to
  the three build files* as well (`buildInjection`), because Workers get no
  `evaluateOnNewDocument`; `Input.setIgnoreInputEvents` is sent over CDP so
  ambient input cannot perturb a page; a `pageerror` handler was added; a
  `_videosReady()` wait was inserted into the pipeline; one `catch` comparison
  tightened. Exception list: `webgl_lightprobes_sponza`, `webgpu_lightprobes_sponza`
  added, `webgpu_generator_city` kept (moved, same comment). **No ladder example
  is on the list.**
- **Dependencies**: `puppeteer` stays `^25.0.0` but the lock moved
  `puppeteer-core` 25.9.0 → 25.10.0, which pulled Chrome **152.0.7977.75**
  (rung 0 ran on 152.0.7977.54). `pixelmatch` is not used by this grader at all
  (`image.js` has its own `compare()`); no imaging dependency changed.
  `package.json` also gained `allowScripts` and a `lint-tsl` script.
- **`examples/screenshots/`**: 14 files touched — `webgl_animation_skinning_ik`,
  `webgl_lightprobes_sponza`, `webgl_sculpt` (new), `webgl_worker_offscreencanvas`,
  `webgpu_custom_fog`, `webgpu_custom_fog_scattering`, `webgpu_generator_building`,
  `webgpu_generator_city`, `webgpu_lightprobes_sponza`, `webgpu_lights_sunlight` (new),
  `webgpu_materials_retroreflection`, `webgpu_sculpt` (new), `webgpu_video_panorama`,
  `webxr_xr_sculpt` (new). **None of them is a ladder example** — every reference
  image rungs 1–13 are graded against is byte-identical to the one rung 0 used.
- The nine ladder example HTMLs (rungs 1–9) are **unchanged** between the two
  commits. `src/` moved in 28 files (new `Packed4x8IntegerNode`, `IndexNode`,
  `LoopNode`, `RTTNode`, `EnvironmentNode`, and in `WGSLNodeBuilder` an
  `isAtomic` scoped-array flag plus integer-component `texture_2d_array<T>` /
  `texture_3d<T>` types); none of it changes the WGSL any ladder example emits
  (see §WGSL below).

## Rung 0 recalibration at r186

`npm run test-e2e-webgpu -- <names>` with the patched flags, full log in
`handoff/rung0/e2e-run8-r186.log`. Three graded against its own references:

| example | rung | r186 | was at 3d010ef |
|---|---|---|---|
| webgpu_depth_texture | 1 | 0.0% | 0.0% |
| webgpu_instance_mesh | 2 | 0.1% (pass) | 0.1% (pass) |
| webgpu_materials_basic | 3 | 0.0% | 0.0% |
| webgpu_rtt | 4 | 0.0% | 0.0% |
| webgpu_lights_phong | 5 | 0.0% | 0.0% |
| webgpu_morphtargets | 6 | 0.0% | 0.0% |
| webgpu_shadowmap | 7 | 0.0% | 0.0% |
| webgpu_lights_physical | 8 | 0.0% | 0.0% |
| webgpu_postprocessing_masking | 9 | 0.0% | 0.0% |
| webgpu_textures_2d-array | (spare) | 0.0% | 0.0% |

`TEST PASSED! 10 screenshots rendered correctly.` **No example regressed; every
one of them is still at its rung-0 number.** `webgpu_instance_mesh` keeps its
thin 0.1% margin (unchanged).

## three-rs e2e against the new references

`/home/tom/src/projects/three-rs/port` @ `fc5d3e7`, tree untouched,
`cargo test -p three-rs --test e2e -- --nocapture --test-threads=1`, Intel Iris
Xe / Mesa 25.3.6 / Vulkan:

| example | pixels different (of 100000, 400×250) | verdict | was |
|---|---|---|---|
| webgpu_depth_texture | 0 | ok | 0 |
| webgpu_instance_mesh | 45 (0.045%) | ok | 45 |
| webgpu_materials_basic | 0 | ok | 0 |
| webgpu_rtt | 1 (0.001%) | ok | 1 |

`4 passed; 0 failed`. Numbers are **identical** to the pre-re-pin run, which is
expected: none of the four reference JPEGs changed. No image inspection was
needed (nothing regressed). Rung 5 (`webgpu_lights_phong`) is still unmerged on
branch `rung5`, so it is not in this suite.

## Scout WGSL re-dump (rungs 6–9)

Re-dumped from the real page with a temporary `test/e2e/_dump_repin.mjs`
(same recipe as the scouts: `--use-angle=vulkan`, no `--disable-vulkan-surface`,
`deterministic-injection.js` + `clean-page.js` + the `buildInjection` rewrites,
networkidle → single RAF, hooks on `GPUDevice.createShaderModule` /
`createBindGroupLayout` / `createRenderPipeline*` / `createTexture` /
`createSampler` and `beginRenderPass`). Script and its profile deleted
afterwards; vendor tree back to `M test/e2e/puppeteer.js`.

New files, next to the old ones, with a `-r186` suffix
(`handoff/scouts/rung{6,7,8,9}/`): every module under the old naming
(e.g. `m03_phong_pillars.frag-r186.wgsl`, `MeshStandardMaterial_18.vert-r186.wgsl`,
`output_quad.frag-r186.wgsl`), plus `dump-r186.json` (modules, pipelines,
bind-group layouts, textures, samplers, passes with draw order) and
`actual_full-r186.png` (Three's own 800×500 frame).

Counts are unchanged in all four: rung 6 4 modules / 2 pipelines / 3 BGLs;
rung 7 16 / 8 / 3, 4 passes; rung 8 13 / 8 / 5, 13 textures; rung 9 5 / 3 / 2,
16 passes, 9 textures.

**WGSL diff: one cosmetic difference across all 38 modules.**

- Every module's banner changes `// Three.js r186dev - Node System` →
  `// Three.js r186 - Node System`. Ignoring that line, **37 of 38 modules are
  byte-identical** to the 3d010ef dumps (rung 6 frag + both output modules,
  all 16 rung-7 modules, all 13 rung-8 modules, all 5 rung-9 modules).
- The only other change, rung 6 `MeshPhongNodeMaterial.vert`: the generated
  morph-influence uniform-buffer struct is renamed
  `NodeBuffer_1015` → `NodeBuffer_1029` (Three's global node-id counter moved).
  Two lines, the declaration and its one use. No structural, binding or
  arithmetic change.
- Pipeline descriptors and bind-group layouts were compared field by field
  against the scouts' JSON: **identical** (labels, vertex buffer strides and
  locations, target formats, blend state, depth state, multisample counts, every
  BGL entry's binding/visibility/type). Rung 8's 13 `createTexture` descriptors
  also match label-for-label.

So every rung-6..9 fact in the scout PLANs still holds at r186; nothing in them
needs rewriting, and the `-r186` dumps are there only as the fresh baseline.

## State left behind

- `~/src/vendor/three.js` at `148ef33` (r186) + `grader-flags.patch`, `npm ci` done.
- Nothing committed, here or in the port tree.
- Still to do by the director: bump the `3d010ef` references in
  `handoff/HANDOFF.md` (§Sources) and in d33, and commit
  `e2e-run8-r186.log`, this file and the `-r186` scout dumps.
