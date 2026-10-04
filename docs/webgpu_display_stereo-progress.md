# `webgpu_display_stereo`

Status: **green.** 0 of 100000 pixels against three.js 5f610f5's own
`test/e2e/image.js`, threshold 0.1%. Intel Iris Xe, Mesa 25.3.6, wgpu on
Vulkan. Steady frame 4.6 ms (release), 5 draw calls (each eye's background
and instanced spheres, then the output quad), 963969 triangles.

The page renders 500 instanced spheres of radius 0.1 (one
`MeshBasicNodeMaterial` with the Park3Med cube as its env map) in front of
the same cube as the background. The camera is at `z = 3` with a 60° field of
view. Each frame moves every sphere to `x = 5cos(t + i)` and
`y = 5sin(t + 1.1i)`, keeping its random depth and scale. The page's default
effect, which is the one graded, is `stereoPass`: the scene rendered twice,
side by side, through a `StereoCamera` with `eyeSep = 0.064`. The GUI also
switches the output to `anaglyphPass` or `parallaxBarrierPass`.

## Reconciling with the plans

There is no scout plan. The page was ported from
`~/src/vendor/three.js/examples/webgpu_display_stereo.html` and the four
nodes in `examples/jsm/tsl/display/`, plus `src/cameras/StereoCamera.js` and
`examples/jsm/utils/CameraUtils.js`. Three's WGSL was dumped by
`tools/dump-webgpu.mjs`, once per effect. The page's effect is a GUI choice,
so the anaglyph and parallax-barrier dumps come from copies of the page that
start in that effect: `tools/dump-pages/display_stereo_anaglyph.html` and
`display_stereo_parallax_barrier.html`. The two composite quads (`m06` of
those dumps) are committed verbatim under `tests/fixtures/nodes_display/`.
`stereoPass` has no quad of its own.

## What was added

| area | what |
| --- | --- |
| `src/cameras/stereo_camera.rs` | `StereoCamera`: `aspect`, `eyeSep`, the two eyes on layers 1 and 2, the seven-value projection cache, `update( camera )` |
| `src/addons/camera_utils.rs` | `frame_corners`, `CameraUtils.frameCorners()` |
| `src/nodes/display/stereo_pass.rs` | `stereo_pass` / `StereoPassNode` |
| `src/nodes/display/stereo_composite_pass.rs` | `StereoCompositePassNode` as the crate-private `CompositeState`, and the parts of `RendererUtils.resetRendererState()` / `restoreRendererState()` a stereo pass needs |
| `src/nodes/display/anaglyph_pass.rs` | `anaglyph_pass` / `AnaglyphPassNode`, `AnaglyphAlgorithm`, `AnaglyphColorMode`, and `anaglyph_matrices()`, the 21-entry matrix table |
| `src/nodes/display/parallax_barrier_pass.rs` | `parallax_barrier_pass` / `ParallaxBarrierPassNode` |
| `src/renderer/pass.rs` | `PassNode::render_with`, crate-private: the pass's own render bracket around a caller's renders |
| `examples/` | `webgpu_display_stereo.rs`, which takes `anaglyph` or `parallax_barrier` as a second argument; the viewer and web shell entries; a manifest |
| `tests/` | `cameras_stereo_camera.rs` against `tools/stereo_camera_reference.mjs`'s fixture; the two quads in `nodes_display_wgsl.rs`; `stereo_frames.rs`; the `webgpu_display_stereo` rung |

`docs/nodes.md` §81 is the long form.

## What the pixels found

The first graded frame was 0 pixels. The CPU fixture test came first:
three's `StereoCamera`, `frameCorners` and `AnaglyphPassNode`, run in node.
Once the eyes matched to 1e-12, the frame matched as well.

The two effects that are not graded were compared by hand against three's
own full-size renders from the dump runs (800 × 500):

| effect | max channel difference | pixels more than 2 levels off | mean |
| --- | --- | --- | --- |
| stereo (graded) | 5 | 69 | 0.22 |
| anaglyph | 12 | 3831 | 0.23 |
| parallax barrier | 3 | 72 | 0.22 |

The pixels more than 2 levels off sit in busier neighbourhoods than the
frame as a whole. For the anaglyph, the median 3×3 luminance range in three's
render is 33 levels around those pixels, against 10 over the whole frame.
The Dubois matrices' rows sum to more than 1 for the eye that drives each
channel, so a small difference in either eye grows in the mix. These runs
were not graded; they were compared by hand once.

## What is not ported

- The inspector GUI. The example exposes its `onChange` handlers as
  `set_effect`, `set_eye_sep`, `set_anaglyph_algorithm`,
  `set_anaglyph_color_mode` and `set_plane_distance`.
- `contextNode` on the composite quads, the full renderer-state save, and
  `dispose()` (§81.5).
- `Timer.connect( document )`. Page visibility does not pause the native
  clock.
