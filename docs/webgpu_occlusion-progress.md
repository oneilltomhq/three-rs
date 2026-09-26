# `webgpu_occlusion`

Status: **green.** 0 of 100000 pixels against three.js 5f610f5's own
`test/e2e/image.js`, threshold 0.1%. Three's own frame for the page also scores
0 on this machine. Intel Iris Xe, Mesa 25.3.6, wgpu on Vulkan. Steady frame
1.9 ms (release), 3 draw calls, 963 triangles.

A green `MeshPhongNodeMaterial` plane (`DoubleSide`) in front of a yellow Phong
sphere, under an `AmbientLight` and a `DirectionalLight`, with MSAA. The sphere
has `occlusionTest = true`. The plane's `colorNode` is the page's
`OcclusionNode`, an `updateType = OBJECT` node whose `update( frame )` writes
blue into its uniform while `frame.renderer.isOccluded( sphere )` is false, and
green once it is true.

## Why the graded frame is blue

The sphere is behind the plane from the first frame, but an occlusion query's
answer is read back asynchronously. three maps the *previous* frame's
read-back buffer, so the first answer reaches `update()` on frame three at the
earliest. The grader takes frame one, where the plane is blue, as three's
screenshot shows.

That means the graded image cannot tell working queries from missing ones.
`tests/renderer_occlusion.rs` checks the behaviour directly, from the page's
own frame loop:

- the plane's centre is blue on frames one and two and green on frames three
  to six;
- with the sphere moved in front of the plane, the plane stays blue for six
  frames. This separates a query that counted samples from one that was never
  recorded.

## What was added

| area | what |
| --- | --- |
| `src/renderer/occlusion.rs` | query sets per render context, the resolve and read-back buffers, the async map and its collection into `occluded` |
| `src/renderer/mod.rs` | `record_pass()` begins and ends queries on the `lastOcclusionObject` walk; `draw()` hands the occlusion results to the object uniforms; `render()` polls when a map is outstanding |
| `src/core/object3d.rs` | `occlusion_test` |
| `src/nodes/node.rs`, `tsl.rs` | `NodeFrame` with `is_occluded()`; `ObjectUpdate` takes a frame; `uniform_frame()` |
| `examples/` | `webgpu_occlusion.rs`; `dump_wgsl` sections `occlusion_plane`, `occlusion_sphere` |
| `tests/` | `renderer_occlusion.rs` (GPU) |
| `tools/dump-webgpu.mjs` | a stray line from a merge (`pageFile`, undefined) made every dump throw; removed |

`docs/nodes.md` §36 is the long form.

The two materials' WGSL matches three's `m00`–`m02` structurally. The only
differences are the ones §8 already lists: `nodeVar` against `nodeConst`,
declaration order, and uniform numbering.

## Why not `webgpu_reversed_depth_buffer`

It was the first page asked for. It cannot be graded on this machine. **three.js
itself fails its own reference for it:** captured through `tools/dump-webgpu.mjs`,
which pins the page exactly as the grader does, three's frame scores 1329 of
100000 pixels (1.33%) against `examples/screenshots/webgpu_reversed_depth_buffer.jpg`
with the unmodified `Image.compare( …, 0.1 )`. The difference is the point of
the page. Each panel shows two planes 0.0002 apart at up to 3200 units away,
and which one wins the depth test there depends on the GPU's depth
precision. On this Intel/Mesa stack the planes z-fight differently from the
machine that took the screenshot, in the normal panel's first four rows and
the fourth row of the other two. The labels under the panels, which are HTML
text, match. Grading the page would take a loosened threshold, which the rules
forbid, so it joins `webgpu_instance_path` and `webgpu_camera` (see
`docs/webgpu_modifier_curve-progress.md` and HANDOFF.md's rung 0).

No port of the page was attempted, because nothing could grade it. Here is
where the port stands against three's `reversedDepthBuffer`:

- **Projection matrix.** `Matrix4::make_perspective` / `make_orthographic` take
  no `reversedDepth` argument and always build the forward `[0, 1]` matrix.
- **Depth compare and clear.** `_getDepthCompare()` in the port maps
  `material.depthFunc` one-to-one. three's flips it under `reversedDepthBuffer`
  (`LessEqual` becomes `GreaterEqual`), and the port has no flip.
  `record_pass()` clears depth to 1, never 0.
- **Frustum culling.** `Frustum::set_from_projection_matrix` already takes the
  `reversed_depth` flag, but `render_list.rs` always passes `false`.
- **Nodes.** `perspectiveDepthToViewZ` and the shadow filters take only the
  non-reversed branch, and each says so in a comment.
- **The rest of the page.** It also needs `logarithmicDepthBuffer` (the middle
  panel), which the port does not have either, and three `WebGPURenderer`s
  side by side on one page. The e2e harness grades one canvas.
