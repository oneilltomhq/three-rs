# `webgpu_cubemap_mix`

Status: **green.** 1 of 100000 pixels against three.js 5f610f5's own
`test/e2e/image.js`, threshold 0.1%. Intel Iris Xe, Mesa 25.3.6, wgpu on
Vulkan. Steady frame 6.4 ms (debug e2e ladder; not yet measured in release), 3 draw calls, 17437 triangles.

DamagedHelmet lit only by `scene.environmentNode = mix( pmremTexture( cube2 ),
pmremTexture( cube1 ), oscSine( time.mul( .1 ) ) )`, which cross-fades the
Milky Way and the Pisa HDR cube. The same node, read at a fixed roughness of
0.5 through `.context( { getTextureLevel } )`, is the background.
`LinearToneMapping`.

## Reconciling with the plans

There is no scout `PLAN.md` for this rung. It was ported from
`~/src/vendor/three.js/examples/webgpu_cubemap_mix.html` directly. Three's
WGSL was dumped into `target/dumps/webgpu_cubemap_mix/` (not committed).
`m08` (the background) and `m10` (the helmet) were diffed against
`dump_wgsl`'s new `cubemap_mix_background` and `cubemap_mix_material`.

## What was added

| area | what |
| --- | --- |
| `src/materials/environment.rs` | `EnvironmentNode` (a graph of `pmremTexture()` reads, as a function of the context's UV and level; `with_texture_level()` is `.context( { getTextureLevel } )`), and `Environment`, which is `Pmrem` or `Node`, taken by `setup()` |
| `src/objects/scene.rs` | `Scene::environment_node`, `Background::EnvironmentNode` |
| `src/materials/node_material.rs`, `src/renderer/mod.rs` | `SetupContext::environment` becomes an `Environment`; `scene.environmentNode` wins over `scene.environment`; `background_environment_color_node()` |
| `examples/` | `webgpu_cubemap_mix.rs`; `dump_wgsl`'s `dump_cubemap_mix()` |

`docs/nodes.md` §54.5–§54.7 is the long form.

## What the pixels found

The first render graded at 1 pixel. The pinned clock puts `oscSine` at 0,
so the frame is entirely `cube2`, the Milky Way. The Pisa cube's PMREM is
generated and sampled at weight 0, and nothing in the grade checks the
blend itself. The dump does: both leaves, the `mix` and the `sin` are
three's, in three's order.

## Not ported

Nothing the page uses. The page never calls `controls.update()`, so the
camera keeps the orientation the constructor's `update()` gave it, as it
does here.
