# webgpu_postprocessing_lensflare

Branch `display-pages-2`.

**Green at 0 pixels of 100000** (limit 0.1%), steady frame 3.3 ms, 29 draw
calls, 55534 triangles.

## What the score covers

The page draws a space-ship hallway into two MRT attachments: the lit colour
and the emissive term. It blooms the emissive term and mirrors the bloom's
bright spots through the screen centre into ghosts (`lensflare`). It blurs the
ghosts (`gaussianBlur( flarePass, 8 )`), sums the three, and tone-maps with
ACES. The graded frame is the first one. The rung asserts that the flare
target is 200×125 and the blur 800×500 on the 800×500 page.

Each quad the page adds is checked against three's dump in
`tests/nodes_display_wgsl.rs`:

| test | fixture |
|---|---|
| `rtt_matches_three` | `m24` |
| `lensflare_matches_three` | `m25` |
| `lensflare_gaussian_blur_horizontal_matches_three` | `m26` |
| `lensflare_gaussian_blur_vertical_matches_three` | `m27` |
| `lensflare_composite_matches_three` | `m29` |

The bloom's own passes were already gated by
`webgpu_postprocessing_bloom_emissive`.

## What this rung adds

| area | what |
|---|---|
| `src/nodes/display/lensflare.rs` | `LensflareNode.js` |
| `src/objects/scene.rs` | `Scene::background_intensity`, `Scene::environment_intensity` |
| `src/renderer/mod.rs` | both intensities reach their uniforms; `update_texture_source()` |
| `src/nodes/display/gaussian_blur.rs` | the input's update-before runs first |

## Two things the page found

**The scene's intensities were missing.** The page sets
`scene.backgroundIntensity = 2` and `scene.environmentIntensity = 15`. The
uniforms that read them already existed but always held 1. The scene now
carries both. `materialEnvIntensity` takes the scene's value only on a draw
whose environment is the scene's, as in `EnvironmentNode.setup()`.

**The hand-drawn blur sized itself from a 1×1 input.** The port's
`blur_pass.render()` runs before the pipeline. On the first frame, its input (`rtt( flarePass )`)
had not been drawn yet. `GaussianBlurNode::render()` now calls
`Renderer::update_texture_source( map )` first, which runs whatever node
renders the input.

## Divergences

- `gaussianBlur( flarePass, 8 )` is written `Some( vec2( 8.0, 8.0 ) )`.
  Three's `vec2( 8 )` of a plain number is the constant `vec2( 8, 8 )`, and
  `vec2( float( 8 ) )` would be a splat that three's WGSL does not have.
- The two `convertToTexture()` calls are `rtt()`s written in the example.
- `scene.background` and `scene.environment` from the raw equirectangular
  map are written out: the background's cube conversion and the
  environment's PMREM, as in `webgpu_postprocessing_bloom_emissive`.
- `animate()` calls `blur_pass.render()` before `render_pipeline.render()`.
  Three's `render()` calls only `renderPipeline.render()`; the port's
  `GaussianBlurNode` is not one of the nodes the renderer runs on its own, so
  the page draws it by hand.
- The page's GUI is not ported. The values it drives are public on `App`.

## Left out

- `dispose()`.
- The shared `builder.getSharedContext()` given to the flare's material.
