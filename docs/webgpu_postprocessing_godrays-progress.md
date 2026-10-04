# webgpu_postprocessing_godrays

Branch `display-pages-2`.

**Green at 3 pixels of 100000** (limit 0.1%), steady frame 12.1 ms, 54 draw
calls, 9234 triangles.

## What the score covers

The page renders a scene pass with depth, ray-marches the point light's cube
shadow map at half resolution (`godrays`), blurs the march edge-aware in two
passes (`bilateralBlur`), and lays it over the beauty pass in the light's
colour (`depthAwareBlend`). The graded frame is the first one. So the score
checks these things:

- the point light's six-face shadow lands in the cube depth texture that
  `godrays` was built with, before the march reads it;
- the march, both blur passes and the blend compose in the right order on
  that first frame;
- the targets have this frame's sizes. The rung asserts that the march and
  both blur targets are 400×250 on the 800×500 page.

`tests/nodes_display_wgsl.rs` checks each shader against three's dump:
`godrays_matches_three` (`m09`), `bilateral_blur_horizontal_matches_three`
and `bilateral_blur_vertical_matches_three` (both `m11`, because three
compiles one program for both directions), and
`depth_aware_blend_matches_three` (`m13`).

The 3 differing pixels are not explained further. They are under the limit,
and the WGSL gates pass.

## What this rung adds

| area | what |
|---|---|
| `src/nodes/display/godrays.rs` | `GodraysNode.js` |
| `src/nodes/display/bilateral_blur.rs` | `BilateralBlurNode.js` |
| `src/nodes/display/depth_aware_blend.rs` | `depthAwareBlend.js` |
| `src/lights/light_shadow.rs` | `LightShadow::point_depth_texture()` |
| `src/renderer/mod.rs` | the point shadow draws into the light's own `CubeDepthTexture` |
| `src/nodes/tsl.rs` | `const_array_of`, `UniformArray::element_xyz` |
| `src/nodes/builder.rs` | `negate()` shared as a var; a `Block` read twice re-counts its result; vector `ConstArray` elements |

## Two builder fixes the gates found

**`negate()` was never shared.** The march's ray-plane `t` is a negation read
three times. Three's `negate()` is a `MathNode`, so it becomes a var. The
port inlined it, and the dump comparison counted 18 `dot`s against three's 10.

**A block read twice was counted once.** Under `renderOutput()`,
`depthAwareBlend()`'s result is read twice. Three's analyze stage builds an
`Fn()` call's output at every reach, so the final `mix` becomes a var. The
port counted the block once and wrote the `mix` twice.

## Divergences

- `bilateralBlur` is two materials with fixed directions, not one material
  with a swapped texture. Both compile to three's single program.
- `GodraysNode` and `BilateralBlurNode` run their input's update-before
  first, so the first frame has this frame's depth and sizes.
- The page's GUI is not ported. Every value it drives is a public uniform on
  `App`, and `output_raw` is the graph the blur toggle swaps in.

## Left out

- `GodraysNode`'s `DirectionalLight` branch. The constructor panics for any
  light but a point light.
- A logarithmic depth buffer.
- `dispose()`.
- An orthographic camera in `depthAwareBlend`.
