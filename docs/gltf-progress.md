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
- `tests/gltf_loader.rs` — 5 tests, all checked to 1e-6 against three.js'
  own `GLTFLoader` run under node: `tests/gltf/samples/sample.mjs` (note the
  `loader.register` texture stub — `loadImageSource` touches `self.URL`, which
  node has not got). Michelle: 68 nodes and their names, 1 skin / 65 bones,
  16340 positions / 84318 indices, first three vertices, skinIndex exactness,
  boneInverses[0], 2 clips with durations and 195 tracks each, track 0 values.
  Soldier: 69 nodes, 2 skins (49 + 2 bones), 4 clips × 156 tracks, counts.

## Next, in order

1. **TargetResolver for the Object3D tree** (task item 3): implement
   `findNode` + the getter/setter half of `PropertyBinding` for `.position`,
   `.quaternion`, `.scale`, `.morphTargetInfluences` against
   `src/animation/binding_target.rs`'s `BindingTarget` / `TargetResolver`. Only
   after that can `mixer.clipAction( clip, root ).play(); mixer.update( 0 )`
   write into the tree. The `morphTargetInfluences` case needs the influences to
   live somewhere reachable from a `Node` — they are currently on
   `SkinnedMesh`/`GltfPrimitive`, not on `Object3D`, so that resolver arm has to
   take the primitive list alongside the root.
2. **The t=0 bone-world-matrix test**: play `SambaDance` at t=0 and compare a few
   `bone.matrixWorld` against three's. `sample.mjs` already has the harness; add
   an `AnimationMixer` + `mixer.update( 0 )` to it.
3. **CUBICSPLINE**: currently reduced to LINEAR by dropping the in/out tangents
   (keeping only the middle value of each triple). Correct only for one-keyframe
   tracks; neither test asset uses it. Needs `GLTFCubicSplineInterpolant`.
4. **Images/textures**: decode PNG/JPEG through `TextureLoader` (its API takes a
   path, not bytes — that needs a `from_bytes` entry point). Today only the bytes
   and the mimeType come out.
5. **Primitive dedup + groups**: three.js' `createPrimitiveKey` shares a geometry
   between primitives and the task's "groups when a primitive is split" follows
   from that. Not ported; each primitive gets its own geometry and its own child
   node named `<mesh>_<i>`.
6. Not ported at all: extensions (`KHR_*`, Draco, meshopt), cameras, lights,
   `GLTFMeshStandardSGMaterial`, `Mesh`/`Points`/`Line` modes (everything is
   treated as `TRIANGLES`).
