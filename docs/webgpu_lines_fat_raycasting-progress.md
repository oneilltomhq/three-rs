# `webgpu_lines_fat_raycasting` — progress

Branch `raycaster-sprite`, issue #143. Scope: the `Raycaster`, `raycast()` on
every drawable object, `Sprite` end to end, and this example as the rung that
grades them.

**PASS: 57 different pixels of 100000, limit 0.1%.** The frame is graded with
`performance.now()` pinned at 0, as every rung is.

## Status

| Piece | Commit | Gate |
| --- | --- | --- |
| `Triangle::get_interpolated_attribute`, `Frustum::intersects_sprite` | `2deb3aa`, `baf01ad` | `tests/math_triangle.rs`, `tests/math_frustum.rs` (QUnit) |
| `Raycaster` (`set`, `set_from_camera`, `intersect_object(s)`, `near`/`far`, `layers`, `params`) and `Intersection` | `baf01ad` | `tests/core_raycaster.rs` (QUnit) |
| `raycast()` on `Mesh`, `InstancedMesh`, `SkinnedMesh`, `BatchedMesh`, `Line`, `LineSegments`, `Points`, `Sprite`, `LineSegments2` | `baf01ad` | `tests/objects_*.rs` (QUnit), `tests/addons_lines_raycast.rs` |
| `Sprite`: `Payload::Sprite`, the shared quad, the render path | `4bdec71` | `tests/renderer_sprites.rs`: WGSL sections against three's dump, and pixels |
| `Line2NodeMaterial` world units and `alphaToCoverage` | `19cde95` | `tests/nodes_line2_world_units.rs`, against three's dump |
| the example and the image | `195d58d` | `tests/e2e/main.rs::webgpu_lines_fat_raycasting` |

## What the graded frame does and does not cover

The frame is one world-units `LineSegments2` with vertex colours and alpha to
coverage on a 4-sample target. That covers the new `Line2NodeMaterial` branch
and the pipeline's `alpha_to_coverage_enabled`.

It does **not** cover the raycast. The harness never moves the pointer, so it
stays at three's initial `( Infinity, Infinity )`, `setFromCamera` builds a NaN
ray, and the two marker spheres stay hidden. The test asserts that they are
hidden. The raycast itself is gated by the QUnit ports and by
`tests/addons_lines_raycast.rs`. three.js has no unit test for
`LineSegments2.raycast()`, so that test casts finite rays through both
branches, screen space and world units, and checks the hit points, the points
on the line and the face indices against three.js' own output for the same
scene.

## Choices worth knowing

* **`line2()` defaults `alpha_to_coverage` to `true`**, as the
  `Line2NodeMaterial` constructor does. `webgpu_lines_fat` sets it to `false`
  explicitly, as its page does, so that rung's image does not move.
* **The pipeline enables alpha to coverage only on a multisampled target**
  (`material.alpha_to_coverage && sample_count > 1`). WebGPU rejects the flag
  on a single-sample target.
* **The WGSL gates compare sections, not whole files.** The vendor dumps come
  from r187dev, whose generator writes `let nodeConstN`, names sub-builds
  `VERTEX_…` and orders the render struct differently. The tests renumber the
  temporaries and compare the parts this branch wrote: the sprite's
  `objectStruct` and centre step, the world-units fragment `main`,
  `closestLineToLine`, and the `worldStart`/`worldEnd` varying writes.

## Gaps left open

* **Sprite fog.** The port builds the fog factor before the material
  installs its sprite `positionView`, so fog on a sprite reads the mesh
  position. No graded rung has a fogged sprite. See the note in
  `tests/renderer_sprites.rs`.
* **Pointer input in the browser shell.** The shell does not forward pointer
  moves yet, so on the web page the spheres never appear. This matches the
  graded frame.
* **Out of scope for #143:** `EventDispatcher`, `LOD`, helpers,
  `ClippingGroup`, and `webgpu_instance_sprites`.
