# `webgpu_materials_transmission`

Status: **graded, 16 of 100000 pixels**, against three.js 5f610f5's own
`test/e2e/image.js` and its 0.1% (100 px) threshold, on Intel Iris Xe, Mesa
25.3.6, Vulkan. three.js itself scores 0.0% against the same reference here,
twice (`npm run test-e2e-webgpu -- webgpu_materials_transmission`, both runs
PASSED). The steady frame is 6.3 ms (4 draw calls, 9921 triangles), and the page
is in `steady_frame_builds_nothing`.

The scene is a `SphereGeometry( 20, 64, 32 )` with `transmission: 1`,
`side: DoubleSide`, `transparent: true` and a striped `alphaMap`. It sits in
front of the UltraHDR `royal_esplanade_2k`, which is both the background and,
through the PMREM, the environment. It is the direct test of the transmission
path that `webgpu_loader_gltf_anisotropy` brought in (`docs/nodes.md` §26).
There the glass is 2% of the frame. Here the transmissive surface is a fifth of
the frame, and only the background is behind it. The page's own notes are
`docs/nodes.md` §96.

## The page, as graded

- **No light, and no inner mesh.** The scene is one mesh, so this page does
  not exercise `PhysicalLightingModel.direct()` either.
- **`thickness` is never applied.** `params.thickness` is 0.01, but only the
  GUI `onChange` writes it, and the graded frame is the first one. The material
  keeps the default 0, so the volume ray has zero length, `volumeAttenuation`
  is the identity, and the refracted read lands on the fragment's own screen
  position. The constructor passes `opacity`, `metalness`, `roughness`, `ior`
  and `exposure` with the GUI's defaults.
- **OrbitControls** with `minDistance` 10 and `maxDistance` 150, as on the
  page. Nothing moves them for the graded frame.

## History

This page was first ported on `rung-materials-transmission` (issue #228). It
stalled at 198 pixels (r186's frame, the port's equirect PMREM). That branch
went stale. It was ported forward onto main file by file:

| from the old branch | on main |
| --- | --- |
| `Material::alpha_map`, `material_opacity_with_map()` | already there, as `alpha_map` and `material_opacity_for()`; dropped |
| `examples/webgpu_materials_transmission.rs` | re-applied, on main's cube PMREM (`PmremEnvironment::from_equirectangular`), `RendererParameters` and `OrbitControls`; the default `LinearMipmapLinear` min filter makes the old `set_min_filter` redundant |
| the `#[ignore]`d e2e test | re-applied, in the 3dlut shape, and now not ignored |
| `examples/dump_wgsl.rs`'s `materials_transmission` section | replaced by a fixture gate, `tests/nodes_transmission_wgsl.rs`, against three's `m12` / `m14` |
| §29 of `docs/nodes.md` | §96, rewritten for what was found |

The fresh port on main scored **208**.

## What the pixels found

The old branch's measurement still held on main. Against three's own 800×500
frame, every pixel off by more than 24/255 was inside the sphere (rows
179–361, columns 288–500), and only on the alpha map's opaque stripes. The
background, the bands' phase and the fully transparent stripes were right. The
old branch had shown that the opaque copy's mip chain is correct level by level
(a temporary readback against a numpy bilinear reference, matching to 1e-4).
It concluded that the *sampling* of that chain was off.

Two things were wrong, and neither was in the WGSL:

1. **The copy's mag filter** (§96.4). Three's `FramebufferTexture` is
   `NearestFilter` for magnification, and the port's copy was linear. Wherever
   the LOD is below 1, `textureBicubicLevel`'s explicit `textureSampleLevel(
   …, 0 )` magnifies, so three's taps there are point samples. Fixing it took
   the port from 208 to 205: real, but small.
2. **The copy's timing** (§96.3). A crop of the worst block showed what
   three's opaque stripes have and the port's lacked: the ceiling lights'
   reflections, and blue and red at the left limb. Those are the sphere's own
   back faces, seen through the front face. The port's opaque stripes were
   about 33/255 darker in exactly those pixels. Three's front face reads
   `viewportOpaqueMipTexture()`, and its back face reads a separate
   `viewportMipTexture()`. Each is copied at the first draw that reads it,
   so the front texture is copied *after* the back faces are drawn. The port
   copied once, before both. The old branch had noticed the second texture
   (two ten-level 800×500 allocations in three's `dump.json`) and assumed that
   on this page "the contents agree". They do not.

With both fixed, the page scores **16**.

## Checked and unchanged

- `webgpu_loader_gltf_anisotropy` stays at 94 (its README number) and
  `webgpu_furnace_test` at 0. Both draw only front-side transmission, which
  still makes one copy at the same draw.
- The WGSL matches three's apart from the differences listed in §96.6. Main
  already had the r187 cube PMREM read
  (`textureSampleLevel( cube, dir, maxLod * r * ( 2 - r ) )`) and the 0.045
  roughness floor, which the old branch did not.

## Left out

- `RenderList.transparentDoublePass`'s order: all back faces before any front
  face, for every transmissive `DoubleSide` material, transparent or not
  (§96.3). With one object it is the same order as the port's.
- Dispersion (`KHR_materials_dispersion`), dead code here as for the barn lamp.
