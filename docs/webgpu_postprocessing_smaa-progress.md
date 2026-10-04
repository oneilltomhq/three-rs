# webgpu_postprocessing_smaa

Branch `smaa-pixel-pages`, stacked on `ssr-node` (#272).

Status: **ported, not graded on this machine.** Three.js 5f610f5 (r187dev) scores 258 of
100000 pixels (0.258%) against its own
`examples/screenshots/webgpu_postprocessing_smaa.jpg` here
(`tools/dump-webgpu.mjs`' `actual.jpg`), over the 0.1% limit. The port scores
the same 258. Graded against three's own frame on this machine, no pixel
differs. Intel Iris Xe, Mesa 25.3.6, wgpu on Vulkan. The e2e test is
`#[ignore]`d with that reason, as `webgpu_refraction`'s is. The page is in the
native viewer and in the steady-frame strip: frames two and three build and
upload nothing (6 draw calls, 16 triangles, 36 lines).

The page shows two 120-unit boxes, one a white wireframe and one
brick-textured, turning. They go through a scene pass and `smaa()` (§65 of
`docs/nodes.md`) before `renderOutput`.

## Grade

| | pixels of 100000 |
| --- | --- |
| three.js 5f610f5 (r187dev) against its own reference JPEG | 258 |
| this port against the same JPEG | 258 |
| this port against three's frame on this machine (graded scale) | 0 |
| the same at 800x500, > 2 of 255 | 448 (max 33) |

Three's 258 pixels all lie on the wireframe's diagonal edges. The reference
was rendered on a GPU whose line rasterizer picks different pixels.

## What was fixed

**The input pass now renders inside `resetRendererState()`.** The first port
ran the scene pass before `SMAANode`'s reset, as SSR and TRAA do. It cleared
to the renderer's default `(0, 0, 0, 0)`. Three's input pass is updated
lazily when the edges quad samples it, so it clears to the reset's
`(0, 0, 0, 1)`. Instrumenting three's `Background.update` through
`dump-webgpu.mjs --html` showed `clearAlpha = 1` for that pass.

The difference is invisible on the solid box. It is not invisible on a
wireframe line over an empty background. The blend pass mixes the line with
its transparent neighbours, which leaves the pixel's alpha at the blend
weight. `renderOutput` then unpremultiplies, which divides the colour by
that alpha and brings the pixel back to the line's full brightness. Lines
came out too bright and too hard, and failed against three's own frame
along every edge. With the reorder, `SmaaNode::update_before` saves the state,
resets it, sizes its targets, runs the input's updater and then draws the
three quads. No graded pixel differs.

No WGSL changed: the edges, weights and blend gates in
`tests/nodes_display_wgsl.rs` (from #272) pass unchanged, and
`webgpu_postprocessing_ssr` still scores 4.

## Not ported

- The GUI. Its defaults (`enabled` and `autoRotate`, both on) are the only
  path.
- The brick texture loads synchronously. The page's `TextureLoader.load()`
  is asynchronous, and three's screenshot is taken once it is in.
