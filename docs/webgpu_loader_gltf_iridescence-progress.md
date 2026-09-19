# `webgpu_loader_gltf_iridescence`

Status: **not green — 1534 of 100000 pixels (1.5%), threshold 0.1%.** Intel
Iris Xe, Mesa 25.3.6, wgpu 30.0.1 on Vulkan. The example renders, the whole
iridescence half of `PhysicalLightingModel` is ported and the ladder is
untouched; the e2e test exists but is `#[ignore]`d and the example is not on
the `rung!()` list. What is left is written out under
"What is missing" below.

The Iridescence Lamp (`IridescenceLamp.glb`) under `venice_sunset_1k.hdr`,
which is both `scene.environment` (PMREM) and, unblurred, `scene.background`
(a cube converted from the equirect). Three glTF materials, three KHR
extensions, no lights.

## Reconciling with the plans

There is no scout `PLAN.md` for this rung. It was ported from
`~/src/vendor/three.js/examples/webgpu_loader_gltf_iridescence.html` directly,
with three's WGSL dumped by `tools/dump-webgpu.mjs` into `target/dumps/irid/`
(uncommitted, per the rules). `m12_fragment_fragment_IridescenceLampIridescence.wgsl`
is the module every statement below was read off.

It builds on what landed the same day: `webgpu_loader_gltf`'s
`scene.environment` fallback and equirect-to-cube background (§23), the sheen
rung's glTF extension plumbing (§25) and the anisotropy rung's transmission
pass and `materials::transmission` (§26). None of them needed changing.

## What was added

| area | what |
| --- | --- |
| `src/materials/physical.rs` | `evalIridescence`, `evalSensitivity`, `Fresnel0ToIor` / `IorToFresnel0`, `Schlick_to_F0`; the iridescence branch of `Physical::start()`; the iridescent F0 in `compute_multiscattering()`; the `USE_IRIDESCENCE` blend in `brdf_ggx()` |
| `src/materials/mod.rs` | `Material::{iridescence, iridescence_ior, iridescence_thickness_range, iridescence_thickness_map}` |
| `src/loaders/gltf_loader.rs` | `KHR_materials_iridescence` (`GltfIridescence`, `iridescence_texture`, `iridescence_thickness_texture`) |
| `src/nodes/tsl.rs` | `material_iridescence()`, `material_iridescence_ior()`, `material_iridescence_thickness_{max,min}()`; the `Iridescence` / `IridescenceIOR` / `IridescenceThickness` properties; `loop_range`'s `inclusive` argument |
| `src/nodes/node.rs`, `src/nodes/builder.rs` | `Node::Loop::inclusive` — `for ( … ; m <= 2 ; … )` |
| `src/renderer/programs.rs`, `src/renderer/mod.rs` | the four new material uniforms |
| `examples/` | `webgpu_loader_gltf_iridescence.rs` |

`docs/nodes.md` §30 is the long form.

## What the pixels found

The diff image is not on the lamp. 1534 differing pixels, and almost all of
them sit on the high-frequency edges of the skyline in the **background** —
the tree line at the left, the buildings at the right — as a one-pixel-wide
speckle along every contrast edge. The lamp body, the iridescent shade and the
glass bulb are inside the threshold apart from a thin rim on the shade's inner
lip, which is where the background is mirrored.

That points at the equirect-to-cube conversion for `scene.background`, not at
iridescence: this is the first graded page to take a Radiance `.hdr`
(`rgba16float`, `flipY = true`, no mip chain, `LinearFilter`) through
`cube_render_target::from_equirectangular_texture`, where every earlier page
fed it an UltraHDR JPEG. A wrong `evalIridescence` would colour the shade, and
the shade is right.

## Where the 1.5% is — measured, not guessed

Four measurements, in the order they were taken. `tools/dump-webgpu.mjs`
leaves three's own frame at `target/dumps/irid/actual_full.png`, which makes
all of them possible.

1. **The grader's reference is reproducible on this machine.** Three's own
   render, put through the same unmodified `test/e2e/image.js`, is **0 of
   100000 pixels** against `examples/screenshots/webgpu_loader_gltf_iridescence.jpg`.
   So the 1534 is the port's, not the reference's, and three's frame is a
   per-pixel oracle.

2. **The `.hdr` decode is bit-identical.** `HDRLoader.parse` over
   `venice_sunset_1k.hdr` under node, against `HdrLoader::parse`'s
   `to_bytes()`: 2097152 bytes, `cmp` clean. (`tests/hdr_loader.rs` only
   grades the six pisa faces, so this needed checking separately.) The source
   texture is right.

3. **The smooth sky is right; the detail is not.** Port frame against three's
   frame, per channel, 800×500:

   | region | mean abs. diff | mean signed (r, g, b) |
   | --- | --- | --- |
   | sky, `y` 0–80 | **0.16** | −0.01, 0.00, 0.00 |
   | skyline, `y` 150–250 | 4.81 | −0.12, −2.46, +1.34 |
   | lamp, `y` 100–400, `x` 300–500 | 3.79 | −3.74, −1.80, −0.12 |
   | ground, `y` 420–500 | 2.23 | −0.05, −0.04, −0.02 |

   A flat gradient drawn from the cube is reproduced to a sixth of a code
   value, so the skybox path, the cube's orientation and the mip level it is
   read at are all right. Everything that differs is high-frequency.

4. **It is not an integer pixel shift.** Rolling three's frame by ±1 px in
   each axis and comparing over a background-only crop makes the match worse
   in every direction but one, and that one (−1, 0) only marginally
   (6.39 vs 6.80) — the signature of a sub-pixel difference, not an offset.

5. **Forcing the source mip chain changes nothing.**
   `CubeRenderTarget.fromEquirectangularTexture()` sets
   `texture.generateMipmaps = true` on the source for the duration of the
   conversion (three's line 76), which is why three's dump shows the equirect
   with `mipLevelCount: 11` where the port's `HdrLoader` texture has 1.
   Porting that line — set on entry, restored on exit — moved the count by
   **0 pixels**, which stands to reason: the conversion box samples with
   `texture( texture, uvNode, 0 )`, an explicit level 0. The change was
   reverted rather than committed unverified against the full ladder.

### The three descriptor checks, against `dump.json` rather than against reasoning

6. **The sampler.** The conversion draw is pass 1 (viewport 512×512, into
   texture 6's layer). It binds `bindGroup_object` 64, whose sampler is 15.
   Three's dump prints sampler 15 as `magFilter/minFilter/mipmapFilter:
   nearest`, `clamp-to-edge` on all three axes — but so are samplers 85, 207
   and 371, which include the one bound for the lamp's 2048² base-colour PNG
   with its 12 mip levels. Four identical all-nearest descriptors with
   `lodMaxClamp: 32` and an empty label are read-back defaults, not what
   `WebGPUTextureUtils.createSampler()` passed; the two samplers the dump
   records faithfully are 17 and 18, which carry a lone `minFilter` and are
   the mipmap generator's. So the dump cannot answer this one. What can:
   `HDRLoader.js` lines 437–441 set `minFilter: LinearFilter, magFilter:
   LinearFilter`, and `hdr_loader.rs` sets the same pair. The port's texture
   is `rgba16float`, `is_unfilterable()` is false (it wants *both* filters
   `Nearest`), and it is bound `SamplerBindingType::Filtering` — so neither
   the `textureLoad` path nor the `Rgba32Float`/`FLOAT32_FILTERABLE` arm is
   reached. Both sides sample linear. **Not the cause.**

7. **The cube size.** Three's texture 6 is `512×512×6, rgba16float,
   mipLevelCount 1`. `from_equirectangular_texture()` takes `source.size().1`
   — 512 for a 1024×512 equirect — and `CubeTexture::render_target` gives it
   one level. **Identical.**

8. **`equirectUV`.** Three's m02 emits `atan2( d.z, d.x ) * 0.15915494309189535
   + 0.5` and `asin( clamp( d.y, -1, 1 ) ) * 0.3183098861837907 + 0.5`, read
   with `textureSampleLevel( …, 0.0 )` at the fragment's own
   `positionWorldDirection`, no half-texel anywhere. `tsl::equirect_uv()` is
   that expression constant-for-constant, and `cube_render_target` wraps it in
   `texture_level( source, …, 0.0 )`. **Identical.** (Same expression again in
   m06, the PMREM equirect pass.)

   One incidental thing the dump shows: three's passes 4–11 *clear* the
   equirect's mip levels 2–10 and draw nothing into them. Only level 1 is
   ever generated. Since every read is an explicit level 0, it changes
   nothing — but it is worth knowing before anyone tries to match that mip
   chain.

9. **The stage split and the box.** Three's m01 puts
   `varyings.v_positionWorldDirection = normalize( ( modelWorldMatrix *
   vec4( positionLocal, 0.0 ) ).xyz )` in the vertex stage and m02 opens with
   `positionWorldDirection = normalize( v_positionWorldDirection )`, with
   `equirectUV` and the sample both in the fragment stage. `position_world_direction`
   in `tsl.rs` is `to_var( to_varying( … .normalize() ).normalize() )` — the
   same double normalize across the same varying — and `cube_render_target`
   wraps `equirect_uv()` around the *var*, so the uv is per fragment on both
   sides. Nothing is hoisted. The box is `box_geometry( 5, 5, 5, 1, 1, 1 )`
   with `Side::Back`, and the face camera is `PerspectiveCamera::new( -90.0,
   1.0, 1.0, 10.0 )` — `CubeCamera( 1, 10 )`'s shape, aspect 1, the negative
   fov intact. **Nothing here diverges.**

10. **Doubling the cube face makes it slightly worse.** Changing
   `from_equirectangular_texture()`'s `let size = source.size().1` to
   `* 2` — a 1024² cube from the same 1024×512 equirect, four times the texels
   — moves the count from **1534 to 1603**. A sub-pixel sampling error in the
   conversion would have fallen away at twice the resolution; instead it is
   flat (and marginally the wrong way). **This exonerates the conversion**,
   and with it measurement 5's mip experiment, checks 6–9 and the coverage
   argument below: the equirect path is not where the pixels are. Reverted.

### What the rest of the ladder already proves about this path

`webgpu_postprocessing_bloom_emissive` is green and is *this* configuration:
`HdrLoader` on a 1k `.hdr`, a 512² cube for `scene.background`, a PMREM for
`scene.environment`. `webgpu_loader_gltf_sheen` is green with the same code at
1024² faces from a 2048×1024 UltraHDR. So the conversion is proven at 512
faces and proven on detailed input — just never both at once: `moonless_golf`
is a near-featureless night sky, which is exactly the input that would hide a
high-frequency error, and every detailed page so far had twice the face
resolution. `venice_sunset` at 512 is the first case that has both, which is
why this is the first page to show it.

The conclusion the next session should start from: **the equirect path is
not the cause.** Measurement 10 settles what 5–9 each only narrowed. What
remains true is the shape of the error — smooth regions identical to 0.16 of
a code value, every high-contrast region off by 2–5 — and that shape is now
unexplained by anything in the background pipeline. It is the signature of an
edge-only difference: the skyline and the lamp are both edge-dense, the sky and
the ground are not, and the grader's 800×500 → 400×250 downsample spreads an
edge difference over its neighbours. `antialias: true` and the 4× MSAA resolve
are the same on both sides and on the green siblings, so it is not the sample
count itself; the next thing to measure is whether the *unresolved* samples
agree, and failing that whether the lamp's own geometry (the glTF normals and
tangents, which feed an iridescent specular lobe that is far more sensitive to
them than a plain one) is what both regions have in common — the skyline is
seen *through* and reflected *in* the lamp's glass in much of the frame.

## What is missing

1. **The equirect→cube conversion's sub-pixel sampling**, as measured above.
   Three's own `m04_fragment_fragment_Background.material.wgsl` is a plain
   `textureSampleLevel( …, render.nodeUniform5 )` cube read, so the shader is
   not where the difference is — the cube's contents are.
2. **A `dump_wgsl.rs` section.** The generated WGSL was read against
   `target/dumps/irid/m12_*.wgsl` by hand while writing
   `src/materials/physical.rs`, but the three
   `loader_gltf_iridescence_{lamp,iridescence,transmission}` sections that
   would keep it diffed are not written.
3. **The README "Examples graded green" row**, which cannot be written until
   the number is under the threshold.

## What was ruled out

* **`iridescenceTexture`.** The extension's `iridescenceFactor` map is parsed
  into `GltfMaterial::iridescence_texture` and then deliberately dropped:
  `MaterialNode.IRIDESCENCE` has no map branch in r186, so three itself never
  reads `material.iridescenceMap`. The asset carries no such texture anyway.
* **`BRDF_GGX`'s `USE_IRIDESCENCE` branch.** Written, never reached: the page
  has no lights, so `PhysicalLightingModel.direct()` is never called and
  `this.iridescenceFresnel` is not in three's dump for this page either.
* **Dispersion and retroreflection**, the two other `PhysicalLightingModel`
  flags: neither is on for any material here.
