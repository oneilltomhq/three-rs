# `webgpu_loader_gltf_transmission`

Status: **not green, and not gradeable yet.** The page's asset,
`IridescentDishWithOlives.glb`, is Draco-compressed and lists
`KHR_draco_mesh_compression` in `extensionsRequired`. Nothing in the port
decodes Draco, so the example cannot put a triangle on screen. There is no
diff count to report and the example is deliberately **not** in
`tests/e2e/main.rs`. `docs/nodes.md` §28 is the long form.

What did land is the guard that turns this from a silent wrong picture into an
error, and the ported example, which is complete except for that one call.

## Reconciling with the plans

There is no scout `PLAN.md` for this rung. It was ported from
`~/src/vendor/three.js/examples/webgpu_loader_gltf_transmission.html`
directly. Against the current tree, nothing in the page needed API that does
not exist: `PmremEnvironment`, `Scene::background_blurriness`,
`Background::Pmrem`, `AnimationMixer::clip_action` and the four KHR material
extensions the asset uses (`_ior`, `_specular`, `_transmission`, `_volume`)
all landed with §21, §23, §25 and §26. The rung is one loader feature short,
not one renderer feature short.

## What was added

| area | what |
| --- | --- |
| `src/error.rs` | `GltfError::UnsupportedRequiredExtension` |
| `src/loaders/gltf_loader.rs` | `SUPPORTED_EXTENSIONS` and `check_required_extensions()`, called from `GLTFLoader::parse` |
| `tests/gltf_loader.rs` | `draco_required_is_an_error` |
| `examples/` | `webgpu_loader_gltf_transmission.rs` (does not run; see above) |
| `docs/nodes.md` | §28 |

Nothing under `src/renderer/` or `src/nodes/` was touched, so this branch has
no merge-conflict surface with the other rungs of the wave.

## What the failure looked like before the guard

The loader read the file without complaint. A Draco primitive's accessors
carry no `bufferView`, and a `bufferView`-less accessor is a legal zero-filled
accessor, so four meshes came out with the right vertex and index counts and
every value zero:

```
glassDish   color/normal/position/uv 1090   index 6144
olives      color/normal/position/uv 10992  index 51840
glassCover  normal/position/uv 1858         index 10752
goldLeaf    color/normal/position/uv 924    index 4608
first position: [ 0, 0, 0, 0, 0, 0, 0, 0, 0 ]
```

A scene that renders the background, with the model collapsed to a point.
three.js never sees this because its DRACOLoader is a plugin the page
installs and `GLTFDracoMeshCompressionExtension` throws without one; three's
own `extensionsRequired` branch is only a `console.warn`. On this stack a
warning is an invisible wrong picture, so the port errors.

## What the pixels found

Nothing — no frame with geometry in it was ever rendered. Grading the example
as it stands would have compared Three's screenshot against an empty scene and
measured only the background, which is worth no number.

## What is missing, exactly

1. **A Draco decoder** (the whole blocker): rANS symbol decoding, the
   sequential and edgebreaker connectivity decoders, the prediction schemes
   (parallelogram, texcoord, octahedral normals) and dequantization. A rung of
   its own; §28.3 says why it is tractable — three's `DRACOLoader` under node
   is an exact oracle for the decoded attributes, and none of it touches the
   renderer or the node system.
2. **`alphaMode: MASK`** for `goldLeaf` (`alphaCutoff` 0.5). The loader
   deliberately leaves `MASK` unwired; three builds `materialAlphaTest` as a
   uniform where this crate has only a literal `alpha_test_node`. §28.4.
3. Grading then exercises, for the first time on this ladder: two stacked
   transmissive materials over one opaque copy, a `thicknessTexture`, a
   `specularColorTexture`, `COLOR_0` on a glTF, and an `AnimationMixer` on a
   non-skinned node.

## What was ruled out

* **Decompressing the asset ahead of time.** Any prebaked copy of
  `IridescentDishWithOlives.glb` is asset data in the tree and a permanent
  divergence from what the page loads; the rule is that the example loads
  Three's own file.
* **Grading the example with an empty scene** to claim a number. See above.
* **Iridescence.** The asset's name says iridescent; its `extensionsUsed` does
  not contain `KHR_materials_iridescence`. The look is the dish's
  `specularColorTexture`. Porting an iridescence lobe here would have been
  unverifiable by this page — §28.1.
* **Double-pass transmission.** Neither transmissive material is
  `doubleSided`, so this page does not close the §26.3 gap either.
