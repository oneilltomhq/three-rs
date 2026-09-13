# rung 8 — `webgpu_lights_physical`, done

**Status: passing.** `webgpu_lights_physical` differs in 4 of 100000 pixels
(0.004%, limit 0.1%). The six earlier rungs are unchanged: depth_texture 0,
instance_mesh 60, lights_phong 31, materials_basic 0, postprocessing_masking 18,
rtt 1. `cargo build --workspace --all-targets` is clean and `cargo test
--workspace` is green.

The generated WGSL is structurally identical to the scout's dumps — same
bindings, same statement order, same expressions — modulo the set listed in
`docs/nodes.md` §8, which rung 8 extended.

## What rung 8 added

| area | what |
|---|---|
| lighting model | `src/materials/physical.rs` — `PhysicalLightingModel`: `BRDF_GGX` (`V_GGX_SmithCorrelated`, `D_GGX`), `DFGApprox` through a 64×64 LUT, `computeMultiscattering`, the dielectric/metallic single- and multi-scattering split, `getRoughness`/`getGeometryRoughness` |
| materials | `MaterialKind::Standard` on the one `NodeMaterial` struct, `MeshStandardNodeMaterial` / `MeshPhysicalNodeMaterial` aliases, the PBR fields (`metalness`, `roughness`, `metalness_map`, `roughness_map`, `emissive`, `emissive_intensity`, `bump_map`, plus the KHR_materials_specular / `ior` fields rung 10 needs) |
| lights | `HemisphereLight` (`LightPayload::Hemisphere`, ground-colour and world-position uniforms), `PointLightShadow`, `src/lights/point_shadow.rs` |
| shadows | `CubeDepthTexture`, `Node::If`, `SampleMode::Compare` → `textureSampleCompare` through a `sampler_comparison`, the eight `ShadowNode` render-group uniforms, `interleavedGradientNoise` + `vogelDiskSample`, and the six-face shadow pass in `Renderer::render_shadows()` |
| TSL | `positionWorld`, `shadowPositionWorld`, `fract`, `int()`, `greaterThanEqual`, `and`, `bumpMap()` (`BumpMapNode` + `@builtin( front_facing )`), `transformed_uv()` for `Texture.offset/repeat/center/rotation` |
| output | Reinhard tone mapping and `toneMappingExposure` in the output pass |
| textures | `RepeatWrapping` + anisotropy as the page sets them |
| example | `examples/webgpu_lights_physical.rs`, the eighth `tests/e2e/main.rs` entry, two more `dump_wgsl` blocks |

## The bugs only the grader (or the dump diff) found

1. **`normalWorld` and the tangent frame were accessor singletons.**
   `normal_view()` is memoised on `(sub-build layer, material normal node)` so a
   bump-mapped material and the `NORMAL` layer get their own vars.
   `normal_world()` and `tangent_frame()` were plain `Lazy` singletons, so they
   baked in whichever `normalView` the *first* material built in the process
   resolved to. Every later material then emitted a second
   `normalView = normalViewGeometry;` — after its bump-mapped assignment —
   throwing the perturbed normal away for the rest of the shader.
   `webgpu_lights_physical` is the first ladder example with a `bumpMap` and a
   `HemisphereLight` (`normalWorld`'s only reader) on the same material, so
   nothing caught it before. Both now share `normal_key()`.

2. **`shadowCoord.xyz` was emitted twice.** `NodeBuilder::needs_var()` promotes
   `Op` / `Math` / `Join` / layout-`Call` on usage count, but not `Swizzle` or
   `Neg`. Three's `toConst()` hides this; here the shadow matrix multiply
   appeared once inside `abs(…)` and again inside `normalize(…)`. Same for
   `viewZ.negate()`, read three times by `viewZToPerspectiveDepth`. Both are
   wrapped in explicit `to_var()`s.

3. **Two wgpu validation errors, in order.** The sampler binding has to be
   declared `sampler_comparison` in the WGSL *and* `SamplerBindingType::
   Comparison` in the layout, and the cube shadow map's
   `TextureSampleType` has to be `Depth`, not `Float { filterable }` — the
   `TextureKind::DepthCube` match arms were added one at a time as each error
   surfaced. Both are hard failures, not silent wrong output.

### Ruled out along the way

* The 331-pixel diff before shadows landed was *only* the two contact shadows
  plus a small spot at the bulb; the PBR shading, the tone mapping, the bump
  map, the textures' repeat/anisotropy and the hemisphere light were all already
  right, checked against the dumps statement by statement.
* Frustum culling per cube face is not a divergence to add: re-running
  `project_scene()` with the face camera reproduces three's own
  `shadow._frustum.setFromProjectionMatrix()` behaviour for free.
* The shadow pass's colour attachment (`rgba8unorm`, 6 layers) is never sampled.
  Only its depth is. That is why the missing `NoBlending` on the override
  material — and so the extra `DiffuseColor.w = 1.0` line the port emits — is
  unobservable.

## Notes for rung 10 (skinning)

* The PBR material API is general: `MeshStandardNodeMaterial` and
  `MeshPhysicalNodeMaterial` are aliases of the one `NodeMaterial` struct with
  `MaterialKind::Standard`. The KHR_materials_specular fields and `ior` are
  present and default to three's values even though nothing in rung 8 reads
  them, so a glTF material can be loaded without touching the struct.
* `setup_standard()` takes the light list by index and the per-light shadow
  map slice; a skinned mesh changes only `setupPosition`, so it plugs in beside
  the instancing branch in `setup_inner()` and nothing in the lighting flow
  needs to move.
* `SetupContext` is `Copy` and now carries `receive_shadow` and `light_kinds`;
  anything skinning needs (a bone-texture handle, say) should go on
  `Renderable` instead if it is not `Copy`, the way `fog` and the shadow maps do.
* `Physical::start()` builds the whole model from thread-local singletons, so a
  second material in the same process reuses the same nodes — see the
  `normal_key()` bug above before adding another accessor that reads
  `normalView`.
