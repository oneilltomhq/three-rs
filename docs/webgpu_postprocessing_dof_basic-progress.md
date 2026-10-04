# webgpu_postprocessing_dof_basic

Branch `dof-node`.

**Green at 36 pixels of 100000** (limit 0.1%), steady frame 7.1 ms, 42 draw
calls, 33694 triangles.

## What the page is

The page is not `DepthOfFieldNode`. It is the "simple DOF" of the lettier
tutorial its source cites:

- the beauty pass and a `boxBlur` of it are mixed by
  `smoothstep( minDistance, maxDistance, | viewZ - focus.z | )`;
- `focus` is a fixed world point, carried into view space every frame;
- the mix goes through `renderOutput()`, then `fxaa()`.

So the rung adds no new effect node. What it needed was scene and loader
support for `bath_day.glb` and its environment.

`DepthOfFieldNode` itself is ported on the same branch, for
`webgpu_postprocessing_dof`. three's e2e harness skips that page, so it has
no grade; see `docs/webgpu_postprocessing_dof-progress.md`.

## What this rung adds

| area | what |
|---|---|
| `src/objects/scene.rs` | `Scene::environment_rotation`, `scene.environmentRotation` |
| `src/renderer/mod.rs` | `materialEnvRotation` takes the scene's rotation while the scene has an environment and the material has no `envMap` of its own |
| `src/loaders/gltf_loader.rs` | `alphaMode: MASK` becomes `alpha_test = alphaCutoff`, as three's `GLTFLoader` does |

## What the score found

**The first run was 1458 pixels, and all of them were `MASK`.** The loader
did not wire `alphaMode: MASK`, on purpose: nothing on the ladder used it,
and the crate had only `alphaTestNode`. Since then `materialAlphaTest` has
become a uniform, so the loader now sets `alpha_test`, as three's does. The
plant leaves and the rug's fringe in `bath_day.glb` are `MASK`. Unwired,
their cut-outs drew as solid quads, and the box blur spread them. With
`MASK` wired, 36 pixels are left.

**`scene.environmentRotation.y = -π / 2`** turns the PMREM under every
material. Without it the reflections on the bath and the toys face the wrong
wall.

## Not ported

- **The GUI** (min/max distance, blur size and spread). The uniforms keep
  their defaults.
- **The tween and the raycast click**, which move the focus point on pointer
  input. The focus stays at `( 1, 1.75, -0.4 )`.
- **`toInspector()`**, which returns its node unchanged.
