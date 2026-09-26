# `webgpu_instance_points`

Status: **green.** 16 of 100000 pixels against three.js 5f610f5's own
`test/e2e/image.js`, threshold 0.1%. Intel Iris Xe, Mesa 25.3.6, wgpu on
Vulkan. Steady frame 21.5 ms, 6 draw calls, 3011 triangles.

The `webgpu_lines_fat` spline, a `hilbert3D` curve through a
`CatmullRomCurve3`, is sampled at 256 points. Each point is drawn as one
instance of a screen-space quad by a `PointsNodeMaterial` on a `Sprite`. A
kernel writes each point's size into a `StorageInstancedBufferAttribute`.
The material reads it back as an instanced attribute, for the point's size
and for its brightness (`mix( vec3( 0 ), color, size / maxWidth )`). The
scene is rendered twice, as in `webgpu_lines_fat`: first the main view, then
a 125 x 125 inset in the top left with its own background.

## Reconciling with the plans

There is no scout `PLAN.md`. The rung was ported from
`~/src/vendor/three.js/examples/webgpu_instance_points.html` directly, and
three's WGSL (`target/dumps/webgpu_instance_points/`, uncommitted: `m00`
kernel, `m01` / `m02` material) was diffed against `examples/dump_wgsl.rs`'s
`instance_points_*` sections.

## What was added

Nothing in `src/` beyond what `webgpu_skinning_points` added (`docs/nodes.md`
§44), apart from `nodes::builder::with_alpha_to_coverage_samples()` for
`dump_wgsl`. The page needs `storage_f32` + `to_attribute()` for the sizes,
`shape_circle()` in its smoothed branch, and `setupVertexSprite()` with an
`f32` `sizeNode`. The inset reuses `webgpu_lines_fat`'s viewport, scissor,
`clearDepth` and `autoClear = false` path.

## What the pixels found

The rung passes at 16 pixels. Our full-resolution frame is
pixel-identical to the `actual_full.png` three renders on this machine
(0 differing pixels).

At the pinned time 0, the kernel's `time.add( float( instanceIndex ) )` is
the instance index, so each size is `( sin( 6 i ) + 1 ) / 2 * 14 + 6`. That
value is fixed per point and repeats on every frame. The kernel runs before
the first render, so the initial `sizes` of 10 are never drawn.

`alphaToCoverage: true` does two things, as in three. It picks
`shapeCircle()`'s `fwidth` / `smoothstep` edge, and it enables the
pipeline's alpha-to-coverage (`alphaToCoverageEnabled: true` in three's
dump).

The ladder did not move.
