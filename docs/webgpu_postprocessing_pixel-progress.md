# webgpu_postprocessing_pixel

Branch `smaa-pixel-pages`, stacked on `ssr-node` (#272).

Status: **ported, not graded on this machine.** Three.js r187 scores 405 of
100000 pixels (0.405%) against its own
`examples/screenshots/webgpu_postprocessing_pixel.jpg` here
(`tools/dump-webgpu.mjs`' `actual.jpg`), over the 0.1% limit. The port scores
417. Graded against three's own frame on this machine, 12 pixels differ.
Intel Iris Xe, Mesa 25.3.6, wgpu on Vulkan. The e2e test is `#[ignore]`d
with that reason. The page is in the native viewer and in the steady-frame
strip: frames two and three build and upload nothing (11 draw calls, 135
triangles).

The scene has two checkered boxes and a bobbing, spinning crystal on a
checkered plane. It is lit by an ambient light, a directional light and a
spot light, all with `BasicShadowMap` shadows. The orthographic camera's
frustum is snapped to the pixel grid (`pixelAlignFrustum`). It is drawn
through `pixelationPass` at a sixth of the drawing buffer, with its depth-
and normal-edge outlines. `PixelationPassNode` was already ported; its WGSL
gate is `webgpu_postprocessing_pixel_m09`. The page needed no node changes
beyond the wrap fix below, plus `Renderer::size()` for `renderer.getSize()`.

## Grade

| | pixels of 100000 |
| --- | --- |
| three.js r187 against its own reference JPEG | 405 |
| this port against the same JPEG | 417 |
| this port against three's frame on this machine (graded scale) | 12 |
| the same at 800x500, > 2 of 255 | 3234 (max 36) |

The remaining difference is all on the crystal, at x 373–444 and y 60–144
of the 800x500 frame. Some of its lit facets are a few levels brighter than
three's, and others a few levels darker. The setup matches the page:

- Phong `0x68b7e9`, emissive `0x4f7e8b` at intensity 0.5 (t = 0), white
  specular, shininess 10;
- casting and receiving shadows;
- the spot light `0xffc100`, 10, distance 10, angle π/16, penumbra 0.02,
  decay 2;
- the directional light at (100, 100, 100) with a 2048² map.

The cause has not been found. Candidates are the self-shadowing of the
crystal's own facets under `BasicShadowMap`, where a texel either side of a
facet edge flips the result, and the spot cone's edge.

## What was fixed

**An unfilterable texture's `textureLoad` honours its wrap mode.**
`NearestFilter` on both filters makes a texture unfilterable, so every tap
becomes `textureLoad`. The port's `textureLoad` always wrapped the uv
through `tsl_coord_clampS_clampT_2d`. The page's checker is
`RepeatWrapping`, repeated 3x on the boxes and 1.5x on the plane, so it
smeared its edge texels across the faces, and 13.4% of the graded frame
differed.
`wgsl::wrap_function_2d` is now three's `generateWrapFunction()`. It builds
the function `tsl_coord_<s>S_<t>T_2d` from the texture's `wrap_s` /
`wrap_t`, with three's `tsl_repeatWrapping_float`,
`tsl_clampWrapping_float` and `tsl_mirrorWrapping_float` helpers. The
clamped case emits the same text as before, and every existing
`textureLoad` gate still passes.

## Not ported

- **`OrbitControls`.** The page attaches them to its `OrthographicCamera`,
  and the port's `OrbitControls` does not handle orthographic cameras. The
  constructor's effect on the camera, pointing it at the origin, is done
  with `lookAt`.
- The GUI. Its defaults are kept: `pixelSize` 6, `normalEdgeStrength` 0.3,
  `depthEdgeStrength` 0.4 and `pixelAlignedPanning` on. The two strengths
  are settable uniforms.
- The checker texture loads synchronously. The page loads it
  asynchronously, and three's screenshot is taken once it is in.
- `scene.add( spotLight.target )` is not repeated. The target stays at the
  origin, as three's does.
