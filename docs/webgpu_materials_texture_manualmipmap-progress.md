# webgpu_materials_texture_manualmipmap — done

**Status: green, 81 of 100000 pixels** (limit 100). This is the graded rung for
issue #140, `scene.fog`. Linear fog is the page's only fog, and it is what
fades the floor into the black background.

## Why this example and not `webgpu_instance_sprites`

The issue names `webgpu_instance_sprites` first and allows another `Fog` or
`FogExp2` example if that one needs something large. It does. It needs:

* the `Sprite` object, with its shared quad and its `center` uniform
* `alphaMap`
* `alphaTest`

`Sprite` is issue #143, which is being built on its own branch
(`raycaster-sprite`), so building it here would collide. Of the other 20
`webgpu_*` pages that set `scene.fog`, this one needed the least new code:

* two things on `Texture`
* scissor and `autoClear = false`, which the port already has
  (`webgpu_lines_fat`)

It also has black backgrounds, so there is no transparent canvas for the
grader to composite over the page's CSS colour.

`webgpu_textures_anisotropy` was the other candidate. Its sky is the page's
`#f1f1f1` body showing through a transparent canvas, and its floor depends on
the hardware's anisotropic filter.

## What was added

| area | what |
|---|---|
| `src/objects/fog.rs`, `Scene::fog`, renderer | `Fog`, `FogExp2`, and `scene.fog` as render-group uniforms (`docs/nodes.md` §28) |
| `src/textures/texture.rs` | `Texture.mipmaps` (`set_mipmaps`), counted by `mip_level_count()` as in `Textures.getMipLevels()`. Adds the other three `MinFilter` constants: `NearestMipmapNearest`, `NearestMipmapLinear`, `LinearMipmapNearest` |
| `src/renderer/mod.rs` | the upload writes each page-supplied level into its mip, flipped like level 0, and generates none (`WebGPUTextureUtils.updateTexture()`'s `mipmaps.length > 0` branch) |

## The page, read closely

* **The graded frame.** `animate()` eases the camera towards the mouse, which
  never moves, so the first frame lifts it from y = 0 to y = 10. Frame one is
  graded.
* **`mipmap( size, color )`** fills a `#444` canvas and paints two opposite
  quarters. The port builds the same bytes. For the 1×1 level each
  `fillRect( …, 0.5, 0.5 )` covers a quarter of the pixel, and the canvas
  blends it in by that coverage, one rectangle after the other. The port
  does the same arithmetic. That level is far beyond the fog's `far`, so
  none of its pixels are visible.
* **The two paintings.**
  * They are `clone()`s of one loaded texture: the port's `clone_texture()`.
  * `minFilter = LinearFilter` does **not** turn mipmaps off in WebGPU.
    `generateMipmaps` stays true, so three allocates and generates the chain,
    and the sampler's `mipmapFilter` is `nearest`. Three's dump has the
    `mipmap` pipeline for them, and the port does the same.
* **The scissor halves.** `renderer.clear()` runs once, then each half
  renders its scene with `autoClear = false`. The black background clears
  the whole frame-buffer target, and the scissored output blit copies only
  that half to the canvas. The 2 px gap between the halves stays cleared
  and transparent over a black page.

## What the WGSL says

These come from three's r186 dump of this page
(`tools/dump-webgpu.mjs webgpu_materials_texture_manualmipmap`). Four
fragment modules carry the fog. Each has the same fog statement as the
port's `dump_wgsl` `difference_scene`, with `render.nodeUniformN` colour,
near and far.

## What the pixels found

The 81 differing pixels are scattered single pixels:

* along the checker edges of the right-hand, `NearestMipmapNearest`, floor
* a few at the bottom edge of the left floor

That is texel-edge rounding under a nearest filter. The painting, the frame,
the shadow and the fog gradient do not show up in the diff.

## Numbers

| | |
|---|---|
| different pixels | 81 / 100000 |
| steady frame | 2.5 ms (`viewer --headless --frames 40`) |
| draw calls | 11 (4 meshes × 2 scenes, plus 3 output blits: one per render and one for the manual clear) |
| triangles | 19 |
