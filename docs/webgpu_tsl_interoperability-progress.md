# `webgpu_tsl_interoperability` — the same shader written twice

**Result: 0 of 100000 pixels different, limit 0.1%.** Exact, on the first
render that reached the canvas.

Two full-width quads under an `OrthographicCamera( -1, 1, 1, -1, 0, 1 )`,
showing the same CRT shader: the top one built from two `wgslFn()` blocks — the
page's hand-written WGSL copied through verbatim — and the bottom one from TSL
nodes. Nothing moves, nothing is lit, and the whole page is two draws.

Re-graded in three's own harness before porting: `Diff 0.0% in file:
webgpu_tsl_interoperability`, stock threshold, `--webgpu`, under the GPU lock.

## What was added

| Area | What | Why |
|---|---|---|
| `src/nodes/code.rs` | `CodeDef.includes` is `Vec<NodeRef>` | three's `includes` is a node list, and this page puts a `varyingProperty` in one |
| `src/nodes/node.rs`, `src/nodes/tsl.rs` | `Node::Code`, `tsl::code()` | a nested `wgslFn` as a member of that list |
| `src/nodes/builder.rs` | `emit_code_fn()` generates its includes | `CodeNode.generate()` builds them before its own code |
| `src/nodes/builder.rs` | the fragment result is formatted to `vec4` | a `fragmentNode` returning a `vec3` |
| `src/nodes/builder.rs` | `JoinNode`'s per-component convert | `vec3( bool, bool, bool )` |
| `src/renderer/mod.rs` | `Renderer::set_output_color_space` | `outputColorSpace = LinearSRGBColorSpace` |
| `src/renderer/mod.rs` | `read_canvas_pixels()` no longer rebuilds the canvas | see below |

Everything else the page needs was already there: `varyingProperty` (from
`webgpu_mesh_batch`), `wgslFn` with texture and sampler parameters (from
`webgpu_materials`), `fragmentNode`, `positionNode`, `time`, `PlaneGeometry`,
`OrthographicCamera`, the PNG loader and `RepeatWrapping`.

`docs/nodes.md` §20 has the detail.

## What the pixels found

A black frame out of a render that had worked.

The page sets `renderer.outputColorSpace = LinearSRGBColorSpace`, which is the
working space, so `needsFrameBufferTarget` is false and the scene is drawn
straight into the canvas with no colour-transform pass behind it. That made
this the first page on the ladder whose *last* pass is the scene itself, and
therefore multisampled. `Renderer::read_canvas_pixels()` opened with
`prepare_canvas( false, 1 )`, and `prepare_canvas()` rebuilds the canvas
whenever the sample count it is asked for differs from the one the canvas has —
so the readback threw away the frame it was about to read. Every page until now
ends with the single-sampled output quad, where that call is a no-op.

It reads what is there now and only builds a canvas when there is none. The
whole ladder is unchanged by it, which is the point: the bug needed a page that
does not end in a quad to show at all.

Once the readback stopped discarding the frame, the comparison was exact.

## What the WGSL found

Two things, both found by diffing against three's dump *before* the first
render, and both about the shader around a `wgslFn` body rather than inside it.

* **An include is load-bearing.** `crtVertex` assigns `varyings.vUv = uv;`
  inside a string, so nothing in the graph reads that varying in the vertex
  stage and nothing declared it. Three's answer is `wgslFn( source, [ vUv ] )`:
  `includes` is a list of *nodes*, built before the code that needs them. The
  port had it as a list of other `wgslFn`s, which covers the only case the
  ladder had until now.
* **`output.color` is a `vec4`.** `crtFragment` returns a `vec3`, and the port
  was assigning it as it stood — WGSL that does not compile, for any
  `fragmentNode` narrower than a `vec4`.

A third, smaller one: `vec3( ind.equal( 0 ), … )` is `vec3<f32>( f32( ( … ==
0.0 ) ), … )`, because `JoinNode.generate()` converts a component whose
primitive type is not the join's. That conversion is in the join rather than in
`wgsl::convert()`'s equal-length arm on purpose — §8 says why — and it changes
no other shader on the ladder.

## What was ruled out

* **`wgsl::convert()`'s equal-length arm.** Implementing three's
  `fromTypeLength === toTypeLength` case wholesale would put an `f32()` around
  every comparison the port widens through `Node::Op`, which §8 already records
  as deliberately absent. The conversion belongs where three has it.
* **`tsl::uv()` in the TSL vertex shader.** Three's `uv()` is
  `attribute( 'uv' )`; the port's is that attribute routed through a varying,
  which is the fragment stage's reading of it. A vertex shader assigning to
  *another* varying wants the attribute, so the example writes
  `attribute( "uv", Type::Vec2 )` — three's own definition.
* **Making the two quads share a program.** They do not in three either: the
  two materials generate different WGSL, and the page's whole point is that the
  two spellings are independent. Two programs, two pipelines, two draws.
* **The inspector's `Math.random()` draws.** `createParameters()` is called
  here, unlike `webgpu_instance_uniform` — but nothing on the page draws a
  random number, so the harness' seeded sequence is never touched.

## WGSL

`cargo run --release --example dump_wgsl` emits `interoperability_wgsl` and
`interoperability_tsl`. Both match `dump-interoperability/m01`–`m04` statement
for statement, including the whole of both copied bodies and the
eighteen-statement TSL fragment; the differences are the ones `docs/nodes.md`
§8 already lists (generated uniform and var numbering, the absent `VERTEX_`
sub-build temps).

## Gates

* `cargo fmt --check`, `cargo clippy --release --all-targets -- -D warnings`,
  `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps` — clean.
* `cargo test --release --lib` — 60 passed; `tests/nodes_wgsl_varying.rs` — 3.
* e2e, release, serial, under the GPU lock — 34 tests, all green, with all 27
  earlier pixel counts unchanged. Steady frame 2.23 ms, ceiling 100 ms, and
  `programs` / `pipelines` / `buffers` all zero on frames two and three.
