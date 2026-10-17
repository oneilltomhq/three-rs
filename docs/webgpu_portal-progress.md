# webgpu_portal

Branch `webgpu_portal`, issue #351.

**Green at 77 pixels of 100000** (limit 0.1%), steady frame 24.9 ms, 9 draw
calls, 53085 triangles and 147336 wireframe lines. At full size the port's
frame and three's dump frame (`tools/dump-webgpu.mjs webgpu_portal`) are the
same 800x500 image, pixel for pixel; the 77 are the JPEG reference.

## What the page is

A plane in the main scene has as its colour a `pass()` of a second scene,
read at `screenUV` rather than at the plane's own uv, so the plane shows the
second scene exactly where it would be on screen. Both scenes hold the
animated `Xbot.glb`; the portal's wears a fractal-noise wireframe. Each scene
has a `backgroundNode` over `normalWorld`: a vertical gradient in the main
scene, scrolling Worley noise in the portal.

Three's frame is 3 render passes: the portal pass (its background, then the
Xbot's two meshes), the main pass (background, Xbot, then the plane's back
half and its front half), and the output colour transform. The port's render
counts match: 9 draws, 9 pipelines.

## What this rung adds

| area | what |
|---|---|
| `src/renderer/pass.rs` | `PassNode::sample( coord )` — `pass( … ).context( { getUV: () => coord } )`, the pass tapped at the caller's uv |
| `src/renderer/mod.rs` | `Renderer::render_shared` is public: a page whose main render shares its camera with a pass renders this way |
| `tests/nodes_display_wgsl.rs` | the plane's material, front and back halves, gated against three's `m12` / `m13` |

**The pass tapped at `screenUV`.** Three reaches this through the builder
context: `TextureNode.setup()` asks `builder.context.getUV` for a tap that
has no uv of its own, so `.context( { getUV } )` around the pass moves its
tap. The port's `texture()` reads the context when the tap is *made*, not at
build, so a context around a finished node changes nothing (#352). The
narrow fix is a second tap on the pass's output texture at the given uv. The
nested render is keyed by the texture (`register_texture_update`), so a draw
that samples this tap fires the pass exactly as one that samples `node()`
does. The tap is the texture node alone, not a var over it as `node()` is:
three's `ContextNode` is no `TempNode`, and the `PassNode` under it is read
once, so the dump has the one var `TextureNode.generate()` makes
(`nodeVar0 = textureSample( … ); DiffuseColor = nodeVar0;`).

**The camera is shared with the pass.** The portal pass renders with the
camera the main scene is being drawn with, from inside the main render. A
render that holds the camera mutably would find it borrowed when the pass
asks for it, so the page renders with `render_shared`, which updates the
camera's matrices through a `try_borrow_mut` and then holds it shared.

## Checked, not assumed

Issue #351 listed four things to check rather than assume. All four hold:

- **A pass of another scene, with its own `backgroundNode`, inside the main
  render.** The portal's Worley background is in the frame and matches
  three's. Both background nodes read `normalWorld`, and both had to be
  deferred (below).
- **`wireframe = true` on the glTF's cloned standard materials.** The two
  cloned materials draw as a `line-list` with the wireframe index: 147336
  lines in the render counts, the fractal-noise wireframe in the frame. The
  doc on `MeshBasicNodeMaterial::wireframe` was already right.
- **`PointLight.clone()` keeps `power = 17000`.** `LightObject::clone`
  copies the `Light`, which holds the intensity `set_power()` wrote. The
  portal scene's light is `ObjectRef::new( light.borrow().clone() )`.
- **A `DoubleSide` transparent plane draws back then front.** The renderer
  splits such a material into a `BackSide` draw and a `FrontSide` one, per
  object and in list order, as `_renderObjectDirect()` does. Three's dump
  builds the back half first (`m12`, `normalView = normalViewGeometry * -1`)
  and then the front (`m13`).

## One thing the scene found

**A page-level `normalWorld` background renders upside down.** Three
resolves `normalWorld` when the skybox material builds, where its `BackSide`
negates the view normal. The port resolves it where the node is made (#352
again), with the side still `Front`, so the main scene's gradient came out
pink at the top. Both background nodes are now wrapped in an argument-less
`Fn()` call, which `backgroundNode` runs inside the skybox build
(`background_node_color_node`), as `webgpu_postprocessing_retro` and
`webgpu_tsl_halftone` already do. Three's page writes neither as an `Fn`.

## Divergences

- `pass( … ).context( { getUV } )` is `PassNode::sample( coord )`, a
  method on the pass rather than a context on any node (#352).
- `gltf.scene.clone()` is `Object3D.clone( true )`, which the port's clone is
  not (#300): the model is loaded twice, and the second load is the page's
  clone, down to its cloned materials.
- The two background nodes are deferred `Fn()` calls (above).
- Three lights the plane through `BasicLightingModel`
  (`MeshBasicNodeMaterial.lights` is true): the hemisphere's irradiance is
  summed into a var nothing reads, and `outgoingLight` is `DiffuseColor.rgb`.
  The port's Basic path takes that model only for an env map, an AO or a
  backdrop (`docs/nodes.md` §61.4), so its module has no lighting block. The
  WGSL gates compare the statements the pass and the opacity node own and
  the `Output` clamp, not the whole body.
- The scene's `background = new THREE.Color( 0x222222 )` is set and then
  shadowed by `backgroundNode`, which wins in `Background.update()`; only the
  node is kept.

## Left out

Nothing on the page. The general `getUV` context (#352) and the recursive
`Object3D` clone (#300) stay open.
