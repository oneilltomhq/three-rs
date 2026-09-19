# Rung 12 — webgpu_compute_points

Branch `rung12-compute-points` from `main` 2cf90bd.

**Status: PASS.** 4 of 100 000 pixels differ (0.004%), limit 0.1%. The ladder
is unchanged: depth_texture 0, instance_mesh 60, materials_basic 0, rtt 1,
lights_phong 31, morphtargets 0, shadowmap 7, lights_physical 4,
postprocessing_masking 18, tsl_galaxy 40. Frames two and three build nothing.

Read the "what the pixels found" section before believing any of that.

## Step 0 — the plan against the tree

The scout plan was written on 2026-09-13 against the pre-0.2.0 tree. What it
asks for that is already here, and so was not built again:

| plan says | the tree already has |
|---|---|
| §1.2 add a `RenderCamera` trait and thread an orthographic camera through `render()` | both, since rung 9's output pass |
| §3.1 add `Type::UVec3` | it exists |
| §4.2 add `blend` / `topology` / `strip_index_format` to `RenderState` | all three, from the lines branch |
| §4.3 add a transparent render list | exists |
| §4.4 give `Info::record_draw` a `PointList` arm | exists |
| §5 add `MaterialKind::Sprite`, `setup_position_view` and the `setupPositionView` seam | all three, from rung 13 |
| §5.1 add `blend_state()` / `is_opaque()` | both exist |

And what the plan could not know: `Node` is a newtype over
`Rc<RefCell<Object3D>>`, `Mesh::new` takes its material, and fallible entry
points return `Result` — so `Points::new( geometry, material )` returns a
`Node` and `Renderer::compute()` returns `Result<(), Error>`.

One thing the plan asked for turned out to be false in the other direction:
`docs/nodes.md` §6 claimed `Stage::Compute` "is in the stage enum and
unreachable". It was not in the enum at all. It is now, and §6 and the new §11
say so.

## Step 1 — literals and casts (`ea8228c`)

Before any of this, two things in the builder printed differently from three
and would have made every compute diff noisy:

* **Literals.** Three prints numbers with ECMA-262 §6.1.6.1.20
  `Number::toString`, which is why its dumps contain `1e-8`, `1e-7` and
  `6.283185307179586` rather than `0.00000001`. `wgsl::js_to_string()` ports
  it. This kernel is full of those constants, so nothing would have matched
  without it.
* **Casts.** `NodeBuilder.format()`. Only the widening half is ported — see
  `docs/nodes.md` §8. The full ladder was tried first and moved 73 lines of
  WGSL across six green rungs.

## Step 2 — the render side of Points (`4d5ebd9`)

*Interim note, kept because it was true for most of the day: the render side
of this rung passed hours before the compute side existed, and was
committed on its own so it could be merged on its own.*

`Points` is a payload beside `Mesh` and `Line`; `Primitive::of()` returns
`PointList` for it, and `PointsNodeMaterial` is a `MeshBasicNodeMaterial` with
`MaterialKind::Points` and `transparent = true`. Three's
`PointsNodeMaterial extends SpriteNodeMaterial`, so it inherits rung 13's
`setupPositionView` seam; the points branch is a plain
`modelViewMatrix * vec3( positionNode )`, not the billboard.

Three facts about this object that `tests/renderer_points.rs` pins, because
each is a silently-wrong-pixels bug and none is visible in this rung's frame:

* **`count` is the instance count, not the vertex count.**
  `RenderObject.getInstanceCount()` takes the instanced geometry's count, else
  `object.count`, else 1. The example is 1 vertex, `drawRange.count = 1` and
  `count = 300000`, i.e. `draw( 1, 300000, 0, 0 )` — 300 000 instances of a
  one-vertex point list, each reading its own particle by `instanceIndex`.
  Reading `count` as the vertex count draws one particle and passes the grader.
* **`drawRange` clamps the vertices.** Separately, and it is what makes the
  one vertex one vertex.
* **The far plane.** The camera is orthographic with `near = 0, far = 1` at
  `z = 1`, and every particle is at `z = 0`, so every particle's NDC depth is
  *exactly* 1. `depthCompare: less-equal` draws them; `less` draws nothing, and
  a frame of nothing is 100% black, which this rung's comparison passes.

## Step 3 — compute (`9795f85`)

`docs/nodes.md` §11 is the description; the table is what it cost.

| added | where |
|---|---|
| `Stage::Compute`, a third visibility bit, the `@compute` template | `src/nodes/builder.rs` |
| `ComputeFlow`, `ComputeProgram`, `NodeBuilder::build_compute()` | `src/nodes/builder.rs` |
| `BufferSource::Storage` | `src/nodes/node.rs` |
| `instanced_array()` → `StorageArray`, `NodeRef::swizzle()` | `src/nodes/tsl.rs` |
| `var<storage, read_write>` / `read` declarations | `src/nodes/builder.rs` |
| `instanceIndex` as builtin / varying / attribute per stage | `src/nodes/builder.rs` |
| storage binding type, `ComputeProgramGpu` | `src/renderer/programs.rs` |
| `Renderer::compute()`, `read_storage_buffer()`, the storage buffer cache | `src/renderer/mod.rs` |
| `info.compute.calls` | `src/renderer/info.rs` |
| the two kernels and the material | `examples/webgpu_compute_points.rs` |

Both kernels generate WGSL byte-identical to three.js r186's own dump —
`docs/rung12/precompute_velocity.compute.wgsl` and
`update_particles.compute.wgsl`, diffed whole-file by
`tests/nodes_compute_wgsl.rs` — modulo the normalisations that test's
`canonical()` performs and §8 justifies: the banner, `enable subgroups;`, the
`@builtin( subgroup_size )` parameter, and first-appearance renumbering of
`NodeBuffer_` / `nodeUniform` / `nodeVarying` / `nodeVar`.

Two template bugs fell out of reading three's compute template beside its
render one, and improved every existing rung: a blank line after `// codes`,
and the `Output` var declared *before* the output node's flow rather than
after it. `docs/lines/LineBasicNodeMaterial_27.frag.wgsl` now matches the
port byte for byte apart from the banner and uniform numbering.

The frame is four submits, in three's order — `onInit` precompute, update,
scene pass, output pass — because `Renderer::compute()` makes one encoder and
one `queue.submit()` per call and `onInit` recurses through it. §11.2.

## What the pixels found

**Nothing, and that is the finding.** The scout flagged it and it is worse
than it sounds. The example's frame at the 400x250 the grader downscales to is
black except for a 2x2 block of lit pixels near the centre. 4 pixels differ
out of 100 000. The 0.1% threshold is 100 pixels. A completely black frame —
compute never dispatched, `onInit` never ran, the points material never
bound the buffer, the draw call clipped — passes this comparison at 0.0%
with room to spare.

So the rung is gated twice over, on things the image cannot see:

* **`tests/nodes_compute_wgsl.rs`** — whole-file diffs of both generated
  kernels against three's dumps, plus the dispatch (`[4688, 1, 1]`) and
  workgroup size (`[64, 1, 1]`), plus targeted assertions that the points
  material declares the same buffer `read` in both stages and reads it through
  a flat `u32` varying in the fragment stage.
* **`tests/renderer_compute_points.rs`** — reads both storage buffers back
  after one and two `compute()` calls and checks 300 000 particles against the
  same arithmetic on the CPU in `f32`. The comparison is split: everything but
  `sin`/`cos` is add, multiply and clamp, so it is asserted *exactly* (the
  particle lands on its velocity bit for bit; two frames land on exactly twice
  it), and only the direction round the circle gets a tolerance, because the
  kernel feeds `sin` arguments up to 9424 radians where an `f32` argument is
  itself good to only ~5e-4 radians. The velocity's *length* is still exact, so
  a kernel reading the wrong index has nowhere to hide.

The negative control is `without_the_precompute_nothing_moves`: the same
update kernel with its `onInit` removed leaves every particle at the origin,
and the graded image does not notice.

Three things the readback caught that nothing else would have:

* The guard has to be built **after** the body or its count uniform takes the
  wrong `nodeUniformN` and the diff fails (it did).
* `@builtin( instance_index )` in a *fragment* entry point is not valid WGSL —
  WGSL has no such builtin there. Caught by diffing against
  `docs/rung12/PointsNodeMaterial.frag.wgsl`, fixed by routing the fragment
  stage through a flat varying, which is what `IndexNode.generate()` does.
* 300 000 over a workgroup of 64 is 4688 workgroups and 300 032 invocations.
  The 32 spare are why the kernel opens with an early return, and the readback
  asserts element 299 999 and 299 967 so a dispatch one workgroup short fails.

## What was ruled out

* **A second comparator, or a tighter threshold for this rung.** Not allowed
  and would not have helped: the information is not in the image.
* **Hand-written WGSL for the kernels.** The whole point of the rung is that
  the builder can reach a `@compute` entry point.
* **Batching the compute into the render's command encoder.** It would be one
  submit instead of three and the pixels would be identical. Three makes one
  encoder and one submit per `renderer.compute()` call, and the scout's
  `dump-r186.json` shows four; reproducing the order is the rung's point.
* **`enable subgroups;`** — see §8. It is a feature request that fails to
  compile on an adapter without the feature, for an entry-point parameter
  nothing reads.
* **Chasing the points material's vert/frag to a byte-identical diff.** It
  differs only by divergences §8 already lists (the `VERTEX_` sub-builds,
  declaration order, name numbering) plus the storage binding number. The
  whole-file diff was replaced with targeted assertions on what is new.

## What was left out

* **`webgpu_compute_texture`.** The plan proposes it as the companion image
  gate for this rung, and it is the right idea: its output is a full-screen
  texture the compute stage writes, so its graded frame *does* distinguish a
  working compute stage from a black one. It was not ported here because one
  rung is one example. What it needs beyond what landed today: a storage
  *texture* binding (`texture_storage_2d<rgba8unorm, write>`), which is a new
  `BindingDesc` variant and a new `TextureSource` access mode;
  `textureStore()`; a compute dispatch over a 2D workgroup grid rather than
  `[n, 1, 1]`; and `texture()` reading the same texture in the fragment stage,
  which needs the read-only/write-only split wgpu enforces on storage textures.
  No new node kinds.
* **`computeAsync`.** `Renderer::compute()` is synchronous. Three's async
  variant only awaits the device's own queue.
* **`info.compute.frameCalls`.** Three clears it in `Info.reset()`, which its
  animation loop calls at the *top* of the frame callback, before
  `renderer.compute()` runs. This port has no animation loop and resets in
  `render()`, which runs *after* the frame's compute calls, so a `frame_calls`
  here would always read zero. The cumulative `calls` is the honest one; "did
  this frame build a compute pipeline" is answered by `info.build`.
* **Subgroup operations**, workgroup shared memory, storage barriers,
  indirect dispatch, and compute over anything but a flat index. None is
  reachable from this example.
* **The example in `src/bin/viewer.rs`.** Every other graded example is
  browsable there; this one is not, because `Scene::camera()` returns a
  `&mut PerspectiveCamera` and `set_size()` writes `aspect` on it, and this
  example's camera is orthographic. Carrying it needs the viewer to hold a
  `RenderCamera` — the trait already exists — rather than a concrete
  `PerspectiveCamera`, which touches every arm of three matches and belongs in
  its own change. The README's steady-frame number for this row therefore
  comes from the release e2e run rather than `viewer --headless --frames 40`.
* **Sweeping the storage-buffer cache.** `Renderer` holds storage buffers by
  `BufferId` and never drops them; a `StorageArray` is application-owned and
  lives as long as the app. Geometry and texture caches are swept (issue #58);
  this one would need the same treatment the first time a rung creates storage
  buffers per frame.
