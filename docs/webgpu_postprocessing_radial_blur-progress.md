# webgpu_postprocessing_radial_blur

Branch `rung-radial-blur`, cut from `main` 2cf90bd and rebased onto
`rung12-compute-points` (rungs 10-12). **PASS — 7 of 100000 pixels differ
(0.007%), limit 0.1%.**

The scout plan (`scouts/scouts/webgpu_postprocessing_radial_blur/PLAN.md`) was
written today against current main, so there are no pre-0.2.0 deltas to
reconcile.

## What was added

| area | what |
|---|---|
| `src/materials/mod.rs` | `ToneMapping::Neutral` — `NeutralToneMapping`, the Khronos PBR Neutral mapper of `ToneMappingFunctions.js` |
| `src/nodes/node.rs` | `Node::Let` (`toConst()`, a WGSL `let`), `Node::Return`, `BufferSource::InstanceColor` |
| `src/nodes/tsl.rs` | `to_const()`, `return_statement()`, `neutral_tone_mapping()`, `instance_color()`; `normal_key()` now carries the flat-shading flag |
| `src/nodes/builder.rs` | `declare_const()` / `nodeConstN`, the `Let` and `Return` arms |
| `src/nodes/display/` | new module: `radial_blur()` and `RadialBlurOptions` |
| `src/materials/node_material.rs` | `SetupContext::instance_color`; `setupDiffuseColor`'s instance-colour multiply and its vec3 colour base |
| `src/renderer/mod.rs` | the `instanceColor` vertex buffer, threaded to `bind_groups` / `node_buffer` / `buffer_for` |
| `src/renderer/render_pipeline.rs` | the quad's tone mapping comes from `renderer.tone_mapping` |
| `src/objects/instanced_mesh.rs`, `src/objects/payload.rs`, `src/core/object3d.rs` | `InstancedMesh::set_color_at()` and the accessor |
| `examples/` | `webgpu_postprocessing_radial_blur.rs`, plus `radial_blur_quad` and `radial_blur_scene` in `dump_wgsl.rs` |
| `src/bin/viewer.rs` | the example on key `c` |

## WGSL

Against `scouts/scouts/webgpu_postprocessing_radial_blur/dump/`:

* **quad fragment** (`m03`) — the body between `// code` and `return output;`
  is byte-identical, including `let nodeConst0` / `let nodeConst1`, the
  `for ( var i : i32 = 0; i < i32( object.nodeUniform1 ); i ++ )` bound and
  `neutralToneMapping`'s early-out. What differs: the `renderStruct` /
  `objectStruct` order, the `// codes` order with the `fn0`/`fn1` swap, and
  `@builtin( position )` ahead of the `@location` parameter — all `docs/nodes.md`
  §8 entries.
* **quad vertex** (`m02`) — identical apart from the banner and blank lines.
* **scene vertex** (`m00`) — identical statement for statement, with the known
  `m[ 0u ]`, varying-location and uniform-numbering divergences.
* **scene fragment** (`m01`) — the §8 "named lighting temps", "hoisted
  accumulator zeros" and "inlined `faceDirection`" divergences, i.e. the port
  inlines what Three names. The line this rung cares about,
  `DiffuseColor = vec4<f32>( ( vInstanceColor * object.nodeUniformN ), 1.0 );`,
  matches.

Every pre-existing `dump_wgsl` section is **byte-identical** before and after
this branch: 97 sections against a baseline taken at 2cf90bd when the branch
was written, and 103 sections against `rung12-compute-points` after the
rebase, checked section by section both times. Only the six `radial_blur_*`
sections are new. That was the scout's first risk: `to_const` perturbing the
shared emitter. It did not.

## What the pixels found

Two bugs, both found by diffing the generated `radial_blur_scene` WGSL rather
than by looking at pixels:

* **`NodeBuilder.format` had no vec3 → vec4 padding.** The port built
  `materialColor` as `vec4( color, 1.0 )` up front, so `instanceColor.mul(
  colorNode )` produced `vInstanceColor * vec4<f32>( u, 1.0 )` — a vec3 times a
  vec4, invalid WGSL. Three keeps `materialColor` a vec3 until
  `diffuseColor.assign()` and pads there (`toTypeLength === 4 &&
  fromTypeLength > 1`). Adding that case and the vec3 base fixed it and changed
  no existing shader's text, because the padding produces the same string the
  explicit join did. On the rebase this branch kept only the vec3 base: rung
  12's `wgsl::convert()` had arrived at the same rule from the other end (it
  widens a literal the way `NodeBuilder.format` does), so `format()` delegates
  to it and the padding case lives there.
* **`normal_key()` omitted the flat-shading flag** while `normal_view()` built
  its own key that included it. `normal_world()` and `tangent_frame()` use
  `normal_key()`, so a smooth-shaded material built earlier in the same process
  poisoned the memo for the flat-shaded one: the fragment got both
  `normalViewGeometry = normalFlat;` and, later,
  `normalViewGeometry = normalize( v_normalViewGeometry );`, plus a varying
  nothing should have produced. This is exactly the rung-8 bug class named in
  `docs/nodes.md` §7 — everything that reads `normalView` has to share one key
  — and it is now one key.

## What was ruled out

* **Rung 2 as the instance-colour canary.** `webgpu_instance_mesh` uses the
  same `setupDiffuseColor` path and is still 60. The whole ladder is unchanged:
  depth_texture 0 / instance_mesh 60 / materials_basic 0 / rtt 1 /
  lights_phong 31 / morphtargets 0 / shadowmap 7 / lights_physical 4 /
  postprocessing_masking 18 / tsl_galaxy 40, and radial_blur 7.
* **`BufferSource::Attribute` for the colours.** `InstanceColor` is a source of
  its own, keyed per draw like `InstanceMatrix`, so two `InstancedMesh`es
  sharing one material cannot alias each other's colours.
* **The 700-draw random sequence.** Seven draws per instance in page order
  (x, y, z, scale, rot.x, rot.y, rot.z) with the centre guard mutating x and y
  between draws 3 and 4. Nothing touches `Math.random` before the loop, so no
  `skip_random_draws` is needed; a wrong order would move every tetrahedron and
  the grader would be nowhere near 7 pixels.

## Frame structure

The e2e counters agree with the dump's two passes: frame one is 2 draw calls
and 2 pipelines — one indexed draw of 100 instances into the `rgba16float`
800×500 pass target, then `draw( 3, 1, 0, 0 )` into the `rgba8unorm` canvas —
and frames two and three build nothing.

## Left out

* **`premultipliedAlpha`** in `radialBlur`'s options: the page does not set it,
  so the port's `RadialBlurOptions` has no field for it. `premultiplyAlpha` /
  `unpremultiplyAlpha` already exist in the port — they are the two anonymous
  `fn`s of `renderOutput` — so adding it later is a flag, not machinery.
* **`ssaa`'s machinery.** `radial_blur` is a plain node function; nothing in
  `src/nodes/display/` presumes a multi-render effect.
