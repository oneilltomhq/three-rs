# `webgpu_materials_arrays`

Status: **ported, not graded.** Three.js itself fails its own reference for
this page on this machine: three's frame differs from
`webgpu_materials_arrays.jpg` in 251 of 100000 pixels (0.251%, over the 0.1%
limit; `rung0/e2e-run6-rung1-candidates2.log` recorded 0.2%). The port's
full-resolution frame is **pixel-identical** to three's: 0 differing pixels,
max channel difference 0, against `tools/dump-webgpu.mjs`' `actual_full.png`.
Against the reference the port scores 246 pixels (0.246%). The e2e test is
`#[ignore]`d with that reason, following the `webgpu_instance_path` /
`webgpu_lights_custom` precedent; the page stays in the steady-frame ladder
(`rung!`). Intel Iris Xe, Mesa, wgpu on Vulkan.

Sixteen open "paper model" polyhedra (tetra-, hexa-, octa-, dodeca- and
icosahedra) stand on a teal table under a hemisphere light and a shadowing
directional light (PCF, `radius` 10, 2048² map), with MSAA. Each solid is one
`Mesh( geometry, materials )` with an **array** of six `DoubleSide`
`MeshStandardMaterial`s. The page's `makeHoleyGeometry()` rebuilds every face
as a ring of quads around a hole and calls `geometry.addGroup( start, count,
face % 6 )` per face.

## What was added

| area | what |
| --- | --- |
| `src/objects/mesh.rs`, `payload.rs` | `Mesh.materials` (the array form of `Mesh.material`), `Mesh::with_materials`, `Payload::material_array()` |
| `src/renderer/render_list.rs` | `_projectObject()`'s `Array.isArray( material )` arm: one render item per `geometry.groups` entry whose material exists and is visible; `RenderItem.group` and `RenderItem::material()`; a unit test |
| `src/renderer/mod.rs` | every draw, the transmission probe and both shadow passes take the item's material (`material[ group.materialIndex ]`); `RenderObject.getDrawParameters()`' group clip of the draw range |
| `examples/` | `webgpu_materials_arrays.rs`, with `makeHoleyGeometry()` ported line by line |

`geometry.addGroup()` was already in the port (`BoxGeometry` and the
cylinder use it); what was missing was everything downstream of it.
`docs/nodes.md` §51 has the design.

## What the pixels found

Nothing to fix in the port: the first run's frame was byte-identical to
three's own frame on this GPU. The 246–251 pixels are all one-pixel
silhouette edges of the thin paper faces (see three's own diff against the
reference). That is MSAA coverage, which this GPU resolves differently from
the machine that rendered the reference. No shader in the page is new, so
there is no `dump_wgsl` section. The six materials are ordinary
`MeshStandardMaterial`s, and each group draw uses the same program three
builds for that material.
