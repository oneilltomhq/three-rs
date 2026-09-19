# webgpu_postprocessing_anamorphic

Branch `rung-anamorphic`, cut from `37aaca1` (the `rung-bloom-selective-2` tip:
origin/main plus `BloomNode`). **Graded green: 2 of 100000 pixels differ
(0.002%), against Three's own `test/e2e/image.js` at its 0.1% threshold.**

Ladder after this branch — the twenty existing rows unchanged, one row added:

    depth_texture 0 / instance_mesh 60 / materials_basic 0 / rtt 1 /
    lights_phong 31 / morphtargets 0 / shadowmap 7 / lights_physical 4 /
    postprocessing_masking 18 / tsl_galaxy 40 / skinning 6 / mesh_batch 0 /
    compute_points 4 / radial_blur 7 / materials 44 / ssaa 0 / lines_fat 0 /
    pmrem_cubemap 0 / pmrem_test 27 / postprocessing_bloom_selective 1
    + postprocessing_anamorphic 2

## Deltas against the plan and the brief

The scout plan is `scouts/scouts/postprocessing-batch/PLAN.md`, written against
the current tree, so there are no pre-0.2.0 API deltas. Three things in the
brief and the plan were wrong about the source and had to be corrected before
any code was written:

| what was assumed | what r186 actually has |
|---|---|
| the effect is `examples/jsm/tsl/display/AnamorphicNode.js`, "its own node, not BloomNode" | **there is no such file.** The page is `bloom( scenePass, 5, 0, 0.3 )` with `bloomPass.highPassFn` replaced and `setResolutionScale( 0.25 )`. Built on `BloomNode::with_high_pass` accordingly; no second bloom implementation exists. |
| `RTTNode.js` lives in `src/nodes/display/` | it is `src/nodes/utils/RTTNode.js`. (The port keeps it under `src/nodes/display/` beside `BloomNode`, where its only caller is.) |
| "the `MirroredRepeatWrapping` passed to `rtt()` never reaches the GPU — both samplers in the dump are nearest / clamp-to-edge" | **it does reach the GPU.** See below; the dump tool is what is lying. |
| "`luminance( vec4 )` widens the coefficient with `w = 1.0` if this dump takes the vec4 branch" | it takes the **vec3** branch: `dot( nodeVar0.xyz, vec3<f32>( 0.2126, 0.7152, 0.0722 ) )`. The port's existing `luminance` already matches; nothing to do. |

### The sampler finding is a dump-tool artefact

`tools/dump-webgpu.mjs` records `desc: descriptor` **by reference**.
`WebGPUTextureUtils` mutates one pooled `_samplerDescriptor` and calls
`.reset()` on it afterwards, whose defaults are exactly what the dump shows:
clamp-to-edge on all three axes, nearest on all three filters,
`lodMaxClamp: 32`, `maxAnisotropy: 1`. Every sampler in every dump taken with
that tool reads that way, whatever was actually created.

The clincher is in the dump itself: three's sampler cache keys on
`minFilter-magFilter-wrapS-wrapT-wrapR-anisotropy-isDepth-compare`, and the
dump has **two** sampler objects — 42, shared by the scene texture and all
eleven bloom targets, and 55, used by exactly one texture, 37, which is the
`rtt()` target. Two cache entries mean two distinct key combinations, and the
only field that differs between those two targets in the page's source is the
wrapping. So `MirroredRepeatWrapping` is real, and `Wrapping::MirroredRepeat`
→ `wgpu::AddressMode::MirrorRepeat` was added for it.

It is also load-bearing, which is the second half of the brief's finding and is
correct: the high pass reads `uv.x ± 4i / width` with `i` up to ±40, i.e. up to
0.2 outside `[0,1]`. Clamping smears the edge column across a fifth of the
screen; mirroring folds the streak back. The graded frame distinguishes them.

## What was added

| area | what |
|---|---|
| `src/nodes/display/rtt.rs` | new module: `RttNode`, `rtt()`, `convert_to_texture()` — the render target, the quad, `set_wrapping`, `set_size` / `set_resolution_scale`, `node()` / `sample()`, `render()` |
| `src/nodes/display/mod.rs` | re-exports |
| `src/nodes/node.rs` | `Node::Loop` gained `start: Option<NodeRef>` — `None` is three's `Loop( count, … )` shorthand (start `0`), `Some` is `Loop( { start, end }, … )` with the node written out as it stands |
| `src/nodes/builder.rs` | the loop header's start; `distance`'s operands widen to the widest **operand**, not to the `f32` result, so `screenUV.distance( 0.5 )` splats to `vec2<f32>( 0.5 )` |
| `src/nodes/tsl.rs` | `loop_range()`, `distance()` |
| `src/textures/texture.rs` | `Wrapping::MirroredRepeat` |
| `src/renderer/mod.rs` | the `MirrorRepeat` address mode; `Renderer::fullscreen_pass` (`RenderContext.fullscreenPass`) set around `render_quad`, and its branch in `current_samples()` |
| `examples/webgpu_postprocessing_anamorphic.rs` | the example, including `anamorphic_high_pass` — the page's replacement `highPassFn` |
| `examples/dump_wgsl.rs` | `anamorphic_background`, `anamorphic_scene`, `anamorphic_rtt`, `anamorphic_high_pass`, `anamorphic_render_pipeline_quad` |
| `tests/e2e/main.rs` | the graded row and the steady-frame `rung!` row |
| `docs/postprocessing.md` | "`rtt()`: a node that is its own render target" |
| `docs/nodes.md` §8 | the one divergence this rung adds |

Nothing from the `rung-postprocessing-batch` worker was duplicated:
`PassNode::previous_texture_node`, the `getOutput` hook and
`DirectRenderPipeline` are not needed here and were not added.

## WGSL

`examples/dump_wgsl.rs` against
`scouts/scouts/postprocessing-batch/dump-anamorphic/`:

| section | module | result |
|---|---|---|
| `anamorphic_background` | `m01` | identical |
| `anamorphic_scene` (fragment) | `m03` | identical |
| `anamorphic_scene` (vertex) | `m02` | one divergence, below |
| `anamorphic_rtt` | `m05` | identical |
| `anamorphic_high_pass` | `m07` | identical |
| `anamorphic_render_pipeline_quad` | `m17` | identical but for the struct-block and helper-`fn` orderings already in `docs/nodes.md` §8 |

The five `Bloom_separable` modules and `Bloom_comp` (`m09`–`m15`) are
`BloomNode`'s own and were diffed by the bloom_selective rung;
`setResolutionScale( 0.25 )` changes target sizes, not WGSL.

Three details of `m07` are worth naming, because each one is a thing the port
did not previously generate:

* `for ( var i : i32 = i32( ( - nodeVar1 ) ); i < i32( nodeVar1 ); i ++ )` —
  a `Loop` whose start is a node. `Node::Loop.start` and `tsl::loop_range`
  exist for this line.
* `render.nodeUniform2.zw` — `viewportSize` in three is `viewport.zw` off the
  four-component viewport uniform, **not** `screenSize`. The port's
  `viewport_size()` is the latter; the example uses `viewport().zw()`.
* `distance( ( fragCoord.xy / render.nodeUniform0 ), vec2<f32>( 0.5 ) )` in
  `m01` — `distance` returns an `f32`, so widening its arguments to the result
  type would have emitted `distance( vec2, f32 )`, which does not compile. It
  widens to the widest argument instead.

The one vertex divergence is `positionNode` ordering: three applies the
instance matrix and *then* the page's bob; this port applies the bob first.
The page's instance matrices are pure translations, so the two are equal
vertex for vertex here. `docs/nodes.md` §8 says what would break the equality.

## What the pixels found

Two pixels, first graded run, no iteration. Worth recording what did *not* go
wrong, since each was a place the rung could have failed silently:

* **Sixteen draws, sixteen programs, 398798 triangles**, matching the plan's
  predicted 15 passes / 16 draws exactly (the background and the spheres share
  a pass).
* **`antialias: true` with a quad to the canvas.** This is the first ladder
  example that does both. Without `RenderContext.fullscreenPass` the port
  would have given the canvas an MSAA resolve three's dump does not have; with
  it, `current_samples()` returns 0 for the final quad and 4 for the scene
  pass, as three does.
* **`Math.random` order.** Four draws per instance — `x`, `y`, `z`, then the
  colour — 800 draws for 200 spheres, and nothing draws before the loop. A
  transposed pair would have moved every sphere.
* **`performance.now()` is pinned to 0 under the harness**, so `time` is 0 —
  but the bob is `sin( instanceIndex * 0.5 * timeScale ) * 5`, which is *not*
  zero. An example that zeroed the whole vertical offset would still have
  looked plausible.

## What was ruled out

* Rewriting `positionNode` ordering to match three. It would have touched every
  instanced material on the ladder for a change that is provably invisible in
  this frame. Documented instead.
* A second `luminance` overload for the `vec4` branch: this dump does not take
  it, and the brief only asked for it conditionally.
* `RTTNode`'s `getTextureNode()` / `setup()` plumbing for the cases three
  handles inside `convertToTexture()` — a node that is already a texture, and a
  `PassNode`. Both are decided by the type system here, so
  `convert_to_texture()` is a spelling of `rtt()` that says why the caller
  wants it.

## What was left out

* `RTTNode.autoUpdate` / the `updateBeforeType` frame throttle. The port's
  targets are rendered by an explicit `render()` call (the divergence in
  `docs/postprocessing.md`), so there is nothing to throttle yet.
* `rtt( node, width, height )`'s fixed-size path is implemented
  (`RttNode::with_size`) and unit-tested, but no example uses it.
* The inspector (`toInspector( 'Color' )` on the page) — a no-op for the graded
  frame, as in every other ported example.
* The page's `OrbitControls` damping: the graded frame is the first, which is
  the camera's constructed position looking at the origin.
