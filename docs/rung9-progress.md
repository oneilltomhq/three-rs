# Rung 9 — webgpu_postprocessing_masking

Branch `rung9` from `port` cdf834a.

## Step 1 — WGSL identity (DONE)

`examples/dump_wgsl.rs` gained a `masking_quad` entry built the way the
example's TSL chain builds it:

```rust
let base  = to_var(None, texture_uv(&base_rt_texture,  uv()));   // pass(scene, camera)
let mask1 = to_var(None, texture_uv(&mask1_rt_texture, uv()));
let mask2 = to_var(None, texture_uv(&mask2_rt_texture, uv()));
let mut compose = base;
compose = mask1.a().mix(compose, texture(&texture1));
compose = mask2.a().mix(compose, texture(&texture2));
material.fragment_node = Some(render_output(compose));
material.vertex_node   = Some(quad_vertex_node());
```

Two facts this pins down:

* `pass()` is a **var wrapping a texture node**, not a bare texture node.
  `PassNode` is a `TempNode` whose `setup()` returns a `PassTextureNode`
  (also a `TempNode`), so each pass emits two vars —
  `nodeVar0 = textureSample(...); nodeVar1 = nodeVar0;` — and `.a` is taken
  on the outer one. `to_var(None, ...)` reproduces that exactly.
* `PassTextureNode` calls `setUpdateMatrix( false )`, so a pass texture
  samples the raw `uv()` varying with **no** texture matrix, while the two
  JPEGs go through `texture()` and so carry a `mat3x3` in the object
  uniform block. That is what produces the dump's binding interleave
  0,1 / 2,3 / **4 = object buffer** / 5,6 / 7,8 / 9,10.

Result: the generated quad vertex and fragment WGSL are byte-identical to
`handoff/scouts/rung9/output_quad.{vert,frag}-r186.wgsl` apart from the
banner and the order/indentation of the `// codes` function bodies (both
listed in `docs/nodes.md` §7). Binding indices, `objectStruct` layout,
`nodeUniformN`/`nodeVarN` numbering and statement order all match.

The scene material is the plain `MeshBasicNodeMaterial` already shipped at
rung 1; diffing it against `scene_basic.{vert,frag}-r186.wgsl` shows only
the §7 divergences (banner, uniform numbering, `VERTEX_` sub-build names,
declaration order).

## Steps 2-7 (DONE) — rung 9 passes

`cargo test -p three-rs --test e2e -- --test-threads=1` →
**webgpu_postprocessing_masking: 18 of 100000 pixels differ (0.018%), limit
0.1%.** Rungs 1, 3, 4 unchanged at 0 / 0 / 1; rung 2 moved 45 → 60, see below.

What landed:

* `src/renderer/pass.rs` — `PassNode`, `src/renderer/render_pipeline.rs` —
  `RenderPipeline`, both described in `docs/postprocessing.md`.
* `Renderer::needs_frame_buffer_target()` is no longer hardcoded `true`: it is
  `!neutral_output`, and `Renderer::with_neutral_output` is
  `RenderPipeline.render()`'s save/set/restore of tone mapping and output colour
  space. With both neutral the quad draws straight into the `rgba8unorm` canvas.
* `Renderer::_clearColor` alpha is now 0, as `WebGPURenderer`'s `alpha: true`
  default makes it. A mask scene has no background, so its target clears to
  `(0,0,0,0)` — that zero alpha *is* the mask.
* A material-less `Mesh` gets the default white `MeshBasicNodeMaterial` instead
  of a panic.
* `Texture` setters for `colorSpace` (→ `rgba8unorm-srgb`), `flipY`,
  `generateMipmaps`, `minFilter`, `magFilter`.
* `RenderTarget::set_samples`, and `set_size` now also drops an attached
  `DepthTexture`'s GPU object so a 1×1 pass target can grow to the drawing
  buffer.
* `examples/webgpu_postprocessing_masking.rs` and its `tests/e2e/main.rs` entry.

## Pass structure vs `dump-r186.json`

Traced with a temporary `eprintln!` in `Renderer::draw` / `generate_mipmaps`:

| # | port | dump |
|---|---|---|
| base | rgba16float + depth24plus, 800×500, samples 1, clear `[0.7454042095350284 ×3, 1]`, 0 draws | identical |
| mask1 | rgba16float + depth24plus, clear `[0,0,0,0]`, 1 draw (box, 36 indices) | identical |
| mask2 | rgba16float + depth24plus, clear `[0,0,0,0]`, 1 draw (torus, 3072 indices) | identical |
| canvas | rgba8unorm + depth24plus, clear `[0,0,0,0]`, 1 draw (`draw(3,1,0,0)`) | identical |
| mipmaps | 12 blits, rgba8unorm-srgb, 1 layer | identical |

Texture facts also confirmed: `758px-Canestra…jpg` 758×600 with 1 mip level
(`generateMipmaps = false`), `2294472375_24a3b8ef46_o.jpg` 4096×2048 with 13.

## The one behaviour change to an earlier rung

`webgpu_instance_mesh` went 45 → 60 differing pixels when the clear alpha was
corrected from 1 to 0. That example has `scene.background === null`, so
three.js clears its framebuffer target to `(0,0,0,0)`; 60 is exactly the score
three.js itself gets against that reference JPEG (`handoff/RUNGS.md`, rung 2),
so the port now agrees with three.js instead of with the old accident. Still
well inside the 0.1% limit.

## Residue

The remaining 18 pixels are the usual zune-jpeg vs libjpeg-turbo decode
difference on the two JPEGs (`docs/nodes.md` §7) plus JPEG ringing on the mask
silhouettes in the reference screenshot.
