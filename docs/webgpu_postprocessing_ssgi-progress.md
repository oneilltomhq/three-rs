# webgpu_postprocessing_ssgi

Branch `ssgi-node`.

**Not graded, and it cannot be.** three lists `webgpu_postprocessing_ssgi`
in its own e2e exception list (`test/e2e/puppeteer.js`, under "Black
screen"), so three's harness does not grade the page and there is no pixel
number to match. A port graded against a reference that upstream does not
grade against itself would claim more than it shows.

## What is checked instead

- **The shaders.** Three of the page's quads, compared with three's dumps of
  this page (`tests/nodes_display_wgsl.rs`), with the fingerprint used by the
  other display gates: the multiset of calls and float literals, the
  if/else/for sequence, and the bindings.
  - `ssgi_matches_three` checks the SSGI quad's fragment body (fixture
    `webgpu_postprocessing_ssgi_m07_ssgi.wgsl`). It is the whole of three's
    `gi` function: two slices of horizon search, each with two
    `stepCount`-long marches and the light gathering, after the
    `useTemporalFiltering` rotations and offsets.
    `ssgi_spatial_offsets_matches_three` and
    `ssgi_gtao_fast_acos_matches_three` check its two helper functions.
  - `ssgi_composite_matches_three` checks the page's composite, the
    `convertToTexture` quad that multiplies the beauty by the AO and adds
    `diffuse × GI` (fixture `…_m09_rtt.wgsl`).
  - `ssgi_traa_resolve_matches_three` checks the TRAA resolve over that
    composite (fixture `…_m11_traa_resolve.wgsl`). The TRAA helpers are
    already gated by `traa_*_matches_three`.

  The dump's other quads are the scene pass and the output pass, which are
  gated elsewhere.
- **The frames.** `tests/ssgi_frames.rs` runs on the GPU. It renders an
  unlit red wall on an unlit white floor at 64×64, with the page's two
  slices and eight steps, and checks:
  - the sky keeps its clear: white in the AO and black in the GI, since
    three's backend clears every attachment after the first to black;
  - the floor at the wall's foot is darker in the AO than the open floor
    (about 210 against 240 on the Iris Xe);
  - no AO pixel is black, so nothing is NaN;
  - the floor at the wall's foot gathers the wall's red as GI, red well
    above green, and more of it than the open floor (about 208 against 0);
  - with temporal filtering off two frames are identical, and with it on
    they differ;
  - `gi_intensity = 0` is a uniform write that blacks the GI out on the
    next frame.
- **The page.** `examples/webgpu_postprocessing_ssgi.rs` renders the page's
  Cornell box with the page's settings and runs in the viewer
  (`viewer ssgi`).

## What is left out

`docs/nodes.md` §69.3 lists them:

- three's `normalNode = null` path, which rebuilds normals from depth;
- logarithmic depth;
- `resolutionScale`, which r187's `SSGINode` does not have.

The page's GUI is not ported either. Its settings are public
`SettableValue`s on `SsgiNode`, and the page's values (two slices, eight
steps) are set in code.

The GI target is `RG11B10UFloat`, as in three. If the adapter cannot render
to it, the port logs three's error and falls back to `RGBA16Float`, where
three would carry on and produce a broken target.

The page is not in the browser shell. The shell builds only the README's
graded examples, and the web gate grades every one it builds (see
`web/README.md`, "Adding an example").
