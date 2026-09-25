# webgpu_postprocessing_sobel

Issue #144. **PASS against r186's screenshot: 77 of 100000 pixels differ,
limit 0.1%.**

Against the vendor checkout's HEAD screenshot the rung scores 329 (0.33%).
That screenshot was regenerated upstream after r186 by the two PMREM commits
tracked in #146, the same commits that move the eight existing rungs listed
there. CI grades against r186, which is the reference this rung passes.

This rung is the gate for `SobelOperatorNode` (`src/nodes/display/sobel.rs`).
The page's output is `sobel( renderOutput( scenePass ) )` with
`outputColorTransform = false`. The scene is the `DragonAttenuation.glb`
dragon (`gltf.scene.children[ 1 ]`) with a default `MeshStandardNodeMaterial`,
lit only by `pmremGenerator.fromScene( new RoomEnvironment(), 0.04 )` under
`LinearToneMapping`.

## How the port is built

- `SobelOperatorNode.setup()` calls `convertToTexture()` on its input, so the
  example draws `renderOutput( pass )` into an `RttNode`. The operator then
  takes nine luminance taps of that texture.
- `Gx` and `Gy` are `mat3` constants indexed `[ col ][ row ]`, and each is
  summed left to right in three's order.
- `update()` is `updateBefore()`: `invSize` from the input texture's size,
  called after the RTT is sized.

## Verification

- `tests/nodes_display_wgsl.rs` checks the `sobel` quad against three's `m18`
  from this page.
- The rung and `steady_frame_builds_nothing`.
