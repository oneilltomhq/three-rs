# `webgpu_tsl_angular_slicing` — progress

Issue #139's rung. Graded green: 4 of 100000 pixels different, steady frame
3.3 ms, 9 draw calls, 45397 triangles.

- **The model is all Draco.** `gears.glb`'s three meshes are
  `KHR_draco_mesh_compression` primitives, decoded by `draco-core` and
  checked value for value against three.js' own `DRACOLoader` by
  `tests/gltf_draco.rs` (see `docs/gltf-progress.md`, item 6). The rung is
  the pixel-level check on top of that.
- **Which page.** The screenshot was last written at three.js f8462e3, when
  the page used a `DirectionalLight`. r186's copy of the page uses the
  `SunLight` addon (#34476), but the screenshot was not regenerated, and
  #34586 restored the `DirectionalLight` after r186. The port follows the
  page the screenshot shows.
- **`envMapIntensity: 0.5` is inert.** `materialEnvIntensity` is
  `material.envMap ? material.envMapIntensity : scene.environmentIntensity`,
  and both materials light from `scene.environment`, so the port needed no
  new material field.
- **Nothing new in the renderer.** `maskNode` (carried into the shadow pass),
  `outputNode` over the `Output` property, `frontFacing`, `DoubleSide` and
  the directional shadow already existed; the rung only needed the decoder.
