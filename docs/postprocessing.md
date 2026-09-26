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

  **Two vars, but only when the graph holds the pass itself.** A `TempNode`
  earns a var at usage two, and `PassTextureNode.setup()` builds its `passNode`
  — that is the second usage. `pass( scene, camera )` composed directly (radial
  blur, ssaa, and `bloom( … )`, which is a pass-like `TempNode` of its own) is
  that case. `passNode.getTextureNode( name )` is not: the graph holds the
  `PassTextureNode`, the `PassNode` is built only from its `setup()`, and the
  sample lands in **one** var. `PassNode::node()` is the first;
  `PassNode::texture_node( name )` is the second. `m03` and `m12` of
  `webgpu_postprocessing_bloom_selective`'s dump show both in one example.
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
third `stencil` argument, which has no buffer behind it here. Every clear this
rung makes has a render target bound, so it is the bare pass; a `clear()` on the
canvas goes through the internal framebuffer target and ends in the output blit,
as `Renderer.clear()` does.

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

## MRT: several colour attachments from one draw (`webgpu_postprocessing_bloom_selective`, part A)

`mrt( { output, bloomIntensity: float( 0 ) } )` names the values a draw
writes to a render target's several colour attachments. Three levels
are involved and all three are ported:

```rust
let pass = PassNode::new();
pass.set_mrt(mrt(vec![
    ("output", output_property()),
    ("bloomIntensity", float(0.0)),
]));
// asking for the node is what creates the attachment
let bloom_intensity = pass.texture_node("bloomIntensity");

// per material
material.mrt_node = Some(mrt(vec![("bloomIntensity", uniform_value(Type::F32, vec![1.0]))]));
```

* **The pass** holds the default. `PassNode::set_mrt` is
  `passNode.setMRT()`; `PassNode::render` hands it to the renderer for the
  duration of its own render and restores the previous one, as
  `PassNode.updateBefore()` does. `Renderer::set_mrt` / `mrt()` are
  `Renderer.setMRT()` / `getMRT()`.
* **The material** overrides it. `NodeMaterial.setup()` merges the two with
  the material's entries winning, which is how fifty spheres sharing one
  pipeline each get their own `bloomIntensity`.
* **The render target** decides the layout. `MRTNode.setup()` resolves each
  output *name* against `renderTarget.textures` and fills the members by
  attachment **index**, so the order the attachments were created in is the
  order of the `@location`s, and the dictionary's own order decides nothing.
  An output with no attachment of that name is dropped.

`PassNode::texture_node( name )` is `getTextureNode( name )`: it creates
the attachment on first ask and memoises the node. Naming an output in the
MRT does *not* create its attachment — three.js is the same, which is why
a page that sets an MRT and never samples the extra output renders
single-attachment.

### What the fragment stage emits

With an MRT in play the generated fragment stage takes three.js's other
output shape, `OutputStructNode`'s:

```wgsl
struct OutputType {
	@location( 0 ) m0 : vec4<f32>,
	@location( 1 ) m1 : vec4<f32>,
	
};
var<private> output : OutputType;
…
	Output = max( vec4<f32>( DiffuseColor.xyz, DiffuseColor.w ), vec4<f32>( 0.0 ) );
	output.m0 = Output;
	output.m1 = vec4<f32>( object.nodeUniform2 );

	// result

	return output;
```

The struct's *name* changes too (`OutputStruct` → `OutputType`), and the
member names are `m0`, `m1`, … — that is three's own branch, not an
accident, and `examples/dump_wgsl.rs`'s `bloom_selective_scene` section is
byte-identical to `m01_fragment_fragment.wgsl` of the scout's dump.

Two details are load-bearing:

* **The assignments are in the flow, not in the result section.**
  `OutputStructNode.generate()` pushes one `output.mN = …` line per member
  and returns the struct's name, so the entry point ends at a bare `return
  output;`.
* **`Output` is analysed once, not twice.** Without an MRT the basic output
  is reached both by the `Output` property and by `output.color`, which is
  what gives it a var of its own. Here the MRT's `output` member reads the
  *property* back, so the output node is reached only by the property
  assignment and stays inlined into it — which is exactly what the dump
  shows. It is the same rule a custom `material.output_node` already had.

Everything else on the ladder is untouched: with no MRT set the fragment
stage is the old `struct OutputStruct { @location( 0 ) color }` and one
`output.color = …`. All 184 pre-existing `dump_wgsl` sections are
byte-identical across this change.

### The pass and the pipeline

A pass gets one colour attachment per `renderTarget.textures` entry, all
sharing the pass's clear op, and a pipeline one `ColorTargetState` per
attachment. The attachment *count* is part of `RenderState`, so a program
drawn into a one-attachment pass and into an MRT pass gets two pipelines
rather than a validation error.

`MRTNode.blendModes` and `MRTNode.clearColors` — three's per-output blend
mode and clear colour — are not ported. Every MRT material here is opaque,
where three's `MaterialBlending` and `NoBlending` both come out as no
blend state at all, and nothing on this ladder sets a per-output clear.
MSAA and MRT never meet either: a `PassNode` with an MRT is `samples: 0`,
so only attachment 0 can have a resolve target.

## `BloomNode`: eleven render targets and twelve quads (`webgpu_postprocessing_bloom_selective`, part B)

`bloom( input )` is the UnrealBloomPass as a node
(`examples/jsm/tsl/display/BloomNode.js`): a luminosity high pass, five
mip levels each blurred horizontally and then vertically with a separable
Gaussian, and a composite that adds the five mips back together. It is the
third and largest of the effect-node shapes in this document — an effect
that owns render targets *and* several materials of its own.

```rust
let mut bloom_pass = bloom(output_pass.mul(bloom_intensity_pass));
// once per frame, before the output quad
bloom_pass.render(&mut renderer);
render_pipeline.output_node = Some(render_output(
    scene_pass.texture_node("output").add(bloom_pass.node()),
    ToneMapping::Neutral,
));
```

### The twelve passes

`BloomNode::render` is `updateBefore()`: `resetRendererState` (no MRT, an
opaque black clear colour, `autoClear` on), then the high pass into
`bright`, then for each of the five mips a horizontal pass into `h[i]` and
a vertical one into `v[i]`, then the composite back into `h0` — which is
the texture `node()` samples. Every one of the twelve is a single
attachment with no depth, because all eleven targets are
`depth_buffer: false`, and every one clears: the quad covers the target
anyway, but `autoClear` is what three.js leaves on here.

### Sizes are floored, not rounded

`setSize()` is `Math.floor( width * resolutionScale )` and then
`Math.floor( res / 2 )` per mip. At 800×500 and the default half scale
that is 400×250, 200×125, 100×**62**, 50×**31**, 25×15 — `floor( 125 / 2 )`
is 62 where rounding would give 63, and the chain diverges from there
down. `src/nodes/display/bloom.rs`'s `the_mip_chain_is_floor_halved` is
the gate; the scout plan's "rounding, not integer division" reading of the
dumped sizes is wrong about the rule and right about the numbers.

### Ten separable materials, five programs' worth of WGSL

three.js swaps `separableBlurMaterial.colorTexture.value` between the
horizontal and the vertical pass of a mip, so one material serves both. A
`Texture` is an identity in this port and a material's graph names it, so
`BloomNode` builds **two** blur materials per mip, one per direction. Their
generated WGSL is the same text — that is what the five
`bloom_separable_N` sections of `dump_wgsl` diff against `m05..m09` — so
the divergence is ten pipelines where three.js has five, not a different
shader.

### The high pass is replaceable

`BloomNode::with_high_pass` takes the function three.js keeps in
`bloomNode.highPassFn`. It is a constructor argument rather than a field
because the port has no `setup()` stage to rebuild the quad materials in:
the materials are built once, in `new`. `webgpu_postprocessing_bloom` and
`_anamorphic` are the callers that will pass their own.

### `uniformArray`

The composite's five bloom tint colours are one `uniformArray`, not five
uniforms: `uniform_array_vec3()` gives a handle whose `element( i )` all
read the same binding, emitted as

```wgsl
struct NodeBuffer_0Struct { value : array< vec4<f32>, 5 > };
```

with each `Vector3` padded to a `vec4` exactly as
`UniformArrayNode.js` does. The five bloom *factors* beside them are a
`array_var()` — a `var<private> nodeVar0 : array< f32, 5 >` written once
and indexed five times, which is what three's `array( [ … ] )` const
becomes when it is read more than once.

## The previous frame (`webgpu_postprocessing_difference`)

`passNode.getPreviousTextureNode( name )` is the frame *before* this one on
that output. Three keeps two textures per name and swaps them in
`toggleTexture( name )`, which `updateBefore()` runs for every such name
**before** it renders — so the very first frame draws into one of the pair and
the "previous" node points at the other, which nothing has ever rendered into.
The zero-initialised texture is what the graded frame of
`webgpu_postprocessing_difference` actually sees: its `| previous − current |`
is `| 0 − current |`, i.e. the whole image saturated. Rendering the previous
buffer, or toggling after the render rather than before, gives a different
picture that still looks plausible.

### Divergence: the swap is on the texture, not on the node

three.js swaps the two `Texture` objects inside the render target and rebuilds
whatever was keyed on them. A `NodeRef` here is immutable and
`texture_uv( texture, uv() )` holds its handle for good, so the port swaps the
**GPU textures behind the two handles** (`Texture::swap_gpu`) and leaves both
identities — and every bind group, pipeline and cache key on them — alone.
`bind_groups()` and `texture_view()` build a fresh view per draw per frame, so
no cached view can go stale, and the rung's second and third frames build
nothing. The previous texture is registered on the render target
(`RenderTarget::add_previous_texture`) so that `prepare_render_target`
allocates and resizes it with the attachments, but it is never itself a colour
attachment.

`getTextureNode( name )` is also *not* `pass( … )` used as a value: it is the
inner `PassTextureNode` on its own, so it emits one var and no
`nodeVarN = nodeVarM;` copy. `webgpu_postprocessing_masking` takes the first
form and this rung the second; both dumps show the difference.

## No pass at all (`webgpu_postprocessing_direct`)

Everything above puts the output transform on a quad of its own: render the
scene into a target, then draw one triangle that reads the target, applies
`renderOutput()` and writes the canvas. `DirectRenderPipeline` is the other
arrangement — **no target, no quad**. The transform is compiled into the end
of every material's fragment shader, and the frame is just the scene's draws
straight to the canvas.

```rust
let mut pipeline = DirectRenderPipeline::new();
pipeline.output_node = Some(vec4_join(vec![
    saturation(output_property().rgb(), saturation_factor),
    output_property().a(),
]));
pipeline.render(&mut renderer, &mut scene, &mut camera);
```

The mechanism is a hook on the material setup rather than a node in a graph.
`DirectRenderPipeline::render()` builds `render_output( output_node,
tone_mapping )` once, hands it to the renderer as an
[`OutputContext`](crate::materials::OutputContext), renders with tone mapping
and output colour space neutralised — so the renderer itself adds no output
pass — and takes the hook back off. `NodeMaterial::setup()` does what three's
`getOutput` closure does: assign the material's result to the `Output`
property, then build the hook's node *in its place*, which reads `Output` back
and assigns it again. Hence the doubled `Output = nodeVarN;` at the top of the
tail, which is in three's dump too.

Because it is a field of `SetupContext`, the hook is part of the program cache
key by construction: the same material drawn with and without it compiles two
programs, and neither can be served from the other's slot. The node itself is
memoised on the pipeline, so a steady frame builds nothing.

### What gets the hook

three.js decides inside the closure (`if ( renderer.isOutputTarget === false
&& renderer.getRenderTarget() !== null ) return materialOutputNode`). The port
decides on the renderer, before any material is set up: the hook is applied
only when the render is going to the canvas (`self.render_target.is_none()`),
and shadow passes pass `output: None` explicitly. Same set of materials, one
place to read it.

### The background stops being a clear colour

A solid `scene.background` is normally the clear value. Under a direct
pipeline it would be the one thing in the frame that skipped the transform
every material now applies — a visibly wrong backdrop. `_getBackgroundNode()`
substitutes `uniform( color )` for the duration of the render, which promotes
the background to a real quad draw whose fragment shader carries the same
tail. The port swaps `scene.background` for `Background::Node` and restores it
afterwards, memoising one uniform node per colour so the quad's program stays
a cache hit.

### Why it is not the default

It saves a full-screen colour target and a full-screen draw, and pays for them
by changing what blending means: every material now writes display-referred,
tone-mapped colour, so anything blending against the framebuffer blends in the
wrong space and a transmissive material sampling the framebuffer reads the
wrong values. `outputColorTransform = false` (hand tone mapping down instead
of baking it), the XR direct target and the `onBeforePipeline` /
`onAfterPipeline` callbacks are not ported; nothing on the ladder uses them.
## `rtt()`: a node that is its own render target (`webgpu_postprocessing_anamorphic`, `webgpu_postprocessing_ca`)

`src/nodes/display/rtt.rs` is `src/nodes/utils/RTTNode.js`. `rtt( node )`
hands back a texture node whose texture is a render target it owns: the node
becomes a full-screen quad's `fragmentNode`, the quad is drawn into the target
once a frame, and everything downstream samples the result rather than
re-evaluating the graph.

`webgpu_postprocessing_anamorphic` is the rung that needs it and shows why.
Its high pass reads the bright pass **eighty times** along x. Without the
`rtt()` those eighty taps would each inline the whole
`mix( vec4( 0 ), scenePass, smoothstep( … ) )`, including eighty samples of
the scene texture; with it they are eighty samples of one 800x500 half-float
texture, and the bright pass is computed once.

`webgpu_postprocessing_ca` reaches the same node by the other door.
`chromaticAberration( node, … )` does not sample its argument: it calls
`convertToTexture( node )` first, which is `rtt( node )` whenever the node is
not already a texture. The effect reads its input at four different uvs, so a
graph passed straight in would be evaluated four times per pixel instead of
once. The page also sets `renderPipeline.outputColorTransform = false`, because
the `renderOutput()` is *inside* the RTT pass; leaving it on applies the output
transform twice, which is a plausible-looking image and a failing one.

Three things about the target are not `BloomNode`'s:

* **It keeps a depth buffer.** `new RenderTarget( w, h, { type: HalfFloatType,
  ...options } )` — an `rtt()` caller passes no `depthBuffer`, so it defaults
  to `true`. Three's dump has the matching `depth24plus` beside the
  `rgba16float`, and the port allocates it for the same reason: to match, not
  because the quad reads it.
* **It carries its own sampler wrapping.** `rtt( node, null, null, { wrapS,
  wrapT } )`, and the anamorphic page passes `MirroredRepeatWrapping` on both
  axes. See `docs/webgpu_postprocessing_anamorphic-progress.md` for why the
  dump makes that look untrue and why it is.
* **It is a `TextureNode`, not a pass.** `super( renderTarget.texture, uv() )`
  gives the base class a non-null uv node, so `setUpdateMatrix( uvNode === null )`
  leaves the uv matrix off. An `rtt()` tap therefore carries no `mat3x3`
  uniform, which is why `RttNode::sample` is built on `texture_uv` rather than
  `texture_sample`.

### Who fires it

The same ownership divergence as `PassNode`, `SsaaPassNode` and `BloomNode`,
recorded above: three.js fires `RTTNode.updateBefore()` from inside the render
that samples the texture, so its pass is *recorded* after the pass that reads
it and *submitted* before it. The port has the application call
`RttNode::render( renderer )` explicitly, ahead of the reader, which gives the
GPU the same submission order — the anamorphic example's `animate()` is
`scene_pass`, `bright_pass`, `bloom_pass`, `render_pipeline`, which is three's
submit order exactly, and `webgpu_postprocessing_ca`'s is the same three-call
shape: the scene pass, the RTT pass, the pipeline.

### `fullscreenPass` and `currentSamples`

`webgpu_postprocessing_anamorphic` is the first example on the ladder that
asks for `antialias: true` *and* draws a quad straight to the canvas. Three's
`Renderer.currentSamples` returns 0 when `renderContext.fullscreenPass` is
set, so the final `RenderPipeline` quad is never multisampled even though the
scene pass behind it is; the port now has the same branch
(`Renderer::fullscreen_pass`, set around `render_quad`). Without it the canvas
would get an MSAA resolve texture three's dump does not have.

## MSAA and the output quad (`webgpu_postprocessing_bloom`)

`Renderer.currentSamples` has three branches, not two:

```js
let samples = this._samples;
if ( this._renderTarget !== null ) samples = this._renderTarget.samples;
else if ( this.needsFrameBufferTarget ||
          this._currentRenderContext?.fullscreenPass === true ) samples = 0;
```

The port had the first two. The third is set by `_renderScene()` from
`scene.isQuadMesh === true`, so **any** full-screen quad drawn to the canvas is
single-sample, whatever `antialias` asked for. It has to be: the texture the
quad samples was resolved when its own pass ended, so multisampling the quad
would cost four shader invocations a pixel to average four identical samples,
and the canvas colour attachment would need a resolve target it has no other
use for.

Until this rung the branch was unreachable. Every graded example's last draw is
either three's own output blit or a `RenderPipeline` quad under tone mapping,
and both of those go through the internal framebuffer target — so
`needsFrameBufferTarget` had already returned 0 and no canvas pass had ever run
multisampled. `webgpu_postprocessing_bloom` is the first page to combine
`new WebGPURenderer( { antialias: true } )` with a `RenderPipeline`:
`RenderPipeline::render` neutralises the tone mapping (the transform moves into
the quad's own `fragmentNode`), `needsFrameBufferTarget` goes false, and the
quad lands on the canvas with the renderer's `samples` still 4. On this adapter
a 4x canvas attachment with a resolve target renders black, and the graded
frame is black with it.

`Renderer` carries `fullscreen_pass`, set and restored around `render_quad()`
and around `render_output()`'s quad. The `PassNode`'s own scene render is
unaffected: it binds a render target, so it takes the first branch and keeps
`renderTarget.samples`, which `PassNode::render` has just set to
`renderer.samples()`. That is where this page's antialiasing actually happens —
the scene is drawn 4x into the pass target and resolved before bloom ever
samples it.

## The display nodes of #144

`src/nodes/display/` now also has these ports of
`examples/jsm/tsl/display/`:

- `GaussianBlurNode`;
- `SobelOperatorNode`, `DotScreenNode`, `RGBShiftNode` and `TransitionNode`;
- `hashBlur` and `boxBlur`;
- `AfterImageNode` and `PixelationPassNode`.

`tests/nodes_display_wgsl.rs` gates each against three's dump of a page that
uses it. `webgpu_procedural_texture`, `webgpu_postprocessing_sobel` and
`webgpu_postprocessing_transition` are the graded rungs.

The nodes follow the shapes above:

- The single-pass nodes are functions that return a node.
- `GaussianBlurNode` and `AfterImageNode` own their targets and quads, as
  `BloomNode` does. Their `render()` is `updateBefore()`, fired by the example.
- `PixelationPassNode` wraps a `PassNode` rather than subclassing one. The
  wrapped pass renders nearest-filtered at `floor( drawingBuffer / pixelSize )`,
  through `PassNode`'s new size divisor, with an `{ output, normal }` MRT.
  `PassNode::render` now takes any `RenderCamera` because that page's camera
  is orthographic.

Divergences, each noted where it lives:

- A node that three hands a `TextureNode` takes a `&Texture`. The example
  makes the `convertToTexture()` RTT itself, as `webgpu_postprocessing_ca`
  does.
- `DotScreenNode` reads `screenSize` from the viewport size, the uniform
  three's dump binds (`render.nodeUniform1`). It does not add a setter.
- `TransitionNode`'s mix texture is fixed when the node is built. Three
  swaps `mixTextureNode.value` per frame.
- `AfterImageNode` keeps last frame's output with the render target's
  previous-texture swap from `webgpu_postprocessing_difference`. It does not
  copy.
- `hashBlur` over `viewportSharedTexture()` (`webgpu_backdrop_area`) needs a
  viewport texture the port does not have yet. `hash_blur_with` takes any tap,
  so that page can pass one once the texture exists.
