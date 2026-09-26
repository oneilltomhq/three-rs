# `webgpu_texturegrad`

Status: **green.** 0 of 100000 pixels against three.js 5f610f5's own
`test/e2e/image.js`, threshold 0.1%. Intel Iris Xe, Mesa 25.3.6, wgpu on
Vulkan. Steady frame 2.2 ms (release, `viewer --headless`), 4 draw calls, 6
triangles.

A unit plane of `uv_grid_opengl.jpg` under an orthographic camera, drawn
twice side by side — the page's WebGPU canvas and its WebGL one. The
`colorNode` is an inline `Fn()` that averages four `textureSampleGrad` taps
around the uv, offset by `blur = pow( ( 0.0625 - cos( 20 uv.x + time ) ) *
0.0625, 2 )`, with `blur` as both gradients in the top half and zero in the
bottom half, and a white line at the seam. The grader pins `time` to 0.

## Reconciling with the plans

There is no scout `PLAN.md` for this rung. It was ported from
`~/src/vendor/three.js/examples/webgpu_texturegrad.html`, with three's WGSL
dumped by `tools/dump-webgpu.mjs` into `target/dumps/webgpu_texturegrad/`
(uncommitted). The dump tool itself needed one line removed: a merge had left
a duplicate `pageBody` read of an undefined `pageFile`, which threw before any
page loaded.

## What was added

| area | what |
| --- | --- |
| `src/nodes/node.rs`, `builder.rs` | `SampleMode::Grad( grad_x, grad_y )`: the two gradients are nodes, built as `vec2`, in place of two baked zero constants |
| `src/nodes/tsl.rs` | `texture_grad( map, uv, grad_x, grad_y )` |
| `examples/` | `webgpu_texturegrad.rs`; a `dump_wgsl` section `texturegrad` |
| `tests/` | `nodes_texture_wgsl.rs::texturegrad_fragment_matches_three` against three's verbatim `m02` |

`docs/nodes.md` §36 is the long form.

## What the pixels found

Nothing to chase: the first frame graded 0 pixels. The page's two canvases
are two `WebGPURenderer`s, one on the WebGL backend; the port renders both
halves of one canvas through the viewport and the scissor with the WGSL path
(§36), and the right half matches the WebGL canvas to the grader's
threshold.
