# `webgpu_upscaling_fsr1`

Status: **ported, not graded on this machine.** Three.js 5f610f5 scores 703
of 100000 pixels (0.7%) against its own
`examples/screenshots/webgpu_upscaling_fsr1.jpg` here, in two runs of its
own e2e harness. That is over the 0.1% limit. The port scores 698 against
the same JPEG. Measured on an Intel Iris Xe, Mesa 25.3.6, wgpu on Vulkan.
The e2e test is `#[ignore]`d with that reason, as
`webgpu_postprocessing_retro`'s is. The page is in the steady-frame strip,
where frames two and three build and upload nothing. A frame is 81 draw
calls and 142372 triangles. Headless at 800×500, it takes 13.05 ms (mean of
30 frames after 10 warm-up frames).

The page draws Littlest Tokyo with a scene pass at half resolution (MSAA,
`antialias: true`). `fsr1()` then upscales the result to the canvas, in two
passes: EASU, edge-adaptive Lanczos upsampling, and then RCAS sharpening.

## Grade first

The e2e harness grades at 400×250. The last two rows compare the port's frame
with three's frame on this machine (`tools/dump-webgpu.mjs`' `actual.jpg` and
`actual_full.png`).

| comparison | differing pixels |
| --- | --- |
| three.js 5f610f5 vs its own reference JPEG (two runs) | 703 of 100000, both times |
| this port vs the same JPEG | 698 of 100000 |
| this port vs three's screenshot on this machine, by three's compare | 0 of 100000 |
| this port vs three's frame on this machine, 800×500, > 2 of 255 | 690 of 400000 (max 31; 31 over 10) |

Three's diff image and the port's mark the same pixels: single edge pixels
scattered over the model's fine detail (window frames, wires, foliage). The
reference was captured on another GPU, whose MSAA coverage of the
half-resolution scene differs. EASU upscales those edge pixels and RCAS
sharpens them. Against three's own frame here, the port passes three's
compare with no pixel over its threshold.

## What is checked

- **The two FSR shaders**, against three's dump of the page, in
  `tests/nodes_display_wgsl.rs`:
  - `fsr1_easu_matches_three` (`m36`, `FSR1_EASU`): the 12-tap footprint,
    the four quadrants' edge direction and length, the kernel shaping and
    the twelve Lanczos2 accumulations, clamped to the nearest four texels.
  - `fsr1_rcas_matches_three` (`m38`, `FSR1_RCAS`): `SharpenNode`'s RCAS
    over the EASU target, with the `vec2( textureSize() )` lookup.

  The fingerprint is the multiset of calls and float literals plus the
  if/else/for sequence, as in the other display gates. `denoise` changes
  only the literal in `false == true`, so it has no dump of its own.
- **The node's frames**, on the GPU, in `tests/fsr1_frames.rs`. A hard
  vertical grey edge goes through a half-resolution `pass()` into
  `Fsr1Node`. The test checks:
  - both targets are at canvas size and the pass is at half of it;
  - flat regions keep their colour;
  - the edge's steepest step per row beats a bilinear upscale of the same
    pass texture: 3086 summed against 1764, with EASU alone at 2606;
  - EASU alone (sharpness 30) stays inside the edge's range;
  - both targets follow a resize;
  - the free `fsr1()` of a plain colour returns that colour.
- **The page**: `examples/webgpu_upscaling_fsr1.rs` renders the page's
  first frame and runs in the viewer (`viewer upscaling_fsr1`).

## What was added

| area | what |
| --- | --- |
| `src/nodes/display/fsr1.rs` | `fsr1`, `Fsr1Node` |
| `src/nodes/display/sharpen.rs` | `rcas()` shared with FSR 1, with a flag for the `vec2` size form |

`docs/nodes.md` §91 is the long form.

## What is left out

- The GUI: `upscaleMethod` (`'Bilinear'` outputs `scenePass` instead) and
  the `resolutionScale` slider. Their defaults are ported, through the
  page's own `updatePipeline()`.
- `toInspector()` and the `Inspector`.
- `FSR1Node`'s `contextNode = context( builder.getSharedContext() )` and
  `dispose()`.

The page is not in the browser shell. The shell builds only the README's
graded examples, and the web gate grades every one it builds (see
`web/README.md`, "Adding an example").
