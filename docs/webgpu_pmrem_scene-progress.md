# webgpu_pmrem_scene — done

2026-09-25: regraded against three.js 5f610f5 (the cube PMREM of 2f80402, #146): 0 of 100000 pixels (was 0); the face gate now reads PMREM cube layers.

**0 different pixels of 100000** against three's own `test/e2e/image.js` at
three's own 0.1% threshold, first run, and the twenty-three rows that were
already green are unchanged to the pixel.

This is `PMREMGenerator.fromScene`'s **other** arm. `webgpu_furnace_test`
landed the one where `useSolidColor` is true: the environment scene's
background is a `Color`, so it is lifted off the scene, becomes the
`BackgroundBox`'s colour, and is drawn once over the whole atlas — after which
the six face renders have an empty scene to draw and three's own dump shows six
viewport-restricted passes with **no draws in them**. Here the background is a
`CubeTexture` and the scene has six `MeshBasicMaterial` spheres in it, so
nothing is lifted, nothing is drawn up front, and the six face renders are the
whole of level 0. That is the first time on this ladder that the viewport
slicing, the `upSign`/`forwardSign` table and `autoClear = false` are
*observable* — and `assert_face_tiles` is where they are held.

The consumer side is the other half of the point: one
`MeshBasicNodeMaterial` whose `colorNode` is `pmremTexture( sceneRT.texture,
normalWorld, uniform( .5 ) )`, so the cube-UV read is the entire fragment
shader with no lighting model in front of it.

Gates, all green:

* `cargo test --release --test e2e webgpu_pmrem_scene` — `assert_face_tiles`
  (below) runs before the frame is looked at, then the image diff, then
  `steady_frame`.
* `examples/dump_wgsl.rs` gained `pmrem_scene_colornode`, diffed against
  `dump-pmrem_scene/m07` and `m08`.
* The full e2e ladder, all 30 tests, including `steady_frame_builds_nothing`
  for this example: frames two and three build nothing and upload nothing.
* `cargo test --release --workspace` — every unit and renderer gate,
  `tests/pmrem*.rs` included; the loader refactor below is why the whole
  suite was run rather than the ladder alone.
* `cargo clippy --workspace --all-targets -- -D warnings` clean, `cargo fmt`
  applied, `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps` clean.

Ladder after the change, in `tests/e2e/main.rs` order: compute_points 4 /
depth_texture 0 / furnace_test 0 / instance_mesh 60 / lights_phong 31 /
lights_physical 4 / lines_fat 0 / materials 44 / materials_basic 0 /
mesh_batch 0 / morphtargets 0 / pmrem_cubemap 0 / pmrem_test 27 /
postprocessing_bloom_selective 1 / postprocessing_difference 13 /
postprocessing_direct 21 / postprocessing_masking 18 /
postprocessing_radial_blur 7 / postprocessing_ssaa 0 / rtt 1 / shadowmap 7 /
skinning 6 / tsl_galaxy 40, **plus pmrem_scene 0**.

## Deltas against the scout material

`scouts/environment-family/ENVIRONMENT-FAMILY.md` §3 row 3 is where this rung
comes from. It had no `PLAN.md` and no dump; the dump was made first
(`scouts/environment-family/dump-pmrem_scene/`, 11 modules, 89 passes, 30
submits) and the example graded once under three's own grader to confirm the
0.0% the family table claims. It does: `Diff 0.0% in file: webgpu_pmrem_scene`.

The row's three predictions all held, and two of them were cheaper than
advertised:

| the row says | actually |
|---|---|
| "the `useSolidColor = false` branch … the background sphere mesh must work while rendering into the atlas" | it already did. `_sceneToCubeUV`'s else-arm was written with `fromScene` and had simply never been reached; it needed **no change at all**. Zero lines changed under `src/renderer/`. |
| "`pmremTexture( target.texture, normalWorld, uniform( 0.5 ) )` directly as a `colorNode`" | `PmremEnvironment::sample` + `MeshBasicNodeMaterial::color_node` already compose, and the generated WGSL matches `m08` statement for statement on the first try. No new node, no new material property, no new uniform. |
| "`CubeTextureLoader` is already ported" | ported, but **PNG-only**: `webgpu_materials_basic`'s faces are PNG and this page's are JPEG. One shared decoder later (below), it loads both. |

## What was added

| file | what |
|---|---|
| `examples/webgpu_pmrem_scene.rs` | the example. |
| `tests/e2e/main.rs` | the e2e entry, `assert_face_tiles`, and the `rung!` row. |
| `examples/dump_wgsl.rs` | `pmrem_scene_colornode`. |
| `src/loaders/texture_loader.rs` | `decode_image()` — the magic-number sniff that `TextureLoader::load` already did inline, lifted to `pub(crate)`. |
| `src/loaders/cube_texture_loader.rs` | its six faces go through `decode_image` instead of a private PNG-only decoder. |
| `docs/nodes.md` §13 | the second `fromScene` arm, the new module, and the face-tile gate. |
| `README.md`, `docs/gallery/` | the row and the thumbnail. |

Public API added: **none**. The example uses `PmremEnvironment::from_scene`,
`PmremEnvironment::sample`, `normal_world()`, `uniform_settable`,
`Scene::set_background` and `MeshBasicNodeMaterial::color_node`, all of which
already existed. `CubeTextureLoader::load` gained JPEG support without a
signature change.

## What the pixels found — and what the atlas found

The image passed on the first run, 0 of 100 000, which says nothing about the
generator that `webgpu_furnace_test` had not already said. `assert_face_tiles`
is the part that is new, and it is worth reading because it is the gate the
whole `fromScene` family has been missing.

**The mapping is derivable, and it is not the identity.** With `fov = 90`,
`aspect = 1` and a 256² viewport, the ray through tile pixel `( px, py )` is
the cube face at normalised `( ( px + .5 ) / 256, ( py + .5 ) / 256 )` — a
plain downsample from 1024², with no flip and no transpose. Working face 0 out
from three's own arithmetic:

```text
face_camera( 0 ) = ( up ( 0, 1, 0 ), lookAt ( 1, 0, 0 ) )   ⇒ d = +x
Object3D.lookAt:  z = -d = ( -1, 0, 0 )
                  x = normalize( cross( up, z ) ) = ( 0, 0, 1 )
                  y = cross( z, x ) = ( 0, 1, 0 )
ray( a, b ) = a·x + b·y + d = ( 1, b, a )
Background.material samples vec3( -dir.x, dir.yz ) (m02) ⇒ ( -1, b, a )
|x| is the major axis and negative ⇒ the -X face, i.e. nx.jpg, with the GL
table sc = +rz = a, tc = -ry = -b ⇒ s = ( a + 1 ) / 2, t = ( 1 - b ) / 2.
```

The six together give `nx, ny, pz, px, py, nz`. Two swaps are in there and
both mean something: `nx`/`px` and `pz`/`nz` trade because of the `-dir.x` of
three's cube convention, and tiles 1 and 4 trade because `forwardSign` points
the "+y" tile's camera at **−y** — which is exactly what the `normalWorld.y`
negation in `PMREMNode.setup` undoes on the way out. A test that recomputed
the port's own mapping would agree with itself; this one holds the six
literals and the derivation beside them.

**The scoring.** Each tile is reduced to a 4 × 4 grid of 64² block means in
linear light, and so is each of the six faces (through the *hardware* sRGB
EOTF, IEC 61966-2-1, not three's rational approximation of it — this side of
the comparison owes the port nothing). The twelve non-central blocks are
compared, which skips the sphere (angular radius `asin 0.2` = 11.5°, so 26 px
around the tile centre) and absorbs the few pixels of drift that come of the
background being a *tessellated* 32 × 32 sphere and not a full-screen blit.
All 6 images × 8 dihedral orientations are scored and the expected one, upright,
must win.

It wins by a lot. Measured, in mean absolute linear error per channel:

| tile | best | runner-up |
|---|---|---|
| 0 | `nx` upright, 0.0003 | `pz` flipX, 0.0116 |
| 1 | `ny` upright, 0.0001 | `ny` flipY, 0.0043 |
| 2 | `pz` upright, 0.0003 | `nx` flipX, 0.0116 |
| 3 | `px` upright, 0.0004 | `px` flipY, 0.0090 |
| 4 | `py` upright, 0.0002 | `py` flipX, 0.0068 |
| 5 | `nz` upright, 0.0002 | `px` rot180, 0.0135 |

22× to 43× clear, so the assertions are set at `best < 0.002` and
`runner_up > 5 × best` and still have four-fold headroom. A permuted face, a
flipped or rolled `up`, a transposed tile or a viewport written at the wrong
offset each fail it.

**And the six spheres pin the direction table on their own.** Each face's
90° frustum sees exactly one of the six `MeshBasicMaterial` spheres — the one
on its own axis; the others sit at 78.5° from the axis against a 54.7° frustum
corner, so they are outside it, which is also why three's dump draws exactly
one sphere per face pass. The middle of tile *i* therefore has to be the
colour of that sphere, and the six colours are distinct. Measured: exact, to
better than 0.01 in every channel, with the page's own hex constants as the
expected values.

Between them these two say the same thing twice, from the background and from
the geometry, and neither of them can be satisfied by an atlas that is right
"on average".

## The loader: one decoder, two loaders

`CubeTextureLoader` decoded PNG only, because the one cube example on the
ladder (`webgpu_materials_basic`) loads `textures/cube/pisa/*.png`. This page
loads `textures/cube/Park3Med/*.jpg`. In the browser neither loader picks a
decoder: `ImageLoader` hands the bytes to `createImageBitmap`, which sniffs
the type. `TextureLoader::load` already did that inline over three magic
numbers; it is now `texture_loader::decode_image()` and both loaders call it.

The side effect is that `CubeTextureLoader`'s PNG path changed decoder — from
its own `png::Decoder` with no transformations to `TextureLoader`'s, which
sets `EXPAND | STRIP_16` and handles palette and greyscale. For an 8-bit RGB
PNG (which pisa's faces are) the two are identical, and the ladder says so:
`webgpu_materials_basic` is unchanged at 0. It is listed here because it is
the one edit in this rung that could have moved another row.

## Two things in the example that are not optional

**The PMREM is generated before the seventh sphere is added.** The page calls
`fromScene( scene )` on the scene that already holds the cube background and
the six small spheres, and only *then* adds the `pmremTexture` sphere to the
same scene. Build the environment after, and the big sphere is in its own
environment; the frame would still look like a perfectly reasonable chrome
ball.

**The 45 is a plain vertical FoV**, as in `webgpu_furnace_test` and unlike
`webgpu_pmrem_test`: `PerspectiveCamera( 45, aspect, 0.25, 20 )` at
`( -1.8, 0.6, 2.7 )`. `OrbitControls` is constructed and `update()`d, but the
camera is 3.3 from the origin against `minDistance` 2 / `maxDistance` 10, so
the clamp is inert and `update()` reduces to `lookAt( origin )`.

**No tone mapping.** The page sets none.

## Divergences

**None new.** `pmrem_scene_colornode` against `m07`/`m08` differs only in the
classes `docs/nodes.md` §8 already lists — the header line, the order of the
two uniform struct declarations, the uniform *numbering* (three counts a
`mat4x4` as three slots, so its `nodeUniform6`/`7`/`9`/`11` are the port's
`4`/`5`/`7`/`8`), the port's single `v_modelViewProjection` where three hoists
a `VERTEX_nodeVar18` temp, and blank lines after a block. Every expression is
identical, `roughnessToMip` / `getFace` / `getUV` included, and so are both
`textureSampleGrad` calls with their explicit zero gradients.

The `fromScene` divergences listed in §13 are unchanged and all of them are
`webgpu_furnace_test`'s: the generator takes the renderer and the scene, a
scene-built `PmremEnvironment` has no source so `update()` is a no-op, and
`_setViewport` is `RenderTarget::set_viewport` in top-left pixels.

## What was ruled out or left out

* **`_blur` / `sphericalGaussianBlur`, and `BLUR_SAMPLES`.** Still no caller.
  `fromScene( scene )` defaults `sigma` to 0, so `_applyPMREM` takes the GGX
  arm here too. `webgpu_instance_path` is the first example that needs it
  (`ENVIRONMENT-FAMILY.md` §3 row 6).
* **Frustum culling as a pass-census claim.** Three draws exactly one sphere
  per face pass because it culls; whether the port issues one draw or six, the
  five outside the frustum rasterise nothing. The tile gate is over pixels,
  not over draw counts, so it does not depend on the port's culling matching
  three's — and the frame does not either.
* **`PMREMGenerator.dispose()` as a separate call.** `from_scene` cleans up
  its own scratch before it returns and hands back a live target the caller
  owns, as it already did.
* **The roughness slider.** One `gui.add( pmremRoughness, 'value', 0, 1 )`
  whose `onChange` re-renders. `clean-page.js` removes the panel, so the
  uniform stays at 0.5 — which is why it is a `uniform_settable` here and not
  a literal: the graph three builds has a uniform in it.
* **`renderer.inspector`.** Constructed on the page, removed by the harness.
