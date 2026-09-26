# `webgpu_camera`

Status: **ported, not graded.** Three.js itself fails its own reference for
this page on this machine: three's frame scores 922 of 100000 pixels (0.922%)
against `examples/screenshots/webgpu_camera.jpg` with three's unmodified
`Image.compare( …, 0.1 )` (captured through `tools/dump-webgpu.mjs`, which
pins the harness exactly as the grader does). The port's frame is
pixel-identical to three's: the max channel difference against three's
`actual_full.png` is 0. So the port also scores 922, and the only way to turn
it green would be to loosen the threshold, which the rules forbid. The e2e
test is `#[ignore]`d and says why, as `webgpu_instance_path`'s is. Intel Iris
Xe, Mesa 25.3.6, wgpu on Vulkan.

772 of the 922 are in the right half, and nearly all of those are stars. The
reference JPEG's star field is denser than the one three draws here: ten
thousand one-pixel `point-list` points under 4x MSAA, where the rasteriser
decides which pixels a point covers. The other 150 are in the left half,
along the wireframe edges and on the few stars there.

One scene is drawn twice per frame into the two halves of the canvas, through
`setScissorTest( true )` and a `setScissor` / `setViewport` pair per half. The
left half is drawn through `cameraPerspective`, which rides on `cameraRig` and
points at the big sphere. The right half is drawn through an overview camera,
where `cameraPerspective`'s `CameraHelper` shows its frustum. The spheres are
`wireframe` `MeshBasicMaterial`s, and the stars a `Points` with a legacy
`PointsMaterial`.

## What was added

See `docs/nodes.md` §50.

* **`CameraHelper`** (`src/helpers/camera_helper.rs`): the 50-vertex
  `LineSegments`, its `pointMap`, `update()` and `setColors()`. Three sets
  `this.matrix = camera.matrixWorld`, which shares the matrix object rather
  than copying it. That is ported as `Object3D::matrix_alias`: the traversal
  copies the camera's world matrix into the helper's `matrix` at the moment
  three would read the shared object. That timing matters on this page. The
  helper sits in the scene before the rig, so the first render's traversal
  gives it the camera's *stale* world matrix, and the second render gives it
  the one the first traversal just computed. The right half shows the second.
* **`RenderCamera::node()`**, so the helper can reach a camera's scene-graph
  node. The port's `OrthographicCamera` has none, so its helper takes a copy of
  the world matrix instead.
* **`Material.wireframe`.** `WebGPUUtils.getPrimitiveTopology()` makes a
  wireframe `Mesh` a `line-list`. `Geometries.getIndex()` swaps in
  `getWireframeIndex()`, each triangle's three edges as six indices, uploaded
  as `uint32` as three's upload does. `_getDrawParameters()` doubles the
  `drawRange` for it.
* **The page**, `examples/webgpu_camera.rs`. `randFloatSpread` is
  `range * ( 0.5 - Math.random() )`, three draws per star. The page has no
  `Inspector`, so it makes no other draws. `Date.now()` is pinned to 0, so
  `r = 0`.

The legacy `PointsMaterial` becomes a `PointsNodeMaterial` with `transparent`
set back to `false`. `NodeLibrary.fromMaterial()` copies every property of the
legacy material over the new node material, and that undoes the `true` it
inherits from `SpriteNodeMaterial`. Three's dump of this pipeline has no blend
state.

## Not ported

* **The `O` / `P` `keydown` handler.** It switches the active camera to
  `cameraOrtho`. None of the port's hosts forwards key presses to an example.
* **`cameraRig.add( cameraOrtho )`.** The port's `OrthographicCamera` is not
  a scene-graph node. Its helper is hidden on every frame that
  `cameraPerspective` is active, so this changes no pixel.

## Steady frame

`webgpu_camera` is not in `steady_frame_builds_nothing`'s strip. The page
calls `cameraPerspectiveHelper.update()` every frame, which sets the helper's
`position.needsUpdate`, so every frame re-uploads that one buffer. Three does
the same.
