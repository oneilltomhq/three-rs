# webgpu_postprocessing_ssr_denoise

Branch `ssr-denoise`, `docs/nodes.md` §89.

**Not graded, and it cannot be.** three lists
`webgpu_postprocessing_ssr_denoise` in its own e2e exception list
(`test/e2e/puppeteer.js`, under "Need more time to render"). three's harness
does not grade the page, so there is no pixel number to match.
`examples/screenshots/webgpu_postprocessing_ssr_denoise.jpg` does exist, but
three's own CI does not hold its page to it, and the screenshot predates
later changes to the page. A port graded against a reference that upstream
does not grade against itself would claim more than it shows.

## What is checked instead

- **The shaders.** Five of the page's materials, compared with three's dump
  of this page (`tests/nodes_display_wgsl.rs`, fixtures
  `webgpu_postprocessing_ssr_denoise_m*.wgsl`):
  - `ssr_denoise_page_floor_matches_three`: the dungeon's stone floor
    (`m14`), with its ORM map, the patched lighting and the four-attachment
    MRT. It checks the `Roughness` statement, the metallic/dielectric mix,
    the MRT tail, and two exact lines: `Metalness` read straight from the
    map, and the same expression spelled out again in
    `DiffuseContribution`. The rest of the body carries two §8 differences
    that are not this page's (see "What differs");
  - `ssr_denoise_page_ssr_matches_three`: the stochastic SSR quad (`m20`);
  - `ssr_denoise_page_denoise_matches_three`: the recurrent denoiser's
    quad, in `'specular'` mode with the ray-length alpha (`m22`);
  - `ssr_denoise_page_grading_matches_three`: the combined output, AgX,
    contrast, saturation and gamma, drawn by the `convert_to_texture` RTT
    (`m24`);
  - `ssr_denoise_page_sharpen_matches_three`: the sharpen quad at the page's
    sharpness 0 (`m29`).

  The page's TRAA resolve and RTT copy are the same shaders as
  `webgpu_postprocessing_traa_m05` and `webgpu_postprocessing_lensflare_m24`, already
  gated. The temporal-reproject resolve is gated by
  `temporal_reproject_resolve_specular`. The fingerprint is the multiset of
  calls and float literals plus the if/else/for sequence, as in the other
  display gates.
- **The frames.** `tests/ssr_denoise_frames.rs`, on the GPU, runs the page's
  whole chain at 64×64 over a white unlit box on a rough metal floor whose
  only light is the box's reflection. It checks:
  - the first frame's SSR and denoise targets have no NaN or infinity;
  - the floor inside the box's mirror image is lit on the canvas (mean
    0.87 on the Iris Xe), and the floor well away from it is dark (0.0);
  - over sixteen frames the reflection stays put: the lit pixels' centroid
    moves by under a pixel, the lit count keeps 90% of its first value, and
    the canvas mean moves by under a dozen levels;
  - meanwhile the denoised floor's frame-to-frame noise, the mean
    per-pixel variance over a window of four frames, falls below half its
    first value (0.0113 to 0.0019 on the Iris Xe);
  - a resize restarts the chain's targets at the new size, and the next
    frames are finite.
- **The page.** `examples/webgpu_postprocessing_ssr_denoise.rs` renders the
  page's first frame and runs in the viewer
  (`viewer webgpu_postprocessing_ssr_denoise`).

## What the port adds

- `environment_specular` on every node material, the page's
  `indirectSpecular` patch: when false, the physical model's specular term
  takes no radiance from the environment, and a clearcoat model's clearcoat
  radiance is zeroed too.
- `material_metalness_value()` / `material_roughness_value()`, the
  `metalness` / `roughness` a standard material actually uses (its value
  times its map's channel), which the page writes to the MRT.
- Three fixes the chain needed: the recurrent denoiser allocates its target
  before the first frame reads it; a struct-typed temporary declares its
  struct type instead of `void`; and `saturation()` shares one `.rgb` of its
  input, as three's swizzle cache does, so the grading pass spells the
  contrast expression twice instead of hoisting it.

## What differs

- **The patch is per material.** three replaces
  `PhysicalLightingModel.prototype.indirectSpecular` for every material on
  the page; the port sets the flag on each material the model brings.
- **No shadow `autoUpdate = false`.** The shadow map is re-rendered every
  frame, to the same result.
- **The floor body's other differences** are §8 ones shared by every
  physical material with an environment: the doubled accumulator zeros
  (issue #281) and a separate single/multi-scattering block for the
  irradiance's indirect diffuse. The floor gate checks the regions this page
  changes.
- **UV transforms** are shared per texture, where three has one per map slot,
  so the port's uniform names differ (`nodeUniform3/6/8` in three's).
- **The first frames.** The model and the HDR load synchronously, and the
  page's first TRAA/sharpen pair, which `updateOutputNode()` replaces before
  it renders, is built once.
- The GUI and the compare modes are not ported.

The page is not in the browser shell. The shell builds only the README's
graded examples, and the web gate grades every one it builds (see
`web/README.md`, "Adding an example").
