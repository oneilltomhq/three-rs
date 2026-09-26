# `webgpu_struct_drawindirect`

Status: **green**, and the green is weaker than it looks. 0 of 100000 pixels
against three.js 5f610f5's own `test/e2e/image.js`, threshold 0.1%. Intel Iris
Xe, Mesa 25.3.6, wgpu 30.0.1 on Vulkan. Issue #167.

## What the graded frame is

The page's `render()` is

```js
renderer.render( scene, camera );
renderer.compute( computeInitDrawBuffer );
renderer.compute( computeDrawBuffer );
```

It draws first and runs its kernels second. On the one frame the grader
captures, the `IndirectStorageBufferAttribute` still holds the zeroed
`Uint32Array( 5 )` it was created with, so the `drawIndirect` draws nothing
and the frame is the `0x00001f` background. three's own dump
(`tools/dump-webgpu.mjs`) shows the same image. A port that drew nothing at
all would pass too.

So the rung is gated on what the image cannot see:

| gate | what it checks |
| --- | --- |
| `tests/nodes_compute_indirect_wgsl.rs` `init_draw_buffer_matches_three` / `draw_buffer_matches_three` | both kernels, line for line against three's `m04` / `m05` (committed as `tests/fixtures/webgpu_struct_drawindirect/`), up to generated-name numbering. The dispatch sizes are `[1, 1, 1]` and `[1563, 1, 1]`. |
| `both_kernels_bind_the_indirect_attribute` | both kernels bind one storage buffer, the attribute's, read-write |
| `render_material_reads_the_instanced_attributes` | the material's four instanced attributes step per instance, and the fragment flow matches three's |
| `tests/renderer_compute_indirect.rs` `the_kernels_write_the_draw_arguments_the_next_frame_draws` | after one frame the buffer reads back `[ 3, 100000, 0, 0, 0 ]`; frame one is all background; frame two, drawn from what the kernels wrote, covers more than a tenth of the frame |

## What was added

| area | what |
| --- | --- |
| `src/core/indirect_storage_buffer_attribute.rs` | `IndirectStorageBufferAttribute`: a `u32` array with a `BufferId`, one GPU buffer that is both `STORAGE` and `INDIRECT` |
| `src/core/buffer_geometry.rs` | `BufferGeometry::set_indirect()` / `indirect()` / `instance_count`; `BufferAttribute::new_instanced()` (`InstancedBufferAttribute`) |
| `src/nodes/node.rs`, `src/nodes/tsl.rs` | `struct_type()` / `storage_struct()` (`struct()` and `storage( attr, structType )`), `.get( member )`; `atomic_store` and the other atomic functions |
| `src/nodes/builder.rs` | the `// structs` declaration, the struct storage binding, `atomic< u32 >` members, and the storage buffers declared before the uniform structs (three's order) |
| `src/renderer/mod.rs` | `drawIndirect` / `drawIndexedIndirect` from the geometry's attribute; `read_indirect_buffer()`; `compute_indirect()`; the compute stage now sees `time` |
| `src/materials/mod.rs` | `force_single_pass`, which keeps a transparent `DoubleSide` material in one draw as three does |
| `examples/webgpu_struct_drawindirect.rs` | the port |

`docs/nodes.md` §28 is the reference for all of it.

## Notes

* **`time` is 0 on every frame under the grader**, so the kernel's
  `max( pow( sin( time ) + 1, 4 ) * 100000, 100 )` is 100000. The GPU test
  pins the same time and asserts that value.
* **The random fill.** The page fills offsets, colours and two orientations
  per instance from `Math.random()`, normalising each. The port draws from the
  same seeded sequence in the same order, but nothing graded depends on it.
* **`webgpu_compute_reduce`** was the issue's third candidate and is not a
  rung. Its page runs two `WebGPURenderer`s on two half-width canvases under a
  DOM panel that draws the threads, and its later reductions use
  `subgroupAdd`, which #167 leaves out. Its workgroup-memory shapes
  (`workgroupArray`, `workgroupBarrier`, `invocationLocalIndex`,
  `workgroupId`) are checked against its dump in
  `workgroup_memory_matches_three_spelling`, and on the GPU in
  `atomics_workgroup_memory_and_indirect_dispatch`.
