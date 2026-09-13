# three-rs

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
- **Workspace crates.** `sdf-text`: signed-distance-field text rendering with
  a `BatchedText` object (ttf-parser outlines, analytic rasteriser). `d3/hierarchy`:
  a port of d3-hierarchy 3.1.2 (cluster, tree, partition, pack, treemap,
  stratify).

## Examples graded green

| Three example | different pixels (of 100000) |
|---|---|
| webgpu_depth_texture | 0 |
| webgpu_instance_mesh | 60 (Three itself scores 60 against the same JPEG) |
| webgpu_materials_basic | 0 |
| webgpu_rtt | 1 |
| webgpu_lights_phong | 31 |
| webgpu_morphtargets | 0 |
| webgpu_shadowmap | 7 |
| webgpu_lights_physical | 4 |
| webgpu_postprocessing_masking | 18 |
| webgpu_tsl_galaxy | 40 |

Measured on Intel Iris Xe, Mesa 25.3.6, Fedora 43, against three.js r186.
Other GPUs and drivers will land somewhere else on the pass threshold; the
threshold is Three's own (0.1% of pixels).

## Building

Requires a Rust toolchain (1.87 or newer) and a Vulkan driver. `wgpu` comes
from crates.io, pinned to `30.0.1` in `Cargo.toml`; nothing else is unusual.

```sh
cargo build --release
cargo test --workspace --exclude three-rs   # the sdf-text and d3-hierarchy crates; no GPU
cargo test -p three-rs --lib                # three-rs unit tests; no GPU
```

The rest of `cargo test --workspace` needs a GPU (the renderer tests) and,
for the e2e grader, the three.js checkout described next. The e2e tests
serialise themselves on the one GPU; no `--test-threads` flag is needed.

### The viewer

```sh
cargo run --release --bin viewer -- webgpu_lights_physical
```

Opens the named example in a window (winit, tested on Wayland) with orbit,
zoom and pan.

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

Two more checkouts are optional:

| env var | default | needed by |
|---|---|---|
| `D3_HIERARCHY_DIR` | `~/src/vendor/d3-hierarchy` | the d3-hierarchy crate's tests (its `test/data` fixtures) |
| `D3_GALLERY_DIR` | `~/src/vendor/d3-gallery` | the `d33_treemap_labels` example (its `flare.json`) |

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
sdf-text/       workspace crate: SDF text and BatchedText
d3/hierarchy/   workspace crate: d3-hierarchy port
docs/           design notes per subsystem and per-example progress logs
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

MIT. See `LICENSE`, which also carries the three.js (MIT) and d3-hierarchy
(ISC) notices this port derives from.
