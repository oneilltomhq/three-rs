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
* **Fog parameters as constants.** `fogColor` / `fogNear` / `fogFar` are folded
  into the WGSL as literals rather than carried as `renderStruct` members,
  because nothing in the port animates them. Same numbers.
* **`toConst` on the shadow filter.** `to_const()` exists since
  `webgpu_postprocessing_radial_blur` (`Node::Let`, a WGSL `let nodeConstN`),
  but `pointShadowFilter`'s `shadowPosition` and `shadowPositionAbs` are still
  `to_var()`s, where Three uses `toConst()`. Same single evaluation, same
  value. Two of them are not optional: `Node::Swizzle` and `Node::Neg` are not
  kinds the builder promotes on usage count, so `shadowCoord.xyz` and
  `viewZ.negate()` are wrapped by hand or the expression would be emitted
  twice.
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

## 13. PMREM (`webgpu_pmrem_cubemap`, `webgpu_pmrem_test`)

`PMREMGenerator` (`src/renderer/pmrem.rs`), the cube-UV read side
(`src/nodes/pmrem_utils.rs`, `src/nodes/pmrem_node.rs`) and `EnvironmentNode`
(`src/materials/environment.rs`). `fromCubemap` and `fromEquirectangular` are
both ported and share everything but `_setSizeFromTexture` and the one-tap
material that fills level 0 — that is three's own shape, `_fromTexture` with a
`PmremSource` in place of the `texture.mapping` test. `fromScene`, and with it
the golden-angle Gaussian blur shader and `BLUR_SAMPLES`, is still deferred:
no example on the ladder reaches it.

The four shaders the two rungs add are generated by the node system and match
three's dumps line for line, up to the classes §8 already lists (the header
line, the session-global uniform/varying counters, uniform-struct field order,
blank lines after a block): `PMREM_cubemap` against `dump-pmrem_cubemap/m01`,
`PMREM_ggx` against its `m03`, the `Background.material` cube-UV read against
its `m05`, and `PMREM_equirect` against `dump-pmrem_test/m02`. All four are in
`examples/dump_wgsl.rs`, as `pmrem_cubemap`, `pmrem_ggx`, `pmrem_background`,
`pmrem_equirect`, with `pmrem_test_background` and `pmrem_test_physical` for
`webgpu_pmrem_test`'s own two.

`PMREM_equirect` is the whole delta between the two examples: `texture(
envTexture, equirectUV( _outputDirection ), 0 )`, four lines of WGSL. Note
what it does *not* carry — the environment rotation. `cubeTexture()` applies
`materialEnvRotation` inside `CubeTextureNode.setupUV()`, so the cubemap
material has it; a plain 2-D `texture()` node does not, and three's dump agrees.
The explicit level `0` matters for the same reason: the six quads of a lod
plane are a wildly non-uniform parameterisation of the sphere, so an implicit
derivative sample would pick a different level per face.

Numeric gates sit under both images. `tests/pmrem.rs` (no GPU) has the GGX
roughness ladder and its `mipInt`s, the atlas rectangles and
`_generateCubeUVSize`, from three's own code driven directly — see the file's
header. `tests/pmrem_equirect.rs` (GPU) adds the two that only the equirect
path can fail: the bright texel of `spot1Lux.hdr` landing on the flipped row,
and the atlas lighting exactly one mip-0 face and every one of the eleven LOD
tiles.

### Divergences specific to this rung

* **`ConvertNode` is not a `TempNode`.** Three's `vec3( direction_immutable )`
  inside `bilinearCubeUV` re-expands at every use, so the rotated direction
  appears twice in the generated body — once for `getFace`, once for `getUV`.
  The port reproduces that on purpose (`.to( Type::Vec3 )` at the top of
  `bilinear_cube_uv` and `texture_cube_uv`); the `needs_var()` reuse rule is
  not applied to a cast. The same fact made `Node::Cast` narrowing go through
  `wgsl::convert`, so a narrowing cast is the swizzle arm (`x.xyz`) and only a
  same-width cast keeps the explicit constructor — which is what
  `ConvertNode.generate()`'s `builder.format( snippet, from, to )` does.
* **An inlined `Fn`'s result is varred explicitly.** Three's `flowShaderNode`
  vars the result of an inlined function when it is used more than once; the
  port applies its reuse rule to expression nodes but not to a `Block`, which
  re-generates at every use. `ggx_convolution` therefore asks for the var by
  hand (`to_var( None, importance_sample_ggx_vndf( … ) )`), as do the two
  `bilinear_cube_uv` taps in `texture_cube_uv`. Without it the VNDF body is
  inlined once per component.
* **Three's `PI` literal, not `f64::consts::PI`.** `PMREMUtils.js` writes
  `2.0 * 3.14159265359`, one digit short of the constant, and the difference is
  visible in the WGSL. Reproduced literally, with an `#[allow]` for clippy.
* **`updateBefore` became `PmremEnvironment::update`.** In three a `PMREMNode`
  carries `NodeUpdateType.RENDER` and builds the PMREM from inside the node,
  during the render. A node here is an immutable `Rc` graph with no
  back-reference to the renderer, and `Renderer` methods take `&mut self`, so
  the trigger moved out: the application calls
  `PmremEnvironment::update( &mut renderer )` before it renders. It is
  idempotent, and the uniforms it writes are the same three cubeUV cells
  three's node owns, so the generated WGSL is unaffected.
* **`_uniformsMap` became a texture handle that gets repointed.** Three swaps
  `ggxUniforms.envMap.value` between the atlas and the ping-pong target between
  the two passes of a step. The port holds one borrowed `Texture`
  (`own_gpu == false`) and calls `set_gpu()`; bind groups are built per draw,
  so the swap lands on the next draw with no cache to invalidate.
* **`mipInt` is computed in floating point.** `_lodMax - lodIn` is JS
  arithmetic and goes negative for the extra LODs — −1 and −2 for a 256² source
  — which a `usize` subtraction would not survive. `tests/pmrem.rs` pins both
  negative values against three.
* **`flipY` is a CPU row reversal, not two render passes.** `HDRLoader` sets
  `texData.flipY = true`, and three's WebGPU backend honours it for a
  buffer-sourced texture with `WebGPUTextureUtils._flipY()`: two extra render
  passes that borrow the mipmap blit pipeline to bounce the source through a
  scratch texture and back. That is why three's dump of `webgpu_pmrem_test`
  submits 25 passes where the port submits 23. `upload_texture_2d` reverses the
  rows in the staging copy instead. A flip is an exact texel permutation and
  the blit samples texel centres of an equally sized target, so the two results
  are bit-identical; the flip is gated on the flag, so a `flip_y == false`
  texture is byte-for-byte what it was. What says so is
  `tests/pmrem_equirect.rs`: one lit texel in a 1024×512 black field, sampled
  at its own texel centre with nearest filtering, has to come back at row
  `512 - 1 - 213 = 298`. A missing flip, an off-by-one flip or a row-stride bug
  each move it somewhere provably wrong — and each of them would still render
  a perfectly plausible shiny sphere.
* **`scene.background = <a texture>` is a `Background::Pmrem` variant.**
  Upstream the background is a `Texture` whose `mapping` is
  `CubeUVReflectionMapping`, which `NodeManager.getBackgroundNode()` turns into
  `pmremTexture( background )` and `Background.update()` wraps in a node
  context supplying `getUV` (`backgroundRotation.mul( normalWorldGeometry )`)
  and `getTextureLevel` (`backgroundBlurriness`). The port has no node context
  and no mapping constant, so the variant carries a `PmremHandle` — which is
  what the three cubeUV uniforms travel on — and the renderer builds the same
  graph with the two accessors passed as arguments
  (`materials::background_pmrem_color_node`). The generated WGSL is three's
  `m06` statement for statement.
* **The lit material is not gated against `m07` as a whole.** `m07` is r186's
  restructured `PhysicalLightingModel` (`multiScatteringCompensation`, `dfg`,
  `NORMAL_normalView`); the port still carries the earlier one, which is what
  the `webgpu_lights_physical` row is gated against, and changing it is a rung
  of its own. What *is* gated is the part this rung adds: the 86-line
  `radiance` + `iblIrradiance` block `EnvironmentNode` contributes matches
  `m07` statement for statement, under the port's existing property-`addAssign`
  spelling and a renumbering of `nodeVar`/`nodeUniform`. With no environment
  the two properties are still declared zero in `indirectSpecular`, which is
  why every other row of the ladder is byte-identical.

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
* **Fog parameters as constants** (the existing §8 entry) is the only
  divergence in this rung's scene fragment: Three keeps `fogColor` / `fogNear`
  / `fogFar` as render uniforms, the port folds the same values in.

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
