# The examples gallery

A browsable index of every example that passes the grader, in the spirit of
three.js's own `examples/` page, in two places:

- **`README.md`**, between `<!-- gallery:start -->` and `<!-- gallery:end -->`:
  a thumbnail grid, four across, near the "Examples graded green" table whose
  numbers it illustrates. The image URLs are absolute
  `raw.githubusercontent.com` ones so the grid renders on crates.io as well as
  on GitHub. Each thumbnail opens the example running in the browser (the
  `web/` shell, deployed to <https://oneilltomhq.github.io/three-rs/>); the
  caption under it links the ported source, and the rung's progress note when
  there is one.
- **`target/gallery/index.html`**, local only: a filter box and one card per
  example, pairing our frame with Three's reference screenshot at full size,
  with the diff count, the steady frame, the draw calls and the triangles
  beside them, and links to the ported source, the rung's progress note,
  Three's live example and Three's source.

Both come from `examples/gallery.rs`.

## Regenerating

Run the ladder, then run the generator:

```sh
cargo test --release --test e2e            # fills target/e2e/<name>/actual.png
cargo run --release --example gallery      # thumbnails, README block, index.html
cargo run --release --example gallery -- --vendor /path/to/three.js
```

To change only the shape of the README block (its links, its captions), with
no ladder run, no GPU and no three.js checkout:

```sh
cargo run --release --example gallery -- --readme-only
```

That rebuilds the block from the thumbnails already committed under
`docs/gallery/`; it does not add a rung, which still needs the full run.

The three.js checkout the local page reads its reference screenshots from is
`--vendor`, else `$THREE_VENDOR`, else `$THREE_JS_DIR`, else
`~/src/vendor/three.js`.

**Do this whenever a rung lands.** The thumbnails under `docs/gallery/` are
committed (~10 KB each, 400 px wide, JPEG quality 80), so a new rung's picture
only appears once someone regenerates and commits it. The generator is
idempotent: re-running it with nothing new changes nothing, and it only ever
rewrites what is between the two markers and the table's `browser` column, so
a rung worker adding a row to the graded table will not collide with it.

## Thumbnails are byte-deterministic

A gallery run on a branch that renders nothing differently leaves
`docs/gallery/` byte-identical, so a rung PR commits every thumbnail the
generator writes and no others need reverting. Three things hold that:

- **The render is deterministic.** On the Iris Xe the full ladder gives
  bit-identical `actual.png` frames run to run, and debug and release builds
  give the same frames (checked for #157: two debug ladders and a release
  ladder, 56 of 56 frames equal).
- **The encoding is deterministic.** The 2:1 box filter is integer
  arithmetic, and `jpeg-encoder` is pure Rust, built without its `simd`
  feature. `the_thumbnail_encoding_is_pinned` fixes the bytes a synthetic
  frame encodes to; if an encoder bump or a change to the size, quality or
  filter moves them, regenerate all of `docs/gallery/` from a fresh full
  ladder in one commit and update the test's numbers. The generator encodes in
  memory and does not touch a file that already holds the same bytes.
- **Stale frames are not thumbnailed.** What made runs rewrite thumbnails
  (#157) was neither of the above: `target/e2e/<name>/actual.png` is whatever
  the last run of that rung left, perhaps from before a rebase or from a
  rung's own earlier iterations, and the generator used to thumbnail all of
  it. Two committed thumbnails (`webgpu_tsl_angular_slicing`,
  `webgpu_textures_2d-array_compressed`) had come from such frames: not even
  the commits that added them render those pixels. Now a frame older than the
  newest e2e test binary (`target/{debug,release}/deps/e2e-*`) is left alone
  and named on the way out; its committed thumbnail stays. So running only
  your own rung and then the generator refreshes only your rung's picture.

The README block has a test too: `the_readme_gallery_block_is_current` fails
when the block is not exactly what `--readme-only` writes. Git merges two
regenerations of the block line by line, and each grid line holds four
examples, so a clean-looking merge once left one example in the grid six
times. After a merge that touches the block, run
`cargo run --release --example gallery -- --readme-only`.

## The `browser` column

The graded table's last column says whether CI's `web-gate` job grades the
example in headless Chrome (see "Grading in the browser" in
[`web/README.md`](../web/README.md#grading-in-the-browser)). The generator
derives it, `--readme-only` included, and it is never typed by hand:

| cell | when |
|---|---|
| `yes` | `web/manifests/<name>.json` exists and `tools/web_gate.skip` does not list the name |
| `no (<reason>)` | `tools/web_gate.skip` lists the name; the reason is the rest of its line |
| `not yet` | no manifest: the example is not on the Pages build, so the gate cannot see it |

A row added by hand may stop at the `triangles` cell; the generator appends
the sixth. The test `the_readme_browser_column_is_current` (run by
`cargo test -p three-rs --example gallery`, which CI runs) fails when a cell
disagrees with the manifests and the skip list, or when a manifest has no row
in the table. The fix is the same either way: add the row if it is missing,
then `cargo run --release --example gallery -- --readme-only`.

## One source of truth

The set of graded examples, and every number the gallery shows, come from the
README's "Examples graded green" table. The e2e harness writes images but no
machine-readable record — it *prints* the diff count, the steady frame and the
`renderer.info()` counts — so the README table is the list that is already
maintained per rung, and the generator parses it rather than keeping a second
one. The table's row order is the rung order, and the gallery keeps it. An
example in the table with no `target/e2e/<name>/actual.png` is named on the way
out and left out of the grid; run the ladder to include it.

## Ported but ungradeable

A page that three's own e2e harness skips has no reference that upstream
holds itself to. Those pages are the ones listed in `test/e2e/puppeteer.js`'s
exception list. Nor does a page that three.js itself renders more than the
limit away from its own reference on this machine, since a port that matched
three would fail the same way. Such a page is not a row in "Examples graded
green", so it gets no thumbnail here and no build in the browser shell, whose manifests are
exactly the graded set. The README lists it instead in a two-column table
under the graded one: the example, and why it has no grade. A page three
misses here that has no row there is only an `#[ignore]`d rung in
`tests/e2e/main.rs`; `docs/parity.md` counts them. The generator and its
tests read only five- and six-cell rows, so they skip that table.

| example | reason |
|---|---|
| `webgpu_postprocessing_traa` | three's exception list, "Black screen". Gated on its resolve shader and on `tests/traa_frames.rs` |
| `webgpu_postprocessing_dof` | three's exception list, "Black screen". Gated on all seven of `DepthOfFieldNode`'s distinct quad shaders against three's dump (`dof_*` in `tests/nodes_display_wgsl.rs`) |
| `webgpu_postprocessing_retro` | three.js misses its own reference here (1503 of 100000 pixels, limit 0.1%); the port scores the same. Gated on its two post-processing shaders and on `tests/retro_frames.rs` |
| `webgpu_postprocessing_ssr_denoise` | three's exception list, "Need more time to render". Gated on its scene material, SSR, denoise, grading and sharpen shaders against three's dump and on `tests/ssr_denoise_frames.rs` |

## Reference images stay out of the tree

Three's reference screenshots are not ours to redistribute, so nothing derived
from them is ever written into the repository. The committed thumbnails are
downscales of **our own** rendered frames (`target/e2e/<name>/actual.png`). The
only place a reference image appears is the local `target/gallery/index.html`,
which points at the vendored JPEGs by absolute `file://` path; `target/` is
git-ignored, so that page never leaves the machine that generated it. The
grader's diff count is the one number in the tree that the references touch,
and it is already in the README.

There is no second comparator here: the gallery reads numbers, it does not
measure any.
