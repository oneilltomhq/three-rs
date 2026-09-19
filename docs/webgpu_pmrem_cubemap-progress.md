# webgpu_pmrem_cubemap — first sitting (steps 1–3), done

**Status: the first half is done.** This is the CPU-and-plumbing half of
`webgpu_pmrem_cubemap`: the RGBE decoder, `HDRCubeTextureLoader`, half-float
textures and cube textures, an `rgba16float` render target with a readback, and
the render-target *viewport* the atlas tiles with (`rung-lines-fat-a`'s, which
this branch is rebased onto). There is deliberately **no image gate yet** — the
example itself needs the PMREM chain, which is the second sitting (steps 4–9).

Gates, all green:

* `cargo test -p three-rs --lib extras` — 5 tests, the `DataUtils` port.
* `cargo test --test hdr_loader` — 5 tests, bit-for-bit against three.js'
  own `HDRLoader.parse` (no GPU).
* `cargo test --release --test renderer_half_float_target` — the formatted
  target, the viewport corner, the half-float 2-D upload and the half-float
  cube.
* The full e2e ladder is **unchanged**, every rung on the number the branch
  below this one left: 0 / 60 / 0 / 1 / 31 / 0 / 7 / 4 / 18 / 40 / 6 / 0 / 4 /
  7 / 44 / 0 (depth_texture, instance_mesh, materials_basic, rtt, lights_phong,
  morphtargets, shadowmap, lights_physical, postprocessing_masking, tsl_galaxy,
  skinning, mesh_batch, compute_points, radial_blur, materials,
  postprocessing_ssaa).
* `cargo clippy --workspace --all-targets -- -D warnings` clean, `cargo fmt`
  applied.

## What was added

| area | what |
|---|---|
| `src/extras/data_utils.rs` | Port of `three.js/src/extras/DataUtils.js`: `to_half_float` / `from_half_float` via the same base/shift lookup tables ("Fast Half Float Conversions"). The tables are ported literally because the algorithm **truncates** the dropped mantissa bits rather than rounding — `to_half_float( 1.0 / 3.0 ) == 0x3555`, not `0x3556` — and every RGBE texel inherits that. New top-level `src/extras/` module, mirroring `three.js/src/extras`. |
| `src/loaders/hdr_loader.rs` | Port of `HDRLoader.js`. `HdrLoader::parse( bytes ) -> HdrTexData { width, height, header, gamma, exposure, data }`, with `HdrData::{ HalfFloat( Vec<u16> ), Float( Vec<f32> ) }`. Radiance header, the `-Y h +X w` resolution line, the adaptive-RLE scanline path with its four separate channel planes, and the flat fallback. |
| `src/loaders/hdr_cube_texture_loader.rs` | Port of `examples/jsm/loaders/HDRCubeTextureLoader.js`: six urls → one `CubeTexture`, face *i* at layer *i*, `HalfFloatType`, `LinearSRGBColorSpace`, `LinearFilter`, `generateMipmaps = false`. |
| `src/textures/texture.rs` | `Texture::data_rgba16float( w, h, &[u16] )` and `data_rgba32float( w, h, &[f32] )` — `new DataTexture( data, w, h, RGBAFormat, HalfFloatType )`, so `flipY = false`, `generateMipmaps = false`, linear filters, and the texture's `format` set. |
| `src/textures/cube_texture.rs` | `CubeTexture` gained the type/filter axis it was missing: `set_texture_type` / `texture_type`, `set_filters`, `set_generate_mipmaps`, `Image::rgba8` / `Image::rgba16float` constructors, and a public `borrow()`. `gpu_format()` now returns `Rgba16Float` for `HalfFloat`. Defaults are unchanged (`UnsignedByte`, `Linear` / `LinearMipmapLinear`), so no existing rung moves. |
| `src/renderer/mod.rs` | Cube upload takes its `bytes_per_row` from the format instead of a hard-coded `* 4` (an eight-byte texel through a four-byte stride shears each face diagonally). The cube sampler now reads the texture's own filters instead of hard-coding them. New `read_target_pixels_rgba16f()`; `read_texture_pixels` was split over a shared `read_texture_bytes` that takes the texel size from the format. |
| `src/renderer/render_target.rs` | Nothing of its own any more: the render-target viewport this rung needs is `rung-lines-fat-a`'s `RenderTarget::set_viewport( x, y, w, h )` / `viewport()`, which this branch is rebased on. See the note below. |
| `src/error.rs` | `Error::Rgbe { reason }`, carrying three.js' own `rgbe_error()` text. |
| gates | `tests/hdr/gen.mjs` + `tests/hdr/oracle.json`, `tests/hdr_loader.rs`, `tests/renderer_half_float_target.rs`. |

## The oracle, and why it is 13 KB

Plan §5.1 asks for a bit-for-bit decode gate. The obvious form — dumping every
half of all six 256×256 faces — is about 6 MB of JSON in the tree. Instead
`tests/hdr/gen.mjs` runs the *vendor's own* `HDRLoader.parse` under node over
the six pisa faces and records, per face, the dimensions, gamma and exposure,
an **FNV-1a-64 over every byte** of the resulting `Uint16Array`, the first and
last four texels, and twelve fixed probes. The hash is the bit-for-bit
assertion; the probes exist so that a failure says *where*. The same file also
carries 26 `toHalfFloat` and 33 `fromHalfFloat` value pairs.

`gen.mjs` copies `HDRLoader.js` to a temp directory to rewrite its
`from 'three'` import, and deletes that directory afterwards: **nothing in the
vendor checkout is modified**, and the `.hdr` files themselves are read out of
the vendor tree at test time via `testing::three_js_dir()` rather than copied
in. Nothing here derives from a reference image.

## Two things that are silently wrong if you get them wrong

**The rounding.** `toHalfFloat` truncates. It also takes a JS number, i.e. an
f64, and the f64→f32 narrowing happens *inside* it. So the RGBE scale
`2^(e-128) / 255` must be computed in **f64** and narrowed only on the way into
the half conversion; rounding it to f32 first double-rounds and moves a
scattering of texels by one ulp. `scale_of()` returns `f64` for exactly that
reason, and the float path in `tests/hdr_loader.rs` reads the exponent scale a
second way that does not go through the half tables at all.

**The orientation.** `HDRCubeTextureLoader` wraps each decoded face in a
`DataTexture`, which leaves `flipY` at `false`; it does **not** pick up
`HDRLoader`'s `texData.flipY = true`, which only `DataTextureLoader.load()`
would have applied, and `CubeTexture.flipY` is `false` too. A `-Y h +X w`
resolution string means the first decoded scanline is the *top* row, so the
faces go up exactly as decoded. A flip here mirrors the environment vertically
and still renders a perfectly plausible reflection —
`the_cube_loader_keeps_the_face_order_and_the_row_order` fails on a flip and on
a face swap, and `layer_zero_is_positive_x` catches a permuted `FACES` array.

Related, and worth knowing before step 4: `cube_texture()` negates x
(`WGSLNodeBuilder.generateTextureSample`'s `vec3( -uv.x, uv.yz )`) — three's
cube convention is mirrored against the GPU's. `tests/renderer_half_float_target.rs`
passes `vec3( -x, y, z )` to undo it and documents the cube-face projection it
uses to hit texel (0,0) of each face.

## The viewport — `rung-lines-fat-a`'s, fed from here

**The viewport this rung needs is scoped to the render target, and the
implementation is the fat-lines one.** This branch first grew its own
`RenderTargetInner.viewport` and a `PassTarget::viewport: Option<(f32, f32,
f32, f32)>`; `rung-lines-fat-a`, below it in the stack, had added the general
`Renderer` / `RenderTarget` viewport-and-scissor API by the time the two met,
so the duplicate went and PMREM's use now feeds
[`RenderTarget::set_viewport`](../src/renderer/render_target.rs) — `&self`,
`f64` arguments, reset to the whole target by `set_size`, resolved into
`PassTarget::viewport: Rect` and applied once in `draw()` right after
`begin_render_pass`, which is where three.js' WebGPU backend applies it
(`beginRender()`: `if ( renderContext.viewport ) this.updateViewport(
renderContext )`, before the first draw of the pass). A full-target rectangle
resolves to wgpu's own default, so no existing pass moved.

What that costs the PMREM chain is nothing: the render-target path does not
consult the renderer's own viewport state at all, so the atlas tiling — 21
passes writing disjoint tiles of two shared 768×1024 targets — sets the
rectangle on the target, binds it, and renders.
`the_viewport_clips_the_draw_to_the_top_right` survived the swap unchanged
apart from its arguments' type (`half as f64`, not `half`).

The convention is **top-left origin**, wgpu's and WebGPU's. That is not a
choice: three.js' WebGPU backend passes `renderTarget.viewport` straight to
`GPURenderPassEncoder.setViewport` with no flip — the *WebGL* backend is the
one that flips, and this port has no GL backend. The test asserts it the only
way that separates the conventions: the viewport is the **top-right** quadrant,
so the lit texels must be at small `y`. Under a bottom-left reading the same
numbers light the bottom-right quadrant and every PMREM mip lands one tile-row
off — a plausible, wrong, silently blurred environment. Note also that
`LoadOp::Clear` is *not* clipped by the viewport (only draws are), which is
what makes the three untouched quadrants readable at all.
(`rung-lines-fat-a` reached the same conclusion from the other side; see
`docs/webgpu_lines_fat-progress.md`, "The one correction to the plan".)

## What was already there

Roughly half of the scout's step 3 turned out to be in the tree already, and
was verified rather than written:

* `TextureType::{ UnsignedByte, HalfFloat, UnsignedInt, Float }` and
  `color_gpu_format()` → `Rgba16Float`.
* `RenderTargetOptions { texture_type, .. }`, so a formatted target only needed
  a readback that understands the format.
* `Texture`'s `format` field driving `data_len()` and the `upload_texture_2d`
  stride — the 2-D path was already format-correct; only the **cube** path was
  not.
* `PassNode`, `QuadMesh` / `render_quad`, and `MaterialKey` variants.

## Ruled out / left for later

* **`FloatType` cube faces.** `HdrCubeTextureLoader::set_data_type(
  TextureType::Float )` decodes fine but `load()` returns
  `Error::UnsupportedFormat`: the `rgba32float` *cube* upload is not
  implemented, and the example uses `HalfFloatType`. The 2-D `rgba32float`
  path (`Texture::data_rgba32float`) does exist.
* **`fgets`.** Upstream reads the header in 128-byte chunks with a 1024-char
  line cap; this port scans byte-at-a-time for the newline, Latin-1, with no
  cap. Same result on every well-formed file, simpler, and documented in the
  module doc.
* **The flat (non-RLE) branch** deliberately reproduces upstream's quirk of
  returning *every remaining byte* rather than exactly `w * h * 4`.
* **Mipmaps on the HDR cube** stay off, as upstream: PMREM builds its own
  pyramid inside the cubeUV atlas and never samples a mip of the source cube.

## What the second sitting (steps 4–9) will find in place

Everything below the PMREM generator itself:

1. `HdrCubeTextureLoader::new().set_path( … ).load( [ "px.hdr", … ] )` returns
   a `CubeTexture` that is `HalfFloat` / `Rgba16Float` / `NoColorSpace`,
   `LinearFilter`, one mip, 256×256, faces at the right layers the right way
   up — i.e. the example's `loadedEnvMap`, ready to be the PMREM input.
2. `RenderTarget::new_with_options( w, h, RenderTargetOptions { texture_type:
   TextureType::HalfFloat, depth_buffer: false, .. } )` gives
   `PMREMGenerator._allocateTargets()`'s target: an `rgba16float` attachment
   with no depth.
3. `target.set_viewport( x, y, w, h )` — `rung-lines-fat-a`'s, `f64`
   arguments — tiles that target in the **top-left** convention, applied to
   every draw in the pass: enough for the cubeUV atlas layout (768×1024,
   `maxMip = log2( 256 ) - 2 = 6`, tiles addressed by hand). See the section
   above.
4. `Renderer::read_target_pixels_rgba16f( &target ) -> ( w, h, Vec<f32> )` for
   gating the atlas without an image, decoding through the exact
   `from_half_float`.
5. `to_half_float` / `from_half_float` for any CPU-side half work (e.g. the
   SH irradiance path, or building a `DataTexture` by hand).
6. Half-float 2-D `DataTexture`s upload and sample correctly, which is what the
   equirect cousin of this example would need.

Still to write, all of it: `PMREMGenerator` (the blur chain, the
equirect/cubemap seam pass, the cubeUV packing), `PMREMNode`,
`EnvironmentNode`, the renderer pre-pass hook that runs the generator before
the frame, and the background. The e2e entry and the image gate against
`webgpu_pmrem_cubemap.jpg` belong to that sitting.

---

# webgpu_pmrem_cubemap — second sitting (steps 4–9), done

**0 different pixels of 100000** against three's own `test/e2e/image.js` at
three's own 0.1% threshold, and the sixteen rows that were already green are
unchanged to the pixel.

## What was added

| file | what |
|---|---|
| `src/renderer/pmrem.rs` | `PMREMGenerator`'s `fromCubemap` path: `_setSize`, `_allocateTarget`, `_init`, `_createPlanes`, `_textureToCubeUV`, `_applyPMREM`, `_applyGGXFilter`, `_setViewport`, and the two node materials |
| `src/nodes/pmrem_utils.rs` | `PMREMUtils.js`: `getFace`, `getUV`, `roughnessToMip`, `bilinearCubeUV`, `textureCubeUV`, the GGX VNDF importance sampler |
| `src/nodes/pmrem_node.rs` | `PMREMNode.js`: `_generateCubeUVSize`, and `PmremEnvironment`, which owns the generated atlas and the three cubeUV uniform cells |
| `src/materials/environment.rs` | `EnvironmentNode.js`: the reflection-vector radiance tap and the world-normal irradiance tap |
| `examples/webgpu_pmrem_cubemap.rs` | the example |
| `tests/pmrem.rs` | the numeric gates |
| `src/renderer/mod.rs` | `render_pmrem_mesh`, the PMREM analogue of `render_quad` |

## Twenty-one passes, and where they come from

A 256² source cube gives `lodMax = 8`, `cubeSize = 256`, an atlas of
**768 × 1024 rgba16float**, and `lodMeshes.length = lodMax - LOD_MIN + 1 +
EXTRA_LODS = 11`. That is one `_textureToCubeUV` pass writing mip 0's six
tiles, then ten GGX steps of two passes each — filter into the ping-pong
target, copy the tile back — which is the 21 render passes three's dump
records.

`_applyGGXFilter`'s arithmetic is the part that is easy to get subtly wrong,
so it is gated rather than eyeballed. `tests/pmrem.rs` carries the whole
ladder — `adjustedRoughness`, both `mipInt`s, the tile rectangle and the LOD's
plane size, for all ten steps — and compares bit-exactly. The numbers were not
transcribed from three's source: they were **printed by running it**, a real
`PMREMGenerator` driven through `_applyGGXFilter` with `_setViewport` and
`renderer.render` replaced by recorders. The script lived in the scratchpad,
never in the vendor tree, and is gone.

That oracle earned its keep immediately: `mipInt` is `_lodMax - lodIn`, and
`lodIn` runs to 9 against a `lodMax` of 8, so the last two steps sample at
**−1** and **−2**. Both subtractions were `usize` and both would have
underflowed. Nothing in the image would have told you — the extra LODs are the
roughest ones, which the example's `uniform( 0.5 )` background never reaches.

`_generateCubeUVSize` is gated the same way, at five atlas heights, which pins
the `7 * 16` floor that makes 256, 128 and 64 share a `texelWidth`.

## Three shaders, diffed against three's dump

`examples/dump_wgsl.rs` grew `pmrem_cubemap`, `pmrem_ggx`, `pmrem_background`
and `pmrem_physical`. The first three match `m01`, `m03` and `m05` line for
line, up to the divergence classes `docs/nodes.md` §8 already lists.

Getting there needed two node-system facts that were not written down:

* **`ConvertNode` is not a `TempNode`**, so it is re-expanded at every use and
  never varred. That is why three's `bilinearCubeUV` emits the rotated
  direction twice — once for `getFace`, once for `getUV` — and why a
  *narrowing* cast is the swizzle arm (`x.xyz`), since
  `ConvertNode.generate()` is `builder.format( snippet, from, to )`. The port's
  `Node::Cast` arm was inverted to match; the sixteen-example baseline stayed
  byte-identical through the change.
* **An inlined `Fn`'s result is varred by `flowShaderNode`** when it is used
  more than once. The port applies its reuse rule to expression nodes but not
  to a `Block`, which re-generates at every use, so `ggx_convolution` asks for
  the var by hand. Without it the VNDF body is inlined once per component.

## `m07` is r186's new lighting model, and that is a different rung

The lit material's PMREM contribution — the 86-line `radiance` +
`iblIrradiance` block `EnvironmentNode` adds — matches `m07` statement for
statement under a `nodeVar`/`nodeUniform` renumbering. The rest of `m07` does
not, and should not be expected to: it is r186's restructured
`PhysicalLightingModel` (`multiScatteringCompensation`, `dfg`,
`NORMAL_normalView`), while the port still carries the earlier one, which is
what `webgpu_lights_physical` is gated against. The scout's `rung8` directory
has both spellings side by side. Porting the restructure is its own rung; it
would move row 4 of the ladder and has nothing to do with PMREM.

The seam is `PhysicalLightingModel::indirect_specular( has_environment, … )`:
with no environment it still declares `radiance` and `iblIrradiance` zero
itself, which is why every existing row is byte-identical. With one, the
declarations travel with `EnvironmentNode`, emitted between `indirectDiffuse()`
and `indirectSpecular()` — three's lighting-node ordering.

## `updateBefore` moved, on purpose

Three's `PMREMNode` carries `NodeUpdateType.RENDER` and builds the PMREM from
inside the node while the renderer is rendering. A node here is an immutable
`Rc` graph with no back-reference to the renderer, and `Renderer` methods take
`&mut self`, so the trigger moved out: the application calls
`PmremEnvironment::update( &mut renderer )`. It is idempotent, so "call it
before you render" is the whole rule, and the example calls it in both `init()`
and `animate()`. The `rung!` steady-upload assertion holds: after the first
frame it does nothing.

The atlas reaches the shader the same way three's does — one borrowed `Texture`
handle (`own_gpu == false`) repointed with `set_gpu()`, which is also how the
GGX material's `envMap` ping-pongs between the two passes of a step. Bind
groups are built per draw, so there is no cache to invalidate.

## Deferred

`fromScene` and `fromEquirectangular`, and with them the golden-angle Gaussian
blur shader and `BLUR_SAMPLES`. Nothing on the ladder reaches them;
`webgpu_pmrem_scene` and `webgpu_pmrem_equirectangular` would.
