# webgpu_postprocessing_bloom_selective

Branch `rung-bloom-selective`, cut from `f949c0e` (the tip of the #93..#102
stack: rungs 10/11/12, radial blur, materials, ssaa, fat lines part A, PMREM
part A).

## Sitting 1 — MRT. **The example is not graded yet.**

The scout plan
(`scouts/scouts/webgpu_postprocessing_bloom_selective/PLAN.md`) calls this
example two rungs and splits them at MRT (§6). This sitting is its **rung A**:
`mrt()`, `material.mrtNode`, `PassNode::set_mrt`, the multi-texture
`RenderTarget`, multi-attachment passes and the `OutputType` fragment struct.
No `BloomNode`, no `uniformArray`, no example, no README row. The gate for this
half is, as the plan says, **the whole existing ladder plus the WGSL** — the
MRT change rewrites the fragment-output path every material on the ladder
depends on, and the struct's own name changes.

Ladder after this branch, all sixteen rows unchanged:

    depth_texture 0 / instance_mesh 60 / materials_basic 0 / rtt 1 /
    lights_phong 31 / morphtargets 0 / shadowmap 7 / lights_physical 4 /
    postprocessing_masking 18 / tsl_galaxy 40 / skinning 6 / mesh_batch 0 /
    compute_points 4 / radial_blur 7 / materials 44 / ssaa 0

### Deltas against the plan

The plan was written on 2026-09-19 against the current tree, so there are no
pre-0.2.0 deltas. Two of its §4 gap-list rows were already closed by the
branches this one was cut from and were used rather than rebuilt:

| plan row | what was there already |
|---|---|
| `NeutralToneMapping` | `ToneMapping::Neutral`, from `radial_blur` |
| effect node owning RTs + `Renderer::clear()` | `SsaaPassNode`, `RenderTarget::clone_target`, `auto_clear`, `UniformSource::Settable`, the frame-counting cache clock, from `ssaa` |

One plan reading was corrected: §5.2 asks for an `eprintln!` descriptor trace
as the MRT gate. A trace is not a gate — nothing fails when it is wrong. It is
`tests/renderer_mrt.rs` instead, which renders through a real two-attachment
pass and reads both attachments back.

## What was added

| area | what |
|---|---|
| `src/nodes/mrt.rs` | new module: `MrtNode`, `mrt()`, `merge()`, and the `members()` layout-by-attachment-index of `MRTNode.setup()` |
| `src/nodes/builder.rs` | `MaterialFlow.mrt`; the `OutputType` struct branch, the `output.mN` flow lines and the empty result section |
| `src/materials/node_material.rs` | `SetupContext.mrt` + `MrtContext`; `NodeMaterial.setup()`'s MRT branch (merge, then the members) |
| `src/materials/mod.rs` | `MeshBasicNodeMaterial::mrt_node` |
| `src/renderer/render_target.rs` | `RenderTargetInner.extra_textures`, `add_texture()`, `attachment_names()`, `textures()`, `OUTPUT_ATTACHMENT`; `set_size` / `clone_target` carry the extras |
| `src/renderer/mod.rs` | `Renderer::set_mrt` / `mrt()`; the per-pass MRT context; one colour attachment per target texture in the pass descriptor; the extra attachments' GPU textures |
| `src/renderer/programs.rs` | `RenderState.color_attachments`, and one `ColorTargetState` per attachment |
| `src/renderer/pass.rs` | `PassNode::set_mrt` / `mrt()` / `texture_node( name )` / `texture_named( name )`; `render()` sets and restores the renderer's MRT |
| `examples/dump_wgsl.rs` | the `bloom_selective_scene` section |
| `tests/renderer_mrt.rs` | new: the drawn gate |
| `docs/postprocessing.md` | "MRT: several colour attachments from one draw" |

## WGSL

Against `scouts/scouts/webgpu_postprocessing_bloom_selective/dump/`:

* **scene fragment** (`m01`) — **byte-identical**, including the `OutputType`
  struct name, the `m0` / `m1` member names, the struct's trailing tab line,
  both `output.mN = …` lines in the flow and the empty result section. The only
  differences are the banner and `nodeUniform5` → `nodeUniform3`, the
  "Generated names" entry of `docs/nodes.md` §8 (three leaves gaps for uniforms
  it consumes but never emits; this port does not).
* **scene vertex** (`m00`) — identical statement for statement, with the known
  `VERTEX_` sub-build divergence: three writes `VERTEX_nodeVar0` and
  `VERTEX_v_modelViewProjection` where the port writes the varying directly.

**No new divergence class.** `docs/nodes.md` §8 gained nothing: the MRT
fragment is the first shader on the ladder that matches three's text in full.

Every pre-existing `dump_wgsl` section is byte-identical before and after this
branch — 184 sections, compared one by one against a baseline built at
`f949c0e`; the three `bloom_selective_scene` ones are the only additions. That
was the scout's first risk (§8.1): the MRT branch perturbing the output path of
all sixteen green rungs. It did not, because the branch is taken only when the
renderer has an MRT *and* a render target, which is a state nothing else on the
ladder is ever in.

## What the pixels found

Nothing yet — this sitting draws no new image. What the gates found instead:

* **The members are laid out by attachment index, not by dictionary order.**
  `MRTNode.setup()` walks `outputNodes` but writes `members[ index ]`, where
  `index` comes from `getTextureIndex( renderTarget.textures, name )`. Getting
  this wrong swaps the two `@location`s and is invisible in the WGSL structure —
  both shapes compile.  `members_are_laid_out_by_attachment_index_not_dictionary_order`
  and the drawn `the_material_mrt_beats_the_pass_default` pin it.
* **`Output` must be analysed once, not twice.** The port's existing rule gives
  the basic output its own var because it is reached by both `Output` and
  `output.color`. With an MRT it is reached only by `Output` — the MRT's
  `output` member reads the property back — so the extra `analyze()` had to be
  suppressed, exactly as a custom `outputNode` already suppresses it. With it
  left in, the fragment grows a `nodeVar0` three's dump does not have.
* **An attachment is created by `getTextureNode( name )`, not by the MRT.**
  Three drops an output whose name is not among `renderTarget.textures`
  (`index === -1`), so an MRT alone renders single-attachment. Reproducing that
  rather than creating an attachment per MRT name is what keeps the count of
  attachments a property of what the graph *samples*.

## What was ruled out

* **`MRTNode.blendModes` / `.clearColors`.** Three carries a per-output blend
  mode (`MaterialBlending` for `output`, `NoBlending` for the rest) and a
  per-output clear colour. Every MRT material here is opaque, where both blend
  modes resolve to no blend state at all, and nothing sets a per-output clear.
  Porting them would have been two fields nothing reads; `docs/postprocessing.md`
  records the omission.
* **A `Texture.name` field.** Three names the textures themselves and looks the
  index up with `getTextureIndex( textures, name )`. The port keeps the names on
  the `RenderTarget` beside the textures instead, because a name on `Texture`
  would be a public field on every texture in the tree that exactly one caller
  reads.
* **MSAA on the extra attachments.** A `PassNode` carrying an MRT is
  `samples: 0` in three and here, so only attachment 0 has a resolve target.
  An MSAA MRT pass would need a resolve texture per attachment; nothing asks.
* **Raising the attachment count into the program cache key.** It is in
  `RenderState`, which keys the *pipeline*, not the program: the WGSL already
  differs (it is a different struct), so the program key separates them by its
  own hash and the attachment count only has to keep two pipelines off one
  program apart.

## Left out — and exactly what sitting 2 must do

Everything in the plan's **rung B** (§6.5–§6.9). In order:

1. **`uniformArray`** — `array<vec4<f32>, 5>` in a uniform block, three's
   `src/nodes/accessors/UniformArrayNode.js`. `Bloom_comp` is the only consumer
   here (`NodeBuffer_1297`); `GodraysNode`, `SSAONode`, `GTAONode` and `SSRNode`
   want it later. ~120 lines in `src/nodes/tsl.rs` + `src/renderer/programs.rs`.
   Gate: a `dump_wgsl` entry that emits the declaration.
2. **The three quad materials**, each with its own WGSL gate:
   `Bloom_highPass` against `dump/m03`, `Bloom_separable` against
   `dump/m05..m09` (five modules, one per mip), `Bloom_comp` against
   `dump/m11`. Do the separable one second — it carries the Gaussian.
3. **The Gaussian coefficients as a unit test** before any GPU work: kernel
   radii `[6, 10, 14, 18, 22]` → `[3, 5, 7, 9, 11]` bilinear taps, computed the
   way `_getSeparableBlurMaterial` does (`0.39894 * exp( -0.5 i² / σ² ) / σ`,
   σ = radius / 3) and asserted against the floats in `m05..m09`. Mip 0 is
   offsets `[1.40733340004593, 3.294214972162989, 5.0]`, weights
   `[0.2970163278514283, 0.09175375661117716, 0.008764100149861079]`,
   centre weight `0.19947`. **Compute them; do not paste them.**
4. **The mip-size table as a unit test**: 800×500 → 400×250 → 200×125 →
   100×**62** → 50×**31** → 25×15. It is `Math.round( w / 2 )` then halved four
   times with rounding, not integer division — 100 → 62, not 50.
5. **`BloomNode`'s eleven render targets** (`bright`, `h0..h4`, `v0..v4`, all
   `rgba16float` and all `depth_buffer: false`), `set_size`'s rounding, and the
   twelve-pass `render()`. Gate: the plan's §3.2 pass table, all fourteen rows —
   pass 1 the only multi-attachment and the only one with depth, passes 2–13
   with one colour attachment and **no depth**.
6. **`examples/webgpu_postprocessing_bloom_selective.rs`**, the `tests/e2e`
   row, the `dump_wgsl` sections and the README row. Assert the plan's §5.4
   sphere oracle *before* the pixel diff: nine `Math.random` draws per sphere in
   page order, the third of which decides whether that sphere blooms. An
   off-by-one there is a dramatic, hard-to-attribute failure.

What sitting 1 leaves ready for it:

* `pass.set_mrt( … )` and `pass.texture_node( "bloomIntensity" )` are the two
  calls the page's TSL needs; `outputPass.mul( bloomIntensityPass )` is then
  ordinary node arithmetic.
* `render_pipeline.output_color_transform = false` with `render_output()` on the
  output node already works (`RenderPipeline` supports it, and `radial_blur`
  uses the same `neutralToneMapping` tail).
* `SsaaPassNode` is the worked example of an effect node that owns render
  targets and drives its own quad passes; `Renderer::render_quad`,
  `RenderTarget::clone_target`, `auto_clear` / `clear()` and
  `UniformSource::Settable` are all in place. `BloomNode`'s `direction` uniform,
  swapped between the horizontal and the vertical pass of each mip, is
  `uniform_settable` — five `Bloom_separable` programs, ten passes, as the dump
  has it.

### Three notes from the director, carried forward

* **`highPassFn` must be replaceable.** `webgpu_postprocessing_bloom` and
  `_anamorphic` reuse `BloomNode` with their own high pass, so the port's
  `BloomNode` takes a node-returning function as three's does, rather than
  hard-coding the selective-bloom luminosity high pass.
* **`resolution_scale` must be real.** `setResolutionScale` has to change the
  mip chain's size, floor-halved per mip as three does. `PassNode._resolutionScale`
  is not ported yet either; both land in sitting 2.
* **`luminance` is vec3 here.** The director flagged that TSL's `luminance` on a
  `vec4` widens the coefficient with `w = 1.0`. This example's dump does **not**
  take that branch — `m03` line 47 is `dot( nodeVar2.xyz, vec3<f32>( 0.2126,
  0.7152, 0.0722 ) )`, because `BloomNode` calls `luminance( color.rgb )` — so
  the port's vec3-only `luminance()` matches it as it stands. An example that
  passes a `vec4` will need the other branch; this one does not.

### Also left out of sitting 1

* **`PassNode::getTexture( 'depth' )` and the previous-frame textures.**
  `_previousTextures` / `toggleTexture()` are the history-buffer half of
  `PassNode`, for `difference`, `afterimage` and `traa`. Untouched.
* **`velocity` as an MRT output.** `webgpu_postprocessing_motion_blur` is the
  next MRT consumer and needs the `velocity` node (previous model-view
  matrices) on top of this sitting's work; the MRT half of it is now free.
* **A `read_target_pixels` that takes an attachment index.** `tests/renderer_mrt.rs`
  reads attachment 1 by sampling it into a probe target instead, which also
  proves the attachment binds as a texture — the thing `BloomNode` will do.

---

## Sitting 2 — `BloomNode`, the example, the image gate. **Green at 1 pixel.**

Branch `rung-bloom-selective-2`, cut from `origin/main` at `512ece1` (which
carries sitting 1's MRT work). This is the plan's **rung B** (§6.5–§6.9) and
the list at the end of sitting 1, in that order.

    webgpu_postprocessing_bloom_selective: 1 of 100000 pixels (0.001%), limit 0.1%

Ladder after this branch, all eighteen rows unchanged:

    depth_texture 0 / instance_mesh 60 / materials_basic 0 / rtt 1 /
    lights_phong 31 / morphtargets 0 / shadowmap 7 / lights_physical 4 /
    postprocessing_masking 18 / tsl_galaxy 40 / skinning 6 / mesh_batch 0 /
    compute_points 4 / radial_blur 7 / materials 44 / ssaa 0 / lines_fat 0 /
    pmrem_cubemap 0

Steady frame 16.5 ms (ceiling 100 ms) over 63 draw calls and 256k triangles,
and frames two and three build and upload **nothing**.

## What was added

| area | what |
|---|---|
| `src/nodes/display/bloom.rs` | new module: `BloomNode`, `bloom()`, `luminosity_high_pass()`, `HighPassInput`, the eleven render targets, `set_size`, the twelve-pass `render()`, and the four unit tests |
| `src/nodes/node.rs` | `Node::ArrayVar` (an `array< T, N >` literal in a `var<private>`); `BufferSource::UniformArray` |
| `src/nodes/builder.rs` | `declare_var_typed()` — a var whose WGSL type is not one of `Type`'s — and the `ArrayVar` generate arm |
| `src/nodes/tsl.rs` | `array_var()`, `uniform_array_vec3()` + `UniformArray::element()`, `texture_sample()` |
| `src/renderer/mod.rs` | the `BufferSource::UniformArray` upload arm, cached on the buffer id |
| `src/renderer/pass.rs` | `texture_node( name )` is now **one** var, not two (see below) |
| `examples/webgpu_postprocessing_bloom_selective.rs` | new: the fifty spheres, the MRT pass, the bloom and the output quad |
| `examples/dump_wgsl.rs` | `bloom_high_pass`, `bloom_separable_0..4`, `bloom_comp`, `bloom_render_pipeline_quad` |
| `tests/e2e/main.rs` | the rung's test, `assert_spheres()` before the pixel diff, and the `rung!()` row |
| `tests/fixtures/webgpu_postprocessing_bloom_selective/` | `spheres_t0.json` + the `oracle.mjs` that generates it |
| `docs/postprocessing.md` | "`BloomNode`: eleven render targets and twelve quads" |
| `docs/nodes.md` | §8: `NodeBuffer_N` numbering and the uniform index a buffer does not consume |

## WGSL

Every one of the eight bloom modules was diffed against the scout's dump.

| module | dump | difference |
|---|---|---|
| `Bloom_highPass` vertex / fragment | `m02` / `m03` | the `// Three.js r186` header line, and one blank line in the empty uniforms section |
| `Bloom_separable` ×5 | `m05`..`m09` | the header line only — the baked offsets, weights and centre weights are **computed**, and they come out bit-identical |
| `Bloom_comp` | `m11` | generated names only: `NodeBuffer_0` vs `NodeBuffer_1297`, `fn0` vs `fn4`, and the object struct's `0, 2, 4, 6, 8, 10, 11` against three's `0, 3, 5, 7, 9, 11, 12` — three's uniform *buffer* consumes a `nodeUniformN` index and this port's does not. Plus the struct declaration order (`docs/nodes.md` §8, "Declaration order") |
| `RenderPipeline` | `m12` | `fn0`/`fn1` numbering and the order the two `fn`s are emitted in; the flow is line-for-line identical |

The separable modules being byte-identical is the load-bearing one: five
kernels, 35 magic floats, none of them pasted.

## What the pixels found

One pixel, on the first GPU run. What did *not* have to be found by pixels,
because an earlier gate caught it:

* **The fifty spheres came out somewhere else entirely** — `assert_spheres()`
  failed on sphere 0's x before the renderer was asked for a frame. The bug was
  in the oracle, not the port: `oracle.mjs` imported three's build unpatched, so
  `Object3D`'s `generateUUID()` — `Math.random() * 0xffffffff` — ate a draw per
  object and shifted the whole 450-draw sequence. `test/e2e/puppeteer.js`
  rewrites exactly that expression to `Math._random()` before injecting the
  build into the page, and `oracle.mjs` now does the same to the two build
  files it loads. Had the oracle been written to match the port instead, this
  would have been a silent agreement between two wrong things.
* **The Gaussian and the mip chain** were unit tests before any GPU work, as
  the plan asks. Both passed first time, which is the outcome that makes them
  worth keeping: the 35 coefficients are now known to be three's, not
  plausible.

## Plan corrections

* **`setSize` floors; it does not round.** The plan (§5.5) and sitting 1's
  hand-off both read the dumped chain as `Math.round( w / 2 )` — "100 → 62, not
  50". `BloomNode.js` is `Math.floor` throughout, and at these sizes the two
  rules disagree: `floor( 125 / 2 )` is 62, `round( 125 / 2 )` is 63. The
  numbers in the plan are right and the rule beside them is not.
  `the_mip_chain_is_floor_halved` pins the rule.
* **The §5.4 sphere oracle did not exist.** The plan proposed dumping
  `spheres_t0.json` with `page.evaluate`; no such file was in `dump/`. It is
  generated instead from three's own `build/` in node with no GPU and no
  browser, the way `webgpu_materials`' `scene_t0.json` was, and the generator
  ships next to it.
* **Nine draws per sphere, not the plan's "450 draws" alone.** The order is
  hue, lightness, the `bloomIntensity` coin flip, x, y, z, the
  `multiplyScalar` radius, and **two** for
  `scale.setScalar( Math.random() * Math.random() + 0.5 )`. 27 of the 50
  spheres bloom.

## Decisions a reviewer should look at

* **`PassNode::texture_node( name )` now emits one var, not two**
  (`src/renderer/pass.rs`, and `docs/postprocessing.md`'s `PassNode` section).
  Sitting 1 gave it the same `to_var( texture_uv( … ) )` pair `node()` has. The
  dumps say otherwise: `m03` samples the scene pass's output into a single
  `nodeVar0`, while `m12` gives the *bloom* texture two
  (`nodeVar2 = nodeVar1`). The rule is three's `TempNode` promotion —
  `PassTextureNode.setup()` builds its `passNode`, so a pass reaches usage two
  and earns a var only when the graph *also* holds the pass itself.
  `pass( scene, camera )` used directly (radial blur, ssaa) is that case;
  `getTextureNode( name )` is not. Nothing green used `texture_node` yet, so the
  change is confined to this rung.
* **Ten separable blur materials where three.js has five.** three.js swaps
  `colorTexture.value` between a mip's horizontal and vertical pass; a
  `Texture` is an identity here, so each direction needs its own material. The
  WGSL is the same text — the five `bloom_separable_N` dump sections prove it —
  and the cost is five extra pipelines. `docs/postprocessing.md`, "Ten
  separable materials".
* **`lerpBloomFactor` is one `FnDef` in a `thread_local`**, the way
  `saturation()` and `hue()` already are. Five `shader_fn()` calls would be five
  identical `fn`s in `Bloom_comp`; the dump has one.
* **The oracle fixture is in this tree**, not in the scouts worktree where
  `webgpu_lines_fat`'s lives, so the branch is self-contained and CI can run it
  without a second checkout. `tests/fixtures/webgpu_materials/scene_t0.json` is
  the precedent.

## Left out

* **`PassNode._resolutionScale`.** `BloomNode::set_resolution_scale` is real and
  tested; the pass's own scale is still unported, and nothing on the ladder sets
  it.
* **`BloomNode.dispose()` and the `RenderTarget` disposal path.** The targets
  live as long as the node, which lives as long as the app.
* **The page's interaction.** `OrbitControls`, the raycaster that toggles a
  sphere's `bloomIntensity` on pointerdown, and the inspector GUI folders are
  not ported; none of them touches the graded frame. The uniform the raycaster
  would flip is reachable (`material.mrt_node`), which is what `assert_spheres`
  reads.
* **`luminance()` on a `vec4`.** Still vec3-only, as sitting 1 left it: this
  example's `m03` takes the `color.rgb` branch. The next `BloomNode` consumer
  with a `vec4` high pass will need the other one.
* **`BloomNode` in the viewer.** Keys 1..0 are full; the steady-frame number
  comes from the e2e harness, as radial blur and ssaa did.
