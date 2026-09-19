# `webgpu_lines_fat`, first half — progress

Branch `rung-lines-fat-a`, cut from `main` at 2cf90bd and rebased onto
`rung-ssaa`. Scope: **steps 1 to 3**
of `scouts/webgpu_lines_fat/PLAN.md` §6 — the shared renderer and node-system
work that the fat-lines material needs but that nothing in it is specific to.
Steps 4 to 6 (`addons/lines`, `Line2NodeMaterial`, the example and its image)
are the second half and are not started here.

The plan itself says these two steps "should go on a branch that can merge
independently of the lines work", which is what this is. Nothing in this branch
mentions a line except `Material::linewidth`, and nothing in it can move a
pixel of the existing ladder — which it does not: every rung comes out on the
number the branch below it left — **0 / 60 / 0 / 1 / 31 / 0 / 7 / 4 / 18 / 40 /
6 / 0 / 4 / 7 / 44 / 0** — `cargo test --release --test e2e` 22 passed.

## Status

| Step | Plan §6 | State | Gate |
| --- | --- | --- | --- |
| 1 | viewport, scissor, `autoClear`, `clearDepth` | done, `fc8d69b` | `tests/renderer_viewport.rs`, 5 tests |
| 2 | `Curve` + `CatmullRomCurve3` | done, `716f94e` | QUnit port (11) + `spline_oracle.json` (4608 bit-exact) |
| 3 | the screen/camera uniforms and the node bits | done, `74ce353` | `dump_wgsl` unchanged for every existing material |
| 4 | `addons/lines` geometries and objects | not started | — |
| 5 | `Line2NodeMaterial` | not started | — |
| 6 | the example and the image | not started | — |

## The one correction to the plan: **there is no y-flip**

PLAN.md §4.5 and §5.4 say the port must convert three.js' bottom-left viewport
origin to wgpu's top-left, and §6 step 6 lists "is the inset in the *top*-left?
(y-flip)" as the first thing to check when the image is wrong. **The premise is
wrong, and implementing the flip is what would break it.**

three.js' *unified* `Renderer` measures the viewport and the scissor from the
**top-left** — `setViewport`'s own documentation says "the vertical coordinate
for the upper left corner". It is `WebGLBackend.updateViewport()` that converts,
with `state.viewport( x, renderContext.height - height - y, … )`, because GL's
origin is bottom-left. `WebGPUBackend.updateViewport()` passes the rectangle
straight through, because WebGPU's origin is already top-left — and so is
wgpu's. So the port applies **no** flip.

Confirmed independently against the scout's own capture: in
`scouts/webgpu_lines_fat/dump/actual_full.png` the inset occupies
`x 20..144, y 355..479` counting down from the top, which is exactly
`setViewport( 20, 355, 125, 125 )` unflipped at DPR 1.

This matters more than a comment, because a wrong flip renders *plausibly* and
is wrong by thousands of pixels. So `tests/renderer_viewport.rs` pins it with a
rectangle chosen so the flipped answer shares **no row** with the right one
(`y = 4, height = 20` in a 64-high frame flips to `y = 40`), and states the
negative case separately so the failure message names the bug. Flipping `y`
inside `Rect::of` fails 3 of the 5 tests; the gate is not vacuous.

Step 6's checklist item (i) is therefore backwards and should be inverted when
the second half works down that list. The example computes
`posY = innerHeight - insetHeight - 20`, which looks like a bottom-left
convention but is handed to a top-left API, so **the correct render puts the
inset at the *bottom* left** — `setViewport( 20, 355, 125, 125 )` at
`innerHeight = 500` means 355 rows down from the top. An inset at the *top*
left (`y = 20`) is the signature of a flip that should not be there.

## What the second half will find in place

### Renderer (step 1)

`Renderer` now carries `viewport: Vector4`, `scissor: Vector4`,
`scissor_test: bool` and the three `autoClear` flags, and `PassTarget` carries
the resolved `viewport: Rect` and `scissor: Option<Rect>` that `draw()` hands
`pass.set_viewport` / `pass.set_scissor_rect` immediately after
`begin_render_pass`. `Rect::of` does the ×`pixel_ratio`, floor and clamp that
`Renderer._getFrameBufferTarget()` does. Every existing pass resolves to
`Rect::full`, which is wgpu's default, which is why the ladder cannot move.

New public API:

```rust
Renderer::set_viewport(&mut self, x: f64, y: f64, width: f64, height: f64)
Renderer::viewport(&self) -> Vector4
Renderer::set_scissor(&mut self, x: f64, y: f64, width: f64, height: f64)
Renderer::scissor(&self) -> Vector4
Renderer::set_scissor_test(&mut self, scissor_test: bool)
Renderer::scissor_test(&self) -> bool
Renderer::clear(&mut self, color: bool, depth: bool)
Renderer::clear_depth(&mut self)
Renderer::auto_clear / auto_clear_color / auto_clear_depth   // pub fields, all true
```

`set_size` resets the viewport and the scissor to the new full frame, as
three's does. Clear resolution follows `Background.update()`: with `auto_clear`
off and no background nothing is cleared, which is what the second render of a
picture-in-picture frame needs — note that a `Color` background sets
`forceClear` and clears anyway, so the inset scene must have **no** background.

`auto_clear` gates the other two, as three's does: with it off neither the
colour nor the depth is cleared, whatever the per-buffer flags say. It and
`set_clear_color()` / `clear_color()` / `clear_alpha()` arrive on the branch
below this one (`rung-ssaa`), which needed the same switches for
`SSAAPassNode`; this step adds `auto_clear_color` / `auto_clear_depth`, the
per-buffer half.

Three's `clearColor()` / `clearStencil()` are deliberately **not** here: the
name `clear_color` is taken by the getter of the `set_clear_color` pair, and
nothing needs a stencil-only clear — the port allocates no stencil buffer,
which is also why `clear` takes `( color, depth )` and not three's third
argument. `clear( color, depth )` and `clear_depth()` cover §2.5.

A `clear()` on the canvas goes through the internal framebuffer target and ends
in the output blit, exactly as `Renderer.clear()` does; a clear with a render
target bound — every one `SsaaPassNode` makes — is the bare pass.

### The render-target seam — **what the PMREM rung should call**

`RenderTarget` grew its own `viewport` / `scissor` / `scissor_test`, exactly as
three's `RenderTarget` has them and exactly as its canvas viewport does *not*
apply to them (target pixels, ratio 1, no `pixel_ratio` anywhere):

```rust
RenderTarget::set_viewport(&self, x: f64, y: f64, width: f64, height: f64)
RenderTarget::viewport(&self) -> Vector4
RenderTarget::set_scissor(&self, x: f64, y: f64, width: f64, height: f64)
RenderTarget::scissor(&self) -> Vector4
RenderTarget::set_scissor_test(&self, scissor_test: bool)
RenderTarget::scissor_test(&self) -> bool
```

**The PMREM rung should call `RenderTarget::set_viewport` (and
`set_scissor` / `set_scissor_test` if it needs them), not any `Renderer`
method.** A target tiled into several views sets the rectangle on the target,
binds it with `set_render_target`, and renders; the renderer's own viewport
state is not consulted on the render-target path at all, so the two rungs
cannot interfere. `tests/renderer_viewport.rs::a_render_target_renders_through_its_own_viewport`
is that case, and it deliberately leaves the renderer's viewport at the full
frame to prove the target does not read it.

These are `&self`, not `&mut self`, because `RenderTarget` is already an
`Rc<RefCell<_>>` handle — the same shape `set_size` has.

### Curves (step 2)

`three_rs::extras::{Curve, CatmullRomCurve3, CurveType, FrenetFrames}`, all
re-exported at the crate root. `Curve` is a trait whose only required method is
`get_point`; `get_point_at`, `get_points`, `get_spaced_points`, `get_length`,
`get_lengths`, `get_u_to_t_mapping`, `get_tangent`, `get_tangent_at` and
`compute_frenet_frames` are provided. Step 4 can build `LineGeometry` straight
off `spline.get_points( 768 )`.

Divergences, each commented at the site: no arc-length cache (a `&self` method
cannot memoise; recomputation is pure, so the numbers are identical, only
slower — `getLengths` on a 768-division curve is not on any hot path here);
`CubicPoly` is a local rather than a module scratch singleton; two explicit
panics where the JS silently yields `NaN`; no `Default` impl, since a derived
one would give `tension = 0.0` rather than three's `0.5`.

One thing the second half should not misread: the oracle gate does **not**
cover the centripetal-vs-chordal exponent, and cannot. `hilbert3D` puts its
control points on a lattice, so every adjacent pair is equidistant, so
`dt0 == dt1 == dt2` and the non-uniform Catmull-Rom degenerates to the uniform
one for any exponent. Changing 0.25 to 0.3 leaves all 4608 oracle values bit
identical. It is the **QUnit** gate that pins the exponent
(`centripetal_basic_check` fails on that mutation and nothing else does). Both
gates were mutation-checked; neither is vacuous.

### Nodes (step 3)

Five new `UniformSource` variants, reachable from TSL as accessors:

| TSL | `UniformSource` | type / group | three |
| --- | --- | --- | --- |
| `viewport()` | `Viewport` | `Vec4` / render | `ScreenNode.VIEWPORT` |
| `screen_dpr()` | `ScreenDpr` | `F32` / render | `screenDPR` |
| `camera_projection_matrix_inverse()` | `CameraProjectionMatrixInverse` | `Mat4` / render | named uniform, as in three |
| `model_world_matrix_inverse()` | `ModelWorldMatrixInverse` | `Mat4` / object | `object.matrixWorld` inverted per object |
| `material_line_width()` | `MaterialLineWidth` | `F32` / object | `MaterialNode.LINE_WIDTH` |

`viewport()` is the **pass' rectangle in device pixels**, not the canvas size —
it is the same `Rect` step 1 resolves, via `Rect::to_vector4`. That is what
makes the fat line four times wider inside a 125-high inset than in a 500-high
frame (step 6's checklist item (ii)); if both look the same, this uniform is
not being refreshed per render.

`Material` gained `pub linewidth: f64`, default `1.0`, with three's note that
the hairline WebGL/WebGPU paths ignore it.

TSL also gained:

* `Node::If { cond, body, else_body }` — the `else` arm. The generate arm emits
  `} else { … }` only when `else_body` is non-empty, so `if_then` and
  `discard_if` produce the same bytes they did: the `dump_wgsl` diff against
  `main` has **0 removals**.
* `if_else( cond, body, else_body )` and `if_else_if( cond, body, other, other_body )`.
  **`if_else_if` generates a nested `if`/`else`, not an `else if`** — that is
  what three does, because `StackNode.ElseIf()` is literally
  `Else( () => If( … ) )`. Match it; do not "tidy" it, or the §5.2 WGSL diff
  will not close.
* `sub_assign`.

`examples/dump_wgsl.rs` gained a `screen_uniforms` section whose position chain
is shaped like `Line2NodeMaterial`'s and exercises all five uniforms,
`sub_assign` and `if_else_if`. Its two uniform structs come to **288** and
**160** bytes with the same membership as three's `renderStruct` and
`objectStruct` in `dump/` — the member *order* differs, which is now recorded
in `docs/nodes.md` §8 as its own divergence class (§3.3 of the plan says to
record these rather than derive them, and this is the general statement of
that).

## The open design question, and the answer to carry forward

PLAN.md §4.4 — how `Line2NodeMaterial` reaches `instanceStart` — is **not
settled by this branch**, and the second half should take the scout's
recommendation as written:

> **(a) for this rung, (b) as a follow-up issue.**

That is: add `line_segments: Option<LineSegmentsAttributes>` to `SetupContext`,
holding the two `Rc<Vec<f32>>` (the interleaved positions and the interleaved
colours), next to the existing `morph: Option<MorphEntry>` — which is the exact
precedent for geometry-derived setup input, and which already hashes by `Rc`
pointer, so the program-cache key stays cheap and correct. Roughly 60 lines,
reusing the `instanced_data_attribute` node the port already has, and it cannot
regress another example because nothing else sets the field.

**(b)** — giving `BufferGeometry` real interleaved instanced attributes
(`BufferAttribute` gains `stride`/`offset`/`instanced`, `VertexBufferDesc::Geometry`
gains stride/offset/step-mode and grouping by underlying array identity,
`InstancedBufferGeometry::instanceCount` becomes a geometry field, and
`attribute( name, ty )` resolves as it does in three) — is the right end state
and is the shape that lets `Line2NodeMaterial` carry zero geometry knowledge.
It is **a follow-up issue, deliberately not done here**: it is ~250 lines
through `ensure_geometry` / `vertex_buffers()` / `programs.rs`, the path every
example uses, and it is worth doing once `Points` / `BatchedMesh` (rungs 11/12)
have shown what else wants it.

Nothing in steps 1 to 3 prejudges this. No `SetupContext` field was added and
no geometry type was touched, so whichever way the second half goes, it starts
from a clean slate.

## Also carried forward from the plan, unchanged

* §9's placement recommendation stands: `Line2NodeMaterial` in
  `src/materials/line2.rs` (core, because it is a node material like the
  others), and `LineSegmentsGeometry` / `LineGeometry` / `LineSegments2` /
  `Line2` in an `addons/lines` crate `three-rs-lines`. The cost the plan names
  is real and should be stated in that rung's report: `cargo test --test e2e`
  in the root crate will not cover the example, so its image test lives in
  `addons/lines/tests/` and has to be run explicitly.
* `blending = NoBlending` on the fat-lines material means `!is_opaque()`, so
  `DiffuseColor.w = 1.0` must **not** be emitted for it (step 6, item (iii)) —
  note that this is the one example where the §8 "`DiffuseColor.w = 1.0`"
  divergence has teeth, so do not extend that bullet's "emit it for all four"
  habit to this material.
