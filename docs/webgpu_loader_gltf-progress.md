# `webgpu_loader_gltf` and `webgpu_mrt`

Status: **both green.** `webgpu_loader_gltf` 59 and `webgpu_mrt` 87 of 100000
pixels against three.js r186's own `test/e2e/image.js`, threshold 0.1%.
Intel Iris Xe, Mesa 25.3.6, wgpu 30.0.1 on Vulkan.

One note for two examples, because they are one scene: an UltraHDR
equirectangular map as both `scene.background` and `scene.environment`, with
DamagedHelmet in front of it. `webgpu_loader_gltf` draws it to the canvas;
`webgpu_mrt` draws it once into four colour attachments and composites the four
side by side.

## Grade first

Both pages were run through three's own harness twice before any Rust was
written. Neither is on `test/e2e/puppeteer.js`'s exception list.

**A correction the brief needs.** The brief asked for "grades 0.0% honestly",
which is not literally satisfiable: the harness prints `toFixed( 1 )`, so
anything from 50 to 149 pixels prints as "0.1%" and both of these do. An
instrumented copy of `puppeteer.js` (deleted afterwards) gave the real counts,
stable across two runs each:

| page | three.js against its own reference JPEG |
| --- | --- |
| `webgpu_loader_gltf` | 63 px (0.063%) |
| `webgpu_mrt` | 84 px (0.084%) |

That is a property of the reference JPEGs and this GPU, not of either page: the
same instrumentation puts three itself at 1 / 31 / 44 / 60 px on
`webgpu_pmrem_equirectangular` / `webgpu_postprocessing_bloom_emissive` /
`webgpu_materials` / `webgpu_instance_mesh`, which are this port's own README
numbers (1 / 28 / 44 / 60) to within noise. The port inherits the baseline; it
does not add to it. Both pages were therefore taken as valid targets rather
than skipped.

`webgpu_mrt` at 87 and `webgpu_loader_gltf` at 59 are both *at* three's own
number, so the port is as close to the reference as the reference engine is.

## Reconciling with the plans

No scout plan exists for either page; they were ported from
`~/src/vendor/three.js/examples/webgpu_loader_gltf.html` and `webgpu_mrt.html`
directly, with three's WGSL dumped by `tools/dump-webgpu.mjs` into `dump-gltf/`
and `dump-mrt/` (uncommitted, per the rules).

**A caveat on `dump-gltf`.** The tool's screenshot for `webgpu_loader_gltf` is
18.95% off its own reference and has no helmet in it: the page fetches the model
over the network, which does not finish inside the tool's single RAF. The
`Material_MR` *modules* are generated regardless, so the WGSL reference is
sound; only the tool's image is not. `dump-mrt`'s image is fine.

## What was added

| area | what |
| --- | --- |
| `src/objects/scene.rs` | `Scene::environment` — `scene.environmentNode`, the fallback `NodeMaterial.setupEnvironment()` reaches for |
| `src/materials/node_material.rs` | `SetupContext::environment`; `setup_standard` prefers the material's own `pmrem_env` |
| `src/materials/environment.rs` | `impl Hash for PmremHandle`, keyed on the atlas texture id |
| `src/renderer/mod.rs` | the background draw carries the pass's MRT; `mrt_context` moved ahead of the render list |
| `src/renderer/render_target.rs` | `msaa_extra` — one multisampled texture per extra attachment, at that attachment's format |
| `src/renderer/pass.rs` | `PassOptions { min_filter, mag_filter }`, `PassNode::new_with_options` |
| `src/textures/texture.rs` | `Texture::is_unfilterable()` |
| `src/nodes/tsl.rs`, `src/nodes/builder.rs` | an unfilterable texture binds `non-filtering`, with no sampler, and taps with `textureLoad` |
| `src/nodes/mrt.rs` | `MrtValue`, `MrtNode::set_deferred` — an MRT output rebuilt per material |
| `examples/` | `webgpu_loader_gltf.rs`, `webgpu_mrt.rs`; `dump_wgsl` sections `loader_gltf_helmet`, `loader_gltf_background`, `mrt_helmet`, `mrt_background` |

`docs/nodes.md` §23 is the long form of all of it.

## What the pixels found

**The skybox was writing one attachment of four.** `webgpu_mrt`'s `normal` and
`diffuse` bands came out at the clear colour everywhere the environment should
be — 7420 px, 7.4%. The background `Renderable` was pushed with a default
`SetupContext`, so `mrt` was `None` for it.
`webgpu_postprocessing_bloom_emissive` could not have caught this: its sky
writes `EmissiveColor`, which is zero, to attachment 1, and zero is the clear
value. §23.4.

**`normalView` was frozen at `init()` time.** With the MRT fixed, the `normal`
band was still wrong, and wrong in a way that named its own cause: every sky
pixel was exactly `1 - reference`. `mrt( { normal: packNormalToRGB( normalView
) } )` is built once in the page's `init()`, and this port's TSL is eager, so
`normal_view()` read the *default* material state — `FrontSide` — and the
`negateOnBackSide()` that `Background.material`'s `BackSide` calls for never
happened. `MrtNode::set_deferred` re-runs the expression inside each material's
setup instead. 7420 px → 87 px. §23.2.

**MSAA and MRT had never met.** `antialias: true` with four attachments is a
wgpu validation error, not wrong pixels — every colour attachment of a pass must
share a sample count. §23.7.

**The model is the checkout's own.** The page loads `DamagedHelmet.glb` from
Khronos over the network. That GLB and
`examples/models/gltf/DamagedHelmet/glTF/DamagedHelmet.gltf` are the same asset:
same node rotation, same `Material_MR`, and the five embedded JPEGs MD5 to the
five files beside the `.gltf`. The port loads the local copy, so neither example
needs the network.

## What was ruled out

* **A second environment path.** `scene.environment` could have been sugar that
  copies the handle onto each material at add time. It is not: the fallback is
  read per build, and `dump_wgsl`'s `loader_gltf_helmet` section is there to
  show the generated fragment shader cannot tell the two apart.
* **Making `normal_view()` lazy in general.** It would need a deferred `Node`
  variant threaded through every match in the builder. `set_deferred` confines
  the shim to the one node kind on this ladder that needs it, and generated code
  is identical either way.
* **A `nearestFilter` flag on `PassNode`.** Three passes an options object to
  `pass()` and puts the filters on the textures; `PassOptions` does the same, so
  `Texture::is_unfilterable()` stays the single predicate and nothing in the
  builder knows about passes.

## What was left out

* `scene.backgroundBlurriness` and `backgroundIntensity`. Both are GUI knobs at
  their defaults — 0 and 1 — in the graded frame, so the skybox is the sharp
  cube.
* `renderer.compileAsync()`. It warms the pipeline cache; this port compiles on
  the first draw.
* `requiredLimits: { maxColorAttachments: 5 }`. A WebGPU device request for a
  limit wgpu's default adapter limits already give (8).
* `OrbitControls` beyond the one `update()` the page does before the graded
  frame, and the `AnimationMixer` `fitCameraToSelection`'s caller would feed —
  DamagedHelmet has no animations.
* The inspector panel both pages add.
