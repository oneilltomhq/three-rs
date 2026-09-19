# Dumping Three's WGSL and GPU descriptors

Every rung starts the same way: pick one Three `webgpu_*` example, find out
exactly what Three's `WebGPURenderer` hands Chrome's WebGPU driver for it, and
port whatever's missing until the generated WGSL and pipeline/binding
descriptors match. `tools/dump-webgpu.mjs` is that capture step.

## Running it

```sh
node tools/dump-webgpu.mjs <example_name> [--out DIR]
```

`<example_name>` is a file under the vendor checkout's `examples/`, without
the `.html` (e.g. `webgpu_rtt`, `webgpu_lights_phong`). It needs a three.js
checkout at `$THREE_JS_DIR` (default `~/src/vendor/three.js`, the same
default `src/testing.rs::vendor_dir` and the e2e grader use) with `npm ci`
already run there — that's where Chrome, puppeteer-core and the example's own
assets come from. Nothing is installed or written into that checkout; the
tool serves it read-only on its own local port and launches Chrome with its
own profile directory under `target/`, so a concurrent grader run or another
worker's dump never collides with it. On this project's shared machine, run
it (and any other puppeteer/GPU invocation) through the lock:

```sh
flock /run/user/1000/three-rs-gpu.lock node tools/dump-webgpu.mjs webgpu_rtt
```

Output lands under `--out` (default `target/dumps/<example_name>/`):

| file | what |
|---|---|
| `dump.json` | every module, render/compute pipeline, bind-group layout, pipeline layout, buffer, texture, texture view, sampler and bind group the driver was asked to create, plus every render/compute pass and a flat chronological `order` log of every create/pass/draw/dispatch/submit call |
| `mNN_<stage>_<label>.wgsl` | one file per shader module, in creation order, named by its detected stage and Three's own `label` |
| `actual_full.png` | the raw 800×500 frame |
| `actual.jpg` | the same frame through three.js' own `test/e2e/image.js` `scale()` — byte-for-byte what the e2e grader compares against `examples/screenshots/<example>.jpg` |

The frame is pinned exactly as the grader pins it: `test/e2e/deterministic-injection.js`'s
seeded `Math.random`, frozen `performance.now`/`Date.now`, and single
deterministic `requestAnimationFrame`; `test/e2e/clean-page.js` removes the
on-page UI first; the page is loaded with `test/e2e/puppeteer.js`'s own
flags with `rung0/grader-flags.patch` applied (`--use-angle=vulkan`, no
`--disable-vulkan-surface`), waiting for `networkidle0` and then the one RAF.

Every created GPU object gets one id, in creation order, shared across every
object kind (so ids interleave — a module might be id 16, the next thing
created id 17, regardless of type). Anything that later refers back to an
object — a bind group's buffer or sampler, a pipeline's layout, a pass's
bound pipeline — is resolved to that id (and, where useful, the object's own
label) instead of being printed as an opaque handle.

## How a rung uses it

1. Dump the target example (and, when comparing a divergence, any example
   already known to match, the way rung 12 diffed its `outputColorTransform`
   module against rung 6's).
2. Port whatever's missing in `src/nodes` / `src/materials` until
   `examples/dump_wgsl.rs`'s printed WGSL for the equivalent material matches
   the dump's `mNN_*.wgsl` files line for line (module-by-module; see
   `docs/nodes.md` for the node system this drives).
3. Confirm with the e2e grader (`cargo test --test e2e -- --nocapture`) that
   the example passes.
4. If a generated `.wgsl` fixture is worth pinning in a unit test (a node
   chain the pixel ladder alone wouldn't catch a regression in — see
   `tests/nodes_mx_noise.rs`), copy only the files that test needs into
   `tests/fixtures/<example>/`, not the whole dump — `target/dumps/` itself
   is scratch and is never committed (issue #70).

Nothing under `target/dumps/` is committed; regenerate it with the tool
whenever you need to re-check a divergence.
