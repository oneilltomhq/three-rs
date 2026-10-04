# `webgpu_postprocessing_retro`

Status: **ported, not graded on this machine.** Three.js 5f610f5 scores 1503
of 100000 pixels (1.5%) against its own
`examples/screenshots/webgpu_postprocessing_retro.jpg` here
(`tools/dump-webgpu.mjs`' `actual.jpg`), over the 0.1% limit. The port also
scores 1503 against the same JPEG. Measured on an Intel Iris Xe, Mesa 25.3.6,
wgpu on Vulkan. The e2e test is `#[ignore]`d with that reason, as
`webgpu_refraction`'s and `webgpu_postprocessing_afterimage`'s are. The page
is in the steady-frame strip, where frames two and three build and upload
nothing. A frame is 6 draw calls and 7690 triangles. Headless at 800×500, it
takes 1.91 ms (mean of 30 frames after 10 warm-up frames).

The page shows `coffeeMug.glb` on a textured table, with a smoke column
rising from the mug, under a PS1 night sky. The retro pass draws it all at a
quarter of the canvas. The CRT stack then post-processes the result:

- barrel distortion
- colour bleeding that grows toward the edges
- Bayer dithering
- posterize
- vignette
- scanlines

## Grade first

The e2e harness grades at 400×250. The last row compares the two frames at
full size.

| comparison | differing pixels |
| --- | --- |
| three.js 5f610f5 vs its own reference JPEG | 1503 of 100000 |
| this port vs the same JPEG | 1503 of 100000 |
| this port vs three's frame on this machine, 800×500, > 2 of 255 | 4243 of 400000 (max 49) |

All 4243 differing pixels are on the JPEG-textured mug and table. The sky,
the smoke and the CRT overlay match. The mug's and the table's textures are
JPEGs. The port decodes them with zune-jpeg and Chrome uses libjpeg-turbo, so
a texel can differ by a few levels. After nearest sampling, dithering and a
32-step posterize, a few levels can flip a pixel to the next step. This
explanation fits where the pixels are, but it is not proven: decoding both
JPEGs with the same decoder has not been tried.

## What is checked

- **The two post-processing shaders**, against three's dump of the page,
  in `tests/nodes_display_wgsl.rs`:
  - `retro_barrel_matches_three` (`m08`): the `RTT` that `colorBleeding()`
    makes of the retro pass, read through
    `replaceDefaultUV( barrelUV( curvature ) )`. It is a clamped
    `textureLoad` of the nearest-filtered target.
  - `retro_crt_matches_three` (`m10`): the pipeline's output. That is the
    bleed taps, `bayerDither`, `posterize`, `vignette` and `scanlines`,
    under the sRGB output transform.

  The fingerprint is the multiset of calls and float literals plus the
  if/else/for sequence, as in the other display gates.
- **The pass's frames**, on the GPU, in `tests/retro_frames.rs`. The test
  draws a 64×64 canvas: a checker-textured plane in front of a background
  of `normalWorld.z`. It checks:
  - the pass's target is 16×16;
  - every 4×4 block of the output is one colour, and a plain `pass()` of
    the same scene is not;
  - the sky is bright at all four edges, which it is only if the
    background's `normalWorld` is the back-side one (a mutation that drops
    the deferral fails this);
  - the plane is pure black and white at level 0. With
    `set_filter_textures( true )` it is grey from the mipmaps, and turning
    the flag off again gives back the first frame exactly;
  - on a tilted plane, `affine_distortion = 1` moves at least 64 pixels
    against `0`, stays blocky, and `0` gives back the
    perspective-correct frame exactly.
- **The mug's material**: `coffee_mug_unlit_material` in
  `tests/gltf_loader.rs`. `KHR_materials_unlit` gives a double-sided Basic
  material with an sRGB map and nothing else.
- **The page**: `examples/webgpu_postprocessing_retro.rs` renders the
  page's first frame and runs in the viewer
  (`viewer postprocessing_retro`).

## What was added

| area | what |
| --- | --- |
| `src/nodes/display/retro_pass.rs` | `retro_pass`, `RetroPassNode`, `RetroPassOptions` |
| `src/nodes/display/crt.rs`, `shape.rs`, `bayer.rs` | `barrel_uv`, `barrel_mask`, `color_bleeding`, `scanlines`, `vignette`, `circle`, `bayer_dither` |
| `src/nodes/display/film.rs`, `sepia.rs`, `bleach_bypass.rs` | `film`, `sepia`, `bleach` (not used by this page; `docs/nodes.md` §80) |
| `src/renderer/` | `RenderObjectFunction`, `PassNode::set_resolution_scale` |
| `src/materials/` | `context_node`; a deferred inline-`Fn` background |
| `src/nodes/` | `texture()` honours `getUV` / `getTextureLevel`; `replace_default_uv`; `textureSampleLevel( …, 0 )` outside the fragment stage |
| `src/loaders/gltf_loader.rs` | `KHR_materials_unlit` |

`docs/nodes.md` §79 is the long form.

## What is left out

`docs/nodes.md` §79.3 lists them. In short:

- the GUI and the Damaged Helmet model it switches to, with
  `venice_sunset_1k.hdr`;
- the retro material's standard + `envMap` reflection branch, which only
  the helmet takes;
- the per-material cache;
- `bayer16`;
- the CRT functions' parameter defaults.

The page is not in the browser shell. The shell builds only the README's
graded examples, and the web gate grades every one it builds (see
`web/README.md`, "Adding an example").
