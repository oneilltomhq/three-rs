# webgpu_postprocessing_dof

Branch `dof-node`.

**Not graded, and it cannot be.** three lists `webgpu_postprocessing_dof` in
its own e2e exception list (`test/e2e/puppeteer.js`, under "Black screen").
three's harness does not grade the page. `examples/screenshots/webgpu_postprocessing_dof.jpg`
does exist, but three's CI does not hold its page to it. This is the same
position as `webgpu_postprocessing_traa`.

## What is checked instead

- **Every shader the node draws.** `DepthOfFieldNode`'s nine draws use
  seven distinct quad shaders (blur64 and blur16 each serve both fields).
  `tests/nodes_display_wgsl.rs` compares all seven with three's dump of this
  page (`node tools/dump-webgpu.mjs webgpu_postprocessing_dof`), with the
  same fingerprint as the other display gates: calls, float literals,
  control flow and bindings. The seven fixtures are
  `webgpu_postprocessing_dof_m*.wgsl`, copied verbatim from the dump.

  | dump | quad | test |
  |---|---|---|
  | `m04` | the CoC, written to two red attachments through `outputStruct( near, far )` | `dof_coc_matches_three` |
  | `m06` | the near field's horizontal Gaussian: `float` taps of the near CoC through its uv matrix | `dof_coc_gaussian_horizontal_matches_three` |
  | `m07` | the vertical Gaussian: `vec4` taps of the horizontal pass's RGBA target, no uv matrix, direction `( 0, 1 )` | `dof_coc_gaussian_vertical_matches_three` |
  | `m08` | the blurred near CoC into the half-resolution target | `dof_coc_blurred_matches_three` |
  | `m10` | `blur64`, the 64-tap Vogel gather | `dof_blur64_matches_three` |
  | `m11` | `blur16`, the 16-tap max filter | `dof_blur16_matches_three` |
  | `m12` | the composite | `dof_composite_matches_three` |

  The fingerprint does not see member types, swizzles or constructors, so
  two string checks sit beside it: `dof_coc_writes_two_f32_members` (the
  CoC's `OutputType` has two `f32` members, assigned bare scalars, and no
  quad swizzles a scalar read twice) and `dof_composite_builds_its_vec4`
  (the composite's `vec4( 0, 0, 0, 1 )` and its two `.xyz` mixes).
- **The kernels.** A unit test checks that `generate_kernels()` splits the
  80 Vogel points into 16 and 64, every fifth point to the small kernel, as
  three's loop does.
- **The page.** `examples/webgpu_postprocessing_dof.rs` renders the page's
  first frame. It shows the same grid as three's screenshot: sharp spheres
  at the focus distance, and bokeh discs nearer and farther. It also runs in
  the native viewer (`viewer postprocessing_dof`). Headless at 800×500 it
  takes 7.2 ms a frame (mean of 30 after 10 warm-up frames, Iris Xe). That
  is 11 draw calls: the instanced spheres, the node's nine quads and the
  output. A steady frame creates no GPU objects.

## What is left out

- **The inspector GUI** (focus distance, focal length, bokeh scale). The
  uniforms keep their defaults: 500, 200 and 10.
- **`CubeTextureLoader`'s asynchrony.** The six faces are decoded before the
  first frame.

The node's own divergences are in `docs/nodes.md` §66 and in the
`depth_of_field.rs` module docs. The page is not in the browser shell, for
the same reason as TRAA's: the shell builds only the graded examples.
