# Ownership model for the scene graph

`Object3D` is the node payload; the tree is

```rust
pub struct Node(Rc<RefCell<Object3D>>);      // Object3D.children entries
pub struct WeakNode(Weak<RefCell<Object3D>>); // Object3D.parent
```

Both are newtypes, as of 0.2.0 (#38); they were type aliases in 0.1.0.
`Node` derefs to `RefCell<Object3D>`, so `node.borrow()` and
`node.borrow_mut()` reach the object exactly as they did through the alias and
every call site is the same text. What the newtype buys is the rest of the
surface: rustdoc prints `Node` in every signature instead of the plumbing, and
the tree methods are **inherent**, so `light.add(&mesh)` compiles with no
import. `WeakNode` has no `Deref` — the only thing to do with a weak handle is
`upgrade()`, which hands back an `Option<Node>`.

Beside the ported methods, `Node` carries the `Rc` associated functions a
consumer would otherwise reach for: `Node::ptr_eq(a, b)` for `a === b`,
`downgrade()`, `as_ptr()` for anyone keying a cache on the address (but read
"Identity and eviction" below first — the renderer keys on `object.id`, and
deliberately), and `Node::new(object)` / `Object3D::into_node()` to wrap one.

A parent holds its children strongly, a child holds its parent weakly. All the
methods that reach outside a single object — `add`, `remove`, `attach`,
`traverse*`, `getObjectBy*`, `updateMatrixWorld`, `updateWorldMatrix`, and the
parent-aware `lookAt`/`localToWorld`/`worldToLocal`/`getWorld*` — are inherent
on `Node`, because each of them has to reach outside the object it is called on
and a `&mut Object3D` cannot. The transform-only methods (`rotateX`,
`translateOnAxis`, `applyMatrix4`, `updateMatrix`, …) stay inherent methods on
`Object3D`, so they stay available on a plain `&mut Object3D` — which is what
`PerspectiveCamera` and the `Object3D` the examples use as a transform
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
  upgrading, and the field itself is a `WeakNode`. In JS the parent link is
  strong and the cycle is the GC's problem.
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
   root, recursing through `Node`'s children. It honours `matrixAutoUpdate`
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
pub enum Payload { None, Mesh(Mesh), InstancedMesh(InstancedMesh), Line(Line), Light(PointLight) }
```

`Payload::None` is a plain `Object3D`, a `Group` or a `Bone`: something the walk
passes through without drawing. `object.is_mesh()` is a match on the payload, and
`Mesh::new( geometry, material )` / `InstancedMesh::new( geometry, material,
count )` / `Line::new( geometry, material )` /
`LineSegments::new( geometry, material )` return a `Node` with the payload
already set, so example code reads like the JS:

```rust
let mesh = Mesh::new( geometry.clone(), material );
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
- a mesh **or a line** is culled when `frustumCulled` is set and its geometry's
  bounding sphere, pushed through `matrixWorld`, misses the frustum; then
  skipped again if its material is not `visible`; otherwise pushed with `z` =
  the bounding-sphere centre in clip space (a `Vector4`, no perspective divide).
  three.js has one arm for all three of `isMesh || isLine || isPoints`, and so
  does `project_drawable()`: nothing it does depends on the primitive. What the
  object *is* matters one step later, in the pipeline — see "Lines" below.
- a `LineLoop` is an error. `_projectObject()` calls `error( 'Renderer: Objects
  of type THREE.LineLoop are not supported. Please use THREE.Line or
  THREE.LineSegments.' )`, so the port has no `LineLoop` type at all.

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

A transparent material therefore draws after every opaque one, back to front, and
its per-fragment alpha survives: `DiffuseColor.w = 1.0` is emitted only under
`builder.isOpaque()`, and the blend state the pipeline gets comes from
`material.blend_state()` — `docs/nodes.md` §9.1 has the table and the gate. Blend
state is part of `RenderState` and so part of the pipeline cache key, which is
what lets one generated program serve an opaque and a blended draw.

### Per-draw resources

Everything a draw binds — bind groups, and now vertex buffers — is resolved from
the `NodeProgram` built for *that material*, never from the compiled `Program`.
The program cache is keyed on the generated WGSL, so two materials with
identical shaders and different textures or `range()` buffers hash to one
entry; reading resources off the cached entry would quietly hand the second
draw the first's data, which is why `Program` holds the shader modules, the
bind-group layouts and the vertex layout *shape* and not a single binding
description. Vertex buffers come from `NodeProgram::vertex_buffers()`
(`docs/nodes.md` §9.2), which is also what the pipeline's vertex layouts were
built from, so a bound buffer and its layout cannot disagree. Per-node GPU
buffers are cached by node identity and uploaded once; the instance matrix is
the exception, re-uploaded each frame because its contents change.

### Program cache

`Renderer::draw` does not run the node builder on a steady frame. It is the
structure of `RenderObjects.get()` + `NodeManager.getForRender()`, ported:

| three.js | here |
|---|---|
| `Material.id`, `Material.version`; `needsUpdate = true` bumps `version` | `MeshBasicNodeMaterial.id`, `.version`; `set_needs_update()` |
| `RenderObject.getMaterialCacheKey()` | `(material.id, material.version)` |
| `RenderObject.getDynamicCacheKey()` — `lightsNode.getCacheKey()`, fog, environment, `receiveShadow`, shadow-map enabled | the hash of the item's `SetupContext` (lights with their shadow maps, the instancing branch, the morph entry) and its `FogNode`, by node identity |
| `RenderObjects.get()`: `renderObject.version !== material.version` → `dispose()` | a material seen at a new version drops every state it had |
| `NodeManager.nodeBuilderCache`: `cacheKey → NodeBuilderState` | `Renderer::node_builder_states`: `material.id → { version, dynamic key → Rc<NodeProgram> }` |
| `NodeBuilderState` shared by cache key | `Program` shared by the WGSL-and-bindings hash the built `NodeProgram` carries |

So the key is **material identity × material version × the scene-dependent
part**. A miss runs `materials::setup()` and `NodeBuilder::build`; a hit hands
back the material's own `NodeProgram`, whose binding descriptions the draw
resolves its resources from. `Renderer::program_builds()` counts the misses,
and the e2e harness (`steady_frame_builds_nothing`) renders every graded rung
three times and asserts the third frame adds none.

Materials are values, so identity is a counter the way `_materialId` is — and
**`clone()` allocates a new id**, as `Material.clone()` gives a new object.
The renderer's own per-frame snapshot of a material takes the key from the
source before cloning; the materials the renderer derives by value (the
shadow-pass material, the quad's, the background's, the output pass's) carry
the source's id and a `variant` naming the derivation, where three.js has a
separate material object for each.

**The `needsUpdate` rule is three.js'.** After changing a material field the
*program* depends on — a node (`color_node`, `position_node`,
`fragment_node`, …), a map, `kind`, `lights`, `flat_shading`, `fog`,
`transparent`, `blending` — call `material.set_needs_update()`, as the examples
do with `material.needsUpdate = true`; otherwise the previous program keeps
drawing. Fields the program reads as uniforms (`color`, `opacity`, `shininess`,
`metalness`, …) are uploaded every frame and need nothing, and pipeline state
(`side`, depth flags, the blend factors) is keyed per draw. The struct docs on
`MeshBasicNodeMaterial` carry the list. `BatchedText` is the in-tree case: its
attribute arrays travel in the node graph, so a `sync()` that changed them
rebuilds the nodes and bumps the version, and one that changed nothing does
neither.

### Identity and eviction

Every per-draw cache the renderer keeps — uploaded geometry, a material's built
programs, a `range()` buffer's one-and-only fill, a geometry's morph texture,
an uploaded texture — is keyed on an **id from a never-reused counter**:
`BufferGeometry.id`, `Material.id`, `BufferId`, `TextureId`, each three.js'
`_id ++` on the matching class. None is keyed on an address.

That is not a detail. The address of an `Rc` is only unique among *live*
objects: drop a geometry and the next one can be allocated at the same address,
and a cache keyed that way hands the new object the dead one's GPU buffers —
a panic when their attribute sets differ (`the geometry has no uv attribute`),
the wrong shape drawn silently when they do not. Issue #58; it took 17
allocations to hit in the reported repro. A counter cannot collide, so a
lookup can only ever find what the object itself put there.

Eviction is then purely about not growing forever, and takes the cheapest
correct form for each kind of key. It happens once, at the top of `render()`:

| cache | key | how an entry dies |
|---|---|---|
| `geometries` | `BufferGeometry.id` | a `Weak` beside the entry: strong count zero means the consumer dropped it |
| morph textures | `BufferGeometry.id` | the same `Weak`, swept on the next `get_entry()` |
| `node_builder_states` | `material.id` | unused for `CACHE_GRACE_RENDERS` (4) renders |
| `buffers` (`range()`) | `BufferId` | unused for `CACHE_GRACE_RENDERS` renders |

A geometry is an `Rc`, so its strong count *is* three.js' `dispose` event —
exact and immediate, and it costs one `Weak` per entry. A material is a value
here (the renderer only ever sees per-frame clones) and a `BufferNode` lives
inside a material's node graph, so neither has a count to read; those age out
instead. The window is four renders rather than one so that a consumer
alternating two scenes, or interleaving passes, does not evict one on the
other's frame and rebuild it every time. A steady frame touches every entry, so
a steady frame evicts nothing and still builds nothing — which
`steady_frame_builds_nothing` keeps honest, and
`churning_geometry_and_materials_does_not_grow_the_caches` checks from the
other side.

Two caches deliberately have no eviction: `programs` and `pipelines` are keyed
by the *content* hash of the generated WGSL and the pipeline state, so distinct
entries are distinct shaders, and their number is bounded by the material
shapes the program uses, not by how many objects it creates. Uploaded textures
are keyed by `TextureId` — correct, never stale — but are still not swept; a
consumer that churns textures holds their GPU memory for the life of the
renderer. That is the remaining leak, and a follow-up.

## Changing geometry

**A geometry is uploaded once per id, and the upload is refreshed only where
an attribute says so.** The renderer sees `BufferGeometry.id` and finds its
buffers; what it then re-reads is one number per attribute,
`BufferAttribute.version`, against the version it recorded when it wrote that
buffer. Writing into the array alone still changes nothing on screen — it is
the version that the renderer looks at.

So there are two routes, three.js' two:

- **`attribute.array_mut()`, then `attribute.set_needs_update()`** —
  three.js' `attribute.needsUpdate = true` (issue #47). The array is behind a
  `RefCell` and the version behind a `Cell`, so both work through the `Rc` a
  mesh is holding: the geometry keeps its id, its entry and every buffer that
  did not change, and the next `render()` re-writes exactly the one that did.
  A write of the same byte length is a `queue.write_buffer` into the buffer
  that is already there; one that changed length has to allocate a new buffer,
  since a `wgpu::Buffer` is a fixed size. Either way it is one
  `info.build.buffers_written` and no `geometries_uploaded`, which
  `a_mutated_attribute_rewrites_one_buffer` in `tests/e2e` asserts. Only
  `position`, `normal` and `uv` are versioned; every other attribute (`color`
  for a `vertex_colors` material, whatever a node graph's `attribute( name )`
  names) is uploaded once and not refreshed, and the index is not versioned
  either, because `BufferGeometry.index` is an `Index`, not a
  `BufferAttribute`, so it has no `needsUpdate` to read.
- **Make a new `BufferGeometry`** and hand it to the mesh. That is a fresh id,
  so the whole geometry is uploaded, and dropping the old one drops its buffers
  at the next `render()`. This is the route for a changed index, a new
  attribute, a changed `color` or other extra attribute, or a different
  attribute set. A `clone()` counts as a new
  geometry: `GeometryId::clone` mints a fresh id exactly as `MaterialId::clone`
  does, so a geometry cloned, mutated and drawn shows its mutation.

`Line::set_positions()` is the first route spelled once for the common case —
rewrite the `position` array and mark it — since a consumer moving one line of
a diagram is what issue #47 was reported from.

**The bounds follow the same rule.** Culling asks every object for its
geometry's bounding sphere every frame, and the render-list sort asks for its
centre, so `BufferGeometry` caches the box and sphere it computes (issue
#133), keyed on the `position` attribute's id and version, each `position`
morph target's id and version, and `morph_targets_relative`. An edit through
`array_mut()` is therefore seen by culling only after `set_needs_update()`,
exactly as it is seen by the GPU buffer; the `&mut` methods that swap or hand
out an attribute (`set_attribute`, `delete_attribute`, `set_morph_attribute`,
`get_attribute_mut`, and so `apply_matrix4`, `translate`, `center` and the
rest) drop the cache outright, and a clone's fresh attribute ids never match
its source's key. This is stricter than three.js, where `boundingSphere` is
computed once and never invalidated by itself: a three.js app that moves
vertices has to call `computeBoundingSphere()` again or be culled against the
old sphere. The pub `bounding_sphere` override field still wins over the
cache, as before.

## Lines

`Line` and `LineSegments` are `Payload::Line( Line )`, one variant with an
`is_line_segments` flag, because that is all `LineSegments extends Line` adds.
They share the render list, the sort and the frustum cull with meshes, and they
share `MeshBasicNodeMaterial`: `LineBasicNodeMaterial` is a bare `NodeMaterial`
with `LineBasicMaterial`'s defaults, every one of which `MeshBasicNodeMaterial`
already has, so it is the constructor `MeshBasicNodeMaterial::line( color )` and
not a `MaterialKind` (Three's own dump of it is in `docs/lines/`).

What separates a line from a mesh is the **pipeline**, and the input is the
object, not the material:

```
WebGPUUtils.getPrimitiveTopology( object, material )
  isPoints                                  -> point-list   (not ported)
  isLineSegments || ( isMesh && wireframe ) -> line-list     (wireframe not ported)
  isLine                                    -> line-strip
  isMesh                                    -> triangle-list
```

`renderer::Primitive::of()` is that function plus `_getPrimitiveState()`'s
`stripIndexFormat`, which is set only for an *indexed* `Line` that is not a
`LineSegments`. Both travel on the `Renderable` into `RenderState`, so they are
part of the pipeline cache key: one white `LineBasicNodeMaterial` shared by a
`Line` and a `LineSegments` is one program and two pipelines. Culling and the
front face still come from `material.side`, as they do in three.js.

Lines go through the shadow pass unchanged — in three.js the shadow pass is an
ordinary `renderer.render()` with an override material, so a line casts a
hairline shadow — but `computeLineDistances()` is not ported, since only
`LineDashedMaterial` reads it, and `Line.morphTargetInfluences` is always empty.

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
- No `LOD`, `Sprite`, `Points`, `BatchedMesh` or `BundleGroup` arm in
  `project_object`, no multi-material `geometry.groups` arm, no clipping context
  and no `transparentDoublePass` (transmission).
- `PointLight` and `AmbientLight` exist (rungs 5 and 6). `DirectionalLight`,
  `SpotLight` and `HemisphereLight` are further lighting rungs; so are shadows
  (rung 7).
