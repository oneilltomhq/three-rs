# webgpu_postprocessing_ao

Branch `gtao-denoise`.

**Not graded, and it cannot be.** three lists `webgpu_postprocessing_ao` in
its own e2e exception list (`test/e2e/puppeteer.js`, under "Black screen"),
with `webgpu_postprocessing_traa`, whose note explains why a port is not
graded against a reference upstream does not grade itself against.

## What is checked instead

- **The shader.** The `GTAO` quad material, compared with three's dump of
  this page: `gtao_matches_three` checks the fragment body and its bindings
  (including the sampler the depth gather binds; that it is a
  non-filtering one is the layout's doing, in `programs.rs`, which the
  dump does not show), and
  `gtao_screen_position_from_clip_matches_three` checks the one `Fn()` with
  a layout (`tests/nodes_display_wgsl.rs`, fixture
  `webgpu_postprocessing_ao_m18_gtao.wgsl`).
- **The noise.** `generate_magic_square( 5 )` is held to three's numbers in
  a unit test.
- **The frames.** `tests/gtao_frames.rs`, on the GPU: a white box on a white
  floor, 64×64, with the AO texture as the pipeline's output. The sky keeps
  the white clear, the open floor is unoccluded, the foot of the box is
  dark, half resolution (the `textureGather` path) and a rebuilt sample
  count with temporal filtering still put the contact dark, nothing is NaN,
  and with `builtinAOContext` a Standard material's beauty is darker at the
  contact and unchanged in the open.
- **The page.** `examples/webgpu_postprocessing_ao.rs` builds the page's
  initial `GTAO` state and runs in the viewer (`viewer ao`, or
  `--headless --frames 40`).

## What is left out

`docs/nodes.md` §64.3 lists them: a normal reconstructed from depth, a
logarithmic depth buffer, the unread `distanceExponent` / `distanceFallOff`
uniforms, `SSAONode` and the `aoOnly` view, and a per-pass sample count.
The page's GUI, its transparent-mesh toggle and `scene.environmentIntensity`
(folded into the environment node as a factor of 0.3) are in the example's
module docs.

The page is not in the browser shell. The shell builds only the README's
graded examples, and the web gate grades every one it builds (see
`web/README.md`, "Adding an example").
