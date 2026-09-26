# `webgpu_sprites`

Status: **green.** 0 of 100000 pixels against three.js 5f610f5's own
`test/e2e/image.js`, threshold 0.1%. Intel Iris Xe, Mesa 25.3.6, wgpu on
Vulkan. Steady frame 13.8 ms (release), 201 draw calls, 401 triangles.

Two hundred `Sprite`s on a sphere of radius 500, in one `Group`, sharing one
`SpriteNodeMaterial`. The colour is `texture( map ).mul( uv() ).mul( 2 )
.saturate()`, the opacity is the texture's alpha, and the rotation is
`userData( 'rotation', 'float' )`, each sprite's own `userData.rotation`. The
scene has `fogNode = fog( color( 0x0000ff ), rangeFogFactor( 1500, 2100 ) )`.

## Reconciling with the plans

There is no scout plan. It was ported from
`~/src/vendor/three.js/examples/webgpu_sprites.html` directly. Three's WGSL
was dumped by `tools/dump-webgpu.mjs` into `target/dumps/webgpu_sprites/`
(uncommitted) and diffed against `examples/dump_wgsl.rs`'s new `sprites`
section (three's `m01` / `m02`). The dump tool itself had a stray line from a
merge that failed every run, and the first commit on the branch removes it.

## What was added

| area | what |
| --- | --- |
| `src/core/object3d.rs` | `Object3D.user_data`, three's `userData` |
| `src/nodes/tsl.rs` | `user_data( name, ty )`, `UserDataNode` on the object-update uniform path |
| `src/nodes/tsl.rs`, `src/materials/node_material.rs` | fog factors built inside the material's `positionView` scope |
| `examples/` | `webgpu_sprites.rs`; `dump_wgsl` section `sprites` |

`docs/nodes.md` §41 is the long form.

## What the pixels found

The first graded frame was already at 0 pixels, before the WGSL matched. The
dump showed why that proved less than it seemed. Three's fragment fogs by
`v_positionView.xyz.z`, the sprite's own billboarded `vec4` varying. The
port's fog factor had been built when the fog node was made, outside the
material. So it read the base class' `modelViewMatrix * positionLocal`, a
second `vec3` varying of the un-billboarded quad. At the pinned time
(`Date.now() = 0`) the group has no rotation, so the two depths agree on
every pixel. They would part as soon as the group turned. The fix (§41.2)
makes the factor an inline `Fn()` that the material's setup builds. After it,
the sprite shaders carry one `v_positionView`, as three's do, and every
existing fog rung keeps its pixel count.

The page's texture callback sets `imageWidth` / `imageHeight`, and `render()`
scales every sprite by them. The grader's single RAF fires after the network
is idle, so the graded frame always has the image's size (128 x 128). The
port loads synchronously and takes the size in `init()`.

Frame one builds 201 programs, one per draw, and keeps 2, as
`webgpu_instance_uniform` does with its twelve meshes. Frames two and three
build and create nothing; only the object-group bytes move.
