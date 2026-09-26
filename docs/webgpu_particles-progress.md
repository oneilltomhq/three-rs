# `webgpu_particles`

Status: **green**, with the same caveat as `webgpu_struct_drawindirect`. 0 of
100000 pixels against three.js 5f610f5's own `test/e2e/image.js`, threshold
0.1%. Intel Iris Xe, Mesa 25.3.6, wgpu 30.0.1 on Vulkan. Issue #167.

## What the graded frame is

Two `SpriteNodeMaterial`s: 2000 smoke sprites through `Mesh.count`, and 1000
fire sprites drawn through `drawIndexedIndirect` from a five-word
`IndirectStorageBufferAttribute` the page fills itself. Every sprite's opacity
is `texture.a * ( 1 - life )`, and with `performance.now()` pinned to 0

```text
lifeTime = ( ( 0 + 5 ) * 0.2 * lifeRange ) mod 1 = lifeRange
life     = lifeTime / lifeRange                   = 1
```

for every `lifeRange` in `[ 0.1, 1 )`. All 3000 sprites are drawn with an
alpha of 0, and the frame is the `GridHelper` on the `0x333333` background.
three's dump shows the same frame. The sprites are still drawn, and
`renderer.info()` counts them (6001 triangles), but the image cannot see them.

So the rung is also gated on:

| gate | what it checks |
| --- | --- |
| `tests/nodes_compute_indirect_wgsl.rs` `smoke_sprite_matches_three_spelling` | the smoke sprite against three's `m03` / `m04`: `range()`'s `array< vec4<f32>, 2000 >` uniform buffers read by `instanceIndex` in the vertex stage and a flat varying in the fragment stage, `mod( 1 )` as `tsl_mod_float`, `rotateUV()` as `RotateNode`'s `mat2x2`, the opacity multiply, and the varying `positionLocal` carrying `positionNode`'s value |
| `tests/renderer_compute_indirect.rs` `the_particles_draw_through_the_indirect_buffer` | the fire's buffer reads back `[ 6, 1000, 0, 0, 0 ]`; the time-0 frame has no fire-coloured pixel; at 1.5 s the sprites change more than a tenth of the frame and the fire lights more than 1000 pixels |

## What was added

Beyond what `webgpu_struct_drawindirect` added (the indirect attribute and
draw):

| area | what |
| --- | --- |
| `src/objects/mesh.rs`, `src/objects/payload.rs` | `Mesh.count`: a plain mesh drawn as `count` instances, which is how `SpriteNodeMaterial` instances without an `InstancedMesh` |
| `src/nodes/tsl.rs` | `rotate_uv()` (`rotateUV`) |
| `src/nodes/builder.rs` | a varying the vertex stage has reassigned carries the reassigned value (see below) |
| `examples/webgpu_particles.rs` | the port |

## What the WGSL found

**The varying `positionLocal` carried the wrong value.** `positionLocal` is a
varying, and `setupPosition()` assigns `positionNode` to it. Three writes every
assignment straight into `varyings.positionLocal`, so the fragment stage reads
the moved position. The port holds the vertex-stage value in a private var
until the fragment stage asks for the varying, and then wrote
`varyings.positionLocal = position`, the geometry's unmoved position. The
smoke's colour reads `positionLocal.y`, so at any time but 0 the port's smoke
was coloured by the quad's own corner rather than by where the sprite had
moved to. The graded frame could not see this: every sprite there is
transparent. The builder now writes the private var's current value when the
vertex stage has assigned to it. No other rung's WGSL changes, because no
earlier rung both reassigns `positionLocal` and reads it in the fragment
stage.

## Divergences

Listed in `docs/nodes.md` §8:

* **One flat `instanceIndex` varying per `range()`, not one in all.** Every
  `tsl::instanced_range` wraps its index in its own varying (§9), so the smoke
  sprite has two flat `u32` varyings holding the same value where three has
  one. Same values, one extra interpolant.
* **`varyings.positionLocal = positionLocal`**, not three's
  `varyings.positionLocal = position` followed by `varyings.positionLocal =
  nodeConst1`. Same final value.
* **`nodeVarN` where three has `let nodeConstN`**, and binding numbers. These
  are the classes §8 already lists.

## Notes

* **Each `range()` is sized by the object it is first set up on.** `lifeRange`
  and `scaleRange` are shared by both materials. The smoke mesh is drawn
  first, with `count = 2000`, so both buffers hold 2000 values and the fire
  reads the first 1000 of them. The port passes the smoke count to every
  shared range for the same reason.
* **`new Inspector()`** is ported only as its five `Math.random()` draws, as
  in `webgpu_tsl_galaxy`.
