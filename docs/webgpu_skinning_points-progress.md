# `webgpu_skinning_points`

Status: **green.** 44 of 100000 pixels against three.js 5f610f5's own
`test/e2e/image.js`, threshold 0.1%. Intel Iris Xe, Mesa 25.3.6, wgpu on
Vulkan. Steady frame 7.5 ms, 2 draw calls (the background and one instanced
`Sprite`), 32681 triangles (16340 point quads plus the background).

Michelle (`Michelle.glb`, the `webgpu_skinning` model) is drawn as one blue
point per vertex. A kernel skins each vertex on the GPU with
`computeSkinning( child )`, moves it to world space with
`objectWorldMatrix( child )`, and writes its position and speed into two
`instancedArray`s. A `PointsNodeMaterial` on a `Sprite` with
`count = 16340` reads them back with `.toAttribute()` and draws a
screen-space quad per point, cut to a circle by `shapeCircle()`.

## Reconciling with the plans

There is no scout `PLAN.md` for this rung. It was ported from
`~/src/vendor/three.js/examples/webgpu_skinning_points.html` directly. Three's
WGSL was dumped by `tools/dump-webgpu.mjs` into
`target/dumps/webgpu_skinning_points/` (uncommitted): `m00` is the `onInit`
kernel, `m01` the per-frame kernel, and `m02` / `m03` the material. They were
diffed against `examples/dump_wgsl.rs`'s new `skinning_points_*` sections.

## What was added

| area | what |
| --- | --- |
| `src/nodes/skinning.rs` | `compute_skinning( mesh )`: `getSkinnedPosition()` over read-only storage copies of `position`, `skinIndex`, `skinWeight` |
| `src/nodes/tsl.rs` | `storage_data` / `storage_f32` (storage over a CPU array), `StorageArray::to_read_only()`, `StorageArray::to_attribute()`, `object_world_matrix( object )`, `shape_circle()`, `compute_node( flow, output )` |
| `src/nodes/node.rs`, `builder.rs` | `Node::Compute` (a `ComputeNode` read as a value); `NodeProgram::computes`; `UniformSource::Live`; `BufferSource::StorageData` and `SkeletonBoneMatrices`; `BuildContext::alpha_to_coverage_samples` |
| `src/materials/` | `PointsNodeMaterial.sizeNode` (`size_node`); `setupVertexSprite()` for a points material on a `Sprite` |
| `src/objects/` | `Sprite.count` |
| `src/renderer/` | a `ComputeNode`'s kernel dispatched once per frame before the pass that reads it; skeleton bone matrices for a kernel; storage buffers usable as vertex buffers |
| `examples/` | `webgpu_skinning_points.rs`; `dump_wgsl` sections `skinning_points_{init,update,material}` |

`docs/nodes.md` §44 has the design.

## What the pixels found

The rung passes at 44 pixels. Our full-resolution frame is pixel-identical to
the `actual_full.png` three renders on this machine (0 differing pixels), so
the 44 are three's own distance from its JPEG.

The graded frame has every point blue and 6 px across, for the same reason
`webgpu_skinning` grades the first keyframe: the pinned timer gives a delta
of 0. The `onInit` kernel runs first and writes a speed of `position − 0`,
but the per-frame kernel then runs on the same pose and writes a speed of
exactly 0 before the pass draws. The order is three's: `onInit` compute, the
per-frame compute, then the render (the dump records them in that order).
If the per-frame kernel ran after the pass, or not at all on the first
frame, the points would be 26 px across and not blue.

The ladder did not move. Every new path runs only for a `Node::Compute`, a
`StorageData` or skeleton buffer, or a points material on a `Sprite`, and no
earlier rung has any of these.
