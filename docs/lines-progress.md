# lines — hairline `Line` / `LineSegments`, done

**Status: done.** `examples/d33_treemap_labels.rs` now draws the page's white
tile outlines, and the image gap against `d3_treemap.jpg` fell from **2551 to
156** of 100000 pixels (0.156 %), now *asserted* under a ceiling of 250. The
serial e2e ladder is unchanged at **0 / 60 / 31 / 4 / 0 / 0 / 18 / 1 / 7 / 40**.

## What was added

| area | what |
|---|---|
| objects | `src/objects/line.rs` — `Line` and `LineSegments` (ported from `src/objects/Line.js` / `LineSegments.js`), one struct with an `is_line_segments` flag, since that is all the subclass adds. `Line::new( geometry, material )` and `LineSegments::new( … )` return the same `Node` as `Mesh::new`. |
| payload | `Payload::Line( Line )` next to `Mesh`, plus the payload-agnostic accessors the shared arms need: `geometry()`, `material()`, `bounding_sphere_in()`, `morph_target_influences()`, `is_line()`, `is_line_segments()`. |
| render list | `project_mesh` → `project_drawable`, gated on `is_mesh() || is_line()`, which is three.js's single `isMesh \|\| isLine \|\| isPoints` arm. Frustum cull, material `visible` gate and the painter sort are shared verbatim. |
| topology | `renderer::Primitive::of()` = `WebGPUUtils.getPrimitiveTopology( object, material )` plus `WebGPUPipelineUtils._getPrimitiveState()`'s `stripIndexFormat`. It rides the `Renderable` into `RenderState`, so it is part of the pipeline cache key. |
| material | `MeshBasicNodeMaterial::line( color )` and `pub type LineBasicNodeMaterial = MeshBasicNodeMaterial;` — no new `MaterialKind`, decided by the dump (below). |
| geometry | `BufferGeometry::set_from_points()`, faithful to `setFromPoints()`'s in-place vs fresh-attribute branches. |
| shadows | Both shadow item loops (planar and cube) now read geometry/material off the payload and take the topology from `Primitive::of()`, so a line with `castShadow` casts a hairline shadow instead of panicking. |
| gates | `tests/renderer_lines.rs` (4 deterministic readback tests), `examples/dump_wgsl.rs`'s `line_basic` section, and the d33 image assertion. |

`LineLoop` is deliberately absent: `Renderer._projectObject()` errors on it
("Please use THREE.Line or THREE.LineSegments"), so there is nothing to port.

## The material decision, from the dump

Three's real `LineBasicNodeMaterial` program was captured off d33's own
`d3_treemap.html` page with a temporary `test/e2e/_dump_lines.mjs` (recipe:
`handoff/scouts/rung8/PLAN.md` §2; script and profile dir deleted afterwards).
It is in `docs/lines/LineBasicNodeMaterial_27.{vert,frag}.wgsl` and
`.layout.txt`.

Diffed against this port's `line_basic` dump, the *only* differences are:

* the banner (`Three.js r186` vs `three-rs`);
* the global `nodeUniformN` counter (`nodeUniform4` vs `nodeUniform2`);
* `var<private>` declaration order;
* Three's `VERTEX_` sub-build temps in the vertex stage.

All four are already catalogued in `docs/nodes.md` §8 ("Generated names",
"Declaration order", "`VERTEX_` sub-builds"). The fragment bodies are otherwise
character-identical, down to `DiffuseColor.w = 1.0;`. That is expected once you
read the source: `LineBasicNodeMaterial` is a bare `NodeMaterial` with
`setDefaultValues( new LineBasicMaterial() )`, and every default it sets that
reaches WGSL is one `MeshBasicNodeMaterial` already has. `linewidth`, `linecap`
and `linejoin` are SVGRenderer-only; WebGPU always draws a one-pixel line.

**So: no `MaterialKind::Line`.** A `line()` constructor on the Basic kind
produces byte-identical WGSL, verified through `cargo run --example dump_wgsl`,
whose `line_basic` output differs from `basic` only in the label.

What *is* line-specific is the pipeline, and its input is the object, not the
material — hence `Primitive::of( object, geometry )`. The dumped layout confirms
it: `topology: "line-strip"`, but `frontFace: "ccw"` / `cullMode: "back"` still
straight off `material.side`, and the same two uniform bind groups a basic mesh
gets.

## The d33 number, and the strip

Before: **2551** of 100000 (2.551 %) — the outlines' pixels, missing.
After: **156** of 100000 (0.156 %), ceiling 250.

Looking at the actual / expected / diff strip: all 156 remaining pixels lie in
rows 157–229 along the **near silhouette of the slab** — the side walls of the
tiles, which this example draws with `MeshBasicNodeMaterial` while the page uses
`MeshStandardNodeMaterial` under a `HemisphereLight` + `DirectionalLight`, so
they are flat instead of shaded. Worst RGB distance 64.4 against the
comparator's 44.2 threshold. **None of the 156 is on an outline**: the white
hairlines land where the page's do, which is what the axis-aligned camera makes
checkable — the horizontal and vertical edges of an axis-aligned treemap under
an orthographic-ish view are pixel-exact between Mesa's Vulkan rasterizer and
Chrome's Dawn, and they are.

250 is the ceiling because it leaves headroom for JPEG and driver jitter on that
one unlit band and no room for anything structural: dropping the outlines is
2551, and a topology regression that filled them in would be far worse.

## The deterministic gate

`tests/renderer_lines.rs` renders 64×64 through the real renderer with
`OrthographicCamera::new( 0.0, W, H, 0.0, -1.0, 1.0 )` — one world unit per
pixel, vertices on pixel centres — and reads the framebuffer back:

* a 5-point strip (a, b, c, e, a) draws a closed rectangle: four edges white,
  interior background;
* a 4-point strip draws three segments and does **not** close;
* a 4-point `LineSegments` draws two separate segments and never joins them;
* the same four vertices as a `Line` **do** join — the one bit that proves the
  topology comes from the object.

Corner pixels are not asserted. Line rasterization follows the diamond-exit
rule, so each segment's final pixel is not produced; the corner is covered by
the next segment's *first* pixel only when the strip continues. That is a
rasterizer property, not a port decision, and it is recorded in the test's
module doc.

## Left out, and why

* **`LineLoop`** — `_projectObject()` errors on it; the port does the same by
  not having the type.
* **`computeLineDistances()`** — only `LineDashedMaterial` reads
  `lineDistance`, and there is no dashed material in the port.
* **`PointList` topology / `Points`** — no `Points` object yet; the arm is
  written down in `docs/scene-graph.md` for when there is.
* **Mesh `wireframe`** — the other producer of `LineList` in
  `getPrimitiveTopology()`. `NodeMaterial` here has no `wireframe` field; adding
  one is a material change, not a line change.
* **`linewidth`** — three.js ignores it under WebGPU too.
