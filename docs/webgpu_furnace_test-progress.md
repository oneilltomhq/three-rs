# webgpu_furnace_test — done

2026-09-25: regraded against three.js 5f610f5 (the cube PMREM of 2f80402, #146): 0 of 100000 pixels (was 0).

**0 different pixels of 100000** against three's own `test/e2e/image.js` at
three's own 0.1% threshold, and the nineteen rows that were already green are
unchanged to the pixel.

This is `PMREMGenerator.fromScene`, in the one form the ladder reaches: an
environment scene that is `new Scene()` with `background = new Color(
0xcccccc )` and nothing else. On the consumer side it is an 11×11
`MeshPhysicalMaterial` roughness × metalness grid under that environment,
which is the strongest numeric gate in the family — with a *constant*
environment an energy-conserving BSDF has to return that constant for every
one of the 121 pairs, so three's own frame is uniformly `0xcccccc`, all
400 000 pixels, and so is the port's.

Gates, all green:

* `cargo test --release --test pmrem_scene` — the six cube faces with no GPU,
  and the atlas readback (see below).
* `cargo test -p three-rs --test pmrem` and `--test pmrem_equirect` —
  the GGX ladder, the atlas rectangles, `_generateCubeUVSize`, the `flipY`
  gate and the equirect atlas gate, all unchanged.
* `tests/e2e/main.rs` asserts the white-furnace identity on the frame
  *before* it runs the image diff: every channel of every pixel within 1 of
  `Color::from_hex( 0xcccccc )` in sRGB-8. Observed range: `204..=204`.
* `examples/dump_wgsl.rs` gained `furnace_background` and `furnace_physical`,
  diffed against `dump-furnace_test/m01` and `m05`.
* The full e2e ladder, all 26 tests, including `steady_frame_builds_nothing`
  for this example: frames two and three build nothing and upload nothing.
* `cargo clippy --workspace --all-targets -- -D warnings` clean, `cargo fmt`
  applied, `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps` clean.

Ladder after the change, in `tests/e2e/main.rs` order: depth_texture 0 /
instance_mesh 60 / materials_basic 0 / rtt 1 / lights_phong 31 /
morphtargets 0 / shadowmap 7 / lights_physical 4 / postprocessing_masking 18 /
tsl_galaxy 40 / skinning 6 / mesh_batch 0 / compute_points 4 / radial_blur 7 /
materials 44 / ssaa 0 / lines_fat 0 / pmrem_cubemap 0 / pmrem_test 27,
**plus furnace_test 0**.

## Deltas against the scout plan

`scouts/environment-family/PLAN.md` §6 step 8 is where this rung comes from —
it hangs `fromScene` off the end of `webgpu_pmrem_test` as a stretch. Two of
its three estimates moved:

| plan says | actually |
|---|---|
| "it needs only one renderer capability the port does not have: *render a scene into a viewport-restricted slice of a render target without clearing it*" | the port **already has it**, from two rungs that arrived independently. `Renderer::render()` builds its pass through `render_target_pass()`, which takes `viewport: Rect::of( inner.viewport, 1.0, inner.width, inner.height )` from the bound target — the seam `webgpu_lines_fat` documents — and the clear is `ClearOps` from `auto_clear` / `auto_clear_color` / `auto_clear_depth`. So `_sceneToCubeUV` is a pure addition inside `src/renderer/pmrem.rs`: **zero lines changed under `src/renderer/` outside that file, and none in the backend.** |
| `_sceneToCubeUV` is "~100 JS lines, maybe 120 Rust" | 120 is about right for the code; the file grew 231 lines with its docs and the two pure helpers the gate needs. |
| "the rest of `webgpu_furnace_test` is … over the same IBL path this rung already proves" | true, and stronger than expected: the consumer side needed **no new node, no new material property and no new uniform**. `MeshPhysicalNodeMaterial::physical` + `pmrem_env` is the whole grid. |

The plan (and `docs/webgpu_pmrem_test-progress.md`) also says the golden-angle
Gaussian blur would come with `fromScene`. It does not: `_applyPMREM` takes
the `sigma == 0` GGX arm for a scene source exactly as it does for a texture
source, so `_blur` / `sphericalGaussianBlur` still has no caller and is still
not ported.

## What was added

| file | what |
|---|---|
| `src/renderer/pmrem.rs` | `from_scene`, `scene_to_cube_uv`, `background_box`, `background_material`, `face_camera`, `face_tile`, `SCENE_SIZE` / `SCENE_NEAR` / `SCENE_FAR`, and `cleanup` extracted out of `from_texture` so both entry points share it. |
| `src/nodes/pmrem_node.rs` | `PmremEnvironment::from_scene`; the source became `Option<PmremSource>` (`None` = built from a scene) and `update()` early-returns for it; the `updateFromTexture` tail became `adopt()`. |
| `examples/webgpu_furnace_test.rs` | the example. |
| `tests/pmrem_scene.rs` | the two gates. |
| `tests/e2e/main.rs` | the e2e entry, `assert_white_furnace`, and the `rung!` row. |
| `examples/dump_wgsl.rs` | two new sections. |

Public API added:

* `PmremGenerator::from_scene( &mut renderer, &mut scene, Option<RenderTarget> )`
* `pmrem::background_material( Color ) -> MeshBasicNodeMaterial` — the
  `MeshBasicMaterial({ name: 'PMREM.Background', side: BackSide, depthWrite:
  false, depthTest: false })` three builds inline, named so `dump_wgsl` can
  diff it
* `pmrem::face_camera( face, position ) -> ( up, look_at )` and
  `pmrem::face_tile( cube_size, face ) -> ( x, y, w, h )`
* `pmrem::{ SCENE_SIZE, SCENE_NEAR, SCENE_FAR }`
* `PmremEnvironment::from_scene( &mut renderer, &mut scene )`

## What the pixels found — and what they could not

The image passed on the first run at 0/100000, and the white-furnace assertion
passed with it. That is the point of the example and it is also its weakness as
a test of the *generator*: the environment is one colour, so a permuted face, a
flipped `up` or a viewport landing in the wrong tile would all produce the same
uniform atlas and the same perfect frame. `tests/pmrem_scene.rs` is where the
generator is actually tested.

**The six faces, with no GPU.** `face_camera` and `face_tile` are pure
functions, pulled out of the render loop for exactly this reason, and the test
holds three's `upSign = [1,1,1,1,-1,1]`, `forwardSign = [1,-1,1,-1,1,-1]` and
`_setViewport( col·size, i>2 ? size : 0, size, size )` as literals rather than
recomputing them — so a drift fails instead of agreeing with itself.

**The atlas is the furnace.** `fromScene` over the example's own environment
scene, then a `rgba16float` readback of the 768×1024 atlas: every texel of
every one of the eleven LOD tiles within **1%** of
`Color::from_hex( 0xcccccc ).r`. The tolerance is justified by the storage and
not by what was measured — half precision near 0.6 is 2⁻¹¹ ≈ 0.049%, and the
ladder re-quantises through it once per GGX step, so the error can only walk up
by about that much per level. It does exactly that, 0.0516% at LOD 0 rising to
0.3751% at LOD 10, one ulp at a time, which is the signature of rounding rather
than of a lost energy term, and leaves the limit about 2.7× clear. Every
level's worst case is printed.

This is the white-furnace identity one level *below* the one the image tests: a
normalised GGX kernel over a constant field is that constant, with no BSDF in
the way to blame if it is not.

**What the example does not exercise.** For *this* scene the viewport slicing
is set up but not observable. The WebGPU `_sceneToCubeUV` lifts a solid-colour
background off the scene, renders the `BackgroundBox` **once at the target's
full viewport** — a unit cube seen from its own centre through a 90° frustum
fills any viewport — and then runs the six face renders over what is left,
which is an empty scene. Three's own dump says so: passes 2–7 of
`dump-furnace_test/dump.json` are the six 256² face viewports with **no draws
in them**. The port sets the same six viewports and issues the same six empty
renders, because dropping them would be a different program that happened to
produce the same pixels. The first environment scene with an object in it will
be the first real test of the slicing, and `face_tile` is the gate that stands
in until then.

## r186 ships two `PMREMGenerator`s, and they disagree

This is the trap in the rung, and a solid-colour furnace is the one example
that cannot catch it.

| | `src/extras/PMREMGenerator.js` | `src/renderers/common/extras/PMREMGenerator.js` |
|---|---|---|
| renderer | WebGL | **WebGPU** — what `three.webgpu.js` is built from |
| `upSign` | `[ 1, -1, 1, 1, 1, 1 ]` | `[ 1, 1, 1, 1, -1, 1 ]` |
| `forwardSign` | `[ 1, 1, 1, -1, -1, -1 ]` | `[ 1, -1, 1, -1, 1, -1 ]` |
| the background box | rendered inside the face loop, once per face, inside that face's viewport | rendered **once**, before the loop, at the target's full viewport |
| tone mapping | forced to `NoToneMapping` for the duration | not touched |
| reversed depth | an extra `clearDepth()` pass | not present |

The port follows the second one, because that is the code the graded page runs:
`webgpu_furnace_test.html` imports `three/webgpu`, and
`dump-furnace_test/dump.json` confirms it from the outside — pass 1 is one
`drawIndexed( 36 )` at `setViewport( 0, 0, 768, 1024 )`, and passes 2–7 are the
six 256² face viewports with nothing in them. Seven passes, not twelve.

Two of those six rows would produce a *different atlas* if the wrong file were
followed, and **nothing in this example would notice**: the environment is one
colour, so a face pointing the wrong way, a flipped `up` and a box drawn six
times instead of once are all invisible in the frame, in the white-furnace
assertion and in the atlas readback alike. `tests/pmrem_scene.rs` holds the
WebGPU generator's table as literals for that reason, with the WebGL one in the
doc comment beside it so the next reader does not have to find this twice.

## The PMREM B caveat: resolved, with evidence

`docs/webgpu_pmrem_test-progress.md` and `docs/nodes.md` §13 both warned that
the port carried the **pre-r186** `PhysicalLightingModel` spelling, and that a
white furnace might expose a term it gets wrong. It does not, and the reason is
that the warning was out of date: the port already carries the r186
restructure. `furnace_physical` against `dump-furnace_test/m05` has, term for
term and in three's order:

* the 16×16 `rg16float` DFG LUT tap at `( Roughness, dot( N, V ) )`;
* `singleScatteringDielectric` / `multiScatteringDielectric` /
  `singleScatteringMetallic` / `multiScatteringMetallic`, each accumulated by
  Fdez-Agüera's `computeMultiscattering` with `Favg = Fr + ( 1 - Fr ) / 21`
  spelled as `* 0.047619`;
* `mix( single…Dielectric, single…Metallic, Metalness )` against `radiance` and
  `mix( multi…Dielectric, multi…Metallic, Metalness )` against
  `iblIrradiance * 1/π`;
* the diffuse `1 - ( singleScatteringDielectric + multiScatteringDielectric )`;
* `cosineWeightedIrradiance`, and the AO node's
  `clamp( ao - ( 1 - pow( NdotV + ao, exp2( -( 1 - roughness·-16 ) ) ) ) )`.

Every remaining difference is one of the classes `docs/nodes.md` §8 already
lists: three vars every intermediate where the port inlines (`m05` is 737 lines
to the port's 526 for the same arithmetic), three hoists `let dfg = …` where
the port re-spells `nodeVar2.xy`, the port hoists the zero initialisers, and
the port emits the ambient `indirectDiffuse()` block before the environment
block where three emits it after — which is numerically identical here because
`irradiance` is the zero vector with no lights in the scene, and the block
multiplies by it. The rendered frame is the independent check: 0 of 100 000
pixels, on a scene built to make an energy error a visible band.

No blocker, and nothing to report upward. The `webgpu_lights_physical` row is
unchanged at 4, which says the same thing from the other side.

## Two things in the example that are not optional

**The 40 is a plain vertical FoV.** Unlike `webgpu_pmrem_test`, this page has
no `updateCamera()` and does not reinterpret the field of view as a horizontal
one: it is `PerspectiveCamera( 40, aspect, 1, 30 )` at `( 0, 0, 18 )` and
nothing else. Taking the sibling example's conversion would shrink every
sphere, and only the image would say so.

**`scene.background` is the *same* `Color` the environment was built from.**
`createEnvironment()` hands `envScene.background` to `PMREMGenerator` and then
assigns it to the real scene, where a `Color` background is the pass' clear
colour rather than a skybox draw. That is why the frame's background is exactly
the environment, and why the furnace identity is visible at all: the spheres
have to disappear into it. `_sceneToCubeUV` restores the background it borrowed
before it returns, which `tests/pmrem_scene.rs` also asserts, so the example's
`env_scene.background.clone()` really is the same colour.

**No tone mapping.** The page sets none. With ACESFilmic the frame would be a
uniform *something else* and the identity would be invisible.

## Divergences, recorded in `docs/nodes.md` §13

* **`PMREMGenerator` does not hold the renderer, so `fromScene` takes it.**
  Already the shape of `from_cubemap` / `from_equirectangular`; `from_scene`
  additionally takes `&mut Scene`, because three's `_sceneToCubeUV` assigns
  `scene.background = null` for the duration of a solid-colour background and
  puts it back afterwards. The port does the same to the same field.
* **`PmremEnvironment` built from a scene has no source, so `update()` is a
  no-op.** Three's `PMREMNode` rebuilds lazily under `NodeUpdateType.RENDER`;
  three's *page* calls `fromScene` eagerly, in `createEnvironment()`, and there
  is no texture to watch for a change. `PmremEnvironment::from_scene` therefore
  builds at construction and `update()` returns immediately, so the example's
  per-frame call is free and the steady-frame assertion still runs over it.
* **`_setViewport` is `RenderTarget::set_viewport`.** Three's helper divides by
  the target's own size into a normalised `Vector4`; the port's target carries
  pixels with a top-left origin (the `webgpu_lines_fat` seam). Same rectangle,
  and `face_tile` is what pins it to three's arithmetic.

## What was ruled out or left out

* **`_blur` / `sphericalGaussianBlur`, and `BLUR_SAMPLES`.** Still no caller:
  `_applyPMREM` takes the GGX arm for every source the ladder has, scene
  included. Listed in §13 as it was.
* **A non-solid environment background.** `_sceneToCubeUV`'s other arm — a
  scene with actual objects, or a texture background — is written and set up
  but unexercised, because the only `fromScene` on the ladder is this one. The
  face loop, the viewports and the `autoClear` handling are all live; what has
  no pixels behind it is the claim that something *drawn into* face *i* stays
  out of face *i−1*. `face_tile` gates the rectangle; the first example with an
  object in its environment scene gates the rest.
* **`PMREMGenerator.dispose()` as a separate call.** `from_scene` cleans up its
  own scratch before it returns, as `from_texture` does, and hands back a live
  target the caller owns.
* **The "Tint for Visibility" checkbox.** One GUI toggle that would set every
  material's colour to `0xccccff`. `clean-page.js` removes the panel, the
  handler never fires, and the colour stays white.
