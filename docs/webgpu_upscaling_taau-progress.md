# `webgpu_upscaling_taau`

Status: **ported, not graded on this machine.** Three.js 5f610f5 scores 540
of 100000 pixels (0.5%) against its own
`examples/screenshots/webgpu_upscaling_taau.jpg` here, over the 0.1% limit.
Two runs of three's own e2e harness both fail it at 0.5%, and 540 is the
count for `tools/dump-webgpu.mjs`' `actual.jpg`. The port scores 539 against
the same JPEG. Measured on an Intel Iris Xe, Mesa 25.3.6, wgpu on Vulkan.
The e2e test is `#[ignore]`d with that reason, as `webgpu_postprocessing_retro`'s
is.

The page is in the steady-frame strip, held to zero from its third frame.
Frame one draws into TAAU's 1×1 previous-depth target and then resizes it to
the pass's depth, as three does, so frame two makes one texture view and one
bind group for the resized texture. Frame three builds and creates nothing.
A steady frame is 81 draw calls and 142372 triangles; frame one has one more
of each, the history seed. Headless at 800×500 it takes 13.17 ms (mean of 30
frames after 10 warm-up frames; max 18.69 ms).

The page draws the animated Littlest Tokyo model, lit by a dim
`RoomEnvironment`, at half the canvas into an `output` + `velocity` MRT.
`taau()` upsamples it to the full canvas, and an RCAS `sharpen` at 0.2
sharpens the result.

## Grade first

The e2e harness grades at 400×250. The last row compares the two frames at
full size.

| comparison | differing pixels |
| --- | --- |
| three.js 5f610f5 vs its own reference JPEG | 540 of 100000 |
| this port vs the same JPEG | 539 of 100000 |
| this port vs three's frame on this machine, 800×500, > 2 of 255 | 25 of 400000 (max 12) |

Against the reference, the diff is along the model's thin wires, poles and
railings, which are drawn at 400×250 before upsampling. The reference was
captured on another GPU, which rasterises them differently. Against three's
frame on this machine, 25 pixels differ by more than 2 of 255. The largest,
12, are in two small clusters around ( 613, 129 ) and ( 643, 132 ); the rest
are isolated pixels of 3 to 5.

## What is graded: the first frame

Three registers TAAU's pipeline callbacks while the first
`renderPipeline.render()` builds the output, after that frame's before
callbacks have run. So the graded frame is unjittered: the jitter uniform is
still `( 0, 0 )` and only the after callback runs. The history is the
`TAAU.seed` bilinear upscale of the beauty with a zero lock. The previous
depth is unwritten and the previous camera matrices are the identity, so the
background counts as disoccluded and takes the 9-tap reconstruction. Most
model pixels keep the seed, clipped to the neighbourhood's variance.
`TaauNode::attach` installs its hooks up front and skips the first before
call, so the port renders the same frame.

The mixer is updated with `timer.getDelta()`, 0 on the graded frame, so the
model is posed at the clip's first keyframe.

## What is checked

- **The two TAAU quads**, against three's dump of the page, in
  `tests/nodes_display_wgsl.rs`:
  - `taau_seed_matches_three` (`m36`).
  - `taau_resolve_matches_three` (`m38`, the whole `main()`), with
    `taau_clip_aabb_matches_three` and `taau_flicker_reduction_matches_three`
    on its two layout functions.

  The page's RCAS quad (`m40`) is byte-identical to the `sharpen_rcas`
  fixture, so it is not gated twice.
- **The node over frames**, on the GPU, in `tests/taau_frames.rs`. A white
  box at half of a 48×48 canvas:
  - The output is 48×48 while the pass is 24×24.
  - The first frame matches a plain bilinear resolve of the same pass,
    except for at most 16 pixels within 3 px of the silhouette.
  - 47 more frames of a static scene converge (the last step moves no
    pixel by more than 8). The inside stays white, the background black,
    and the blended pixels sit on the silhouette.
  - When the box moves for three frames, the region it vacated holds no
    ghost.

## Differences from the page

- **`sharpen( taauNode.getTextureNode(), params.sharpness )`** is
  `SharpenNode::new( &taau.texture(), float( 0.2 ), false )`. Three passes a
  number, which is a constant in its shader, so the GUI's
  `sharpenNode.sharpness.value = …` has no effect. `sharpen()` would also put
  an `RTTNode` around a texture node that three's `convertToTexture()`
  passes through.
- **One scene.** Three replaces `scene` after `renderer.init()`, and the
  model's load callback adds it to the replacement, the one with the
  background and environment. The port builds only that scene.
- **No GUI.** The bilinear/TAAU switch, the resolution-scale slider and
  the sharpening controls are not ported. The page's initial state is.
