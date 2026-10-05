# Changelog

All notable changes to `three-rs` are documented here. `sdf-text` and
`three-rs-controls`, the other crates in this workspace, version separately and
have their own sections after the release they ship with. The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

## [Unreleased]

### Added

- **Scene-graph events** (#159, #153 option C): `SceneEvent` (`Added`,
  `Removed`, `ChildAdded(child)`, `ChildRemoved(child)`), `SceneEventType`,
  and `ObjectRef::add_event_listener` / `has_event_listener` /
  `remove_event_listener` / `dispatch_event`, with an opaque
  `ListenerHandle` standing for the listener. `ObjectRef::add`, `remove` (and so
  `clear` and `remove_from_parent`) and `attach` dispatch at three's points
  and in three's order. Dispatch copies the list first, so a listener may
  remove itself. `tests/core_event_dispatcher.rs` ports
  `EventDispatcher.tests.js`; `tests/core_object3d.rs` checks the dispatch
  from the tree methods. No string-keyed dispatcher: `docs/api.md`
  decision 12 says why.
- **`on_before_render` / `on_after_render`** (#159): `Option<Box<dyn ...>>`
  fields on `Object3D` (`ObjectRenderHook`: `FnMut(&ObjectRef, &Renderer,
  &Scene, &dyn RenderCamera, Option<&Group>)`) and on `Scene`
  (`SceneRenderHook`: `Fn(&Renderer, &Scene, &dyn RenderCamera,
  Option<&RenderTarget>)`), with `set_on_before_render` /
  `set_on_after_render` setters. The renderer calls them in
  `_renderScene()`'s order on every scene render (`docs/api.md` decision
  12); `tests/renderer_render_hooks.rs` (GPU) checks the order and the
  arguments. `LightProbeHelper` now installs three's `onBeforeRender()` as
  its hook, so `webgpu_lightprobe` and `webgpu_lightprobe_cubecamera` no
  longer call `helper.update()` each frame (pixel counts unchanged).
- **`dispose()`** (#159): documented no-op aliases for `Drop` on `Texture`,
  `CubeTexture`, `DataTexture`, `Data3DTexture`, `DataArrayTexture`,
  `DepthTexture`, `CubeDepthTexture`, `RenderTarget`, `CubeRenderTarget`,
  `MeshBasicNodeMaterial` (`dispose(self)`) and `BufferGeometry`
  (`dispose(self: Rc<Self>)`), so three.js code ports line for line.

- **`KHR_materials_iridescence` and `webgpu_loader_gltf_iridescence`**
  (graded, 0 pixels; #229): the thin-film half of `PhysicalLightingModel`
  (`evalIridescence` as a WGSL `fn` with its early return and `m <= 2` loop,
  `evalSensitivity`, `Fresnel0ToIor` / `IorToFresnel0`, the iridescent F0s
  `computeMultiscattering` blends towards, and `BRDF_GGX`'s
  `USE_IRIDESCENCE` mix), the `iridescence*` material fields and their
  `materialIridescence*` / `Iridescence*` TSL nodes, and the glTF extension.
  Gated by `gltf_iridescence_lamp_matches_three` against three's dump of the
  page, `tests/gltf_loader.rs`'s `iridescence_lamp_materials` and the e2e
  rung. The page's auto-rotating camera turns three times before the
  screenshot, as three's does (docs/nodes.md §95).
- **`webgpu_loader_gltf_transmission`** (graded, 6 of 100000 pixels; #230):
  `IridescentDishWithOlives.glb` under the blurred `royal_esplanade` PMREM,
  ported forward from `rung-gltf-transmission` now that the Draco decoder
  has landed. The ladder's first graded frame of two stacked transmissive
  materials over one opaque copy, a `thicknessTexture`, a
  `specularColorTexture`, glTF `COLOR_0`, an `AnimationMixer` on a
  non-skinned node and an auto-rotating `OrbitControls` updated with no
  delta. The gold leaf's `alphaMode: MASK` is gated against three's dump of
  the page as the `materialAlphaTest` uniform
  (`gltf_transmission_gold_leaf_alpha_test_matches_three`;
  docs/nodes.md §94).
- **`webgpu_materials_transmission`** (graded, 16 of 100000): a
  `transmission: 1`, `DoubleSide` glass sphere with a striped `alphaMap` in
  front of the royal esplanade. It is the ladder's direct test of the
  transmission path. `tests/nodes_transmission_wgsl.rs` gates both halves of
  the split against three's dump (#228; `docs/nodes.md` §96).
- **`webgpu_postprocessing_ssr_denoise`** (ungraded: three's own e2e
  exception list): the stochastic `ssr()`, `temporal_reproject()` and
  `recurrent_denoise()` in a history loop, added to the beauty, graded and
  resolved by `traa()` and `sharpen()`. `MeshBasicNodeMaterial` gains
  `environment_specular`, the page's `indirectSpecular` patch, and
  `nodes::tsl` gains `material_metalness_value()` /
  `material_roughness_value()`, the map-resolved values the page writes to
  its MRT. The floor material and the SSR, denoise, grading and sharpen
  quads are gated against three's dump of the page, and
  `tests/ssr_denoise_frames.rs` checks the chain converges in place. Also
  fixed on the way: the recurrent denoiser allocates its target before its
  input reads it, a struct-typed var declares its struct, and
  `saturation()` shares its `.rgb` as three does (docs/nodes.md §89).
- **Typed `BufferAttribute`s** (#294): an attribute's array is a `TypedArray`
  of any of three's nine kinds (`Int8` to `Float64`, plus `Float16`), with a
  `normalized` flag, `get`/`set` accessors that decode and encode like
  three's, and `InterleavedBuffer` / `InterleavedBufferAttribute` views over
  a shared stride. The vertex-input `format()` follows three's table
  (`tests/core_vertex_format.rs`, 144 rows), non-normalized 8/16-bit
  attributes are widened to 32-bit on upload as `createAttribute()` does,
  and the renderer's program key carries the geometry's attribute layout.
  `StorageBufferAttribute` backs `storage()` over a typed array.
  `BufferGeometryLoader` reads typed and interleaved attributes;
  `LineSegmentsGeometry` carries three's interleaved instanced views, so
  Line2's vertex inputs match three's fat-lines WGSL. Skinning, `BatchedMesh`
  and `computeVertexNormals()` read and write through the typed accessors.
  WGSL dumps are gated in `tests/nodes_typed_attributes.rs`, the GPU path in
  `tests/renderer_typed_attributes.rs`. See the module doc of
  `core::buffer_attribute` for the design note.
- **Clipping planes** through **`ClippingGroup`** (`objects`), a port of
  `ClippingGroup.js` and `ClippingContext.js`: a group's `clipping_planes`
  clip every descendant, as a union or, with `clip_intersection`, an
  intersection, nested groups combine, and `clip_shadows` carries them into
  the shadow passes. `NodeMaterial.setupClipping()` /
  `setupHardwareClipping()` push three's `clipping()`, `clippingAlpha()`
  (alpha-to-coverage edges under MSAA) or `hardwareClipping()` (WGSL
  `clip_distances`, on an adapter with `CLIP_DISTANCES`). All three modes are
  gated against three's WGSL in `tests/nodes_clipping_wgsl.rs`. See
  `docs/nodes.md` §6.
- **Stencil state on materials**: `stencil_write`, `stencil_func`,
  `stencil_ref`, `stencil_func_mask`, `stencil_write_mask`, `stencil_fail`,
  `stencil_z_fail` and `stencil_z_pass` (`StencilFunc`, `StencilOp`), and
  `color_write`, as pipeline state per draw. They take effect on a renderer
  created with `RendererParameters::stencil`, whose depth buffer is then
  `depth24plus-stencil8`.
- **`webgpu_clipping`** and **`webgpu_clipping_stencil`**, graded green at 17
  and 0 of 100000 pixels, in the steady-frame strip, the native viewer and
  the browser shell.
- **The core helpers** (`helpers`, #300): `AxesHelper`, `ArrowHelper`,
  `BoxHelper`, `Box3Helper`, `PlaneHelper`, `PolarGridHelper`,
  `DirectionalLightHelper`, `HemisphereLightHelper`, `PointLightHelper`,
  `SpotLightHelper` and `SkeletonHelper`, ports of `src/helpers/`. Each
  keeps three's geometry, material, flags and child transforms, and
  `tests/helpers_core.rs` compares all of them node for node against
  three's own classes under node (`tools/helpers_reference.mjs`, 24
  scenarios). `Box3Helper`, `PlaneHelper` and `SkeletonHelper`'s
  `updateMatrixWorld` overrides are an explicit `update_matrix_world(force)`
  the caller runs each frame; see the `helpers` module docs.
- **`ssr()`'s stochastic path** (`nodes::display`): `SsrOptions` gains
  `stochastic`, `reflect_non_metals`, `environment`,
  `env_importance_sampling`, `diffuse` and `binary_refine`, and `SsrNode`
  gains `set_env_map`, `set_history`, `render_target` and the mirror-bias,
  screen-edge-fade and environment-intensity uniforms. This ports the rest
  of `examples/jsm/tsl/display/SSRNode.js`: GGX-sampled rays jittered by a
  per-frame noise index, binary refinement, the environment fallback on a
  miss and the multi-bounce history. The three SSR quads of
  `tools/dump-pages/ssr_stochastic.html` are gated against three's dump in
  `tests/nodes_display_wgsl.rs`, and `tests/ssr_stochastic_frames.rs` checks
  a mirror floor's reflection. See `docs/nodes.md` §65.3.
- **`temporal_reproject`** (`nodes::display`), a port of
  `examples/jsm/tsl/display/TemporalReprojectNode.js`, the temporal stage of
  `webgpu_postprocessing_ssr_denoise`'s denoiser. It reprojects a history
  along the velocity attachment with a depth- and normal-weighted 4-tap
  fetch, clips it to the neighbourhood's YCoCg variance box, and writes
  `1 / frameCount` in alpha. It covers both modes (`Specular` adds the
  parallax hit-point history), `accumulate`, `set_history_texture()`, and
  the `max_frames`, `hit_point_reprojection`, `clamp_intensity` and
  `flicker_suppression` uniforms as `SettableValue`s. The seed quad and both
  resolve quads are gated against three's dump in
  `tests/nodes_display_wgsl.rs`, and `tests/temporal_reproject_frames.rs`
  checks the history on the GPU. See `docs/nodes.md` §87.
- **`taau`** (`nodes::display`), a port of
  `examples/jsm/tsl/display/TAAUNode.js`: temporal anti-aliased upsampling
  of a reduced-resolution scene pass to the drawing buffer's size, with the
  page **`webgpu_upscaling_taau`** in the native viewer. The page is
  ungraded: three.js misses its own reference on this machine (540 of
  100000 pixels), and the port scores 539, so its e2e rung is ignored. The
  seed and resolve quads are gated against three's dump, and
  `tests/taau_frames.rs` checks the node on the GPU. The `TAAUtils.js`
  helpers it shares with `traa` now live in one crate-private module. See
  `docs/nodes.md` §90.
- **`ImportanceSampledEnvironment`** and **`EnvMapCdfGenerator`**
  (`nodes::display`), with the SpecularHelpers microfacet helpers
  (`d_gtr`, `ggx_reflection_sample`, `mis_power_heuristic`, …) and
  `bind_analytic_noise` in `nodes::tsl`. These port
  `examples/jsm/tsl/display/ImportanceSampledEnvironment.js`,
  `tsl/utils/SpecularHelpers.js` and `tsl/utils/RNoise.js`: the CPU
  luminance CDF tables and the reflect, BRDF and MIS environment lookups the
  SSR-denoise stack shares. The WGSL is gated against three's dump of
  `tools/dump-pages/specular_helpers.html` in `tests/nodes_display_wgsl.rs`,
  and the CDF tables by a hand-computed 4×2 unit test.
- **`WaterMesh`** (`addons::objects`), a port of
  `examples/jsm/objects/WaterMesh.js`: a planar `reflector()` distorted by
  four scrolling taps of a normal map, with a sun highlight and a Fresnel mix
  into the water colour. Every uniform is a public `SettableValue`.
  `webgpu_ocean` is graded green at 0 of 100000 pixels, and the material's
  WGSL is gated against three's dump in `tests/nodes_water_wgsl.rs`.
- **`Water2Mesh`** (`addons::objects`), a port of
  `examples/jsm/objects/Water2Mesh.js`: two normal maps scrolled along a
  flow direction or a flow map and cross-faded on a half cycle, with a
  Fresnel mix of a refraction (`viewportSharedTexture( viewportSafeUV() )`)
  and a planar `reflector()`. `flowConfig` advances by the frame's delta
  time in the node's `updateBefore`. Every uniform is a public
  `SettableValue`. Both flow branches' WGSL is gated against three's dumps
  in `tests/nodes_water_wgsl.rs`, and `tests/water2_frames.rs` checks the
  flow, the refraction and the tint on the GPU. `examples/webgpu_water.rs`
  ports the page. Three's e2e skips the page, so it has no rung; scored
  informally, its first frame is 0.006% off three's screenshot. See
  `docs/nodes.md` §83.
- **`transmission_map`** on materials, three's `transmissionMap`: its red
  channel multiplies `transmission`. The glTF loader reads
  `KHR_materials_transmission.transmissionTexture` into it.
- **`ReflectorNode::add_target_on_setup()`**: adds the mirror's `target` to
  an object just before the reflector's first update, which is when three
  runs an `add()` written inside a material's `Fn()`. See `docs/nodes.md`
  §76.
- **`nodes::display::sss`** (`SssNode`), a port of `SSSNode.js`. It casts
  screen-space shadows by marching a ray from each pixel towards one light
  through a depth pre-pass. `maxDistance`, `thickness`, `shadowIntensity` and
  `quality` are `SettableValue`s, and `set_resolution_scale` and
  `set_use_temporal_filtering` stand in for three's properties. Its shader is
  gated against three's dump in `tests/nodes_display_wgsl.rs`, and its frames
  in `tests/sss_frames.rs`. See `docs/nodes.md` §71.
- **`builtinShadowContext`**: `PassNode::set_context_shadow( shadow, &light )`
  multiplies `shadow` into that light's colour, after its shadow-map factor,
  for every draw the pass makes where the light's shadow map applies.
  `clear_context_shadow()` removes it. The page's ground material, with the
  context, is gated against three's dump.
- **`webgpu_postprocessing_sss`**, the page: a depth and velocity pre-pass,
  SSS from the directional light with temporal filtering, and TRAA resolving
  the scene pass. It is in three's e2e exception list, so it is in the native
  viewer and not graded.
- **`ssgi()` / `SsgiNode`** (`nodes::display`), a port of
  `examples/jsm/tsl/display/SSGINode.js`. It is screen space global
  illumination with a visibility bitmask, and writes an AO and a one-bounce
  GI texture. Every option is a public `SettableValue`, and
  `set_use_temporal_filtering()` is three's property. The SSGI shader, the
  page's composite and its TRAA resolve are gated against three's dump in
  `tests/nodes_display_wgsl.rs`, and `tests/ssgi_frames.rs` checks the
  frames, the three boolean options included. `webgpu_postprocessing_ssgi`
  is ported, ungraded because three's e2e harness skips it, and in the
  viewer. An arbitrary `normalNode`, `normalNode = null`, a logarithmic
  depth buffer, the `SSGI.AO` name, `contextNode` and `dispose()` are not
  ported. Without `RG11B10UFLOAT_RENDERABLE` the effect fails, as three's
  does.
- The renderer requests `RG11B10UFLOAT_RENDERABLE` when the adapter has it.
- **`nodes::display::ao`** (`GtaoNode`), a port of `GTAONode.js`: ground
  truth ambient occlusion from a depth and a packed-normal pre-pass, with
  `radius`, `thickness` and `scale` as `SettableValue`s and `set_samples`,
  `set_resolution_scale` and `set_use_temporal_filtering` as three's
  properties. Its shader is gated against three's dump in
  `tests/nodes_display_wgsl.rs`, its frames in `tests/gtao_frames.rs`. See
  `docs/nodes.md` §64.
- **`builtinAOContext`**: `PassNode::set_context_ao( node )` multiplies an
  occlusion into every non-transparent material the pass draws, through the
  same `AmbientOcclusion` property an `aoMap` writes
  (`SetupContext::ambient_occlusion`). Basic, Lambert, Phong, Toon, Standard
  and Physical read it.
- **`webgpu_postprocessing_ao`**, the page: a normal + velocity pre-pass,
  GTAO at half resolution with temporal filtering, and TRAA resolving it.
  It is in three's e2e exception list, so it is in the native viewer and
  not graded.
- **`tsl::depth_texture_gather`** (`texture( depth ).gather()`), with the
  non-filtering sampler binding a gathered depth texture needs;
  `tsl::get_screen_position_from_clip`;
  `TraaNode::set_use_subpixel_correction`.
- **Texture wrapping on `textureLoad`**: an unfilterable (`NearestFilter`)
  texture is read through three's `tsl_coord_<S>S_<T>T_2d` wrap function
  built from its `wrapS` / `wrapT`, instead of always clamping.
- **`TransformControls`** (`addons::controls`), a port of
  `examples/jsm/controls/TransformControls.js` as of the pinned 5f610f5
  (r187dev): the translate, rotate and scale gizmo with the same picker,
  gizmo and helper graph, the same drag math, snaps and limits, and
  perspective and orthographic cameras. Pointer input arrives as method calls
  (`on_pointer_down`, `pointer_hover`, …), events are returned as
  `Vec<TransformControlsEvent>`, and the host calls `update(camera)` where
  three's renderer runs the helper's matrix update. `toneMapped: false` is
  dropped, and `enabled`, `show*` and `min*`/`max*` are plain fields that send
  no events. `tools/transform_controls_reference.mjs` runs three's class under
  node through 17 scenarios and writes `tests/fixtures/transform_controls.json`;
  `tests/addons_transform_controls.rs` replays them and matches every event,
  transform, handle state and working vector to 1e-9, and compares the gizmo
  graph node for node. See `docs/controls.md`.
- **`FirstPersonControls`** and **`FlyControls`** (`addons::controls`), ports
  of `examples/jsm/controls/FirstPersonControls.js` and `FlyControls.js` as
  of the pinned 5f610f5 (r187dev), with the same fields, defaults and `update( delta )`. Input
  arrives as method calls (`pointer_down`, `pointer_move`, `key_down` with a
  new `KeyCode`, …) rather than DOM listeners, the camera is passed to
  `update` as a `&ObjectRef`, and Fly's `change` event is `update`'s return value.
  `tools/first_person_controls_reference.mjs` and
  `tools/fly_controls_reference.mjs` run three's own classes under node over
  scripted input and write `tests/fixtures/first_person_controls.json` and
  `fly_controls.json`; `tests/addons_first_person_controls.rs` and
  `tests/addons_fly_controls.rs` replay the scripts and match every camera
  position and quaternion to 1e-9. See `docs/controls.md`.
- **TSL sweep 1**: the last absent `three/tsl` math names and nine texture
  utilities (`texture_bicubic_level` is now public), each gated against three's own WGSL dump in
  `tests/nodes_tsl_batch.rs`.
  - Packing: `pack_snorm_2x16`, `pack_unorm_2x16`, `pack_half_2x16`,
    `pack_snorm_4x8`, `pack_unorm_4x8` and the five matching `unpack_*`,
    which print WGSL's `pack2x16snorm` family.
  - Packed 4x8 integers: `pack_4x_i8`, `pack_4x_u8`, `pack_4x_i8_clamp`,
    `pack_4x_u8_clamp`, `unpack_4x_i8`, `unpack_4x_u8`, `dot_4u8_packed` and
    `dot_4i8_packed`. These print the native builtins. Three's emulation for
    devices without `packed_4x8_integer_dot_product` is not ported.
  - `any` (function and method), `transform_normal_by_view_matrix`,
    `transform_normal_by_inverse_view_matrix`, and the deprecated spellings
    `faceforward` and `inversesqrt`. `all` already existed; it is now gated.
  - Textures: `equirect_direction`, `matcap_uv`, `max_mip_level`,
    `spritesheet_uv`, `triplanar_textures`, `texture_bicubic`,
    `texture_bicubic_level`, `texture_3d_load` and `texture_3d_level`. The
    functions that take a texture node in three take the `Texture` here,
    as `triplanar_texture` already did.
- **TSL sweep 2**: thirty-one `three/tsl` accessors, each gated against three's
  own WGSL dump in `tests/nodes_tsl_batch.rs`. See `docs/nodes.md` §67.
  - Tangent frame: `bitangent_geometry`, `bitangent_local`, `bitangent_world`,
    `tangent_world`, and `tangent_geometry` and `tangent_local`, now public.
    Also `parallax_direction` and `parallax_uv`.
  - Camera: `camera_normal_matrix`. `camera_near` and `camera_far` already
    existed and are now gated.
  - Model and object: `model_direction`, `model_position`, `model_scale`,
    `model_view_position` and `model_radius`, plus the `object_*` forms of
    each, which take the target `&ObjectRef`. Also `mediump_model_view_matrix`,
    `highp_model_view_matrix` and `highp_model_normal_view_matrix`.
  - `transform_normal` (function and method), `transform_normal_to_view`,
    `reflect_view`, `refract_view`, `refract_vector` and `clip_space`.
  - `material_refraction_ratio`, with a new
    `MeshBasicNodeMaterial::refraction_ratio`: 0.98 from `new`, `lambert`
    and `phong`, whose three.js materials have `refractionRatio`, and 0
    from the other constructors.
  - `reflect_vector` is now cached per normal, like `refract_vector`.
  - `object_direction` refreshes the target's world matrix and negates a
    camera's direction, as `getWorldDirection()` does.
  - A geometry without a `tangent` attribute no longer fails to draw when
    the material reads the tangent in the vertex stage. The attribute
    becomes three's `vec4( 0, 0, 0, 1 )` constant, with three's warning.
  - `clip_space` warns and yields `vec4()` outside the fragment stage.
  - `webgpu_tsl_raging_sea` uses the crate's `transform_normal_to_view` in
    place of its local helper.
- **TSL sweep 3**: twenty-nine display, lighting and material `three/tsl` names.
  Every shader-building addition is gated against three's own WGSL dump in
  `tests/nodes_tsl_batch.rs`; `get_texture_index`, a CPU helper, is
  unit-tested only. Divergences are recorded in `docs/nodes.md` §68.
  - Depth: `view_z_to_reversed_orthographic_depth`,
    `orthographic_depth_to_view_z`, `view_z_to_reversed_perspective_depth`,
    `view_z_to_logarithmic_depth` and `logarithmic_depth_to_view_z`.
  - Colour: `blend_burn`, `blend_dodge`, `blend_screen`, `blend_color`,
    `vibrance`, `cdl` and `cineon_tone_mapping`. `ToneMapping::Cineon`
    selects it as a material or pass tone mapping, as `CineonToneMapping`
    does.
  - Screen: `get_screen_position`, `get_normal_from_depth`,
    `viewport_coordinate` and `viewport_uv`. `screen_size` already existed
    and is now gated. `direction_to_face_direction`, three's deprecated alias
    of `negateOnBackSide`, takes the material side as a parameter.
  - Passes: `depth_pass( scene, camera )`, whose node is the scene's linear
    depth (`PassNode::a` follows it), and `PassNode::linear_depth_node`.
    `nodes::get_texture_index` is the MRT name lookup; it takes attachment
    names and returns an `Option`.
  - Lighting: `light_projection_uv`, `direct_point_light` (returns the
    `( lightDirection, lightColor )` pair) and `get_parallax_correct_normal`.
    `shadow_matrix( i )` now returns one shared node per light, as three's
    `lightShadowMatrix` does. The renderer now refreshes `light.shadow.matrix`
    every render for a light whose shadow is not rendered (`cast_shadow` or
    `shadow_map_enabled` off), as `lightShadowMatrix`'s render update does,
    so `light_projection_uv` follows a light that casts nothing.
  - Material: `material_normal`, `material_clearcoat_normal`,
    `material_specular_strength`, `material_light_map` and `material_ao`.
    Each takes the material it reads the maps from; `material_normal` and
    `material_clearcoat_normal` also read its `side` and `flat_shading`. Also
    `material_point_size` and `point_width`. New `MeshBasicNodeMaterial` fields:
    `light_map`, `light_map_intensity` (default 1), `specular_map` and
    `size` (default 1). The material flows do not apply `light_map` or
    `specular_map`; the renderer warns once per material that only the
    accessors read them, and `check_supported()` does not fail on them.
- **TSL sweep 4**: the `three/tsl` utils names. Each one that emits WGSL is
  gated against three's own dump in `tests/nodes_tsl_batch.rs`. See
  `docs/nodes.md` §70.
  - Context: `uniform_flow` makes a two-branch `select()` print WGSL's
    `select()`. `set_name` names the first uniform built under it, unless
    that uniform has a name of its own; either way the name is used up.
    `label` is three's deprecated alias of `set_name`. MaterialX's
    `mx_select` and `mx_negate_if` now use `uniform_flow`, with
    byte-identical output.
  - The method `NodeRef::set_name` (and `label`) on a uniform renames it in
    place and returns it, as three's `UniformNode.setName()` does.
    `UniformNode::name` is now a `Cell`.
  - `bypass`, `vertex_stage` (method `to_vertex_stage`), `unpack_normal`,
    `unpack_rgb_to_normal`, `expression( snippet, type )`, `debug` with an
    optional `DebugCallback`, `sample` with its `SampleNode` handle, and raw
    `wgsl( code, includes )` for a `wgsl_fn`'s includes. `direction_to_color`
    and `color_to_direction` are ported as three's deprecated aliases.
  - Event hooks: `on_object_update`, `on_material_update`, `on_frame_update`,
    `on_after_object_update`, `on_before_object_update`,
    `on_before_material_update` and `on_before_frame_update`. Each is a
    `void` node attached with `.bypass()`. Its callback runs in three's update
    phase and receives the `Renderer`.
- **TSL sweep 5**: lighting and material `three/tsl` names, in
  `nodes::tsl`. Each one that emits WGSL is gated against three's own dump in
  `tests/nodes_tsl_batch.rs`. See `docs/nodes.md` §78.
  - `material_anisotropy( material )`. The physical material's anisotropy
    setup now reads it, and its WGSL is unchanged.
  - `d_ggx_anisotropic`, `v_ggx_smith_correlated_anisotropic` and
    `schlick_to_f0`. These are standalone: the physical lighting still
    evaluates only the isotropic lobe.
  - `ltc_uv`, `ltc_evaluate` and `ltc_evaluate_volume`. These are standalone
    too, because there is no `RectAreaLight`.
  - `lights( indices )` builds `MeshBasicNodeMaterial::lights_node`.
    `webgpu_lights_selective` uses it.
- **TSL sweep 6**: the compute, storage and subgroup `three/tsl` names, in
  `nodes::tsl`, each gated against three's own dump in
  `tests/nodes_tsl_batch.rs`. See `docs/nodes.md` §84.
  - The subgroup family: `subgroup_add`, `subgroup_mul`, `subgroup_and`,
    `subgroup_or`, `subgroup_xor`, `subgroup_min` and `subgroup_max` with
    their inclusive and exclusive scans where three has them, `subgroup_all`,
    `subgroup_any`, `subgroup_ballot`, `subgroup_elect`,
    `subgroup_broadcast_first`, `subgroup_broadcast`, `subgroup_shuffle`
    and its `_xor`/`_up`/`_down` forms, `quad_swap_x`/`_y`/`_diagonal` and
    `quad_broadcast`. Also the builtins `subgroup_size`, `subgroup_index` and
    `invocation_subgroup_index`. A stage that uses any of them gets
    `enable subgroups;`. The renderer requests `wgpu::Features::SUBGROUP`
    when the adapter has it. Without the feature, a subgroup kernel or a
    material whose fragment stage uses subgroups logs three's message and is
    skipped.
  - Known gaps: naga 30 cannot parse `subgroup_elect`, and accepts only `u32`
    ids for `subgroup_broadcast`, `subgroup_shuffle` and `quad_broadcast`.
    Three cannot build `quadBroadcast` at all; the port takes the id it
    needs.
  - `attribute_array`: the `StorageArray` whose `to_attribute()` steps once
    per vertex rather than once per instance.
  - `storage_element`, `atomic_func` (public now, under three's name) and
    `texture_barrier`. `storage_texture_3d` is now gated.
- **`SkyMesh`** (`addons::objects`), a port of `examples/jsm/objects/SkyMesh.js`.
  It is the Preetham daylight model with a sun disc and an fbm cloud layer.
  Every uniform is a public `SettableValue`. `webgpu_sky` is graded green at 0
  of 100000 pixels, and its WGSL is gated against three's dump in
  `tests/nodes_sky_wgsl.rs`.
- **`CubeCamera`** and **`CubeRenderTarget`**: `new CubeCamera( near, far,
  renderTarget )` and `update( renderer, scene )` render the scene into a
  cube's six faces. `activeMipmapLevel` is not ported.
- **`tsl::to_var_intent()`**, the assigned form of three's `toVarIntent()`.
  It is a function-scope `var` declared where it is first built. See
  `docs/nodes.md` §59.
- **`LightProbe`**, a light holding nine spherical-harmonic coefficients
  (`SphericalHarmonics3`) that adds `getShIrradianceAt( normalWorld )` to a
  lit material's irradiance and nothing to its radiance, through the same
  `setupLight` funnel as the other lights, so Standard, Physical, Phong,
  Lambert, Toon and custom lighting models all take it. The coefficients ride
  a per-draw uniform array premultiplied by the intensity, so changing them
  rebuilds nothing. `tsl::get_sh_irradiance_at` is the TSL function.
- **`addons::lights::LightProbeGenerator`**: `from_cube_texture` projects an
  RGBA8 `CubeTexture`'s decoded faces on the CPU, `from_cube_render_target`
  reads a rendered cube back (RGBA8 or half float) and projects that. Both
  are checked against three's own `LightProbeGenerator.js`, the first under
  node (`tools/light_probe_generator_reference.mjs`), the second against the
  first on the same environment.
- **`addons::helpers::LightProbeHelper`**, a sphere showing a probe's
  irradiance over π.
- Rungs `webgpu_lightprobe` and `webgpu_lightprobe_cubecamera`. The second
  is native only for now: its readback blocks, which the browser cannot do,
  so `tools/web_gate.skip` (new) lists it.
- **Screen reads** (#169): `viewportSharedTexture`, `viewportTexture`,
  `viewportDepthTexture`, `viewportLinearDepth` and `viewportSafeUV`, the
  `screenSize` and `screenCoordinate` scopes, and the `cameraNear` and
  `cameraFar` uniforms. The renderer copies the framebuffer before the first
  draw that reads it, in three's opaque/transparent order. A pass that reads
  nothing is recorded as before. See `docs/nodes.md` §61.
- **`backdropNode` and `backdropAlphaNode`** on node materials, a custom
  `lighting_model` included. A backdrop material goes in the transparent
  list. On a Normal material the field is loud (`docs/api.md` decision 7).
- TSL `grayscale`, `posterize` and `blendOverlay`.
- `webgpu_backdrop`, graded green (23 pixels, the same as three's own frame).
  `webgpu_refraction` is ported, but not graded on this machine, because
  three.js itself fails its reference there.
- **`velocity`** (`nodes::velocity`): the screen-space motion since the last
  frame, as an MRT member, with three's previous-frame model, view and
  projection matrices. The history lives in the renderer's `NodeFrameState`.
  Skinned meshes skin `positionPrevious` with last frame's bones when the MRT
  has `velocity`. `Renderer::set_velocity_projection_matrix()` is
  `velocity.setProjectionMatrix()`. (#163)
- **`motion_blur`** (`nodes::display`), `MotionBlur.js`. (#163)
- **`webgpu_postprocessing_motion_blur`** is graded: 5 of 100000 pixels. Its
  graded frame has zero velocity, so it proves the page builds and composes.
  `tests/velocity_frames.rs` checks the motion on the GPU. (#163)
- **`traa`** (`nodes::display`), `TRAANode.js` with `TAAUtils.js`:
  temporal reprojection anti-aliasing. `TraaNode::attach()` installs its
  camera jitter on a `RenderPipeline`, where three's `setup()` does it
  itself. Three's resolve dump gates the shader; three lists the page in its
  e2e exception list, so it has no graded rung. `tests/traa_frames.rs` checks
  it over sixteen frames. (#165)
- **`webgpu_postprocessing_traa`** in the native viewer. (#165)
- **Node graph pieces** used by TRAA: struct values (`struct_new`,
  `struct_get`), `texture_load`, `texture_load_offset`,
  `depth_texture_load`, `all`, `view_z_to_perspective_depth` and
  `get_view_position`. Also `Renderer::init_render_target` and
  `RenderPipeline::claim_view_offset`. (#165)
- **`retro_pass`** (`nodes::display`), `RetroPassNode.js`: a scene pass at a
  quarter of the canvas, nearest-filtered, that draws classic materials with
  snapped vertices, level-0 textures and optional affine mapping. A node
  material is drawn as itself, as in three. See `docs/nodes.md` §79.
- **The CRT effects** of `CRT.js` (`barrel_uv`, `barrel_mask`,
  `color_bleeding`, `scanlines`, `vignette`), `circle` from `Shape.js`, and
  `bayer_dither` from `Bayer.js`.
- **`film`, `sepia` and `bleach`**, from `FilmNode.js`, `Sepia.js` and
  `BleachBypass.js`. No three page uses them, so they are gated against
  `tools/dump-pages/film_sepia_bleach.html`. See `docs/nodes.md` §80.
- **`tsl::replace_default_uv`** and **`MeshBasicNodeMaterial::context_node`**.
  `texture()` now takes its uv from a `getUV` in the build context, and a
  `getTextureLevel` makes it sample at that level.
- **`PassNode::set_resolution_scale`** and a crate-private per-draw
  render-object function on `PassNode`.
- **glTF `KHR_materials_unlit`**: such a material loads as a basic material.
- **`webgpu_postprocessing_retro`** in the native viewer. three.js misses its
  own reference for the page on this machine (1503 of 100000 pixels), so its
  e2e rung is ignored. Three's dump gates its two post-processing shaders,
  and `tests/retro_frames.rs` checks the pass on the GPU.
- **`oit_pass`** (`nodes::display`), `OITPassNode.js`: weighted blended
  order-independent transparency. Transparent `NormalBlending` materials
  accumulate into an `rgba16float` / `r8unorm` pair that shares the pass's
  depth, and the composite does not depend on draw order.
  `webgpu_oit` is graded at 0 of 100000 pixels. Its shaders are gated against
  three's dump and `tests/oit_frames.rs` checks order independence on the
  GPU. See `docs/nodes.md` §82.
- **`BlendMode` is public** (`materials`), with `BlendMode::new( blending )`
  and `From<Blending>`. `MrtNode::set_clear_color` / `clear_color` are three's
  `setClearColor` / `getClearColor`: a per-attachment clear value.
- **`RenderTarget::set_texture_name( 0, name )`** names the first
  attachment, which used to answer only to `output` and panic otherwise.
- **`StereoCamera`** (`cameras`), a port of `StereoCamera.js`, and
  **`addons::camera_utils::frame_corners`**, `CameraUtils.frameCorners()`.
  Both are checked against three's own code run under node
  (`tools/stereo_camera_reference.mjs`).
- **The stereo display passes** (`nodes::display`): `stereo_pass`,
  `anaglyph_pass` (all seven `AnaglyphAlgorithm`s in all three
  `AnaglyphColorMode`s) and `parallax_barrier_pass`. The two composite quads
  are gated against three's dumps, and `tests/stereo_frames.rs` checks where
  each eye lands. See `docs/nodes.md` §81.
- **`webgpu_display_stereo`** is graded: 0 of 100000 pixels.
- **`outline`** (`nodes::display`), `OutlineNode.js`: selection outlines,
  with visible and hidden edges, `edgeThickness`, `edgeGlow` and
  `downSampleRatio`. Its two scene renders go through a crate-private
  renderer hook that stands in for `setRenderObjectFunction()`. The depth
  and mask scene materials, the copy, edge-detection, X-blur and composite
  quads, and the page's output are gated against three's dump; the sprite
  depth and mask materials (the page has no sprites, so the dump has none)
  and the Y blurs (one module with the X blur in three) are not.
  `tests/outline_frames.rs` checks what a selection draws.
- **`webgpu_postprocessing_outline`** is graded: 15 of 100000 pixels, the
  same as three's own frame. Nothing is selected in the graded frame.
- **`lut_3d`** (`nodes::display`), `Lut3DNode.js`, and
  **`tsl::texture_3d_sampled`**, `texture3D( texture )` with no level.
- **`LutCubeLoader`**, **`Lut3dlLoader`** and **`LutImageLoader`**
  (`loaders`). Each is checked byte for byte against three's own loader,
  quirks included (`tests/loaders_lut.rs`).
- **`webgpu_postprocessing_3dlut`** is graded: 0 of 100000 pixels.
  `tests/lut_3d_frames.rs` checks intensity and table swaps.
- **`ObjLoader`** (`loaders`), `OBJLoader.js` for meshes: `v`, `vn`, `vt`,
  `f`, `o`, `g`, `s`, `usemtl` and `mtllib`. `l` and `p` elements are
  refused. There is no `MTLLoader`. Checked against three's parse
  (`tests/loaders_obj.rs`).
- **`ssr`** (`nodes::display`), `SSRNode.js`: screen-space reflections, a
  march through the depth buffer with roughness taken from a blurred mip
  chain. `stochastic`, `binaryRefine`, `reflectNonMetals`,
  `screenEdgeFadeBlack`, multi-bounce history and orthographic cameras are
  not ported. See `docs/nodes.md` §65.
- **`smaa`** (`nodes::display`), `SMAANode.js`: SMAA 1x with colour edge
  detection, with three's area and search textures.
- **`webgpu_postprocessing_ssr`** is graded: 4 of 100000 pixels, with the
  reflections in the graded frame. Its six fragment shaders are gated against
  three's dumps.
- `webgpu_postprocessing_smaa` and `webgpu_postprocessing_pixel` are ported
  and in the native viewer, but not graded on this machine, because three.js
  itself fails their references there (258 and 405 pixels). The port scores
  the same 258 and 405, and differs from three's own frames in no graded
  pixel.
- **`Renderer::size()`**, three's `renderer.getSize()`: the canvas size in
  logical pixels.
- **Rendering into a mip level**: `RenderTarget::set_mip_level_count()`,
  `Renderer::set_render_target_level()` (three's `setRenderTarget( rt, 0,
  level )`) and `Renderer::active_mipmap_level()`.
- TSL `continue_loop()` (`Continue()`) and `get_screen_position()`.
- **`dof`** (`nodes::display`), `DepthOfFieldNode.js`: bokeh depth of field
  in nine full-screen draws. All seven distinct quad shaders are gated against
  three's dump (`dof_*` in `tests/nodes_display_wgsl.rs`). three lists the
  page in its e2e exception list, so it has no graded rung.
  `webgpu_postprocessing_dof` is in the native viewer. See `docs/nodes.md`
  §66.
- **`webgpu_postprocessing_dof_basic`** is graded: 36 of 100000 pixels. It
  is the page's own `boxBlur` + `smoothstep` mix, not `DepthOfFieldNode`.
- **`tsl::output_struct()`**, `outputStruct()` as a material's `outputNode`.
  Each member keeps its own type, where an `mrt()` member takes its
  attachment's type.
- **Red render targets**: `RenderTarget::set_red_format()`. A texture node
  over a one-channel map is now a `float` node read as `.x`, as three's
  `getTextureType()` makes it.
- **`tsl::uniform_array_vec2()`** and `UniformArray::element_xy()`.
- **`Scene::environment_rotation`**, `scene.environmentRotation`, applied
  through `materialEnvRotation`.
- **`godrays`**, **`bilateral_blur`** and **`depth_aware_blend`**
  (`nodes::display`), ports of `GodraysNode.js`, `BilateralBlurNode.js` and
  `depthAwareBlend.js`. The godrays node ray-marches a point light's cube
  shadow map. The `DirectionalLight` branch is not ported. Three's dump
  gates all three shaders. `webgpu_postprocessing_godrays` is graded green
  at 3 of 100000 pixels. See `docs/nodes.md` §74.
- **`lensflare`** (`nodes::display`), a port of `LensflareNode.js`.
  `webgpu_postprocessing_lensflare` is graded green at 0 of 100000 pixels.
  Each quad the page adds is gated against three's dump. See `docs/nodes.md`
  §75.
- **`LightShadow::point_depth_texture()`**, three's
  `light.shadow.map.depthTexture` for a point light. The renderer draws the
  light's shadow into the texture it returns.
- **`Scene::background_intensity`** and **`Scene::environment_intensity`**,
  three's `scene.backgroundIntensity` and `scene.environmentIntensity`.
- **`tsl::const_array_of`**, a literal array of vectors, and
  `UniformArray::element_xyz`.
- **`sharpen()` / `SharpenNode`** (`nodes::display`), a port of
  `examples/jsm/tsl/display/SharpenNode.js`: AMD FidelityFX FSR 1's RCAS,
  a contrast-limited five-tap sharpen drawn once a frame into a half-float
  target, with optional noise attenuation. `sharpness` is a number (a
  constant, as in three) or any float node, such as a `uniform_settable`.
  Both variants' WGSL is gated against three's dump of
  `tools/dump-pages/sharpen.html` in `tests/nodes_display_wgsl.rs`, and
  `tests/sharpen_frames.rs` checks on the GPU that it steepens a soft edge
  without moving flat regions. See `docs/nodes.md` §86.
- **`recurrent_denoise()` / `RecurrentDenoiseNode`** (`nodes::display`), a
  port of `examples/jsm/tsl/display/RecurrentDenoiseNode.js`: the
  edge-aware eight-tap Vogel-disk denoiser of the SSR-denoise stack, in
  `'diffuse'` or `'specular'` mode, with luma, plane, lobe-normal, albedo,
  roughness and ray-length or AO edge stopping, and an optional Karis
  temporal blend that writes the frame weight to alpha. Every three uniform
  is a public `SettableValue`; `set_alpha_source` rebuilds the shader. Both
  of `tools/dump-pages/recurrent_denoise.html`'s quads (diffuse with AO,
  and the page's specular configuration) are gated against three's dump in
  `tests/nodes_display_wgsl.rs`, with function gates for the layouted
  helpers, and `tests/recurrent_denoise_frames.rs` checks on the GPU that it
  cuts a noisy face's variance without moving its mean or its silhouette,
  and keeps cutting it as frames accumulate. See `docs/nodes.md` §88.
- **`denoise()` / `DenoiseNode`** (`nodes::display`), a port of
  `examples/jsm/tsl/display/DenoiseNode.js`: the single-pass 16-tap
  Poisson-disk denoiser with luma, depth-plane and normal edge stopping,
  rotated per pixel by a 64x64 simplex-noise texture; the normal input is
  optional (`None` reconstructs it from depth). With it,
  **`addons::simplex_noise::SimplexNoise`**, a port of
  `examples/jsm/math/SimplexNoise.js` (2D, 3D and 4D) with a unit test
  against three's JS. Both of `tools/dump-pages/denoise.html`'s quads are
  gated against three's dump in `tests/nodes_display_wgsl.rs`, and
  `tests/denoise_frames.rs` checks on the GPU that it flattens a
  pixel-scale checkerboard while keeping a depth edge sharp. See
  `docs/nodes.md` §92.
- **`ssao()` / `SsaoNode`** (`nodes::display`) and
  **`tsl::depth_aware_blur`**, ports of
  `examples/jsm/tsl/display/SSAONode.js` and `depthAwareBlur.js`:
  Vogel-disk SSAO with one depth tap per sample, drawn into its own target
  at `resolutionScale` and cleaned by a separable five-tap blur weighted by
  view-Z difference (`blurEnabled`, `blurSharpness`). The AO and blur quads
  are gated against three's dump of `tools/dump-pages/ssao.html` in
  `tests/nodes_display_wgsl.rs`, and `tests/ssao_frames.rs` checks on the
  GPU that a box's crease is darker than open floor and that the blur cuts
  the pixel-to-pixel noise. See `docs/nodes.md` §93.
- **`fsr1()` / `Fsr1Node`** (`nodes::display`), a port of
  `examples/jsm/tsl/display/FSR1Node.js`: AMD FidelityFX Super Resolution
  1.0. It draws two half-float passes a frame at the drawing-buffer size.
  EASU is edge-adaptive Lanczos2 upsampling over a 12-texel footprint, and
  RCAS is `SharpenNode`'s sharpen over the result. The EASU and RCAS WGSL
  is gated against three's dump of `webgpu_upscaling_fsr1` in
  `tests/nodes_display_wgsl.rs`. `tests/fsr1_frames.rs` checks on the GPU
  that a half-resolution pass comes out at canvas size, with flat regions
  unchanged and an edge steeper than bilinear's. See `docs/nodes.md` §91.
- **`webgpu_upscaling_fsr1`**, ported but not graded: three.js scores 703 of
  100000 pixels against its own reference on this machine, over the 0.1%
  limit, and the port scores 698. Its e2e rung is `#[ignore]`d. It is in
  the steady-frame strip and the native viewer, but not the gallery or the
  browser shell.

### Changed

- **The scene-graph handle is `ObjectRef`, not `Node`** (#250, breaking).
  `core::Node` is renamed `ObjectRef` and `WeakNode` is `WeakObjectRef`;
  the crate-root re-export follows. The shader-graph `nodes::Node` keeps
  its name. The handle wraps `Object3D` and every other `Rc` handle in the
  crate is already `<Thing>Ref` after the struct it points at, so the
  scene handle now matches `NodeRef`, `SceneRef` and `CameraRef`; why
  this name and not `core::Object` is `docs/api.md` decision 2. Migration
  is a rename: `three_rs::Node` → `three_rs::ObjectRef`, `WeakNode` →
  `WeakObjectRef`. No method moved.
- **`Object3D` has three more public fields**, `listeners`,
  `on_before_render` and `on_after_render` (#159). A struct literal that
  ends in `..Default::default()` still builds; one that names every field
  does not. `Object3D`'s `Debug` is written by hand now and prints whether
  each hook is set; `Clone` copies neither the listeners nor the hooks, as
  three's `copy()` does not. `Scene` gains `on_before_render` and
  `on_after_render`.
- `LightProbeHelper` has a private field, so it can no longer be built
  with a struct literal; `LightProbeHelper::new` is the constructor, as
  before (#159).
- Which fields each material kind reads is declared once, in
  `MaterialKind::table()` (`src/materials/fields.rs`): an exhaustive `match`
  that gives each kind its fragment flow, the fields it reads, the fields it
  is loud about and its draw-time rules. `materials::setup()` picks the flow
  from it, `unsupported_fields()` is derived from it, and the renderer's
  once-per-material warning reads it under one registry, replacing the
  `if`/`else` chain and the two hand-kept rule lists that disagreed (#253).
  A no-GPU test checks every kind's program against its row. No shader
  changes, apart from the `env_map` fix under Fixed.
- **`MrtNode::set_blend_mode` takes `impl Into<BlendMode>`** rather than a
  `Blending`, and returns `&mut Self` so calls chain. `blend_mode()` returns
  a `BlendMode`. A bare `Blending` still converts. Under an MRT, a target's
  first attachment now follows `getBlendMode( texture.name )` as in three:
  the material's blending only when it is named `output`, and no blending
  for any other unset name. Members take their attachment's channel count as
  their type, so a one-channel attachment gets an `f32` output.
- A plain texture sample built outside the fragment stage emits
  `textureSampleLevel( …, 0 )`, as three's `_generateTextureSample()` does.
  It used to emit `textureSample`, which WGSL rejects in a vertex shader.
  The 3dlut page's smoke vertex shader is gated against three's dump.
- **`InstancedBufferAttribute.array` is private**, read through `array()` and
  written through `array_mut()`, which bumps the new `version()`;
  `set_needs_update()` and `id()` join them. `set_matrix_at` / `set_color_at`
  bump the version when they change a value. (#89)
- `step()` builds both operands at the wider type, as `MathNode` does, so a
  scalar edge against a vector emits `step( vec3<f32>( 0.5 ), x )`.
- `hashBlur`'s WGSL gate compares three's loop exactly. It now blurs
  `viewportSharedTexture()`, as `webgpu_backdrop_area` does.
- **`SpotLight::new` and `DirectionalLight::new` start the light at
  `DEFAULT_UP`, (0, 1, 0)**, as three's constructors do. They used to leave
  it at the origin. A light whose position the application never sets now
  shines down from one unit up, as in three; a scene that relied on the old
  origin should set the position itself.
- **A compute kernel with a barrier has no bounds check, and its vars are
  local to `main`**, as `BarrierNode.setup()` makes them in three. Before,
  `workgroup_barrier()` and `storage_barrier()` kernels kept the
  `if ( instanceIndex >= count ) { return; }` guard and module-scope
  `var<private>`s. Without the guard, every invocation of the last workgroup
  runs, as in three: the dispatch is still `ceil( count / workgroup )`, so
  tail invocations index past `count`, and a kernel must guard its own
  accesses, e.g. `If( instanceIndex < count )` around the stores after the
  barrier. See `docs/nodes.md` §84.6.
- **Nested renders save and restore the renderer's state in one place**
  (#252). `RendererUtils.saveRendererState()` / `resetRendererState()` /
  `restoreRendererState()` are one crate-private value in
  `src/renderer/renderer_state.rs`, used through a scope that restores on
  every exit, a panic included. `PassNode`, the display effects (bloom,
  SMAA, TRAA, SSR, GTAO, outline, stereo and the rest), SSAA, PMREM, the
  cube captures and the reflector all use it, where each kept its own
  hand-rolled subset before. Sites calling `resetRendererState()` now also
  clear the render-object function and restore everything three's list
  holds, as three does; no ladder frame changes. `Renderer::render`,
  `render_shared` and `render_nested` share one body.

### Fixed

- A transmissive material drawn `BackSide` reads its own viewport texture,
  copied before the first back-side draw, as three's `viewportBackSideTexture`
  is. The front side's `viewportOpaqueMipTexture()` is now copied at the first
  front-side draw, so the front face of double-sided glass refracts its own
  back faces. The port used to copy once, before either half, and bind that
  copy to both (`docs/nodes.md` §96.3).
- The transmission copies magnify `Nearest`, like three's
  `FramebufferTexture`. `textureBicubicLevel` reads level 0 at an explicit
  level wherever the LOD is below 1, so the filter there is the mag filter
  (`docs/nodes.md` §96.4).
- `ssaaPass` renders its samples with the pass's own MRT and puts the
  frame's MRT back afterwards (#252), as `SSAAPassNode.updateBefore()` does.
  It used to render under whatever MRT the caller had set.
- A map's filters and wrap modes changed after the first draw rebuild the
  program (#276). Three's WebGPU backend folds every texture property's
  `magFilter`, `minFilter`, `wrapS` and `wrapT` into the material cache key;
  the port keyed programs on the material's id and version only, so switching
  a map to `Nearest` (unfilterable: `textureLoad`, no sampler) or changing an
  unfilterable map's `wrap_s` (its `tsl_coord_*` wrap function) kept the stale
  program. The sampler state is now read off the material's textures into the
  dynamic half of the key at every draw. `tests/texture_sampler_rebuild.rs`
  covers it.
- Phong, Lambert and Toon materials zero each lighting accumulator
  (`irradiance`, `directDiffuse`, `directSpecular`, `indirectDiffuse`, and
  Lambert's specular pair) once, right above its first use, as three's dumps
  do (#281). Each accumulator is a var with a zero initialiser, and the flow
  also assigned the same zero explicitly, so the WGSL had every zero twice in
  a row. The physical flow keeps its hoisted zeros but emits each once,
  `clearcoat*`, `radiance` and `iblIrradiance` included.
  `sss_shadow_context_matches_three` no longer drops repeated lines; it
  checks the zero count and position against the dump, and
  `lit_accumulators_are_zeroed_once` covers the other lit flows.
- A depth-stencil texture is sampled through a depth-only view. With
  `RendererParameters::stencil` on, `viewportDepthTexture()`'s copy of the
  `depth24plus-stencil8` canvas depth failed bind-group validation, because
  the view had both aspects. `tests/renderer_viewport_depth.rs` covers it.
- The glTF loader applies `occlusionTexture.strength` as `ao_map_intensity`,
  as three's `GLTFLoader` does. It was ignored, so `pool.glb`'s
  `SPWallsFloorStairs`, which sets it to 0, was darkened by its AO map.
- `positionViewDirection` is `vec3( 0, 0, 1 )` under an orthographic
  camera, as three's is. It was always the perspective
  `normalize( -positionView )`, which moved Phong and physical specular
  highlights under an `OrthographicCamera`.
- `smaa()` renders its input inside the reset renderer state, as three's
  lazily updated input does. A scene pass behind it now clears to opaque
  black instead of the renderer's clear alpha, so an anti-aliased line over
  an empty background is no longer brightened back by `renderOutput`'s
  unpremultiply.
- glTF `alphaMode: MASK` now sets `alpha_test = alphaCutoff`. Before, masked
  cut-outs drew as solid quads.
- A `negate()` read more than once becomes a shared `var`, as three's
  `MathNode` does. It used to be inlined at every read.
- A `Fn()` block read twice counts its result twice, so the result is
  promoted to a `var` where three promotes it.
- `GaussianBlurNode::render()` runs its input's update-before first. On the
  first frame, a blur over an `rtt()` or another display node used to size
  its targets from a 1×1 input.
- A `HemisphereLight` with no `AmbientLight` beside it no longer has its
  irradiance overwritten with zero before the Phong, Lambert and Toon models
  read it.
- An `InstancedMesh`'s `instanceMatrix` and `instanceColor` are no longer
  written to the GPU on every draw: each attribute keeps one buffer and is
  re-written only when its version moves, and each write counts in
  `info.build.buffers_written`. The render list no longer copies the arrays
  each frame either. A still 131072-instance mesh goes from about 25 ms a
  frame to about 3 ms (`benches/instanced_mesh.rs`). (#89)
- A lit `MeshBasicNodeMaterial` zeroes `indirectDiffuse` before it adds to
  it, as `BasicLightingModel` does.
- **Skinned shadow casters** are skinned in directional and spot shadow maps.
  Point-light shadows of skinned meshes are still unskinned. (#163)
- A texture sampled outside the fragment stage, such as in a `positionNode`,
  emits `textureSampleLevel( …, 0 )`, as three does. It used to emit
  `textureSample`, which WGSL rejects in a vertex shader.
- A background that is an inline `Fn()` call is built inside the skybox
  material, as three builds every `Fn` body. So `normalWorld` in it is the
  back-side normal.
- A Sprite, Points or Line2 material with `env_map` set no longer samples
  it (#253). Only `MeshBasicNodeMaterial.setupEnvironment()` wraps the env
  map in a `BasicEnvironmentNode` in three; the port's unlit flow sampled it
  for every kind that reached it, while `unsupported_fields()` warned that
  those kinds ignore it. The warning stays; the sample is gone.

## [0.2.0] - 2026-09-29

Ships with `sdf-text` 0.2.0 and the first release of `three-rs-controls`
(0.1.0); their sections follow this one.

### Changed

Breaking changes, settled in [`docs/api.md`](docs/api.md) after 0.1.0's first
outside consumers and batched together for the 0.2.0 release:

- **`Node` is a newtype**, not a `pub type Node = Rc<RefCell<Object3D>>` alias,
  with the tree operations (`add`, `children`, `matrix_world`, ...) inherent
  instead of living on a separately-imported `Object3DNode` trait. Call sites
  are unchanged; `use three_rs::core::Object3DNode` is no longer needed.
  `WeakNode` gets the same treatment and `Rc::ptr_eq` becomes `Node::ptr_eq`.
  In the same change, **`Mesh::new` takes the material**:
  `Mesh::new(geometry, material)`, matching `new Mesh(geometry, material)`,
  instead of `Mesh::new(geometry)` followed by setting `.material` by hand.
  (#72)
- **Fallible entry points return `Result`.** Loaders, `Renderer::new`,
  `Renderer::with_instance`, `Renderer::read_canvas_pixels`,
  `RenderTarget::new_with_options`, `DepthTexture::set_type`,
  `AnimationClip`/`KeyframeTrack` parsing and `VectorFont::parse` return
  `three_rs::Error` (or `sdf_text::Error`) instead of panicking. Both error
  enums are `#[non_exhaustive]`. Constructors, geometry builders and the rest
  of the scene-graph API stay infallible. (#74)
- **`Renderer::with_device(parameters, adapter, device, queue)`** lets a host
  that already owns a `wgpu::Device` and `Queue` hand them to the renderer
  instead of the renderer creating its own; `Renderer::new` and
  `with_instance` now end in it. `Texture::external(gpu, color_space)` wraps
  a `wgpu::Texture` the renderer never re-uploads or destroys, and
  `Texture::set_data` / `set_needs_update` / `version` let a texture whose
  pixels change every frame (a screencast, a shared-memory client buffer)
  keep its GPU allocation instead of being recreated. (#64)
- **Passes render themselves.** `PassNode`, `RttNode` and `BloomNode` are now
  `Rc` handles with `&self` setters; give one its scene and camera with
  `pass(scene, camera)` or `set_scene`, and the renderer runs it from its
  `updateBefore()` hook the first time a frame samples the pass's texture,
  the way three.js does. Their explicit `render()` methods are deprecated
  (see *Removed*). (#206)
- **A scene's or material's environment is an `Environment` enum** —
  `Environment::Pmrem(PmremHandle)` or `Environment::Node(EnvironmentNode)` —
  instead of a raw PMREM handle, so an environment can be a generated cube or
  a graph of `pmremTexture()` reads. (#208)
- **Options structs are `#[non_exhaustive]`**, so a field added later is not
  a break: `PassOptions`, `RenderTargetOptions`, `RendererParameters`,
  `GaussianBlurOptions`, `BoxBlurOptions`, `HashBlurOptions`,
  `RadialBlurOptions`, `Billboarding`, `ExtrudeGeometryOptions`,
  `TextGeometryOptions`, `ReflectorParameters`, `OverrideNodes`,
  `Ktx2Support`, `RaycasterParams`, and sdf-text's `LayoutParams` and
  `BatchedTextOptions`. Replace `RendererParameters { antialias: true }` (and
  `S { a, ..S::default() }`) with `let mut p = RendererParameters::default();
  p.antialias = true;`. `ComputeFlow` literals become `ComputeFlow::new(
  statements, count)`, with `workgroup_size`, `name` and `on_init` set as
  fields afterwards; `MaterialFlow` literals become `MaterialFlow::new(output,
  position)`. `Info`, `RenderCounts`, `BuildCounts`, `MemoryCounts`,
  `ComputeCounts` and sdf-text's `TextRenderInfo` are `#[non_exhaustive]` too:
  read their fields, and destructure them with `..`. (`docs/api.md` decision
  3; #217)
- **One `ColorSpace`.** `math::ColorSpace` and `textures::ColorSpace` are
  merged into `three_rs::math::ColorSpace` (also `three_rs::ColorSpace`),
  with the variants `NoColorSpace`, `Srgb` and `LinearSrgb`. Replace
  `textures::ColorSpace` with `math::ColorSpace`. `ColorManagement::convert`,
  `working_to_color_space`, `color_space_to_working` and `get_transfer` take a
  `ColorSpace` instead of an `Option<ColorSpace>`: write
  `ColorSpace::NoColorSpace` where you passed `None`, and drop the `Some(..)`.
  The HDR, cube HDR, Ultra HDR and KTX2 loaders now tag linear textures
  `LinearSrgb` rather than `NoColorSpace`, as three.js does. Both sample the
  same way. (#216)
- **Acronyms in public names use Rust casing** (API Guidelines C-CASE), with
  the three.js name in the doc comment:
  `ColorSpace::SRGB` becomes `Srgb`, `ColorSpace::LinearSRGB` becomes `LinearSrgb`,
  `GLTFLoader` becomes `GltfLoader`, and
  `CoordinateSystem::WebGL`/`WebGPU` become `WebGl`/`WebGpu`.
  Dimension suffixes (`Data3DTexture`), axis orders (`EulerOrder::XYZ`) and
  glam-style vector types (`Type::UVec2`) are unchanged; see
  [`docs/api.md`](docs/api.md) decision 3. (#216)
- **The public surface was audited** (`docs/api.md` decision 11, #213).
  About 290 items that were public only for the crate's own tests and
  examples are now `pub(crate)`, 79 more are `#[doc(hidden)]` (the grader
  readbacks, the `draco`/`meshopt` loader modules), and about 25 dead items
  are gone, among them `Renderer::program_builds` and `ColorManagement`'s
  deprecated helpers. Items re-exported one level up lost their long paths:
  import them from the parent module. `CurveVector` is sealed. 18 enums are
  `#[non_exhaustive]`, `nodes::Node` among them, so match them with a `_`
  arm.
- **`RenderCamera` is sealed**, matching `CurveVector`: it cannot be
  implemented for a type outside this crate. Nothing that legitimately used
  it as a trait object or bound needs to change. (#218)
- **`NodeRef`, `RootId` and `ActionHandle` are opaque handles**: their tuple
  field is private. `NodeRef::node()` (or `as_rc()` for the `Rc<Node>`
  itself) replaces reading `.0`; `RootId::MIXER_ROOT` replaces
  `RootId(0)`. `ActionHandle` and `RootId` are otherwise unchanged — get one
  from the `AnimationMixer` that owns it. (#218)

Other changes:

- The `rust-version` in `Cargo.toml` is documented as 1.90, which is what the
  lockfile already needed. (#73)

### Added

- **glTF loading.** `GltfLoader` for `.gltf`/`.glb` (#119), with
  `KHR_materials_sheen` (#122), `KHR_materials_anisotropy` (#124),
  `KHR_draco_mesh_compression` via `draco-core` (#150), `KHR_texture_basisu`
  / KTX2 textures (#176), `EXT_meshopt_compression` (#182),
  `EXT_texture_webp` (#183, AVIF evaluated and left out), and
  `KHR_materials_diffuse_roughness` with the EON diffuse lobe (#204). A required
  extension the loader cannot read is now a load error rather than a
  silently wrong scene (#125), and every glTF texture reference is a
  `GltfTextureRef` carrying its own sampler and wrap state (#127).
- **Post-processing.** SSAA (#100), selective bloom and `uniformArray()`
  (#108), a previous-frame texture and pass `getOutput` hook (#109),
  anamorphic flare (`rtt()`, node-valued loop bounds, mirrored-repeat wrap)
  (#111), chromatic aberration (#118), a node library covering GaussianBlur,
  Sobel, DotScreen, RGBShift, Transition, AfterImage, Pixelation, hashBlur
  and boxBlur (#148), FXAA (#197) and halftone (#209).
- **PMREM environment lighting.** HDR and half-float texture loading and
  readback (#102); `PMREMGenerator` ports `from_cubemap` (#105),
  `from_equirectangular` (#107, plus `UltraHDRLoader`, #117), `from_scene`
  (#110, #112) and the furnace-test path (#110), each gated against three's
  own numbers; `RoomEnvironment` (#118); an LDR cubemap upload path for
  `webgpu_materials_envmaps` and `_cubemap_mipmaps` (#114).
- **Compute.** Compute kernels and storage buffers (#96); indirect
  draw/dispatch, atomics and workgroup memory (#178); `Data3DTexture` and
  storage textures (#175).
- **Rendering.** Multiple render targets and MRT readback (#106, #207);
  `Renderer::copy_texture_to_texture` and in-place partial texture updates
  (#202); Basic and VSM shadow filters with `filterNode`/`shadowNode` hooks
  (#177); occlusion queries and `isOccluded` (#190); `ArrayCamera` and
  user-defined `LightingModel`s (#191); `reflector()`, a planar mirror node
  (#208); layer-based render filtering (#203); `MeshToonNodeMaterial` and a
  toon outline pass (#193); `BatchedMesh` drawn as one `drawIndexed` per
  range (#95); fat lines (`Line2NodeMaterial` and the `lines` addon) with
  viewport/scissor state and `CatmullRomCurve3` (#101, #104); `Raycaster`,
  `Sprite` and instanced sprites (#149, #192); skinned and instanced points
  (#199); an `alphaHash` blend mode and multi-material geometry groups
  (#201); `textureGrad`/`textureGather` sampling (#188); per-object uniform
  buffers and `wgslFn` varyings (#116); `wgslFn` and a MaterialX node
  function library for TSL (#98, #151); the direct clearcoat lighting lobe,
  with every other unsupported material field now warning once (or, via
  `check_supported()`/`unsupported_fields()`, failing before the first
  frame) instead of being silently ignored (#181).
- **Nodes.** `Node::Custom(Rc<dyn CustomNode>)` for user node types that
  compose existing variants, `tsl::custom()`, and `tsl::context()` /
  `tsl::isolate()` (`.context()` / `.isolate()`) for three's `ContextNode`
  and `IsolateNode`, sharing one parent-chained cache during a build (#185,
  #189, #194).
- **Geometry, math and cameras.** `Shape`, `Path` and the curve family
  (`CurvePath`, Béziers, ellipses/arcs, splines) with `ShapeUtils`/Earcut
  triangulation (#180); `Fog` and `FogExp2` behind `scene.fog` (#145);
  `Box3::set_from_object` and a camera-fit helper (#79); `project`/
  `unproject` on any camera, and `crossed` (#77); `OrthographicCamera::
  look_at`, matching the perspective camera (#83); `RenderCamera::
  set_view_offset` and render-pipeline before/after hooks (#186); mutable
  geometry attributes and render-target readback (#81); a cached geometry
  bounding box and sphere (#136).
- **Controls.** `OrbitControls`, in the native viewer and the browser
  (#130); map-style controls over a curved ground (#86).
- **Browser.** The graded examples run in the browser on wasm32 + WebGPU,
  animated on a clock (#129, #130), with a live gallery published to GitHub
  Pages (#131).
- **Diagnostics.** `Renderer::info()` reports per-frame draw-call and
  triangle counts (#76).
- **Documentation.** Every public item in `three-rs`, `sdf-text` and
  `three-rs-controls` has a doc comment naming the three.js (or lib3) item it
  ports, each crate has a crate-level overview with an example, and
  `#![warn(missing_docs)]` keeps it that way (#215, #219–#225). A
  `CHANGELOG.md`, a code of conduct and issue templates were added (#212).
- 69 more three.js examples graded green since 0.1.2, for 79 in total — see
  the README's gallery table for the full list.

### Fixed

- `texture()` reads an unfilterable map with `textureLoad` instead of a
  filtering sample. (#121)
- An instanced attribute uploads once per array instead of once per draw.
  (#90)
- The renderer keeps uniform buffers, views, samplers and bind groups across
  frames instead of rebuilding them every frame. (#138)
- The texture and view caches are swept by liveness, closing a leak on
  long-running scenes. (#174)
- `map` is read on every material path, and `vertexColors` is honoured.
  (#78)
- Non-skinned glTF materials read a `vec4` `COLOR_0`, matching skinned ones.
  (#113)

### Removed

- **`PassNode::render`, `RttNode::render` and `BloomNode::render()`**,
  deprecated since the passes-render-themselves change above, are removed.
  Give a pass its scene and camera and the renderer runs it automatically.
- `d3-hierarchy` moved out of this workspace into its own repository; its
  examples and gates no longer live here. The SDF examples and gates moved
  into `sdf-text` in the same change. (#66)

## sdf-text 0.2.0 - 2026-09-29

Breaking, because of the `Result` and `Option` signatures below; it depends on
`three-rs` 0.2.

### Changed

- `sdf_text::Error`, a `#[non_exhaustive]` crate-level error enum, replaces
  the remaining panics on fallible paths, mirroring `three-rs::Error`.
- **`BatchedText::add_text` returns `Option<usize>`** (`None` at capacity)
  and **`Text::member_id` returns `Option<usize>`** (`None` when the member
  has not joined a batch), replacing the `-1`-sentinel `i64` both used to
  return. (#218)

### Added

- `VectorFont::measure()`, for laying out text before there is an object to
  measure. (#75)
- A refilled `BatchedText`'s newly-added glyphs draw without waiting for a
  full rebuild. (#85)
- Batch members honour their own full `matrix_world`, and a batch's bounds
  come from its members rather than its node. (#75)
- Roboto ships inside the package so the example runs standalone. (#75)

## three-rs-controls 0.1.0 - 2026-09-29

### Added

- First release: `MapControls`, a map camera over a ground with curvature,
  in the spirit of three.js' `MapControls` and damped the way camera-controls
  damps, for `three-rs` 0.2. (#86, #92)

## [0.1.2] - 2026-09-13

### Fixed

- Geometry, attribute and texture caches are keyed by never-reused ids
  instead of `Rc` address, closing a freed-address ABA bug a two-plane
  dendrogram scene hit. (#58)

_Ships alongside the 0.1.1 fix, both already on `main` before this tag._

## [0.1.1] - 2026-09-13

### Fixed

- Program cache keys are bindings hashed by identity and shape instead of
  their `Debug` string, so a frame with a large texture no longer formats
  its pixels into a cache key. (#56)

## [0.1.0] - 2026-09-13

### Added

- First crates.io release: three.js core and math (`Object3D` scene graph,
  `BufferGeometry`, cameras, `Vector`/`Matrix`/`Quaternion`/`Euler`/`Color`),
  the `WebGPURenderer` port on wgpu with a TSL-generated node system, the
  core materials, lights and shadow maps, and the first graded examples.

[Unreleased]: https://github.com/oneilltomhq/three-rs/compare/v0.2.0...HEAD
[0.2.0]: https://github.com/oneilltomhq/three-rs/compare/v0.1.2...v0.2.0
[0.1.2]: https://github.com/oneilltomhq/three-rs/compare/v0.1.1...v0.1.2
[0.1.1]: https://github.com/oneilltomhq/three-rs/compare/v0.1.0...v0.1.1
[0.1.0]: https://github.com/oneilltomhq/three-rs/releases/tag/v0.1.0
