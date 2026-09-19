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
