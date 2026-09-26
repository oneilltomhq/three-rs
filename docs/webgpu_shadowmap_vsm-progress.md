# webgpu_shadowmap_vsm

Branch `shadows-vsm`, issue #168. **PASS: 0 of 100000 pixels differ**
(limit 0.1%). The steady frame is 4.5 ms in the viewer. The grader's frame is
22 draw calls and 10419 triangles, and frames two and three build nothing.

## What was added

| area | what |
|---|---|
| `src/lights/shadow_filter.rs` | `ShadowMapType`, `ShadowFilter::{Basic, Pcf, Vsm, Custom}`, `ShadowFilterFn` (the `filterNode` hook), `ShadowFilterMap`, `BasicShadowFilter`, `PCFShadowFilter` (moved from `phong.rs`), `VSMShadowFilter`, and the `VSMVertical` / `VSMHorizontal` pass nodes |
| `src/lights/light_shadow.rs` | `LightShadow::{filter_node, shadow_node}` |
| `src/lights/point_shadow.rs` | `BasicPointShadowFilter` beside `PointShadowFilter`, both behind the same `ShadowFilter` dispatch |
| `src/materials/phong.rs` | `ShadowMap::{Filtered, Node}`: a filter over any map, or a whole custom `shadowNode` |
| `src/materials/node_material.rs` | `shadow_material_for( source, type )`: VSM keeps `material.side` |
| `src/renderer/mod.rs` | `Renderer::shadow_map_type`; the depth filter per type; `receiveShadow` objects drawn into VSM maps; `render_vsm_passes()` (two RG HalfFloat targets, two quad passes); programs keyed on the target's component count |
| `src/renderer/render_target.rs` | `RenderTarget::set_rg_format()` |
| `src/nodes/builder.rs` | `with_output_components()`: `getOutputType()` is `vec2` for an RG target; an RG texture node's fetch gets `.xy` |
| `src/nodes/tsl.rs` | `getTextureType()` for two-channel formats; `depth_texture_sample()`; `shadow_blur_samples()` |
| `examples/` | `webgpu_shadowmap_vsm.rs`, and the `shadowmap_vsm_*` sections of `dump_wgsl.rs` |
| `tests/nodes_shadow_filters.rs` | GPU-free checks, with lines copied from three's dumps |

## What the WGSL says

Three's dump has nine modules. The port's `VSMHorizontal` fragment matches
three's `m04` apart from the header comment. `VSMVertical` (`m03`) differs
only in the order of two generated var numbers (§24.6's class). In the lit
Phong module (`m06`), the `VSMShadowFilter` body lines up statement for
statement; the rest is §8's `toConst` / var class and uniform numbering. The
shadow material is the one `webgpu_shadowmap` already matched.

Two things the dump made visible:

* **An RG texture is a `vec2` node.** Three caches the horizontal tap as
  `nodeVar2 : vec2<f32> = textureSample( … ).xy`, so `getTextureType()`
  had to come across. It also changed the DFG LUT sample in every physical
  material to three's own form. That is pixel-neutral: every rung that
  samples the LUT was rerun.
* **The blur passes write `vec2`.** `NodeBuilder.getOutputType()` follows the
  target's format, so the `OutputStruct` member is `vec2<f32>`.

## What the pixels found

The first steady-frame run rebuilt four programs every frame. The pass
materials were keyed after `clone()`, which mints a fresh `MaterialId`; the
key now comes from the stored material.

## Divergences

See `docs/nodes.md` §28.5. The fog is the same constant `fog()` stand-in as
`webgpu_shadowmap`. The `filterNode` hook is a closure over
`ShadowFilterInputs` with `index` in place of `shadow`. There is no
`material.shadowSide`.
