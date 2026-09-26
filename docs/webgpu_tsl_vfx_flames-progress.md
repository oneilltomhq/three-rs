# `webgpu_tsl_vfx_flames` — two billboarded flames from the TSL wrapper tier

**Result: 31 of 100000 pixels different, limit 0.1%.** Steady frame 5.4 ms,
5 draw calls, 9 triangles.

Two `SpriteNodeMaterial` flames over a `0x201919` background: the left one a
cellular noise ramped through a five-stop gradient, the right one a cellular
noise displaced by a Perlin noise. Both are turned toward the camera by
`billboarding( { horizontalRotation: true } )` in a `vertexNode`. The page
landed as the graded rung for issue #141, the thin-wrapper TSL tier: nothing
in it needed a new renderer feature, only the TSL names it calls.

## What it needed from the wrapper tier

All in `src/nodes/tsl/wrappers.rs`, each with a doc comment naming its JS
source and pinned against three's WGSL dump in `tests/nodes_tsl_batch.rs`:

| TSL | Used for |
|---|---|
| `billboarding()` (`utils/SpriteUtils.js`) | both flames' `vertexNode` |
| `spherizeUV()` (`utils/UVUtils.js`) | both flames' bulging uv |
| `.remap()` | flame 1's gradient lookup |
| `.step()`, `.smoothstep()` as methods | the noise thresholds and edges |
| `x += …` on a swizzle | flame 2's displaced `x` |

## Draw calls

5 = two flames × two passes + the output pass. A transparent `DoubleSide`
material is drawn back faces first, then front faces, as three's
`renderObject()` does, so each quad is two draws; 9 triangles is four quads'
eight plus the output pass's one.

## Stand-ins

Two, neither of which changes a pixel; the example's module doc has the
detail.

- **`THREE.Sprite` is a `Mesh` over the sprite's own quad.** With a
  `vertexNode` set, `NodeMaterial.setupVertex()` returns it as it stands and
  `SpriteNodeMaterial`'s position seam — the only reader of `Sprite.center`
  — never runs. What is left of a `Sprite` is its shared geometry, which the
  example rebuilds vertex for vertex. A real `Sprite` object is issue #143's.
- **The gradient `CanvasTexture` is filled in Rust.** The page draws a
  128 × 1 `createLinearGradient` through five stops; the example evaluates the
  same gradient at each pixel centre, interpolating the sRGB bytes linearly as
  a 2D canvas does, and tags the texture `SRGBColorSpace`. Nothing here is
  read from the reference image.

## The 31 pixels

A few small clusters, mostly on the right flame's hard `step` edges, where a
`smoothstep` or `step` of a filtered noise sample is close to its edge and a
rounding difference decides the side. Nothing structural.
