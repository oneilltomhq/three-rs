# Ownership model for the scene graph

`Object3D` is the node payload; the tree is

```rust
pub type Node = Rc<RefCell<Object3D>>;   // Object3D.children entries
pub type WeakNode = Weak<RefCell<Object3D>>; // Object3D.parent
```

A parent holds its children strongly, a child holds its parent weakly. All the
methods that reach outside a single object — `add`, `remove`, `attach`,
`traverse*`, `getObjectBy*`, `updateMatrixWorld`, `updateWorldMatrix`, and the
parent-aware `lookAt`/`localToWorld`/`worldToLocal`/`getWorld*` — are on the
`Object3DNode` trait, implemented for `Node`. The transform-only methods
(`rotateX`, `translateOnAxis`, `applyMatrix4`, `updateMatrix`, …) stay inherent
methods on `Object3D`, so the existing by-value API that `Mesh`,
`InstancedMesh`, `PerspectiveCamera` and `src/renderer` use is untouched.

## Why not an arena

An arena of `Vec<Object3D>` plus `Handle(u32)` indices is the usual Rust answer
and it is better for cache locality on a per-frame walk, but it loses on the two
things that actually shape this port:

- **Three's API is the spec.** Every three.js method takes and returns object
  references (`scene.add( mesh )`, `geometry.boundingSphere`,
  `object.parent.matrixWorld`). With `Rc<RefCell<..>>` each one ports to a line
  of Rust with the same shape and the same operation order, which is the whole
  discipline of this port; with handles every call needs an arena argument
  threaded through it, and Three's own unit tests — our anchor — have no arena
  to pass.
- **Object lifetime is not scene-scoped.** Examples build objects, `attach()`
  them between parents, and hold references to them outside the tree (the
  dummy `Object3D` in `webgpu_instance_mesh`, the camera, render-target quads).
  An arena forces either generational handles or a second allocation story for
  everything that is not currently parented.

The per-frame cost is a `RefCell` borrow flag check and a pointer chase per
node. The renderer does not pay it per draw: it walks the tree once per frame
into a flat render list (`Scene.children` today), and that list is what the
draw loop iterates.

`Rc`, not `Arc`: three.js' scene graph is single-threaded and `WebGPURenderer`
never touches an `Object3D` off the main thread, so there is nothing to gain
from atomics. If the renderer is ever parallelised, the swap is mechanical.

## Divergences from three.js

- `Object3D.parent` is weak, so `object.parent()` returns `Option<Node>` by
  upgrading. In JS the parent link is strong and the cycle is the GC's problem.
- `getObjectByProperty( name, value )` has no Rust equivalent of dynamic
  property lookup; it takes a predicate (`&dyn Fn(&Object3D) -> bool`), and
  `getObjectById`/`getObjectByName` are built on it exactly as in three.js.
- `Clone for Object3D` is three.js' `copy( source, recursive = false )` minus
  the tree: a fresh `id`, no parent, no children. Sharing the children's `Rc`s
  between two objects would give both the same child list with one `parent`
  pointer between them.
- No `EventDispatcher`, so `add`/`remove`/`attach` dispatch no `added`,
  `removed`, `childadded` or `childremoved` events.
