# webgpu_postprocessing_outline

Branch `display-pages-1`.

**Green at 15 pixels of 100000** (limit 0.1%), steady frame 4.1 ms, 46 draw
calls, 96243 triangles. Three.js scores the same page 15 pixels different
from its own reference on this machine.

## What the score says, and what it does not

The rung proves the scene is right. The outline only gets as far as building
and sampling an empty composite.

Nothing is selected until the pointer moves, and the harness never moves
it. So on the graded frame `OutlineNode.updateBefore()` finds an empty
selection and returns before any of its passes. The output adds
`texture( composite ) * edgeStrength * …`, which is zero. What the score
checks:

- the twenty Lambert spheres, the floor and the torus, the `tree.obj` mesh
  loaded by the new `ObjLoader`, with its normals, centring and scale;
- the directional light's 2048² shadow map;
- the page's random draws, in order, after `new Inspector()`'s five;
- an output graph that samples the outline's composite, with an empty
  composite adding nothing.

What the outline draws once something is selected is checked in two other
ways:

- `tests/nodes_display_wgsl.rs` gates every one of its shaders against
  three's own: the depth and mask materials, copy, edge detection, both
  blurs and the composite, plus the page's output. Three's page never
  builds the pass shaders unprompted, so they come from
  `tools/dump-pages/outline_selected.html`, the page with
  `selectedObjects.push( torus )` added.
- `tests/outline_frames.rs` renders a selection on the GPU:
  - a box in the open gets a red ring just outside its silhouette;
  - behind a blocker, the ring is green;
  - an orthographic camera also gets a ring;
  - deselecting clears the composite, which stays clear.

## What this rung adds

| area | what |
|---|---|
| `src/nodes/display/outline.rs` | `OutlineNode.js`: `outline()`, `OutlineParams`, `visible_edge()` / `hidden_edge()`, the seven-step `update_before()` |
| `src/renderer/mod.rs` | `OutlineSelection`, standing in for the two `setRenderObjectFunction()` closures: skip by selection, draw with the pass's material, no background and no shadow maps for the render |
| `src/loaders/obj_loader.rs` | `OBJLoader.js`, faces only (`docs/nodes.md` §72 and the module doc) |
| `tests/obj/` | `gen.mjs` and `oracle.json`: three's `OBJLoader.parse` under node over `tree.obj` and synthetic inputs, for `tests/loaders_obj.rs` |
| `examples/webgpu_postprocessing_outline.rs` | the page, with `pointer_move()` for its raycast selection |

## Divergences

These are listed in full in `docs/nodes.md` §72:

- The blur draws use one material each, not one material rewritten between
  draws.
- `cameraNear` and `cameraFar` are settables, refreshed in
  `update_before()`.
- The composite's GPU texture is created up front, not when first bound.
- The tree loads synchronously in `init()`. Three adds it from the loader's
  callback once the file arrives, which is before its harness takes the
  screenshot, since the tree is in the reference. It goes into `obj3d`,
  which is already the group's first child, so the scene graph is the same
  either way.

## Left out

- `dispose()`.
- Changing the parameters after construction.
- The inspector names for the two scene renders.
- In `ObjLoader`: lines, points, MTL materials and material names.
