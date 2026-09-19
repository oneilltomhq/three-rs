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
