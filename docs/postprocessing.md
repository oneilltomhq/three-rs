# Post-processing (rung 9)

Ports of `three.js/src/renderers/common/RenderPipeline.js`,
`src/renderers/common/QuadMesh.js` and `src/nodes/display/PassNode.js`, as the
`webgpu_postprocessing_masking` example exercises them.

## The shape

```rust
let base  = PassNode::new();
let mask1 = PassNode::new();
let mask2 = PassNode::new();

let mut compose = base.node();
compose = mask1.a().mix(compose, texture(&texture1));
compose = mask2.a().mix(compose, texture(&texture2));

let mut render_pipeline = RenderPipeline::new();
render_pipeline.output_node = Some(compose);

// each frame
base.render(&mut renderer, &mut base_scene, &mut camera);
mask1.render(&mut renderer, &mut mask_scene1, &mut camera);
mask2.render(&mut renderer, &mut mask_scene2, &mut camera);
render_pipeline.render(&mut renderer);
```

## `PassNode`

`pass( scene, camera )` owns a `RenderTarget` of `HalfFloatType` (so
`rgba16float`) plus its **own** `DepthTexture` named `depth` — three targets
means three depth textures, not one shared one. `updateBefore()` sizes the
target to the drawing buffer, sets `renderTarget.samples = renderer.samples`,
and does `setRenderTarget( rt ); render( scene, camera ); setRenderTarget( prev )`.

Two structural facts, both visible in the dumped WGSL:

* `PassNode` is a `TempNode` whose `setup()` returns a `PassTextureNode`, which
  is a `TempNode` too. So a pass emits **two** vars —
  `nodeVar0 = textureSample( … ); nodeVar1 = nodeVar0;` — and `.a` is taken on
  the outer one (`nodeVar4.w` in the dump). The port models this as
  `to_var( None, texture_uv( rt.texture(), uv() ) )`.
* `PassTextureNode` calls `setUpdateMatrix( false )`, so a pass samples the raw
  `uv()` varying. `texture( map )` does not, so each JPEG contributes a
  `mat3x3<f32>` to the object uniform block. That asymmetry is what produces the
  dump's binding interleave `0,1 / 2,3 / 4 = object buffer / 5,6 / 7,8 / 9,10`.

### Divergence: who fires `updateBefore`

three.js collects the graph's `updateBefore` nodes while the quad's material is
built and fires them from *inside* the quad's own render, which is why the
canvas `beginRenderPass` is the first thing in the API trace even though the
nested passes execute first. The nested renders use their own command encoders
and are submitted before the canvas pass's encoder.

Rust ownership makes a node that holds `&mut Scene` across a frame impractical,
so the port keeps the node and the application calls `PassNode::render`
explicitly, immediately before `RenderPipeline::render`. The GPU sees the same
four passes in the same order; only the recording order of the canvas pass's
descriptor differs, and nothing observes that.

## `RenderPipeline`

`RenderPipeline` is a `QuadMesh` whose material is named `RenderPipeline` and
whose `fragmentNode` is `renderOutput( outputNode, toneMapping,
outputColorSpace )` when `outputColorTransform` is true (the default).

`render()` saves the renderer's tone mapping and output colour space, sets them
to `NoToneMapping` / the working colour space, draws the quad, and restores.
With both neutral, `Renderer.needsFrameBufferTarget` is **false**, so the quad
draws straight into the `rgba8unorm` canvas and there is no second output pass:
the unpremultiply → sRGB OETF → premultiply sandwich lives in the quad's own
shader. In the port that is `Renderer::with_neutral_output`, which flips the
`neutral_output` flag `needs_frame_buffer_target()` reads.

The quad is a full-screen **triangle**: positions come from the material's
`vertexNode` (`array<f32,3>( -1, -1, 3 )[ vertexIndex ]` etc.), so the only
vertex buffer is the `uv` attribute at stride 8 — and the draw is a non-indexed
`draw( 3, 1, 0, 0 )`. Its canvas pass still carries a `depth24plus` attachment
and the pipeline still declares `less-equal` / `depthWriteEnabled: true`, which
is what three.js does.

## Clear colour and alpha

`WebGPURenderer`'s `alpha` parameter defaults to `true`, so
`Renderer._clearColor` is `(0, 0, 0, 0)` — **not** opaque black. `Background.update()`
then either leaves it alone (`scene.background === null`) or, for a `Color`
background, copies the colour and forces alpha to 1 and a clear.

At rung 9 that zero alpha is the entire masking effect: the two mask scenes have
no background, so their `rgba16float` targets clear to `(0, 0, 0, 0)` and only
the box and torus texels have `a == 1`. The base scene's
`new Color( 0xe0e0e0 )` is converted sRGB → linear on the CPU by
`Color.setHex`, giving the clear value `0.7454042095350284` that the target,
being linear `rgba16float`, wants.

Fixing this (the port previously used an opaque-black clear) moved
`webgpu_instance_mesh` from 45 to 60 differing pixels — which is exactly what
three.js itself scores against that reference JPEG, so the port now agrees with
three.js rather than with the old accident.

## Materials

`new Mesh( geometry )` with no material gets `new MeshBasicMaterial()`, which
under `WebGPURenderer` is a `MeshBasicNodeMaterial`: white, opaque, front side,
depth test and write on. The renderer supplies it rather than panicking.

## Single-pass effect nodes (`webgpu_postprocessing_radial_blur`)

An effect from `three.js/examples/jsm/tsl/display/` is a node graph, not a
renderer feature: `radialBlur( textureNode, options )` is one `Fn()` that reads
the pass texture several times and returns a colour. The port keeps those in
`src/nodes/display/` **inside the main crate**, not in `addons/`, for that
reason — they need nothing but the node system.

`radial_blur( map, options )` mirrors the JS line for line: a `sampleUv` var, a
`base` const, a `blur` var, an `offset` const, a weight var `w`, the
interleaved-gradient-noise jitter, a node-bounded `Loop` and the final
`mix( blur, base.mul( 2 ), 0.5 )`. Three facts it pins down:

* **`Loop( { end: int( count ) } )` needs no new node kind.** `Node::Loop`'s
  count was already a `NodeRef`, so a uniform bound emits
  `for ( var i : i32 = 0; i < i32( object.nodeUniformN ); i ++ )`.
* **`uniform( int( 32 ) )` is an `f32` uniform.** `UniformNode` takes its type
  from the JavaScript value it is handed, which here is a *node*, so three.js
  falls back to float and the dump carries four `f32` object uniforms. The
  `i32` appears only at the loop bound.
* **The whole effect is wrapped in one `to_var()`.** `Node::Block` is not a
  kind the builder caches or promotes, so without the wrapper the statements
  would be emitted once per read of the result. `docs/nodes.md` §8.

### Tone mapping comes from the renderer, not from the quad

`RenderPipeline` captures `renderer.toneMapping` in its constructor and
re-checks it in `_update()`, *before* `render()` neutralises it for the draw.
So `renderer.toneMapping = NeutralToneMapping` set by the page ends up baked
into the quad's own `renderOutput( outputNode, toneMapping, outputColorSpace )`
— the tone mapper runs inside the post-processing shader, and the canvas still
needs no second output pass. `RenderPipeline::built_for` carries the tone
mapping alongside the output node's identity, so a steady frame is still a
cache hit and a change to `renderer.tone_mapping` rebuilds the quad's program
once.

## Effect nodes that own render targets (`webgpu_postprocessing_ssaa`)

`radialBlur` is a shader; `SSAAPassNode` is a *schedule*. It extends `PassNode`,
keeps a second render target of its own, and renders the scene eight times with
a sub-pixel jitter on the camera, accumulating the results additively into the
pass target that the `RenderPipeline` quad then samples. The port is
`src/renderer/ssaa_pass.rs`.

```rust
let mut ssaa_pass = SsaaPassNode::new();
render_pipeline.output_node = Some(ssaa_pass.node());
ssaa_pass.sample_level = 3;                       // 2^3 = 8 samples

// each frame
ssaa_pass.render(&mut renderer, &mut scene, &mut camera);
render_pipeline.render(&mut renderer);
```

`SsaaPassNode` *composes* a `PassNode` rather than extending one — Rust has no
inheritance, and `node()` / `texture()` forward to it — and, like `PassNode`, it
is fired by the application instead of from inside the quad's render, for the
ownership reason above.

### The schedule

Per sample `i` of `n`:

1. `camera.setViewOffset( w, h, ox + jitter.x * 0.0625, oy + jitter.y * 0.0625,
   w, h )` — the jitter is added to the offset the page already set, which is
   why the restore is a `setViewOffset` and not a `clearViewOffset`.
2. the weight uniform is set (below), the sample target is bound,
   `clear( true, true )`, then `render( scene, camera )`;
3. the accumulation target is bound; on `i == 0` only, it is cleared to
   `(0, 0, 0, 0)`; then the quad is drawn with additive blending.

That is 8 scene renders + 8 accumulation quads + 1 canvas quad = 17 draws
across 26 passes (9 of them clear-only), 2119689 triangles, 3 programs.

`_JitterVectors` is six tables of 1, 2, 4, 8, 16 and 32 offsets, all in
sixteenths of a pixel; `sampleLevel` indexes them and levels above 5 clamp to
the last. The weight of sample `i` is `1/n`, or, with `unbiased` (the default),

```
1/n + (1/32) * ( -0.5 + (i + 0.5)/n )
```

a spread around `1/n` that still sums to exactly 1 and hides the `rgba16float`
rounding of a flat `1/n`. The scout plan's table of the eight values is
arithmetically wrong (it steps by 3/512 and sums to 1.0547); the formula steps
by 1/256, and `the_eight_unbiased_sample_weights` in `ssaa_pass.rs` pins the
real values.

### `autoClear` and `clear()`

This is the first rung to need either. `Renderer.autoClear` decides whether a
render clears its target before drawing; `SSAAPassNode` turns it off for the
whole schedule, so all 17 draws carry `loadOp: "load"` and the only clears are
the explicit `renderer.clear( true, true )` calls — which are, in the port as in
three.js, a `beginRenderPass` with `loadOp: "clear"` and no draws at all.
`setClearColor` and `clear( color, depth )` are the three.js API, minus its
third `stencil` argument, which has no buffer behind it here; the port's
`clear()` does not run the output pass when clearing the canvas, which three.js
would.

A scene with a `Color` background still clears, `autoClear` or not: three.js's
`Background.update()` forces the clear itself. The eight scene renders here have
no background, so `autoClear = false` is what keeps them from clearing.

**The accumulator's clear is alpha 0, not alpha 1.** The dumped pass descriptor
says `clearValue: (0, 0, 0, 1)`, but that is a serialization artifact of the
`renderContext` object three.js reuses across passes — the JS is
`setClearColor( 0x000000, 0.0 )`. The blend algebra decides it: the weights sum
to 1 and the blend is `one/one/add` on alpha too, so with the eight samples each
contributing `w * 1.0` the accumulator ends at alpha 1; starting from alpha 1
would end at 2, and the `unpremultiplyAlpha` in the quad would halve the colour.
The 0-pixel grade confirms it. (The scout plan says to reproduce the dumped
descriptor rather than the JS reading. On this line the plan is wrong.)

### `premultipliedAlpha` on the accumulation quad

The quad's material is `transparent`, `blending: AdditiveBlending`,
`premultipliedAlpha: true`, depth test and write off. `premultipliedAlpha`
changes two things: the blend factors become `one/one/add` on **both** colour
and alpha (rather than `srcAlpha/one`), and `NodeMaterial.setupOutput()` wraps
the output in `premultiplyAlpha()`. Since the fragment node is already
`unpremultiplyAlpha( texture( sample ) * weight )`, the pair cancels and the
sum is over straight colour — which is what makes the weights linear.

### The by-use cache clock counts frames, not renders

The by-use caches (`node_builder_states`, `buffers`) evict what has not been
used for `CACHE_GRACE_FRAMES` ticks. Ticking that clock once per *render* was
fine while a frame was one render; with nine renders per frame it evicted a
frame's own state mid-frame and rebuilt it, which
`steady_frame_builds_nothing` catches. The clock now advances only on a render
whose destination is the screen (`Renderer::begin_frame`) — nested target
renders belong to the frame they precede. Every render still sweeps: a geometry
the scene dropped should go on the render that notices. The grace window is
unchanged at 4, and so are the numbers in
`churning_geometry_and_materials_does_not_grow_the_caches`.

### Left out: the depth copy

three.js ends `updateBefore()` with
`renderer.copyTextureToTexture( this._sampleRenderTarget.depthTexture,
this.renderTarget.depthTexture )`, so that a later effect reading the pass's
`getTextureNode( 'depth' )` gets the last sample's depth rather than an
uninitialised texture. Nothing in this example reads it, the port has no
`copyTextureToTexture`, and the pass target's depth texture is simply never
written. An effect that chains depth off an SSAA pass needs that copy first.
