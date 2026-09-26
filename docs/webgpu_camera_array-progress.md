# `webgpu_camera_array`

Status: **green.** 20 of 100000 pixels against three.js 5f610f5's own
`test/e2e/image.js`, threshold 0.1%. Intel Iris Xe, Mesa 25.3.6, wgpu on
Vulkan. Steady frame 4.0 ms, 73 draw calls, 4808 triangles.

One `ArrayCamera` with 36 sub-cameras in a 6 x 6 grid of viewports looks at a
red Phong cylinder in front of a dark blue Phong plane. The scene is lit by an
`AmbientLight` and a shadow-casting `DirectionalLight`, whose shadow camera has
`zoom = 4`.

## Reconciling with the plans

There is no scout `PLAN.md` for this rung. It was ported from
`~/src/vendor/three.js/examples/webgpu_camera_array.html` directly, with
three's WGSL dumped by `tools/dump-webgpu.mjs` into
`target/dumps/webgpu_camera_array/` (uncommitted), and three's
`m02` / `m03` (the cylinder's vertex and fragment stages) diffed against
`examples/dump_wgsl.rs`'s new `camera_array_phong` section.

## What was added

| area | what |
| --- | --- |
| `src/cameras/array_camera.rs` | `ArrayCamera`: a base `PerspectiveCamera` plus `cameras` |
| `src/cameras/` | `PerspectiveCamera::viewport`; `RenderCamera::sub_cameras()` |
| `src/nodes/node.rs`, `builder.rs` | `UniformGroup::CameraIndex` (with `ORDER`); `BufferSource::CameraViewMatrices` / `CameraProjectionMatrices`; `NodeBuilder::with_array_cameras()`, which swaps the camera matrices for `cameraViewMatrices[ v_cameraIndex ]` at build time |
| `src/materials/node_material.rs` | `SetupContext::array_cameras`, in the program's cache key |
| `src/renderer/` | per-sub-camera `cameraIndex` bind groups and viewports in `record_pass`; `FrustumArray` culling |
| `examples/` | `webgpu_camera_array.rs`; `dump_wgsl` section `camera_array_phong` |

`docs/nodes.md` §36 has the full design.

## What the pixels found

The rung passes at 20 pixels. Our full-resolution frame is
pixel-identical to the `actual_full.png` three renders on this machine
(0 differing pixels), so the 20 are three's own distance from its JPEG.

Two things on the page were easy to get wrong:

* `subcamera.copy( camera )` copies the `ArrayCamera`'s projection parameters.
  Those are `new PerspectiveCamera()`'s defaults (fov 50, near 0.1, far 2000),
  not the sub-cameras' constructor values (40 / 10).
* `light.shadow.camera.zoom = 4` tightens the orthographic shadow frustum,
  and the cylinder's shadow on the plane depends on it.

Draw calls count as three's `info` does: `WebGPUBackend.draw()` calls
`info.update()` once per sub-camera. That gives 2 meshes x 36 sub-cameras,
plus the cylinder once in the shadow pass (128 + 2 triangles per
sub-camera, plus 128 in the shadow pass).

The ladder did not move. With no array camera, `UniformGroup::ORDER` gives
the same group numbers as before.
