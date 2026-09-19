# Rung 10 — webgpu_skinning

Branch `rung10-skinning` from `main` 2cf90bd. **PASS: 6 of 100000 pixels
differ (0.006%), limit 0.1%.**

Michelle.glb, `SambaDance` at t = 0, a `MeshPhysicalNodeMaterial` with five
textures, a `screenUV` background gradient and `LinearToneMapping` at exposure
0.4.

## Step 1 — the plan against the current tree

The scout's `PLAN.md` was written on 2026-09-13 and its data half landed since
(`docs/gltf-progress.md`). The deltas:

* **The data half is done.** GLTFLoader, `Bone` / `Skeleton` / `SkinnedMesh`,
  `SceneResolver` / `NodeTarget`, `AnimationMixer` and
  `tests/gltf_loader.rs`'s bone-matrix pins are all on main. Only the render
  half was left.
* **`Node` is a newtype** over `Rc<RefCell<Object3D>>` with the tree methods
  inherent, so `SkinnedMesh` could not be "a mesh beside the tree": it is a
  `Payload` variant holding a `Mesh`, the way `InstancedMesh` already is. The
  glTF loader installs it on the node the skin's joints already point at,
  rather than constructing a second object.
* **`Mesh::new( geometry, material )`** takes the material, and loaders return
  `Result`, so `GltfPrimitive` carries the geometry and the loader hands the
  built material to `SkinnedMesh::of()`.
* **No bone texture.** The plan left the choice open. The dump settles it:
  `array< mat4x4<f32>, 65 >` in a uniform buffer (4160 bytes, far under the
  65536 limit), so only `getBoneMatricesNode()`'s uniform branch is ported.

## What was added

| Where | What |
|---|---|
| `src/objects/skinned_mesh.rs`, `payload.rs` | `SkinnedMesh` as a `Payload` variant; `normalize_skin_weights`, `apply_bone_transform`, the bounding volumes |
| `src/core/node.rs` | `update_bind_matrix_inverse` inside `update_own_matrix_world` — `AttachedBindMode` recomputes `matrixWorld⁻¹` every frame |
| `src/nodes/skinning.rs` | `SkinningNode`: the bone uniform buffer, `getSkinnedPosition`, `getSkinnedNormalAndTangent` |
| `src/nodes/node.rs` | `Type::UVec4`, `UniformSource::BindMatrix` / `BindMatrixInverse`, `BufferSource::BoneMatrices` |
| `src/core/buffer_geometry.rs` | `BufferAttribute::new_integer` — the port stores every attribute as `f32`, but `skinIndex` reaches the GPU as `uint32x4` |
| `src/materials/mod.rs` | `MaterialKind::Physical`, `Material::physical()`, `normal_map` / `normal_scale` / `specular_color_map`, `ToneMapping::Linear` |
| `src/materials/node_material.rs` | `setupSpecular()`'s ior/specular block; `negateOnBackSide` for a `DoubleSide` normal |
| `src/loaders/texture_loader.rs` | `TextureLoader::from_bytes()` with a PNG decode |
| `src/loaders/gltf_loader.rs` | `GLTFParser.loadMaterial` + `assignTexture`, `KHR_materials_specular`, `KHR_materials_ior`, the `useDerivativeTangents` clone |
| `src/nodes/tsl.rs`, `src/objects/scene.rs` | `screenUV`; `Background::Node` holds any `NodeRef`, not a `Color` |
| `examples/webgpu_skinning.rs`, `tests/e2e/main.rs`, `src/bin/viewer.rs` | the example, its graded entry, its `rung!` entry and viewer key `a` |
| `examples/dump_wgsl.rs` | `skinning_background` and `skinning_body` sections |

## What the pixels found

* **The identity bind matrix is not a no-op.** `GLTFLoader` binds with
  `_identityMatrix`, so `bindMatrix` is identity — but `bindMatrixInverse` is
  `matrixWorld⁻¹` and Michelle's root node has a 0.01 scale. Leaving the
  inverse at identity renders her a hundred times too large, with no error
  anywhere. It has to be recomputed inside the matrix-world walk, not once at
  bind time.
* **`skinIndex` must be an integer format.** Uploading it as `float32x4` and
  casting in the shader costs precision above 2^24 and, more to the point,
  generates WGSL that does not match the dump. `BufferAttribute::new_integer`
  makes the upload `uint32x4`, which is what `WebGPUAttributeUtils` reads off
  the JS `Uint32Array`.
* **`normalScale.y = -1` is not in the asset.** It comes from `GLTFLoader`'s
  `useDerivativeTangents` material clone (mrdoob/three.js#11438), because
  Michelle has no `tangent` attribute and the derivative TBN frame has the
  opposite handedness. Without it the lighting on her skin inverts.
* **`DoubleSide` reaches the WGSL.** `negateOnBackSide` multiplies the normal
  by `faceDirection`; the first attempt did not emit it because
  `TBNViewMatrix` was cached in a process-wide `Lazy` and the material's normal
  node was built before the side was in scope. Both are fixed;
  `docs/nodes.md` §8 has the cache-key entry.
* **t = 0 is not the bind pose.** `SambaDance`'s first keyframe is at 0.0333 s
  and `LinearInterpolant` clamps below it, so `mixer.update( 0 )` is that
  keyframe — which is what the reference screenshot shows.

## WGSL

`cargo run --example dump_wgsl` gained two sections, diffed against the
scout's dump:

* `skinning_background` vs `m01_fragment_Background.material-r186.wgsl` —
  identical statement for statement, including `mix( vec3( 0.1328…, 0.4969…,
  1.0 ), vec3( 0.0578…, 0.1328…, 1.0 ), ( fragCoord.xy / render.nodeUniform0
  ).y )`. Only the `renderStruct` member order differs (§8).
* `skinning_body` vs `m03_vertex_Ch03_Body` / `m04_fragment_Ch03_Body` — the
  vertex stage matches statement for statement; the fragment stage matches
  through the physical specular block, the TBN frame and the normal-map scale.

Two of Three's `OperatorNode.generate()` matrix arms are reproduced
deliberately, because they are visible in the dump: `isMatrix( typeA ) &&
typeB === 'float'` emits `( b op a )` (so the position term reads `(
skinWeight.x * B ) * nodeVar0`), and `typeA === 'float' && isMatrix( typeB )`
emits `a op b` with **no** enclosing parens (so the normal term reads
`skinWeight.x * B + skinWeight.y * B`). Both are commented in
`src/nodes/builder.rs`.

New divergences are in `docs/nodes.md` §8: the skin matrix CSEd into one var,
the missing bone-texture branch, and the `TBNViewMatrix` cache key.

## What was ruled out

* **A bone `DataTexture`.** Not reached by any skeleton on the ladder; see
  above.
* **Mirrored repeat wrapping.** glTF `WEBGL_WRAPPINGS` has it; the port's
  `Wrapping` does not, and silently falling back to `Repeat` on an asset that
  asked for mirroring would be a wrong-pixel bug. Michelle's sampler declares
  no wrap at all (glTF default `REPEAT`).
* **Loosening anything.** Three's `test/e2e/image.js` at 0.1%, unmodified.

## What was left out

* **Non-skinned glTF primitives get no payload.** The loader builds and
  attaches a material only for the skinned meshes, because those are the only
  primitives the renderer draws today; a plain glTF `Mesh` is still a
  `GltfPrimitive` record with a geometry. Michelle's file has exactly one
  primitive, and it is skinned.
* **Tangents.** No `computeTangents` and no `tangent` attribute path — the
  derivative frame is the only one, which is what this asset uses.
* **The rest of `loadMaterial`.** `alphaMode` MASK/BLEND, `emissiveTexture`,
  `occlusionTexture`, `KHR_texture_transform`, `texCoord > 0` and the other
  `KHR_materials_*` extensions are parsed into `GltfMaterial` but not wired,
  since `Ch03_Body` uses none of them.
* **`Timer`.** The example's `animate()` feeds the mixer a literal 0, which is
  the page's first delta under the harness; the viewer integrates real time so
  she dances there.

## Residue

6 of 100000 pixels, against a limit of 100. The comparator's diff image
(`target/e2e/webgpu_skinning/diff.jpg`) shows no structure — the body, the
gradient and the shadowed edges all land inside the threshold — so what is
left is the usual JPEG round-trip on the reference screenshot at the
silhouette. Nothing here is worth chasing; the same residue is on every rung
that compares against a JPEG.
