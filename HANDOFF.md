# three-rs — handoff

Written 2026-09-12 from the throne session. Port Three.js core and
WebGPURenderer to Rust on wgpu, verified by pixel-diffing Three's own
examples. This is the plan that a claude.ai conversation of 2026-08-27
("AI-powered drones in futuristic warfare games", which turned into a
compositor design) called the single highest-leverage agent target, and
that throne's requirements walk (item 16) names as the later, separate
project throne is the playground for.

## The claim being tested

Three.js core is ~200k lines, dependency-free, with a clean object model,
and it already has a WebGPU backend, so the mapping onto wgpu is near
one-to-one. Validation needs no display: render Three's examples headless
and diff against the reference PNGs. The axiom underneath: an agent loop
can rewrite a codebase of this size in days if every step has a pixel
check. The chat's own words: steps that validate headless are "exactly
where agent loops shine".

## Measured on this machine (2026-09-12)

- `~/src/vendor/three.js` at 3d010ef (2026-08-31). `src/` is 752 files,
  183,419 lines. Of that, `src/renderers/webgpu` + `src/renderers/common`
  is 34,368 lines: the renderer half of the port. `src/renderers/webgl`
  and `webgl-fallback` are out of scope.
- 602 example pages in `examples/`. `test/e2e/` already has the harness:
  puppeteer.js, deterministic-injection.js, image.js, and the reference
  screenshots. That is the grader; reuse it rather than writing one.
- `~/src/vendor/wgpu` at v30.0.0-223-gb82deac64 (2026-08-16), the same
  vendored path 3os/glass builds against.
- `~/src/vendor/smithay` at v0.7.0-428 (2026-07-30). Not needed for
  three-rs itself; it is step 2 of the larger plan.

## Prior work to read, in order

1. `~/src/projects/3os/glass/docs/FINDINGS.md` (2,128 lines). Measured
   facts about three.js r182–r185 on wgpu→Vulkan, Intel Iris Xe, Mesa
   25.3.6. Its headline: the dominant failure mode is silent wrong output,
   not exceptions. Three separate spikes rendered correctly-looking wrong
   results with zero errors thrown. Verify with identity assertions, not
   counts. This applies directly to a pixel-diff harness.
2. `~/src/projects/3os/glass/docs/BRIEF.md` and `DESIGN.md`. The vision
   three-rs serves and the settled decisions (one wgpu Vulkan device,
   zero GL in the process tree, `scripts/check-zero-gl.sh` as the gate).
3. `~/src/projects/3os/glass/gpu` and `glass/renderer`: the existing Rust
   crates that adopt wgpu textures into a Deno-hosted three.js. They are
   the JS-side of the bridge this port removes.
4. `~/src/projects/3os/PRIOR-ART.md`: who else has done which layer, and
   the two gaps (process isolation, a typed verb surface) that shape what
   the scene API has to look like from the outside.
5. `~/src/projects/lib3` (TSL nodes, SDF text) and `~/src/projects/crush`
   (Ghostty WASM terminal, ADR-001): the first real consumers. If their
   materials and BatchedText cannot be expressed in three-rs, the port is
   not done.

## The ordered plan from the 27 Aug chat

1. Port core + WebGPURenderer to wgpu; pixel-diff the examples. Headless.
2. Wire Smithay dmabuf import: one textured quad per surface, with damage
   and frame callbacks. "Where the humans earn their keep."
3. Blitz + Stylo + Taffy + Parley + Vello rendering HTML/CSS to a wgpu
   texture, for panels and HUD. Headless.
4. Only then port GNOME Shell / Mutter behaviour, using them as the spec.

Only step 1 is this repo. Steps 2–4 belong to the compositor project that
throne's walk deferred.

## Things the chat left open, still open

- Godot as the scene layer versus Rust all the way. Both left live; this
  repo is the Rust-all-the-way bet, and step 1 is cheap enough to settle it.
- Servo or Chromium as a texture for real web pages: an escape hatch, not a
  decision.
- Interaction design. Not this repo's problem; that is throne.

## First moves

- Decide the unit of porting: per file, per subsystem (math, core, objects,
  materials, nodes/TSL, renderer backend), or per example (make
  `webgpu_cubes` pass, then the next). Per example gives a pixel check at
  every step and is the shape the axiom needs.
- Walk that decision and the crate layout with the approval-walk skill
  before writing code. CLAUDE.md here is copied from throne's walked one
  with the name changed; it has not been walked for this repo.
