# webgpu_postprocessing_bloom_emissive

2026-09-25: regraded against three.js 5f610f5 (the cube PMREM of 2f80402, #146): 0 of 100000 pixels (was 28).

Branch `rung-bloom-emissive`, cut from `9df096f` (the tip of `rung-gltf-bloom`,
i.e. the `webgpu_postprocessing_bloom` rung).

**Green at 28 pixels of 100000** (0.028%, limit 0.1%), steady frame 5.1 ms, 15
draw calls, 17449 triangles. The 32-row ladder is otherwise unchanged; this is
row 33.

    depth_texture 0 / instance_mesh 60 / materials_basic 0 / rtt 1 /
    lights_phong 31 / morphtargets 0 / shadowmap 7 / lights_physical 4 /
    postprocessing_masking 18 / tsl_galaxy 40 / skinning 6 / mesh_batch 0 /
    compute_points 4 / radial_blur 7 / materials 44 / ssaa 0 /
    pmrem_cubemap 0 / bloom_selective 1 / lines_fat 0 / pmrem_test 27 /
    postprocessing_difference 13 / postprocessing_direct 21 / furnace_test 0 /
    postprocessing_anamorphic 2 / pmrem_scene 0 / bloom 0 /
    **bloom_emissive 28**

Graded twice in Three itself first — `Diff 0.0%` both times, and the example is
not on `test/e2e/puppeteer.js`'s `exceptionList` — so the reference is honest.

## A correction to the brief

The brief describes the page as `HDRLoader` →
`pmremGenerator.fromEquirectangular` → `scene.environment`. The r186 page calls
**no `PMREMGenerator` at all**:

```js
texture.mapping = THREE.EquirectangularReflectionMapping;
scene.background = texture;
scene.environment = texture;
```

Both conversions are the renderer's. `CubeMapNode.updateBefore()` turns the
1024×512 map into a 512² **cube** for the background, and `EnvironmentNode`
PMREM-filters the same map for the environment. Three's dump has both: `m01` /
`m02` are the cube conversion, `m05`–`m08` the PMREM equirect and GGX passes.

## What this rung adds

| area | what |
|---|---|
| `src/renderer/cube_render_target.rs` | `CubeRenderTarget.fromEquirectangularTexture` — the `BoxGeometry( 5, 5, 5 )` seen from inside, six `CubeCamera` faces at `fov = -90`, into a `CubeTexture` |
| `src/nodes/tsl.rs` | `positionWorldDirection`; `materialAOMapIntensity`; the `AmbientOcclusion` property |
| `src/materials/mod.rs` | `emissiveMap`, `aoMap`, `aoMapIntensity` |
| `src/materials/node_material.rs` | `setupAmbientOcclusion()`; `MaterialNode.EMISSIVE`'s `emissiveMap` factor; the `AONode` entry of `setupLightsNode()` |
| `src/materials/physical.rs` | `ambientOcclusion()` no longer re-declares the var when an `AONode` already read it |
| `src/nodes/mrt.rs` | `mrtNode.setBlendMode( name, blendMode )` / `getBlendMode()`, merged with the outputs |
| `src/renderer/programs.rs` | `ExtraColorTarget` — a format and a blend state per colour attachment, in the pipeline key |
| `src/renderer/mod.rs` | the extra attachments take their own texture's format; `RenderState.extra_color_targets`; `copy_to_cube_layer()` |
| `src/textures/texture.rs` | `Texture::set_texture_type()` — `emissiveTexture.type = UnsignedByteType` |
| `src/textures/cube_texture.rs` | `CubeTexture::render_target()` — six faces with no pixels, to be rendered into |
| `src/loaders/gltf_loader.rs` | `emissiveTexture` (sRGB) → `emissiveMap`; `occlusionTexture` → `aoMap` |

## The three things the dump settled

Three's WGSL and pass structure were dumped with `tools/dump-webgpu.mjs` into
an uncommitted `dump-bloom_emissive/`; `examples/dump_wgsl.rs` grew three
sections diffed against it.

**1. The emissive attachment is a second `@location`, not a second pass.**
`m10`'s tail is

```wgsl
Output = max( vec4<f32>( ( outgoingLight + EmissiveColor ), DiffuseColor.w ), vec4<f32>( 0.0 ) );
output.m0 = Output;
nodeVar157 = vec4<f32>( EmissiveColor, Output.w );
output.m1 = nodeVar157;
```

which the port reproduces statement for statement (it inlines the temp). The
skybox's own material — `m04` — writes the same two attachments, and its
`EmissiveColor` is *declared and never assigned*, so WGSL's zero-initialised
`vec3` is what attachment 1 gets. That is why the sky does not bloom, and it
falls out of the flow rather than needing a rule.

**2. `aoMap` reads `uv`, not `uv1`.** The dumped vertex shader has a single
`nodeVarying6 : vec2<f32>` uv varying and `DamagedHelmet.gltf`'s only texture
coordinate is `TEXCOORD_0`, so three's `uv1` and the port's `uv` are the same
attribute here. No `TEXCOORD_n` plumbing was needed or added.

**3. The AO var's `= 1.0` is emitted once, by the `AONode`.** Three builds
`ambientOcclusion` as `float( 1 ).toVar()`, whose initialiser lands at the
var's *first read* — which with an `aoMap` is `AONode`'s `mulAssign`, not
`PhysicalLightingModel.ambientOcclusion()`. The port's first attempt emitted it
twice; `PhysicalLightingModel::ambient_occlusion( has_ao_node, … )` now skips
its own declaration, and the dump agrees line for line:

```wgsl
AmbientOcclusion = ( ( ( nodeVar1.x - 1.0 ) * object.nodeUniform6 ) + 1.0 );
…
ambientOcclusion = 1.0;
ambientOcclusion = ( ambientOcclusion * AmbientOcclusion );
```

## The MRT blend mode

`mrtNode.setBlendMode( 'emissive', new BlendMode( NormalBlending ) )` is
pixel-neutral on this page — `_getBlending()` bypasses `material.transparent`
for an MRT attachment, and alpha is 1 everywhere — but it is visible in the
descriptor, and `dump.json` shows it:

```
renderPipeline_Material_MR_20 [
  { "format": "rgba16float", "writeMask": 15 },
  { "format": "rgba8unorm",
    "blend": { "color": { "srcFactor": "src-alpha", "dstFactor": "one-minus-src-alpha", "operation": "add" },
               "alpha": { "srcFactor": "one", "dstFactor": "one-minus-src-alpha", "operation": "add" } },
    "writeMask": 15 } ]
```

Both halves of that row are new: before this rung every colour attachment of a
pass took the *target's* format and the *material's* blend state.
`RenderState` now carries an `ExtraColorTarget` per extra attachment, so the
two are independent and both are in the pipeline cache key.

## Not ported

* **`Scene::environment`.** The port has no such field; the example puts the
  `PmremHandle` on the loaded materials itself, which is the same node graph
  `EnvironmentNode` builds. `rung-room-environment` owns the field.
* **`OrbitControls`** reduces to the initial pose, as on every sibling:
  `camera.look_at( 0, 0, -0.2 )`.
* **Cube mipmaps.** `dump.json` gives the 512²×6 cube `mipLevelCount: 1` and
  the background samples it at `backgroundBlurriness = 0`, so there is no chain
  to build. `rung-envmaps` owns cube mip levels.
* **`KHR_materials_emissive_strength`**, `occlusionTexture.strength` and
  `TEXCOORD_n` for `n > 0` — none of them is in this asset.
