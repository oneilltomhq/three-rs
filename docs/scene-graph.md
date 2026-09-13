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
methods on `Object3D`, so they stay available on a plain `&mut Object3D` — which
is what `PerspectiveCamera` and the `Object3D` the examples use as a transform
scratchpad still hold.

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
into a flat render list (`RenderList`, below), and that list is what the draw
loop iterates.

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

## The render path

`Renderer::render( scene, camera )` is `WebGPURenderer.render()`, in the same
order:

1. `scene.update_matrix_world()` — `Object3D.updateMatrixWorld()` on the scene
   root, recursing through `Object3DNode`. It honours `matrixAutoUpdate`
   (whether the local matrix is recomposed), `matrixWorldAutoUpdate` (whether
   this object's world matrix is written) and `matrixWorldNeedsUpdate`, and
   threads three.js' `force` down the tree: an object that did recompute forces
   every descendant to recompute against it. `tests/core_object3d.rs`'s
   `update_matrix_world` is the ported QUnit case for all of that.
2. `camera.update_matrix_world()`.
3. `Renderer::project_scene()` — `RenderList::new()`, then
   `renderer::project_object()` over the whole tree, then `RenderList::sort()`.
4. the skybox is `unshift`ed onto the front of the sorted opaque list, exactly
   where `Background.update()` puts it, and the list is drawn.

### What a node *is*

`Object3D` carries a `payload: Payload` — the state three.js gets from
subclassing:

```rust
pub enum Payload { None, Mesh(Mesh), InstancedMesh(InstancedMesh), Light(PointLight) }
```

`Payload::None` is a plain `Object3D`, a `Group` or a `Bone`: something the walk
passes through without drawing. `object.is_mesh()` is a match on the payload, and
`Mesh::new( geometry )` / `InstancedMesh::new( geometry, material, count )` return
a `Node` with the payload already set, so example code reads like the JS:

```rust
let mesh = Mesh::new( geometry.clone() );
mesh.borrow_mut().position.set( x, y, z );
scene.add( &mesh );
```

`Scene` is not itself a `Node`; it owns one (`scene.node`, with `is_scene` true)
plus the fields `Scene` adds to `Object3D` — `background`, `fogNode` and
`overrideMaterial`. `scene.add()`, `scene.children()` and
`scene.update_matrix_world()` forward to the root, so anything can nest under
anything: a `Group` holding meshes, a light holding its bulb mesh
(`webgpu_lights_phong`, rung 5), a loaded glTF hierarchy (rung 10).

`PerspectiveCamera` owns a `Node` (`camera.node`) as of rung 6:
`webgpu_morphtargets` does `scene.add( camera )` and parents its point light to
the camera, so the camera is both *in* the tree and a parent within it, which a
bare `Object3D` cannot express. `camera.update_matrix_world()` walks the node and
takes the inverse of its world matrix. `OrthographicCamera` still holds an
`Object3D` by value — nothing on the ladder nests one yet, and rung 7's shadow
cameras are the trigger to give it a `Node` too.

### projectObject

`renderer::project_object()` is `Renderer._projectObject()`, and its gates are
deliberately asymmetric, as three.js' are:

- `visible === false` returns immediately — a hidden object hides its whole
  subtree.
- failing `object.layers.test( camera.layers )` skips only *this* object's own
  render item. Its children are still projected.
- a `Group` replaces the inherited `groupOrder` with its own `renderOrder` for
  everything below it.
- a light (`is_light`) goes into `RenderList.lights` and is never drawn — but its
  children still are, which is how rung 5's bulb spheres reach the draw list (see
  "Lights in the tree" below).
- a mesh is culled when `frustumCulled` is set and its geometry's bounding sphere,
  pushed through `matrixWorld`, misses the frustum; then skipped again if its
  material is not `visible`; otherwise pushed with `z` = the bounding-sphere
  centre in clip space (a `Vector4`, no perspective divide).

### Ordering

`RenderList` keeps two arrays and sorts them with three.js' own comparators:

| list | sort keys |
|---|---|
| `opaque` | `groupOrder`, `renderOrder`, `z` ascending, `id` |
| `transparent` | `groupOrder`, `renderOrder`, `z` *descending*, `id` |

`material.transparent` decides which array an item lands in. `slice::sort_by` is
stable, as `Array.prototype.sort` is, and the `id` tie-break makes the order total
anyway. The `id` is `Object3D.id`, a creation counter, so two objects at the same
depth draw in construction order — which is what the pre-tree-walk renderer got
from a stable sort over a flat `Vec` in `scene.add()` order. That is why folding
`Child` into the tree changed no pixels.

`Renderer.sort_objects` mirrors `Renderer.sortObjects`. With it off three.js
leaves each item's `z` at whatever `_vector4` last held; here it stays 0, so the
lists keep traversal order.

## Lights in the tree

A light is an ordinary node: `PointLight::new( color, intensity, distance )`
returns a `Node` whose payload is `Payload::Light( PointLight )` and whose
`object.is_light` is true, and it goes in with plain `scene.add( &light )`. There
is no `Scene.lights`, no `add_light()` and no `Scene::drawables()`; rung 5 had all
three and the tree walk removed the need for them:

```rust
let light = PointLight::new( Color::from_hex( 0x0040ff ), 1.0, 100.0 );
light.borrow_mut().light_mut().unwrap().set_power( 1700.0 );
light.add( &bulb );        // an ordinary child — it draws through the walk
scene.add( &light );
```

`is_light` is what `project_object()` branches on, exactly as three.js'
`_projectObject()` reads `object.isLight`; the payload is what the renderer then
*reads* (`object.light()` → colour, intensity, `distance`, `decay`) and
`matrixWorld` on the node itself is the world position. The bulb inherits the
light's world matrix for free, because it is a child.

`RenderList.lights` is three.js' `lightsArray`: **scene-traversal order**. But
that is not the order the shader sees: `LightsNode.setupLightsNode()` runs
`sortLights( lights )`, which is `lights.sort( ( a, b ) => a.id - b.id )` —
**creation** order. The renderer sorts `render_list.lights` by `Object3D.id`
before building `LightState`, so `UniformSource::Light*( i )` and
`material.lights_node = Some( vec![ 0 ] )` (that is `lights( [ light1 ] )`) both
index the sorted list. `material.lights_node = Some( vec![ 0 ] )`
is `lights( [ light1 ] )` — an index into that list. In
In `webgpu_lights_phong.html` the two orders agree — the four lights are created
and `scene.add()`ed in the same order, before the three teapots. In
`webgpu_morphtargets.html` they do not: the `AmbientLight` is created first but
the `PointLight` is a child of the camera, which is added to the scene first, so
the walk reaches the point light first and only `sortLights()` puts the ambient
light's `irradiance` statements ahead of it, as the dump has them.

## What is not wired up yet

- `SkinnedMesh` is still a sibling struct owning its own `Node` rather than a
  `Payload` variant, so the walk does not draw it. Rung 10 adds
  `Payload::SkinnedMesh` and moves `geometry`/`skeleton` into it.
- No `LOD`, `Sprite`, `Line`, `Points`, `BatchedMesh` or `BundleGroup` arm in
  `project_object`, no multi-material `geometry.groups` arm, no clipping context
  and no `transparentDoublePass` (transmission).
- `PointLight` and `AmbientLight` exist (rungs 5 and 6). `DirectionalLight`,
  `SpotLight` and `HemisphereLight` are further lighting rungs; so are shadows
  (rung 7).
