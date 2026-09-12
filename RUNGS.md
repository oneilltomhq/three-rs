# Rung log

| rung | example | result | commit on `port` | notes |
|---|---|---|---|---|
| 0 | grader | calibrated | bdbb76a (handoff) | see rung0/RUNG0.md |
| 1 | webgpu_depth_texture | PASS 0/100000 px, Intel Iris Xe | 2362510 | max RGB distance 15.6 of 44 limit; hand-written WGSL in src/renderer/shaders/{basic,quad}.wgsl to replace at rung 4; in-page devicePixelRatio is 1 (grader sets no deviceScaleFactor) |
| 2 | webgpu_instance_mesh | PASS 45/100000 px (three itself scores 60 vs the same JPEG) | f15e158 | key finding: scene renders into an internal rgba16float MSAA target in linear, then a separate output pass applies sRGB (Renderer.needsFrameBufferTarget / _renderOutput); converting inside the scene pass fails at edges. New WGSL: normal_world_range_mix.wgsl, output_color_transform.wgsl |
| 3 | webgpu_materials_basic | PASS 0/100000 px (max RGB distance 43.0 of 44 limit, single JPEG-ringing pixels on edges) | f4c47da | cube texture rgba8unorm-srgb (GPU does sRGB→linear, no colour-space node), Three's mipmap blit is per-face 2d-array bilinear box, background is a BackSide SphereGeometry(1,32,32) drawn first with depthCompare always. New WGSL: basic_envmap, background_cube, mipmap. Last rung with hand-written WGSL. |

## Side branches (cut from f4c47da, to rebase onto `port` after rung 4)

| branch | state | notes |
|---|---|---|
| viewer | done, verified (window opens on Wayland, orbit/zoom/pan correct, present is byte-identical to canvas) | `cargo run --release --bin viewer -- <example>`; renderer seam is `src/renderer/present.rs` + 4 small hunks in mod.rs |
| geometries | done, 29 tests, all bit-exact vs three.js samples | Box, Plane, Cylinder, Cone, Torus, Polyhedron family, Circle, Ring, Lathe, Capsule, Teapot addon, RoundedBox addon, to_non_indexed. `*_with_groups()` variants until BufferGeometry gets `groups`; math extras in src/geometries/math_extras.rs to fold into src/math |
| unit-tests | done, verified: 293 unit tests + 3 e2e green (7c15ea5, 13 commits) | Three's QUnit math/core tests: Vector2/3/4, Matrix3/4, Quaternion, Euler, Color, MathUtils, Object3D transforms, BufferAttribute/Geometry, PerspectiveCamera view/film API. Two fixes: compute_vertex_normals panicked on a trailing partial triangle; Color::default() now white like Three. No graded image moved. Skipped: no-parent Object3D tree ops, named-attribute map ops, JSON/uuid/typed-array families. |
