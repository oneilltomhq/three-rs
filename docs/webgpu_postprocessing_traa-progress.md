# webgpu_postprocessing_traa

Branch `velocity-traa`, issue #165.

**Not graded, and it cannot be.** three lists `webgpu_postprocessing_traa`
in its own e2e exception list (`test/e2e/puppeteer.js`, under "Black
screen"). three's harness does not grade the page, so there is no pixel
number to match. `examples/screenshots/webgpu_postprocessing_traa.jpg` does
exist, but three's own CI does not hold its page to it. A port graded against
a reference that upstream does not grade against itself would claim more
than it shows.

## What is checked instead

- **The shader.** The resolve material, compared with three's dump of this
  page: `traa_resolve_matches_three` checks the fragment body and its
  bindings, and three more tests check `subpixelCorrection`, `clipAABB` and
  `flickerReduction` (`tests/nodes_display_wgsl.rs`, fixture
  `webgpu_postprocessing_traa_m05_traa_resolve.wgsl`). The fingerprint is
  the multiset of calls and float literals plus the if/else/for sequence, as
  in the other display gates.
- **The frames.** `tests/traa_frames.rs`, on the GPU, renders a white box
  turned to give slanted edges, at 48×48 with no MSAA. It checks:
  - the first frame is the jittered beauty: at most a few pixels are
    neither box nor background (see "The first frame" below);
  - after sixteen jittered frames, at least eight more pixels are in
    between, and every one of them is on the silhouette, within two pixels
    of both box and background (60 against 3 on the Iris Xe);
  - the inside of the box and the background do not change, so nothing
    smears and nothing is NaN;
  - the camera's view offset was set, and is cleared after each frame;
  - a second `claim_view_offset()` on the pipeline is refused;
  - a resize restarts the history at the new size.
- **The page.** `examples/webgpu_postprocessing_traa.rs` renders the page's
  first frame, which shows the same pose as three's screenshot, and runs in
  the viewer (`viewer traa`). Headless, at 800×500, it takes 2.35 ms a frame
  (mean of 30 after 10 warm-up frames, Iris Xe). It makes four draw calls
  and creates no GPU objects per frame.

## The first frame

The history is seeded with a copy of the beauty, so you would expect the
first frame to be the beauty itself. Not quite. Variance clipping moves the
history into the 3×3 neighbourhood's mean ± σ even when it equals the
current colour. At a one-pixel tip of the silhouette, where most of the
neighbourhood is background, that range stops short of the tip's own white.
The flicker-reducing blend then lands at about 233 of 255. That is three's
arithmetic, so the test allows a few such pixels instead of asserting none.

## What is left out

`docs/nodes.md` §63.3 lists them: orthographic cameras, logarithmic or
reversed depth, a beauty that is an `RTTNode`, a non-global `velocity`, and
the tuning properties, which are constants at three's defaults.

The page is not in the browser shell. The shell builds only the README's
graded examples, and the web gate grades every one it builds (see
`web/README.md`, "Adding an example").
