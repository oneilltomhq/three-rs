# `webgpu_equirectangular`

Status: **green.** 0 of 100000 pixels against three.js 5f610f5's own
`test/e2e/image.js`, threshold 0.1%. Intel Iris Xe, Mesa 25.3.6, wgpu on
Vulkan. Steady frame 3.6 ms, 2 draw calls,
1985 triangles.

The page is a 4096×2048 sRGB panorama set as
`scene.backgroundNode = texture( equirectTexture, equirectUV(), 0 )`, with
no mesh and no light. `OrbitControls.autoRotate` has turned the camera 0.1°
about the origin by the graded frame. The constructor's own `update()` runs
before `autoRotate` is set, and `animate()`'s `update()` runs with a null
delta.

## Reconciling with the plans

There is no scout plan. The page was ported from
`~/src/vendor/three.js/examples/webgpu_equirectangular.html`, and three's
WGSL was dumped into `target/dumps/webgpu_equirectangular/` (uncommitted).
`equirect_uv` and `texture_level` were already ported, for the PMREM
generator's equirect material. The Inspector and its
`backgroundIntensity` slider draw nothing into the canvas and are not
ported. The graded intensity is the default 1.

## What was added

| area | what |
| --- | --- |
| `src/materials/node_material.rs` | `background_node_color_node` keeps a vec4 node as it is (`vec4( node )`) |
| `examples/` | `webgpu_equirectangular.rs`; `dump_wgsl` section `background_equirect` |

`docs/nodes.md` §48 is the long form.

## What the pixels found

Nothing to chase. The first frame that compiled graded at 0 pixels. The
background's vertex and fragment modules match three's `m01`/`m02`
structurally: `textureSampleLevel` at level 0 of
`equirectUV( positionWorldDirection )`, times `backgroundIntensity`.
