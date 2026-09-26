# `webgpu_shadowmap_opacity`

Status: **green.** 9 of 100000 pixels against three.js 5f610f5's own
`test/e2e/image.js`, threshold 0.1%. Intel Iris Xe, Mesa 25.3.6, wgpu on
Vulkan. Steady frame 6.0 ms (release, `viewer --headless --frames 40`),
6 draw calls, 408644 triangles. Frames 2 and 3 build nothing.

Two transmissive dragons from `DragonAttenuation.glb`, one amber and one red,
stand on the cloth backdrop under a blue `DirectionalLight` with a 2048² PCF
shadow. Each dragon casts a shadow tinted by its own `attenuationColor`:

```js
renderer.shadowMap.transmitted = true;
dragon.material.castShadowNode = customShadow( dragon.material.attenuationColor ); // mix( 1, color, 1 )
renderer.toneMapping = THREE.AgXToneMapping; // exposure 1.5
```

## Reconciling with the plans

There is no scout `PLAN.md` for this rung. It was ported from
`~/src/vendor/three.js/examples/webgpu_shadowmap_opacity.html` directly.
Three's WGSL was dumped by `tools/dump-webgpu.mjs` into
`target/dumps/webgpu_shadowmap_opacity/` (uncommitted).

## What was added

| area | what |
| --- | --- |
| `src/renderer` | `Renderer::shadow_map_transmitted`. A spot or directional light's shadow map is wrapped in `ShadowMap::Transmitted` with the shadow pass's colour target |
| `src/materials/phong.rs` | `ShadowMap::Transmitted` and `shadow_factor_transmitted`: `mix( 1, mix( shadowColor, 1, shadow ), intensity * shadowColor.a )` |
| `src/materials` | `cast_shadow_node`, which feeds `shadow_material_for`'s colour as `_getShadowNodes()` does; `thickness_map` (`thickness * map.g`); `ToneMapping::AgX` |
| `src/nodes/tsl.rs` | `agx_tone_mapping` |
| `src/loaders/gltf_loader.rs` | `KHR_materials_volume.thicknessTexture` |
| `examples/` | `webgpu_shadowmap_opacity.rs`; a `dump_wgsl` section (three's `m01`, `m05`, `m09`) |
| registrations | the `rung!` and `#[test]` entries, the README row and gallery thumbnail, the web trio and manifest, the viewer row |

`docs/nodes.md` §36.4–36.6 is the long form.

## What the pixels found

The first graded frame was 9 pixels, with every piece above in place. The 9
are isolated pixels in `diff.png`, all on the two transmissive dragons. None
is on the backdrop, where the tinted shadows fall.

`dump_wgsl` matches three on each module it covers:

* the shadow override material, line for line;
* the backdrop's transmitted PCF shadow: the colour sampled before the frustum
  branch, five vogel taps, the same double `mix`;
* the AgX output pass: equal apart from how the `mat3x3` constant is spelled.

The divergences are listed in §36.6:

* the tinted factor is `.xyz`'d where three widens the light colour to `vec4`;
* point lights ignore `transmitted`;
* the shadow map is re-rendered every frame where the page sets
  `autoUpdate = false`.
