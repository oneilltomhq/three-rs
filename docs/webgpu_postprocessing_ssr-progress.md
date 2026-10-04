# webgpu_postprocessing_ssr

Branch `ssr-node`.

**Green at 4 pixels of 100000** (limit 0.1%), steady frame 10.2 ms, 17 draw
calls, 22888 triangles.

## What the score says

The graded frame is the page's first, and SSR is visible in it: rendering the
same frame with `intensity` at 0 changes 67196 of the 400000 pixels at
800x500, 17373 of them by more than 32 levels. They are the reflections in
the brass and red casing, and the model's faint image on the disc. Matching
three's frame to within 4 pixels therefore covers these things:

- the march itself;
- the mip-chain blur that roughness picks from;
- the additive composite;
- the SMAA pass over all of it.

Each fragment shader is also gated against three's dump in
`tests/nodes_display_wgsl.rs`:

| test | three's dump | what |
|---|---|---|
| `ssr_matches_three` | `m21` | `SSRNode.SSR`, the march |
| `ssr_copy_matches_three` | `m23` | `SSRNode.Copy`, mip 0 of the blur chain |
| `ssr_blur_matches_three` | `m24` | `SSRNode.Blur`, mips 1–4 (`boxBlur`, `size = 1`) |
| `ssr_resolve_matches_three` | `m26` | the page's `RTT`, `scenePassColor.add( ssrPass.rgb )`: the blur chain at `clamp( roughness² · 4, 0, 4 )` |
| `smaa_edges_matches_three` | `m28` | `SMAANode.edges` |
| `smaa_weights_matches_three` | `m30` | `SMAANode.weights` |
| `smaa_blend_matches_three` | `m32` | `SMAANode.blend` |

The steady frames build nothing: `steady_frame_builds_nothing` has a rung for
the page.

## What this rung adds

| area | what |
|---|---|
| `src/nodes/display/ssr.rs` | `SSRNode.js`: `ssr()`, `SsrNode`, `SsrOptions` |
| `src/nodes/display/smaa.rs` | `SMAANode.js`: `smaa()`, `SmaaNode`; `smaa_area.png` and `smaa_search.png` are three's two embedded PNGs, base64-decoded byte for byte |
| `src/nodes/display/box_blur.rs` | `box_blur_with()`, the blur with a caller's sample function, which is what SSR's blur pass is; a constant `size` emits an integer loop bound, as three's does |
| `src/nodes/node.rs`, `builder.rs`, `tsl.rs` | `Node::Continue` / `tsl::continue_loop()` (`Continue()`), and `tsl::get_screen_position()` |
| `src/renderer/render_target.rs` | `RenderTarget::set_mip_level_count()`, the port of `texture.mipmaps.push( … )` on a render target |
| `src/renderer/mod.rs` | `Renderer::set_render_target_level()` and `active_mipmap_level()`, three's `setRenderTarget( rt, 0, level )`: the pass draws into a one-level view, and the viewport and scissor shrink with the level |
| `src/objects/scene.rs`, `src/renderer/mod.rs` | `Scene::environment_intensity`, three's `scene.environmentIntensity`, which scales the scene environment's radiance and irradiance (the page sets 1.25) |
| `src/loaders/mod.rs` | `texture_loader` is `pub(crate)`, so SMAA can reuse its PNG decode |

## Divergences

- **The model loads synchronously.** The page loads `steampunk_camera.glb`
  asynchronously and adds it after the floor. Three's screenshot is taken
  once the model is in, so the port loads it at that same point in `init()`.
  The GLB's textures are `EXT_texture_webp`, not Draco, so the page's
  `DRACOLoader` has nothing to do.
- **`sample( fn )` is a Rust closure.** The page's `sceneNormal` is
  `sample( ( uv ) => unpackRGBToNormal( … ) )`. The port passes
  `SampleFn = Rc<dyn Fn(NodeRef) -> NodeRef>` to `ssr()`, and `ssr()` calls
  it at the default uv and at the march's `uvS`, as three's
  `normalNode.sample( uv )` does. `unpackRGBToNormal` is written inline
  (`rgb * 2 - 1`) in the page and the gate, because the helper is in the
  unmerged `gtao-denoise` branch.
- **`scenePassColor.add( ssrPass.rgb )`.** In three, `add` widens the vec3 to
  `vec4( rgb, 1.0 )`, as the `m26` dump shows. The port writes that out with
  `vec4_join`.
- **`toInspector()`** is dropped. It returns its node unchanged.
- **Inputs are forced first.** Both nodes run the scene pass's (or the RTT's)
  updater at the top of `update_before()`, as `TraaNode` does, so that their
  inputs are current. Three gets the same order from `setup()`.
- **The GUI is not ported.** Its defaults are applied through the page's own
  `updateParameters()`. `enabled` and the model `roughness` slider stay at
  their starting values.

## Not ported

From `SSRNode`, all of these are left at their defaults by the page:

- a `resolutionScale` other than 1;
- an orthographic camera;
- a logarithmic depth buffer.

`SSRNode`'s stochastic path, `reflectNonMetals`, `binaryRefine`,
`screenEdgeFadeBlack`, `setHistory()` and `diffuseNode` are ported since,
for `webgpu_postprocessing_ssr_denoise` (`docs/nodes.md` §65.3); this page
leaves them at their defaults.

From `SMAANode`: `SMAANode.js` has no options to leave out.

## Viewer and browser

The native viewer has the page as key 87. It is on the Pages build, and its
manifest is `web/manifests/webgpu_postprocessing_ssr.json`.
