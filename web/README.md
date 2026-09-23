# The graded examples, in a browser

This crate is the third driver of the ported examples. `tests/e2e/main.rs`
renders one graded frame and hands it to three.js' comparator;
`src/bin/viewer.rs` renders them in a window on the wall clock; this renders
the same graded frame onto a `<canvas>`, on the browser's own WebGPU, from the
same `init()`. The examples are included as modules, not reimplemented — see
`src/shell.rs` for the flow.

Part of [issue #128](https://github.com/oneilltomhq/three-rs/issues/128). This
first pass shows the *static* graded frame; the animation loop the viewer
already has is the next item there.

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
<http://localhost:8000/?example=webgpu_loader_gltf> for one example.

Browsers: Chrome and Edge ship WebGPU. Firefox and Safari mostly do not yet —
those get the still frame from the README's gallery and a line saying why,
rather than a blank canvas.

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
3. add it to `EXAMPLES` in `web/src/shell.rs` and to the `EXAMPLES` array in
   `web/index.html`.

Steps 2 and 3 are hand-maintained lists because a `#[path]` attribute takes a
string literal and cannot be generated from one; the `#[test]`s in step 2 fail
until step 1 and step 2 agree.
