# webgpu_materials_envmaps and webgpu_materials_cubemap_mipmaps — done

**Status: both green.** One sitting, two graded examples, no node-system work.

| example | different pixels (of 100000) |
|---|---|
| `webgpu_materials_envmaps` | **0** |
| `webgpu_materials_cubemap_mipmaps` | **1** |

The full ladder is 25 rows and every earlier number is exactly where the
branch found it: depth_texture 0 / instance_mesh 60 / materials_basic 0 /
rtt 1 / lights_phong 31 / morphtargets 0 / shadowmap 7 / lights_physical 4 /
postprocessing_masking 18 / tsl_galaxy 40 / skinning 6 / mesh_batch 0 /
compute_points 4 / radial_blur 7 / materials 44 / ssaa 0 / pmrem_cubemap 0 /
bloom_selective 1 / lines_fat 0 / pmrem_test 27 / postprocessing_difference 13 /
postprocessing_direct 21 / furnace_test 0, plus the two new rows.

Gates: `cargo fmt --check`, `cargo clippy --release --all-targets -D warnings`,
`RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps`,
`cargo test -p three-rs --lib` (58), and the e2e ladder under the GPU lock —
all clean.

## The correction the scout's note needs

`scouts/environment-family/ENVIRONMENT-FAMILY.md` lists these two as adding
"refraction mapping (`CubeRefractionMapping`, `refractionRatio`) and the
UI-toggled reflection/refraction on envmaps". **That is not in the graded
frame.** The e2e frame is the page's *first* frame, and the page's GUI
defaults are `Type: 'Cube'` and `Refraction: false`; both settings only take
effect through an `onChange` handler that the harness never fires. So the
graded image is the plain `CubeReflectionMapping` path, the equirectangular
texture the page also loads is never sampled, and `refractionRatio` never
appears. Nothing refractive or equirectangular was ported — see
`docs/nodes.md` §17 for what it would take.

The rest of the scout's read was right: both are asset-light LDR cube maps on
the non-PMREM `CubeMapNode` path that `webgpu_materials_basic` already proves,
and `cubemap_mipmaps` is about the explicit mip chain.

## What the WGSL diff found: nothing

Three's dumps (`tools/dump-webgpu.mjs`, into a scratch dir, not committed) give
seven modules for `webgpu_materials_envmaps` and five for
`webgpu_materials_cubemap_mipmaps`. The sphere modules are identical across the
two examples — `diff` reports no lines — and both match what
`examples/dump_wgsl.rs` has printed as `basic_envmap` since rung 3, modulo the
§8 divergences the whole ladder shares (`renderStruct` field order, uniform
renumbering, and the port's inlining of single-use `nodeVar`s). The background
module of `webgpu_materials_envmaps` matches `background_cube` the same way.

So `dump_wgsl.rs` gained a **comment** on those two sections naming the dumps
they now also stand for, rather than two more sections printing the same bytes
under different labels.

That leaves the whole of both rungs on the **upload** side, which is where the
bugs were.

## What was added

| area | what |
|---|---|
| `src/loaders/cube_texture_loader.rs` | Rewritten around a magic-number sniff, the way `TextureLoader` already does it and the way `ImageLoader` leaves the choice to the browser. Every cube on the ladder so far has been PNG; `textures/cube/Bridge2/` and `textures/cube/angus/` are JPEG. Also gained `set_path()` (`Loader.path`, joined per url — an absolute url still wins, so the existing callers are untouched) and `load_images()`, the six decoded faces without a `CubeTexture` around them. |
| `src/loaders/texture_loader.rs` | `decode_png` / `decode_jpeg_bytes` are `pub(crate)` so the cube loader shares them rather than keeping a second, PNG-only decoder. |
| `src/textures/cube_texture.rs` | `CubeTextureInner::mipmaps: Vec<Vec<Image>>` (level 1 first, six faces each) with `set_mipmaps()`; `clone_texture()`, three.js' `Texture.clone()` — a new id over the same decoded faces, as against `Clone`, which is object identity. `mip_level_count()` now follows `Textures.getMipLevels()` *and* the `isCubeTexture` correction beside its call site. |
| `src/renderer/mod.rs` | `ensure_cube_texture()` writes the hand-supplied levels (`_copyCubeMapToTexture()`: face by face, `mipmaps[ j ].images[ face ]` to `mipLevel = j + 1`) and generates a chain only when the texture supplied none. New file-private `write_face()` helper, since the copy is now two calls. |
| `examples/` | `webgpu_materials_envmaps.rs`, `webgpu_materials_cubemap_mipmaps.rs`, with `[[example]]` entries and `tests/e2e/main.rs` tests + `rung!` rows. |
| `docs/nodes.md` | New §17: why these two rungs add no divergence class, what is *not* ported from the page's GUI, and the mip-count quirk below. |

## What the pixels found

**The mip count, and only the mip count.** `webgpu_materials_cubemap_mipmaps`'
cube is 256² with eight hand-authored mips, and three.js' own texture
descriptor says `mipLevelCount: 9`. Read `getMipLevels()` alone and you get 8,
because it returns `texture.mipmaps.length` for every texture — correct for a
2D texture, whose `mipmaps` array holds level 0 too, and one short for an
uncompressed cube, whose `mipmaps` array holds the mips only. three.js does not
fix the function; it corrects the result at the call site, under a `TODO`:

```js
if ( texture.isCubeTexture && texture.mipmaps.length > 0 ) options.levels ++;
```

Ported with the same shape. Eight levels would have left the sampler's LOD
clamp a level short — visible on the far, grazing side of the right-hand
sphere, which is exactly the part of the image the page exists to show.

The sibling trap in the same code is `needsMipmaps()`:
`generateMipmaps === true || mipmaps.length > 0`. The page sets
`generateMipmaps = false`, and it would be natural to take that as "one level".
It is not; it only suppresses *generation*, and generation is suppressed by a
separate `texture.mipmaps.length === 0` guard rather than by the flag.

**The single different pixel** in `cubemap_mipmaps` is the JPEG decoder.
`zune-jpeg` and Chromium's libjpeg-turbo differ by the rounding of the inverse
DCT (already noted on `TextureLoader`), and this rung feeds 54 JPEGs straight
into a mip chain. One pixel of 100000 against a 0.1% threshold; nothing was
tuned for it.

## What was ruled out

* **Refraction and equirect.** See the correction above: out of the graded
  frame, so out of the tree. `Mapping::CubeRefraction` stays an unread
  constant.
* **A `CubeTexture` per mip level.** The page builds one whole `CubeTexture`
  per level and then reads only `images` off each. The port loads the faces
  (`load_images()`), because a `CubeTexture` that never reaches the GPU would
  be a texture id, a sampler policy and a colour space with nothing to apply
  them to.
* **Two sections in `dump_wgsl.rs`.** They would print bytes already printed.
  A comment naming the dumps is the honest version.
* **The viewer.** Keys `1`..`0` and the letters are full; the steady-frame
  numbers below come from the e2e harness, as `radial_blur` and `ssaa` did.

## What was left out

* `EquirectangularReflectionMapping` / `EquirectangularRefractionMapping`, and
  a `Texture` (rather than a `CubeTexture`) in `material.env_map`. Needs
  `EquirectUVNode`.
* `CubeRefractionMapping` and `material.refractionRatio` — `setupEnvironment()`
  choosing `refractVector()` over `reflectVector()`.
* `scene.backgroundRotation` and `material.envMapRotation` as *live* values.
  The uniform is in the shader (`materialEnvRotation`) and is the identity
  here; the page's three rotation toggles and `syncMaterial` default off.
* `CubeTexture` mip levels of a non-`UnsignedByte` type. `set_mipmaps()` takes
  whatever `Image`s it is given and the stride comes from the format, so a
  half-float chain should work, but nothing on the ladder uploads one and it is
  untested.
* An assertion that the two spheres agree with each other. The page is a
  comparison and a broken upload path on one side would show as a disagreement
  before it shows as a pixel count, but the grader already catches it at 1
  pixel, so no second gate was added.

## The numbers

| Three example | different pixels (of 100000) | steady frame (ms) | draw calls | triangles |
|---|---|---|---|---|
| webgpu_materials_envmaps | 0 | 2.1 | 3 | 7105 |
| webgpu_materials_cubemap_mipmaps | 1 | 2.6 | 3 | 65025 |

Three draw calls each: for `envmaps` the background, the sphere and the output
pass; for `cubemap_mipmaps` the two spheres and the output pass. Frames two and
three of both build, compile and upload nothing.
