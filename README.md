# three-rs

[![CI](https://github.com/oneilltomhq/three-rs/actions/workflows/ci.yml/badge.svg)](https://github.com/oneilltomhq/three-rs/actions/workflows/ci.yml)

A port of [three.js](https://github.com/mrdoob/three.js) core and its
`WebGPURenderer` to Rust on [wgpu](https://github.com/gfx-rs/wgpu), graded by
Three's own end-to-end pixel comparison: each ported example renders headless
and is diffed against the reference screenshot Three ships for that example,
using Three's unmodified `test/e2e/image.js` comparator.

Two things distinguish it from other three.js-shaped Rust crates (including
the similarly named `threers`): the shaders are not hand-written but generated
from Three's node graph (TSL) by a port of Three's `NodeBuilder`, the way
`WebGPURenderer` does it, so Three's own WGSL dumps are the reference; and
correctness is judged by Three's own examples and reference screenshots, not
by scenes written for the port.

**Status: early, working, incomplete.** Ten of Three's `webgpu_*` examples
pass the grader; the vast majority of Three's 600-odd examples have not been
attempted. The API follows Three's object model but is not stable. Vulkan on
Linux is the only backend that has been run.

## What is ported

- **Math and core.** Vector/Matrix/Quaternion/Euler/Color and the geometric
  helpers (Box, Sphere, Plane, Ray, Frustum, Triangle, ...), `Object3D` scene
  graph, `BufferGeometry` with named attributes, groups, draw ranges and morph
  attributes, layers, cameras. Verified against Three's QUnit tests.
- **Geometries.** Box, Plane, Cylinder, Cone, Torus, the polyhedra, Circle,
  Ring, Lathe, Capsule, plus the Teapot and RoundedBox addons, bit-exact
  against samples generated from Three.
- **Node system (TSL).** A `NodeBuilder` that generates both WGSL stages and the
  bind group layout from a node graph, following Three's `nodes/` and
  `renderers/webgpu/nodes/`. Generated WGSL is kept structurally identical to
  Three's dumps; deliberate divergences are listed in `docs/nodes.md`.
- **Materials.** `MeshBasic`, `MeshPhong`, `MeshStandard` (physical lighting
  model with DFG LUT), `Sprite`, `LineBasic`, and the shadow-pass material.
  Bump maps, env maps, blending modes, tone mapping.
- **Lights and shadows.** Ambient, Hemisphere, Point, Spot, Directional;
  planar and cube shadow maps with Three's Vogel-disk filter.
- **Renderer.** Render lists, instancing, morph targets, render targets,
  MSAA, the linear-to-sRGB output pass, `PassNode` post-processing, mipmaps,
  cube textures, line topology.
- **Loaders.** glTF/GLB (all accessor types, skins, animations, KHR specular
  and ior), textures (PNG, JPEG), cube textures.
- **Animation.** Interpolants, keyframe tracks, clips, `PropertyMixer`,
  `AnimationAction` and `AnimationMixer`.
- **Workspace crate.** `sdf-text`: signed-distance-field text rendering with
  a `BatchedText` object (ttf-parser outlines, analytic rasteriser), and the
  SDF text examples with their gates. Its `d33_treemap_labels` example lays
  its treemap out with [d3-hierarchy](https://github.com/oneilltomhq/d3-hierarchy),
  a port of d3-hierarchy 3.1.2 that began in this repository.

## Examples graded green

| Three example | different pixels (of 100000) | steady frame (ms) | draw calls | triangles |
|---|---|---|---|---|
| webgpu_depth_texture | 0 | 11.0 | 43 | 671746 |
| webgpu_instance_mesh | 60 (Three itself scores 60 against the same JPEG) | 9.3 | 2 | 967001 |
| webgpu_materials_basic | 0 | 16.1 | 118 | 113345 |
| webgpu_rtt | 1 | 2.3 | 3 | 14 |
| webgpu_lights_phong | 31 | 4.3 | 5 | 62001 |
| webgpu_morphtargets | 0 | 2.8 | 2 | 12289 |
| webgpu_shadowmap | 7 | 7.7 | 19 | 39399 |
| webgpu_lights_physical | 4 | 4.2 | 11 | 4267 |
| webgpu_postprocessing_masking | 18 | 1.0 | 3 | 1037 |
| webgpu_tsl_galaxy | 40 | 5.3 | 2 | 40001 |

Measured on Intel Iris Xe, Mesa 25.3.6, Fedora 43, against three.js r186.
Other GPUs and drivers will land somewhere else on the pass threshold; the
threshold is Three's own (0.1% of pixels).

The steady frame is the whole cost of a frame once the first has built and
uploaded everything — the example's `animate()` through to the GPU finishing
it, at 800x500, mean of 30 frames after a 10-frame warm-up in a release build
(`viewer <example> --headless --frames 40`, below). The same machine caveat
applies. The e2e grader renders three more frames after the graded one and
fails a rung whose steady frame is over a ceiling set well above these
(`STEADY_FRAME_CEILING` in `tests/e2e/main.rs`), so a per-frame cost the single
graded frame cannot see fails the ladder.

The last two columns are `renderer.info()` for the whole graded frame — draw
calls and the triangles they drew, instancing multiplied in (`webgpu_rtt`'s 14
is a cube and two full-screen quads; `webgpu_instance_mesh`'s million is one
instanced draw). Unlike the time, these are exact: the same grader asserts
that frames two and three of every rung compile, build and upload *nothing*,
as an equality, which is what catches a regression that re-uploads a live
geometry every frame while staying well under a time ceiling. `Info` has the
rest of the counts (pipelines, geometries, attribute buffers, textures, and
the resident totals).

## Building

(For how to contribute, what is in scope, and the gates a change has to pass,
see `CONTRIBUTING.md`.)

Requires a Rust toolchain (1.90 or newer) and a Vulkan driver. `wgpu` comes
from crates.io, pinned to `30.0.1` in `Cargo.toml`; nothing else is unusual.

```sh
cargo build --release
cargo test -p sdf-text --lib                # sdf-text unit tests; no GPU
cargo test -p three-rs --lib                # three-rs unit tests; no GPU
```

The rest of `cargo test --workspace` needs a GPU (the renderer tests and the
SDF text gates) and, for the e2e grader, the three.js checkout described
next. The e2e tests serialise themselves on the one GPU; no `--test-threads`
flag is needed for them, but `sdf-text`'s three GPU gates still want
`-- --test-threads=1`.

### The viewer

```sh
cargo run --release --bin viewer -- --list
cargo run --release --bin viewer -- webgpu_lights_physical
cargo run --release --bin viewer -- 8                        # the same, by key
cargo run --release --bin viewer -- shadowmap --headless --frames 40
```

Opens the named example in a window (winit, tested on Wayland) with orbit,
zoom and pan. All ten graded examples are there; `--list` prints them with
their keys (`1`-`9`, `0`), which switch examples in the window and stand in
for the name on the command line. The window prints one line a second with
the frame rate and the steady-state render time (mean and max over the last
60 frames, after a 10-frame warm-up):

```text
webgpu_lights_phong — 1000x625 — 59.9 fps — render mean 1.61 ms max 1.79 ms (last 60 frames, after 10 warm-up)
```

`--headless --frames N` renders N frames with no window and reports the same
numbers for the whole frame, CPU and GPU, which is how the table above was
measured; add `--screenshot out.png` to keep the last frame. It also writes
`target/e2e/<example>/strip.png`: three more frames tiled left to right at
1:1, each with its draw calls and its build counts in the gutter under it, so
the count ladder sits beside the time one. The e2e harness writes the same
strip per rung (`steady-strip.png`) and one for the geometry-mutation case.

### Controls

`three_rs::controls::Helicopter` flies a camera over a `Ground`: a sphere whose
north pole is the world origin, so `R = 1e7` is a plane and a small R a planet,
with no separate case for either. The camera never rolls — `up` is the ground
normal — and every field is damped with `math_utils::smooth_damp`, a port of
camera-controls' `smoothDamp`.

`cargo run --release --bin heli` is the demo: **W A S D** fly, **Q** / **E** and
the wheel climb, either mouse button drags the view round, **[** and **]** curl
the ground up and flatten it, **P** snaps it flat, **Home** resets the pose,
**Tab** is the overview (click a pane there to drop onto it) and **Esc** quits.

```sh
cargo run --release --bin heli -- --headless shots/heli-sphere-high.png \
    --radius 300 --pose 0,-120,600,0,-60 [--overview] [--size 1600x1000]
```

### Running the examples and the e2e grader

The examples load their textures and models from Three's own `examples/`
directory, and the grader reads Three's reference screenshots and runs its
comparator under node. So they need a three.js checkout:

```sh
git clone --branch r186 --depth 1 https://github.com/mrdoob/three.js ~/src/vendor/three.js
(cd ~/src/vendor/three.js && npm ci)
export THREE_JS_DIR=~/src/vendor/three.js     # this is the default location
cargo test --test e2e -- --nocapture
```

Each e2e test writes `actual.png`, the reference, and a diff strip under
`target/e2e/<example>/`. The grader is unmodified; `rung0/grader-flags.patch`
is only needed to run Three's *own* Chrome-based e2e suite on Linux with a
real Vulkan adapter, which is how the reference numbers were calibrated
(`rung0/RUNG0.md`).

One more checkout is optional:

| env var | default | needed by |
|---|---|---|
| `D3_GALLERY_DIR` | `~/src/vendor/d3-gallery` | `sdf-text`'s `d33_treemap_labels` example (its `flare.json`) |

Tests that need a checkout that is missing fail on the open with the path
they looked for.

## Layout

```
src/            the three-rs crate, mirroring three.js's src/ tree
  math core cameras geometries lights loaders materials objects textures animation
  controls/     the helicopter camera and its ground (not a three.js port)
  nodes/        TSL nodes and the WGSL NodeBuilder
  renderer/     the wgpu backend: pipelines, bindings, passes, shadows, present
  bin/viewer.rs bin/heli.rs
examples/       one file per ported Three example, also compiled into tests/e2e
tests/          Three's QUnit tests ported per module, plus the e2e harness
sdf-text/       workspace crate: SDF text and BatchedText, with its examples and gates
docs/           design notes per subsystem and per-example progress logs
shots/          headless frames from bin/heli.rs, one per ground and pose
rung0/          how the grader was calibrated
```

`docs/nodes.md` and `docs/scene-graph.md` are the two to read first: how the
node system maps onto Three's, and how the `Rc<RefCell<Object3D>>` scene graph
replaces Three's prototype tree.

## How it was built

Example by example. Each "rung" takes one Three example, dumps the WGSL Three
generates for it, ports whatever the example needs until the generated WGSL
matches and the grader passes, then merges. The port was directed and largely
written by Claude agents with the pixel diff as the ground truth; the
per-rung progress notes in `docs/` record what each rung found, including the
places where Three's own output was reproduced bug-for-bug and the places
where it deliberately was not.

## License

MIT. See `LICENSE`, which also carries the three.js (MIT) notice this port
derives from.
