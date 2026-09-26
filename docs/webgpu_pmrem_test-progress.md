# webgpu_pmrem_test — done

2026-09-25: regraded against three.js 5f610f5 (the cube PMREM of 2f80402, #146): 0 of 100000 pixels (was 27).

**27 different pixels of 100000** against three's own `test/e2e/image.js` at
three's own 0.1% threshold, and the eighteen rows that were already green are
unchanged to the pixel.

This is `PMREMGenerator.fromEquirectangular` on top of the `fromCubemap` that
`webgpu_pmrem_cubemap` landed. The scout's estimate was 250–350 lines of Rust,
of which ~90 the example; the delta came in under that, because two of the
seven steps in the plan's order of work — `HdrLoader::load()` and the `flipY`
upload — had already landed with PMREM part A and were verified rather than
written.

Gates, all green:

* `cargo test --test hdr_loader` — 6 tests, bit-for-bit against three's own
  `HDRLoader.parse`, now including `spot1Lux.hdr` and the `DataTextureLoader`
  property copy (no GPU).
* `cargo test --release --test pmrem_equirect` — the flip gate and the atlas
  gate (GPU; see below).
* `cargo test -p three-rs --test pmrem` — the GGX ladder, the atlas rectangles
  and `_generateCubeUVSize`, unchanged.
* `examples/dump_wgsl.rs` gained `pmrem_equirect`, `pmrem_test_background` and
  `pmrem_test_physical`, diffed against `dump-pmrem_test/m02`, `m06` and `m08`.
* The full e2e ladder, all 25 tests, including `steady_frame_builds_nothing`
  for this example: frames two and three build nothing and upload nothing.
* `cargo clippy --workspace --all-targets -- -D warnings` clean, `cargo fmt`
  applied, `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps` clean.

Ladder after the change, in `tests/e2e/main.rs` order: depth_texture 0 /
instance_mesh 60 / materials_basic 0 / rtt 1 / lights_phong 31 /
morphtargets 0 / shadowmap 7 / lights_physical 4 / postprocessing_masking 18 /
tsl_galaxy 40 / skinning 6 / mesh_batch 0 / compute_points 4 / radial_blur 7 /
materials 44 / ssaa 0 / lines_fat 0 / pmrem_cubemap 0, **plus pmrem_test 27**.

## Deltas against the scout plan

The plan was written on 2026-09-13, before 0.2.0 and before PMREM parts A
and B landed. Three of its fourteen gap-list rows had moved by the time this
sitting started:

| plan says | actually |
|---|---|
| `HDRLoader.load()` **missing**, ~50 Rust | **present**, landed with PMREM A: `HdrLoader::load()` reads the file, decodes through `parse()`, and applies `texData`'s four properties including `flipY = true`. Verified by a new test rather than rewritten. |
| `flipY` on a half-float upload **missing**, ~25 Rust | **present**: `upload_texture_2d` already reversed the rows when `texture.flip_y` is set — option (1) of plan §5.1, the CPU row reversal. What was missing was the *gate*, which this rung adds. |
| `scene.background` accepting a cube-UV texture, ~40 Rust | the port already had `Background::Node`, which `webgpu_pmrem_cubemap` uses for `scene.backgroundNode`. `scene.background = <texture>` is a different upstream path (`NodeManager.getBackgroundNode` plus the node context), so it became its own variant rather than being spelled out in the example. |
| `envMapIntensity` "may be new" | already there — `material_env_intensity()`, which `EnvironmentNode` was already multiplying both taps by. |

The plan's §4 also lists `equirectUV`, `atan2`/`asin`, `texture( map, uv, level )`,
`_getEquirectMaterial`, `fromEquirectangular` and the `_fromTexture` non-cube
branch as missing. All six were, and all six are now here.

## What was added

| file | what |
|---|---|
| `src/nodes/tsl.rs` | `NodeRef::asin` and `NodeRef::atan2` (`MathNode.ASIN` / two-argument `ATAN`), `texture_level( map, uv, level )` for a 2-D tap at an explicit mip, and `equirect_uv( direction )` — the port of `nodes/utils/EquirectUV.js`. |
| `src/renderer/pmrem.rs` | `PmremSource`, `_setSizeFromTexture` as a method on it, `_fromTexture` as `from_texture`, `from_equirectangular`, and `_getEquirectMaterial` as `equirect_material`. `from_cubemap` is now a two-line wrapper. |
| `src/nodes/pmrem_node.rs` | `PmremEnvironment` holds a `PmremSource` instead of a `CubeTexture`; `PmremEnvironment::from_equirectangular`. |
| `src/objects/scene.rs` | `Background::Pmrem( PmremHandle )` — `scene.background = <a generated PMREM's texture>`. |
| `src/materials/node_material.rs` | `background_pmrem_color_node`, which is `Background.update()`'s node branch with the context's two accessors passed as arguments. |
| `src/materials/environment.rs` | `PmremHandle::sample` is public, because the background builds its own tap rather than going through a material. |
| `examples/webgpu_pmrem_test.rs` | the example. |
| `tests/pmrem_equirect.rs` | the two GPU gates. |
| `tests/hdr/gen.mjs`, `tests/hdr/oracle.json`, `tests/hdr_loader.rs` | the `spot1Lux.hdr` oracle and its test. The oracle grew by 25 lines and changed nothing that was already in it. |
| `examples/dump_wgsl.rs` | three new sections. |

Public API added: `Background::Pmrem`, `PmremSource`,
`PmremGenerator::{from_equirectangular, from_texture}`, `equirect_material`,
`PmremEnvironment::from_equirectangular`, `PmremHandle::sample`,
`materials::background_pmrem_color_node`, `tsl::{equirect_uv, texture_level}`,
`NodeRef::{asin, atan2}`.

## The one shader, and it is four lines

The whole difference between `webgpu_pmrem_cubemap`'s dump and this one is a
single fragment module. Three's `m02_fragment_fragment_PMREM_equirect.wgsl`:

```wgsl
nodeVar0 = normalize( nodeVarying4 );
nodeVar1 = textureSampleLevel( nodeUniform0, nodeUniform0_sampler,
	vec2<f32>( ( ( atan2( nodeVar0.z, nodeVar0.x ) * 0.15915494309189535 ) + 0.5 ),
	           ( ( asin( clamp( nodeVar0.y, -1.0, 1.0 ) ) * 0.3183098861837907 ) + 0.5 ) ),
	0.0 );
output.color = nodeVar1;
```

The port's `pmrem_equirect` is that, character for character, under the varying
renumbering §8 of `docs/nodes.md` already lists. Everything else — the cubeUV
atlas, the twenty-pass GGX chain, `textureCubeUV`, `PMREMNode`,
`EnvironmentNode`, `envMap` on the physical material, ACESFilmic — is shared
verbatim with the cubemap rung and needed no work at all.

Two things about those four lines are easy to get wrong and invisible
afterwards:

* **no environment rotation.** The cubemap material carries
  `materialEnvRotation`, because `CubeTextureNode.setupUV()` applies it inside
  the node. A plain 2-D `texture()` node does not, so `_getEquirectMaterial`
  has none, and neither does this. Three's dump has exactly one uniform in the
  fragment module — the map — which is how you can tell from the outside.
* **the explicit level 0.** `texture( envTexture, equirectUV( … ), 0 )`, not a
  derivative sample. With `generateMipmaps = false` on the decoded HDR the two
  agree numerically here, but the six quads of a lod plane are a wildly
  non-uniform parameterisation of the sphere, so a derivative sample would pick
  a different level per face the moment the source had a mip chain.

`pmrem_test_background` matches `m06` statement for statement.
`pmrem_test_physical` matches `m08` only in the part this family owns — the
`radiance` + `iblIrradiance` block and the directional-light block; the rest of
`m08` is r186's restructured `PhysicalLightingModel`, which
`docs/nodes.md` §13 explains is a rung of its own.

## What the pixels found — nothing, and that is the point

The image passed on the first run at 27/100000. The two gates under it are
what actually tested the rung, and both are `spot1Lux.hdr`: a 1024×512 black
image with **one** bright texel at (597, 213), 27 490 nits. One lit texel in a
black field is the perfect oracle, because every way of getting this wrong
moves it somewhere provably wrong, where an environment map with structure in
it would just look like a different plausible sky.

**The flip gate.** `HDRLoader` sets `texData.flipY = true`, so the texel must
be sampled from row `512 - 1 - 213 = 298`. `tests/pmrem_equirect.rs` uploads
the texture as the loader builds it, renders it 1:1 with nearest filtering into
an `rgba16float` target, reads it back and asserts that **exactly one** texel
is lit, that it is at (597, 298), and that it is grey at the half three's own
decoder produces. A missing flip, an off-by-one flip and a row-stride bug each
fail it differently. None of them would fail "does it render".

**The atlas gate.** After `fromEquirectangular`, the 768×1024 atlas has exactly
one of its six mip-0 face tiles lit and the other five black — one delta
direction hits one face — and all eleven LOD tiles are lit, which is the whole
prefilter ladder having run through the new entry point. A permuted `FACE_LIB`,
a smeared blit or a viewport landing in the wrong tile all break the first;
a GGX chain that stopped after the first step breaks the second.

Neither gate carries a number derived from a reference image: the oracle is
three's own `HDRLoader.parse` run over a file in the vendor tree, recorded by
`tests/hdr/gen.mjs`.

## Divergences, both recorded in `docs/nodes.md` §13

* **`flipY` is a CPU row reversal, not two render passes.** Three's WebGPU
  backend honours `flipY` on a buffer-sourced texture with
  `WebGPUTextureUtils._flipY()`, which borrows the mipmap blit pipeline to
  bounce the source through a scratch texture and back: two extra render
  passes and two extra submits, which is why three's dump of this example has
  25 render passes where the port has 23 (1 `_textureToCubeUV` + 20 GGX +
  1 scene + 1 output). A flip is an exact texel permutation and the blit
  samples texel centres of an equally sized target, so the results are
  bit-identical; the flip is gated on the flag, so every `flip_y == false`
  texture on the ladder is byte-for-byte what it was — which the eighteen
  unchanged rows say independently.
* **`scene.background = <texture>` is a `Background::Pmrem` variant.**
  Upstream the background is a `Texture` whose `mapping` is
  `CubeUVReflectionMapping`; `NodeManager.getBackgroundNode()` turns it into
  `pmremTexture( background )` and `Background.update()` wraps that in a node
  context supplying `getUV` (`backgroundRotation.mul( normalWorldGeometry )`)
  and `getTextureLevel` (`backgroundBlurriness`). The port has neither a node
  context nor a mapping constant, so the variant carries the `PmremHandle` —
  the three cubeUV uniforms travel on it — and the renderer builds the same
  graph with the two accessors passed as arguments. The example then writes the
  one line the page writes.

## Two things in the example that are not optional

**The camera's 40 is a *horizontal* FoV.** `updateCamera()` converts it before
the first frame: at 800×500 the stored vertical fov is
`2 * atan( tan( 20° ) / 1.6 ) = 25.5115…°`. The example computes it the page's
way, degrees → radians → degrees, rather than baking a rounded constant. Get it
wrong and every sphere is the wrong size, and only the image says so.

**The `DirectionalLight` has intensity 0.** It contributes nothing to the frame
— the page's GUI toggle, which `clean-page.js` removes, is what would give it
an intensity — but it is in the light list, so the physical fragment shader
carries the directional block and the render uniform buffer has its cells. A
port that dropped zero-intensity lights would render a byte-identical image and
a different shader. That is why `dump_wgsl` has `pmrem_test_physical` with the
light in its `SetupContext`: the assertion is on the WGSL, not the pixels.

## What was ruled out or left for later

* **`fromScene`**, and with it `webgpu_furnace_test`. Plan §6.8 hangs it off
  the end of this rung as a stretch: a solid-colour env scene, a `BackgroundBox`
  and six cube-camera renders into viewport tiles with `autoClear` off. It is
  ~120 Rust lines and needs one renderer capability the port does not have —
  render a *scene* into a viewport-restricted slice of a render target without
  clearing it — so it is a rung, not a tail. Nothing here blocks it: the atlas,
  the viewport and the GGX chain it would write into are all in place and
  gated.
* **`_flipY` as two render passes.** Option (2) of plan §5.1, ~60 lines and a
  pipeline the port does not otherwise need, for a bit-identical result. See
  the divergence above.
* **`compileEquirectangularShader()`.** A warm-up that builds the material
  before the file arrives. The port builds the material inside
  `_textureToCubeUV`, from the texture it is handed, so there is nothing to
  pre-compile and nothing observable to reproduce.
* **`FloatType` equirect sources.** `HdrLoader::set_data_type( Float )` decodes
  and `Texture::data_rgba32float` exists, but nothing on the ladder asks for
  it; the example's commented-out `.setDataType( THREE.FloatType )` stays
  commented out here too.
* **`m08` as a whole.** r186's restructured `PhysicalLightingModel` is a rung
  of its own and would move the `webgpu_lights_physical` row.
