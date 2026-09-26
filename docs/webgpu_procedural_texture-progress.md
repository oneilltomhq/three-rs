# webgpu_procedural_texture

Issue #144. **PASS: 0 of 100000 pixels differ, limit 0.1%.**

This rung is the gate for `GaussianBlurNode`
(`src/nodes/display/gaussian_blur.rs`). The page renders `checker( uv() *
4 )` into a 512 x 512 `convertToTexture()`. It then blurs that with
`gaussianBlur( …, uniform( .5 ), 20 )` and shows the result on a 1 x 1 plane
under an orthographic camera.

## How the port is built

- `GaussianBlurNode` works the way `BloomNode` does. It owns two
  half-resolution render targets (`resolutionScale` 0.5, no depth) and a quad
  for each. `render()` is the port of `updateBefore()`: it saves the
  renderer's target, MRT, auto-clear and clear colour, draws the horizontal
  pass and then the vertical one, and restores them.
- Each pass sums the sigma's Gaussian coefficients over pairs of
  linear-filtered taps. The sum is unrolled, as three's `setup()` builds it.
  A unit test holds the sigma-20 coefficients to three's dump digit for digit.
- The `direction` node scales the per-pass direction. With no direction node
  the scale is `vec2( 1 )`.

## Verification

- `tests/nodes_display_wgsl.rs` compares `gaussian_blur_horizontal` and
  `gaussian_blur_vertical` with three's `m02` / `m03` modules from this page.
  It compares tap count, coefficients, calls and bindings.
- The rung and `steady_frame_builds_nothing`. Steady frames build and create
  nothing from frame two.
