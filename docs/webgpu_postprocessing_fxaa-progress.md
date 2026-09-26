# `webgpu_postprocessing_fxaa`

Status: **green.** 56 of 100000 pixels against three.js 5f610f5's own
`test/e2e/image.js`, threshold 0.1%. Intel Iris Xe, Mesa 25.3.6, wgpu on
Vulkan. Steady frame 4.0 ms, 3 draw calls, 402 triangles. Three itself scores
56 against the same JPEG: the port's frame is byte-identical to three's
`actual_full.png` on this machine.

One hundred flat-shaded red tetrahedra in an `InstancedMesh`, lit by a
hemisphere light and a directional light on white. The pipeline turns off
`outputColorTransform`, runs `renderOutput( scenePass )` into an `RTTNode` and
applies `fxaa()` to that, so FXAA sees sRGB values, as the page intends.

## Reconciling with the plans

There is no scout `PLAN.md` for this rung. It was ported from
`~/src/vendor/three.js/examples/webgpu_postprocessing_fxaa.html` and
`examples/jsm/tsl/display/FXAANode.js`, with three's WGSL dumped by
`tools/dump-webgpu.mjs` into `target/dumps/webgpu_postprocessing_fxaa/`
(uncommitted, per the rules). The FXAA pass is three's `m05`. That module is
now a committed fixture that `tests/nodes_display_wgsl.rs` gates against.

## What was added

| area | what |
| --- | --- |
| `src/nodes/display/fxaa.rs` | `fxaa( map )`, `FxaaNode` with its `update()` of the inverse size; `ApplyFXAA` as the one layout `Fn`, `FxaaPixelShader( uv, texSize )` |
| `src/nodes/node.rs`, `builder.rs` | `SampleMode::Bias`, emitted as `textureSampleBias` |
| `src/nodes/tsl.rs` | `texture_bias()`, `uniform_array_f32()`, `UniformArray::element_x()` |
| `tests/nodes_display_wgsl.rs` | `Region::Function( name )`, which takes the fingerprint over one named `fn`; `fxaa_matches_three` |
| `examples/` | `webgpu_postprocessing_fxaa.rs`, viewer, web shell and manifest registration |

`docs/nodes.md` §42 is the long form.

## What the pixels found

Nothing to chase. The first full render was already at 56 pixels and
byte-identical to three's own frame. The 56 are the reference JPEG's
compression against both renderers alike. The single divergence in the WGSL
(`max( pixelBlend, edgeBlend )` gets a var in each final arm, §42.2) does not
change a value.
