# `webgpu_tsl_raging_sea` — the MaterialX noise on a live surface

**Result: 12 of 100000 pixels different, limit 0.1%.** Steady frame about
10 ms; 2 draw calls, 131073 triangles (the 256 x 256 plane and the output
quad).

The rung for issue #142. It is a `PlaneGeometry( 2, 2, 256, 256 )` under a
`MeshStandardNodeMaterial` whose `positionNode` displaces every vertex with
two sines plus three octaves of `mx_noise_float`. Its `normalNode` evaluates
the same wave function at two neighbouring points in the fragment stage, and
its `emissiveNode` remaps the elevation into a pink glow. `time` is pinned to
0, so the graded frame is the sea's first frame.

`raging_sea`'s reference screenshot is the same at r186 and at the vendor's
current dev head. The fragment and vertex WGSL were compared against a dump
of the r186 build (`tools/dump-webgpu.mjs` with `THREE_JS_DIR` pointing at
an r186 tree). The wave code, all three `wavesElevation` expansions, the
emissive remap and the `normalView` line agree statement for statement,
except for the var and uniform numbering. The lighting tail is the
standard flow every other lit rung already grades.

## What was added

| Area | What | Why |
|---|---|---|
| `src/nodes/materialx/` | the whole of `MaterialXNodes.js` | #142; gated by `tests/nodes_mx_library.rs` |
| `src/nodes/builder.rs` | a non-`int` loop bound is written `i32( … )` | `Loop( { start: float( 1 ), end: smallWavesIterations.add( 1 ) } )` |
| `src/nodes/builder.rs` | an inlined `Fn()` block is built once per scope | `elevation` is read by `emissiveNode` and `normalNode`; see below |
| `src/nodes/builder.rs` | a layout `fn` is emitted into each stage that calls it | `mx_perlin_noise_float_1` is called from both stages |
| `src/nodes/builder.rs` | a varying the vertex stage reassigned carries that value | the fragment reads the *displaced* `positionLocal`; see below |
| `src/materials/node_material.rs` | lit materials honour `emissiveNode` | the glow |
| `examples/webgpu_tsl_raging_sea.rs` | the page, with local `remap()` and `transformNormalToView()` | neither is in `tsl.rs` yet |

## What the WGSL found

Every one of these would have been visible in the pixels. The diff against
three's dump found each one before the first render.

- **The fragment reads the displaced position.** In three `positionLocal` *is*
  the varying: `setupPosition()` writes `varyings.positionLocal = (
  varyings.positionLocal + … )`, so the fragment stage's `positionLocal` is
  the displaced surface. That is what the page's normals and emissive are
  computed from, and it is why they sit a wave's height above the geometry
  they shade. The port had a private var in the vertex stage and fed the
  varying from the `position` attribute. Now a varying that the vertex stage
  has assigned to is written from that var (`varyings.positionLocal =
  positionLocal;`). A varying that was only read keeps its old form, so no
  other material's WGSL moved.
- **The small waves ran twice.** `elevation` is one inlined call site read by
  two consumers. The builder cached the var it returns but re-emitted its
  statements, so the `Loop` subtracted the octaves a second time before
  `normalNode` read it. Three builds a node's stack once per stage. The only
  other dump change is `shadowmap_phong_ground` losing a repeated, idempotent
  split assign.
- **`f32( i )`.** The loop index is an `i32`; three's `format()` converts it
  at each use. The port's `wgsl::convert` deliberately has no same-length
  arm (`docs/nodes.md` §8), so the example casts at each use. It uses a
  separate cast per use, because a shared one would be hoisted.

## Scope

- No `INSPECTOR_RANDOM_DRAWS`: nothing on this page reads `Math.random` after
  `new Inspector()` does.
- The GUI and `OrbitControls` input are ported as in every other rung and do
  nothing to the graded frame. `controls.target.y = -0.25` and the damping are
  applied by the first `controls.update()`.
