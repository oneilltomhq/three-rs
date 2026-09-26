# `webgpu_multisampled_renderbuffers`

Status: **ported, not graded on this machine.** Three itself scores 2405 of
100000 pixels (2.4%) against its own reference screenshot here, and the port
scores the same 2405. The port's frame against three's own frame on this
machine (`tools/dump-webgpu.mjs`'s `actual.jpg`) is 0 different pixels. Intel
Iris Xe, Mesa 25.3.6, wgpu on Vulkan; three through Chrome's Dawn on the same
driver. The e2e test is `#[ignore]`d with that reason, as
`webgpu_textures_anisotropy`'s is, and the page sits in the steady-frame
strip, which checks that frames two and three build and upload nothing. It
has no README, web or viewer registration. `docs/nodes.md` §58 is the long
form.

Fifty wireframe boxes and fifty red boxes just inside them, two
`InstancedMesh`es, rendered into a `RenderTarget` with `samples: 4` and a
depth buffer, then shown through a `QuadMesh` that samples
`renderTarget.texture`.

## Where the pixels sit

Every differing pixel is on a wireframe line: whole segments present in the
reference that are absent here, and the reverse, the pattern of a different
line rasterizer (Vulkan leaves the line rasterization mode to the
implementation, and the reference was taken on other hardware). The red faces,
the background and the resolve are exact. Since three's own render on this
machine misses by the same count, the page cannot gate anything here, the same
situation as `webgpu_materials_alphahash` in
`docs/webgpu_postprocessing_ca-progress.md`. The grader is not loosened and no
second comparator was added.

## What was added

Nothing in `src/`. The page needs `Material.wireframe`, which PR #203
(`webgpu_layers`) adds and `docs/nodes.md` §50.2 describes: a `line-list`
through a `getWireframeIndex()` buffer built once per geometry, with the draw
range doubled. This branch is built on #203 and adds only the example, its
ignored e2e test, this doc and `docs/nodes.md` §58.

The render target path needed nothing new either: `samples: 4` already gives
an MSAA colour attachment resolved into the `rgba8unorm` texture and a
multisampled `depth24plus`, as in the dump (textures 0, 1, 2).

## On #203's wireframe

This branch first carried its own wireframe implementation. #203's gives the
same numbers on this page: 2405 against the reference screenshot, and 0
against three's own frame (`tools/dump-webgpu.mjs`' `actual.jpg`, through
three's `image.js`).
