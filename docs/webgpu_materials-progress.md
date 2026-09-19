# `webgpu_materials` — the TSL breadth example

**Result: 44 of 100000 pixels different (0.044%), limit 0.1%.** Passed on the
first render, with no iteration against the reference image.

Seventeen teapots over a `GridHelper`: sixteen `MeshBasicNodeMaterial` with a
different `colorNode` each, and one `MeshNormalMaterial`. No lights, no
post-processing, one `TeapotGeometry` shared by all seventeen. Everything new
here is in the node system and in `setupDiffuseColor()`.

## Deltas from the scout plan

The plan was written today against current main, so the brief's "the plans
predate 0.2.0" reconciliation did not apply. Two things the plan asked for
turned out to need no library code at all:

* **Step 6, the `Fn` named-input ergonomics.** Three's `Fn( ( input ) => … )`
  called with `{ color: … }` is an object destructure in a function with no
  layout, so it inlines. The port's `inline_fn` is positional, and a named JS
  input is just a named Rust closure argument. Materials 28 and 29 generate
  byte-identical WGSL for free.
* **Step 7's `VarNode` loop hoisting.** The port already declares `var<private>`
  at module scope and assigns in place, which is what three's dump shows inside
  the `for`.

## What was added

| Area | What | Why |
|---|---|---|
| `src/materials/mod.rs` | `opacity_node`, `alpha_test_node`, `MaterialKind::Normal`, `MeshBasicNodeMaterial::normal()`, `MeshNormalNodeMaterial` alias | materials 24, 25 and 27 |
| `src/materials/node_material.rs` | the `opacityNode` / `alphaTestNode` tail of `setupDiffuseColor()`, and `MeshNormalNodeMaterial`'s override of it | every material compiles through this, so it landed alone with a full ladder rerun as its gate |
| `src/helpers/` (new) | `GridHelper` | the page's floor |
| `src/nodes/tsl.rs` | `srgb_transfer_eotf()`, `srgb_to_working()`, `pack_normal_to_rgb()` | `MeshNormalNodeMaterial`'s diffuse colour |
| `src/nodes/tsl.rs` | `screen_uv()`, `.flip_y()`, `triplanar_texture()`, `.zx()` | materials 32 and 33 |
| `src/nodes/tsl.rs` | `wgsl_fn()` re-export, `call_wgsl()` | materials 30 and 31 |
| `src/nodes/code.rs` (new) | `CodeDef`, `ParamKind`, the WGSL declaration parser, `getCode()` | the port of `WGSLNodeFunction.js` |
| `src/nodes/node.rs` | `Node::CodeCall` | `FunctionCallNode` over a `wgslFn` |
| `src/nodes/builder.rs` | the `CodeCall` arms of `sources()`, `needs_var()` and `generate_inner()`, plus `emit_code_fn()` and `code_texture()` | emitting the code, ordering `includes`, binding texture and sampler parameters |
| `src/nodes/tsl.rs` | `vertex_color()` widens to `vec4` *before* the varying | the grid helper is the port's first graded `vertexColors` |

`examples/webgpu_materials.rs`, its `[[example]]` entry, its e2e test, its
viewer entry (key `d`, the fourteenth) and twelve `materials_*` sections in
`examples/dump_wgsl.rs`.

## What the pixels found

Nothing. The frame passed the first time it was rendered, which is what the
eight internally gated steps were for: every one of them was checked against
three's dumped WGSL module before the next was started, so by the time the
example existed there was nothing left for the image to disagree with.

What the *WGSL* diff found, before any pixels, was the vertex-colour varying:
the port varied the `vec3` `color` attribute and joined the alpha on in the
fragment stage, where three's `VertexColorNode` — an `AttributeNode` declared
`vec4` — widens in the vertex stage and varies a `vec4`. Identical pixels (a
constant interpolates to itself), different varying layout. m13/m14 pinned it.

## Three's `Loop`-as-a-value bug, reproduced

The page's last material sets `colorNode` to a `Loop()`. `LoopNode.generate()`
returns an empty snippet, so `vec4( colorNode )` casts nothing and three emits
`DiffuseColor = vec4<f32>(  );` — two spaces, a zero vector, and the loop's
four texture taps thrown away. That teapot is opaque black in three's own
reference image.

The port reproduces it exactly: `Node::Cast` formats `"{type}( {snippet} )"`
and lands on the same two spaces. Pinned by
`tests/scene_webgpu_materials.rs::a_loop_as_a_color_node_discards_its_result`
rather than left to the 0.1% threshold, which one teapot out of seventeen would
very nearly slip past. `docs/nodes.md` §8.

## What was ruled out

* **The numeric oracle first.** `tests/scene_webgpu_materials.rs` checks the 51
  deterministic `Math.random()` draws, the seventeen mesh transforms after the
  single `animate()` step, the camera's three matrices and `GridHelper`'s two
  vertex buffers against three's own `src/` run in node
  (`scouts/webgpu_materials/scene_t0.json`). Rung 5 lost 2.41% of its pixels to
  a Euler-written-without-syncing-the-quaternion bug of exactly this class, and
  the image is the most expensive possible way to find it.
* **The camera's projection matrix** is compared to the oracle *after* the
  WebGL→WebGPU clip-space conversion (`m[10] → (m[10]-1)/2`, `m[14] → m[14]/2`),
  derived in the test. The oracle ran three's `src/` directly, so its camera
  kept `WebGLCoordinateSystem`; converting rather than loosening the tolerance
  keeps the oracle as ground truth.
* **`Math.random()` tolerance.** `x = sin( seed ) * 10000` puts a one-ulp
  difference between Rust's `f64::sin` and V8's at ~2e-12 in the fraction, so
  the draws are compared at 1e-9 and the transforms at 1e-8. Both still pin the
  sequence exactly.
* **Materials 28 and 29 sharing one pipeline** is asserted through
  `renderer.info()` in the e2e test: nineteen programs built (the grid, the
  seventeen teapots, the output pass), eighteen resident.

## WGSL

`cargo run --example dump_wgsl` emits twelve `materials_*` sections. All of
them match `scouts/webgpu_materials/dump/m0*.wgsl` statement for statement; the
differences are the ones `docs/nodes.md` §8 already listed (generated name
numbering, `var<private>` declaration order, the absent `VERTEX_` temps) plus
two new cosmetic ones — `FlipNode`'s `( 1.0 - v.y )` where three writes the
bare string `1.0 - v.y`, and one fewer trailing blank line in `// codes`.

The `// codes` block of both `wgslFn` materials matches three's character for
character, tabs and all, including the whitespace the example page's template
literal leaves after the closing brace.

## Gates

* `cargo clippy --workspace --all-targets -- -D warnings` — clean.
* `cargo test --workspace` — all green.
* e2e, release, serial, under the GPU lock, on the rebased branch (the whole
  stack: rungs 10-12, the radial-blur rung, then this one): compute_points 4,
  depth_texture 0, instance_mesh 60, lights_phong 31, lights_physical 4,
  **materials 44**, materials_basic 0, mesh_batch 0, morphtargets 0,
  postprocessing_masking 18, postprocessing_radial_blur 7, rtt 1, shadowmap 7,
  skinning 6, tsl_galaxy 40 — all of 100000, limit 100. Steady frame 8.56 ms,
  ceiling 100 ms; `viewer --headless --frames 40` gives a 7.3 ms mean over the
  last 30, and `programs`/`pipelines`/`buffers` all zero on frames two and
  three.

## Not done

* **`toneMapped: false`** on `GridHelper`'s material. The port's tone mapping is
  applied in the output pass from the renderer's setting; `webgpu_materials`
  uses `NoToneMapping`, so it is a divergence only on a page that tone maps, and
  none on the ladder does.
* **Scalar `Material.alphaTest`** and its `materialAlphaTest` uniform. Only the
  node form is ported, because that is what the page sets and a uniform nothing
  writes is a trap rather than an API.
* **`wgslFn` gaps**: a `void` return, `ptr<>` parameters, `glslFn`, and `wgslFn`
  in the vertex stage. `docs/nodes.md` §8.
* **`triplanarTexture`'s `positionNode` / `normalNode` arguments.** The page
  passes neither, so the defaults (`positionLocal`, `normalLocal`) are baked in.
* **`Inspector`.** Registers the renderer with the panel and makes no draw calls
  and no `Math.random()` draws, unlike rung 13's.
