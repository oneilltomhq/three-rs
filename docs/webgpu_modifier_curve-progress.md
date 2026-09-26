# `webgpu_modifier_curve`

Status: **green.** 3 of 100000 pixels against three.js 5f610f5's own
`test/e2e/image.js`, threshold 0.1%. Three's own frame for the page scores the
same 3 against the same JPEG. Intel Iris Xe, Mesa 25.3.6, wgpu 30.0.1 on
Vulkan.

"Hello three.js!" is built as a `TextGeometry` (an `ExtrudeGeometry` over the
shapes `Font.generateShapes()` makes from `helvetiker_regular.typeface.json`)
and bent along a closed centripetal `CatmullRomCurve3` by `Flow`
(`examples/jsm/modifiers/CurveModifierGPU.js`). The page also has four white
handle boxes and the curve drawn as a green `Line`.

## Why this rung and not `webgpu_instance_path`

Issue #170 names `webgpu_instance_path` as the rung that grades the geometry
port. That page cannot be graded on this machine. **three.js itself fails its
own reference for it:** it scores 314 of 100000 pixels (0.314%) against
`examples/screenshots/webgpu_instance_path.jpg` with three's unmodified
`Image.compare( …, 0.1 )` (captured through `tools/dump-webgpu.mjs`, which
pins the harness exactly as the grader does). The port's frame is
pixel-identical to three's: the max channel difference against three's
`actual_full.png` is 0. So the port also scores 314, and the only way to turn
it green would be to loosen the threshold, which the rules forbid.

The port stays in the tree. `examples/webgpu_instance_path.rs` is the full
page: a heart `Path` of `bezierCurveTo`s, a thousand ico-spheres placed with
`path.getPointAt( i / count )`, one plain `Mesh` with `mesh.count = 1000`, and
`instancedBufferAttribute` nodes. Its e2e test is `#[ignore]`d and says why. It
is still in the steady-frame strip, which checks that frames two and three
build and upload nothing, and grades no pixels.

This rung exercises more of the same port than `webgpu_instance_path` would
have. It uses `Font` → `ShapePath.toShapes()` → `Shape`/`Path` holes →
`ShapeUtils`/Earcut triangulation → `ExtrudeGeometry` with bevels, then
`CatmullRomCurve3.getSpacedPoints` and `computeFrenetFrames` baked into
`Flow`'s spline texture.

## Grade first

| | pixels of 100000 |
| --- | --- |
| three.js 5f610f5 against its own reference JPEG | 3 |
| this port against the same JPEG | 3 |

## What was added

| area | what |
| --- | --- |
| `src/loaders/font_loader.rs` | `FontLoader`, `Font`, `Font::generate_shapes`, `TextDirection` |
| `src/addons/text_geometry.rs` | `TextGeometry` as `text_geometry( text, &font, &options )` |
| `src/addons/curve_modifier_gpu.rs` | `Flow`: the spline texture, `updateCurve`, `moveAlongCurve` and the `positionNode` / `normalNode` graph |
| `examples/webgpu_modifier_curve.rs` | the port |
| `examples/dump_wgsl.rs` | the `modifier_curve` section |
| `tools/geometry_reference.mjs` | `text_modifier_curve`, `text_defaults` and `flow_modifier_curve` scenarios |
| `tests/geometries_shape_oracle.rs` | `text_geometries` (every float of the page's `TextGeometry` against three's, 1e-6) and `flow_spline_texture` (every half-float texel of the spline texture, exact) |
| `tests/e2e/main.rs` | the graded test and the `rung!()` line |

## What the pixels found

**536 pixels: the text sat one spline texel behind three's.** Everything
upstream of the draw matched. The oracle compared the spline texture bit for
bit, the sampler and format matched, the boxes were pixel-identical (so the
camera was right), and the WGSL matched apart from `var` ordering. A
diagnostic that added a constant to the texture `u` scored best at almost
exactly `+1 / 1024`. The cause was the page, not the renderer: `animate()`
calls `flow.moveAlongCurve( 0.001 )` before every render, so the graded frame
is already `pathOffset = 0.001` along the curve. That is 1.02 texels of the
1024-wide texture. With `moveAlongCurve` ported into `animate()`, the rung
scores 3.

## Not ported

* **`TransformControls` and the `Raycaster`.** The page creates the gizmo only
  on a `pointerdown` that hits a handle, so neither draws on the graded frame.
  The handles cannot be dragged in the viewer or the browser shell.
* **A generic `Flow` over an `Object3D` tree.** `docs/nodes.md` §28 has the
  details.

`docs/nodes.md` §28 lists the divergences.
