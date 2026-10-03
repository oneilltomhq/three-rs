# `webgpu_sky`

Status: **green.** 0 of 100000 pixels against three.js 5f610f5's own
`test/e2e/image.js`, threshold 0.1%. Intel Iris Xe, Mesa 25.3.6, wgpu on
Vulkan. Steady frame 5.8 ms (release), 3 draw calls, 3981 triangles.

A `SkyMesh` scaled to 450 000 is the whole background: the Preetham daylight
model, the sun disc, and a procedural cloud layer of four octaves of gradient
noise. A sphere of radius 400 at the origin reflects it. A `CubeCamera` at the
origin renders the scene into a 256² half-float `CubeRenderTarget` every
frame, with the sphere hidden. The sphere's `MeshBasicNodeMaterial( { envMap
} )` samples that cube along the reflected view vector. ACES Filmic at
exposure 0.05 (the GUI's default) applies to the canvas pass only. The sun is
at elevation 65° and azimuth 0, and time is pinned to 0, so the clouds have
neither drifted nor evolved.

## Reconciling with the plans

There is no scout plan. The rung was ported from
`~/src/vendor/three.js/examples/webgpu_sky.html` and
`examples/jsm/objects/SkyMesh.js`, and three's WGSL was dumped by
`tools/dump-webgpu.mjs` into `target/dumps/webgpu_sky/` (uncommitted). The
sky's two stages (`m00`, `m01`) are committed verbatim as
`tests/fixtures/webgpu_sky/` and gated by `tests/nodes_sky_wgsl.rs`. The
sphere's material and the output pass are ones every envmap rung already
covers.

## What was added

| area | what |
| --- | --- |
| `src/addons/objects/` | `SkyMesh`: the mesh and its eleven uniforms as `SettableValue`s; `vertexNode`, `colorNode` and the inline `gradient` / `noise` / `fbm` `Fn()`s |
| `src/cameras/cube_camera.rs` | `CubeCamera::new( near, far, renderTarget )` and `update( renderer, scene )` |
| `src/renderer/cube_render_target.rs` | `CubeRenderTarget::new( size, type )`: the cube texture and the face target the camera renders through |
| `src/nodes/` | `Node::VarIntent` and `tsl::to_var_intent()`: the function-scope `var` three's `toVarIntent()` becomes once assigned (`Lin.mulAssign`) |
| `examples/` | `webgpu_sky.rs`; a `dump_wgsl` section `sky` |
| `tests/` | `nodes_sky_wgsl.rs` against three's `m00` / `m01` |

`docs/nodes.md` §59 is the long form.

## What the pixels found

Nothing to chase: the first frame graded 0 pixels. The fixture test came
first. Once it matched three's flow statement for statement, the frame matched
too. Getting there took the intent var (§59.1), the `bool` type of
`showSunDisc` (§59.2), and a `to_const` for each temp three's dump has as a
`let`.

## What is not ported

- `renderer.inspector` and its parameters panel draw nothing into the canvas.
  `EffectController` and `gui_changed()` hold the GUI's values and its one
  callback, so a host can drive them.
- `CubeCamera.activeMipmapLevel` is not ported. The page renders mip 0 only.
