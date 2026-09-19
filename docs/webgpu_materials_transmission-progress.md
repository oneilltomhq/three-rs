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
| `examples/dump_wgsl.rs` | the `materials_transmission` section, diffed against three's `m12` |

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

## The WGSL diff

`node tools/dump-webgpu.mjs webgpu_materials_transmission` (15 modules, 9 render
pipelines; the dump is uncommitted, per the rules) and a new
`examples/dump_wgsl.rs` section, `materials_transmission`. Three's sphere is
`m12` (front-side) and `m10` (back-side); **the two differ from each other in
exactly one line**, the normal flip, which answers §29.3's first question: both
halves of the `DoubleSide` split compile the same transmission flow.

The port's fragment matches `m12` statement for statement through the whole of
`getIBLVolumeRefraction`: the ray, the NDC projection, the `1 - y` viewport
flip, `log2( cameraViewport.z ) * applyIorToRoughness( Roughness, IOR )`, both
`textureDimensions` levels, all eight bicubic taps, the `ceil`/`floor` pair and
the final `mix( …, fract( lod ) )`. The remaining differences are the ones §8
already lists: temporary folding, the order of the zero-initialised
accumulators, `.xy` taken at the DFG sample rather than at its use, and
`normalView`'s flip — three resolves it per pass (`* vec3( -1.0 )` in `m10`,
nothing in `m12`) because the split sets `material.side`, where the port emits
the dynamic `( f32( isFront ) * 2 - 1 )` factor and draws the one program twice.

So the crescent is not the shader.

Sizes match too: the equirect is 2048×1024, three's cube from it is 1024² (id 6
in `dump.json`) and the port's is `source.size().1` = 1024; the PMREM atlas is
1536×2048 on both; the opaque-frame copy is 800×500 `rgba16float` with 10 mip
levels on both, and the port generates them with the same hand-written mipmap
shader three does.

**Measured against three's own 800×500 frame** (`actual_full.png` from the
dump), rather than the graded 400×250 JPEG:

| | |
| --- | --- |
| pixels off by more than 24/255 | 1243, all with 288 ≤ x ≤ 500 and 179 ≤ y ≤ 361 — inside the sphere |
| background, outside the silhouette | mean abs difference 0.63/255, nothing over 24 |
| best whole-image shift | (0, 0) — it is not a half-pixel offset |
| mean gradient inside the sphere | ours 3.67, three's 3.83 — **the port's transmitted image is ~4% softer** |
| mean signed difference inside the sphere | −1.3/255 — and slightly darker |

Softer and darker, in place, is a mip level: the transmission LOD is
`log2( 800 ) * applyIorToRoughness( Roughness, ior )` ≈ `9.64 * Roughness`, and
`Roughness` is never 0 — `setupVariants()` clamps it to 0.0525 and *adds the
geometric roughness* `max( abs( dpdx( normalViewGeometry ) ), abs( dpdy( … ) ) )`.
So the sphere samples level 0.5 at its centre and climbs steeply towards the
limb, where the normals turn fastest — which is exactly where the crescent is,
and exactly where the background behind it is the bright rainbow wall. Both
shaders compute that LOD identically, so the difference is in the mip chain
being sampled, not in the level being asked for.

### Where the 1243 pixels sit, row by row

Counting the pixels over 24/255 per row inside the sphere gives two blocks and
nothing else: rows 179–222 and rows 277–321, with rows 223–276 and everything
below 322 clean. That is the `alphaMap`'s band structure — the sphere is seven
horizontal stripes of alpha 1 and 0, and **only the opaque stripes differ**.
Where the material is fully transparent the port's pixels are three's, which is
one more confirmation that the background, the cube conversion and the blend
are right, and that the error is in the sphere's own shading.

Within a stripe the error hugs the left limb. Two terms are large there and both
are read from a mip chain at a roughness-driven level:

* the **transmission** sample, at `9.64 * Roughness` of the opaque-frame copy;
* the **PMREM radiance**, whose Fresnel weight goes to 1 at grazing angles —
  which also means `( 1 - F )` takes the transmitted light to nearly nothing at
  the very edge, so the outermost pixels of the crescent are mostly reflection,
  not transmission.

The port's PMREM is not exact either (`webgpu_pmrem_test` is 27 px, and there
the environment is not magnified by a mirror sphere), so the two had to be
separated by experiment: render the same scene with `transmission` at 0 (the
one-line change, not committed) and difference both renders against three's
frame over the striped region.

| over the opaque stripes, x 288–500 | mean abs diff | px > 24/255 |
| --- | --- | --- |
| ours at `transmission` 1 vs three | 6.75 | 1241 |
| ours at `transmission` 0 vs three | 72.14 | 18024 |
| ours at 1 vs ours at 0 | 75.02 | 18077 |
| *left limb only (x 288–360)*, ours at 1 vs ours at 0 | 78.81 | 5763 |

**The transmitted term dominates the crescent**, by 78/255 against a residual of
11/255 there — the Fresnel argument above is wrong, `( 1 - F )` does not kill
the transmission at this incidence. So the error is in the transmitted term,
not in the PMREM radiance.

Inside that term, the level asked for is right: `applyIorToRoughness` is
byte-identical to three's (`roughness * clamp( ior * 2 - 2, 0, 1 )`), and so is
the whole `Roughness` line, `min( max( materialRoughness, 0.0525 ) + max(
max( g.x, g.y ), g.z ), 1.0 )` over the same
`max( abs( dpdx( normalViewGeometry ) ), abs( -dpdy( normalViewGeometry ) ) )`.
The port asks for three's LOD and gets back something ~4% softer, so what
differs is the **content of the opaque copy's mip levels**, not the shader, the
level, or the reflection. (Three's sampler descriptors in `dump.json` cannot
settle the sampler question: three reuses and resets one descriptor object, so
all five come out as nearest/nearest/nearest, which the PMREM read disproves.)

## What was ruled out

* **The transmission pass ordering.** The background is drawn into the opaque
  frame before the copy — a missed copy leaves a milky sphere, which is the
  failure §26.4 describes and is not what the diff shows.
* **`thickness` / `attenuation`.** Both are at their defaults on the graded
  frame (above), so the refraction offset is zero by construction.
* **The exposure and tone mapping.** ACES Filmic at exposure 1; a mismatch
  there moves the whole frame, and the background matches.

## What was left out

* **The viewport uniform and the copy ordering** (both checked, both correct):
  `ViewportSize` is `( target.width, target.height )` = 800×500 physical
  pixels, so `log2( cameraViewport.z )` is three's 9.64 and not 8.64; and the
  copy is taken between the two halves of the split pass, after the background
  and every opaque draw — the background is visible through the sphere, which
  it would not be otherwise.
* **The mipmap pass itself**, by inspection against
  `WebGPUTexturePassUtils`: the same hand-written shader, the same
  `createSampler( { minFilter: Linear } )` (so nearest magnification and
  nearest mipmap filter, both defaults), the same `noFlip` uniform, the source
  view restricted to `base_mip_level - 1` with `mip_level_count: 1`, and the
  destination the single next level. The chain is generated the way three
  generates it.
### The opaque copy's mip chain is right, level by level

Measured, not inspected: a temporary `read_opaque_frame_mip( level )` on the
renderer (a `copy_texture_to_buffer` at `mip_level`, plus `COPY_SRC` on the
texture — neither committed) dumped levels 0–3 of `viewportOpaqueMipTexture`
after the graded frame, and each level was compared against a downsample of the
one above it computed in numpy. Three's `WebGPUTexturePassUtils` mipmap pass
samples the previous level with a `minFilter: Linear` sampler at the
destination texel centre, so the reference is a **bilinear tap at the
destination texel centre**, which equals a 2×2 box average only when the source
dimension is even.

| level | size | vs 2×2 box of the level above | vs bilinear at the destination texel centre |
| --- | --- | --- | --- |
| 1 | 400×250 | mean abs 0.00007, max 0.0068 | mean abs 0.000067, max 0.0068 |
| 2 | 200×125 | mean abs 0.00007, max 0.0068 | mean abs 0.000069, max 0.0068 |
| 3 | 100×62 | mean abs **0.0155**, max 1.04 | mean abs **0.000105**, max 0.0076 |

The whole chain agrees with the bilinear reference to 1e-4 on a mean level of
0.259 — float16 noise, and nothing else. Level 3 is the one level that is *not*
a box average, and the reason is arithmetic rather than a bug: its source is
200×**125**, an odd height, so 62 destination rows straddle 125 source rows and
the linear tap lands off the half-texel grid. Three's pass, sampling the same
way from the same odd-height level, reproduces it exactly. Compounding three
box steps from level 0 instead gives the 0.0155 discrepancy in the first
column, which is an artefact of the reference, not of the texture.

The shift test rules the other failure modes out too: level 1 against the box
average of level 0 scores 0.00007 at offset (0, 0) and 0.023 or worse at every
neighbouring whole-texel offset, so there is no half-texel shift and no missing
or skipped level. The MSAA suspect also falls by inspection —
`PassTarget.color_texture` is `inner.texture`, the *resolve* target, and the
first half of the split pass is submitted (and therefore resolved) before
`copy_framebuffer_to_opaque_frame` runs.

**So the opaque copy's mip chain is not the bug.** The port's transmitted term
is ~4% softer than three's while reading a byte-correct chain at a
byte-identical LOD, which leaves the *sampling* of that chain rather than its
content:

* **The bicubic filter's `textureDimensions` levels.** `textureBicubicLevel`
  takes `textureDimensions( map, i32( lod ) )` at both the `floor` and the
  `ceil` level and derives its eight taps from them. The dimensions of an
  odd-sized level are exactly where a port and three can disagree without the
  WGSL differing: `textureDimensions` returns the *allocated* size, and the
  chain here goes 800×500, 400×250, 200×125, 100×62, 50×31, 25×15 — five odd
  dimensions in ten levels. Worth checking the port's `lod` clamp against
  `textureNumLevels`, and whether the `ceil` level is clamped at all.
* **The sampler bound for the transmission read.** Three binds the opaque copy
  with `LinearMipmapLinearFilter`, but `textureBicubicLevel` calls
  `textureSampleLevel` at explicit levels, so the mipmap filter is bypassed and
  only min/mag matter. If the port's binding resolves to a sampler whose
  `mag_filter` is nearest, every one of the eight taps quantises, which reads
  as exactly this: in place, slightly softer where the LOD is fractional, no
  shift, and worst where the LOD climbs fastest — the limb.

The second is the cheaper check and the better fit for "softer and darker, in
place", so it is the next thing to measure.

* **The last 98 pixels.** The evidence above says the opaque-frame *mip chain*
  is slightly softer than three's, not that the shader or the LOD is wrong.
  Three allocates two 800×500 ten-level textures (ids 212 and 265 in
  `dump.json`) — `viewportMipTexture()` and `viewportOpaqueMipTexture()` — and
  the port allocates one; the contents agree here, but each is mipped by its
  own chain of passes. Both of those are now measured and clean — see "The
  opaque copy's mip chain is right, level by level" above.
* **A README "Examples graded green" row and a gallery entry**, since the
  example is not green.
* **Dispersion**, still — `KHR_materials_dispersion` is dead code for this page
  as it was for the barn lamp (§26).
