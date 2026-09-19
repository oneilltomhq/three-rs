# `webgpu_loader_gltf_iridescence`

Status: **not green — 1534 of 100000 pixels (1.5%), threshold 0.1%.** Intel
Iris Xe, Mesa 25.3.6, wgpu 30.0.1 on Vulkan. The example renders, the whole
iridescence half of `PhysicalLightingModel` is ported and the ladder is
untouched; the e2e test exists but is `#[ignore]`d and the example is not on
the `rung!()` list. What is left is written out under
"What is missing" below.

The Iridescence Lamp (`IridescenceLamp.glb`) under `venice_sunset_1k.hdr`,
which is both `scene.environment` (PMREM) and, unblurred, `scene.background`
(a cube converted from the equirect). Three glTF materials, three KHR
extensions, no lights.

## Reconciling with the plans

There is no scout `PLAN.md` for this rung. It was ported from
`~/src/vendor/three.js/examples/webgpu_loader_gltf_iridescence.html` directly,
with three's WGSL dumped by `tools/dump-webgpu.mjs` into `target/dumps/irid/`
(uncommitted, per the rules). `m12_fragment_fragment_IridescenceLampIridescence.wgsl`
is the module every statement below was read off.

It builds on what landed the same day: `webgpu_loader_gltf`'s
`scene.environment` fallback and equirect-to-cube background (§23), the sheen
rung's glTF extension plumbing (§25) and the anisotropy rung's transmission
pass and `materials::transmission` (§26). None of them needed changing.

## What was added

| area | what |
| --- | --- |
| `src/materials/physical.rs` | `evalIridescence`, `evalSensitivity`, `Fresnel0ToIor` / `IorToFresnel0`, `Schlick_to_F0`; the iridescence branch of `Physical::start()`; the iridescent F0 in `compute_multiscattering()`; the `USE_IRIDESCENCE` blend in `brdf_ggx()` |
| `src/materials/mod.rs` | `Material::{iridescence, iridescence_ior, iridescence_thickness_range, iridescence_thickness_map}` |
| `src/loaders/gltf_loader.rs` | `KHR_materials_iridescence` (`GltfIridescence`, `iridescence_texture`, `iridescence_thickness_texture`) |
| `src/nodes/tsl.rs` | `material_iridescence()`, `material_iridescence_ior()`, `material_iridescence_thickness_{max,min}()`; the `Iridescence` / `IridescenceIOR` / `IridescenceThickness` properties; `loop_range`'s `inclusive` argument |
| `src/nodes/node.rs`, `src/nodes/builder.rs` | `Node::Loop::inclusive` — `for ( … ; m <= 2 ; … )` |
| `src/renderer/programs.rs`, `src/renderer/mod.rs` | the four new material uniforms |
| `examples/` | `webgpu_loader_gltf_iridescence.rs` |

`docs/nodes.md` §30 is the long form.

## What the pixels found

The diff image is not on the lamp. 1534 differing pixels, and almost all of
them sit on the high-frequency edges of the skyline in the **background** —
the tree line at the left, the buildings at the right — as a one-pixel-wide
speckle along every contrast edge. The lamp body, the iridescent shade and the
glass bulb are inside the threshold apart from a thin rim on the shade's inner
lip, which is where the background is mirrored.

That points at the equirect-to-cube conversion for `scene.background`, not at
iridescence: this is the first graded page to take a Radiance `.hdr`
(`rgba16float`, `flipY = true`, no mip chain, `LinearFilter`) through
`cube_render_target::from_equirectangular_texture`, where every earlier page
fed it an UltraHDR JPEG. A wrong `evalIridescence` would colour the shade, and
the shade is right.

## What is missing

1. **The background cube.** The suspects, in the order they are worth
   checking: the `flipY` the `.hdr` data texture carries versus the
   `positionWorldDirection` the conversion box samples along; the face size
   (`texture.image.height` is 512 here, half what every UltraHDR page uses);
   and the filtering of a 512² `rgba16float` face with no mips. Three's own
   `m04_fragment_fragment_Background.material.wgsl` is a plain
   `textureSampleLevel( …, render.nodeUniform5 )` cube read, so the shader is
   not where the difference is — the cube's contents are.
2. **A `dump_wgsl.rs` section.** The generated WGSL was read against
   `target/dumps/irid/m12_*.wgsl` by hand while writing
   `src/materials/physical.rs`, but the three
   `loader_gltf_iridescence_{lamp,iridescence,transmission}` sections that
   would keep it diffed are not written.
3. **The README "Examples graded green" row**, which cannot be written until
   the number is under the threshold.

## What was ruled out

* **`iridescenceTexture`.** The extension's `iridescenceFactor` map is parsed
  into `GltfMaterial::iridescence_texture` and then deliberately dropped:
  `MaterialNode.IRIDESCENCE` has no map branch in r186, so three itself never
  reads `material.iridescenceMap`. The asset carries no such texture anyway.
* **`BRDF_GGX`'s `USE_IRIDESCENCE` branch.** Written, never reached: the page
  has no lights, so `PhysicalLightingModel.direct()` is never called and
  `this.iridescenceFresnel` is not in three's dump for this page either.
* **Dispersion and retroreflection**, the two other `PhysicalLightingModel`
  flags: neither is on for any material here.
