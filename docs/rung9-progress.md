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

## Still to do

2. RenderTarget multiplicity: three simultaneous `rgba16float` targets, each
   with its own `depth24plus` depth texture.
3. `PassNode` / `pass(scene, camera)` with the nested render in `updateBefore`.
4. `RenderPipeline` with `outputNode`; `needs_frame_buffer_target` must become
   state-driven so the quad draws straight to the `rgba8unorm` canvas.
5. Per-target clear: `0xe0e0e0` → linear 0.7454042095350284 (a = 1) for the
   base scene, `(0,0,0,0)` for the two mask scenes (`background === null`).
6. The two JPEGs with their exact settings (flipY false, generateMipmaps false
   + LinearFilter on texture1, SRGBColorSpace on both → `rgba8unorm-srgb`,
   13 mip levels on texture2).
7. `examples/webgpu_postprocessing_masking.rs` + the e2e entry.
