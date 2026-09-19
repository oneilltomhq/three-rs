# webgpu_postprocessing_ssaa

Branch `rung-ssaa` from `rung-radial-blur` 4f9b2bc. **PASS — 0 of 100000
pixels differ (0.0%), limit 0.1%.**

The scout plan (`scouts/scouts/webgpu_postprocessing_ssaa/PLAN.md`) and the
family survey were written today against current main, so there are no
pre-0.2.0 deltas to reconcile.

## What was added

| area | what |
|---|---|
| `src/renderer/ssaa_pass.rs` | new module: `SsaaPassNode`, the six `_JitterVectors` tables, the unbiased sample weights, and the eight-render schedule |
| `src/renderer/mod.rs` | `Renderer::auto_clear`, `clear( color, depth )`, `set_clear_color()`, `clear_color()`, `clear_alpha()`; the by-use cache clock turned from renders into frames (`begin_frame`) |
| `src/renderer/render_target.rs` | `RenderTarget::clone_target()` — a deep clone with the same descriptor and fresh textures, as against `Clone`, which stays the JS-identity handle copy |
| `src/renderer/pass.rs` | `PassNode::render_target()`, so a subclass can size and sample the target its base owns |
| `src/nodes/node.rs`, `src/nodes/tsl.rs`, `src/renderer/programs.rs` | `UniformSource::Settable` and `uniform_settable()` — a uniform whose value can change between draws without changing the program |
| `src/materials/node_material.rs` | `setupOutput()`'s `premultipliedAlpha` wrap |
| `examples/` | `webgpu_postprocessing_ssaa.rs`, plus `ssaa_quad`, `ssaa_render_pipeline_quad` and `ssaa_scene` in `dump_wgsl.rs` |
| `docs/postprocessing.md` | "Effect nodes that own render targets" |

`SsaaPassNode` composes a `PassNode` instead of extending one, and — like
`PassNode` — is fired by the application rather than from inside the quad's
render. Both divergences are `docs/postprocessing.md`.

## WGSL

Against `scouts/scouts/webgpu_postprocessing_ssaa/dump/`:

* **accumulation quad fragment** (`m03`) — byte-identical apart from the banner
  and the already-listed `fn0` / `fn1` declaration-order swap — `fn0` is
  `premultiplyAlpha` and `fn1` `unpremultiplyAlpha` in both, so the one line
  this rung turns on,
  `output.color = fn0( fn1( ( nodeVar0 * vec4<f32>( object.nodeUniform2 ) ) ) )`,
  is the same text with the same meaning.
* **`RenderPipeline` quad fragment** (`m05`) — one new divergence, now in
  `docs/nodes.md` §8: the port always emits `PassNode`'s outer `to_var`, so the
  quad carries an extra `nodeVar1 = nodeVar0;` and every later `nodeVarN` is
  shifted by one. Three promotes a `TempNode` to a var only when `analyze()`
  saw it read more than once, and here the pass texture is read once.
* **scene vertex / fragment** (`m00` / `m01`) — the known §8 divergences and
  nothing new; this is the same instanced-standard-material path
  `webgpu_postprocessing_radial_blur` already walks, with `vInstanceColor`
  reaching `DiffuseColor` the same way.

Every pre-existing `dump_wgsl` section is **byte-identical** before and after
this branch (102 sections, compared one by one against a baseline taken at
4f9b2bc). The `premultipliedAlpha` wrap in `setupOutput` is the only change on
a shared emitter path, and it is gated on a flag no other material sets.

## What the pixels found

* **`Math.random` starts five draws in.** `renderer.inspector = new Inspector()`
  runs *before* the instance loop on this page, and each of the inspector's five
  `List`s draws one `Math.random()` for its DOM id
  (`examples/jsm/inspector/ui/List.js:11`). Without the skip the 120 spheres get
  the wrong positions, scales and hues — visibly, not subtly. This is the same
  skip `webgpu_tsl_galaxy` makes, and the constant is named
  `INSPECTOR_RANDOM_DRAWS` in the example for that reason.
* **The accumulator's clear alpha.** The dumped descriptor says
  `clearValue: (0, 0, 0, 1)`; the JS says `setClearColor( 0x000000, 0.0 )`. The
  scout plan says to reproduce the dump. The dump is a serialization artifact of
  the `renderContext` object three.js reuses, and the blend algebra settles it:
  the weights sum to 1 and the blend is `one/one/add` on alpha too, so alpha 1
  in would mean alpha 2 out and the quad's `unpremultiplyAlpha` would halve the
  colour. Alpha 0 grades 0 pixels; that is the whole argument.
* **The weights table in the plan is wrong.** §2.3 lists the eight unbiased
  weights stepping by 3/512 and summing to 1.0547. The JS formula
  `1/n + (1/32) * ( -0.5 + (i + 0.5)/n )` steps by 1/256 and sums to exactly 1.
  `the_eight_unbiased_sample_weights` pins the real values.

## What was ruled out

* **Raising the cache grace window.** Nine renders per frame evicted the frame's
  own builder state mid-frame — a failure the scout did not name and
  `steady_frame_builds_nothing` caught. Raising `CACHE_GRACE_RENDERS` would have
  moved the numbers in
  `churning_geometry_and_materials_does_not_grow_the_caches`. Counting *frames*
  instead — only a render whose destination is the screen ticks the clock —
  strictly slows the clock and left every existing number alone
  (`frame 1 (2, 2, 1), frame 10 (2, 6, 5), frame 50 (2, 6, 5)`, unchanged).
* **A uniform per sample weight.** `UniformSource::Value` bakes its bytes into
  the program cache key, so eight weights would have meant eight programs and
  eight pipelines. `UniformSource::Settable` holds an `Rc<RefCell<Vec<f64>>>`
  hashed by pointer identity and read per draw. That is three.js's default, not
  an extension of it: every `UniformNode` there is mutable and the frozen
  `Value` is the special case.
* **Raising `STEADY_FRAME_CEILING`, or a per-rung allowance.** Eight scene
  renders of 120 instanced spheres cost 11.9 ms steady against the 100 ms
  ceiling. Nothing needed touching.
* **The ladder.** depth_texture 0 / instance_mesh 60 / materials_basic 0 /
  rtt 1 / lights_phong 31 / morphtargets 0 / shadowmap 7 / lights_physical 4 /
  postprocessing_masking 18 / tsl_galaxy 40 / radial_blur 7, and ssaa 0. Every
  previously green number is unchanged — which is what makes `autoClear`, which
  touches every pass's `loadOp`, safe.

## Frame structure

26 passes, in this order: clear / scene / accumulator-clear / quad, then
(clear / scene / quad) × 7, then the canvas pass — exactly the plan's §3.2, with
the rung-9 canvas recording-order divergence. 17 draws, 2119689 triangles,
3 programs and 3 pipelines on frame one; frames two through four build nothing.

## Left out

* **`copyTextureToTexture` for the depth.** three.js copies the last sample's
  depth into the pass target's depth texture so a later effect can read
  `getTextureNode( 'depth' )`. Nothing here reads it and the port has no
  `copyTextureToTexture`; see `docs/postprocessing.md`.
* **The page's GUI.** `sampleLevel`, `unbiased`, `clearColor`, `clearAlpha` and
  `viewOffsetX` are fields on `SsaaPassNode` / `Renderer` rather than dat.GUI
  bindings, and the example sets the ones `animate()` sets.
* **The viewer.** `src/bin/viewer.rs` keys its examples `1`..`9`, `0` — a table
  of exactly ten — so this row's steady frame comes from the e2e harness's own
  measurement at 800x500, as the radial-blur row's does.

