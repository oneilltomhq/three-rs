# `webgpu_layers`

Status: **green.** 76 of 100000 pixels against three.js 5f610f5's own
`test/e2e/image.js`, threshold 0.1%. Three's own frame for the page scores the
same 76 against the same JPEG: the port's frame is pixel-identical to three's
(`tools/dump-webgpu.mjs`' `actual_full.png`, max channel difference 0). Intel
Iris Xe, Mesa 25.3.6, wgpu on Vulkan. Steady frame 3.5 ms (full ladder run), 5 draw calls,
16985 triangles.

Three plain `Mesh`es of 2500 instanced petals each (`mesh.count = 2500`), one
per layer (`particles.layers.set( i )`), over a `screenUV` gradient
`backgroundNode`. The camera is enabled on layers 0, 1 and 2, so all three
pass `_projectObject`'s `layers.test( camera.layers )`. The page's GUI toggles
a camera layer; the graded frame is the untouched default.

## What was added

* **`RotateNode`'s `vec3` branch** (`tsl::rotate`). The port had only the
  `vec2` branch. The `vec3` one builds one `mat4` per axis and chains them in
  the default `'XYZ'` order, then `.mul( vec4( position, 1 ) ).xyz`. Every
  `rotation.x` / `cos( rotation.x )` is a fresh node in three, so each is
  emitted inline; only `rotation` itself is shared, twelve times, which is why
  the builder hoists it into a temp. See `docs/nodes.md` §50.
* **The page**, `examples/webgpu_layers.rs`. `MathUtils.randFloat` is
  `low + Math.random() * ( high - low )` over the harness' seeded sequence:
  eight draws per petal, three meshes in order. `getMaterial()` runs before the
  renderer and its `Inspector` exist, so the Inspector's own draws come after
  all of them.

Everything else was already in the port: `Layers` and the render list's
layer test, `instancedBufferAttribute` over separate buffers, `mesh.count`,
`mod`, `time`, `screenUV`, `map` + `alphaMap` + `alphaTest`, `DoubleSide` with
`forceSinglePass`, and the texture's mipmap chain.

## What the pixels found

Nothing to chase: the first run was 76 pixels, and a byte comparison against
three's own frame showed no difference at all. The 76 are three's own JPEG
disagreement at the petals' edges.

`examples/dump_wgsl.rs`' `layers_petals` section matches three's `m03` / `m04`
statement for statement, apart from two things the port already did on other
rungs:

* the shared temps are `nodeVarN` private vars where three writes
  `let nodeConstN` in the vertex stage;
* `map` and `alphaMap` are the same texture here, and the port gives both
  samples one uv-transform uniform (`nodeUniform3`) where three has two
  (`nodeUniform7`, `nodeUniform9`). Both hold the same identity matrix.
