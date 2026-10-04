# webgpu_postprocessing_pixel

Branch `smaa-pixel-pages`, stacked on `ssr-node` (#272).

Status: **ported, not graded on this machine.** Three.js 5f610f5 (r187dev) scores 405 of
100000 pixels (0.405%) against its own
`examples/screenshots/webgpu_postprocessing_pixel.jpg` here
(`tools/dump-webgpu.mjs`' `actual.jpg`), over the 0.1% limit. The port scores
the same 405, and its frame is pixel-identical to three's own frame on this
machine (`actual_full.png`, max channel difference 0).
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
beyond the two fixes below, plus `Renderer::size()` for `renderer.getSize()`.

## Grade

| | pixels of 100000 |
| --- | --- |
| three.js 5f610f5 (r187dev) against its own reference JPEG | 405 |
| this port against the same JPEG | 405 (417 before the fix below) |
| this port against three's frame on this machine (graded scale) | 0 (12 before) |
| the same at 800x500, > 2 of 255 | 0, max 0 (3234, max 36 before) |

The last difference was all on the crystal, at x 373–444 and y 60–144 of
the 800x500 frame: some of its lit facets were a few levels brighter than
three's, and others a few levels darker. The Phong setup matched the page
(`0x68b7e9`, emissive `0x4f7e8b` at 0.5, white specular, shininess 10, the
spot and directional lights as the page sets them).

Candidates were the self-shadowing of the crystal's own facets under
`BasicShadowMap` and the spot cone's edge. It was neither: it was the
specular half-vector. Three's `positionViewDirection` (`Position.js`) is
the constant `vec3( 0, 0, 1 )` when `builder.camera.isOrthographicCamera`,
and three's own dump of this page's Phong fragment shader says so; the port
always used `normalize( -positionView )`, the perspective form. Under this
page's orthographic camera that tilts every view vector by the fragment's
offset from the camera axis, which moves the Blinn-Phong highlight on the
crystal's flat facets.

## What was fixed

**`positionViewDirection` under an orthographic camera.** `SetupContext`
gains `orthographic`, which the renderer sets from the camera a pass is
drawn through: the render camera for the scene and the skybox, the shadow
camera for a planar shadow pass (a directional light's is orthographic),
perspective for the point-light pass, and orthographic for every `QuadMesh`
draw. It is part of the dynamic program key beside `array_cameras`, so an
orthographic and a perspective draw of one material get two programs.
`position_view_direction()` reads it from the build context. The other
graded pages with an orthographic camera (`webgpu_compute_points` 4,
`webgpu_compute_texture` 0, `webgpu_procedural_texture` 0,
`webgpu_texturegather` 16, `webgpu_texturegrad` 0,
`webgpu_tsl_interoperability` 0) and the ungraded `webgpu_camera` (922,
three's own score) score as before.

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

## Merge note

`gtao-denoise` (#267) also replaces `main`'s `wgsl::CLAMP_WRAP_SNIPPET`
with a `NodeBuilder::wrap_function`, but with a different signature
(`wgsl::wrap_function_name` / `wgsl::wrap_function`) and helper strings that
end in `"\n"`, which three's do not. Whichever of the two lands second
should keep this branch's `NodeBuilder::wrap_function` and
`wgsl::wrap_function_2d` (with `CLAMP_WRAP_SNIPPET` gone, as here) and take
only `Texture::wrapping()` from `gtao-denoise`.
