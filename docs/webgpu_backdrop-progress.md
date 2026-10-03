# `webgpu_backdrop`

Status: **green.** 23 of 100000 pixels against three.js 5f610f5's own
`test/e2e/image.js`, threshold 0.1%. Three's own frame for the page scores the
same 23 against the same JPEG. The port's frame is pixel-identical to three's
(`tools/dump-webgpu.mjs`' `actual_full.png`, max channel difference 0). Intel
Iris Xe, Mesa 25.3.6, wgpu on Vulkan. Steady frame 6.1 ms, 11 draw calls,
37771 triangles.

Michelle dances inside a ring of eight spheres. Each sphere is a transparent
`MeshBasicNodeMaterial` whose `backdropNode` reads the frame behind it
(`viewportSharedTexture()`): hue-shifted, inverted, grey, saturated, overlaid
with a checker, pixelated at two sizes, and reduced to its blue channel.
Michelle's own `outputNode` posterizes her. A spot light rides on the camera.

The graded frame is at pinned time 0. `oscSine( 0 )` is 0, so the hue shift
is zero, the saturated sphere's `backdropAlphaNode` is 0 (its backdrop mixes
in at alpha 0, showing only the lit sphere), and Michelle is not posterized.

## What was added

* **The framebuffer copy and the viewport nodes**, shared with
  `webgpu_refraction`: see `docs/nodes.md` §61. Every sphere's
  `viewportSharedTexture()` is its own node, so the renderer copies the frame
  eight times, once before each sphere, in the transparent list's
  back-to-front order. Each sphere sees the spheres drawn before it, as in
  three.
* **`SpotLight` and `DirectionalLight` start at `DEFAULT_UP`.** Three's
  constructors set `position` to `Object3D.DEFAULT_UP`, (0, 1, 0). The port
  left them at the origin. The page adds the spot light to the camera without
  moving it, so the light sits one unit above the eye. Before the fix, every
  sphere's highlight was too low and 20966 pixels differed by more than 8.
* **`step()` takes the wider operand type.** `MathNode.getNodeType()` is the
  wider of the two inputs, and three builds both at that type. That gives
  `step( vec3<f32>( 0.5 ), base )`, which WGSL needs: it has no mixed
  overload. `blendOverlay` is the first caller to need it.
* **TSL `grayscale`, `posterize` and `blendOverlay`** (`src/nodes/tsl.rs`).
  `blendOverlay` is a `Fn` in three, so it is a `shader_fn` here and emits a
  WGSL function of the same name.
* **`BasicLightingModel` zeroes `indirectDiffuse` first.** A lit Basic
  material now starts with three's `indirectDiffuse = vec4( 0 ).xyz` store.
* **The page**, `examples/webgpu_backdrop.rs`, and `dump_wgsl`'s
  `dump_backdrop()`.

## The dump

`dump_backdrop()` prints each sphere's fragment WGSL. Three's dump of the
page has the spheres' programs in `m06` to `m17`. The grey sphere's program
was compared line by line against three's `m15`. It matches apart from
variable names, declaration order, and two habits the port already had: it
re-assigns some zero stores, and it emits the `singleScattering` block twice.
Neither changes a value. The frame being pixel-identical covers the other
seven.
