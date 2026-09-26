# `webgpu_postprocessing_afterimage`

Status: **ported, not graded on this machine.** The port's frame is
pixel-identical to three.js 5f610f5's own frame for the page (0 differing
pixels, max channel difference 0, against `tools/dump-webgpu.mjs`'
`actual_full.png`). But three itself scores 521 of 100000 pixels (0.5%)
against its own `examples/screenshots/webgpu_postprocessing_afterimage.jpg`
here, both through `tools/dump-webgpu.mjs` and through three's unchanged
`test/e2e/puppeteer.js --webgpu`, so the port scores the same 521. Intel Iris
Xe, Mesa 25.3.6, wgpu on Vulkan. The e2e test is `#[ignore]`d with that
reason, as `webgpu_textures_anisotropy`'s is, and the page sits in the
steady-frame strip, which checks that frames two and three build and upload
nothing.

The page draws 50000 additive `SpriteNodeMaterial` sprites, 2 units across,
placed on a sphere and spiralled by `instancedBufferAttribute` time offsets,
through one scene pass and `afterImage( scenePass, damp )`. The graded frame is
the first, so `AfterImageNode`'s old target is still zero and the composite is
`max( scene, 0 )`.

## Grade first

| | pixels of 100000 |
| --- | --- |
| three.js 5f610f5 against its own reference JPEG | 521 |
| this port against the same JPEG | 521 |
| this port against three's frame on this machine | 0 (max channel difference 0) |

The differing pixels are scattered single pixels over the particle cloud,
densest around the bright core: which sprites cover which half-scaled pixel,
and how their additive sums round. That is the GPU the reference was rendered
on. Nothing in the port can close the gap without deriving from the
reference.

## What was added

| area | what |
| --- | --- |
| `examples/` | `webgpu_postprocessing_afterimage.rs` |
| `tests/e2e/main.rs` | the ignored grade and the steady-frame strip entry |

Nothing in `src/`: `AfterImageNode` (`src/nodes/display/after_image.rs`)
already matched three's current `AfterImageNode.js` line for line, and its
composite shader was already checked against this page's dump by
`tests/nodes_display_wgsl.rs`. The page is its first user in an example.
The inspector's five `Math.random()` draws come before the particle loop here
(`new Inspector()` is created before the geometry), so the example skips them
first, as `webgpu_postprocessing_direct` does.
