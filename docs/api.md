# API design note

The decisions issue #14 asked for, settled 2026-09-13 after the first outside
consumer (a native app drawing dendrograms on two planes) built three rungs
against the published 0.1.0. Each decision records its reason so it is not
reopened per pull request. Breaking changes ship together in 0.2.0.

## 1. The scene stays single-threaded, by design

Nothing in the scene graph is `Send` or `Sync`, and that is the design rather
than a 0.x limitation. three.js's graph is single-threaded and
`WebGPURenderer` never touches an `Object3D` off the main thread; the port
inherits that shape, and it is the shape of every renderer built on the same
model. Renderers that work across threads fall into two camps: flat entity
stores with a scheduler (Bevy, Unity DOTS), or one owner thread with a copy of
the scene handed to the renderer (Unreal's proxies, Godot's servers, Filament,
rend3). Sharing a pointer tree behind locks is the third option, and
OpenSceneGraph is the cautionary tale; its successor VulkanSceneGraph went
back to a single owner. three-rs is in the second camp.

Parallelism belongs beside the scene, not inside it: asset decoding, layout,
the SDF rasteriser, and a git or filesystem scan all run on other threads and
hand finished data to the scene thread. `wgpu` itself is thread-safe. A
compositor drives the scene from one thread and feeds it messages.

`Rc`, not `Arc`; `RefCell`, not `Mutex`. The compiler refuses to let a `Node`
cross a thread, which is the guarantee wanted.

## 2. A scene object is a `Node`, a newtype over `Rc<RefCell<Object3D>>`

0.1.0 spells the handle out as a type alias, `pub type Node =
Rc<RefCell<Object3D>>`, with the tree operations on an `Object3DNode` trait.
Two costs showed up in the first consumer and in six of the fourteen examples:
every user has to discover `use three_rs::core::Object3DNode` before
`light.add(&mesh)` compiles, and rustdoc prints the plumbing in every
signature instead of a type called `Node`.

0.2.0 makes `Node` a newtype with the trait's methods inherent, and
`Deref<Target = RefCell<Object3D>>` so `borrow()` and `borrow_mut()` keep
working. Every call site stays the same text, so the examples, the QUnit
ports in `tests/`, and the line-for-line correspondence with three.js that
the pixel grader depends on are untouched. `WeakNode` gets the same
treatment; `Rc::ptr_eq` becomes `Node::ptr_eq`.

Rejected: an arena of indices, the shape a Rust reviewer expects and what the
d3-hierarchy port uses. `docs/scene-graph.md` gives the reasons and they
still hold: every three.js method would gain an arena argument, objects live
outside any scene (cameras, the instance-mesh dummy, loaded glTF trees before
they are added), `attach()` across parents stops being expressible, and the
QUnit ports have no arena to pass. The arena earns its cost only under
decision 1's first camp, which this crate is not in.

Users hold `Node` clones across frames; that is what every consumer does. The
parent link is `Weak`, the one place the port's ownership differs from
three.js: a child does not keep its parent alive, so a subtree removed and
not held is freed.

## 3. Keep three.js's shape wherever there is no real trade-off

- **Field access, not accessor layers.** `mesh.borrow_mut().position.y = -1.0`
  is the JavaScript with one extra call. A full getter and setter layer over
  `Object3D`'s public fields would hide the cell behind `update(|o| ..)`,
  which is `borrow_mut()` under another name, and would cost the ports their
  shape. Convenience setters are added one at a time where they clearly pay
  (`set_position`, light-specific passthroughs that avoid
  `light_mut().unwrap()`).
- **`Mesh::new(geometry, material)`** takes the material, as `new Mesh(
  geometry, material )` does. 0.1.0's `Mesh::new(geometry)` followed by
  `mesh.borrow_mut().mesh_mut().unwrap().material = Some(m)` is the ugliest
  line in every example and appears 22 times; `Line::new` and
  `LineSegments::new` already take the material. Breaking, 0.2.0.
- **Materials keep public fields and named constructors; no builders.**
  `MeshBasicNodeMaterial` has some 40 public fields and three.js's docs name
  them. Struct-update syntax already gives builder ergonomics:
  `MeshPhongNodeMaterial { shininess: 80.0, ..MeshPhongNodeMaterial::phong(c) }`.
  The named constructors (`phong`, `standard`, `line`, `sprite`) carry the
  per-kind defaults, which is where three.js's subclasses live.
- **Lights, cameras and textures** take three.js's positional constructor
  arguments and expose the rest as fields or one-line setters, for the same
  reason.
- **Rust names throughout**, with the three.js name in the doc comment. This
  is already how the crate is written (`set_rotation`,
  `matrix_world_needs_update`, `is_mesh()`); recorded so it is not reopened.
- **`Scene` and the cameras own a `node` field** and are not `Node`s
  themselves. `Scene` adds `background`, `fog_node` and `override_material`;
  a camera adds its projection state. Both forward `add()`, `children()` and
  `update_matrix_world()` to their node, so both can sit inside the tree.

## 4. Surface hygiene

- `three_rs::testing` is `#[doc(hidden)]`. It holds the harness helpers
  (`write_png`, `three_js_dir`) that the examples, the viewer, the e2e tests
  and consumers' own graders use, so it stays public but out of the docs.
  The audit found no other accidental `pub`; the rest is the ported surface.
- Only things that touch the filesystem or the GPU return `Result`: the
  loaders, `Renderer::new`, `render()`. Constructors, geometry builders and
  the scene-graph methods stay infallible, as they are in three.js. Issue #9
  does the work of replacing the remaining panics under this rule.

## Where each decision came from

Decisions 1 and 2 came out of the plane-dendro consumer test and a
conversation about which renderers are multi-threaded and how. Decision 3 is
the port's founding discipline restated for the public API. The consumer
gaps that test produced (undocumented windowed path, silent frustum culling
of text, per-frame program rebuild) are filed as their own issues; they are
about missing pieces and performance, not about the shape above.
