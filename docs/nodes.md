# The node system (rungs 4–5)

Rungs 1–3 shipped hand-written WGSL whose headers documented, statement by
statement, the three.js node flow each file stood in for. Rung 4 replaces those
headers with code: the WGSL is now generated. This note says how, and is the
contract later rungs extend.

Ground truth for the shape of the output is a dump of what Chrome's
`GPUDevice.createShaderModule` actually receives for `webgpu_depth_texture`,
`webgpu_instance_mesh`, `webgpu_materials_basic`, `webgpu_rtt` and
`webgpu_lights_phong` (a temporary
puppeteer hook; dumps under `target/dumps/`, nothing committed). Every generated
shader is read against its dump.

## 1. The graph in Rust

```rust
pub struct NodeRef(Rc<Node>);          // src/nodes/node.rs
pub enum Node { Const(..), Uniform{..}, Attribute{..}, Varying{..},
                Property{..}, Var{..}, Assign{..}, Operator{..}, Math{..},
                Swizzle{..}, Join{..}, Convert{..}, Texture{..}, Buffer{..},
                ArrayElement{..}, Builtin{..}, Stack{..}, If{..}, Code{..}, … }
```

**`Rc<Node>`, immutable, enum.** Three's nodes are mutable JS objects: `setup()`
returns a replacement node, `TempNode` caches its generated snippet on itself,
`getCacheKey()` walks and memoises, `NodeBuilder.getUniformFromNode` /
`getVarFromNode` / `getVaryingFromNode` stash per-node state in
`nodeData[ node.uuid ][ shaderStage ]`. Three trade-offs, decided as follows.

*Identity without an arena.* All of that per-node state is keyed on node
identity. `Rc::as_ptr` is a stable `usize` key, so the builder keeps
`HashMap<(usize, Stage), NodeData>` and nodes need no mutable fields at all. An
arena with `NodeId`s would give the same keys but forces every TSL function to
take the arena (or a thread-local), which destroys the reading order of ported
example code. `Rc<RefCell<dyn Node>>` would model Three literally but re-enters
`borrow_mut()` during a recursive `generate()` — a guaranteed runtime panic
class, and exactly the silent-failure shape this project is told to avoid.

*Module singletons.* `positionLocal`, `normalWorld`, `cameraViewMatrix` are
module-level singletons in TSL, and that sharing is load-bearing: two uses of
`cameraViewMatrix` must resolve to *one* uniform. The Rust accessors are
`thread_local!` lazily-initialised `NodeRef`s cloned on each call, so
`camera_view_matrix()` returns the same `Rc` every time, and pointer identity
does the deduplication that Three gets from object identity.

*`positionLocal` is a varying.* In r186 `Position.js` reads
`positionLocal = positionGeometry.toVarying( 'positionLocal' )`, not
`.toVar()`. It matters as soon as a fragment-stage node is built on it — rung
7's `maskNode` is — because the attribute itself may only be read in the vertex
stage. The port's `Node::Varying` arm already degrades a varying that no
fragment-stage node requests into a plain `var<private>`, so making the
accessor faithful changed no earlier rung's WGSL by a byte.

*`setup()` returning a replacement.* Three's `Node.build()` calls `setup()`,
caches the returned node, then generates from it. Here `setup` is a method on
the builder (`NodeBuilder::setup(&NodeRef) -> NodeRef`) with the result cached
in the same node-data map, so the node itself stays immutable. A node whose
whole job is to expand into a subgraph (`RangeNode`, `InstanceNode`,
`BasicEnvironmentNode`, `RenderOutputNode`, `CubeMapNode`) is a variant whose
`setup` builds the subgraph; a node that emits a snippet directly
(`OperatorNode`, `MathNode`, `SwizzleNode`) has no `setup`.

*`TempNode` caching.* A `Math`/`Operator`/`Texture` node reached more than once
in a stage is emitted once into a `nodeVarN` private var and referenced
thereafter — the same `node_var` mechanism as `VarNode`, decided by the same
reference count Three uses (`builder.getDataFromNode(...).usageCount`). The
reference count is gathered in an *analyse* pass before the generate pass, which
is how Three gets `nodeVar0 = textureSample(...)` ahead of three uses of it in
`saturation()` (see `webgpu_rtt`'s FX fragment).

*`getCacheKey()`.* A recursive hash over the variant discriminant plus child
keys plus the *identity* of uniform/texture/attribute slots (not their values) —
so two materials with the same graph shape share a program, and changing a
uniform's value never rebuilds. Memoised per `Rc::as_ptr` in the program cache.

*Stacks and `Fn()`.* `builder.addStack()` / `removeStack()` become a
`Vec<Vec<NodeRef>>` of statement lists on the builder; `Node::Stack` holds the
finished list. `Fn()` is `Node::Call { body: Rc<dyn Fn(&[NodeRef]) -> NodeRef>,
layout: Option<FnLayout> }`. With `layout: None` the body is *inlined* into the
current stack, which is what Three does and what the dumps show: `saturation()`
appears three times expanded inside one expression in `webgpu_rtt`'s FX
fragment, not as a `fn` call. With a layout it emits a real
`fn tsl_name(...) -> T` into the `// codes` section — that is how
`sRGBTransferOETF` and the premultiply helpers appear in the output pass, and
it is the hook rung 7's custom `Fn()` lands on.

Why an enum rather than `dyn Node`: the node set is closed (this is a port of a
fixed library, not a plugin surface), and an exhaustive `match` in
`node_type()`/`generate()` means a new variant for rung 5 cannot be silently
forgotten in one of the passes. The escape hatches for anything genuinely open
are `Node::Call` (TSL `Fn()`) and `Node::Code` (raw WGSL, e.g.
`tsl_inverse_mat3`).

## 2. TSL in Rust

`NodeRef` carries inherent methods, so ported example code reads like the JS:

```rust
material_fx.color_node = Some(hue(
    saturation(scene_pass_texture.rgb(), screen_fx.x().one_minus()),
    screen_fx.y(),
));
```

versus

```js
materialFX.colorNode = hue( saturation( scenePassTexture.rgb, screenFXNode.x.oneMinus() ), screenFXNode.y );
```

Rules:

* Properties in JS (`.rgb`, `.x`, `.xyz`, `.a`) are methods in Rust (`.rgb()`,
  `.x()`, `.xyz()`, `.a()`) — Rust has no property syntax, and making them
  methods keeps swizzles and operations in one namespace.
* Methods that JS spells on the node (`.mul`, `.add`, `.sub`, `.div`, `.max`,
  `.mix`, `.normalize`, `.cross`, `.dot`, `.cos`, `.sin`, `.one_minus`,
  `.negate`, `.element`, `.assign`, `.to_var`, `.append`) are inherent methods.
  Every argument is `impl Into<NodeRef>`, with `From<f64>`, `From<f32>`,
  `From<i32>`, `From<Color>`, `From<Vector2/3>` and `From<Matrix4>`, so
  `.mul(0.1)` and `.mul(some_node)` both work, exactly as in TSL.
* Functions that JS spells free (`mix`, `vec3`, `vec4`, `float`, `dot`, `max`,
  `texture`, `cube_texture`, `uniform`, `attribute`, `range`, `osc_sine`, `hue`,
  `saturation`, `luminance`) are free functions in `three_rs::tsl`.
* Accessor singletons are zero-argument functions: `position_local()`,
  `normal_local()`, `normal_world()`, `normal_view_geometry()`,
  `model_view_matrix()`, `model_world_matrix()`, `model_normal_matrix()`,
  `camera_projection_matrix()`, `camera_view_matrix()`,
  `camera_world_matrix()`, `uv()`, `vertex_index()`, `instance_index()`,
  `screen_coordinate()`, `time()`. Parens are the only divergence from TSL's
  bare names; the alternative (`static` `Lazy<NodeRef>`) cannot be a
  `thread_local` and `Rc` is not `Send`.
* Names track TSL even where Rust convention would differ
  (`position_local` for `positionLocal`, `osc_sine` for `oscSine`), so a reader
  can diff a ported material against the JS line by line.

## 3. NodeBuilder / WGSLNodeBuilder

**`nodes::NodeBuilder`** (`src/nodes/builder.rs`) is everything that is not
WGSL syntax:

* the build context: material, geometry, object flags (`is_instanced_mesh`,
  `instance_count`, `is_quad_mesh`), render-target-or-canvas, camera kind;
* the two stages, built **fragment first, then vertex** — Three's
  `defaultShaderStages = [ 'fragment', 'vertex' ]`, which is why the dumped
  uniform indices run `nodeUniform0/1/2` in the fragment and jump to
  `nodeUniform5` in the vertex;
* `setup_position` / `setup_vertex` / `setup_diffuse_color` /
  `setup_variants` / `setup_lighting` / `setup_outgoing_light` /
  `setup_environment` / `setup_output`, i.e. `NodeMaterial`'s flow, as methods
  on the material port so a subclass overrides one of them;
* slot allocation: `get_uniform_from_node`, `get_var_from_node`,
  `get_varying_from_node`, `get_attribute`, `get_buffer_from_node`,
  `get_code_from_node`, each idempotent per `(node, stage)`;
* the flow: a `Vec<String>` of statement lines per stage, plus the var and
  varying declaration lists, plus `result` (the vertex clip-space snippet / the
  fragment output snippet);
* attribute → `@location(n)` assignment, in first-use order across stages
  (fragment first), which is why `webgpu_rtt`'s box gets `uv` at location 0 and
  `position` at location 1;
* uniform group layout and byte offsets (§4);
* the cache key.

**`nodes::wgsl`** (`src/nodes/wgsl.rs`) is the WGSL specialisation:

* `Type` → `f32` / `vec3<f32>` / `mat3x3<f32>` / `texture_2d<f32>` /
  `texture_depth_2d` / `texture_cube<f32>` and the constant formatter
  (`1.0`, `vec3<f32>( 0.57735, 0.57735, 0.57735 )`);
* method-name mapping (`one_minus` → `( 1.0 - x )`, `inverse_sqrt` →
  `inverseSqrt`, `mod` → `%`, comparison-to-`vec3<bool>` casts);
* the sampling snippets: `textureSample`, `textureSampleLevel`, and the
  `textureLoad` + `textureDimensions` + `tsl_coord_clampS_clampT_2d` form a
  *non-filterable* texture (a depth texture with nearest filters) forces —
  `webgpu_depth_texture`'s quad uses that path and has **no sampler binding**;
* the fixed skeleton (`// uniforms`, `// varyings`, `// vars`, `// codes`,
  `@vertex fn main(...) -> VaryingsStruct`,
  `@fragment fn main(...) -> OutputStruct`, `var<private> varyings`,
  `diagnostic( off, derivative_uniformity )`);
* bind-group-layout emission: `wgpu::BindGroupLayoutEntry` per binding with the
  visibility mask the dump shows (`7` = vertex|fragment|compute for uniform
  buffers, `2` = fragment for textures and samplers).

The split is a module boundary rather than a trait, because there is exactly one
backend and a trait with one implementor is noise; the boundary is drawn where a
trait would go, so introducing one later is mechanical.

## 4. How the renderer consumes it

```
(material, geometry, object flags, render-target-ness)
      │  NodeBuilder::build()
      ▼
NodeProgram {
    vertex_wgsl, fragment_wgsl,
    attributes:  Vec<AttributeSlot>   // name, Type, location, array_stride
    bind_groups: Vec<BindGroupDesc>   // group index → Vec<Binding>
    cache_key:   String
}
```

* `Binding` is `Uniforms { members: Vec<(UniformSource, Type, byte_offset)>,
  size, update: UpdateType }`, `TextureBinding(TextureSource)`,
  `SamplerBinding(SamplerDesc)` or `BufferBinding(BufferSource)`.
* `UniformSource` names *where the value comes from*, not a value:
  `CameraProjectionMatrix`, `CameraViewMatrix`, `CameraWorldMatrix`,
  `ModelWorldMatrix`, `ModelNormalMatrix`, `MaterialColor`, `MaterialOpacity`,
  `MaterialReflectivity`, `MaterialEnvRotation`, `TextureMatrix(id)`,
  `BackgroundRotation`, `BackgroundBlurriness`, `BackgroundIntensity`, `Time`,
  `ViewportSize`, `Literal(Vec<f32>)` (that last is `uniform( mouse )`). Writing
  a uniform block is a `match` over it, so the shader text and the bytes are
  produced by the same pass and cannot disagree — the failure mode that a
  hand-maintained `write_object_slot` has.
* Groups follow the dumps: the **render** group (camera, time, viewport,
  background params) is group 0 when it is used at all, the **object/node**
  group (material uniforms, textures, samplers, node buffers) is the next index
  — so a material that touches no camera uniform, like
  `webgpu_depth_texture`'s and `webgpu_rtt`'s quads, puts its object group at
  group 0. Bindings inside a group are numbered in creation order across both
  stages, fragment first.
* Uniform struct layout is WGSL's: `f32` align 4, `vec2` 8, `vec3`/`vec4`/
  `mat3x3`/`mat4x4` align 16 (`mat3x3<f32>` occupying 48 bytes), struct size
  rounded up to 16. Offsets are computed while the member list is built.
* `NodeProgram`s are cached on `cache_key`; pipelines on
  `(cache_key, color_format, depth_format, sample_count, front_face,
  depth_write, depth_compare)`.
* Per frame, `Renderer::render()` walks the bindings and writes the ones whose
  `UpdateType` is due — `Render` once per `render()` call (camera, viewport),
  `Frame` once per frame (`time`), `Object` per draw (world matrix, normal
  matrix, material colour). That is `NodeFrame.update` plus
  `NodeUniformsGroup.updateType` in Three. Per-object blocks keep rung 1's
  single buffer with 256-byte dynamic offsets.

## 5. What rungs 1–3's hand-written WGSL is replaced by

| file | replaced by |
|---|---|
| `basic.wgsl` | `MeshBasicNodeMaterial` with no `colorNode`: `setup_diffuse_color` → `materialColor`, `setup_outgoing_light` → `diffuseColor.rgb`, `setup_output` → `max( vec4( .., a ), 0 )` |
| `quad.wgsl` | the same material with `colorNode = texture( depthTexture )` on a `QuadMesh` (vertexNode = the `array().element(vertexIndex)` triangle), via the `textureLoad` path |
| `normal_world_range_mix.wgsl` | `colorNode = mix( normal_world(), range(min,max), osc_sine( time().mul(0.1) ) )` plus `instanced_mesh()` inside `setup_position` |
| `basic_envmap.wgsl` | `setup_environment` → `BasicEnvironmentNode( cube_texture( envMap ) )` → `CubeMapNode`, plus `BasicLightingModel`'s `indirect`/`finish` |
| `background_cube.wgsl` | `Background::update`'s `NodeMaterial`: `vertexNode` = the skybox viewProj with `setZ(w)`, `colorNode` = `vec4(cubeMap).mul(backgroundIntensity)` under a context overriding `getUV`/`getTextureLevel` |
| `output_color_transform.wgsl` | `NodeMaterial` with `fragment_node = render_output( texture( fbTarget ) )` → `ColorSpaceNode`'s sRGB OETF between unpremultiply/premultiply, on a `QuadMesh` drawn with the ortho camera through the ordinary vertex flow |
| `mipmap.wgsl` | **stays.** Three's mipmap generator is raw WGSL too (`WebGPUTexturePassUtils`), not node-generated. The dumps confirm it: the module is named `mipmap` and the pipeline `mipmap-rgba8unorm-2d-array`, built from a string template, not from a node graph. |

Each file is deleted in the commit that replaces it, and rungs 1–3 are re-run
after each deletion: a regression there is the signal that the builder, not the
new material, is wrong.

## 6. Explicitly deferred

| deferred | where it plugs in |
|---|---|
| Phong / Standard / Physical lighting models (rungs 5, 8) | `setup_lighting_model` returns a `LightingModel` with `direct`/`indirect`/`ambient_occlusion`/`finish` hooks; rung 4 ships only `BasicLightingModel`. `LightsNode` (the per-light loop) is the one new builder concept. |
| lights themselves | a `LightsNode` variant plus a render-group uniform array; the render group already exists. |
| shadow maps (rung 7) | a `ShadowNode` inside the lighting model, plus a depth-only render pass the renderer already has the machinery for (`RenderTarget` + depth texture). |
| morph targets (rung 6), skinning (rung 10), batching (rung 11) | `setup_position`, which already has the `instanced_mesh()` hook in exactly the place Three calls `morphReference()` / `skinning()` / `batch()`. |
| tone mapping (rung 7) | `RenderOutputNode` already branches on tone mapping; rung 4 passes `NoToneMapping`. |
| post-processing `pass()` (rung 9, done — see `docs/postprocessing.md`) | `PassNode` is a `Texture` whose source is a `RenderTarget` the renderer renders first; `TextureSource` already has that variant shape. |
| compute (rung 12) | `Stage::Compute` is in the stage enum and unreachable; it needs storage buffers (`BufferSource` with `var<storage>`) and a `@compute` entry point. |
| `SpriteNodeMaterial` (rung 13) | `setup_position_view`, which `NodeMaterial` already routes through `builder.context`. |
| MRT, clipping planes, vertex colours, fog, alpha test | all are single branches in `NodeMaterial`'s setup flow, omitted because no rung 1–4 material sets them. |

## 7. Sub-builds, and `normalMap` as the value of `normalView`

Rung 5's centre teapot sets `material.normalNode = normalMap( texture( … ) )`.
`normalMap()` is `TBNViewMatrix * ( texel * 2 - 1 )`, and `TBNViewMatrix` is
built from `normalView` — so `normalView` has to mean two different things in
one shader: *the geometric normal* inside the normal map's own expression, and
*the mapped normal* everywhere downstream (the lighting model). A single
memoised accessor cannot do that.

Three's answer is the **sub-build**. `NodeMaterial.setupNormal()` does not
assign anything; it installs a thunk on the builder:

```js
// NodeMaterial.js:472
builder.context.setupNormal = () => subBuild( this.setupNormal( builder ), 'NORMAL', 'vec3' );
```

`subBuild( node, name )` pushes `name` onto `builder.subBuildLayers` for the
duration of that node's build. Two things follow from being inside a layer:

1. `normalView` resolves **geometrically** (the layer is what tells the
   accessor not to call back into `context.setupNormal`), so the recursion
   terminates.
2. Every var the layer is *tagged on* gets its layer as a name prefix, which is
   where the dump's `NORMAL_normalView`, `NORMAL_tangentView`,
   `NORMAL_bitangentView` and `NORMAL_TBNViewMatrix` come from.

### What this port does

`src/nodes/tsl.rs` holds the layer and the context in two thread-locals,
because our TSL functions are free functions rather than methods on a builder
that is threaded through every call:

* `SUB_BUILD: Option<&'static str>` — the single active layer.
  `in_sub_build( "NORMAL", || … )` is `subBuild()`; `sub_build_name( "x" )` is
  the prefixer, and `to_var()` runs every name through it.
* `NORMAL_VALUE: Option<NodeRef>` — `builder.context.setupNormal`'s *result*,
  i.e. the material's `normalNode`. `NodeMaterial::setup()` installs it for the
  whole of the material's setup with `with_material_normal( material.normal_node,
  … )`, which is the direct equivalent of Three assigning the thunk to
  `builder.context`: both make the material's choice visible to any accessor
  reached from anywhere inside the setup, without passing it down by hand.

`normal_view()` then reads both:

```rust
let layer = SUB_BUILD.with(|s| *s.borrow());
let value = if layer.is_some() { None } else { NORMAL_VALUE.with(|v| v.borrow().clone()) };
```

Inside `NORMAL` it is the geometric normal; outside it, the material's
`normalNode` if there is one. The memo is keyed on `(layer, value)` rather than
being a singleton, so the two meanings coexist as two vars in one shader —
which is exactly what the dump shows.

### Tagging is on ancestors, not descendants

Three tags the node a layer is *declared through*, and the tag propagates up
`builder.chaining` to that node's **ancestors** — not down to what it is built
from. So in the dump `NORMAL_TBNViewMatrix` is prefixed but the two vars it is
assembled from, `tangentViewFrame` and `bitangentViewFrame`, are not, even
though they are only ever reached from inside the layer. `to_var_untagged()` is
the sibling of `to_var()` that skips the prefixer for exactly these.

### Scope

One layer at a time is all the ladder needs, and `NORMAL` is the only name so
far. Three also runs the position chain as a `VERTEX` sub-build; this port does
not (see §8, "`VERTEX_` sub-builds" — cosmetic, because nothing reads a second
meaning of a vertex-stage node). If a later rung needs nested or concurrent
layers, `SUB_BUILD` becomes a `Vec<&'static str>` and `sub_build_name()` joins
it, which is what `getSubBuildProperty()` does.

## 8. Known divergences from the dumps

Textual identity is not a goal; these are the deliberate or unexplained
differences, each verified to be pixel-neutral.

* **Generated names.** `nodeVarN` / `nodeUniformN` / `nodeVaryingN` counters are
  allocated by this port's own traversal order, so the numbers differ from the
  dumps even where the structure matches. Semantic names (`DiffuseColor`,
  `positionLocal`, `modelViewMatrix`, `v_normalViewGeometry`,
  `cameraProjectionMatrix`) do match.
* **`DiffuseColor.w = 1.0`.** Three emits it for three of the four examples but
  not for `webgpu_depth_texture`'s scene material, despite identical material
  settings (`transparent: false`, `blending: NormalBlending`, so
  `builder.isOpaque()` is true) — unexplained. This port emits it uniformly. It
  is pixel-neutral here because `materialOpacity` is 1 in every rung-1–4
  material, so the preceding `w = w * opacity` already leaves 1.
* ~~**Output-pass depth attachment.**~~ Withdrawn at rung 9: the port's canvas
  passes do carry the `depth24plus` attachment three.js gives them
  (`canvas_pass( true )`), and the pipeline declares `less-equal` /
  `depthWriteEnabled: true` to match `renderPipeline_RenderPipeline_19` in the
  rung-9 dump.
* **Canvas format.** `rgba8unorm` instead of Chrome's preferred `bgra8unorm`,
  so readback is already in comparator order. Same 8-bit unorm precision.
* **Declaration order.** `var<private>` declarations and `fn` definitions are
  emitted in the order the flow first reaches them; Three emits them in its own
  cache order. The set is identical, so naga sees the same program.
* **`VERTEX_` sub-builds.** Three runs `subBuild( node, 'VERTEX' )` for the
  position chain, which gives its vertex stage a second name for every node
  (`VERTEX_nodeVarN`) and one extra temp: `nodeVarN = cameraProjectionMatrix *
  vec4( v_positionView, 1.0 ); v_modelViewProjection = nodeVarN;`. This port
  writes the varying directly. Cosmetic.
* **Property-assignment temps.** Where Three writes `nodeVarN = expr; prop =
  nodeVarN;` this port writes `prop = expr;`, and Three's no-op
  `indirectDiffuse = vec4<f32>( 0.0, 0.0, 0.0, 0.0 ).xyz;` is omitted.
* **Instance buffer binding indices.** The two bindings of the instanced
  material's object group are swapped relative to the dump; the layout is built
  from the same descriptors the shader is, so they cannot disagree.
* **Matrix column index literal.** `m[ 0u ]` where Three prints `m[ 0 ]`.
* **MaterialX `fn` declaration order.** Three emits the `mx_*` helpers in
  dependency order (`mx_select`, `mx_negate_if`, `mx_gradient_float_1`,
  `mx_gradient_vec3_1`, `mx_trilerp_1`, …); this port's `Node::Call` arm emits
  the function before it generates the call's arguments, so a caller lands
  before its callees. WGSL has no forward-declaration rule for functions
  defined in the same module, so naga accepts both orders and the bodies are
  identical.
* **Splatted vector constants.** `clamp( x, vec3<f32>( 0.0, 0.0, 0.0 ),
  vec3<f32>( 1.0, 1.0, 1.0 ) )` where Three prints `vec3<f32>( 0.0 )` /
  `vec3<f32>( 1.0 )`. Same value.
* **Shared sub-expressions across chains.** Where a single node feeds both the
  position chain and the colour chain of one material — the ground's
  `mx_fractal_noise_vec3` in `webgpu_shadowmap` — Three's cache emits it once
  and reuses the temp; this port re-evaluates it in each chain. The function is
  pure, so the values agree; only the instruction count differs.
* **Fog parameters as constants.** `fogColor` / `fogNear` / `fogFar` are folded
  into the WGSL as literals rather than carried as `renderStruct` members,
  because nothing in the port animates them. Same numbers.
* **JPEG decode.** `TextureLoader` decodes through `zune-jpeg`; Chromium uses
  libjpeg-turbo, so the inverse DCT rounds differently. Measured on
  `uv_grid_opengl.jpg` against the browser's own decode: 34030 of 4194304
  channels differ by 1, 4428 by 2, 16 by 3; worst per-texel RGB distance 3.46,
  against a comparator threshold of 44.
