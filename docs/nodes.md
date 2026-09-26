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
| ~~compute (rung 12)~~ | Done — see §11. `Stage::Compute` is reachable, `BufferSource::Storage` declares `var<storage>` and `build_compute()` emits the `@compute` entry point. |
| ~~`SpriteNodeMaterial` (rung 13)~~ | Done — see §10. `position_view()` is context-driven the way `normal_view()` is, and `MaterialKind::Sprite` supplies the billboarded `vec4`. |
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

Rung 6 added a third input to that key: `builder.isFlatShading()`, which
`normalViewGeometry` switches on (`normalFlat` — the screen-space derivative
cross product — instead of the `v_normalViewGeometry` varying). The key stands
in for Three's *per-build* `nodeData`, so every fact the accessor reads has to
be in it; with the flag missing, a flat-shaded material built after a smooth one
in the same process silently reused the smooth normal.
**Everything that reads `normalView` has to share that key.** `normalWorld` and
the `tangentView` / `bitangentView` pair both do, through `normal_key()`. They
were plain `accessor!` singletons until rung 8, which made them bake in whatever
`normalView` the *first* material built in the process happened to resolve to;
every later material then re-emitted `normalView = normalViewGeometry;` after
its bump-mapped assignment, silently replacing the perturbed normal with the
geometric one for the rest of the shader. Only `webgpu_lights_physical`
exercised it — it is the first ladder example with both a `bumpMap` and a
`HemisphereLight` (the one `normalWorld` reader) on the same material.

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

* **`enable subgroups;` and `@builtin( subgroup_size )`.** Three's compute
  template emits the directive and threads a `subgroupSize : u32` parameter
  into every `@compute` entry point, whether or not the kernel uses either;
  `webgpu_compute_points` uses neither. This port emits neither, because
  `enable subgroups;` is a WebGPU feature request that fails to compile on an
  adapter without the feature, and an unused entry-point parameter is the only
  thing it would buy. If a rung ever ports `subgroupAdd` and friends, the
  directive comes back conditional on the flow reading them — which is what
  three.js should be doing. `tests/nodes_compute_wgsl.rs::canonical()` strips
  both before diffing.
* **One storage binding, not two.** Three allocates a fresh `NodeStorageBuffer`
  per stage (`sharedNodeData` is declared but never written for storage
  buffers), so the *same* particle buffer appears twice in the points
  material's group 1: binding 0 visible to `FRAGMENT` and binding 2 visible to
  `VERTEX`. wgpu cannot give one resource two binding numbers in one group, and
  would not gain anything by it. This port emits one binding with
  `VERTEX | FRAGMENT`, which shifts the object group's later binding numbers by
  one. Same class as the instance-buffer binding swap below: the layout and the
  shader come from the same descriptors.
* **Widening-only `format()`.** Three's `NodeBuilder.format()` will narrow
  (`vec4` → `vec3` by swizzle) and re-wrap same-width casts; `wgsl::convert()`
  ports only the widening half (`vec2` → `vec4( v, 0.0, 1.0 )` and friends) and
  leaves same-width and narrowing casts to the explicit constructor the node
  already carries. The full ladder was tried first and moved 73 lines of WGSL
  across six green rungs, all of them Three re-wrapping a value it had just
  built; the widening half is the part that is load-bearing.
* **Generated names.** `nodeVarN` / `nodeUniformN` / `nodeVaryingN` counters are
  allocated by this port's own traversal order, so the numbers differ from the
  dumps even where the structure matches. Two more counters join them at
  `webgpu_postprocessing_bloom_selective`: a `NodeBuffer_N` block is numbered
  from the port's own buffer table rather than from three's node id
  (`NodeBuffer_0` against the dump's `NodeBuffer_1297`), and a uniform *buffer*
  does not consume a `nodeUniformN` index here where three.js gives it one —
  so `Bloom_comp`'s object struct runs `0, 2, 4, 6, 8, 10, 11` where three's
  runs `0, 3, 5, 7, 9, 11, 12`. Numbering the buffer would shift every green
  dump for nothing. Semantic names (`DiffuseColor`,
  `positionLocal`, `modelViewMatrix`, `v_normalViewGeometry`,
  `cameraProjectionMatrix`) do match.
* **`DiffuseColor.w = 1.0`.** Emitted under `builder.isOpaque()`, as
  `setupDiffuseColor()` does (see §9.1). Three emits it for three of the four
  examples but not for `webgpu_depth_texture`'s scene material, despite
  identical material settings (`transparent: false`, `blending:
  NormalBlending`, so `isOpaque()` is true there too) — unexplained; this port
  emits it for all four. It is pixel-neutral here because `materialOpacity` is 1
  in every rung-1–4 material, so the preceding `w = w * opacity` already
  leaves 1. **This habit stops at `Line2NodeMaterial`**, whose constructor sets
  `blending = NoBlending`: `isOpaque()` is false there, Three does not emit the
  line, and neither does the port. It is not pixel-neutral for a fat line —
  `alphaLine()` writes the coverage into `DiffuseColor.w` and the output pass
  reads it — so §12 treats it as a structural requirement, not a cosmetic one.
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
* **Uniform-struct member order.** Members are appended to a group's struct in
  the order the traversal first reaches them, so a struct's *layout* can differ
  from the dump's even when its membership and total size agree. The
  `screen_uniforms` material of `dump_wgsl` is the worked example: its render
  group holds the same six members as Three's `renderStruct` and comes to the
  same 288 bytes, and its object group the same five as `objectStruct` at the
  same 160, but `viewport` sits at byte 128 here and Three puts it elsewhere.
  This is safe only because the struct, its `std140`-style padding and the
  bytes the CPU writes all come from one description of the group (see
  `UniformContext::bytes`) — nothing outside the port ever addresses a member
  by offset. It is a divergence to *record*, not one to chase: matching Three's
  order would mean reproducing its node-cache order, which the "Generated
  names" bullet above already declines to do.
* **Instance buffer binding indices.** The two bindings of the instanced
  material's object group are swapped relative to the dump; the layout is built
  from the same descriptors the shader is, so they cannot disagree.
* **Attribute `@location` order.** On the instanced-attribute path (§9.2) this
  port assigns locations in flow order: since rung 6 `positionLocal = position`
  is emitted first, so `position` takes 0, the four instance-matrix `vec4`s
  1–4 and `normal` 5; Three's dump puts both geometry attributes first.
  Pixel-neutral, but note it is *not* free: the same flow order decides which
  `range()` buffer is filled first, which is load-bearing (§9.2,
  `tests/nodes_range_buffers.rs`). Same class as the binding-index swap: the vertex buffer
  layouts come from the same `AttributeSlot`s the shader's declarations do.
* **Matrix column index literal.** `m[ 0u ]` where Three prints `m[ 0 ]`.
* **`NORMAL_normalView` with no normal map.** Three always runs its
  `setupNormal` thunk, so even a material with no `normalNode` gets the
  sub-build pair `NORMAL_normalView = normalViewGeometry; normalView =
  NORMAL_normalView;`. This port's `normal_view()` short-circuits to
  `normalViewGeometry` and emits one var. Same value.
* **`ivec2` morph coordinate.** `getMorph()`'s `ivec2( x, y )` lands in a var of
  its own in Three's dump even though it is read once: `TextureNode` builds its
  uv snippet twice during the analyze stage, so `TempNode`'s usage count passes
  1. This port counts one usage, so `morph_reference()` asks for the var
  explicitly rather than leaving the join inlined — same shape, for a different
  reason.
* **Branch-local temps.** A node read from both arms of a `Select` is promoted
  to a var by this port (usage 2) and emitted in each arm; Three caches the
  property name per flow scope, so its `else` arm re-inlines the expression
  (`nodeVar10 = ( 1.0 / max( pow( length( nodeVar7 ), … )` in the rung-6
  fragment dump). Same value in both arms.
* **MaterialX `fn` declaration order.** Three emits the `mx_*` helpers in
  dependency order (`mx_select`, `mx_negate_if`, `mx_gradient_float_1`,
  `mx_gradient_vec3_1`, `mx_trilerp_1`, …); this port's `Node::Call` arm emits
  the function before it generates the call's arguments, so a caller lands
  before its callees. WGSL has no forward-declaration rule for functions
  defined in the same module, so naga accepts both orders and the bodies are
  identical.
* **`FlipNode`'s one-minus.** `screenUV.flipY()` prints `vec2<f32>( v.x, 1.0 -
  v.y )` in three, because `FlipNode.generate()` builds that component as a
  bare string. The port builds it out of `sub`, so it comes out parenthesised:
  `vec2<f32>( v.x, ( 1.0 - v.y ) )`. Same value.
* **One trailing blank line in `// codes`.** Three's assembly leaves one more
  empty line between the last `fn` and `@fragment` than this port's does. Pure
  whitespace; naga does not see it.
* **Splatted vector constants.** `clamp( x, vec3<f32>( 0.0, 0.0, 0.0 ),
  vec3<f32>( 1.0, 1.0, 1.0 ) )` where Three prints `vec3<f32>( 0.0 )` /
  `vec3<f32>( 1.0 )`. Same value.
* **Shared sub-expressions across chains.** Where a single node feeds both the
  position chain and the colour chain of one material — the ground's
  `mx_fractal_noise_vec3` in `webgpu_shadowmap` — Three's cache emits it once
  and reuses the temp; this port re-evaluates it in each chain. The function is
  pure, so the values agree; only the instruction count differs.
* ~~**Fog parameters as constants.**~~ Gone since `scene.fog` (§28): a page
  that sets `scene.fog` gets `fogColor` / `fogNear` / `fogFar` (or
  `density`) as `renderStruct` members, as three does. Only a hand-built
  `scene.fog_node` with literal arguments — `webgpu_lights_phong`'s, which
  is literal in the page too — still has constants.
* **`toConst` on the shadow filter.** `to_const()` exists since
  `webgpu_postprocessing_radial_blur` (`Node::Let`, a WGSL `let nodeConstN`),
  but `pointShadowFilter`'s `shadowPosition` and `shadowPositionAbs` are still
  `to_var()`s, where Three uses `toConst()`. Same single evaluation, same
  value. Two of them are not optional: `Node::Swizzle` and `Node::Neg` are not
  kinds the builder promotes on usage count, so `shadowCoord.xyz` and
  `viewZ.negate()` are wrapped by hand or the expression would be emitted
  twice.
* **`sphericalGaussianBlur` hoists its direction into a var.** Three inlines
  `normalize( outputDirection )` into both the `getFace` and the `getUV` call
  of the `mipInt == 0` arm, and inlines the whole spiral-sample direction
  (`axis * cos( theta ) + ( ... ) * sin( theta )`) twice inside the sample
  loop; the port emits each once as a `nodeVar` and reads it twice. Same
  arithmetic, one fewer evaluation, and it shifts every `nodeVarN` number in
  `m09` of `dump-postprocessing_ca/` by one or two. `color.divAssign(
  weightSum )` is spelled `color.assign( color.div( weightSum ) )` and prints
  identically.

* **Helper `fn` declaration order and `fn0`/`fn1` numbering.** The `// codes`
  block is emitted in the order the builder first *generates* a call, which is
  not the order Three declares them in, and an anonymous `Fn()` takes its
  number from that order too — so `premultiplyAlpha` / `unpremultiplyAlpha` are
  `fn0` / `fn1` in the radial-blur quad where Three has them the other way
  round. WGSL has no forward-declaration rule for functions in the same module,
  so naga accepts both orders and the bodies are identical. (The MaterialX
  entry below is the same cause, seen first.)
* **`Node::Block` needs an explicit var.** An effect built as a statement list
  ending in a value — `radial_blur()` — is wrapped in a `to_var()` by the port,
  because `Node::Block` is not a kind `generate()` caches or `analyze()`
  promotes, so without it the whole block would be emitted once per read. Three
  gets the same result from `Fn()`'s own stack. Same statements, same order.
* **`PassNode`'s outer var is always emitted.** `PassNode.setup()` returns a
  `PassTextureNode`, and both are `TempNode`s, so the pair *can* emit two vars —
  `nodeVar0 = textureSample( … ); nodeVar1 = nodeVar0;`. Three only promotes a
  `TempNode` to a var when `analyze()` saw it read more than once, so in the
  `RenderPipeline` quad of `webgpu_postprocessing_ssaa`, where the pass texture
  is read exactly once, Three emits only `nodeVar0` (dump `m05`) while this port
  emits the copy and shifts every later `nodeVarN` by one. The port builds the
  pass as `to_var( None, texture_uv( … ) )` unconditionally — the same choice
  the masking rung's dump *did* show, where the outer node is read twice.
  The copy is a `var<private>` assignment of a value already in a register; the
  colour is the same.
* **Named lighting temps.** Three names `singleScatteringDielectric`,
  `multiScatteringDielectric`, `singleScatteringMetallic`,
  `multiScatteringMetallic`, `dfg` and `multiScatteringCompensation`; this port
  leaves them as numbered vars or inlines them where they are read once.
* **MRT member values are not promoted to a var.** Three's G-buffer fragment
  for `webgpu_deferred` writes `nodeVar1 = vec4<f32>( v_positionView,
  Metalness ); output.m1 = nodeVar1;` — the joined `vec4` is a `TempNode` that
  `analyze()` saw twice, once through the MRT node and once through the output
  struct member. This port's `MrtValue::Deferred` builds the value once and
  writes it straight into the member: `output.m1 = vec4<f32>( v_positionView,
  Metalness );`. Same value, one fewer `var<private>`, and every later
  `nodeVarN` shifts down. (Named MRT members whose value is a property read —
  `webgpu_postprocessing_bloom_selective`'s — get no var in either, which is
  why this only shows up here.)
* **Hoisted accumulator zeros.** `LightingContextNode`'s five accumulators
  (`directDiffuse`, `directSpecular`, `irradiance`, `indirectDiffuse`,
  `indirectSpecular`) are zeroed together before the light loop rather than each
  at its first use. Nothing reads one before it is written either way.
* **Inlined `faceDirection` and the extra `length()` temp.** Three keeps
  `faceDirection` and the point light's `length( lVector )` as their own vars;
  this port inlines the first and re-emits the second inside each arm of the
  distance-attenuation `if`/`else`, exactly as the dump does in the `else` arm.
* **Render-struct member order.** `nodeUniformN` numbers are assigned at
  *generation* time, but three.js orders the `renderStruct` / `objectStruct`
  members by the order the uniform *objects* were created — `uniform()` eagerly,
  `reference()` lazily — so dump 18's members run `18, 29, 30, 32, 33, 31, 17,
  19, 23, 22, 21, 24, 26, 27, 28`. This port appends members in generation
  order, and numbers without the gaps three leaves for uniforms it consumes but
  never emits (20, and object-group 8 in dump 18). The layout is built from the
  same member list the shader is, so they cannot disagree.
* **Builtins before varyings.** `@builtin( front_facing )` / `@builtin(
  position )` are emitted ahead of the `@location` parameters of `main`, and the
  `@location` numbering follows this port's varying order.
* **`NoBlending` on the shadow override material.** Three's shadow material sets
  `blending = NoBlending`, which makes `builder.isOpaque()` false and drops the
  `DiffuseColor.w = 1.0` line (dump 24). This port has no `blending` field and
  emits the line. The shadow pass's colour attachment is never sampled — only
  its depth is — so it is unobservable.
* **JPEG decode.** `TextureLoader` decodes through `zune-jpeg`; Chromium uses
  libjpeg-turbo, so the inverse DCT rounds differently. Measured on
  `uv_grid_opengl.jpg` against the browser's own decode: 34030 of 4194304
  channels differ by 1, 4428 by 2, 16 by 3; worst per-texel RGB distance 3.46,
  against a comparator threshold of 44.
* **meshopt filters follow the SIMD build, on purpose.** three.js'
  `meshopt_decoder.module.js` runs meshoptimizer's WebAssembly SIMD filters
  wherever SIMD validates (every current browser, and node). Those round
  `x + 0x1.8p23` to nearest-even and keep the low bits of the float, where the
  scalar C++ filters, and the `meshopt-rs` crate's copy of them, round half
  away from zero, and the crate's quaternion filter also reads its components
  unsigned. `src/loaders/meshopt.rs` ports the SIMD kernels operation for
  operation; `tests/gltf_meshopt.rs` holds them to the WebAssembly bytes.
* **Malformed `EXT_meshopt_compression` is an error.** A mode, filter and
  `byteStride` the extension does not allow together is `GltfError::Meshopt`
  up front; meshoptimizer only `assert`s them, and its release WebAssembly
  build compiles the asserts out and decodes garbage.

* **The skin matrix is CSEd.** `getSkinnedNormalAndTangent()` builds
  `bindMatrixInverse * skinMatrix * bindMatrix` once and reads three columns off
  it; Three re-emits the whole 4x4 product inside each `mat3x3<f32>` argument
  (three times in `m03_vertex_Ch03_Body`), because `OperatorNode` is a
  `TempNode` whose usage count is taken per *generated* argument. This port
  promotes it to one var and indexes that. Same matrix.
* **No bone texture.** `getBoneMatricesNode()` picks a `DataTexture` over the
  uniform array when the skeleton exceeds the device's uniform-buffer limit.
  Only the uniform branch is ported (`src/nodes/skinning.rs`); Michelle is 65
  bones = 4160 bytes, and a skeleton large enough to need the texture would
  fail loudly on the binding, not silently in the pixels.
* **`TBNViewMatrix` is keyed by sub-build *and side*.** Three caches the frame
  on the builder; this port caches it in a map keyed by the sub-build layer and
  `material.side`, because `negateOnBackSide` makes a `DoubleSide` material's
  frame a different expression from a `FrontSide` one and a process-wide cache
  would hand the first one out forever. Same expression per material.
* **`batch()`'s `toConst` block becomes vars (rung 11).** Every value
  `BatchNode` builds — the indirect texel coordinate pair, the matrix and
  colour coordinates, the four matrix rows — is a `toConst()` in Three, so its
  dump reads `let nodeConst0 = …` in construction order. This port has no
  `let`; `batch()` pushes the same values onto the statement list as
  `nodeVarN` vars, in Three's construction order, so the *sequence* of
  statements matches one for one and only the keyword and the numbering
  differ. Same class as the `toConst` entry above.
* **`mat3( batchingMatrix )` is hoisted (rung 11).** Three inlines the whole
  `mat3x3<f32>( m[0].xyz, m[1].xyz, m[2].xyz )` construction seven times in
  the `normalLocal` line, because `MathNode`'s usage count is reset per
  sub-build. This port sees seven usages of one node and hoists it to
  `nodeVar17`. Identical arithmetic, one seventh of the text.
* **`vBatchIndirectId` is declared and never read (rung 11).** Three's
  `batchIndirectIndex` is a `varyingProperty`, so it is written in the vertex
  stage and declared in the fragment stage even though nothing in this
  material reads it. The port reproduces the declaration, but its
  `@location` numbers differ from the dump's: the port builds the fragment
  stage first, so the varyings are numbered in fragment-flow order
  (`vBatchIndirectId` 0, `vBatchColor` 1, `v_normalViewGeometry` 2) where
  Three has 0/1/2 the other way round. Same class as "Attribute `@location`
  order".
* **Data-texture binding indices (rung 11).** Three's object group puts the
  uniform buffer at binding 0 and the three data textures at 1–3; the port
  emits the textures first and the buffer last. Same class as "Instance buffer
  binding indices": the layout and the shader come from the same descriptors.
* **The indirect-diffuse block is emitted before the environment's (§25).**
  `PhysicalLightingModel.indirectDiffuse()` reads `irradiance`, and three runs
  it after `EnvironmentNode` has written `radiance` / `iblIrradiance`; this port
  pushes it while building `indirectDiffuse`, which is before. Nothing between
  the two points writes `irradiance` — on a lit page the light loop has already
  run, and on `webgpu_loader_gltf_sheen` there are no lights at all — so the
  statements are the same statements in a different order, with the same
  values. It is visible in the dump as the whole `singleScattering` /
  `multiScattering` pair (and, with sheen, the first `IBLSheenBRDF` term)
  sitting above `radiance = vec3<f32>( 0.0, 0.0, 0.0 )` instead of below it.
* **`IBLSheenBRDF` is inlined three times (§25).** Three gives it no
  `setLayout`, so it is not a `fn`; each of its three readers — the `irradiance`
  sheen term, the `iblIrradiance` one and the energy compensation — rebuilds
  the whole fit. The port does the same, because the three calls are three
  separate `NodeRef`s and the builder promotes by `Rc` identity. Identical
  arithmetic, identical text apart from the temp numbers.
* **`uv().flipY()` as a texture coordinate is not re-declared (§29).** In
  `webgpu_textures_2d-array_compressed` three writes the flip as `nodeVar0 =
  nodeVarying4; let nodeVar0 = vec2<f32>( nodeVar0.x, 1.0 - nodeVar0.y );` —
  a `let` shadowing the var it was just assigned — and samples with the `let`.
  The port keeps the var and puts the `vec2` inline in the `textureSample`
  call. Same expression evaluated once either way; the parenthesised one-minus
  is the `FlipNode` entry above.

* **A shared conversion is written out at each use (issue #141).** Three's
  `Node.build()` caches *any* cacheable node read more than once as
  `let nodeConstN`; the port promotes only `Op`, `Math` and `Join` nodes
  (`needs_var`) and leaves a shared `Cast` inline, so
  `determinant( mat3x3<f32>( m ) )` / `inverse( … )` repeat the constructor
  where three names it once. Same expression, same value; widening the
  promotion to casts would renumber the temps in every green dump.
  `tests/nodes_tsl_batch.rs::inline_let()` folds three's `let` back in before
  comparing.
* **`let nodeConstN` against `var nodeVarN`.** Where three caches a shared
  value it writes an immutable `let`; the port's promoted temp is a `var`.
  Pixel-neutral; `nodes_tsl_batch.rs` maps the one onto the other.
* **`billboarding()`'s matrices are explicit vars (issue #141).** Three
  assigns straight into the `modelWorldMatrix` / `modelViewMatrix` operator
  nodes and lets its builder turn them into temps; the port asks for the two
  `var`s itself (`to_var`) and assigns their elements in the flow. Same
  statements, same order, same value — `webgpu_tsl_vfx_flames` grades 31 of
  100000.

### `LineBasicNodeMaterial` adds no divergence class

Three's own `LineBasicNodeMaterial` program, dumped off
`~/src/projects/d33/rung0/examples/d3_treemap.html`, is in
`docs/lines/LineBasicNodeMaterial_27.{vert,frag}.wgsl` and `.layout.txt`.
Diffed against this port's `line_basic` section of `examples/dump_wgsl.rs`
(`MeshBasicNodeMaterial::line( 0xffffff )`), the only differences are the
banner, the `nodeUniformN` / `nodeVarN` counters, `var<private>` declaration
order and Three's `VERTEX_` sub-build temps — the "Generated names",
"Declaration order" and "`VERTEX_` sub-builds" entries above. Nothing about the
material is line-specific: `LineBasicNodeMaterial` is a bare `NodeMaterial`
with `setDefaultValues( new LineBasicMaterial() )`, and every default it sets
that reaches WGSL (`color`, `opacity`, `fog`, `transparent`) is one
`MeshBasicNodeMaterial` already has. `linewidth` / `linecap` / `linejoin` are
SVGRenderer-only — WebGPU always draws a one-pixel line. Hence no
`MaterialKind::Line`; see `src/materials/mod.rs`'s `line()`. What is
line-specific is the *pipeline*, and it comes from the object
(`getPrimitiveTopology( object, material )`), not the material.

### `vertexColor()` has no "no attribute" fallback

Three's `VertexColorNode.generate()` asks `builder.hasGeometryAttribute(
'color' )` and, when the geometry has none, emits a white `vec4` constant
instead of the attribute; `setupDiffuseColor()` guards the multiply with the
same question. The port has no geometry in hand at setup time —
`SetupContext` is the whole of what `setup()` reads off the object, and it
carries no attribute list — so `material.vertex_colors` alone decides, and a
material that sets it must be drawn with a geometry that has a `color`
attribute or the renderer panics naming it. Adding the flag to `SetupContext`
would make it part of the program cache key for every material, to model a
case (`vertexColors` on a geometry without colours) that is a consumer bug
either way.

Three's `vertexAlphas` branch — a four-component `color` attribute whose alpha
reaches `DiffuseColor.a` — is not ported. `vertexColor()` is declared `vec4`
as in three, and the three-component attribute is widened with an alpha of 1
by the same rule as `NodeBuilder.format()`.

The widening happens in the *vertex* stage, before the varying, so the
interpolated value is a `vec4` and the fragment stage reads it whole — three's
`VertexColorNode` is an `AttributeNode` declared `vec4`, and `webgpu_materials`'
grid helper (m13/m14) pins the shape. Widening after the varying would give the
same pixels, because a constant interpolates to itself, but a different varying
layout.

### `Loop()` as a value renders black, on purpose

`webgpu_materials`' last teapot sets `colorNode` to a `Loop()`. `LoopNode` is a
statement node: `generate()` writes the `for` into the flow and returns an
**empty snippet**. `setupDiffuseColor()` then does `vec4( colorNode )`, which
casts nothing, and three emits

```wgsl
	DiffuseColor = vec4<f32>(  );
```

— two spaces, a zero-initialised `vec4`, and the four texture taps the loop ran
are thrown away. The teapot is opaque black in three's own reference image.

**This port reproduces it bit for bit.** `Node::Cast` formats
`"{type}( {snippet} )"`, and with the `Loop` arm's empty snippet that is exactly
three's text. The loop body is still built, so its texture bindings are still in
the layout and its cost is still in the frame.

Not fixing it is the point: the graded frame contains that black teapot, and a
port that made the loop's value flow through would lose those pixels. Pinned by
`tests/scene_webgpu_materials.rs`'s
`a_loop_as_a_color_node_discards_its_result`, rather than left to the 0.1%
threshold, which one teapot out of seventeen would very nearly slip past.

### `triplanarTexture` takes textures, not texture nodes

Three's `triplanarTextures()` is handed texture *nodes* and reads `.value` back
off each one to rebuild a tap per axis. A `NodeRef` here is an opaque
`Rc<Node>` with no way back to the `Texture`, so `triplanar_texture()` takes
the maps themselves; `None` for the y or z map means "sample x", exactly as
three's `null` does. The generated WGSL is identical (m11/m12).

### `wgslFn`: what is parsed and what is not

`src/nodes/code.rs` ports `WGSLNodeFunction.js`: three's declaration regex as a
hand-written scanner, `wgslTypeLib` as far as the port's `Type` reaches, and
`getCode()`, which rebuilds `fn <name> ( <inputs> ) -> <out>` and then appends
the source's own block **verbatim** — the example page's six-tab indentation
and the whitespace a JS template literal leaves after the closing brace both
end up in the shader, and three's dump pins that.

Left out, because nothing on the ladder reaches it: a `void` return, `ptr<>`
parameters, three's GLSL sibling (`glslFn`), and `wgslFn` in the vertex stage.
A declared type the port does not model panics at setup; three silently
produces a shader that will not compile.

Two things about the arguments are worth knowing:

* a `texture_2d<f32>` or `sampler` parameter takes a texture *node* and is
  passed the binding, not a sample of it. Those arguments are therefore left
  out of `sources()`: counting one would promote it to a var and emit a
  `textureSample` nothing reads.
* one `texture( map )` node bound to both parameters gives `nodeUniform0` and
  `nodeUniform0_sampler`, which is how three's example writes it.

`includes` are emitted before their caller, depth first, so `someFn` can call
`desaturate` (m28).

### MRT adds no divergence class, and reproduces two quirks on purpose

The MRT fragment of `webgpu_postprocessing_bloom_selective` is the first shader
on the ladder that matches three's dump in full — no entry here. Two details of
that match are deliberate rather than incidental, and a tidier port would lose
them:

* **the struct's name changes with the shape.** A single-attachment fragment
  declares `struct OutputStruct { @location( 0 ) color: vec4<f32> }`; an MRT
  one declares `struct OutputType` with `m0`, `m1`, …. That is two different
  classes in three (`OutputStructNode` names its own type `OutputType`), not
  one type with a variable member count, and both names are in the dumps.
* **`OutputType` ends with a blank member line** — a `,` then a line holding
  one tab. `getStructMembers()` pushes `` `\t${ this.getBuiltins( 'output' ) }` ``
  as a final member for any output struct, and a fragment stage with no output
  builtins makes that the tab alone. Reproduced literally.

One thing that is *not* a divergence but is easy to read as one: with an MRT,
`Output` is analysed once where the ordinary path analyses it twice. The second
analysis exists because the basic path reaches the value through both `Output`
and `output.color`, which promotes it to a var; an MRT's `output` member reads
the property back instead, so the count is one and no var appears. Three gets
there by the same route — `getFragmentOutput` vs. `OutputStructNode` — and its
dump has no var either.

### `webgpu_postprocessing_anamorphic` adds one divergence

Four of its five modules match three's dump statement for statement: the
background (`m01`), the scene fragment (`m03`), the `rtt()` quad (`m05`) and
the streak (`m07`) — including the loop header
`for ( var i : i32 = i32( ( - nodeVar1 ) ); i < i32( nodeVar1 ); i ++ )`, which
is what `Loop( { start, end } )` with **node** bounds generates and what
`tsl::loop_range` was added for. The final `RenderPipeline` quad (`m17`) differs
only in the two orderings already listed above (struct blocks and helper `fn`
declaration order).

The vertex stage is where the port and three genuinely disagree:

* **`positionNode` runs before the instance matrix, not after.** Three's
  `NodeMaterial.setupPosition()` applies instancing first and *then*
  `positionLocal.assign( positionNode )`, so the page's bob is added to an
  already-instanced position; this port applies `positionNode` first (see the
  comment at `src/materials/node_material.rs`, `setup()`, which names the
  downstream consumer written against it). The two agree whenever the
  instance matrix is an affine map with no scale or rotation applied to the
  offset — and `webgpu_postprocessing_anamorphic`'s instance matrices are pure
  translations, so the graded frame is unaffected. A rung that instances with
  rotation *and* a `positionNode` will have to pick three's order.

### The MaterialX library (#142)

`src/nodes/materialx/` ports every export of `MaterialXNodes.js` except
`mx_frame`, which reads `frameId`, a node the port does not have. Three's
unreachable internal overloads (`mx_hash_int_0/3/4`,
`mx_cell_noise_float_0/3`, `mx_cell_noise_vec3_0/3`) are also not ported:
no export resolves to them. `tests/nodes_mx_library.rs` compares every
`mx_*` fn line for line with r186's dump of
`tests/fixtures/materialx_library/page.html`. It makes two concessions:

* **`mx_hsvtorgb` is compared by skeleton only.** Three's body reads `f`,
  `p`, `q` and `t` from vars it assigned in a *sibling* `If` branch, so its
  WGSL is wrong for hue sectors 2 to 5. The port assigns each value in the
  branch that reads it. The signature and every `if` / `else` / `for` /
  `return` line still match.
* **`uv()`'s varying name is normalised.** The port numbers `nodeVaryingN`
  from its own counter; three's page happens to make it `nodeVarying3`.

The library needed one change in the builder:

* **A same-type `Node::Cast` prints as its operand.** This is three's
  `ConvertNode` when no conversion is needed (`float( x )` on a float). It
  is still a node of its own, so wrapping a value in it counts that value
  once and keeps it inline. `mx_place2d` needs this to leave its `div` where
  three leaves it.

The `webgpu_tsl_raging_sea` rung, the library's graded consumer, needed four
more (`docs/webgpu_tsl_raging_sea-progress.md`):

* **A non-`int` loop bound is written `i32( … )`.** This is what
  `LoopNode`'s `.build( builder, 'int' )` does. `wgsl::convert` still has no
  same-length arm, so the loop header writes it.
* **An inlined `Fn()` block is built once per scope.** A second reader gets
  the result, not a second run of its statements. Three builds a stack once
  per stage.
* **A layout `fn` is emitted into each stage that calls it.** Each stage is
  its own module. The name is shared.
* **A varying the vertex stage has assigned to is written from that var.**
  In three `positionLocal` *is* the varying, so the fragment stage reads the
  value after `positionLocal.assign( positionNode )`. The port writes
  `varyings.positionLocal = positionLocal;` after the assignment instead of
  three's `varyings.positionLocal = ( varyings.positionLocal + … )` in
  place: the text differs, but the value the fragment reads is the same. A
  varying that was only read keeps its old form.

## 9. Blending, and the instanced-attribute path

Two pieces of shared renderer work that no rung 1–9 material exercises, built
for rung 13 (`webgpu_tsl_galaxy`), sdf-text, and any scene past ~1024 instances.

### 9.1 Blend state

`src/materials/blending.rs` is `WebGPUPipelineUtils`' `_getBlending()` /
`_getBlendFactor()` / `_getBlendOperation()`, with `constants.js`' blending
enums. The material carries Three's fields — `blending` (default
`NormalBlending`), `transparent`, `premultiplied_alpha`, `alpha_to_coverage`,
and the six `blendSrc` / `blendDst` / `blendEquation` (+`Alpha`) slots for
`CustomBlending`.

The gate is Three's, in `_getBlending()`:

```
blending !== NoBlending && ( blending !== NormalBlending || transparent )
```

so an opaque `NormalBlending` material gets no blend state at all — which is why
the rungs' generated pipelines are unchanged. `RenderState` carries
`Option<wgpu::BlendState>` and it is part of the pipeline cache key, so one
program can serve an opaque and a transparent draw.

The table, with `premultipliedAlpha: false` on the left (Three's default) and
`true` on the right, as `( colorSrc, colorDst, colorOp ) / ( alphaSrc, alphaDst,
alphaOp )`:

| blending | non-premultiplied | premultiplied |
|---|---|---|
| `No` | no blend state | no blend state |
| `Normal` | `(SrcAlpha, OneMinusSrcAlpha, Add)` / `(One, OneMinusSrcAlpha, Add)` | `(One, OneMinusSrcAlpha, Add)` / `(One, OneMinusSrcAlpha, Add)` |
| `Additive` | `(SrcAlpha, One, Add)` / `(One, One, Add)` | `(One, One, Add)` / `(One, One, Add)` |
| `Subtractive` | unsupported — Three logs an error and leaves the blend undefined | `(Zero, OneMinusSrc, Add)` / `(Zero, One, Add)` |
| `Multiply` | unsupported — same | `(Zero, Src, Add)` / `(Zero, SrcAlpha, Add)` |
| `Custom` | the material's own factors and equations, `blendSrcAlpha ?? blendSrc` etc. | same |

`blending.rs`' unit tests assert every row and every factor / equation mapping
against that source, so a future edit cannot drift from it silently.

`material.is_opaque()` is `NodeBuilder.isOpaque()`: `transparent === false &&
blending === NormalBlending && alphaToCoverage === false`. At r186 `alphaHash`
is **not** part of it (it is a separate discard). `setupDiffuseColor()` emits
`DiffuseColor.w = 1.0` only when that is true, so a transparent material's
per-fragment alpha now reaches the output and the transparent render list (drawn
after opaque, back to front — `reverse_painter_sort_stable`) shows it.

### 9.2 `range()` and `instanceMatrix` as instanced vertex attributes

`RangeNode.setup()` and `createInstanceMatrixNode()` both branch on
`uniformBufferSize <= builder.getUniformBufferLimit()`, the limit being
`device.limits.maxUniformBufferBindingSize` — 65536 in Chrome and under wgpu's
`Limits::default()`, so the port branches exactly where Three's dumps do:

| node | uniform-buffer size | branches at |
|---|---|---|
| `instanceMatrix` | `max( count, 1 ) * 16 * 4` | 1024 instances |
| `range( min, max )` | `count * 4 * 4` | 4096 instances |

Under the limit: a uniform buffer read as `buffer( array, type, count ).element(
instanceIndex )`, which is what rung 2 draws (1000 instances). Over it:

* the matrices become `new InstancedInterleavedBuffer( array, 16, 1 )`, read as
  four `instancedBufferAttribute( interleaved, 'vec4', 16, offset )` views at
  float offsets 0/4/8/12 and joined back with `mat4( … )` — one vertex buffer,
  `array_stride` 64, attribute offsets 0/16/32/48, `VertexStepMode::Instance`;
* a `range()` becomes one instanced `vec4` attribute, `array_stride` 16. Because
  the port reads it in the fragment stage, `AttributeNode.generate()`'s rule
  applies — an attribute read outside the vertex stage becomes `varying( this )`
  — so the whole `vec4` crosses through a generated varying rather than a
  flat `instanceIndex` lookup.

`Node::InstancedAttribute { buffer: Rc<InstanceBuffer>, offset, ty }` is the node.
**`Rc` identity is the buffer's identity everywhere**: the builder names
attributes by `( Rc::as_ptr( buffer ), offset )`, `BindingDesc::Buffer` carries
the source node's `id`, and the renderer caches one GPU buffer per id, uploaded
once. So two `range( 0, 1 )` calls are two buffers with two different random
fills, exactly as two `RangeNode`s are in three.js — a cache keyed on the
*values* (`min`, `max`, `count`) would collapse them and hand every instance the
first node's numbers, with no error anywhere.

Buffer identity is deliberately **not** in `cache_key`: the instance matrix's
node is made afresh by every `setup()` and the generated WGSL does not depend
on which one it was.

`NodeProgram::vertex_buffers()` is `WebGPUAttributeUtils.createShaderVertexBuffers()`:
the `@location`-ordered attributes grouped into layouts in first-use order, one
buffer per geometry attribute and one per `InstanceBuffer`. The renderer resolves
*both* bind groups and vertex buffers from the material's own `NodeProgram`
(memoised per material, `docs/scene-graph.md` "Program cache"), never from the
WGSL-keyed program cache, so two materials with identical shaders and different
buffers cannot alias.

Coverage: `tests/nodes_instanced_attributes.rs` pins each branch on both sides of
both limits and the two-`range()` rule; `tests/renderer_instanced.rs` draws 1000,
2000 and 5000 instances headless and counts pixels; `tests/nodes_range_buffers.rs`
pins rung 13's four buffers, fill order included.

### 9.3 A `range()`'s `min` / `max` are `Vector4`s, and they disagree about `w`

`RangeNode.setup()` widens each end to a `Vector4` by three different rules:

| value | `Vector4` |
|---|---|
| a scalar — `range( 0, 1 )` | `setScalar( v )`, i.e. `( v, v, v, v )` |
| a `Color` — `range( new Color( … ), … )` | `( r, g, b, 1 )` |
| any other vector — `range( vec3( -1 ), vec3( 1 ) )` | `( x, y, z \|\| 0, w \|\| 0 )` |

So a `vec3` range has `w` **0 at both ends**, not 1. The fourth draw per instance
is still consumed — the loop is `stride * count` long regardless — it just lands
on a constant. `BufferSource::Range` therefore carries `[f64; 4]` rather than a
`Color`; the `Color` spelling the port shipped at rung 2 forced `w = 1` and would
have shifted rung 13's whole random sequence had it survived.

`RangeNode.getNodeType()` is the value's own type, and the buffer is a `vec4`, so
the node ends in a `convert()` — which for a narrowing is `NodeBuilder.format()`,
i.e. a swizzle: `range( 0, 1 )` reads `….x` and `range( vec3( … ), … )` reads
`….xyz`. `tsl::instanced_range` returns the narrowed node, so callers do not
swizzle again.

## 10. `SpriteNodeMaterial` and the `setupPositionView` seam (rung 13)

`NodeMaterial.setup()` installs three entries on `builder.context` before either
stage is flowed:

```js
builder.context.setupNormal = () => subBuild( this.setupNormal( builder ), 'NORMAL', 'vec3' );
builder.context.setupPositionView = () => this.setupPositionView( builder );
builder.context.setupModelViewProjection = () => this.setupModelViewProjection( builder );
```

`positionView` and `modelViewProjection` are `Fn( … ).once()` accessors that read
the last two. The port mirrors `NORMAL_VALUE` / `with_material_normal` exactly:
`POSITION_VIEW_VALUE` holds the override for the duration of
`materials::setup()`, and `position_view()` / `model_view_projection()` come out
of one cache keyed on that value's `NodeRef::key()`, so two materials in one
process cannot inherit each other's node.

The base class' value is `modelViewMatrix.mul( positionLocal ).xyz` — a `vec3`.
`SpriteNodeMaterial.setupPositionView()` returns a **`vec4`**, which is why
`v_positionView` is declared `vec4<f32>` in this rung's shader and why
`cameraProjectionMatrix.mul( positionView )` needs no padding. Nothing in the
fragment stage reads it, so it is emitted as a `var<private>`, not a varying.

The sprite vertex shader itself (`SpriteNodeMaterial.js:110`), in full:

```js
const mvPosition = modelViewMatrix.mul( vec3( positionNode || 0 ) );
let scale = vec2( modelWorldMatrix[ 0 ].xyz.length(), modelWorldMatrix[ 1 ].xyz.length() );
if ( scaleNode !== null ) scale = scale.mul( vec2( scaleNode ) );
if ( camera.isPerspectiveCamera && sizeAttenuation === false ) scale = scale.mul( mvPosition.z.negate() );
let alignedPosition = positionGeometry.xy;                 // object.center is a Sprite field
alignedPosition = alignedPosition.mul( scale );
const rotation = float( rotationNode || materialRotation );
return vec4( mvPosition.xy.add( rotate( alignedPosition, rotation ) ), mvPosition.zw );
```

Three things fall out of it:

* **`sizeAttenuation` defaults to `true`**, and `true` is the branch that *omits*
  the `mvPosition.z.negate()` factor. The name reads backwards; the dump settles
  it.
* **`positionNode` is read twice** — once by `setupPosition()` as
  `positionLocal.assign( positionNode )`, once here — so it is always a
  `nodeVarN` temp, and the billboard is built around the *node's* value, not
  around `positionLocal` (which instancing has already transformed).
* **`materialRotation`** is a new object-group `f32` uniform
  (`SpriteMaterial.rotation`), landing at offset 96 in this rung's
  `objectStruct`, between `modelWorldMatrix` and the example's `size`.

`rotate()` is `RotateNode`'s `vec2` branch: `mat2( cos, sin, sin.negate(), cos
).mul( position )`, with `cos` and `sin` one node each used twice, so both become
temps. The `vec3`/`vec4` branch (three chained `mat4` rotations) is not ported.

`MaterialKind::Sprite` shares `Basic`'s fragment flow — `SpriteNodeMaterial`
leaves `lights` false, so `setupOutgoingLight()` is `diffuseColor.rgb`. What it
does change is `transparent`, which its constructor sets to `true`; that takes it
out of `isOpaque()`, so the `DiffuseColor.w = 1.0` line is absent and the
per-fragment alpha survives to `Output`.

### Divergences specific to this rung

Only the ones §8 already lists: generated name numbering, attribute `@location`
order (the four instance-matrix `vec4`s take 0–3 here, pushing `position` /
`normal` / `uv` to 4 / 5 / 9), `m[ 0u ]`, the absent `VERTEX_` temps, and
`var<private>` declaration order. Statement for statement the vertex and fragment
flows match `handoff/scouts/rung13/{vertex,fragment}-r186.wgsl`.

## 11. The compute stage (rung 12)

`webgpu_compute_points` is the first example whose work does not happen in a
render pass. Nothing about it is a new kind of node: `Fn().compute()` is the
same flow machinery pointed at a different entry-point template, and the
particle buffer is a `BufferSource` like any other. What is new is a third
stage, a buffer the shader writes, and a frame made of four submits.

### 11.1 `ComputeFlow` and `build_compute()`

`ComputeFlow` is the port of three's `ComputeNode`: a list of statements, the
number of invocations wanted, a workgroup size, an optional name, and an
optional `on_init` — a second `ComputeFlow` the renderer runs exactly once,
the first time it sees this one, which is three's `computeNode.onInit`.

`NodeBuilder::build_compute()` mirrors `build()`: analyze, generate, assemble.
Three things differ.

* **The entry point.** `@compute @workgroup_size( x, y, z )` over
  `@builtin( global_invocation_id ) globalId : vec3<u32>`, with
  `instanceIndex = globalId.x` assigned in the prologue. `IndexNode.generate()`
  picks the raw builtin in the vertex and compute stages, a flat varying in the
  fragment stage, and an attribute otherwise; this port does the same, and the
  fragment case matters — WGSL has no `instance_index` builtin in a fragment
  entry point at all, so the points material's fragment stage reads
  `nodeVarying0`, declared `@interpolate(flat, either)`.
* **The guard, built last.** `4688` workgroups of 64 is 300 032 invocations for
  300 000 particles, so the kernel opens with
  `if instanceIndex >= object.nodeUniformN { return; }`. Three builds that
  condition *after* the body, so its count uniform takes the last
  `nodeUniformN` rather than the first; this port builds it in the same order
  and prepends the line, so the numbering agrees with the dump.
* **Access.** `WGSLNodeBuilder.getNodeAccess()` returns `read_write` only in
  the compute stage. A `BufferSource::Storage` is therefore declared
  `var<storage, read_write>` in the kernels and `var<storage, read>` in the
  points material's vertex and fragment stages — from the same `StorageArray`,
  which is what lets the material read what the kernel wrote. wgpu enforces
  this at bind-group-layout creation, so a mistake here is a panic rather than
  wrong pixels, for once.

The array itself is runtime-sized — `value : array< vec2<f32> >`, with no
element count, inside a one-field struct, exactly as three emits it. The
element count lives in the dispatch and in the guard, not in the type.

### 11.2 Four submits, in three's order

`Renderer::compute()` creates **one** `GPUCommandEncoder`, opens one compute
pass, dispatches, ends it, and submits — one encoder and one `queue.submit()`
per call, which is what `Renderer.compute()` does in three. It is not batched
into the render's encoder, and the `onInit` flow recurses through the same
function, so it gets its own encoder and its own submit *before* the outer
one. A frame of this example is therefore, in order:

1. the `onInit` precompute submit (first frame only),
2. the update submit,
3. the scene render pass,
4. the output pass.

matching the four command encoders in the scout's `dump-r186.json`. The order
is load-bearing and nothing in the image can see it:
`tests/renderer_compute_points.rs::without_the_precompute_nothing_moves` is the
test that can.

### 11.3 What the image cannot grade

The graded frame is 400x250 and black except for a 2x2 block of lit pixels, so
Three's comparator passes it whether the simulation runs or the compute passes
never dispatch at all. The rung's gates are `tests/nodes_compute_wgsl.rs`,
which diffs both generated kernels against `docs/rung12/*.compute.wgsl`
(three.js r186's own output) whole-file, and
`tests/renderer_compute_points.rs`, which reads the storage buffers back and
checks 300 000 particles against the same arithmetic on the CPU. See
`docs/rung12-progress.md`.

### Divergences specific to this rung

`enable subgroups;`, the `@builtin( subgroup_size )` parameter and the
one-binding-not-two storage layout, all three in §8, plus the generated-name
numbering §8 already lists. Statement for statement both kernels match
`docs/rung12/precompute_velocity.compute.wgsl` and
`docs/rung12/update_particles.compute.wgsl`.

## 12. The fat line (`webgpu_lines_fat`)

A fat line is not a line: `LineSegments2 extends Mesh`, and every segment is one
instance of a fixed eight-vertex quad that the vertex shader expands into a
screen-space ribbon. `WebGPUUtils.getPrimitiveTopology()` reads `object.isLine`,
which a `LineSegments2` never sets, so it draws `triangle-list` like any other
mesh. Two halves, in two places, exactly as three.js splits them:

* `src/materials/line2.rs` — `Line2NodeMaterial`, which three.js ships in
  **core** (`src/materials/nodes/`). `mvpLine`, `trimSegmentAlpha`, `alphaLine`.
* `src/addons/lines.rs` — `LineSegmentsGeometry`, `LineGeometry`,
  `LineSegments2`, `Line2`, which three.js ships in `examples/jsm/lines/`.

### 12.1 `setupPosition` and the round trip that must not be simplified

`Line2NodeMaterial` overrides `setupPosition()`, and what it puts back into
`positionLocal` is the *clip-space* result pushed all the way back down:

```js
const localPosition = modelWorldMatrixInverse
    .mul( cameraWorldMatrix ).mul( cameraProjectionMatrixInverse ).mul( mvpLine );
positionLocal.assign( localPosition.xyz.div( localPosition.w ) );
```

The ordinary MVP tail then re-does the transform the material just undid. That
is not an identity in `f32`, and it is what lets the fat line reuse
`modelViewProjection`, `positionView` and everything downstream of them without
a second seam. Three's dump still shows `v_positionView` and
`v_modelViewProjection` computed after it, and so does the port's. Do not
"optimise" the round trip away; the picture would move.

Three new uniforms fall out of it, all ported in the first sitting:
`cameraProjectionMatrixInverse`, `modelWorldMatrixInverse` and
`materialLineWidth`, plus `viewport()` and `screenDPR()`, which `mvpLine` reads
to turn a pixel width into clip space.

`viewport` is the **pass'** rectangle, not the canvas'. The example's 125-high
inset therefore draws the same 5-pixel line four times wider in clip space than
the 500-high main frame does, and the inset looks like a zoom of the main view
even though both cameras sit at the same place. Getting `viewport()` from the
canvas instead would pass the main frame and quietly thin the inset.

### 12.2 `instanceStart` / `instanceEnd`: one buffer, two views

`LineSegmentsGeometry.setPositions()` wraps the array in
`InstancedInterleavedBuffer( array, 6, 1 )` and takes two
`InterleavedBufferAttribute` views at float offsets 0 and 3. That is **one**
vertex buffer of stride 24 with two attributes at byte offsets 0 and 12 — not
two buffers of stride 12 reading alternate halves. Both feed the shader the same
numbers, so the image cannot tell them apart; `tests/nodes_line2_layout.rs`
asserts the layout instead.

The port carries the pairs on the object, not on the geometry:
`SetupContext::line_segments` is an `Option<LineSegmentsAttributes>` holding
`Rc<Vec<f32>>` arrays, hashed by `Rc` pointer, sitting next to the existing
`morph: Option<MorphEntry>`. `crate::nodes::tsl` gained the split that makes the
sharing work — `instanced_data_buffer( data, item_size )` mints the
`Rc<InstanceBuffer>` and `instanced_buffer_attribute( buffer, offset, ty )` takes
a view of it, because `vertex_buffers()` groups by `Rc::as_ptr`, and the old
single call would have minted a fresh buffer per view.

This is option (a) of the plan. Option (b) — real interleaved instanced
attributes on `BufferGeometry`, which is the end state and lets the material
carry no geometry knowledge at all — is a filed follow-up, deliberately not done
on this rung; `docs/webgpu_lines_fat-progress.md` weighs the two.

### 12.3 `if_else_if` nests, and split assignment

Two shapes the dump forced:

* **`ElseIf` is `Else( () => If( … ) )`.** `StackNode.ElseIf()` is sugar for a
  nested `if`/`else`, so the generated WGSL is `if ( a ) { … } else { if ( b ) {
  … } }`, not `else if`. `mvpLine` uses it twice (the near-plane trim and the
  endcaps), and `if_else_if()` in `tsl.rs` generates the nested form.
* **`needsSplitAssign`.** WGSL has no swizzle assignment, so
  `DiffuseColor.rgb.mulAssign( instanceColor )` cannot be written as one
  statement. `AssignNode.generate()` detects the case, emits a temp var for the
  value and then one assignment per component. The port's
  `split_assign_target()` in `src/nodes/builder.rs` does the same. It is a
  shared-path change, so the gate is the usual one: `dump_wgsl` against `main`
  has **0 removals** — every pre-existing material generates the same bytes.

`AssignNode.generate()` also generates the **target** before the value, which is
what puts `nodeVar0 = clipEnd` ahead of `nodeVar1 = clipStart` in the dump; the
port matches it.

### 12.4 `alphaLine`, and `discard` that is not `setupDiscard`

The quad runs `uv.y` from -2 to 2 with the segment between -1 and 1, so
`abs( uv.y ) > 1` is inside one of the two round caps, and the cap is cut to a
circle:

```js
If( uv.y.abs().greaterThan( 1.0 ), () => { … len2.greaterThan( 1.0 ).discard(); } );
```

That is a plain `If( cond ) { Discard }`. The port's `discard_if()` is
`setupDiscard`'s form, `If( cond.not(), … )`, and using it here would emit a
stray `!` and keep exactly the fragments three throws away. `alpha_line()` uses
`if_then( …, vec![ discard() ] )` instead.

With `alphaToCoverage` the same circle is antialiased by `fwidth` rather than
discarded. Three also requires `renderer.currentSamples > 0`; the port folds
that into the material flag, because a material with `alphaToCoverage` on an
unsampled target is not a case any example makes. `dump_wgsl`'s
`line2_alpha_to_coverage` section keeps that branch honest even though the
graded frame does not take it.

### 12.5 Not ported

`_useDash` (the `instanceDistance*` attributes, `lineDistance`, `dashSize` /
`gapSize` and the `mod`-discard) and `_useWorldUnits` (`closestLineToLine` and
the world-space ribbon). Both are off the graded frame — the example sets
`dashed: false` and leaves world units at their default — and both want a node
shape the port does not have yet (a `varyingProperty` assigned in the vertex
stage before it is read). `LineSegments2.computeLineDistances()` and `raycast()`
are not ported either.

### Divergences specific to this rung

All from §8: generated-name numbering (the port's `nodeAttribute0…3` against
three's `instanceStart` / `instanceEnd` / `instanceColorStart` /
`instanceColorEnd`, and `nodeVarying0…3` against three's `nodeVarying4…7`),
uniform-struct member order (the same six render members at 288 bytes and the
same five object members at 160, at different offsets), and the absent `VERTEX_`
sub-build temps. Plus the one that is *not* cosmetic and is listed above: no
`DiffuseColor.w = 1.0`, because `NoBlending` makes `isOpaque()` false.
Statement for statement the two stages match
`scouts/scouts/webgpu_lines_fat/dump/m00_vertex_vertex.wgsl` and
`m01_fragment_fragment.wgsl`.

## 13. PMREM (`webgpu_pmrem_cubemap`, `webgpu_pmrem_test`, `webgpu_furnace_test`, `webgpu_pmrem_scene`)

`PMREMGenerator` (`src/renderer/pmrem.rs`), the read side
(`src/nodes/pmrem_utils.rs`, `src/nodes/pmrem_node.rs`) and `EnvironmentNode`
(`src/materials/environment.rs`), following
`src/renderers/common/extras/PMREMGenerator.js` as of three.js 2f80402
(#34585), which replaced r186's cubeUV atlas with a mipmapped cube, and
b745e6c (#34645), which simplified `roughnessToMip` to
`maxLod * r * ( 2 - r )` on a clamped `r`. The port moved with it in #146 and
the vendor pin moved to 5f610f5; the atlas, its `_sizeLods` planes, the
ping-pong target and `textureCubeUV` are gone.

**The target.** A PMREM is a `HalfFloatType` cube with
`LinearMipmapLinearFilter`, `max( 256, floorPowerOfTwo( size ) )` wide and
`maxLod + 1` levels deep, `maxLod = log2( size ) - 3` — six levels at 256
(`CubeTexture::pmrem_render_target`, `pmrem::allocate_target`). `fromCubemap`
sizes it from the cube's face width (256 for an empty cube),
`fromEquirectangular` from a quarter of the map's width, `fromScene` at 256;
`PmremSource` stands in for the `texture.mapping` test. The source is copied
(`PMREM_cubemap` or `PMREM_equirect`) or captured into a second, full-chain
cube whose mips are then generated.

**The levels.** `_applyPMREM` fills level `lod` with roughness
`lodToRoughness( lod, maxLod ) = 1 - sqrt( 1 - lod / maxLod )`. The last
`INTEGRATION_LEVELS = 3` levels are `PMREM_integration`: a brute-force sum over
three faces of the source's 16² level (`sourceLod = log2( size / 16 )`, an
`int` loop bound of 16). The others are `PMREM_ggx`: `GGX_SAMPLES = 256` VNDF
importance samples, each read at `max( log2( alpha2 * invQ ) + lodBias, 0 )`
with `lodBias = log2( size ) + 0.5 * log2( 6 / ( 256 r^4 ) ) + 0.5`, and a
straight read of level 0 below roughness 0.001. `tests/pmrem.rs` holds the
level ladder (roughness, material, `lodBias` / `sourceLod`) for 256, 512 and
1024 against values printed by node from three's own class, and
`roughnessToMip` at eight points.

**The faces.** `_renderCube` draws a 5-unit `BoxGeometry` with `BackSide`, no
blending and no depth, through a `CubeCamera( 1, 10 )` whose `fov` is −90
(which flips both screen axes) and whose face table is WebGPU's
(`cube_render_target::FACES`, shared with `CubeRenderTarget`). `fromScene`
captures the scene itself through the same table with `near = 0.1`,
`far = 100`, `autoClear` forced on and `scene.background` set to the clear
colour for the capture when the scene has none; the solid-colour
`BackgroundBox` arm of r186 is gone upstream and here. A non-zero `sigma` —
every `RoomEnvironment` page passes 0.04 — blurs source → PMREM level 0 →
source level 0 with `sphericalGaussianBlur` at `min( sigma, PI ) / sqrt( 2 )`
(`BLUR_SAMPLES = 20`, `GOLDEN_ANGLE = 2.399963229728653`) before the source's
mips are generated.

**The read.** `pmremTexture( env, uv, level )` is
`cubeTexture( env ).sample( materialEnvRotation * uv ).level(
roughnessToMip( level, maxLod ) ).rgb` (`PmremHandle::sample`). The r186 read
negated `y`; the cube read negates `x` like every other cube sample. The
background takes the PMREM only at `backgroundBlurriness > 0` or for an
`isPMREMTexture` — an equirectangular background at blurriness 0 goes through
`CubeMapNode`, which is why `webgpu_deferred` now uses
`cube_render_target::from_equirectangular_texture` for its background.

**The shaders.** Every PMREM program is generated by the node system and
matches three's dump of `webgpu_pmrem_test` at 5f610f5 statement for
statement, up to §8's classes and the divergences below: `PMREM_equirect`
(`m01`), `PMREM_ggx` (`m04`), `PMREM_integration` (`m06`), the background
(`m08`) and the lit sphere (`m10`). `dump_wgsl` prints `pmrem_cubemap`,
`pmrem_equirect`, `pmrem_ggx`, `pmrem_integration`, `pmrem_blur` and the
examples' own materials.

**The gates.** `tests/pmrem.rs` (no GPU) is above. `tests/pmrem_scene.rs`
captures a solid-colour scene with sigma 0 and 0.04 and requires every face of
every level to stay that colour within 1% (it is within 0.06% and 0.22%): a
constant convolved with a normalised kernel is the constant, so a lost energy
term shows up there with no BSDF in the way. `tests/pmrem_equirect.rs` holds
`spot1Lux.hdr`'s one bright texel on the flipped row and requires it to light
exactly one face of level 0 and something on every level. Those run on
environments a permuted or rolled face cannot fail, so `assert_face_tiles` in
`tests/e2e/main.rs` reads each layer of `webgpu_pmrem_scene`'s PMREM back:
layer *i* must be `images[ i ]` upright (the −90° `fov` table is exactly the
inverse of the `vec3( -d.x, d.yz )` lookup; the test's doc comment works it
through) and must be the best of all 6 images × 8 dihedral orientations by
about 20× or more, and its centre must be the colour of the sphere that face's
camera looks at.

### Divergences specific to this rung

* **Each face is drawn into a 2-D target and copied into the cube.** Three
  renders straight into a layer and mip of the cube render target
  (`setRenderTarget( target, face, lod )`). The renderer has no layered
  attachment, so `render_faces` draws each face into a half-float
  `RenderTarget` of `size >> lod` and `copy_to_cube_layer` copies it into
  ( layer, mip ). A copy of an equally sized half-float texture is exact.
* **Materials bake their texture; nothing is repointed.** Three keeps one GGX,
  one integration and one blur material and repoints `envMap.value` between
  targets. A texture node here holds its texture, so the GGX and integration
  materials are built against the source target and rebuilt only when it is
  reallocated, and the blur has two materials, one per direction.
* **No dead `reflectVector` lines.** Three's `cubeTexture( envMap )` in the GGX
  and integration fragments starts from its default uv, so `m04` and `m06`
  compute `reflectVector` / `normalView` and bind uniforms that nothing reads.
  The port passes the direction it samples along and emits neither.
* **Two `let`s are asked for by hand.** Three's `d` in `PMREM_integration`
  and the four normalised taps of `PMREM_equirect` come out as `let`s because
  of how its builder caches them; the port's reuse rule would var them, so
  they are `to_const` explicitly.
* **`int` uniforms are written as `i32` bits.** `uniform( 16, 'int' )` is the
  first `int` value uniform on the ladder; it rides `UniformSource::Value`
  like every baked value, and `programs.rs` writes it as an integer next to
  the existing `u32` case. Written as an `f32` it read back as about 10⁹ and
  the loop hung the device.
* **Upstream changes past 2f80402 that are not ported here.** 92b71a5 drops
  `materialEnvRotation` from `CubeTextureNode` (the port's cube background
  still multiplies by it, the identity, so only the WGSL differs), 43feb74
  emits `let` for cached temporaries where the port emits `var`, and 2d3ca24
  (shared dielectric scattering) and 18e109a (the EON diffuse option) reshape
  the lit sphere's `m10` without changing its arithmetic at the defaults.
  None of them moves a pixel on the ladder.
* **Three's `PI` literal, not `f64::consts::PI`.** `PMREMUtils.js` writes
  `3.14159265359`, one digit short of the constant, and the difference is
  visible in the WGSL. Reproduced literally, with an `#[allow]` for clippy.
* **`updateBefore` became `PmremEnvironment::update`.** In three a `PMREMNode`
  carries `NodeUpdateType.RENDER` and builds the PMREM from inside the node,
  during the render. A node here is an immutable `Rc` graph with no
  back-reference to the renderer, and `Renderer` methods take `&mut self`, so
  the trigger moved out: the application calls
  `PmremEnvironment::update( &mut renderer )` before it renders. It is
  idempotent, and since 2f80402 the PMREM cube is allocated up front (its size
  is known from the source), so the `maxLod` uniform the read needs is set
  before anything samples it. A `PmremEnvironment` built from a scene has no
  source and builds at construction, so its `update()` is free.
* **`flipY` is a CPU row reversal, not two render passes.** `HDRLoader` sets
  `texData.flipY = true`, and three's WebGPU backend honours it for a
  buffer-sourced texture with `WebGPUTextureUtils._flipY()`: two extra render
  passes that bounce the source through a scratch texture. `upload_texture_2d`
  reverses the rows in the staging copy instead. A flip is an exact texel
  permutation, so the results are bit-identical; `tests/pmrem_equirect.rs`
  holds it with one lit texel in a black field that has to come back at row
  `512 - 1 - 213 = 298`.
* **`scene.background = <a PMREM texture>` is a `Background::Pmrem` variant.**
  Upstream the background is the PMREM's texture, flagged `isPMREMTexture`,
  which `NodeManager.updateBackground()` turns into `pmremTexture( background )`
  and `Background.update()` wraps in a node context supplying `getUV`
  (`backgroundRotation.mul( normalWorldGeometry )`) and `getTextureLevel`
  (`backgroundBlurriness`). The variant carries a `PmremHandle` — which is
  what the `maxLod` uniform travels on — and the renderer builds the same
  graph with the two accessors passed as arguments
  (`materials::background_pmrem_color_node`).
* **`fromScene` takes the renderer *and* the scene.** `PMREMGenerator` does not
  hold a renderer in this port, and `fromScene` assigns `scene.background` for
  the capture when there is none and puts it back afterwards, so
  `from_scene( &mut renderer, &mut scene, … )` does the same to the same field.
  `tests/pmrem_scene.rs` asserts the restore.

## 14. The previous frame (`webgpu_postprocessing_difference`)

Three builder rules this rung pinned. None of them is a divergence — each one
moves the port onto three.js's behaviour — but each was invisible until a
shader needed it.

* **`dot` builds its operands at the *input* type.**
  `MathNode.generate()`'s generic branch builds every operand at the node's
  input type — the widest of the operands — and only `dot` has a result type
  narrower than that. `luminance( vec4 )` is the case that matters: the `vec3`
  coefficients widen to `vec4<f32>( vec3<f32>( 0.2126, 0.7152, 0.0722 ), 1.0 )`
  and the alpha difference is weighted 1.0 rather than silently dropped.
  `tests/nodes_dot_widening.rs`.
* **An inlined `Fn()` call is transparent to `analyze()`.**
  `ShaderCallNodeInternal.build()` in the analyze stage is
  `outputNode.build( builder, output )` — it neither counts itself nor stops
  the walk — so two call sites sharing one memoised body count *the body*
  twice and it becomes a var. The port used to count the call node and early
  out, which inlined the body once per use. `saturation()` read as a vec3 and
  again for its `.w` is this rung's case; `webgpu_rtt`'s `hue( saturation( … ) )`
  is the pre-existing one, and its quad fragment now hoists
  `max( mix( … ) )` into a var exactly as three does. It is the only
  `dump_wgsl` section this rung moved.
* **A swizzle past the end of its source expands the source.**
  `SplitNode.generate()` builds its input at a type long enough for the
  components asked for (`getVectorLength()`), so `renderOutput()` taking the
  alpha of a `vec3` output node emits `vec4<f32>( nodeVar2, 1.0 ).w`. The port
  emitted `nodeVar2.w`, which is not WGSL.

### Divergences specific to this rung

* **`toggleTexture()` swaps GPU textures, not `Texture` objects.** A `NodeRef`
  is immutable and holds its texture handle for good, so the pair alternates
  behind two fixed handles (`Texture::swap_gpu`) and every bind group,
  pipeline and cache key keyed on them stays valid. See
  `docs/postprocessing.md`, "The previous frame".
* The scene fragment has no divergence left. It used to fold the fog's
  colour, near and far in as constants (the old §8 entry); since `scene.fog`
  (§28) the page's `new THREE.Fog( 0x0487e2, 7, 25 )` is ported as written,
  and the fog statement, uniform names included, is three's r186 `m02` line
  for line: `nodeVar1 = vec4<f32>( mix( Output.xyz, render.nodeUniform4,
  smoothstep( render.nodeUniform5, render.nodeUniform6, ( - v_positionView.z )
  ) ), Output.w );`.

## 15. A context hook on the material output (`webgpu_postprocessing_direct`)

`builder.context.getOutput( materialOutputNode, builder )` is the one seam
`DirectRenderPipeline` uses, and it is inside `NodeMaterial.setup()` — the
function every material in the crate goes through. The port's shape:

* [`OutputContext`](crate::materials::OutputContext) is a field of
  `SetupContext`, so it is part of the program cache key by construction
  rather than by remembering to hash it.
* `MaterialFlow::output_assign` carries the hook's *own*
  `output.assign( materialOutputNode )`, which is why a direct-pipeline
  fragment shader assigns the `Output` property twice in a row. That is
  three's output, not a port artefact.
* The gate for a change on this path is `dump_wgsl`: 244 sections before, 250
  after (the rung's own `direct_scene` / `direct_background`), **zero changed**.
  A material with `output: None` generates the byte-identical shader it
  generated before.

### Divergences specific to this rung

* **The hook is chosen on the renderer, not inside the closure.** three.js's
  closure tests `renderer.isOutputTarget` / `getRenderTarget()` per material;
  the port filters once in `Renderer::render()` — the hook travels only when
  the render goes to the canvas — and passes `output: None` from the shadow
  passes. Same materials end up hooked.
* **The scene fragment's divergences are the existing §8 ones**: named
  lighting temporaries, hoisted accumulator zeros and the inlined
  `faceDirection`, plus uniform renumbering. The tail from the doubled
  `Output =` through `sRGBTransferOETF` is statement for statement three's.
* **The background quad** matches `m00` / `m01` statement for statement; what
  differs is uniform order inside the two structs, the `// codes` order with
  `fn0` / `fn1` swapped, and the order of two `var<private>` declarations —
  all §8 entries already.

## 16. `webgpu_postprocessing_bloom` — the glTF scene under the bloom

This rung adds no node type. `BloomNode`, `mrt()` and the pass plumbing all
landed with `webgpu_postprocessing_bloom_selective`; what is new is the
*input*, a real glTF scene, and it turned up exactly one gap in the node
system and one in the renderer.

**`COLOR_0` at its declared item size.** `vertexColor()` built
`vec4( attribute( 'color', 'vec3' ), 1.0 )` unconditionally. Every one of
`PrimaryIonDrive.glb`'s six primitives carries a four-component `COLOR_0`, and
three.js reads the attribute whole: `m00_vertex_vertex_constant1` declares
`color : vec4<f32>` at `@location( 0 )` and assigns it straight to the varying.
`vertex_color( item_size )` now picks between a widening and a whole-read
accessor, and `SetupContext.vertex_color_size` carries the item size into the
program cache key, because it changes the program's shape. The port's `m00`
signature is three's, attribute for attribute and location for location.

**A full-screen quad is never multisampled.** See `docs/postprocessing.md`;
this is a renderer fact rather than a node one, but it is the whole of this
rung's image.

### Divergences specific to this rung

None new. The two scene materials (`constant1`, `HoloFillDark`) are
`MeshStandardMaterial`s, so their dumps differ from `m00`/`m01`/`m02` in
exactly the classes §8 already lists for `webgpu_lights_physical` and
`webgpu_skinning` — generated names, render-struct member order, builtins
before varyings, named lighting temps, hoisted accumulator zeros, the inlined
`faceDirection` and the `VERTEX_` sub-build. `m04` (the single-texture high
pass) and `m13` (the `RenderPipeline` quad under `ReinhardToneMapping`) match
the selective rung's `bloom_high_pass` and `bloom_render_pipeline_quad`
line for line apart from declaration order.

One thing is missing rather than divergent: **a loaded material has no name.**
`Material.name` is a `&'static str` here, so `GLTFLoader` cannot copy
`materialDef.name` onto it and three's `constant1` / `HoloFillDark` module
names have no counterpart. Nothing generated reads the name — it is not in the
WGSL, the cache key or the bindings — so the dump sections pick the materials
out by the mesh they sit on instead.

## 17. Environment cube maps without the node system (`webgpu_materials_envmaps`, `webgpu_materials_cubemap_mipmaps`)

Two rungs that add no divergence class, and the reason is worth writing down:
neither page reaches the node system at all.

`webgpu_materials_envmaps` is graded on its *first* frame, which is its GUI
defaults — `Type: 'Cube'`, `Refraction: false` — so the frame is a
`CubeTexture` background and one `MeshBasicMaterial` sphere on the
`CubeMapNode` reflection path. `webgpu_materials_cubemap_mipmaps` is two
`MeshBasicMaterial` spheres on that same path. The vertex and fragment modules
three.js dumps for all three of those spheres (`m03` / `m04` of the one,
`m00` / `m01` of the other) are byte-for-byte identical to each other and to
what `examples/dump_wgsl.rs`' `basic_envmap` has printed since rung 3;
`webgpu_materials_envmaps`' background matches `background_cube`. Everything
these rungs added is on the upload side.

**`CubeRefractionMapping` and `EquirectangularReflectionMapping` are not
ported.** `Mapping::CubeRefraction` exists as a constant and nothing reads it:
`setupEnvironment()` would have to pick `refractVector()` over
`reflectVector()` and thread `material.refractionRatio`, and an equirect env
map would need `EquirectUVNode` and a `Texture` rather than a `CubeTexture` in
`material.env_map`. Both are only reachable by moving the page's GUI, which the
graded frame never does.

### A three.js mip-count quirk, reproduced on purpose

`CubeTexture::mip_level_count()` is `mipmaps.len() + 1` when the levels were
supplied by hand. That `+ 1` is not arithmetic the port chose: three.js'
`Textures.getMipLevels()` returns `texture.mipmaps.length` for *every* texture,
which is right for a 2D texture (whose `mipmaps` array holds level 0 too) and
one short for an uncompressed cube (whose `mipmaps` array holds the mips only,
level 0 living in `images`). Rather than fix `getMipLevels()`, three.js
corrects it at the call site:

```js
// TODO: Uniformly handle mipmap definitions
if ( texture.isCubeTexture && texture.mipmaps.length > 0 ) options.levels ++;
```

The port carries the same shape and the same comment, because the count is
observable: `webgpu_materials_cubemap_mipmaps`' 256² cube with eight
hand-authored mips has to be a nine-level GPU texture, which is what three's
own texture descriptor says, and an eight-level one would put the sampler's
`lodMaxClamp` a level short and drift the far side of the sphere.

`needsMipmaps()` comes with it: `generateMipmaps === true || mipmaps.length > 0`,
so a texture with `generateMipmaps = false` and hand-supplied levels is still
mipmapped — and generation is skipped only because
`Textures.updateTexture()` guards it with `texture.mipmaps.length === 0`, not
because `generateMipmaps` is false.

## 18. `webgpu_postprocessing_bloom_emissive` — the emissive attachment

The page draws the scene once into two attachments and blooms only the second,
which makes it the smallest statement of what an MRT is for:

```js
const mrtNode = mrt( { output: output, emissive: vec4( emissive, output.a ) } );
mrtNode.setBlendMode( 'emissive', new THREE.BlendMode( THREE.NormalBlending ) );
scenePass.setMRT( mrtNode );
scenePass.getTexture( 'emissive' ).type = THREE.UnsignedByteType;
```

**`emissive` is the `EmissiveColor` var, not a node of its own.** `m10`'s tail
is `output.m0 = Output; output.m1 = vec4( EmissiveColor, Output.w )`, and the
skybox material — `m04`, drawn into the same two attachments — *declares*
`EmissiveColor` and never assigns it, so attachment 1 gets WGSL's
zero-initialised `vec3` for every background pixel. Nothing special marks the
background; the sky does not bloom because a `MeshBasicNodeMaterial` has no
emissive term to write.

**A colour target per attachment.** `emissiveTexture.type = UnsignedByteType`
makes attachment 1 `rgba8unorm` beside attachment 0's `rgba16float`, and
`setBlendMode` gives it a blend state where attachment 0 has none —
`_getBlending()` reads an MRT attachment's blend mode whatever
`material.transparent` says. `RenderState` grew
`extra_color_targets: [Option<ExtraColorTarget>; 3]`, a format and a blend
state each, and they are part of the pipeline cache key because they are part
of the pipeline.

**`materialAO`.** `MaterialNode.AO` is `tex.r.sub( 1 ).mul( aoMapIntensity )
.add( 1 )` assigned to the `AmbientOcclusion` *property*, written by
`setupAmbientOcclusion()` right after `DiffuseColor`; the `AONode` that
`setupLightsNode()` appends then does `ambientOcclusion.mulAssign(
AmbientOcclusion )`. The `ambientOcclusion` var's `float( 1 )` initialiser is
emitted at its first read, which with an `aoMap` is that `mulAssign` — so
`PhysicalLightingModel::ambient_occlusion()` takes a `has_ao_node` flag and
skips its own declaration rather than emitting a second `= 1.0`.

`aoMap` reads `uv` here where three reads `uv1`: the dumped vertex shader has a
single uv varying and `DamagedHelmet.gltf` has only `TEXCOORD_0`, so the two
are the same attribute. A `TEXCOORD_n, n > 0` asset would need the loader work
the `GLTFLoader` scout note lists as item 6.

### Divergences specific to this rung

**The equirectangular → cube conversion is a copy, not a layered attachment.**
`CubeRenderTarget.fromEquirectangularTexture` renders the `BoxGeometry( 5, 5,
5 )` six times, once per array layer of the cube texture. This renderer has no
layered colour attachment, so `src/renderer/cube_render_target.rs` draws each
face into a plain 2-D render target of the same size and format and then
`copy_texture_to_texture`s it into its layer. Same format, same extent, no
sampling — the texels are three's — but the command buffer is a draw plus a
copy where three's is a draw. `m02`, the conversion's fragment module, matches
three's line for line.

**`scene.environment` has no field yet.** The page's one line becomes a
`PmremEnvironment` and a traversal that puts its handle on each loaded
material. The generated WGSL is the same, because `EnvironmentNode` is reached
through the material either way.

**A loaded material still has no name** (§16): `Material_MR` in three's module
names has no counterpart, which is why `examples/dump_wgsl.rs` builds the
helmet material by hand to diff `m09`/`m10`.

## 19. A uniform that is read per object (`webgpu_instance_uniform`)

Twelve teapots over a `GridHelper`, sharing **one** `MeshBasicNodeMaterial`
whose `colorNode` and `emissiveNode` are built from the same custom node:

```js
class InstanceUniformNode extends THREE.Node {
    constructor() {
        super( 'vec3' );
        this.updateType = THREE.NodeUpdateType.OBJECT;
        this.uniformNode = uniform( new THREE.Color() );
    }
    update( frame ) { this.uniformNode.value.copy( frame.object.color ); }
    setup() { return this.uniformNode; }
}
```

The graph has exactly one uniform node. What makes the twelve teapots twelve
colours is `updateType = OBJECT`: three's `Bindings.updateBindings()` calls
`nodeFrame.updateNode()` for the render object whose object-group buffer it is
about to fill, so `uniformNode.value` is rewritten between the draws and the
program, the pipeline and the bind-group layout are shared.

### What the port has

The object group is already per render object (`UpdateType::Object`), and
`UniformSource::Settable` is already read at the moment the bytes are written
rather than at build time. The only missing piece was *who* gets to write the
value, so `UniformSource::ObjectUpdate` carries the callback and
`UniformContext` carries `frame.object`:

| three.js | three-rs |
|---|---|
| `Node.updateType = NodeUpdateType.OBJECT` + `uniform()` | [`tsl::uniform_object( ty, \|object\| … )`](../src/nodes/tsl.rs) |
| `update( frame ) { … frame.object … }` | the callback, run from `UniformContext::bytes()` |
| `frame.object` | `UniformContext::object`, set from `Renderable::object` |

The callback runs once per draw, inside the same window three runs `update()`
in, and it is identity-compared and identity-hashed exactly as `SettableValue`
is: a value that moves between draws must never reach a cache key, and two
callbacks spelled alike are still two uniforms.

### `mesh.color` has nowhere to live

The page hangs an ad-hoc `.color` on each `Mesh`. `Object3D` is a struct here,
so there is no such property and adding one for a single example would be API
invented for a page rather than ported from three. Instead the callback
receives the `Object3D` and the *application* decides what to answer from —
`examples/webgpu_instance_uniform.rs` keys a `HashMap` on `Object3D.id`. That
is the same graph, the same single uniform and the same twelve values; only the
storage moved from the object to the closure.

### One material, twelve materials

Three's page passes one `Material` object to all twelve `Mesh`es, so
`RenderObjects` builds two programs (the teapot material and the grid) plus the
output pass. A `MeshBasicNodeMaterial` is a **value** here and `Material.clone()`
gets a fresh `MaterialId` (`src/materials/mod.rs`), so each mesh owns its own
material and the node builder runs fourteen times. All twelve teapot runs
generate the same WGSL, so the program cache holds three entries and the
pipeline cache three — the GPU sees what three's does. The divergence is
twelve first-frame `NodeBuilder::build()` calls and nothing at all on a steady
frame; `tests/e2e/main.rs::webgpu_instance_uniform` pins the numbers (14 built,
3 resident, 3 pipelines) so that a future change to material identity shows up
here rather than as a frame-time regression.

### `emissiveNode` on an unlit material

`NodeMaterial.setupLighting()`'s EMISSIVE tail runs for any material with an
`emissiveNode`, lit or not — `MeshBasicMaterial` has no `emissive` colour, so
the node is the only way in. `setup_diffuse_color()` is untouched; the addition
is two statements at the end of the unlit branch of
`materials::node_material::setup_inner()`:

```wgsl
EmissiveColor = ( vec4<f32>( object.nodeUniform0, 1.0 ) * nodeVar1 ).xyz;
nodeVar2 = max( vec4<f32>( ( DiffuseColor.xyz + EmissiveColor ), DiffuseColor.w ), vec4<f32>( 0.0 ) );
```

### `cubeTexture( map )` with no uv

`CubeTextureNode.getDefaultUV()` is `reflectVector` for a
`CubeReflectionMapping`, and `setupUV()` then multiplies by `materialEnvRotation`
(a `mat4`, the identity unless the material or the scene rotates its
environment) and negates `x` for the WebGPU coordinate system. That is exactly
what `MeshBasicNodeMaterial.envMap` already builds in
`setup_inner()`, so the example composes the existing
`material_env_rotation().mul( vec4( reflect_vector(), 1.0 ) )` and
`tsl::cube_texture()` rather than adding a third spelling of it.

### Divergences specific to this rung

None new. The port's `m01`/`m02` differ from three's in the classes §8 already
lists: generated uniform and var numbering, render-struct member order
(`cameraWorldMatrix` first here, last there), and the absent `VERTEX_` sub-build
temps. The grid's two modules are byte-identical to `webgpu_materials`' `m13` /
`m14`, which §8 already covers.

## 20. Hand-written WGSL beside TSL (`webgpu_tsl_interoperability`)

The page draws the same CRT shader twice: once out of two `wgslFn()` blocks and
once out of TSL nodes. The WGSL half is the interesting one, because the node
system never looks inside it — `WGSLNodeFunction` parses far enough to learn the
name, the parameters and the return type and copies the rest through verbatim.
Everything the port had to add is about the shader *around* that body.

| Three | Port | |
|---|---|---|
| `varyingProperty( 'vec2', 'vUv' )` | `tsl::varying_property( "vUv", Type::Vec2, false )` | already there, from `webgpu_mesh_batch` |
| `wgslFn( source, [ vUv ] )` | `wgsl_fn( source, vec![ v_uv ] )` | `CodeDef.includes` is a node list now |
| a nested `wgslFn` in `includes` | `tsl::code( &def )` — `Node::Code` | the same list, the other kind of member |
| `sampler( map )` | the texture node in the `sampler` parameter | already there, from `webgpu_materials` |
| `material.fragmentNode` returning a `vec3` | widened at the entry point | new |
| `renderer.outputColorSpace = LinearSRGBColorSpace` | `Renderer::set_output_color_space` | new |

### `includes` is a list of nodes, not of functions

Three's `CodeNode.includes` holds nodes and `generate()` builds each of them
before its own code. Until this rung the port had it as a list of other
`wgslFn`s, which is what the one case on the ladder — `webgpu_materials`'
`someFn` calling `desaturate` — needs, and the builder emitted them recursively.

That is not what this page uses it for. `crtVertex` assigns to a varying:

```wgsl
fn crtVertex( position: vec3f, uv: vec2f ) -> vec3<f32> {
	varyings.vUv = uv;
	return position;
}
```

Nothing in the graph reads `vUv` in the vertex stage — the assignment is inside
a string — so without the include the varying is never declared and
`VaryingsStruct` has no member for the body to write. Passing the
`varyingProperty` node in `includes` is how the page says so, and building it is
what declares it. `CodeDef.includes` is therefore `Vec<NodeRef>`, a nested
`wgslFn` reaches it through `tsl::code()`, and `emit_code_fn()` *generates* each
include rather than recursing on it. `tests/nodes_wgsl_varying.rs` pins both
halves, including that dropping the include drops the declaration.

### The entry point writes a `vec4`

`fragmentNode` replaces the whole fragment flow: no `DiffuseColor`, no
`Output` property, just `output.color = <the node>`. `crtFragment` returns a
`vec3`, and three builds the output node with `vec4` as its output type, so its
dump ends

```wgsl
output.color = vec4<f32>( crtFragment( vUv, nodeUniform0, … ), 1.0 );
```

The port was assigning the node's own snippet, which for any `fragmentNode`
narrower than a `vec4` is WGSL that does not compile. It formats to `vec4` now,
which is a no-op for every other material on the ladder.

### A join converts its components

`JoinNode.generate()` runs each input through `format()` when the input's
*primitive* type is not the join's, which is how

```js
vec3( ind.equal( 0.0 ), ind.equal( 1.0 ), ind.equal( 2.0 ) )
```

becomes `vec3<f32>( f32( ( nodeVar7 == 0.0 ) ), … )`. The port's
`wgsl::convert()` deliberately leaves the equal-length arm out (§8: reaching it
from `Node::Op` would put an `f32()` around every comparison), so the conversion
lives in the join's own generation, where three has it. It changes no other
shader on the ladder — nothing else joins components of mixed primitive type.

### `outputColorSpace` and the canvas

`Renderer.needsFrameBufferTarget` is `isOutputTarget && ( toneMapping !==
NoToneMapping || outputColorSpace !== workingColorSpace )`. The port had no
`outputColorSpace`, so the second term was always true and the predicate was
`neutral_output` alone. This page sets the output space *to* the working space,
which makes it false: the scene is drawn straight into the canvas and there is
no colour-transform pass behind it — three's dump for the page has three render
pipelines, one of which is the mipmap chain, and no output quad.

That uncovered a real bug in the port, and not in the node system:
`Renderer::read_canvas_pixels()` opened with `prepare_canvas( false, 1 )`, and
`prepare_canvas()` rebuilds the canvas whenever the sample count differs from
the one it has. Every page until now ends its frame with the single-sampled
output quad, so asking for a single-sampled canvas was a no-op. Here the last
pass is the scene itself, multisampled, and the readback was *discarding* the
frame it was about to read — a black PNG out of a render that had worked. It
only prepares a canvas now when there is not one already.

### Divergences specific to this rung

* Three's `uv()` is `attribute( 'uv' )`; the port's is that attribute already
  routed through a varying, which is the fragment stage's reading of it. The TSL
  half assigns `vUv` in the *vertex* stage, so the example writes
  `attribute( "uv", Type::Vec2 )` — three's own definition — rather than
  `tsl::uv()`.
* The usual §8 classes: generated uniform and var numbering (three's
  `nodeUniform13`/`14` for the model matrix against the port's `11`/`12`) and the
  absent `VERTEX_` sub-build temps. Everything else in all four modules is
  statement for statement three's, including the whole of both copied bodies and
  the eighteen-statement TSL fragment.

## 21. `webgpu_pmrem_equirectangular` — the UltraHDR loader

The node graph in this example is §13's. `scene.backgroundNode = pmremTexture(
map, normalWorldGeometry, uniform( 0.5 ) )` generates the module that
`examples/dump_wgsl.rs`'s `pmrem_background` prints, and it matches three's
`m05`/`m06` line for line once the uniform numbering is normalised. The grid's
`MeshPhysicalNodeMaterial` generates `m07`/`m08`, whose only differences from
the port are the multi-scattering and `NORMAL_normalView` entries §8 already
records from earlier rungs. Nothing shader-side is new.

What is new is the decode. `royal_esplanade_2k.hdr.jpg` is an UltraHDR file: a
baseline sRGB JPEG with a second JPEG (the *gain map*) appended after it, an
MPF APP2 index giving the second image's offset, and per-image XMP describing
how to recombine them. `UltraHdrLoader` ports
`examples/jsm/loaders/UltraHDRLoader.js`, and the divergences below are the
ones that could move a pixel.

**`SRGB_TO_LINEAR` truncates its argument.** Upstream builds a 1024-entry table
and indexes it with `value | 0`, so an input of 512.9 reads entry 512 rather
than interpolating. That is not a rounding convenience, it is what produced the
reference image, so `srgb_to_linear` reproduces it:

```rust
if value < 10.31475 { return value * 0.000303527; }
if value < 1024.0 { return srgb_to_linear_table()[value as usize]; }
```

and `srgb_to_linear_truncates_in_the_table_range` pins it.

**The JPEG decoder is zune-jpeg, not libjpeg-turbo.** The browser decodes both
JPEGs through Chromium's decoder; the port uses `decode_jpeg_bytes` from
`texture_loader.rs`. The two IDCTs disagree by at most one unit in the last
place on some blocks. That is the whole of this example's diff: **1 pixel of
100000**, on a specular highlight where a one-unit difference in the gain map
crosses a rounding boundary after the half-float conversion. It is not fixable
without shipping a second JPEG decoder, and it is a decoder difference, not a
port bug.

**The ICC profile is ignored.** Upstream's own feature list says "ICC profile
(not implemented)" and every UltraHDR asset in `three.js/examples` carries a
plain sRGB profile, so Chromium's canvas conversion is the identity. The port
skips the APP2 `ICC_PROFILE` segment rather than parsing one it would then not
apply. An asset with a wide-gamut profile would decode wrong in both.

**`resize_bilinear` is dead code against the in-tree assets.** `applyGainMap`
draws the gain map onto a canvas sized to the SDR image, which rescales it when
the two differ. Both SOF0s in this asset are 2048x1024, so the draw is a 1:1
copy. The resize is kept because the format permits a half- or quarter-scale
gain map and a future asset will use one; it is untested against a reference
until such an asset lands.

**There is no placeholder texture.** `UltraHDRLoader` is a `Loader`, not a
`DataTextureLoader`: it returns a 0x1 `DataTexture` immediately and fills it in
the fetch callback. The port's `load()` is synchronous and returns the finished
texture, so the frame where three has a 0x1 environment does not exist here.
The graded frame is after the callback either way.

**`generateMipmaps` is set and never read.** Upstream's texture asks for
mipmaps and three's dump duly contains twelve mipmap passes, but
`_getEquirectMaterial` samples at an explicit level 0, so no pixel depends on
them. The port sets the flag anyway so the pass structure keeps matching the
dump.

The metadata parser handles both containers the format allows — the legacy
Adobe `hdrgm:` XMP attributes and the binary ISO 21496-1 APP2 block, including
its common-denominator and per-value-denominator encodings — and has unit tests
for each, because eight further examples (`webgpu_loader_gltf`, `_anisotropy`,
`_sheen`, `webgpu_mrt`, `webgpu_materials_transmission`, `webgpu_deferred`,
`webgpu_performance`, `webgpu_custom_fog_background`) load UltraHDR files and
will exercise paths this one does not.

## 22. The room, the RTT and the aberration (`webgpu_postprocessing_ca`)

`webgpu_postprocessing_ca` is the cheapest example that exercises
`RoomEnvironment` as a capability: the page's whole lighting is
`scene.environment = pmremGenerator.fromScene( new RoomEnvironment(), 0.04 )`,
so every reflective shape in the graded frame is a readout of the PMREM cube.
Three programs come from the room itself (`room_box`, `room_boxes`,
`room_panel` in `dump_wgsl`), two from the post chain (`ca_rtt_quad`,
`ca_render_pipeline_quad`).

`chromaticAberration()` calls `convertToTexture()` on the `renderOutput( pass )`
it is handed, which is an `RTTNode`: the effect samples its input at four
different uvs, so the input must be a texture and not a graph evaluated four
times. The page then sets `renderPipeline.outputColorTransform = false`,
because the output transform is already inside the RTT pass.

The green channel of the effect is sampled at `greenScale = 1.0` with zero
offset, so the green channel of the graded image is exactly the scene render —
a free way to separate "the aberration is wrong" from "what it samples is
wrong". It was what said the residual lived in the environment, not in the
effect.

### Divergences specific to this rung

* **The three room materials** (`m02`..`m07`) match three's statement for
  statement; what differs is the `// varyings` order, `var` hoisting and the
  duplicated zero-initialisations — all §8 entries already. None of the three
  has a `uv` attribute: `geometry.deleteAttribute( 'uv' )`.
* **`ca_rtt_quad`'s `main` is byte-identical to `m17`**; only the order the
  three helper functions are emitted in differs (`fn0` / `sRGBTransferOETF` /
  `fn1` here, `fn1` / `sRGBTransferOETF` / `fn0` in the dump) — the `// codes`
  ordering entry of §8.
* **`ChromaticAberrationShader` hoists two subexpressions three inlines.**
  `( scale * 0.02 ) * strength` and `offset * aberration` each become a
  `nodeVar` here and are repeated three times in the dump, and each
  `textureSample` gets a second `nodeVar` for the `.toVar()` the addon puts on
  its result where three folds the two together. Twelve vars against six; the
  returned `vec4` is the same four swizzles of the same four samples.
* **The body arrives as function parameters, not uniforms.** Three declares it
  with `setLayout( { name: 'ChromaticAberrationShader', … } )`, so `strength`,
  `center` and `scale` are WGSL parameters and the texture is the only thing
  the body closes over. The port does the same.

## 23. `webgpu_loader_gltf` and `webgpu_mrt` — the scene environment, and MRT as a pass property

Two examples, one scene: an UltraHDR equirectangular map as both
`scene.background` (converted to a 512² cube, drawn as the skybox) and
`scene.environment` (PMREM-filtered), with DamagedHelmet in front of it.
`webgpu_loader_gltf` renders it to the canvas; `webgpu_mrt` renders it once into
four colour attachments and composites the four side by side.

### 23.1 `scene.environment` is a field on the scene, not on the material

`NodeMaterial.setupEnvironment()` reads the material's `envNode` first and falls
back to `builder.context.environment`, which the renderer fills from
`scene.environmentNode`. Up to this rung every graded example put the PMREM
handle on the material by hand
(`MeshBasicNodeMaterial::pmrem_env`), because `webgpu_pmrem_*` and
`webgpu_postprocessing_bloom_emissive` build their own materials. The helmet's
`Material_MR` comes out of `GLTFLoader` and carries no `envMap`, so the fallback
is the only path to it.

[`Scene::environment`] is the field, and `SetupContext::environment` carries it
into the build:

```rust
let env = material.pmrem_env.as_ref().or(ctx.environment.as_ref());
```

Because it sits in `SetupContext`, which is the render object's *dynamic* cache
key, `PmremHandle` grew a `Hash` keyed on the PMREM cube's id alone — the
`maxLod` beside it is a uniform and changes no code.

`dump_wgsl`'s `loader_gltf_helmet` section exists to prove the two paths are the
same program: its fragment shader is byte-identical to
`bloom_emissive_helmet`'s but for the MRT tail, although one got the handle from
the material and the other from the scene.

**Dedupe note for the integrator.** The `rung-room-environment` branch adds
`Scene::environment` with the same shape and the same doc comment, and the same
`impl Hash for PmremHandle`. They are meant to collapse to one; take either
side.

`scene.backgroundBlurriness` and `backgroundIntensity` stay at their GUI
defaults of 0 and 1 in the graded frame, so the skybox is a sharp cube read and
not a PMREM one. The port has no knob for either yet — see §23.5.

### 23.2 An MRT output can depend on the material, so it cannot be an eager node

`mrt( { normal: packNormalToRGB( normalView ) } )` is set on the *pass*, once,
in the page's `init()`. Upstream that is harmless: `normalView` is a node
object, and a node object's `setup( builder )` runs once per material, so the
helmet's `normalView` is its normal map's and the skybox's is
`normalViewGeometry * - 1` because `Background.material` is `BackSide`.

This port's TSL is eager. `normal_view()` reads the thread-locals
`with_material_normal()` installs — the material's normal node, its
`flatShading` and its `side` — at the moment it is *called*, and an example's
`init()` is outside any material setup, so it would freeze the defaults
(no normal map, `FrontSide`) into every draw. The symptom was exact: the skybox
filled the `normal` band with `1 - reference` everywhere, the missing
`negateOnBackSide()`.

[`MrtValue::Deferred`] is the shim. `MrtNode::set_deferred( name, closure )`
stores a closure instead of a node, and `MRTNode.setup()`'s port
(`MrtNode::members`) calls it — which happens inside
`NodeMaterial.setup()`, where the material state is installed:

```rust
scene_mrt.set_deferred("normal", || pack_normal_to_rgb(normal_view()));
```

`MrtNode`'s `Hash` takes the closure's `Rc` address, not its result: resolving
it to hash it would build nodes outside any material's setup, the very thing it
exists to avoid.

This is a **deliberate divergence in API shape, not in generated code**. The
`mrt_background` and `mrt_helmet` sections of `dump_wgsl` match
`dump-mrt/m04` and `dump-mrt/m10` statement for statement, including
`normalView = ( normalViewGeometry * vec3<f32>( -1.0 ) )` in the skybox and the
normal-mapped `normalView` in the helmet.

Only `normal` needs it on this ladder; `output`, `diffuseColor` and
`emissive` are property reads, resolved by name at codegen, and stay plain
nodes.

### 23.3 A `vec3` MRT member is written as `vec4( value, 1.0 )`

`normal` and `emissive` are `vec3`s. `OutputStructNode` converts each member to
the attachment's `vec4` by appending 1.0 — *not* the fragment's alpha:

```
output.m1 = vec4<f32>( ( ( normalView * vec3<f32>( 0.5 ) ) + vec3<f32>( 0.5 ) ), 1.0 );
output.m3 = vec4<f32>( EmissiveColor, 1.0 );
```

`webgpu_postprocessing_bloom_emissive` looks like a counter-example and is not:
its page writes `vec4( emissive, output.a )` out by hand, so the `vec4` is in
the graph and the conversion has nothing to do.

### 23.4 The skybox is an ordinary draw, so it writes every attachment

`NodeMaterial.setup()`'s MRT branch runs for `Background.material` like any
other material, because the MRT is a property of the pass. The renderer used to
push the background `Renderable` with a default `SetupContext` — `mrt: None` —
and `webgpu_postprocessing_bloom_emissive` never noticed, because its sky wrote
`EmissiveColor` (zero) to attachment 1, which is the clear value. `webgpu_mrt`
notices immediately: the environment is the whole picture of the `normal` and
`diffuse` bands. The background now carries the pass's `mrt_context`, which is
computed before the render list rather than after it.

### 23.5 Divergences and gaps

| item | status |
| --- | --- |
| `mrt( { normal: packNormalToRGB( normalView ) } )` | `set_deferred` with a closure; generated code identical (§23.2) |
| `NORMAL_normalView` sub-build temp, inlined `nodeVarN` temps | pre-existing, §7 and §8 |
| `scene.backgroundBlurriness` / `backgroundIntensity` | not ported; both are at their defaults in every graded frame |
| `requiredLimits: { maxColorAttachments: 5 }` | a WebGPU device request; wgpu's default adapter limit is 8 |
| `renderer.compileAsync()` | not ported; it only warms the pipeline cache and this port compiles on first draw |
| `MRTNode.setBlendMode` per attachment | ported, but `NoBlending` and `NormalBlending` agree bit-for-bit on an opaque draw — `docs/postprocessing.md` |

### 23.6 `isUnfilterable()`: `NearestFilter` on both sides removes the sampler

`pass( scene, camera, { minFilter: NearestFilter, magFilter: NearestFilter } )`
is the other half of `webgpu_mrt`, and it changes the *generated code* of the
composite rather than any pixel of the scene.

`WGSLNodeBuilder.isUnfilterable( texture )` is true when
`minFilter === NearestFilter && magFilter === NearestFilter`. Such a texture is
bound with a `non-filtering` sample type and **no sampler at all**, and every
tap on it becomes a `textureLoad` against `textureDimensions` instead of a
`textureSample`. Three's `dump-mrt/m12` has four bare `texture_2d<f32>`
bindings and not one `_sampler`.

The port's three pieces:

* `Texture::is_unfilterable()` — the predicate.
* `tsl::texture_uv()` picks `SampleMode::Load` over `SampleMode::Sample` for
  one.
* `NodeBuilder`'s texture slots bind it as `TextureKind::FloatData2D`, which is
  the `non-filtering` sample type with no companion sampler.

`PassNode::new_with_options( PassOptions { min_filter, mag_filter } )` is how the
filters reach the pass's attachments; `PassNode::new()` delegates to it with
`Linear` / `Linear`.

### 23.7 MSAA and MRT meet for the first time

Every colour attachment of a WebGPU render pass must share a sample count.
`webgpu_mrt` is `antialias: true` *and* four attachments, which the renderer had
never seen: the comment where the two paths crossed said outright that MSAA and
MRT never meet on this ladder. wgpu says so as a validation error rather than
wrong pixels —

> Attachments have differing sample counts: the color attachment at index 0's
> texture view has count 4 but is followed by the color attachment at index 1's
> texture view which has count 1

`RenderTargetInner` grew `msaa_extra: Vec<wgpu::Texture>`, one multisampled
texture per extra attachment at that attachment's own format (the three LDR ones
are `rgba8unorm` where attachment 0 is `rgba16float`), and `PassTarget::extra_colors`
became `Vec<(TextureView, Option<TextureView>)>` — the view drawn into and the
resolve target — so each extra attachment resolves like attachment 0 always has.

## 24. `webgpu_custom_fog_background` — a pass's depth as a value

The scene is §23's, minus `scene.background` and minus the renderer's tone
mapping. Everything new is in the `RenderPipeline` quad:

```js
const scenePass     = pass( scene, camera );
const scenePassViewZ = scenePass.getViewZNode();
const fogFactor     = rangeFogFactor( 2.7, 4 ).context( { getViewZ: () => scenePassViewZ } );
const compose       = fogFactor.mix( scenePass.toneMapping( ACESFilmicToneMapping, 1 ), color( 0x4080cc ) );
```

Three's `m08_fragment_fragment_RenderPipeline.wgsl` is 40 lines of flow, and
the port reproduces all of it; the four pieces it needed are below.

### 24.1 `pass.getViewZNode()` — the depth attachment as a bound texture

`PassNode.getViewZNode( name = 'depth' )` is
`perspectiveDepthToViewZ( getTextureNode( name ), cameraNear, cameraFar )`, and
with `reversedDepthBuffer` off that is

```
near * far / ( ( far - near ) * depth - far )
```

([`tsl::perspective_depth_to_view_z`]). `getTexture( 'depth' )` is the pass's
own `DepthTexture` — the constructor seeds `_textures[ 'depth' ]` with it — so
the composite quad binds the very attachment the scene pass wrote, with no copy
and no resolve. [`PassNode::view_z_node`] memoises the graph the way
`_viewZNodes` does, and [`PassNode::depth_texture`] is `getTexture( 'depth' )`.

`cameraNear` / `cameraFar` are `uniform( 0 )`s the pass owns and writes from its
camera in `updateBefore()`. They are **object**-group uniforms of whatever
material samples the pass, which is why the composite reads
`object.nodeUniform1` / `object.nodeUniform2` and not the camera block. The port
holds them as [`uniform_settable`] pairs and writes them at the top of
`PassNode::render`, exactly where three does.

### 24.2 `texture_depth_multisampled_2d`

The page is `antialias: true`, so `PassNode.setup()`'s
`renderTarget.samples = renderer.samples` makes the pass target 4×MSAA. WebGPU
resolves colour attachments and **never** depth ones, so the depth texture the
composite binds is still multisampled. Three's dump declares it

```wgsl
@binding( 3 ) @group( 0 ) var nodeUniform3 : texture_depth_multisampled_2d;
```

and reads sample 0 of the fragment:

```wgsl
nodeVar3 = textureDimensions( nodeUniform3 );
nodeVar2 = textureLoad( nodeUniform3, vec2<u32>( … ), u32( 0 ) );
```

Three pieces of the port carry that:

* [`TextureKind::DepthMultisampled2D`] — the declared WGSL type, no companion
  sampler, and `multisampled: true` on the bind-group layout entry. A
  `multisampled: false` entry against a 4-sample view is a wgpu validation
  error, not wrong pixels.
* `DepthTextureInner::multisample`, three's
  `texture.isMultisampleRenderTargetTexture`. `RenderTarget::set_samples` keeps
  it in step with the target's sample count, and `RenderTarget::set_depth_texture`
  seeds it, so nothing outside the render target ever sets it by hand.
  `NodeBuilder`'s texture slots read it to pick the kind.
* `wgsl::texture_dimensions` drops its level argument for this kind. WGSL gives
  a multisampled texture no `textureDimensions( t, level )` overload at all —
  the same `u32( 0 )` every other `textureLoad` carries is a compile error here.
  The sample index inside `textureLoad` stays.

### 24.3 `rangeFogFactor( … ).context( { getViewZ } )` — a divergence in shape

`Fog.js`' `getViewZNode( builder )` reads `builder.context.getViewZ` and falls
back to `positionView.z`, then negates whichever it got. Upstream the choice is
made **while the material is built**, because `rangeFogFactor` is an `Fn()` and
its body runs against the builder; `.context( … )` is a `ContextNode` wrapped
round it.

This port's TSL is eager — the same reason §23.2 needed `MrtValue::Deferred` —
so there is no builder in scope when `rangeFogFactor( 2.7, 4 )` is written, and
no context to read. The port makes the choice by which function the caller
calls:

* [`tsl::range_fog_factor`] — `positionView.z`, what `scene.fogNode` uses;
* [`tsl::range_fog_factor_with_view_z`] — the explicit override, what this page
  uses.

The generated WGSL is identical either way; this is an **API-shape divergence,
not a code one**. A general `ContextNode` would be the upstream shape, and it is
what a page that overrode `getViewZ` for a *material* would need; nothing on the
ladder does, and §8's rule is to add only what a rung needs.

### 24.4 `.toneMapping( mode, exposure )` on a node, and `outputColorTransform`

`ToneMappingNode` is `vec4( toneMappingFn( color.rgb, exposure ), color.a )`,
with `NoToneMapping` returning the colour untouched. The port had the four
functions already, but only inside `render_output()` and only ever with the
renderer's `toneMappingExposure` uniform. [`materials::tone_mapping_node`] is
that step lifted out and given an explicit exposure node; `render_output()` now
calls it. The page passes the literal `1`, so the shader reads
`acesFilmicToneMapping( nodeVar1.xyz, 1.0 )` with no uniform at all.

`renderPipeline.outputColorTransform = true` is the **default** and the port
already had it (`webgpu_postprocessing_ca` is the one that sets it to `false`).
What makes it interesting here is that it is paired with
`renderer.toneMapping = NoToneMapping`: `renderOutput()` around the composite
comes out as the alpha clamp, the unpremultiply, the sRGB OETF and the
premultiply back, and nothing else. The tone map that *is* applied is the one
inside the composite, on the scene pass alone — the fog colour is never tone
mapped.

### 24.5 Why the empty pixels are fog

The page sets no `scene.background`. The pass target is cleared to
`( 0, 0, 0, 0 )` at depth 1.0, so outside the helmet
`perspectiveDepthToViewZ( 1, 0.25, 20 )` is exactly `-far`, `-20`; negated that
is 20, and `smoothstep( 2.7, 4, 20 )` is 1. The composite is then the fog colour
alone, at alpha 1 — which is why the background of the graded frame is a flat
`0x4080cc` and not the clear colour. Get the near/far uniforms, the sample index
or the multisample flag wrong and it is the whole image that moves, not an edge.

### 24.6 Divergences

None new. The composite quad differs from three's `m08` only in the classes §8
already lists: generated var numbering (three numbers the `textureLoad` result
before the `textureDimensions` temp, the port the other way round), the order
the `// codes` helpers are emitted in, and the splat form
`vec3<f32>( 0.0, 0.0, 0.0 )` where three prints `vec3<f32>( 0.0 )` inside
`acesFilmicToneMapping`. The scene program is §23's `Material_MR`, unchanged.

## 25. `webgpu_loader_gltf_sheen` — the sheen lobe, and a glTF asset's uv transforms

§23's environment with SheenChair in front of it. The page adds **no lights**
(its one `DirectionalLight` is commented out upstream), so every term in the
frame is indirect and the sheen shows as a rim rather than a highlight. Three
things are new, and they are independent of each other: the sheen half of
`PhysicalLightingModel`, `KHR_texture_transform`, and a second uv set.

### 25.1 `useSheen`, and the two properties it turns on

`MeshPhysicalNodeMaterial` sets `useSheen = this.sheen > 0`, so *the float*
decides whether the lobe is built at all. The material carries both halves of
three's shape — `Material::sheen` (a `f64`, 0 by default) and
`Material::sheen_color` (a `Color`, black) — and `setup_standard()` gates on the
first:

```rust
let use_sheen = material.kind == MaterialKind::Physical && material.sheen > 0.0;
if use_sheen {
    fragment.push(sheen().assign(material_sheen_color().mul(material_sheen())));
    fragment.push(sheen_roughness().assign(material_sheen_roughness().clamp(0.0001, 1.0)));
}
```

which is `MaterialNode.SHEEN` (`sheenColor.mul( sheen )`) and
`MaterialNode.SHEEN_ROUGHNESS` (`sheenRoughness.clamp( 0.0001, 1.0 )`), in
three's order, between `DiffuseContribution` and `EmissiveColor`. The multiply
stays in the shader rather than being folded into the uniform, because the GUI
slider the page adds writes `material.sheen` and nothing else — three's
`sheenColor` uniform keeps the asset's value.

The lower clamp is not cosmetic: `D_Charlie`'s `invAlpha` is `1 / r²`, so a
sheen roughness of 0 is a division by zero.

### 25.2 The lobe: `D_Charlie`, `V_Neubelt`, `IBLSheenBRDF`

`src/materials/physical.rs` ports all four functions from `BRDF_Sheen.js` /
`PhysicalLightingModel.js`. The split between what becomes a WGSL `fn` and what
is inlined is three's, and it is decided by `setLayout`:

| three | here | shape |
| --- | --- | --- |
| `D_Charlie` (`setLayout`) | `d_charlie()` | a real `fn D_Charlie( roughness : f32, dotNH : f32 ) -> f32` |
| `V_Neubelt` (`setLayout`) | `v_neubelt()` | a real `fn V_Neubelt( dotNV : f32, dotNL : f32 ) -> f32` |
| `BRDF_Sheen` (plain `Fn`) | `brdf_sheen()` | inlined — `sheen * D * V` |
| `IBLSheenBRDF` (plain `Fn`) | `ibl_sheen_brdf()` | inlined, three times |

`IBLSheenBRDF` is the analytic fit of the Charlie BRDF integrated over the
hemisphere, and its three readers — the `irradiance` sheen term, the
`iblIrradiance` one and the energy compensation — each rebuild it in full. The
port produces the same three copies for a different reason (three separate
`NodeRef`s, and the builder caches by `Rc` identity); §8 records it.

`brdf_sheen` — the *direct* lobe — is unreachable on this page, because there
are no lights. It is ported anyway, and the fact that no dump pins it is stated
in its doc comment: the first sheen page that does light something would
otherwise find a lighting model quietly missing half of itself.

### 25.3 Where the sheen terms sit in the flow

`Physical` gained a `sheen: bool` and five insertion points, one per hook in
three's model. With `sheen` false every one of them is skipped and the emitted
WGSL is byte-identical to what it was before this rung — which is how the nine
green physical-material examples stayed at their exact pixel counts.

| three's hook | what the port pushes |
| --- | --- |
| `start()` | `sheenSpecularDirect = vec3( 0 ); sheenSpecularIndirect = vec3( 0 );` |
| `direct()` | `sheenSpecularDirect += irradiance * BRDF_Sheen(…)`, then `irradiance *= sheenEnergyComp( max( albedoV, albedoL ) )` |
| `indirectDiffuse()` | `sheenSpecularIndirect += irradiance * Sheen * albedo / π`, then `diffuse *= sheenEnergyComp( albedo )` |
| `indirectSpecular()` | `sheenSpecularIndirect += iblIrradiance * Sheen * albedo / π` **first**, then one shared `sheenEnergyComp` multiplying *both* accumulators |
| `ambientOcclusion()` | `sheenSpecularIndirect *= ambientOcclusion`, before `indirectDiffuse` |
| `finish()` | `outgoingLight = ( outgoingLight + sheenSpecularDirect ) + sheenSpecularIndirect` |

Two orderings in that table are load-bearing and were taken from the dump, not
from reading the JS:

* **`indirectSpecular()` pushes the `iblIrradiance` sheen term before the
  multiscattering block**, because three's `indirectSpecular()` starts with it.
  Push it after and `sheenSpecularIndirect` is still right, but every
  `nodeVarN` moves and the reader loses the correspondence.
* **The energy compensation there is one value, not two.** Three builds
  `sheenAlbedo` and `sheenEnergyComp` once and calls `mulAssign` on
  `indirectSpecular` and then `indirectDiffuse`; both must be `toVar`s for that
  to be expressible, so the port wraps both in `to_var()` in this branch only.

`direct()` has the mirror of that last point: `irradiance` is a `.toVar()` in
three unconditionally, and the port promotes it **only when sheen is on**, so
that the lit examples already on the ladder keep their inlined form.

### 25.4 `KHR_texture_transform`, and why one case needs the matrix written

Every map in SheenChair carries a transform: the fabric's base colour is tiled
seven times (`offset ( -3, 3 ), scale ( 7, 7 )`), its normal map twice, and the
wood's colour and normal maps are also **rotated**.

The port already emits a per-texture uv matrix — `TextureNode.getTransformedUV`
is `matrixUniform * vec3( uv, 1 )` whenever the node has no explicit uvNode —
so offset and repeat alone need nothing new in the node system:
`Texture::set_offset` / `set_repeat` feed the same uniform three's do.

The rotation does not, and the reason is a composition order:

* glTF composes the transform **`T * R * S`** (`KHR_texture_transform`).
* three.js' `Texture.updateMatrix()` composes **`T * S * R`** about `center`.

They agree only when the rotation or the scale is the identity. Three's own
`GLTFTextureTransformExtension` handles it by writing `texture.matrix` directly
and setting `matrixAutoUpdate = false`; `assign_texture()` does the same through
the new `Texture::set_matrix`, and nothing in the port recomputes a texture
matrix after load, so the flag has no counterpart here — the written matrix is
simply the last word. The offset and repeat fields are still set, so anything
that reads them back (the debug view, a future `updateMatrix`) sees the asset's
own numbers.

`assign_texture()` also does three's cloning rule, which is easy to get wrong:
a glTF *texture* is shared between materials, but a `texCoord` or a transform
belongs to the *reference*. So the loader clones the `Texture` when the
reference asks for either and leaves the cached original alone — the same
reason three's extension calls `texture.clone()`.

### 25.5 `TEXCOORD_1`

All four occlusion maps are `texCoord: 1`. `Texture` gained a `channel`
(`Texture.channel` in three, 0 or 1 here — the assert names the limit), and
`tsl::texture()` resolves its default uv through it, which is
`TextureNode.getDefaultUV()`'s `uv( this.value.channel )`. The attribute side is
`tsl::uv1()`, an `attribute( 'uv1', vec2 )` through a varying, and it appears in
the fabric's attribute list exactly where three's vertex dump has it:
`uv`, `uv1`, `normal`, `position`.

### 25.6 Promoting a glTF material to physical

`build_material()` now produces a `MeshPhysicalNodeMaterial` when the material
carries `KHR_materials_ior`, `KHR_materials_specular` or `KHR_materials_sheen`,
and a standard one otherwise. That is what three's per-extension
`getMaterialType()` comes to, and the dump pins it: only the fabric is a
physical material here. Its `SpecularColor` comes out of the IOR and the
specular factor —

```wgsl
	SpecularColor = ( min( ( vec3<f32>( ( nodeVar3 * nodeVar3 ) ) * object.nodeUniform12 ), vec3<f32>( 1.0, 1.0, 1.0 ) ) * vec3<f32>( object.nodeUniform13 ) );
	SpecularF90 = mix( object.nodeUniform13, 1.0, Metalness );
```

— where the label, the wood and the metal compile the standard material's
constant, `SpecularColor = vec3<f32>( 0.04, 0.04, 0.04 )`, with `SpecularF90 =
1.0`. Promote all four and three of them change colour.

### 25.7 Divergences

Nothing new beyond the two bullets §8 gained for this rung (the indirect-diffuse
block emitted above the environment's, and `IBLSheenBRDF` inlined three times).
Everything else between the port's fabric fragment and three's
`m10_fragment_fragment_fabric_Mystere_Mango_Velvet.wgsl` is in classes §8
already lists: generated `nodeVarN` numbering, the hoisted accumulator zeros,
"Named lighting temps" (three's `dfg`, `multiScatteringCompensation`,
`singleScatteringDielectric` and friends are `let`s there and inlined or
numbered here), and property-assignment temps. The arithmetic of every sheen
statement — the `-1.9362 / 1.0678 / 0.4573 / 0.8469 / -0.6014 / 0.5538 / 0.467
/ 0.1255` fit, the `1 / π`, the `max( max( Sheen.x, Sheen.y ), Sheen.z )` — is
term for term three's.

## 27. `webgpu_deferred` — a G-buffer, a resolve quad and a shared depth buffer

The page renders four times per frame:

1. **the opaque pass** — the teapot into a three-attachment render target
   (`output` = albedo, `position` = `vec4( positionView, metalness )`,
   `normal` = `vec4( normalView, roughness )`), with the scene's lighting
   switched off and the light layer excluded;
2. **the resolve quad** — a full-screen `MeshStandardNodeMaterial` on layer 2
   whose `positionView`, `positionViewDirection` and `normalView` are
   *overridden* to read the G-buffer, so three's ordinary physical lighting
   flow runs once per pixel against eight point lights and the environment;
3. **the transparent pass** — the six `DoubleSide` planes and the light
   spheres, sharing the opaque pass's depth attachment and not clearing it;
4. **the composite** — `opaque.rgb * ( 1 - transparent.a ) + transparent.rgb`
   through the `RenderPipeline`'s output transform.

Everything new here is a *pass* property or a *material* property that three
sets on the fly; no new node type was needed.

### 27.1 `lighting: false` is not just "no lights"

`PassNode`'s `lighting` option reaches `RenderList.finish()` as
`lightsNode.setLights( this.lighting.enabled ? this.lightsArray : _emptyArray )`,
so the obvious reading is "build the materials with an empty light list". That
is not what three's G-buffer fragment (`m12`) shows: it has no lighting chain at
all, and `outgoingLight` is `DiffuseColor.xyz`. The gate is in
`NodeMaterial.setupLighting()`:

```js
const sceneLighting = this.lights === true && builder.renderer.lighting.enabled;
const materialLightings = sceneLighting ? this.setupMaterialLightings( builder ) : [];
const lightsNode = lights ? ( this.lightsNode || builder.lightsNode ) : null;
if ( lightsNode && ( materialLightings.length > 0 || lightsNode.getScope().hasLights ) ) { … }
```

`setupMaterialLightings()` is where the **environment** lives (with the light
map and the AO node), so a pass with lighting disabled drops `scene.environment`
too — and with no lights and no environment the whole `lightingContext` is
skipped and `setupOutgoingLight()` stands. The teapot's G-buffer program
therefore computes albedo, metalness and roughness and nothing else, which is
the point: a lit colour in the `output` attachment would be lit twice.

The port carries this as [`SetupContext::lighting_disabled`] — inverted so that
`Default` stays "lighting enabled" — set by the renderer from its own
`lighting_enabled` flag, which [`PassNode::set_lighting_enabled`] saves, sets
and restores around the pass's render the way three's `renderer.lighting` is a
renderer-level object. `setup_standard()` reads it for both halves of the gate:
no `materialLightings` (so no environment) and no chain.

This was found by diffing the dump, not by the pixels: with the chain present
the extra statements write to `Output`, which an MRT material never reads, so
the frame was already correct. It is still a real fix — a deferred page that
kept `scene.environment` on the G-buffer pass would bind the PMREM textures and
the BRDF LUT to a program that cannot use them.

### 27.2 `overrideNodes()` — three properties replaced for one sub-build

```js
const resolveMaterial = new THREE.MeshStandardNodeMaterial();
resolveMaterial.overrideNodes( [
  [ positionView, positionAttachment.xyz ],
  [ positionViewDirection, positionAttachment.xyz.negate().normalize() ],
  [ normalView, normalAttachment.xyz ],
] );
```

`OverrideContextNode` swaps the three node singletons for the duration of the
material's build and hands the replacement back **as-is** — no `toVar` — so
every use inlines. Three's `m14` is the proof: `nodeVar3.xyz` appears eleven
times, `normalize( ( - nodeVar3.xyz ) )` nine, and no var holds either.

The port's TSL is eager, so there is no builder context to swap. The overrides
travel on the material as [`MeshBasicNodeMaterial::context_overrides`] (an
[`OverrideNodes`]), and `NodeMaterial::setup()` installs them in a thread-local
for the duration of the build; `normal_view()`, `position_view()` and
`position_view_direction()` return the replacement when one is installed. The
memo key for `normal_view()` was widened so the same material built with and
without an override cannot share a cached node.

The resolve fragment matches three's `m14` statement for statement modulo the
§8 classes — including the `nodeVar4.w` roughness read and the two
`mix( singleScatteringDielectric, … , Metalness )` blends.

### 27.3 `depthNode` — `@builtin( frag_depth )`, written first

```js
resolveMaterial.depthNode = depthAttachment;
resolveMaterial.colorNode = Fn( () => { If( depth.greaterThanEqual( 1.0 ), () => { Discard(); } ); return outputAttachment; } )();
```

`NodeMaterial.setupDepth()` runs *before* `setupDiffuseColor()`, so the depth
write is the first statement in the fragment flow, ahead of the discard that
reads the same value. The port added `MaterialFlow::depth`, analysed with the
fragment stage and generated first, and the fragment output struct gained the
shape

```wgsl
struct OutputStruct {
	@location( 0 ) color: vec4<f32>,
	@builtin( frag_depth ) depth : f32
};
```

which is what `assemble_with_mrt( …, depth: true )` emits. Copying the G-buffer
depth into the resolve quad's fragment depth is what lets the *transparent*
pass, which reuses that same depth attachment, occlude the planes against the
teapot even though the teapot was never drawn into the transparent pass.

### 27.4 One depth texture, two render targets — and `depthInitialized`

```js
const transparentPass = pass( scene, camera, { depthTexture: opaquePass.getTexture( 'depth' ), autoClearDepth: false } );
```

Two `PassNode`s, two render targets, one `DepthTexture`. The port's
[`PassOptions::depth_texture`] hands the texture over and marks the borrower as
not owning it, so `PassNode::render()` resizes through
[`RenderTarget::set_size_keeping_depth`] and does not drop the shared depth
allocation on a resize.

`autoClearDepth: false` then runs into a quirk that is worth naming, because it
is the difference between a plane hiding behind the teapot's spout and not:

> `Renderer._renderScene()` keeps `renderTargetData.depthInitialized`, and the
> **first** time it renders into a target that has a depth buffer with
> `autoClear === false || autoClearDepth === false` it clears the depth anyway,
> once. The flag lives on the *render target*, not on the depth texture — so
> the transparent pass, a second target over the same texture, wipes the depth
> the opaque pass just wrote, on frame one only.

The port reproduces this bit-for-bit ([`RenderTarget::depth_initialized`], and
the gate in `Renderer::render`). It is a three quirk, not a design: on frame two
onwards the depth survives. The graded frame is frame one, and without it 557
pixels differ.

### 27.5 `opaque`, `transparent`, and the two-draw `DoubleSide` split

`renderer.opaque` / `renderer.transparent` gate the two halves of the render
list. Two details the page depends on:

* **`opaque = false` also drops the background.** `_background.update()`
  unshifts the skybox mesh into `renderList.opaque`, so the transparent pass,
  which has `opaque: false`, does not draw a second copy of the environment
  behind the planes. The port gates the background node on the same flag.
* **a transparent `DoubleSide` object is drawn twice, back then front, per
  object.** This is `Renderer._renderObjectDirect()` — `material.side =
  BackSide`, draw, `material.side = FrontSide`, draw, restore — not
  `RenderList.transparentDoublePass`, which batches all back sides before all
  front sides and only applies when `transmission > 0`. The two halves
  interleave per object, which is what makes six overlapping planes come out in
  three's order.

  The port clones the material for each half (`Side::Back` / `Side::Front`) and
  keys the program on a [`MaterialKey`] *variant*. The key is taken from the
  **original** material: `MaterialId::clone()` mints a fresh id by design, so
  keying on the clone would have minted a new program key every frame — the
  `steady_frame_builds_nothing` assertion catches exactly that.

`PassNode::set_layers` is the third gate: `camera.layers.disable( 2 )` /
`.set( 2 )` is how the page keeps the resolve quad out of the G-buffer pass and
the eight lights' sphere meshes out of the resolve.

### 27.6 Two small three behaviours the pixels found

* **`getGeometryRoughness()` is `float( 0 )` with no normal attribute.** The
  resolve quad's geometry has `position` and `uv` only, so three's
  `builder.geometry.attributes.normal === undefined` branch returns a constant
  instead of the `dFdx`/`dFdy` term, and the dump shows
  `Roughness = min( ( max( nodeVar4.w, 0.045 ) + 0.0 ), 1.0 )`. The port
  carries it as [`SetupContext::geometry_missing_normal`].
* **`Color.setHSL()` defaults to the *working* colour space.** The eight light
  colours are `new THREE.Color().setHSL( i / 8, 1.0, 0.5 )`, and
  `ColorManagement.workingColorSpace` is linear-sRGB, not sRGB. Treating them as
  sRGB moved 167 pixels.

### 27.7 Divergences

Both of the rung's programs match three's dump modulo the §8 classes (varying
order, uniform numbering and member order, `VERTEX_` / `NORMAL_` sub-build
temps, inlined single-use temps). The one new class is listed in §8: **MRT
member values are not promoted to a var**, so the G-buffer fragment ends

```wgsl
output.m1 = vec4<f32>( v_positionView, Metalness );
normalView = normalViewGeometry;
output.m2 = vec4<f32>( normalView, Roughness );
```

where three writes each through a `nodeVarN` first. The resolve quad's vertex
stage (`m13`) is byte-identical apart from the var number.

### 27.8 What was left out

* **`OrbitControls`.** Emulated as `camera.lookAt( 0, 0, -0.2 )`, as on every
  earlier rung.
* **A general `overrideNodes()`.** The port takes the three overrides the page
  uses as named fields rather than an arbitrary `[ node, node ]` list; an
  arbitrary list would need node identity in the memo keys.
* **`PassNode` options beyond `depthTexture` / `autoClearDepth` /
  `setLayers` / `opaque` / `transparent` / `lighting`.** Nothing else on the
  ladder sets one.
* **`renderer.lighting` as an object.** It is a bool on the renderer and on the
  pass; three's `Lighting` class also owns the lights node itself, which this
  port builds per draw.

[`Scene::environment`]: ../src/objects/scene.rs
[`SetupContext::lighting_disabled`]: ../src/materials/node_material.rs
[`SetupContext::geometry_missing_normal`]: ../src/materials/node_material.rs
[`MeshBasicNodeMaterial::context_overrides`]: ../src/materials/mod.rs
[`OverrideNodes`]: ../src/nodes/tsl.rs
[`PassNode::set_lighting_enabled`]: ../src/renderer/pass.rs
[`PassOptions::depth_texture`]: ../src/renderer/pass.rs
[`RenderTarget::set_size_keeping_depth`]: ../src/renderer/render_target.rs
[`RenderTarget::depth_initialized`]: ../src/renderer/render_target.rs
[`MaterialKey`]: ../src/renderer/mod.rs
[`MrtValue::Deferred`]: ../src/nodes/mrt.rs
[`tsl::perspective_depth_to_view_z`]: ../src/nodes/tsl.rs
[`tsl::range_fog_factor`]: ../src/nodes/tsl.rs
[`tsl::range_fog_factor_with_view_z`]: ../src/nodes/tsl.rs
[`uniform_settable`]: ../src/nodes/tsl.rs
[`PassNode::view_z_node`]: ../src/renderer/pass.rs
[`PassNode::depth_texture`]: ../src/renderer/pass.rs
[`TextureKind::DepthMultisampled2D`]: ../src/nodes/wgsl.rs
[`materials::tone_mapping_node`]: ../src/materials/node_material.rs
[`Texture::set_matrix`]: ../src/textures/texture.rs
[`Texture::set_channel`]: ../src/textures/texture.rs
[`tsl::uv1`]: ../src/nodes/tsl.rs
[`materials::physical::brdf_sheen`]: ../src/materials/physical.rs

## 26. `webgpu_loader_gltf_anisotropy` — anisotropy, a clearcoat, and a frame read back through glass

The Anisotropy Barn Lamp is three glTF materials in one draw list, and each of
them turns on a different branch of `PhysicalLightingModel`:

| material | extensions | what it gates |
| --- | --- | --- |
| `lamp metal` | `KHR_materials_anisotropy`, `KHR_materials_clearcoat` | the bent normal and the coat |
| `lamp filament` | `KHR_materials_emissive_strength` | `emissive * 25` |
| `lamp glass` | `KHR_materials_transmission`, `KHR_materials_volume` | the second pass and `getIBLVolumeRefraction` |

The page has **no lights at all**. Everything lit in the frame comes from the
PMREM of `royal_esplanade_2k.hdr.jpg`, which is both `scene.environment` and —
at `backgroundBlurriness = 0.5` — the background. That is what makes the rung
worth grading and also what bounds it: see §26.5.

### 26.1 `materialAnisotropyVector` and the bent normal

`MeshPhysicalNodeMaterial.setupAnisotropy()` turns the scalar strength and
rotation into a vector, optionally rotated again by the anisotropy texture's
`rg`, and the lighting model normalises it:

```
anisotropyV = mat2( aV.x, aV.y, -aV.y, aV.x ) * normalize( anisotropyMap.rg * 2 - 1 )
Anisotropy  = length( anisotropyV )        // If( Anisotropy == 0 ) { anisotropyV = vec2( 1, 0 ) } Else { anisotropyV /= Anisotropy }
AlphaT      = mix( roughness², 1, Anisotropy² )
AnisotropyT = TBNViewMatrix[ 0 ] * anisotropyV.x + TBNViewMatrix[ 1 ] * anisotropyV.y
AnisotropyB = TBNViewMatrix[ 1 ] * anisotropyV.x - TBNViewMatrix[ 0 ] * anisotropyV.y
```

With no lights, `D_GGX_Anisotropic` and `V_GGX_SmithCorrelated_Anisotropic`
are never reached — they live in `direct()`, and `direct()` is called once per
light. The only thing anisotropy does to this frame is the **bent normal**,
which is what `indirect()` hands to the radiance reflect vector:

```
bentNormalView = normalize( cross( cross( AnisotropyB, positionViewDirection ), AnisotropyB ) )
```

So the anisotropic streak on the lamp's shade is an environment reflection read
along a normal that has been bent towards the anisotropy bitangent, not a
specular lobe. §26.5 says what that leaves unverified.

### 26.2 The tangent frame comes from the attribute, not from the uv derivative

`AnisotropyT/B` are built from `TBNViewMatrix`, which with a `TANGENT`
attribute present is `mat3( tangentView, bitangentView, normalView )` — three's
`Tangent.js` and `Bitangent.js`, not the screen-space derivative frame a normal
map would otherwise synthesise. `with_tangent_attribute()` is the switch; the
shade carries `TANGENT`, so a wrong frame here shows as a streak pointing the
wrong way rather than as an error. The `NORMAL_v_bitangentView` varying is the
part that has to be named right for the dumps to line up.

### 26.3 Transmission needs the frame behind the glass

`getIBLVolumeRefraction` samples the *already drawn* opaque frame along the
refracted ray. Three does this by splitting the frame in two, and the dump of
`webgpu_loader_gltf_anisotropy` shows it exactly:

* pass 0 draws the background, `lamp filament` and `lamp metal` into an MSAA
  attachment that resolves into texture 0;
* the resolved texture is copied into texture 349 (800×500, 10 mips,
  `rgba16float`, single-sampled);
* passes 81–89 blit the nine mip levels;
* pass 90 draws `lamp glass` alone and reads texture 349;
* pass 91 is the output colour transform.

The port does the same in [`Renderer::draw`]: `transmission_split` is the index
of the first item whose material has `transmission > 0`, the draws before it go
in one pass, the encoder is submitted, `copy_framebuffer_to_opaque_frame()`
copies and re-mips, and the draws from the split on go in a second pass that
loads rather than clears. Three things fall out of that:

* **The copy source is the resolve target, not the MSAA attachment.** A
  multisampled texture cannot be sampled as a plain `texture_2d`, so
  `PassTarget` gained `color_texture: Option<wgpu::Texture>` holding the
  single-sample side. The MSAA attachment survives the first pass
  (`StoreOp::Store`), so the second pass loads it and resolves once at the end.
* **Bind groups may be built before the copy.** A bind group references the
  texture object, not its contents; building every draw up front and copying in
  between is correct, and it keeps one `Draw` list for both passes.
* **No double pass.** `needsDoublePass()` is `hasTransmission && side ===
  DoubleSide && forceSinglePass === false`. The lamp's glass is single-sided,
  so `transparentDoublePass` is empty and the port does not implement it.

`Renderer::opaque_frame_texture()` mirrors `viewportOpaqueMipTexture()`: a
`FramebufferTexture` with `generateMipmaps: true` and
`MinFilter::LinearMipmapLinear`, cached and reused while the size and format
hold.

### 26.4 A transmissive material sorts with the transparent list

`RenderList.push()` reads

```js
if ( material.transparent === true || material.transmission > 0 || … )
```

— the transmission test is an **or**, not a refinement of `transparent`. The
glTF sets no `alphaMode` on the glass, so `material.transparent` is `false` and
the object would otherwise sort among the opaques. It cost 1411 px before the
port matched it: the split landed ahead of `lamp filament`, the copy caught a
frame with no filament in it, and the bulb came out an even milky white with
the glow missing. §26.6 lists the pixel counts.

### 26.5 What the frame cannot check

* **The anisotropic GGX.** `D_GGX_Anisotropic` / `V_GGX_SmithCorrelated_Anisotropic`
  are behind `direct()`, and this scene has no lights. They are deliberately
  left out rather than written blind: nothing on this ladder would grade them,
  and an unverified lobe in the lighting model is worse than a missing one.
  The rung that adds a light to an anisotropic material adds them.
* **`anisotropyMap`'s rotation.** The barn lamp's anisotropy texture is read and
  its `rg` rotate the vector, but the strength-only path (no texture) is not
  separately graded here.
* **Double-pass transmission.** See §26.3.
* **Dispersion, iridescence, sheen, retroreflection.** Other flags of the same
  lighting model; none is in this glTF.

### 26.6 What the pixels found

| state | different pixels (of 100000) |
| --- | --- |
| glass opaque (no transmission at all) | ~2400 |
| transmission, glass sorted among the opaques | 1411 |
| transmission, glass in the transparent list | **27** |

### 26.7 Divergences

None new beyond the classes §8 already lists, and one fix that was a real bug:

* **`refract`'s eta.** `MathNode.REFRACT` builds its third operand as a
  `float`; the port was widening it to the input type and emitting
  `refract( vec3, vec3, vec3<f32>( … ) )`, which naga rejects. Fixed in
  `Node::Math`'s arm rather than worked around in the caller.
* **Uniform slot numbering.** `lamp glass`'s object struct has the same members
  in the same order and types as three's `m12`, but the `nodeUniformN` indices
  differ (three's run `0–3, 5–14, 17, 19, 23, 24, 26, 27, 29`; the port's
  `0–15, 19, …`) because the gaps are the texture uniforms, numbered by each
  builder's own traversal. §8, "Generated names".
* **The backdrop is a var on purpose.** Three keeps `getIBLVolumeRefraction`'s
  result in `nodeVar42` and reads it twice — once for `DiffuseColor.w`, once
  for `totalDiffuse`. The port does the same with an explicit `to_var`; without
  it the whole inlined bicubic expression is emitted twice, which is correct
  but doubles the fragment.
* **`getVolumeTransmissionRay`, `applyIorToRoughness` and `volumeAttenuation`
  are real `fn`s; `getTransmissionSample`, `getIBLVolumeRefraction` and
  `textureBicubicLevel` are inlined.** That is three's own split: the first
  three carry a `setLayout()`, the rest are plain `Fn()`. Reproduced
  deliberately so the two dumps line up statement for statement.

[`Renderer::draw`]: ../src/renderer/mod.rs
[`materials::transmission`]: ../src/materials/transmission.rs
[`tsl::with_tangent_attribute`]: ../src/nodes/tsl.rs
[`tsl::bent_normal_view`]: ../src/nodes/tsl.rs
[`Scene::background_blurriness`]: ../src/objects/scene.rs

## 28. `scene.fog` — `Fog`, `FogExp2` and their render-group uniforms

Issue #140. Until this section, the port had only `scene.fogNode`, so every
page that says `scene.fog = new THREE.Fog( … )` was ported as a
`fog( color, rangeFogFactor( near, far ) )` with the numbers folded in.

### 28.1 What three does

`NodeManager.updateFog( scene )` runs every render. When `scene.fog` is set,
it builds (and caches per fog object) one node whose parameters are
`reference( 'color' | 'near' | 'far' | 'density', …, sceneFog ).setGroup(
renderGroup )` uniforms:

```js
fog( color, rangeFogFactor( near, far ) )   // Fog
fog( color, densityFogFactor( density ) )   // FogExp2
```

and `getFogNode()` is `scene.fogNode || sceneData.fogNode`: an explicit fog
node wins. `NodeMaterial.setupFog()` then mixes the output towards the fog
colour: `vec4( mix( output.rgb, color, factor ), output.a )`.

### 28.2 The port

* [`Fog`] / [`FogExp2`] are the two classes, and [`SceneFog`] the enum
  `scene.fog` holds, so `isFog` / `isFogExp2` is a `match`.
* [`SceneFog::node`] is `updateFog()`'s node. Its uniforms name *where* the
  value comes from — `UniformSource::FogColor` / `FogNear` / `FogFar` /
  `FogDensity`, render group — rather than holding the fog object, and
  `Renderer::render` writes the scene's current values into the render group
  each frame. So the node does not depend on the values and there is one per
  fog kind, built once per thread: its identity, and with it the program
  cache key, is the same across frames and across fog objects of one kind. A
  new `near` is a uniform write; switching `Fog` for `FogExp2` is a new
  program, as in three.
* `Renderer::render` picks `scene.fog_node` first, then `scene.fog`'s node —
  `getFogNode()`'s `||`.
* [`tsl::density_fog_factor`] and [`tsl::exponential_height_fog_factor`] are
  `Fog.js`' other two factors, each with a `_with_view_z` twin for the
  `.context( { getViewZ } )` form (§24.3). `viewZ` is wrapped in `to_const`
  by hand because the builder does not promote a negation on usage count
  (§8, "`toConst` on the shadow filter"); three's `let` comes from the usage
  count.

### 28.3 Checked against

No three.js example renders a lit material under plain `Fog` or `FogExp2`
alone, so two minimal pages do it: `tools/dump-pages/fog_standard_linear.html`
and `fog_standard_exp2.html` (a `MeshStandardNodeMaterial` sphere, one
directional light). `tools/dump-webgpu.mjs --html FILE` serves such a page in
place of the checkout's. Against the r186 build, the fog statement is line
for line the port's (`dump_wgsl`'s `fog_standard_linear` /
`fog_standard_exp2`), and the fog members sit in the render struct after the
light members in both; `tests/nodes_fog.rs` holds that shape. The dumps of
`webgpu_postprocessing_difference` and `webgpu_shadowmap` now match on the fog
line too. The rest of the standard fragment differs only in the classes §8
already lists.

`webgpu_materials_texture_manualmipmap` is the graded rung: two scenes under
`new THREE.Fog( 0x000000, 1500, 4000 )`, and a floor whose eight mip levels
the page paints by hand (`Texture::set_mipmaps`: three uploads
`texture.mipmaps` level by level and generates nothing).

### 28.4 Divergences

* **Uniforms name a source, not an object.** Three's `reference()` reads
  `sceneFog.color` from the fog it was built for; the port's uniforms read
  whichever fog the scene being rendered holds. The same node therefore serves
  every scene, where three builds one per fog object. The WGSL is the same.
* **`webgpu_instance_sprites`, the issue's first choice of rung, is not
  ported.** It needs the `Sprite` object (issue #143, in progress on its own
  branch), `alphaMap`, and `alphaTest`. Building `Sprite` here would collide
  with #143, so the rung is `webgpu_materials_texture_manualmipmap`.

[`Fog`]: ../src/objects/fog.rs
[`FogExp2`]: ../src/objects/fog.rs
[`SceneFog`]: ../src/objects/fog.rs
[`SceneFog::node`]: ../src/objects/fog.rs
[`tsl::density_fog_factor`]: ../src/nodes/tsl.rs
[`tsl::exponential_height_fog_factor`]: ../src/nodes/tsl.rs

## 29. KTX2 and compressed textures (`webgpu_textures_2d-array_compressed`, issue #172)

`Ktx2Loader` (`src/loaders/ktx2_loader.rs`) is `KTX2Loader.js` on three pure
Rust crates: `ktx2` for the container, `basisu` (Basis Universal v2.1) for
ETC1S / UASTC / UASTC HDR transcoding, and `ruzstd` for Zstandard
supercompression. `tests/ktx2_loader.rs` checks it against three's own loader
under node (`tools/ktx2_reference.mjs`), byte for byte, on every `.ktx2` in
the examples, two zstd repacks and the Basis images of the three basisu GLBs,
for each of the four device profiles a WebGPU adapter can present (no
compression, BC, ASTC, ETC2).

A compressed texture is a [`Texture`] with `mipmaps` and, for an array, a
`depth` (`src/textures/compressed_texture.rs`); the renderer uploads the
levels as given with a block-counted row stride and never generates mips for
it. A texture with `depth > 0` binds as `texture_2d_array<f32>`
(`TextureKind::Sampled2DArray`) and `texture_array( map, uv, layer )` samples
it as `textureSample( t, s, uv, i32( layer ) )`, which is three's
`texture( map, uv ).depth( layer )`.

### Divergences

* **ETC1S → BC7 turns Basis v2's chroma filtering off.** Three ships the
  v1.16 transcoder; `basisu` is v2.1, whose ETC1S → BC7 path smooths chroma
  by default. `transcode_flags()` passes `NO_ETC1S_CHROMA_FILTERING` for that
  one pair, which makes it bit-exact with three again (the test fails on
  `2d_etc1s` mips 0–3 under the `bc` profile without it). Every other path
  already matched.
* **`GLTFLoader` without `setKTX2Loader`.** Three refuses a
  `KHR_texture_basisu` texture unless a `KTX2Loader` was set. `GLTFLoader::load`
  / `parse` use `Ktx2Loader::new()` instead, which transcodes to uncompressed
  RGBA; `load_with_ktx2` / `parse_with_ktx2` take a loader that has run
  `detect_support`, which is three's call.
* **`ruzstd` 0.7, not 0.9.** `basisu` depends on 0.7; using the same version
  keeps one Zstandard decoder in the build.
* **`CompressedCubeTexture` and `Data3DTexture` load but do not render.**
  `Ktx2Loader::parse` returns them (the eight PMREM cubes are in the oracle
  test), but `Ktx2Texture::into_texture` returns an error for both, because the
  renderer has no compressed-cube or 3-D upload path yet.
* **Display P3 has no gamut conversion.** `parse_color_space` reports
  `display-p3` / `display-p3-linear` like three; `into_texture` maps it to
  `SRGB` / `NoColorSpace` by transfer function only, since the port has no P3
  working space. No graded page uses a P3 file.
* **An unfilterable (`NearestFilter`) array texture is not supported.** Three
  would `textureLoad` it; the builder asserts instead. `KTX2Loader` only
  makes arrays of compressed (linear-filtered) textures, so only a hand-built
  array with both filters set to `Nearest` reaches the assertion.

[`Texture`]: ../src/textures/texture.rs
## 28. WebP and AVIF glTF textures (issue #179)

Three has no image decoders: `GLTFLoader` hands a texture's bytes to the
browser's `createImageBitmap` (with `premultiplyAlpha: 'none'`,
`colorSpaceConversion: 'none'`) and the renderer uploads the bitmap with
`copyExternalImageToTexture`. The port decodes in Rust instead, in
[`TextureLoader`](../src/loaders/texture_loader.rs), and a decoder is only
taken when it gives the texels Chromium gives.

**WebP** goes through `image-webp`. `tests/loaders_webp.rs` runs Chromium's own
decode and upload (`tools/image_reference.mjs`, in the headless Chrome three's
`npm ci` downloads) over every WebP image in the examples — 64 images in eight
GLBs: lossy, lossy with an `ALPH` plane, and lossless — and requires every
texel to be equal. They are, with no tolerance. `EXT_texture_webp` is in
`SUPPORTED_EXTENSIONS`, and a texture carrying it samples the extension's
`source`, never its own fallback `source`, as `GLTFTextureWebPExtension` does.

### 28.1 Divergences

* **AVIF is not decoded.** `EXT_texture_avif` is not in `SUPPORTED_EXTENSIONS`,
  so a file that *requires* it (`AVIFTest/forest_house.glb`, the only one in
  the examples) is refused with `UnsupportedRequiredExtension`, where Chrome
  would decode it. A file that only *uses* it gets the texture's fallback
  `source`, which is what three does in a browser without AVIF. An AVIF image
  reached any other way is an error that names AVIF, not a JPEG error. No
  pure-Rust AV1 decoder is fit yet: `rav1d` (and its `re_rav1d` fork) do not
  compile for wasm32-unknown-unknown; `avif-rust` 0.0.7 compiles everywhere
  but refuses 2 of forest_house's 12 images ("too many padding bits"), which
  dav1d decodes; `rav1d-safe` and `zenavif` are AGPL; `avif-decode` is `rav1d`
  underneath and needs Rust 1.98; `oxideav-av1` is a scaffold.
* **An animated WebP gives its first frame**, which is what `createImageBitmap`
  gives too; no three.js asset is animated, so this is untested.
