# three-rs — handoff

Port enough of Three.js core + WebGPURenderer to Rust on wgpu that the
examples in the ladder below render headless and pass Three's own e2e
image comparison against Three's reference screenshots.

## Sources

- `~/src/vendor/three.js` @ 3d010ef. Port from `src/`; `renderers/webgl*`
  is out of scope. Addons (`examples/jsm/`) only as a rung needs them.
- `~/src/vendor/wgpu` @ v30.0.0-223. Vulkan backend, Intel Iris Xe, Mesa.

## The grader

`~/src/vendor/three.js/test/e2e/` — `puppeteer.js`, `image.js`,
`deterministic-injection.js`, references in `examples/screenshots/*.jpg`.
Facts the Rust harness must reproduce exactly:

- Viewport 800×500 (400×250 at viewScale 2), screenshot downscaled ×½
  with `image.js`'s `scale()` (box filter; port it or call it via node).
- One frame. `performance.now`, `Date.now` return 0; RAF fires once.
- `Math.random` is `x = sin(seed++) * 10000; x - floor(x)`, seed starts
  at `PI/4`. Replicate bit-for-bit (f64) wherever an example uses it.
- `window.TESTING = true` (a few examples branch on it).
- Pass = pixelmatch threshold 0.1 AND fewer than 0.1% of pixels differ.
  Use `image.js`'s `compare()` unchanged; do not write a new comparator
  and do not loosen the tolerance.

## Rules that keep the green honest

- The example scene is ported from the example's JS, calling the three-rs
  API. Only API the current rung needs gets added.
- Nothing derived from the reference image may appear in the tree: no
  reference bytes, no hard-coded pixels, no per-example colour fudges.
- The renderer must go through the same route for every example: scene
  graph → node materials → WGSL → wgpu. Hand-written WGSL is allowed as
  scaffolding through rung 2 only; from rung 4 on, shaders come from the
  ported node system. (Under WebGPURenderer every material, even
  MeshBasicMaterial, is a NodeMaterial. There is no non-TSL path.)
- Known trap on this stack: failures are silent wrong output, not
  errors (`~/src/projects/3os/glass/docs/FINDINGS.md`). Trust the diff
  image, not draw counts or "no panic".
- A rung is done when the diff passes and the director has looked at
  actual/expected/diff images.

## Rung 0 — calibrate the grader on this machine

Done 2026-09-12, see `rung0/RUNG0.md`. The stock grader renders black on
this machine (lavapipe override + `--disable-vulkan-surface`); apply
`rung0/grader-flags.patch` to the vendor checkout. The grader then runs on
the Intel adapter, and the ladder below is what survived it.

## The ladder

| # | example | forces into three-rs |
|---|---|---|
| 1 | webgpu_depth_texture | scene graph, PerspectiveCamera, Mesh, TorusKnotGeometry, MeshBasicNodeMaterial (overrideMaterial), scene.background, deterministic Math.random, RenderTarget + DepthTexture, QuadMesh, texture() node. (webgpu_camera dropped at rung 0: 0.9% point/line coverage diff with Three itself.) |
| 2 | webgpu_instance_mesh | InstancedMesh, per-instance matrix + colour, BufferGeometryLoader (JSON) |
| 3 | webgpu_materials_basic | TextureLoader (PNG), CubeTexture, envMap reflection/refraction, scene.background |
| 4 | webgpu_rtt | first real TSL: texture(), uniform(), colorNode; RenderTarget, QuadMesh |
| 5 | webgpu_lights_phong | PointLight, MeshPhongNodeMaterial, normalMap node, specularNode, TeapotGeometry addon |
| 6 | webgpu_morphtargets | morph attributes, AmbientLight |
| 7 | webgpu_shadowmap | spot + directional shadow maps, Fog, ACES tone mapping, custom Fn() on shadow/colour |
| 8 | webgpu_lights_physical or webgpu_materials | MeshStandard/Physical PBR — director reads both and picks |
| 9 | webgpu_postprocessing_* (director picks; plain webgpu_postprocessing dropped at rung 0, sits on the 0.1% line) | RenderPipeline, pass(), display nodes |
| 10 | webgpu_skinning | GLTFLoader addon, SkinnedMesh, AnimationMixer at t=0 |
| 11 | webgpu_mesh_batch | BatchedMesh (what crush's BatchedText sits on) |
| 12 | webgpu_compute_points | compute via TSL, storage buffers |
| 13 | webgpu_tsl_galaxy | pure-TSL material, SpriteNodeMaterial (the lib3 shape) |

None are in the e2e exception list. Rungs 1–7 need nothing outside `src/`
except TeapotGeometry. Order is a default, not a contract: the director
reorders when a rung's gap list says so.

## Orchestration

- Director: the session model, holding this file, the ladder, and the
  judgement calls (is the green real, what is the next rung, when to
  reorder, when hand-written scaffolding has to be replaced).
- Workers: Opus, one per rung. A worker gets: the example's HTML, the
  current crate, the harness, and "make this rung pass". It reports:
  pass/fail with the diff %, the API it added, and the gaps it found.
- Rungs are sequential (one crate grows). Fan out only inside a rung
  when a worker's gap list has independent items (e.g. PNG decode and
  SphereGeometry): 2–3 Opus workers at once, never more than 6.
- Three's unit tests (`test/unit`, QUnit) are a cheap side gate for
  math/core; port them opportunistically, never instead of a rung.

## Layout (suggested, walk it if it fights you)

One crate `three-rs`. `examples/<name>.rs` is the ported example scene,
one per rung. `tests/e2e/` renders each to PNG, downscales, and compares
against `~/src/vendor/three.js/examples/screenshots/<name>.jpg`, writing
actual/expected/diff on failure. Repo workflow: CLAUDE.md.

## Out of scope here

Smithay/compositor work, HTML panels, GNOME behaviour, Godot-vs-Rust,
interaction design. Those live in throne and the deferred compositor
project. The lib3/crush "can their materials be expressed" check is a
later round, after rung 13.
