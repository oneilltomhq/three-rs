# webgpu_pmrem_equirectangular — done

2026-09-25: regraded against three.js 5f610f5 (the cube PMREM of 2f80402, #146): 0 of 100000 pixels (was 1).

**1 different pixel of 100000** against three's own `test/e2e/image.js` at
three's own 0.1% threshold, and the twenty-nine rows that were already green
are unchanged to the pixel.

The picture is `webgpu_pmrem_test`'s: `PMREMGenerator.fromEquirectangular` over
a 6x5 grid of `MeshPhysicalNodeMaterial` spheres, the same environment behind
them. Every node in it was already ported, and the port's generated WGSL for
both modules matched three's dump before a line of this rung was written.

What was not ported is the file. `royal_esplanade_2k.hdr.jpg` is an **UltraHDR**
image: a baseline sRGB JPEG with a second JPEG — the gain map — appended after
it, an MPF APP2 index giving the second image's offset, and per-image metadata
saying how to recombine them into HDR. The rung is `UltraHDRLoader.js`, and the
one differing pixel is the whole of what the recombination got wrong.

## Grade first

Before any Rust: `npm run test-e2e-webgpu -- webgpu_pmrem_equirectangular` in
`~/src/vendor/three.js`, twice, under the GPU lock. Both runs: `Diff 0.0% ...
TEST PASSED!`. The example is not in `test/e2e/puppeteer.js`'s exception list
and has a reference screenshot. The rung grades honestly.

## Deltas against the brief

| brief says | actually |
|---|---|
| a `MeshBasicNodeMaterial` `colorNode` with the map as `scene.background` | `scene.backgroundNode = pmremTexture( map, normalWorldGeometry, uniform( 0.5 ) )`, and a 6x5 grid of `MeshPhysicalNodeMaterial( { roughness: i / 5, metalness: j / 4, envMap: map } )` on `SphereGeometry( 0.4, 64, 64 )`. The background is a *node*, so it has no `backgroundRotation` and no `backgroundBlurriness` and samples at a fixed roughness of 0.5. `Background::Pmrem` is the other branch and would be wrong. |
| "33 rows in your worktree" | 29 before this rung, 30 after. The e2e binary has 36 tests; the extra six are the gates (render-target readback, the white furnace, the steady-frame checks) rather than graded rows. |

## What was added

| file | what |
|---|---|
| `src/loaders/ultra_hdr_loader.rs` | the port of `examples/jsm/loaders/UltraHDRLoader.js`: `UltraHdrLoader` (`new`, `set_data_type`, `data_type`, `load`, `parse`), `UltraHdrMetadata`, `UltraHdrTexData`, `UltraHdrData`, `apply_gain_map` and `srgb_to_linear`. JPEG section scan, MPF parse, XMP `hdrgm:` parse, ISO 21496-1 parse, gain-map recovery, half-float or float output. |
| `src/loaders/mod.rs` | re-exports of the six public items above. |
| `examples/webgpu_pmrem_equirectangular.rs` | the example, in the page's order. |
| `tests/e2e/main.rs` | the graded row plus its steady-frame assertion. |
| `examples/dump_wgsl.rs` | `pmrem_equirectangular_physical`, the grid's `i = 0, j = 4` cell (roughness 0, metalness 1 — the mirror sphere, where a wrong environment shows first), and the `pmrem_background` comment now names this example and its dump. |
| `docs/nodes.md` §21 | the divergence list. |

It is written as a general loader, not as this example's decoder: eight further
examples (`webgpu_loader_gltf`, `_anisotropy`, `_sheen`, `webgpu_mrt`,
`webgpu_materials_transmission`, `webgpu_deferred`, `webgpu_performance`,
`webgpu_custom_fog_background`) load UltraHDR files. The metadata parser has
nine unit tests of its own, covering both containers the format allows and the
JS quirks below; they need no GPU.

## What the pixels found

**`SRGB_TO_LINEAR` truncates.** Upstream indexes a 1024-entry table with
`value | 0`, so 512.9 reads entry 512 rather than interpolating between 512 and
513. Reproducing the interpolation instead of the truncation moved hundreds of
pixels. `srgb_to_linear_truncates_in_the_table_range` pins it.

**`OffsetSDR` and `OffsetHDR` are rescaled.** They are stored as a fraction of
the SDR range and upstream divides both by `1 / 64` to put them on the 0-255
axis the recovery loop works on. Reading them raw is a visible, uniform lift in
the shadows.

**`maxDisplayBoost = 1.8 ** ( hdrCapacityMax * 0.5 )`** — not `2 **`, and not
`hdrCapacityMax` unhalved. Either mistake rescales the whole environment and
therefore every sphere.

**The half-float conversion truncates too.** `DataUtils.toHalfFloat` drops the
low mantissa bits rather than rounding to nearest. Rounding to nearest is
"better" and disagrees with the reference on a few hundred pixels.

The one pixel that remains is on a specular highlight, and it is the JPEG
decoder: zune-jpeg's IDCT and libjpeg-turbo's differ by a unit in the last
place on some blocks, and there one unit in the gain map crosses a rounding
boundary in the half-float. See `docs/nodes.md` §21.

## What was ruled out

**The gain map is not rescaled.** `applyGainMap` draws it onto a canvas sized
to the SDR image, which would resample a smaller gain map. Both SOF0s in this
asset are 2048x1024, so the draw is a 1:1 copy. `resize_bilinear` is kept
because the format permits a half-scale gain map, but no in-tree asset
exercises it.

**The ICC profile is the identity.** The asset's profile is a plain sRGB one
(desc "sRGB", Google copyright), so Chromium's canvas conversion does nothing.
Upstream's own feature list says "ICC profile (not implemented)". The port
skips the segment.

**`generateMipmaps` does not reach a pixel.** The flag is set and three's dump
duly contains twelve mipmap passes, but `_getEquirectMaterial` samples at an
explicit level 0. Kept for pass-structure fidelity, not for correctness.

**One PMREM, not thirty-one.** `PMREMNode` caches per renderer in a `WeakMap`
keyed on the *source* texture, so the background's `pmremTexture( map, ... )`
and all thirty materials' `envMap` share one generated atlas. The example has
one `PmremEnvironment` for the same reason.

**The physical material's WGSL divergence is old.** `pmrem_physical` differs
from `m08` by 171 lines in the multi-scattering / DFG block
(`multiScatteringDielectric`, `singleScatteringMetallic`,
`multiScatteringMetallic`, `dfg`, `multiScatteringCompensation`) and in
`NORMAL_normalView`. Both are already in `docs/nodes.md` §8 from earlier rungs;
this rung introduced neither. `pmrem_background` against `m06` is an exact
match, 0 differing lines.

## What was left out

* **No ICC profile handling**, as above; an asset with a wide-gamut profile
  would decode wrong here and in three.
* **No `resize_bilinear` reference gate**, because no in-tree asset has a
  scaled gain map.
* **No async load.** `UltraHdrLoader::load` is synchronous, so the 0x1
  placeholder `DataTexture` three returns before its fetch resolves has no
  counterpart. The graded frame is after the callback in both.
* **`OrbitControls` is inert** here as in the sibling PMREM examples: no
  pointer events, and the camera already looks at the target.
