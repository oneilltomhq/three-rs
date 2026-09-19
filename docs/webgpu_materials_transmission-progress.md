# `webgpu_materials_transmission`

Status: **not green — 198 of 100000 pixels**, against three.js r186's own
`test/e2e/image.js` and its 0.1% (100 px) threshold. 0.198%. Intel Iris Xe,
Mesa 25.3.6, wgpu 30.0.1 on Vulkan. Steady frame and build counts pass: the
example is in `steady_frame_builds_nothing`, and its pixel test is written and
`#[ignore]`d in `tests/e2e/main.rs` rather than left failing.

A `SphereGeometry( 20, 64, 32 )` with `transmission: 1`, in front of the
UltraHDR `royal_esplanade_2k` used as both background and env map. It is the
direct test of the transmission path `webgpu_loader_gltf_anisotropy` brought in
the same day (`docs/nodes.md` §26): there the glass was 2% of the frame, here
the transmissive surface is a fifth of it and everything behind it is
background.

## Reconciling with the plans

There is no scout `PLAN.md` for this rung. It was ported from
`~/src/vendor/three.js/examples/webgpu_materials_transmission.html` directly.
Two notes on the page, because the rung brief described a different one:

* **There is no light and no inner `MeshBasicMaterial` mesh.** The scene is one
  mesh. `PhysicalLightingModel.direct()` is therefore *not* exercised by this
  page either — the rung does not close that gap.
* **`thickness` is never applied.** `params.thickness` is 0.01, but only the
  GUI `onChange` writes it, and the graded frame is the first one. The material
  keeps `MeshPhysicalMaterial`'s default 0, so the volume ray has zero length,
  `volumeAttenuation` is the identity, and the transmission is a straight
  re-read of the opaque frame at the fragment's own screen position, blurred by
  the roughness mip (roughness 0 → level 0). The same goes for `opacity`,
  `metalness`, `roughness`, `ior` and `exposure`, which the constructor does
  pass with the same values the GUI defaults to.

## What was added

| area | what |
| --- | --- |
| `src/materials/mod.rs` | `Material::alpha_map` |
| `src/materials/node_material.rs` | `material_opacity_with_map()` — `MaterialNode.OPACITY`'s `materialOpacity.mul( texture( alphaMap ) )`, narrowed to `float` |
| `examples/` | `webgpu_materials_transmission.rs` |
| `tests/e2e/main.rs` | the (ignored) pixel test and the `steady_frame_builds_nothing` rung |

Everything else the page needs was already in the tree: the transmission pass
and `materials::transmission` (§26), the UltraHDR loader and the PMREM (§21),
`cube_render_target::from_equirectangular_texture` for the sharp equirect
background (§23), and the texture matrix for `repeat` (§25).

## What the pixels found

**The alpha map is read through its red channel.** `MaterialNode.OPACITY`
multiplies `materialOpacity` (a `float`) by the texture (a `vec4`), and the
scope's node type is `float`, so three converts the product back by taking its
first component. `WebGLRenderer` reads `.g` for an `alphaMap`; the WebGPU node
path reads `.r`. The page's canvas is white/transparent, so either would grade
the same here — but a coloured alpha map would not, and the port follows the
node path. §29.

**The 2×2 canvas is a `Texture` in canvas row order.** `generateTexture()`
leaves row 0 at `rgba( 0, 0, 0, 0 )` and fills row 1 white. Written to a
`Texture` top row first, the uploader's `flipY` (true, as on a `CanvasTexture`)
puts it the right way up, and `repeat.set( 1, 3.5 )` with `RepeatWrapping` and
`NearestFilter` gives the seven bands. Nothing here is derived from the
reference image.

**The 198 that are left are all one crescent.** They sit on the sphere's left
limb, roughly 7 to 10 o'clock, where the grazing-angle reflection and the
refracted read both magnify the bright rainbow wall behind the sphere. The
background outside the silhouette, the bands, the right limb and the whole
right half of the frame are inside the threshold. So it is not the alpha map,
not the band phase, not the tone mapping and not the transmission pass being
absent — those all fail loudly and everywhere. It is a small sampling
difference amplified by the one high-contrast region in the image.

## What was ruled out

* **The transmission pass ordering.** The background is drawn into the opaque
  frame before the copy — a missed copy leaves a milky sphere, which is the
  failure §26.4 describes and is not what the diff shows.
* **`thickness` / `attenuation`.** Both are at their defaults on the graded
  frame (above), so the refraction offset is zero by construction.
* **The exposure and tone mapping.** ACES Filmic at exposure 1; a mismatch
  there moves the whole frame, and the background matches.

## What was left out

* **A WGSL diff against three's own dump.** `tools/dump-webgpu.mjs` was not run
  for this page and `examples/dump_wgsl.rs` gained no section — that is the
  next step, and the most likely place to find the crescent: the back-side half
  of the `DoubleSide` split samples a *different* texture upstream
  (`viewportMipTexture()` rather than `viewportOpaqueMipTexture()`, chosen by
  `material.side === BackSide` inside `getTransmissionSample`), and the port
  binds the opaque copy to both halves. The two have the same content here, but
  not necessarily the same mip chain.
* **A README "Examples graded green" row and a gallery entry**, since the
  example is not green.
* **Dispersion**, still — `KHR_materials_dispersion` is dead code for this page
  as it was for the barn lamp (§26).
