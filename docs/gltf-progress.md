# rung 10 data side — progress

Branch `gltf`, cut from `port` at d1c8f3f. Scope: the data side of
`webgpu_skinning` (GLTFLoader addon, SkinnedMesh, AnimationMixer at t=0), so
the rung worker only has to do the skinning node and the render.

**The example loads `Michelle.glb`, not `Soldier.glb`** (`examples/webgpu_skinning.html`
line 74), and plays `gltf.animations[ 0 ]` = `SambaDance`. Soldier.glb is tested
too because it exercises two skins and a split mesh.

## Done

- `src/objects/bone.rs`, `skeleton.rs`, `skinned_mesh.rs` — `Bone`, `Skeleton`,
  `SkinnedMesh` (bindMode, bindMatrix/Inverse, bind, pose, normalizeSkinWeights,
  applyBoneTransform, computeBoundingBox/Sphere). `Bone` is an `Object3D` node
  marked by `object_type == "Bone"`: `Object3D` has no `is_bone` field and the
  parallel renderer worker owns core additions. `SkinnedMesh` owns a `Node`
  rather than an `Object3D` (like `Mesh` owns an `Object3D`) because the bones
  are in the tree and both ends need `matrixWorld`.
  `Skeleton::bone_matrices` + `bone_texture_size()` are the renderer's hook; the
  `DataTexture` itself is not built here.
- `Gltf::scene_resolver()` (the animation root, ready to hand the mixer) and
  `Gltf::primitive( &node )` (node to geometry/material/skin, since `Object3D`
  carries no geometry) are the two entry points the rung worker wants.
- `src/loaders/gltf_loader.rs` — GLB container (header/JSON/BIN chunks), .gltf
  JSON, `data:` URIs (own base64 decoder), buffers/bufferViews/accessors (every
  component type, `normalized`, `byteStride` incl. interleaved, `sparse`),
  meshes/primitives into `BufferGeometry` with three.js' `ATTRIBUTES` renaming,
  index (u16 when it fits), morph targets with `morphTargetsRelative = true`,
  nodes into the `Object3D` tree (matrix or TRS) with `createUniqueName` /
  `sanitizeNodeName`, scenes, skins into `Skeleton` (inverseBindMatrices, joints
  as `Bone` nodes), animations into `AnimationClip` via the existing
  `KeyframeTrack` types and `PATH_PROPERTIES`, materials/textures/images as data
  records (`GltfMaterial` keeps the PBR fields; `GltfImage` keeps the bytes).
- `src/animation/object3d_target.rs` — `PropertyBinding`'s getter/setter half
  for the `Object3D` tree: `SceneResolver` is a `TargetResolver` (`findNode`
  included, skeleton branch and all), `NodeTarget` the `BindingTarget` for
  `.position` / `.quaternion` / `.scale` / `.morphTargetInfluences`, each a
  `HasFromToArray`-or-`EntireArray` setter with
  `Versioning.MatrixWorldNeedsUpdate`. `objectName`, `propertyIndex` and every
  other property are explicit TODOs returning `None` (three's "it wasn't
  found"). `morphTargetInfluences` lives in an `Rc<RefCell<Vec<f64>>>` on
  `SkinnedMesh`/`GltfPrimitive` and is registered with the resolver by node id,
  because `Object3D` has no such field.
- `tests/gltf_loader.rs` — 6 tests, all checked to 1e-6 against three.js'
  own `GLTFLoader` run under node: `tests/gltf/samples/sample.mjs` (note the
  `loader.register` texture stub — `loadImageSource` touches `self.URL`, which
  node has not got). Michelle: 68 nodes and their names, 1 skin / 65 bones,
  16340 positions / 84318 indices, first three vertices, skinIndex exactness,
  boneInverses[0], 2 clips with durations and 195 tracks each, track 0 values.
  Soldier: 69 nodes, 2 skins (49 + 2 bones), 4 clips × 156 tracks, counts.
  Plus `michelle_mixer_at_zero`: `clipAction( SambaDance ).play()`,
  `update( 0 )`, `updateMatrixWorld( true )`, and two bone `matrixWorld`s
  (`mixamorigHips`, `mixamorigLeftHand`) against three's, to 1e-6. And
  `michelle_skinning_at_zero`: `Skeleton::update()`'s first two flattened bone
  matrices and `applyBoneTransform( 0, … )`, also against three's at t = 0.

## Two things that will bite the rung worker

- `GLTFLoader` binds with the **identity** bind matrix
  (`mesh.bind( skeleton, _identityMatrix )`) — glTF joints are already relative
  to the skin. The work is done by `bindMatrixInverse`, which in the default
  `AttachedBindMode` is recomputed from `matrixWorld` — so
  **`SkinnedMesh::update_matrix_world()` has to run every frame**, not just
  `node.update_matrix_world()`. Getting this wrong scales the skin by the node's
  scale (a factor of 100 on Michelle) and is silent.
- The order the tests use, and the renderer must use: mixer writes the tree →
  `scene.update_matrix_world( true )` → `skinned_mesh.update_matrix_world( true )`
  → `skeleton.update()` → upload `bone_matrices`.

## Next, in order

1. **Hand the renderer the skinning inputs**: `Skeleton::update()` then
   `bone_matrices` into a `DataTexture` (`Skeleton.computeBoneTexture`);
   `bone_texture_size()` is the hook. The renderer also needs the
   `SkinnedMesh` in its draw list. `Child` is gone: the renderer now walks the
   real tree and draws whatever node carries a `Payload::Mesh` /
   `Payload::InstancedMesh`, so the rung worker adds a `Payload::SkinnedMesh`
   variant and moves `SkinnedMesh`'s geometry/skeleton into it (see
   `docs/scene-graph.md`).
2. **CUBICSPLINE**: currently reduced to LINEAR by dropping the in/out tangents
   (keeping only the middle value of each triple). Correct only for one-keyframe
   tracks; neither test asset uses it. Needs `GLTFCubicSplineInterpolant`.
3. **Images/textures**: decode PNG/JPEG through `TextureLoader` (its API takes a
   path, not bytes — that needs a `from_bytes` entry point). Today only the bytes
   and the mimeType come out.
4. **Primitive dedup + groups**: three.js' `createPrimitiveKey` shares a geometry
   between primitives and the task's "groups when a primitive is split" follows
   from that. Not ported; each primitive gets its own geometry and its own child
   node named `<mesh>_<i>`.
5. Not ported at all: extensions other than `SUPPORTED_EXTENSIONS` (AVIF
   textures), cameras, lights,
   `GLTFMeshStandardSGMaterial`, `Mesh`/`Points`/`Line` modes (everything is
   treated as `TRIANGLES`).
6. **meshopt** (`EXT_meshopt_compression`, issue #173): a compressed bufferView
   is decoded the first time it is read, by `src/loaders/meshopt.rs`: the
   vertex and index codecs from the `meshopt-rs` crate, the four filters ported
   from meshoptimizer 1.1's SIMD kernels (the crate's quaternion filter is
   wrong and its others are the scalar variants; the module header has the
   numbers). `KHR_mesh_quantization`, which both meshopt assets also require,
   needs nothing more than the accessor reading already does.
   `tests/gltf_meshopt.rs` compares `coffeemat.glb` and `facecap.glb` (the only
   two meshopt assets in the examples) with three.js' `MeshoptDecoder`, run
   under node by `tools/meshopt_reference.mjs`: every compressed bufferView
   byte for byte, every accessor, and every built primitive. It also sweeps
   the filters and the `INDICES` mode, which the assets do not reach, against
   the same decoder. Both assets also need `KHR_texture_basisu`, which
   `Ktx2Loader` (issue #172) now reads; neither graded example that uses them
   (`webgpu_loader_gltf_compressed`, `webgpu_morphtargets_face`) is ported yet.
7. **Draco** (`KHR_draco_mesh_compression`, issue #139): decoded with the
   `draco-core` crate, then turned into the typed arrays `DRACOLoader` hands
   `GLTFLoader` by `src/loaders/draco.rs`. `tests/gltf_draco.rs` compares
   every Draco asset in the three.js examples with `DRACOLoader` itself, run
   under node by `tools/draco_reference.mjs`; all fifteen match bit for bit.
   The AVIF one (`forest_house.glb`) still stops at load on its texture
   extension.

## Running the tests

`cargo test` runs the e2e tests in parallel with everything else, which on this
one GPU produces a spurious e2e failure (`handoff/RUNGS.md` says the same about
two worktrees at once). Use `cargo test --lib --tests --exclude-nothing` style
runs for the unit work and
`cargo test --test e2e -- --nocapture --test-threads=1` on its own for the
images. Last full run on this branch: 645 unit/integration tests green, e2e
0 / 45 / 0 / 1 pixels.
