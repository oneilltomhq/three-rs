# `webgpu_postprocessing`

Status: **not green, and not the port's to fix.** 107 of 100000 pixels against
three.js 5f610f5's own `test/e2e/image.js`, threshold 0.1% (limit 100). Intel
Iris Xe, Mesa 25.3.6, wgpu on Vulkan. Three itself scores the same 107 on
this machine. The port's 800×500 frame is byte-identical to three's
`actual_full.png` (maximum channel difference 0), so the two renderers fail on
exactly the same pixels. The e2e test is `#[ignore]`d with that reason, like
`webgpu_instance_path`, and the page stays in `steady_frame_builds_nothing`
(93 draw calls, 2186 triangles, nothing built after the first frame). It is
not registered in the viewer, the web shell or the README's graded table.

One hundred flat-shaded Phong spheres in fog, spun by the page's `animate`,
through `rgbShift( rtt( dotScreen( scenePass ) ) )`. The page builds a
`new Inspector()`, which spends five `Math.random()` draws before the scene
takes its 800.

## Reconciling with the plans

This page was dropped once before for "sitting on the 0.1% line". It still
sits there, with a known cause now. `dot_screen` and `rgb_shift` were already
ported and gated against this page's `m03` and `m05`
(`tests/nodes_display_wgsl.rs`). The rung adds only the example and the e2e
test. Porting it also fixed `tools/dump-webgpu.mjs`, which threw a
`ReferenceError` on a stale `pageFile` line before it could dump any page.

## Where the 107 pixels are

They are isolated one- and two-pixel spots on the rims of halftone dots,
scattered across the whole frame (for example (274,10), (268,15) and
(226,245) in the 400×250 graded image), not a region or an object. The dot
screen multiplies the channel average by 10 before it adds the dot pattern,
so a difference of one LSB between the GPU that made the reference and this
one moves a dot's rim by a pixel. Three in Chrome and the port agree to the
bit on this Iris Xe, and both disagree with the reference by the same 107.

Nothing in the port can close that without fitting to the JPEG, which the
rules forbid. The test comes back the day the reference is regenerated or
the page is graded on a GPU that matches it.
