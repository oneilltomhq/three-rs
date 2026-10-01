# Architecture

A map of the code, for finding where something lives and which design note
explains it. The notes it points at are the detail; this page should stay
short enough to read in five minutes.

three-rs is a port, so half of its documentation is three.js itself. Almost
every file names the three.js file it ports in its first line. When a type's
behaviour is in question, three.js' source at the pinned commit is the
reference, and a doc comment here only repeats it where the port differs.

## The one idea

Under three.js' `WebGPURenderer`, every material is a `NodeMaterial`, and its
WGSL is generated from a node graph rather than written by hand. The port
works the same way, and that is what keeps the grader honest: every example
reaches the GPU by the same route, so a green rung can't be a hand-tuned
shader. The exceptions are the mipmap shader in
`src/renderer/shaders/mipmap.wgsl`, which three.js hand-writes too; the
viewer's surface blit in `src/renderer/present.rs`, which is outside the
port; and user code passed through `wgslFn()` (`src/nodes/code.rs`), which
is verbatim by design.

## One frame

`Renderer::render( scene, camera )` in `src/renderer/mod.rs` is
`Renderer.render()`, in the same order:

1. **Scene graph.** A scene object is a `core::Node`, a newtype over
   `Rc<RefCell<Object3D>>` (`src/core/node.rs`). It shares its name with the
   shader-graph `nodes::Node` of step 4 and nothing else; which one a
   sentence means is clear from its module. `update_matrix_world()` walks it as
   three.js does. Why `Rc<RefCell>` and not an arena:
   [`docs/scene-graph.md`](docs/scene-graph.md).
2. **Projection.** `src/renderer/render_list.rs` walks the tree into sorted
   opaque and transparent lists (`projectObject` and `RenderList`); `render()`
   then puts the skybox on the front. The list is the GPU-free part of the
   renderer, so it is unit-tested directly.
3. **Program lookup.** `Renderer::node_builder_state()` looks up each
   item's built program in `node_builder_states` by material id, then
   material version, then the scene-dependent part: lights, shadows,
   instancing, morphing, fog. A steady frame stops here. The table is in
   [`docs/scene-graph.md`](docs/scene-graph.md) (*Program cache*).
4. **On a miss, the node system.** `materials::setup()`
   (`src/materials/node_material.rs`) builds the material's node graph, as
   `NodeMaterial.setup()` does. `NodeBuilder` (`src/nodes/builder.rs`, with
   the WGSL backend in `src/nodes/wgsl.rs`) turns that graph into a
   `NodeProgram`: vertex and fragment WGSL, attribute slots, and descriptors
   for every binding the shader declared. The TSL functions examples write in
   are in `src/nodes/tsl.rs`. All of it: [`docs/nodes.md`](docs/nodes.md),
   §1–4 first.
5. **GPU objects.** `src/renderer/programs.rs` turns a `NodeProgram` into
   shader modules, bind-group layouts and a pipeline, all cached. Per draw,
   the bind groups are resolved from the program's descriptors. Nothing on
   the GPU side is per-material. Uniforms are written when their update type
   is due: once per render, once per frame, or once per object
   (`src/nodes/frame.rs`).
6. **Draw and output.** `draw()` records the pass. If tone mapping or the
   output colour space needs a separate pass, `render_output()` draws the
   frame buffer target to the canvas through one more node material.
   Headless callers read the result back with `read_canvas_pixels()`.

## Renders inside a render

A lot of what an example draws is a nested `render()`. Shadow maps are drawn
by `render()` itself; the rest are fired from a node's `updateBefore()`
before the draw that samples it. Each of these lives with the renderer:

- shadow maps: `render_shadows()` in `mod.rs`
- the mirror: `src/renderer/reflector.rs`
- PMREM environment maps: `src/renderer/pmrem.rs`
- post-processing passes and render pipelines: `src/renderer/pass.rs`,
  `render_pipeline.rs` and `direct_render_pipeline.rs`

Post-processing has its own note: [`docs/postprocessing.md`](docs/postprocessing.md).
Compute kernels are a third route through the same builder:
`Renderer::compute()` with a `ComputeFlow` ([`docs/nodes.md`](docs/nodes.md) §11).

## Identity and caching

A shader-graph node is an immutable `Rc<nodes::Node>`, handled as a
`NodeRef`, and its identity is its address (`src/nodes/node.rs`). The
per-build state three.js hangs on a node lives in the builder, keyed by that
address, and dies with the build. An address is not an identity once the
build is over: a freed one is reused, and a cache keyed by it hands the next
owner stale GPU objects (#58). So the renderer's caches key on counters that
are never reused (`Material.id`, `Texture.id`, …) and are swept at the top of
`render()`, by weak count or by frames since last use. The exception is the
compute-program cache, which still keys on addresses and holds the nodes to
keep them unique; it needs eviction (#237).

`renderer.info()` counts what each frame built. The e2e harness checks it two
ways (`tests/e2e/main.rs`). `steady_frame_builds_nothing` renders most graded
rungs three times with nothing changed between frames and asserts the second
and third build and upload nothing. Every graded frame is also followed by
three more through the example's own `animate()` (`steady_frame()`), which
must stay under a time ceiling; that one catches work the counters don't
see, such as re-formatting a cache key per frame.

## The examples and their three drivers

`examples/*.rs` are ports of three.js' `webgpu_*` pages, written against the
public API in the page's own order. Three programs run them:

- `tests/e2e/main.rs` renders the graded frame and hands it to three.js'
  comparator. What that grader reproduces: [`docs/grader.md`](docs/grader.md).
- `src/bin/viewer.rs` runs them in a window. Its only seam into the renderer
  is `src/renderer/present.rs`.
- `web/` runs them in a browser, on the browser's own WebGPU, and is
  deployed to <https://oneilltomhq.github.io/three-rs/>.

A new rung starts by dumping what three.js hands the GPU for the page
(`tools/dump-webgpu.mjs`, [`docs/dumping.md`](docs/dumping.md)) and porting
until the generated WGSL and the frame both match.

## The rest of the tree

- `src/addons/`: ports of three.js' `examples/jsm/`, kept in the crate so the
  e2e harness stays one test binary.
- `src/loaders/`: PNG, JPEG, GIF, WebP, KTX2, HDR, UltraHDR and glTF with
  Draco and meshopt.
  They read bytes only through `src/io.rs`, which a browser build fills from
  memory.
- `sdf-text/`: signed-distance-field text and `BatchedText`, a workspace crate
  that depends on three-rs the way a user would.
- `addons/controls/`: `three-rs-controls`, the map camera over a ground. Its
  own design, not a port.
- `docs/api.md`: the settled public-API decisions, each with its reason.
- `docs/*-progress.md`: one log per rung. They are history, kept for how a
  number was reached, not reference. `docs/gallery.md` is the rendered
  examples; `docs/RELEASING.md` is the release checklist.
