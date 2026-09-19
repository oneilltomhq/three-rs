# `webgpu_loader_gltf_anisotropy`

Status: **green.** 27 of 100000 pixels against three.js r186's own
`test/e2e/image.js`, threshold 0.1%. Intel Iris Xe, Mesa 25.3.6, wgpu 30.0.1
on Vulkan. Steady frame 3.2 ms, 5 draw calls, 12188 triangles.

The Anisotropy Barn Lamp (`AnisotropyBarnLamp.glb`) under an UltraHDR
equirectangular map that is both `scene.environment` and, blurred to 0.5, the
background. Three glTF materials, five KHR extensions, no lights.

## Reconciling with the plans

There is no scout `PLAN.md` for this rung. It was ported from
`~/src/vendor/three.js/examples/webgpu_loader_gltf_anisotropy.html` directly,
with three's WGSL dumped by `tools/dump-webgpu.mjs` into `target/dumps/aniso/`
(uncommitted, per the rules) and diffed against `examples/dump_wgsl.rs`'s new
`loader_gltf_anisotropy_metal`, `_glass` and `_filament` sections.

It builds directly on what landed the same day: `webgpu_loader_gltf`'s
`scene.environment` fallback (§23.1) and `webgpu_pmrem_equirectangular`'s
UltraHDR loader and PMREM (§21). Neither needed changing.

## What was added

| area | what |
| --- | --- |
| `src/loaders/gltf_loader.rs` | `KHR_materials_anisotropy`, `_clearcoat`, `_emissive_strength`, `_transmission`, `_volume` |
| `src/materials/mod.rs` | `Material::{anisotropy, anisotropy_rotation, anisotropy_map, clearcoat_normal_map, clearcoat_normal_scale, transmission, thickness, attenuation_distance, attenuation_color}` |
| `src/materials/physical.rs` | the anisotropy and clearcoat branches of `PhysicalLightingModel`; `Physical::total_diffuse()` |
| `src/materials/transmission.rs` | `getVolumeTransmissionRay`, `applyIorToRoughness`, `volumeAttenuation`, `textureBicubicLevel`, `getTransmissionSample`, `getIBLVolumeRefraction`, `OpaqueFrame` |
| `src/nodes/tsl.rs` | the attribute tangent frame (`Tangent.js`/`Bitangent.js`), `bent_normal_view()`, `clearcoat_normal_view()`, the four transmission material uniforms, `camera_position()`, `refract()`, `ceil()` |
| `src/nodes/builder.rs` | `refract`'s eta operand is built as a `float` |
| `src/renderer/mod.rs` | the two-pass split, `opaque_frame_texture()`, `copy_framebuffer_to_opaque_frame()`, `PassTarget::color_texture`, `Draw` hoisted to module level |
| `src/renderer/render_list.rs` | `material.transmission > 0` sorts with the transparent list |
| `src/objects/scene.rs` | `Scene::background_blurriness` |
| `examples/` | `webgpu_loader_gltf_anisotropy.rs`; `dump_wgsl` sections `loader_gltf_anisotropy_metal`, `_glass`, `_filament` |

`docs/nodes.md` §26 is the long form.

## What the pixels found

**The glass was opaque.** Everything else in the frame was already at the
grader's threshold with the glass left as an ordinary physical material — about
2400 px, all of them the bulb. Transmission was not optional for this rung, so
`src/materials/transmission.rs` and the renderer's two-pass split are the bulk
of the work. §26.3.

**The split landed ahead of the filament.** With transmission working the count
came down to 1411, and the diff image was still exactly the bulb: an even milky
white with the filament's glow missing. `RenderList.push()` tests
`material.transparent === true || material.transmission > 0`, and the glTF sets
no `alphaMode` on the glass, so the port had it sorted among the opaques — the
copy of the opaque frame was taken before `lamp filament` had been drawn into
it. One `||` later: 1411 px → 27 px. §26.4.

**`refract( I, N, eta )` takes a scalar eta.** The port widened every `MathNode`
operand to the input type, so `getVolumeTransmissionRay` came out as
`refract( vec3, vec3, vec3<f32>( 1.0 / ior ) )` and naga rejected the module.
`MathNode.REFRACT` builds its third operand as `float`; so does the port now.
This one was a compile error rather than wrong pixels, which on this stack is
the lucky case.

**The background is a cubeUV read, not the sharp cube.** `backgroundBlurriness
= 0.5` routes the skybox through the PMREM, which the dumped `m06` background
fragment confirms. `webgpu_loader_gltf` left `backgroundBlurriness` out as a
GUI knob at its default; this page is the one that needs it.

## What was ruled out

* **Writing the anisotropic GGX anyway.** `D_GGX_Anisotropic` and
  `V_GGX_SmithCorrelated_Anisotropic` sit behind `direct()`, and this scene has
  no lights, so nothing on the ladder would grade them. An unverified lobe in
  the lighting model is worse than a missing one. §26.5.
* **A separate transmission render target.** Three reuses the canvas's own
  resolved colour texture and mips it in place; a second target would have
  meant a second resolve and a different frame from the dump's.
* **Copying the MSAA attachment.** A multisampled texture cannot be bound as
  `texture_2d`. `PassTarget::color_texture` carries the resolve target instead.
  §26.3.
* **Sorting the draws into two `Draw` lists.** Bind groups reference the texture
  object, not its contents, so one list built up front and sliced at the split
  is correct and keeps the uniform upload in one place.

## What was left out

* The anisotropic GGX (above), and with it `AlphaT`'s effect on direct light.
* Double-pass transmission (`side === DoubleSide`): the lamp's glass is
  single-sided.
* Dispersion, iridescence, sheen and retroreflection — other flags of the same
  lighting model, none of them in this glTF.
* `OrbitControls` beyond the one `update()` the page does before the graded
  frame; `minDistance` / `maxDistance` are interaction limits.
* The `UltraHDRLoader`'s progressive path; the loader reads the whole file.
