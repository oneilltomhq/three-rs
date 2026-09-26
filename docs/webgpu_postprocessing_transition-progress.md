# webgpu_postprocessing_transition

Issue #144. **PASS: 0 of 100000 pixels differ, limit 0.1%.**

**This rung does not exercise `TransitionNode` in its graded frame.** The
page's tween waits 2000 ms before it moves `transition` off 0, and while
`transition === 0` the page's `render()` draws `fxSceneB` straight to the
canvas. With `performance.now()` pinned to 0 the graded frame is scene B
alone. Three's own dump of the page has no transition module, for the same
reason.

What the frame does check is the scene the transition blends:

- 500 flat-shaded Phong icosahedra in an `InstancedMesh`;
- `setColorAt()` greys;
- an ambient light and a directional light;
- the seeded `Math.random` sequence, which scene A's 500 boxes consume first
  (5000 draws).

`TransitionNode` (`src/nodes/display/transition.rs`) runs in the viewer, where
time passes and the tween moves.

## Divergences, visible only once time runs

- The tween is evaluated from the timer's elapsed time: a 2 s delay, a 1.5 s
  linear ramp, yoyo, repeated. It does not use `tween.js`.
- `cycle` swaps `mixTextureNode.value` at each end of the tween. In the port a
  texture is part of the graph, not a uniform, so the mix texture stays
  `textures[ 5 ]`, the page's initial one.

## Verification

- The rung and `steady_frame_builds_nothing`.
