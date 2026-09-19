# `webgpu_custom_fog_background`

Status: **green.** 57 of 100000 pixels against three.js r186's own
`test/e2e/image.js`, threshold 0.1%. Intel Iris Xe, Mesa 25.3.6, wgpu 30.0.1 on
Vulkan.

The page is `webgpu_loader_gltf`'s scene — DamagedHelmet under the UltraHDR
`royal_esplanade` map as `scene.environment` — with no `scene.background`, the
renderer's tone mapping off, and a `RenderPipeline` composite that reads the
scene pass's **depth attachment** and mixes a fog colour over the tone-mapped
scene by it.

## Grade first

Three's own frame for this page, captured through `tools/dump-webgpu.mjs`
(which pins the harness exactly as the grader does) and compared against
`examples/screenshots/webgpu_custom_fog_background.jpg` with three's unmodified
`Image.compare( …, 0.1 )`:

| | pixels of 100000 |
| --- | --- |
| three.js r186 against its own reference JPEG | 60 (0.060%) |
| this port against the same JPEG | 57 (0.057%) |

Both print as "0.1%" under the harness's `toFixed( 1 )`. The port is at three's
own number, which is the most this ladder can ask: the residue is the reference
JPEG and this GPU, not the port. `webgpu_loader_gltf` (59) and `webgpu_mrt` (87)
sit the same way — see `docs/webgpu_loader_gltf-progress.md`, "Grade first",
for the same measurement on the shared scene.

Unlike `dump-gltf`, this dump's image is sound: the page loads the model from
the checkout's own `examples/models/`, not over the network, so the helmet is
in three's captured frame.

## Reconciling with the plans

No scout plan exists for this page. It was ported from
`~/src/vendor/three.js/examples/webgpu_custom_fog_background.html` directly,
against three's WGSL dumped into
`target/dumps/webgpu_custom_fog_background/` (uncommitted, per the rules).
The composite quad is `m08_fragment_fragment_RenderPipeline.wgsl`; the scene
program `m06_fragment_fragment_Material_MR.wgsl` is byte-identical to
`webgpu_loader_gltf`'s and needed nothing.

Two pieces of the brief turned out to be already present and are recorded here
so no one re-adds them:

* **`RenderPipeline.outputColorTransform = true`** is the port's default, and
  the custom `outputNode` path around it already existed
  (`webgpu_postprocessing_ca` is the example that sets it to `false`). The
  example assigns it anyway, because the page does.
* **`rangeFogFactor`** already existed, as `positionView.z.negate()` through
  `smoothstep` — the `scene.fogNode` form `webgpu_postprocessing_difference`
  uses. Only the `getViewZ` override was missing.

## What was added

| area | what |
| --- | --- |
| `src/renderer/pass.rs` | `PassNode::view_z_node( name )` and `PassNode::depth_texture()` — `getViewZNode()` / `getTexture( 'depth' )`; the `_cameraNear` / `_cameraFar` uniforms and their per-frame write in `render()` |
| `src/nodes/tsl.rs` | `perspective_depth_to_view_z()`; `range_fog_factor_with_view_z()`; `pass_depth_texture()` (a depth `textureLoad` on the raw `uv()`, no texture matrix) |
| `src/nodes/wgsl.rs` | `TextureKind::DepthMultisampled2D`; `texture_dimensions()` drops its level argument for it |
| `src/nodes/builder.rs` | the texture slot picks the multisampled kind from the depth texture's flag |
| `src/renderer/programs.rs` | `multisampled: true` on that kind's bind-group layout entry |
| `src/textures/depth_texture.rs` | `DepthTextureInner::multisample` — `texture.isMultisampleRenderTargetTexture` — with `set_multisample()` / `is_multisample()` |
| `src/renderer/render_target.rs` | `set_samples()` propagates the flag (and drops the MSAA/depth allocations when the count actually changes); `set_depth_texture()` seeds it |
| `src/materials/node_material.rs` | `tone_mapping_node( mode, exposure, color )` — `ToneMappingNode` lifted out of `render_output()`, which now calls it |
| `examples/webgpu_custom_fog_background.rs` | the port |
| `examples/dump_wgsl.rs` | a `custom_fog_quad` section, built from a real `PassNode` |
| `tests/e2e/main.rs` | the graded test and the `rung!()` line |

`docs/nodes.md` §24 is the reference for all of it.

## What the pixels found

**Nothing.** The example passed the grader on its first run, at 57 pixels. That
is unusual on this ladder and worth saying why: the whole rung is one shader,
the shader was diffed against three's dump before the first GPU run, and the
scene behind it was already green as `webgpu_loader_gltf`.

What the *diff against the dump* found, before any pixels, was one real bug:

**`textureDimensions( t, u32( 0 ) )` does not compile against a multisampled
texture.** The port emits the level argument for every non-filterable
`textureLoad`, and three's dump for this page emits `textureDimensions(
nodeUniform3 )` with no level. WGSL gives a multisampled texture no
`textureDimensions( t, level )` overload at all, so this would have been a shader
compile failure, not wrong pixels — but it was found by reading the diff, and it
is the one place where the multisampled kind changes generated code rather than
just a declaration. `wgsl::texture_dimensions` now takes the kind.

Two things that would have been wrong pixels, caught by transcribing rather than
by the grader:

* **`pass_depth_texture()` uses `uv()`, not `transformed_uv()`.** The existing
  `tsl::depth_texture()` (the `webgpu_depth_texture` rung's) carries a `mat3x3`
  texture matrix in the object uniform block. A `PassTextureNode` calls
  `setUpdateMatrix( false )`, so the pass's depth read has none, and reusing the
  old helper would have added a uniform three's dump does not have.
* **The fog colour is never tone mapped.** `fogFactor.mix( scenePassTM, fogColor )`
  tone maps only the first operand. Folding the tone map outside the `mix` —
  which reads more naturally — washes the background from `0x4080cc` to
  something noticeably paler across 80% of the frame.

## What was ruled out

* **A general `ContextNode`.** Three implements `.context( { getViewZ } )` as a
  node that swaps `builder.context` for the duration of a sub-build. This port's
  TSL is eager, so there is no builder to swap; `range_fog_factor_with_view_z`
  passes the node as an argument instead and generates the same WGSL.
  `docs/nodes.md` §24.3 has the argument in full. A page that overrode
  `getViewZ` for a *material* would need the real thing; nothing on the ladder
  does.
* **Resolving the depth attachment.** WebGPU resolves colour attachments only,
  and three binds the multisampled depth texture directly. Adding a resolve pass
  would have been a second, invisible divergence with its own pixel cost;
  `multisampled: true` plus `textureLoad( …, 0 )` is what three does.
* **Reading `renderer.samples()` inside `PassNode::new()`.** The pass has no
  renderer at construction, the way three's does not either — `PassNode.setup()`
  assigns `renderTarget.samples` on the first build. The port assigns it in
  `PassNode::render()`, which is the same moment relative to the first draw.
* **A `Scene::fog` route.** `scene.fog = new THREE.Fog( 0x4080cc, 2.7, 4 )` is
  what the page's comment says the composite is *equivalent to*, and the port
  has that path (`fog()` / `FogNode`). It is not the same frame: material fog is
  applied per draw, so the empty background would stay at the clear colour
  instead of becoming fog. The point of the page is that the fog is the
  background.

## What was left out

* **`OrbitControls`.** The page's `minDistance` / `maxDistance` clamp nothing at
  this camera distance and no pointer events reach the harness, so
  `controls.update()` reduces to `camera.lookAt( 0, -0.1, -0.2 )`, which the
  example does directly — the same emulation every earlier postprocessing rung
  uses.
* **`onWindowResize`.** The harness never resizes.
* **`PassNode::view_z_node` for a name other than `'depth'`.** Three allows a
  custom depth output (an extra colour attachment written by an MRT); the port
  asserts on the name. No ported page asks for one.
* **`getLinearDepthNode()`.** `viewZToOrthographicDepth` over the same viewZ.
  Nothing on the ladder reads it yet.
* **Exposure as a uniform on `tone_mapping_node`.** The function takes a node,
  so a uniform works; the page passes the literal `1` and that is what is
  exercised.

## Ladder after the change

Unchanged, all 42 e2e tests green: depth_texture 0, instance_mesh 60,
materials_basic 0, rtt 1, lights_phong 31, morphtargets 0, shadowmap 7,
lights_physical 4, postprocessing_masking 18, tsl_galaxy 40, loader_gltf 59,
mrt 87, and this page 57.
