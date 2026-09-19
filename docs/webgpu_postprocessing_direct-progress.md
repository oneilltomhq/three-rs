# webgpu_postprocessing_direct

Branch `rung-postprocessing-batch`, cut from `main` 512ece1. **PASS — 21 of
100000 pixels differ (0.021%), limit 0.1%.**

The plan is `scouts/scouts/postprocessing-batch/PLAN.md` §3; the reference WGSL
is `scouts/scouts/postprocessing-batch/dump-direct/`.

## What was added

| area | what |
|---|---|
| `src/materials/node_material.rs` | `OutputContext` — `builder.context.getOutput` — and `SetupContext::output`; `setup()` applies the hook |
| `src/nodes/builder.rs` | `MaterialFlow::output_assign`, the hook's own `output.assign( materialOutputNode )` |
| `src/renderer/mod.rs` | `Renderer::set_output_hook()`, and the hook travelling to every material of a canvas render |
| `src/renderer/direct_render_pipeline.rs` | `DirectRenderPipeline` |
| `examples/` | `webgpu_postprocessing_direct.rs`, plus `direct_scene` and `direct_background` in `dump_wgsl.rs` |

## What the rung is actually about

Not the effect — `saturationFactor` is `uniform( 0 )`, so the graded image is
the flat-shaded spheres fully desaturated. It is *where* the output transform
happens. A `RenderPipeline` renders the scene into a target and transforms it
on a quad; a `DirectRenderPipeline` has neither, and every material's fragment
shader ends with the transform inlined:

```wgsl
Output = nodeVar5;
Output = nodeVar5;
nodeVar6 = vec4<f32>( max( mix( vec3<f32>( dot( Output.xyz, LUM ) ), Output.xyz, object.nodeUniformN ), vec3<f32>( 0.0 ) ), Output.w );
nodeVar7 = fn1( vec4<f32>( nodeVar6.xyz, clamp( nodeVar6.w, 0.0, 1.0 ) ) );
nodeVar8 = vec4<f32>( neutralToneMapping( nodeVar7.xyz, render.nodeUniformM ), nodeVar7.w );
output.color = fn0( vec4<f32>( sRGBTransferOETF( nodeVar8.xyz ), nodeVar8.w ) );
```

The doubled `Output = nodeVar5;` is real: `NodeMaterial.setup()` assigns the
property, and then the hook's own `output.assign( materialOutputNode )` assigns
it again. Reproducing it is free and it is what the diff compares.

The frame has **no intermediate colour texture at all** — 93 draws, one pass,
two programs, and the only texture is the depth buffer. The ladder's counters
say the same thing.

## The risk, and the fixture that covers it

The hook edits `NodeMaterial.setup()`, the one function every material in the
crate goes through. The gate is `dump_wgsl`: **244 sections in, 250 out, zero
changed**. The six new ones are the two `difference_*` and the two `direct_*`
triples. A material that is not being drawn under a `DirectRenderPipeline`
gets `SetupContext::output = None` and generates the byte-identical shader it
generated before.

Where three.js decides inside the closure — `if ( renderer.isOutputTarget ===
false && … ) return materialOutputNode` — the port decides on the renderer:
the hook is attached to a material only when the render is going to the
canvas. A shadow pass, a `PassNode`'s target and the internal framebuffer
target are all left alone. It is part of `SetupContext`, so it is part of the
program cache key by construction: the same material drawn with and without
the hook is two programs, not one.

## The background is not a clear colour here

A solid `scene.background` is normally the clear value, which no shader sees —
so it would be the one thing in the frame that missed the transform every
material now applies. `_getBackgroundNode()` substitutes `uniform( color )`
for the duration of the render, which turns it into the background quad's
`colorNode`, and the dump has a `Background.material` pipeline with the same
tone-mapping tail. The port swaps `scene.background` for `Background::Node`
and restores it after, memoising one uniform node per colour so the quad's
program stays a cache hit.

## WGSL

Against `dump-direct/`:

* **scene fragment** (`m03`) — the tail above is statement for statement
  three's, down to the doubled assign. The body above it carries the §8
  "named lighting temps / hoisted accumulator zeros / inlined `faceDirection`"
  divergences every lit rung has, plus uniform renumbering.
* **background vertex and fragment** (`m00`, `m01`) — identical statement for
  statement; what differs is the uniform *order* inside the two structs, the
  `// codes` order with the `fn0`/`fn1` swap, and the order two `var<private>`
  declarations appear in. All §8.

## Numbers

| | |
|---|---|
| different pixels | 21 of 100000 (0.021%) |
| steady frame | 4.9 ms |
| draw calls | 93 (101 objects, the rest frustum culled) |
| triangles | 4192 |

Frames 2 and 3 build nothing. They draw 92, not 93: one sphere leaves the
frustum as `object.rotation` advances, which is the culling working.

## Left out

`outputColorTransform = false` (the contextData path that hands tone mapping
down instead of baking it), XR's direct render target, and the
`onBeforePipeline` / `onAfterPipeline` callbacks. Nothing on the ladder uses
any of them.
