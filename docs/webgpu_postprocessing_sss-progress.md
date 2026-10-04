# webgpu_postprocessing_sss

Branch `sss-node`.

**Not graded, and it cannot be.** three lists `webgpu_postprocessing_sss` in
its own e2e exception list (`test/e2e/puppeteer.js`, under "Black screen"),
with `webgpu_postprocessing_traa`. That page's note explains why the port is
not graded against a reference that upstream does not grade itself.

## The shaders

three's dump of the page has 17 WGSL modules. That is 8 vertex/fragment
pairs plus the mipmap blit, for the 8 materials below.

| modules | material | gated |
|---|---|---|
| m00 | mipmap blit | no (renderer internal, not part of this page's port) |
| m01 / m02 | `ShadowMaterial` (shadow-map depth pass) | no (existing shadow path) |
| m03 / m04 | the statue in the pre-pass (velocity MRT) | no |
| m05 / m06 | the ground in the pre-pass | no |
| m07 / m08 | **the SSS quad** | **yes**: `sss_matches_three`, fragment body |
| m09 / m10 | the statue in the scene pass | no |
| m11 / m12 | **the ground in the scene pass, with the shadow context** | **yes**: `sss_shadow_context_matches_three`, fragment body |
| m13 / m14 | TRAA resolve | yes, already: m14 is byte-identical to the TRAA page's fixture after its header, so `traa_resolve_matches_three` covers it |
| m15 / m16 | the pipeline's output quad | no (existing) |

That makes three of the eight fragment shaders gated. Two of them are new on
this branch. The statue's pre-pass and scene materials are glTF Standard
materials with nothing SSS-specific beyond the context. The ground gate
covers the context's multiply, and the statue takes it through the same
`setup_light` path.

The ground gate fingerprints the port's WGSL as it is. It also checks that
each lighting accumulator is zeroed once, right above the statement that
first uses it, as in three's dump. Before issue #281 was fixed, the Phong
flow emitted the zero of `irradiance`, `directDiffuse`, `directSpecular` and
`indirectDiffuse` twice in a row, and the gate dropped adjacent repeated lines
to get past it.

## What else is checked

- **The frames.** `tests/sss_frames.rs` runs on the GPU. A unit box sits on
  a floor, 64×64, with a directional light behind it and the SSS texture as
  the pipeline's output. The test checks that:
  - the sky keeps the white clear;
  - a band of floor in front of the box is black at `shadowIntensity` 1 and
    mid-grey at 0.5, with no pixel below that (no NaN);
  - a sub-pixel `maxDistance` occludes nothing;
  - a `thickness` of 0.0001 takes the box's face for a surface the rays
    pass behind, so the band goes;
  - `quality` 1 occludes more than three's 0.5, and 0.1 less than half as
    much;
  - temporal filtering and half resolution keep the band;
  - with the shadow context, a Phong floor's beauty is unchanged wherever
    the SSS is white and darker in the band;
  - clearing the context restores the plain frame;
  - a floor that does not receive shadows is unchanged by the context.
- **The page.** `examples/webgpu_postprocessing_sss.rs` builds the page's
  initial state and runs in the viewer (`viewer postprocessing_sss`, or
  `--headless --frames 40`).

## What is left out

`docs/nodes.md` §71.3 lists what the node leaves out: an orthographic
camera, a logarithmic depth buffer, and the quad's shared context. The
example's module docs cover what the page does differently:

- the model loads synchronously;
- the inspector GUI is not ported.

The page is not in the browser shell, and it has no web manifest. The shell
builds only the README's graded examples, and the web gate grades every one
it builds (see `web/README.md`, "Adding an example").

## Merge note

`gtao-denoise` (#267) added `builtinAOContext` as `PassNode::set_context_ao`,
the AO twin of this branch's `set_context_shadow`: a `PassState` field that
the pass hands to a `Renderer` field for the length of its render. It landed
on `main` first, and the merge of `main` into this branch keeps both contexts
side by side in `src/renderer/pass.rs` and `src/renderer/mod.rs`; neither
replaces the other. `docs/nodes.md` has GTAO at §64 and SSS at §71.
