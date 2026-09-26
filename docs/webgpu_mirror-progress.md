# `webgpu_mirror`

Status: **green.** 22 of 100000 pixels against three.js' own
`test/e2e/image.js`, threshold 0.1%. Intel Iris Xe, Mesa 25.3.6, wgpu on
Vulkan. Steady frame STEADY ms, CALLS draw calls, TRIS triangles.

A Cornell-style box of six Phong planes around a half sphere and a
flat-shaded icosahedron, lit by four point lights. The floor and the back wall
are `reflector()`s. The floor's reflection is mixed in under a decal's alpha,
and the wall's is added to a dim blue. Both are sampled at
`screenUV.flipX()`, offset by a normal map.

## Reconciling with the plans

There is no scout plan for this rung. It was ported from
`~/src/vendor/three.js/examples/webgpu_mirror.html` and
`src/nodes/utils/ReflectorNode.js` directly. Three's WGSL and command stream
were dumped by `tools/dump-webgpu.mjs` into `target/dumps/webgpu_mirror/`
(uncommitted) and diffed against `examples/dump_wgsl.rs`'s new
`mirror_vertical` and `mirror_ground` sections (three's `m06` and `m08`).

## What was added

| area | what |
| --- | --- |
| `src/nodes/reflector_node.rs` | `reflector( parameters )`, `ReflectorNode` (its `uvNode`, `target`), `ReflectorBaseNode`'s state |
| `src/renderer/reflector.rs` | `ReflectorBaseNode.updateBefore()`: the virtual camera, the oblique projection, the nested render |
| `src/renderer/mod.rs` | the pre-pass walk in `render()`, per-item texture overrides in `draw()`, nested-render bookkeeping |
| `src/nodes/tsl.rs` | `flipX()` |
| `src/cameras/` | `RenderCamera::id()` |
| `src/objects/payload.rs` | `material_mut()`, for `material.visible = false` around the nested render |
| `examples/` | `webgpu_mirror.rs`; `dump_wgsl` sections `mirror_vertical`, `mirror_ground` |

`docs/nodes.md` §55 is the long form.

## What the pixels found

**Five renders, nested the way three nests them.** With `bounces: true`
each mirror also renders inside the other's render. Three's dump is the main
pass, `floor( V1 )` containing `wall( V2 )`, and `wall( V3 )` containing
`floor( V4 )`, each nested pass submitted first. The subtle part is which
target each draw binds. Three binds `textureNode.value` as it is when it
records the draw, and a later nested render can move it. The port runs every
update before it records the pass and gives each item the values as they
stood right after its own update. The frame came out green on its first
grade.

**The reflector has to outlive the page's handle.** On the first run, no
reflector ever fired and the draw panicked on an unallocated default target.
The page's `ReflectorNode` goes out of scope at the end of `init()`. In three
the material graph keeps the base node alive through the `ReflectorNode`;
here the graph holds only a texture node, so the registry now holds the
reflector strongly. It lets go once nothing else holds that default texture.

The 22 remaining pixels are isolated single pixels, most of them on the
red wall's edge and around the rim of the back mirror, where the normal-map
offset lands a sample on a reflected edge. They were not chased further.
