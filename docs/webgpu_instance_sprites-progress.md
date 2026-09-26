# `webgpu_instance_sprites`

Status: **green.** 32 of 100000 pixels against three.js 5f610f5's own
`test/e2e/image.js`, threshold 0.1%. Intel Iris Xe, Mesa 25.3.6, wgpu on
Vulkan. Steady frame 1.0 ms (release), 2 draw calls, 20001 triangles.

One `Sprite` with `count = 10000`. It is a `SpriteNodeMaterial( {
sizeAttenuation: true, map, alphaMap: map, alphaTest: 0.1 } )` with
`positionNode = instancedBufferAttribute( positionAttribute )`,
`rotationNode = time.add( instanceIndex ).sin()` and `scaleNode = uniform( 15 )`,
under `scene.fog = new FogExp2( 0x000000, 0.001 )`. `render()` sets the
colour to `setHSL( h, 0.5, 0.5 )`, and at the pinned time `h` is 0.

## Reconciling with the plans

There is no scout plan. It was ported from
`~/src/vendor/three.js/examples/webgpu_instance_sprites.html`, with three's
WGSL in `target/dumps/webgpu_instance_sprites/` (uncommitted), against
`examples/dump_wgsl.rs`'s `instance_sprites` section.

## What was added

| area | what |
| --- | --- |
| `src/objects/sprite.rs`, `payload.rs` | `Sprite.count`, the draw's instance count |
| `examples/` | `webgpu_instance_sprites.rs`; `dump_wgsl` section `instance_sprites` |

It also relies on `webgpu_sprites`' fog change (§41.2): `FogExp2`'s density
factor now reads the sprite's `v_positionView`, as three's does.

## What the pixels found

The first graded frame passed at 32 pixels. They sit on the rims of the
nearest snowflakes, where the `alphaTest` cut crosses the texture's soft
edge. That is the pattern the other alpha-tested rungs leave.

The page builds its 30000 positions before it constructs the `Inspector`.
The inspector's own `Math.random()` draws therefore come after the positions
and do not shift them; the port reads the first 30000 values of the seeded
sequence. The pointer never moves under the grader, so `mouseX` / `mouseY`
stay 0 and the camera stays on the z axis. The inspector's
`sizeAttenuation` toggle is not ported.

Structurally, the one divergence from the dump beyond §8's classes is the uv
matrix. Three has one `mat3` per `texture()` node (map and alphaMap), and the
port has one per texture (§41).
