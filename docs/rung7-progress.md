# rung 7 — `webgpu_shadowmap`, done

**Status: passing.** `webgpu_shadowmap` differs in **7 of 100000 pixels**
(0.007%, limit 0.1%). The six earlier rungs are unchanged: `depth_texture` 0,
`instance_mesh` 60, `lights_phong` 31, `materials_basic` 0,
`postprocessing_masking` 18, `rtt` 1. `cargo test --workspace` is 774 tests in
55 suites plus the 7 e2e comparisons, all green; `cargo build --workspace
--all-targets` is clean (the only warnings are the pre-existing `value assigned
to a is never read` ones in the `math_*` tests).

The generated WGSL matches the scout's r186 dumps structurally for all eight
programs the example builds — same bindings, same uniform members, same
expression order, same `fn` bodies — modulo the cosmetic set in
`docs/nodes.md` §8, to which rung 7 adds three entries.

## What rung 7 added

| area | what |
|---|---|
| shadows | `src/lights/light_shadow.rs` (`LightShadow`, `ShadowCamera` orthographic/perspective, `update_matrices`, the bias matrix, `update_spot_projection`), `Object3D.cast_shadow` / `receive_shadow`, `Renderer.shadow_map_enabled`, `Renderer::render_shadows()` (`ShadowNode.updateShadow()`), the `shadow_targets` / `shadow_maps` caches |
| node primitives | `Node::Block`/`Loop`/`If`/`Discard`/`Not`, `TextureSource::ShadowMap`, `SampleMode::Compare`, `TextureKind::DepthCompare2D`, 11 new `UniformSource` variants, `Type::UVec3` |
| lights | `src/lights/light_object.rs` — one `LightObject` with a `LightKind` (`Ambient`/`Directional`/`Point`/`Spot`), a `target`, and an optional `LightShadow`; replaces `src/lights/point_light.rs` |
| materials | `phong::{shadow_factor, pcf_shadow, ambient_lights, direct_light}`, `LightDesc`, `materials::shadow_material()` (`_getShadowNodes`), `Blending::{Normal,None}`, `is_opaque()`, `mask_node` + `setupDiscard`, `received_shadow_position_node`, `fog` flag, `ToneMapping::AcesFilmic` in `render_output()` |
| MaterialX | `src/nodes/materialx/mx_noise.rs` — the 17 `mx_*` function singletons behind `mx_fractal_noise_float` / `mx_fractal_noise_vec3`, with `tests/nodes_mx_noise.rs` |
| TSL | `position_world`, `shadow_position_world`, `transform_direction`, `light_target_direction`, `shadow_map_compare`, `interleaved_gradient_noise`, `vogel_disk_sample`, `aces_filmic_tone_mapping`, `block`, `loop_n`, `if_then`, `discard`, `discard_if`, `int`, `vec2_join` |
| scene | `Background::Node( Color )`, `ProjectCamera::from_parts()`, `RenderState.blend` + the `SrcAlpha`/`OneMinusSrcAlpha` blend state, `DepthTexture::set_filters()` |
| example | `examples/webgpu_shadowmap.rs` + the seventh `tests/e2e/main.rs` entry + eight `dump_wgsl` programs |

## The readings that mattered

* **`positionLocal` is a varying, not a var.** In r186
  `positionLocal = positionGeometry.toVarying( 'positionLocal' )`. The knot's
  `maskNode` is built on it and is evaluated in the *fragment* stage, so with
  the old `to_var` accessor the builder tripped its own
  `attribute position read outside the vertex stage` assertion. Switching the
  accessor to `to_varying` is byte-neutral for rungs 1–6, because the port's
  `Node::Varying` arm already degrades a varying nothing in the fragment stage
  asks for into a plain `var<private>`.
* **The shadow material's missing `DiffuseColor.w = 1.0` comes from
  `blending`, not `transparent`.** `builder.isOpaque()` is
  `!transparent && blending === NormalBlending`, and `_getShadowNodes` sets
  `blending = NoBlending`. Hence the `Blending` enum and `is_opaque()`; the
  `RenderState.blend` flag falls out of the same field.
* **`frontFace: cw` on the shadow pipelines is not a pipeline flag.**
  `_shadowSide[ material.side ]` flips `FrontSide` → `BackSide` for the shadow
  pass, and the existing `Side` → front-face mapping then produces `cw` on its
  own.
* **The shadow factor multiplies the light colour before the cone and the
  distance attenuation** — `lightColor = colorUniform * vec3( shadowFactor )`
  first, then `smoothstep( coneCos, penumbraCos, … )`, then the distance term.
  Getting this order wrong would have been invisible in a casual read of
  `AnalyticLightNode` but is explicit in m11.
* **The frustum guard has no `z >= 0`**: `x >= 0 && x <= 1 && y >= 0 && y <= 1
  && z <= 1`, `else 1.0`, then `mix( 1.0, shadow, intensity )`.
* **`resetRendererAndSceneState` is not needed.** `render_shadows()` is a
  self-contained `draw()` into its own target with its own `UniformContext`,
  no background item, no fog (`ShadowMaterial.fog = false`) and no output
  pass, so none of the state three.js has to put back is ever read.
* **The ground's noisy position is two writes to one var.** m11 has
  `nodeVar6.x = nodeVar7[ 0 ]; nodeVar6.z = nodeVar7[ 1 ];`; the example
  reproduces that with `element_node( int( 0 ) )` / `int( 1 )` assignments
  rather than a swizzle constructor, so the generated text matches.

## Bugs the grader found

None. Once the example compiled and the eight programs matched the dumps, the
first complete e2e run came in at 7 pixels. The bugs on the way to that run
were all caught earlier — by the builder's own stage assertion (`positionLocal`
above) and by the WGSL diff — not by the image comparison.

### Ruled out, so the next rung does not re-investigate

* **The 7 residual pixels are not a shadow-filter difference.** `diff.jpg`
  puts every one of them on the knot and its immediate surround — the one
  surface with a per-pixel `normalMap`-free specular highlight — and not on a
  shadow boundary, a penumbra edge or the ground's noise. Differencing
  `actual.jpg` against `expected.jpg` directly shows the whole frame is within
  JPEG-quantisation distance; the survivors are the high-gradient specular
  pixels the ACES curve then stretches. This is the same class of residual
  rungs 5 (31 px) and 9 (1 px) carry.
* **Shadow-map filtering is `Linear` min *and* mag with
  `compare: LessEqual`**, sampled through a `sampler_comparison` /
  `texture_depth_2d` pair — not a manual depth compare against a colour
  target. The dump's `textureSampleCompare` fixes this; nothing here was
  guessed from the image.
* **The shadow pass needs its own `_projectObject` walk** against the shadow
  camera's frustum, not a filter of the main render list: three.js calls
  `renderer.render( scene, shadow.camera )`, so culling happens against the
  light's frustum and only then is `object.castShadow` applied.
* **The guard has no `z >= 0` term and no `clamp` on the shadow coordinate** —
  m11 shows five conditions, not six. Do not "fix" it.
* **`normalWorld` is a row-vector multiply**:
  `normalize( ( vec4( normalView, 0.0 ) * cameraViewMatrix ).xyz )`. The
  column form transposes the rotation and moves the normal-bias offset.

## Divergences

Recorded in `docs/nodes.md` §8. Rung 7 adds: the MaterialX `fn` declaration
order, `vec3<f32>( 0.0, 0.0, 0.0 )` vs `vec3<f32>( 0.0 )` in `clamp` bounds,
and the ground's position block being evaluated twice rather than shared
between the colour and the position chains. All three are textual.
