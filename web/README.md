# The graded examples, in a browser

This crate is the third driver of the ported examples. `tests/e2e/main.rs`
renders one graded frame and hands it to three.js' comparator;
`src/bin/viewer.rs` runs them in a window; this runs them on a `<canvas>`, on
the browser's own WebGPU, from the same `init()`. The examples are included as
modules, not reimplemented — see `src/shell.rs` for the flow.

The page shows the graded frame first, at the example's own 800x500 with both
clocks pinned to 0, exactly as the ladder renders it. Then it goes live: the
canvas is sized to the window the way the three.js pages size theirs, and
`requestAnimationFrame` calls the example's own `animate()` once a frame. The
status line carries the frame rate.

Drag to orbit, right-drag to pan, the wheel to dolly, the arrow keys to pan —
on the 27 examples whose three.js page creates an `OrbitControls`. There is no
controls implementation in this crate: the canvas' pointer, wheel and key
events are translated into the value types `three_rs::addons::controls`
defines and handed to the example's own controls, which is the same adapter
the viewer writes over winit.

Part of [issue #128](https://github.com/oneilltomhq/three-rs/issues/128).

**Live:** <https://oneilltomhq.github.io/three-rs/>, deployed from `main` by
`.github/workflows/pages.yml` (which runs `web/build.sh` and publishes
`web/dist/`). The README gallery's thumbnails link there, one
`?example=<name>` page each. Every path the shell loads is relative, so it
works under the `/three-rs/` sub-path as it does at a server's root; keep it
that way.

## Build

    web/build.sh

That produces `web/dist/` (git-ignored): the `.wasm`, wasm-bindgen's JS glue
and `index.html`. Two prerequisites:

    # the wasm target — with rustup:
    rustup target add wasm32-unknown-unknown
    # on Fedora without rustup:
    sudo dnf install -y rust-std-static-wasm32-unknown-unknown

    # the wasm-bindgen CLI, at the exact version Cargo.lock pins for the crate
    # (they are one program in two halves; build.sh checks and tells you the
    # command if they disagree)
    cargo install wasm-bindgen-cli --version <version> --locked

`web/build.sh --debug` builds an unoptimised `.wasm` — far larger, and worth it
when a panic trace matters.

## Serve

WebGPU needs a secure context, so `file://` will not do; `localhost` counts as
secure, so a plain static server is enough:

    python3 -m http.server --directory web/dist 8000

Then <http://localhost:8000/> for the index, or
<http://localhost:8000/?example=webgpu_loader_gltf> for one example; add
`&hold` to stay on the graded frame (see "Grading in the browser").

Browsers: Chrome and Edge ship WebGPU. Firefox and Safari mostly do not yet —
those get the still frame from the README's gallery and a line saying why,
rather than a blank canvas.

## Grading in the browser

`tools/web_gate.mjs` is the ladder again, in headless Chrome: every graded
example (one per committed manifest) is opened as
`?example=<name>&hold`, which stops the page on the graded frame — no resize,
no animation loop, no listeners — reads the renderer's 800x500 canvas texture
back and publishes it as `window.__three_rs_graded`, then sets
`<body data-graded>` (or `<body data-error>` on any failure, a panic
included). The gate writes those pixels to `target/web-gate/<name>/actual.png`
and runs `src/testing/compare.mjs` on them, the very script
`tests/e2e/main.rs` runs natively: three.js' own `test/e2e/image.js`,
unmodified, against `examples/screenshots/<name>.jpg`, at Three's threshold.

    web/build.sh
    node tools/web_gate.mjs                      # every graded example
    node tools/web_gate.mjs webgpu_rtt webgpu_mrt  # some of them

It needs the three.js r186 checkout at `$THREE_JS_DIR` (default
`~/src/vendor/three.js`) with `npm ci` done: puppeteer-core, pngjs, `image.js`
and the screenshots come from there, and so does Chrome (the one puppeteer
downloads; `--chrome PATH` or `$CHROME` picks another). The assets the page
would fetch from GitHub are answered from that checkout instead, so the gate
needs no network. It prints a table (name, different pixels, verdict, ms),
writes `target/web-gate/summary.json`, and exits non-zero if an example fails
that `tools/web_gate.skip` does not list. CI runs it as the `web-gate` job, the
one pixel gate CI can run without a GPU.

The adapter is software on purpose: Dawn on SwiftShader's Vulkan, which Chrome
ships, selected with `--enable-unsafe-webgpu --enable-features=Vulkan
--use-angle=swiftshader --use-vulkan=swiftshader --ignore-gpu-blocklist` in
new headless (`--headless=new`, puppeteer's default). On a desk with a GPU
those flags still hand the page the `google`/`swiftshader` fallback adapter
rather than the hardware, so the desk and CI grade the same renderer;
`--hardware` swaps in `test/e2e/puppeteer.js`' own flags to grade the desk's
GPU instead.

**The finding** (#128 asked whether the web gate can share the native
references or needs its own): it shares them. On SwiftShader, all 39 graded
examples pass Three's own screenshots at Three's own threshold, with no skip
list: the worst is `webgpu_mrt` at 0.08% different pixels against a 0.1%
limit, then `webgpu_deferred` at 0.06%; 28 of the 39 round to 0.00%.

## Where the assets come from

Nothing of three.js' is committed here, the same promise `examples/gallery.rs`
keeps for the gallery thumbnails. Each example's assets are listed in
`web/manifests/<example>.json` as paths relative to a three.js checkout, and
the page fetches them at run time from the pinned `r186` tag on
`raw.githubusercontent.com`.

Those manifests are generated, never hand-written:

    cargo run --release --example web_manifests            # rewrite them
    cargo run --release --example web_manifests -- --check # fail if stale

The generator switches on the I/O seam's recorder (`three_rs::io`), runs each
graded example's `init()` natively, and writes down exactly what the loaders
asked for. It needs a GPU and a three.js checkout, as the gallery generator
does. The gate CI can run is the `#[test]`s in `examples/web_manifests.rs`,
which need neither and check that the committed set of manifests is exactly the
README's set of graded examples.

## Adding an example

1. it has to be in the README's "Examples graded green" table;
2. add it to `GRADED` in `examples/web_manifests.rs` and re-run the generator;
3. add a row to the `examples!` table in `web/src/shell.rs` and a name to the
   `EXAMPLES` array in `web/index.html`.

Steps 2 and 3 are hand-maintained lists because a `#[path]` attribute takes a
string literal and cannot be generated from one; the `#[test]`s in step 2 fail
until step 1 and step 2 agree.
