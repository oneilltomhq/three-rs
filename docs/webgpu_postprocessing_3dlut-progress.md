# webgpu_postprocessing_3dlut

Branch `display-pages-1`.

**Green at 0 pixels of 100000** (limit 0.1%), steady frame 1.9 ms, 4 draw
calls, 5705 triangles.

## What the score says, and what it does not

The graded frame is the coffee mug and its smoke, through `renderOutput()`
and `Lut3DNode` with `Bourbon 64.CUBE` at full intensity. A wrong lookup
would show on every pixel, so the score checks these things:

- `LutCubeLoader`'s table;
- the half-texel pull-in;
- the `textureSample` of a 3D texture;
- `mix` with the base alpha.

The smoke's noise sample in `positionNode` is also in the frame. That sample
needed the builder to emit `textureSampleLevel( …, 0 )` outside the
fragment stage, as three does.

The page loads all nine tables in `init()`, so the rung also runs
`Lut3dlLoader` and `LutImageLoader` on the vendor files. Nothing grades their
output there. `tests/loaders_lut.rs` does that, against three's own loaders:

- `.CUBE` and `.3dl` through `tests/lut/gen.mjs` under node, as
  `UnsignedByteType` and `FloatType`, plus synthetic inputs for each quirk;
- the PNG strips in Chrome, with `flip` off and on.

The rung grades one table at one intensity. `tests/lut_3d_frames.rs` covers
the rest on the GPU, with identity and inversion tables:

- intensity 0 returns the input;
- the inversion maps `c` to `1 - c`;
- swapping tables changes the next frame.

## What this rung adds

| area | what |
|---|---|
| `src/nodes/display/lut_3d.rs` | `Lut3DNode.js` |
| `src/loaders/lut_cube_loader.rs`, `lut_3dl_loader.rs`, `lut_image_loader.rs`, `lut_text.rs` | the three LUT loaders, and the JavaScript regex and number semantics they rely on |
| `src/nodes/tsl.rs` | `texture_3d_sampled()`, `texture3D( texture )` with no level |
| `src/nodes/builder.rs` | a plain sample outside the fragment stage is `textureSampleLevel( …, 0 )` |
| `src/error.rs` | `Error::Lut` |
| `tests/lut/` | `gen.mjs` and `oracle.json` |

## Divergences

- The page swaps tables by assigning `lutPass.lutNode.value`. The port builds
  a new `Lut3DNode` and hands it to the pipeline (`docs/nodes.md` §73).
- The loaders' `setType()` refuses a type other than `UnsignedByteType` or
  `FloatType`. Three falls back to float.
- `LutImageLoader` refuses an image that is not `size` slices of `size²`
  texels. Three would fail later, at upload.

## Left out

- `Loader`'s `manager`, `path` and `crossOrigin`, and the callback `load()`.
- The page's GUI: `params.lut` and `params.intensity` are public fields on
  `App`.
