# `webgpu_volume_perlin` — a raymarched `Data3DTexture`

**Result: 23 of 100000 pixels different, limit 0.1%.**

A 128³ `Data3DTexture` of Perlin noise (`ImprovedNoise`, filled on the CPU,
`RedFormat` / `r8unorm`, `LinearFilter`) is raymarched inside a unit box drawn
`BackSide`, by `RaymarchingBox` from `examples/jsm/tsl/utils/Raymarching.js`.
The first of 200 steps whose value crosses `threshold = 0.6` is refined by
four bisection steps. The hit is then shaded with the volume's
central-difference gradient (`Texture3DNode.normal()`) and its position.

## What was added (issue #166)

| Area | What | Why |
|---|---|---|
| `src/textures/data3d_texture.rs` | `Data3DTexture`, and `Data3DTexture::storage()` for `Storage3DTexture` | `new Data3DTexture( data, w, h, d )` with any byte format |
| `src/addons/improved_noise.rs` | `ImprovedNoise` | `examples/jsm/math/ImprovedNoise.js`, in f64 as the JS is |
| `src/addons/raymarching.rs` | `raymarching_box()` | `examples/jsm/tsl/utils/Raymarching.js` |
| `src/nodes/tsl.rs` | `texture_3d()`, `Texture3DNode::{sample, sample_r, normal}` | `texture3D( t, null, level )` and the two methods a raymarcher calls |
| `src/nodes/tsl.rs` | `loop_float()`, `break_loop()`, `boolean()` | `Loop( { type: 'float', start, end, update } )`, `Break()`, `bool( false )` |
| `src/nodes/builder.rs` | `bool` uniforms as a `u32` member read through `bool( … )` into a var | `UniformNode` of type `bool` |
| `src/nodes/builder.rs` | a scalar `textureSampleLevel( … ).x` | `.sample( uv ).r` builds the texture node as a `float` |
| `src/renderer/mod.rs` | `ensure_data3d_texture()`, the `D3` view, the 3-D sampler | `WebGPUTextureUtils` for `is3DTexture` |

`docs/nodes.md` §28 has the detail.

## What the WGSL found

`tests/nodes_texture_wgsl.rs` diffs the fragment's uniform block and its whole
colour flow against three's dump: the slab test, the float-indexed `for`, the
bisection with its two `select`s, the six-deep nested `If` of `normal()`, and
`break`. They match line for line after §8's renumbering. The vertex stage's
two ray varyings are checked by expression. The same test pins the volume's
bytes. A checksum over all 2 097 152 texels matches three's `ImprovedNoise.js`
run under Node through the page's own `Uint8Array` fill, whose `ToUint8`
wraps a noise value of exactly 1 to 0 rather than saturating.

Two things in the dump are not in the page's source and are reproduced on
purpose:

* the hit-box temps and `p = surfacePos + 0.5` are `let`s, because three at
  5f610f5 turns a multiply-read temp into one (§8, "Usage-promoted temps");
* `refine`, a `uniform( true )`, is a `u32` struct member cast back with
  `bool( … )` into a var at its first read, inside the hit branch.

## What the pixels found

23 pixels, scattered singly along the edges of the surface's holes (the
diff strip in `target/e2e/webgpu_volume_perlin/`). There a ray grazes the
threshold, so which of two steps finds the hit depends on the last bits of
the trilinear filter. Nothing else moved on the first render.

## Steady frame

5.2 ms, 2 draws, 13 triangles. That is the cost of the march: up to 200
trilinear taps per fragment over the half of the frame the box covers.
