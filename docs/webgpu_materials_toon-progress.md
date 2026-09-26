# `webgpu_materials_toon`

Status: **green.** 0 of 100000 pixels against three.js 5f610f5's own
`test/e2e/image.js`, threshold 0.1%. Intel Iris Xe, Mesa 25.3.6, wgpu on
Vulkan. Steady frame 58.9 ms (debug build), 438 draw calls, 421985 triangles.

A 6 x 6 x 6 cube of `MeshToonNodeMaterial` spheres: hue along x, lightness
along z, darkening along y. Each hue column has its own `RedFormat`
`DataTexture` gradient ramp, with one more step per column. They are lit by an
ambient light and a decay-free point light that rides a small white sphere.
Four text labels complete the scene. It all goes through `toonOutlinePass()`,
which draws every toon sphere a second time as a black back-face outline.

## Reconciling with the plans

There is no scout plan. The page was ported from
`three.js/examples/webgpu_materials_toon.html`. Three's WGSL was dumped by
`tools/dump-webgpu.mjs` into `target/dumps/webgpu_materials_toon/`, which is
not committed. It was diffed against `examples/dump_wgsl.rs`'s new `toon`
section (three's `m05`) and `toon_outline` section (three's `m02` and `m03`).
Both match statement for statement. The one difference is the port's usual
`nodeVarN` where three writes `let nodeConstN`.

## What was added

| area | what |
| --- | --- |
| `src/materials/toon.rs` | `ToonLightingModel`: `getGradientIrradiance()` (gradient-map lookup, or the built-in `fwidth`-antialiased two-step ramp), `direct()` |
| `src/materials/mod.rs`, `node_material.rs` | `MaterialKind::Toon`, `gradient_map`, `MeshBasicNodeMaterial::toon()`, the `MeshToonNodeMaterial` alias; Toon takes Lambert's flow with its own per-light term |
| `src/nodes/display/toon_outline_pass.rs` | `ToonOutlinePassNode` / `toon_outline_pass()` and its `Toon_Outline` material |
| `src/renderer/mod.rs` | the renderer's `toon_outline` hook, which puts an outline draw before every toon draw |
| `src/textures/texture.rs` | `Texture::data_r8()`, a `RedFormat` `DataTexture` |
| `src/nodes/tsl.rs` | `texture_with_uv()`, which is `texture( map ).context( { getUV } )` |
| `tools/dump-webgpu.mjs` | dropped a stale `pageFile` line that threw `ReferenceError` on every page |
| `examples/` | `webgpu_materials_toon.rs`; `dump_wgsl` sections `toon` and `toon_outline` |

`docs/nodes.md` §42 is the long form.

## What the pixels found

The page graded 0 on its first run. Nothing in it was tuned against the
screenshot. The outline is most of the picture's character: rendering the
same scene through a plain `PassNode` (outline off) is 2973 pixels off.

**The ramp is read through the map's uv matrix.** `getUV` swaps in
`vec2( dotNL * 0.5 + 0.5, 0 )` for the default `uv()`, but the texture node
still applies `updateMatrix`. So three's dump multiplies the coordinate by
the map's uv matrix before the clamp-wrap `textureLoad`. The port does the
same, through `transformed_uv`.

**The ramps are nearest-filtered, so they are unfilterable.** `DataTexture`
defaults to `NearestFilter` for both min and mag. That is what makes the
steps hard, and it is why the tap is a `textureLoad` and not a
`textureSample`.

The steady frame is slow for a debug build because it makes 438 draws, and
turning the outline off does not shorten it much. The steady strip holds
frames 2 and 3 at zero builds and zero uploads. The frame time is noisy on a
loaded machine: 58.9 ms when run alone, and 101.9 ms in a full ladder run
while other builds were going. Both are well under the 600 ms ceiling.
