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
  cube textures, line topology, viewport / scissor / `clearDepth` and
  `autoClear`.
- **Addons.** `src/addons/` holds the `three/addons/…` tier that needs a core
  change to work: `lines` (`LineSegmentsGeometry`, `LineGeometry`,
  `LineSegments2`, `Line2` — fat lines, with `Line2NodeMaterial` in core beside
  them, as three.js ships it) and `geometry_utils`. An addon that needs nothing
  from core is a workspace crate instead — `addons/controls` — and that stays
  the preferred shape; these live in the root crate because the e2e harness
  pulls examples in with `#[path = "../../examples/…"]`, and an example in
  another crate would need its own test binary.
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

<!-- gallery:start -->
| | | | |
| --- | --- | --- | --- |
| [<img src="https://raw.githubusercontent.com/oneilltomhq/three-rs/main/docs/gallery/webgpu_depth_texture.jpg" alt="webgpu_depth_texture" width="200">](https://github.com/oneilltomhq/three-rs/blob/main/examples/webgpu_depth_texture.rs) | [<img src="https://raw.githubusercontent.com/oneilltomhq/three-rs/main/docs/gallery/webgpu_instance_mesh.jpg" alt="webgpu_instance_mesh" width="200">](https://github.com/oneilltomhq/three-rs/blob/main/examples/webgpu_instance_mesh.rs) | [<img src="https://raw.githubusercontent.com/oneilltomhq/three-rs/main/docs/gallery/webgpu_materials_basic.jpg" alt="webgpu_materials_basic" width="200">](https://github.com/oneilltomhq/three-rs/blob/main/examples/webgpu_materials_basic.rs) | [<img src="https://raw.githubusercontent.com/oneilltomhq/three-rs/main/docs/gallery/webgpu_rtt.jpg" alt="webgpu_rtt" width="200">](https://github.com/oneilltomhq/three-rs/blob/main/examples/webgpu_rtt.rs) |
| [`webgpu_depth_texture`](https://github.com/oneilltomhq/three-rs/blob/main/examples/webgpu_depth_texture.rs) | [`webgpu_instance_mesh`](https://github.com/oneilltomhq/three-rs/blob/main/examples/webgpu_instance_mesh.rs) | [`webgpu_materials_basic`](https://github.com/oneilltomhq/three-rs/blob/main/examples/webgpu_materials_basic.rs) | [`webgpu_rtt`](https://github.com/oneilltomhq/three-rs/blob/main/examples/webgpu_rtt.rs) |
| [<img src="https://raw.githubusercontent.com/oneilltomhq/three-rs/main/docs/gallery/webgpu_lights_phong.jpg" alt="webgpu_lights_phong" width="200">](https://github.com/oneilltomhq/three-rs/blob/main/docs/rung5-progress.md) | [<img src="https://raw.githubusercontent.com/oneilltomhq/three-rs/main/docs/gallery/webgpu_morphtargets.jpg" alt="webgpu_morphtargets" width="200">](https://github.com/oneilltomhq/three-rs/blob/main/docs/rung6-progress.md) | [<img src="https://raw.githubusercontent.com/oneilltomhq/three-rs/main/docs/gallery/webgpu_shadowmap.jpg" alt="webgpu_shadowmap" width="200">](https://github.com/oneilltomhq/three-rs/blob/main/docs/rung7-progress.md) | [<img src="https://raw.githubusercontent.com/oneilltomhq/three-rs/main/docs/gallery/webgpu_lights_physical.jpg" alt="webgpu_lights_physical" width="200">](https://github.com/oneilltomhq/three-rs/blob/main/docs/rung8-progress.md) |
| [`webgpu_lights_phong`](https://github.com/oneilltomhq/three-rs/blob/main/examples/webgpu_lights_phong.rs) | [`webgpu_morphtargets`](https://github.com/oneilltomhq/three-rs/blob/main/examples/webgpu_morphtargets.rs) | [`webgpu_shadowmap`](https://github.com/oneilltomhq/three-rs/blob/main/examples/webgpu_shadowmap.rs) | [`webgpu_lights_physical`](https://github.com/oneilltomhq/three-rs/blob/main/examples/webgpu_lights_physical.rs) |
| [<img src="https://raw.githubusercontent.com/oneilltomhq/three-rs/main/docs/gallery/webgpu_postprocessing_masking.jpg" alt="webgpu_postprocessing_masking" width="200">](https://github.com/oneilltomhq/three-rs/blob/main/docs/rung9-progress.md) | [<img src="https://raw.githubusercontent.com/oneilltomhq/three-rs/main/docs/gallery/webgpu_tsl_galaxy.jpg" alt="webgpu_tsl_galaxy" width="200">](https://github.com/oneilltomhq/three-rs/blob/main/docs/rung13-progress.md) | [<img src="https://raw.githubusercontent.com/oneilltomhq/three-rs/main/docs/gallery/webgpu_skinning.jpg" alt="webgpu_skinning" width="200">](https://github.com/oneilltomhq/three-rs/blob/main/docs/rung10-progress.md) | [<img src="https://raw.githubusercontent.com/oneilltomhq/three-rs/main/docs/gallery/webgpu_mesh_batch.jpg" alt="webgpu_mesh_batch" width="200">](https://github.com/oneilltomhq/three-rs/blob/main/docs/rung11-progress.md) |
| [`webgpu_postprocessing_masking`](https://github.com/oneilltomhq/three-rs/blob/main/examples/webgpu_postprocessing_masking.rs) | [`webgpu_tsl_galaxy`](https://github.com/oneilltomhq/three-rs/blob/main/examples/webgpu_tsl_galaxy.rs) | [`webgpu_skinning`](https://github.com/oneilltomhq/three-rs/blob/main/examples/webgpu_skinning.rs) | [`webgpu_mesh_batch`](https://github.com/oneilltomhq/three-rs/blob/main/examples/webgpu_mesh_batch.rs) |
| [<img src="https://raw.githubusercontent.com/oneilltomhq/three-rs/main/docs/gallery/webgpu_postprocessing_radial_blur.jpg" alt="webgpu_postprocessing_radial_blur" width="200">](https://github.com/oneilltomhq/three-rs/blob/main/docs/webgpu_postprocessing_radial_blur-progress.md) | [<img src="https://raw.githubusercontent.com/oneilltomhq/three-rs/main/docs/gallery/webgpu_materials.jpg" alt="webgpu_materials" width="200">](https://github.com/oneilltomhq/three-rs/blob/main/docs/webgpu_materials-progress.md) | [<img src="https://raw.githubusercontent.com/oneilltomhq/three-rs/main/docs/gallery/webgpu_postprocessing_ssaa.jpg" alt="webgpu_postprocessing_ssaa" width="200">](https://github.com/oneilltomhq/three-rs/blob/main/docs/webgpu_postprocessing_ssaa-progress.md) | [<img src="https://raw.githubusercontent.com/oneilltomhq/three-rs/main/docs/gallery/webgpu_pmrem_cubemap.jpg" alt="webgpu_pmrem_cubemap" width="200">](https://github.com/oneilltomhq/three-rs/blob/main/docs/webgpu_pmrem_cubemap-progress.md) |
| [`webgpu_postprocessing_radial_blur`](https://github.com/oneilltomhq/three-rs/blob/main/examples/webgpu_postprocessing_radial_blur.rs) | [`webgpu_materials`](https://github.com/oneilltomhq/three-rs/blob/main/examples/webgpu_materials.rs) | [`webgpu_postprocessing_ssaa`](https://github.com/oneilltomhq/three-rs/blob/main/examples/webgpu_postprocessing_ssaa.rs) | [`webgpu_pmrem_cubemap`](https://github.com/oneilltomhq/three-rs/blob/main/examples/webgpu_pmrem_cubemap.rs) |
| [<img src="https://raw.githubusercontent.com/oneilltomhq/three-rs/main/docs/gallery/webgpu_postprocessing_bloom_selective.jpg" alt="webgpu_postprocessing_bloom_selective" width="200">](https://github.com/oneilltomhq/three-rs/blob/main/docs/webgpu_postprocessing_bloom_selective-progress.md) | [<img src="https://raw.githubusercontent.com/oneilltomhq/three-rs/main/docs/gallery/webgpu_compute_points.jpg" alt="webgpu_compute_points" width="200">](https://github.com/oneilltomhq/three-rs/blob/main/docs/rung12-progress.md) | [<img src="https://raw.githubusercontent.com/oneilltomhq/three-rs/main/docs/gallery/webgpu_lines_fat.jpg" alt="webgpu_lines_fat" width="200">](https://github.com/oneilltomhq/three-rs/blob/main/docs/webgpu_lines_fat-progress.md) | [<img src="https://raw.githubusercontent.com/oneilltomhq/three-rs/main/docs/gallery/webgpu_pmrem_test.jpg" alt="webgpu_pmrem_test" width="200">](https://github.com/oneilltomhq/three-rs/blob/main/docs/webgpu_pmrem_test-progress.md) |
| [`webgpu_postprocessing_bloom_selective`](https://github.com/oneilltomhq/three-rs/blob/main/examples/webgpu_postprocessing_bloom_selective.rs) | [`webgpu_compute_points`](https://github.com/oneilltomhq/three-rs/blob/main/examples/webgpu_compute_points.rs) | [`webgpu_lines_fat`](https://github.com/oneilltomhq/three-rs/blob/main/examples/webgpu_lines_fat.rs) | [`webgpu_pmrem_test`](https://github.com/oneilltomhq/three-rs/blob/main/examples/webgpu_pmrem_test.rs) |
| [<img src="https://raw.githubusercontent.com/oneilltomhq/three-rs/main/docs/gallery/webgpu_postprocessing_difference.jpg" alt="webgpu_postprocessing_difference" width="200">](https://github.com/oneilltomhq/three-rs/blob/main/docs/webgpu_postprocessing_difference-progress.md) | [<img src="https://raw.githubusercontent.com/oneilltomhq/three-rs/main/docs/gallery/webgpu_postprocessing_direct.jpg" alt="webgpu_postprocessing_direct" width="200">](https://github.com/oneilltomhq/three-rs/blob/main/docs/webgpu_postprocessing_direct-progress.md) | [<img src="https://raw.githubusercontent.com/oneilltomhq/three-rs/main/docs/gallery/webgpu_furnace_test.jpg" alt="webgpu_furnace_test" width="200">](https://github.com/oneilltomhq/three-rs/blob/main/docs/webgpu_furnace_test-progress.md) | [<img src="https://raw.githubusercontent.com/oneilltomhq/three-rs/main/docs/gallery/webgpu_postprocessing_anamorphic.jpg" alt="webgpu_postprocessing_anamorphic" width="200">](https://github.com/oneilltomhq/three-rs/blob/main/docs/webgpu_postprocessing_anamorphic-progress.md) |
| [`webgpu_postprocessing_difference`](https://github.com/oneilltomhq/three-rs/blob/main/examples/webgpu_postprocessing_difference.rs) | [`webgpu_postprocessing_direct`](https://github.com/oneilltomhq/three-rs/blob/main/examples/webgpu_postprocessing_direct.rs) | [`webgpu_furnace_test`](https://github.com/oneilltomhq/three-rs/blob/main/examples/webgpu_furnace_test.rs) | [`webgpu_postprocessing_anamorphic`](https://github.com/oneilltomhq/three-rs/blob/main/examples/webgpu_postprocessing_anamorphic.rs) |
| [<img src="https://raw.githubusercontent.com/oneilltomhq/three-rs/main/docs/gallery/webgpu_pmrem_scene.jpg" alt="webgpu_pmrem_scene" width="200">](https://github.com/oneilltomhq/three-rs/blob/main/docs/webgpu_pmrem_scene-progress.md) |  |  |  |
| [`webgpu_pmrem_scene`](https://github.com/oneilltomhq/three-rs/blob/main/examples/webgpu_pmrem_scene.rs) |  |  |  |

<sub>Our own rendered frames, one per graded example. See [`docs/gallery.md`](https://github.com/oneilltomhq/three-rs/blob/main/docs/gallery.md).</sub>
<!-- gallery:end -->
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
| webgpu_skinning | 6 | 3.0 | 3 | 30091 |
| webgpu_mesh_batch | 0 | 2.5 | 454 | 44341 |
| webgpu_postprocessing_radial_blur | 7 | 3.9 | 2 | 401 |
| webgpu_materials | 44 | 7.3 | 19 | 350065 |
| webgpu_postprocessing_ssaa | 0 | 11.9 | 17 | 2119689 |
| webgpu_pmrem_cubemap | 0 | 8.2 | 32 | 243905 |
| webgpu_postprocessing_bloom_selective | 1 | 16.5 | 63 | 256013 |
| webgpu_compute_points | 4 (see below) | 10.7 | 2 | 1 + 300000 points |
| webgpu_lines_fat | 0 | 3.8 | 6 | 11191 |
| webgpu_pmrem_test | 27 | 6.1 | 35 | 67457 |
| webgpu_postprocessing_difference | 13 | 1.5 | 2 | 13 |
| webgpu_postprocessing_direct | 21 | 4.9 | 93 | 4192 |
| webgpu_furnace_test | 0 | 7.1 | 122 | 116161 |
| webgpu_postprocessing_anamorphic | 2 | 8.1 | 16 | 398798 |
| webgpu_pmrem_scene | 0 | 3.8 | 9 | 58433 |
| webgpu_postprocessing_bloom | 0 | 8.6 | 19 | 52085 |

`webgpu_compute_points` is graded like the rest and its 4 pixels mean less
than the rest: its frame is black apart from a 2x2 block at the centre, so
Three's comparator would pass it at 0.0% even if the compute passes never ran.
That rung is gated on the WGSL its kernels compile to and on reading the
storage buffers back — `docs/rung12-progress.md` says why and what the tests
assert.

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
zoom and pan. All the graded examples but `webgpu_compute_points` are there;
`--list` prints them with their keys (`1`-`9`, `0`, then letters), which
switch examples in the window and stand in for the name on the command
line. The window prints one line a second with
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

### Addons

three.js keeps its controls, loaders and helpers out of core, in
`examples/jsm/` — exported as `three/addons/*`, importing from `three` like any
user, and without core's stability promise. `addons/` is the same tier here:
workspace crates that depend on `three-rs` and are not ports of anything in
three.js' `src/`.

`addons/controls/` is `three-rs-controls`: a `MapControls` in the spirit of
three.js' addon of that name, over a `Ground` — a sphere whose north pole is the
world origin, so `R = 1e7` is a plane and a small R a planet, with no separate
case. The orbit target is a point *on* the ground, the camera never rolls —
`up` is the ground normal — and every field is damped with `smooth_damp`, a port
of camera-controls' `smoothDamp`.

`cargo run --release -p three-rs-controls --bin heli` is the demo, with its key
map on screen: **left-drag** grabs the ground,
**right-drag** (or **ctrl**-drag) orbits, the **wheel** zooms to the pointer,
the **arrows** pan, **[** **]** curl the ground and **P** flattens it, **Home**
resets, **Tab** is the overview (click a pane to drop onto it), **Esc** quits.

```sh
cargo run --release -p three-rs-controls --bin heli -- \
    --headless addons/controls/shots/heli-sphere-high.png \
    --radius 300 --pose 0,0,800,0,20 [--overview] [--size 1600x1000] [--no-legend]
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
  nodes/        TSL nodes and the WGSL NodeBuilder
  renderer/     the wgpu backend: pipelines, bindings, passes, shadows, present
  bin/viewer.rs
examples/       one file per ported Three example, also compiled into tests/e2e
tests/          Three's QUnit tests ported per module, plus the e2e harness
sdf-text/       workspace crate: SDF text and BatchedText, with its examples and gates
addons/         workspace crates in the role of three.js' examples/jsm: not ports
  controls/     three-rs-controls — the map camera, its ground, and the heli demo
docs/           design notes per subsystem and per-example progress logs
rung0/          how the grader was calibrated
tools/          dump-webgpu.mjs, the rung dump hook (see docs/dumping.md)
```

`docs/nodes.md` and `docs/scene-graph.md` are the two to read first: how the
node system maps onto Three's, and how the `Rc<RefCell<Object3D>>` scene graph
replaces Three's prototype tree. `docs/dumping.md` says how a rung captures
the WGSL and GPU descriptors it ports against.

## How it was built

Example by example. Each "rung" takes one Three example, dumps the WGSL Three
generates for it (`tools/dump-webgpu.mjs`; see `docs/dumping.md`), ports whatever the example needs until the generated WGSL
matches and the grader passes, then merges. The port was directed and largely
written by Claude agents with the pixel diff as the ground truth; the
per-rung progress notes in `docs/` record what each rung found, including the
places where Three's own output was reproduced bug-for-bug and the places
where it deliberately was not.

## License

MIT. See `LICENSE`, which also carries the three.js (MIT) notice this port
derives from.
