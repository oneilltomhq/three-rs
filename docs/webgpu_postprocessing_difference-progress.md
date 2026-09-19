# webgpu_postprocessing_difference

Branch `rung-postprocessing-batch`, cut from `main` 512ece1. **PASS — 13 of
100000 pixels differ (0.013%), limit 0.1%.**

The plan is `scouts/scouts/postprocessing-batch/PLAN.md`; the reference WGSL is
`scouts/scouts/postprocessing-batch/dump-difference/`.

## What was added

| area | what |
|---|---|
| `src/loaders/gif.rs` | a GIF89a first-frame decoder — palette, transparency, interlace, LZW |
| `src/loaders/texture_loader.rs` | the decoder is picked by magic number, not by extension, the way `createImageBitmap` does |
| `src/textures/texture.rs` | `Texture::swap_gpu()` — the port's half of `PassNode.toggleTexture()` |
| `src/renderer/render_target.rs` | `add_previous_texture()` / `toggle_texture()`, and the previous textures are sized and allocated with the attachments |
| `src/renderer/pass.rs` | `PassNode::previous_texture_node()`; `getTextureNode()` is now the bare texture node, not a var copy; `render()` toggles before it renders |
| `src/nodes/builder.rs` | `dot` builds its operands at the node's *input* type; an inlined `Fn()` call is transparent to `analyze()`; a swizzle past the end of its source expands the source |
| `examples/` | `webgpu_postprocessing_difference.rs`, plus `difference_quad` and `difference_scene` in `dump_wgsl.rs` |
| `tests/` | `nodes_dot_widening.rs`; the rung's e2e test and its `rung!` row |

## The two things that are easy to get wrong

* **The "previous" texture is never rendered.** `PassNode.updateBefore()` calls
  `toggleTexture()` for every previous-texture name **before** it renders, so
  on frame 0 the scene lands in one of the pair and the node named "previous"
  points at the other — zero-initialised, never drawn into. The graded frame is
  therefore `| 0 − current |`, i.e. the whole image saturated, not a still
  black screen. Rendering the previous buffer, or toggling after the render,
  gives a different picture that still looks plausible.
* **`luminance()` of a vec4.** `frameDiff` is a `vec4`, and
  `MathNode.generate()` builds a `dot`'s operands at the *input* type — the
  widest operand — not at the result type, so the `vec3` coefficients widen to
  `vec4( vec3( 0.2126, 0.7152, 0.0722 ), 1.0 )` and the alpha difference is
  weighted 1.0 rather than dropped. `tests/nodes_dot_widening.rs` pins it.

## The toggle, and why it is on the texture and not on the node

three.js swaps the two `Texture` objects inside the render target and rebuilds
whatever was keyed on them. A `NodeRef` in this port is immutable and the
`texture_uv()` node holds its `Texture` handle for good, so the port swaps the
**GPU textures behind the two handles** instead (`Texture::swap_gpu`), leaving
both identities — and every bind group, pipeline and cache key on them — where
they are. That is safe because `bind_groups()` and `texture_view()` build a
fresh view per draw per frame, and it is what keeps the rung's second and third
frames at `BuildCounts::default()`. The previous texture is registered on the
render target (not merely conjured) so `prepare_render_target` allocates it;
it is never a colour attachment (`pass.rs`'s unit tests assert both).

## WGSL

Against `dump-difference/`:

* **quad vertex** (`m03`) — byte-identical apart from the banner.
* **quad fragment** (`m04`) — byte-identical in the flow, including
  `vec4<f32>( vec3<f32>( 0.2126, 0.7152, 0.0722 ), 1.0 )`, the five vars and
  `fn1( vec4<f32>( nodeVar2, clamp( vec4<f32>( nodeVar2, 1.0 ).w, 0.0, 1.0 ) ) )`.
  What differs is the `// codes` order and the `fn0`/`fn1` numbering, both
  `docs/nodes.md` §8 entries.
* **scene vertex** (`m01`) — the uniform-numbering, varying-order and
  sub-build-name (`VERTEX_…`) divergences of §8, and nothing else.
* **scene fragment** (`m02`) — identical apart from those and the fog
  parameters, which the port folds to constants where Three keeps uniforms
  (§8, "Fog parameters as constants"). The values are the same.

Every pre-existing `dump_wgsl` section is byte-identical before and after this
branch **except `rtt_fx_quad`**, which changed on purpose: see below. 205
sections in, 211 out, six of them the new `difference_*` ones.

## Two builder rules this rung had to fix

Both were found by diffing the generated quad fragment, not by looking at
pixels, and both move the port *towards* three.js rather than away:

* **An inlined `Fn()` call is transparent to `analyze()`.**
  `ShaderCallNodeInternal.build()` in the analyze stage is
  `outputNode.build( builder, output )` and nothing else — it neither counts
  itself nor stops the walk — so two call sites that share one memoised body
  count *the body* twice and it becomes a var. The port was counting the call
  node and early-returning, so `saturation()` read once as a vec3 and once for
  its `.w` was inlined twice, doubling the work. This is also why
  `webgpu_rtt`'s quad fragment now hoists `max( mix( … ) )` into `nodeVar1`
  instead of spelling it out three times inside `hue()`: three.js hoists it
  too. `webgpu_rtt` still grades at 1 pixel.
* **A swizzle past the end of its source expands the source.**
  `SplitNode.generate()` builds its input at a type long enough to hold the
  components asked for, so `renderOutput()` taking the alpha of a vec3
  `outputNode` is `vec4<f32>( nodeVar2, 1.0 ).w` and not `nodeVar2.w` — which
  is not WGSL at all. Only this rung reaches it.

## crate.gif

`TextureLoader` fed every file to the JPEG decoder. `ImageLoader` hands bytes
to the browser, which sniffs the type, so the port now sniffs three magic
numbers. The GIF decoder is the browser's *first frame* only: logical screen
descriptor, global and local colour tables, the graphic control extension's
transparent index, interlace, and variable-width LZW. It was checked
byte-for-byte against PIL's decode of `examples/textures/crate.gif` — 262144
bytes of RGBA, identical — with a temporary test that is not in the tree,
because nothing derived from a reference may be. What is in the tree is a
hand-written 2x2 GIF89a fixture and a truncation test.

## Numbers

| | |
|---|---|
| different pixels | 13 of 100000 (0.013%) |
| steady frame | 1.5 ms |
| draw calls | 2 |
| triangles | 13 |

Frames 2 and 3 build nothing: `programs 0 pipelines 0 geometries 0 buffers 0
textures 0`, with the toggle running on both.
