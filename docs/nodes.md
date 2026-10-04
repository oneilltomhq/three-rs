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

> Since §38 the layer and `NORMAL_VALUE` are the `sub_build` and
> `setup_normal` fields of one `BuildContext` stack; what follows is
> otherwise unchanged.

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
* **`clearcoatNormalView` assigned before the light loop (§34).** Three
  assigns the var at its first read, inside `direct()` after `irradiance`; the
  port assigns it where the normal is set up, ahead of the loop, and emits the
  direct clearcoat statement before the `irradiance` var rather than after.
  Straight-line assignments of the same values, each ahead of every read.
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
* **Usage-promoted temps (§31).** Three at 5f610f5 turns a temp that is read
  more than once into `let nodeConstN` without a `toConst()` in the source:
  `webgpu_compute_texture`'s `posX`, `posY`, `x`, `y` and `v`,
  `RaymarchingBox`'s hit-box temps and `webgpu_volume_perlin`'s
  `surfacePos + 0.5`. The port's builder still promotes such a temp to a
  `nodeVarN`, so both examples and `raymarching_box()` ask for `to_const`
  where the dump has a `let`, with a comment at each. The material tail's
  output value, `let nodeConstN = max( vec4<f32>( DiffuseColor.xyz,
  DiffuseColor.w ), vec4<f32>( 0.0 ) )`, is shared by every rung and stays a
  var here. Same single evaluation,
  same value; `tests/nodes_texture_wgsl.rs` compares the sections up to it.
* **Whitespace-only lines (§31).** Three leaves an indented blank line after
  the last statement of every `If` body and an empty `// directives` block at
  the top of a render stage. The port emits neither;
  `tests/nodes_texture_wgsl.rs::canonical()` drops lines that are empty or
  only whitespace on both sides.
* **Two unread `vec3` privates in `webgpu_volume_perlin` (§31).** Three's
  fragment declares two `var<private>` `vec3<f32>` temps (`nodeVar14`,
  `nodeVar15` in the fixture) that no statement in the module reads or writes. Nothing in `RaymarchingBox`, the page's
  `opaqueRaymarchingTexture` or `Texture3DNode.normal()` explains them —
  unexplained, and the port does not declare them. The gate compares the
  uniform block and the flow, not the `// vars` block.
* **One flat `instanceIndex` varying per `range()` (§33).** Every
  `tsl::instanced_range` wraps its own `instanceIndex` in its own varying (§9),
  so `webgpu_particles`' smoke sprite passes two flat `u32` varyings holding
  the same value, where three's `IndexNode` gives the one `instanceIndex` one
  varying. Same values, one extra interpolant, and the later `@location`s
  shift by one.
* **`varyings.positionLocal = positionLocal` (§33).** Three writes every
  assignment to a varying straight into `varyings.name`, so its vertex stage
  has `varyings.positionLocal = position;` and later `varyings.positionLocal =
  nodeConst1;`. The port keeps the vertex-stage value in a private var and
  writes the varying once, from that var, when the fragment stage asks for it.
  The fragment stage reads the same final value. Before #167 the port wrote
  the geometry's position there instead of the var, which was wrong, not a
  divergence; no graded frame could see it.
* **The banner.** The pinned commit's dumps are headed
  `// Three.js r187dev - Node System`, where r186's said `r186` and the port
  says `three-rs`.
  `tests/nodes_compute_indirect_wgsl.rs::canonical()` drops it, as
  `tests/nodes_compute_wgsl.rs` drops r186's.

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
  per stage. The result sits in the builder's `NodeCache` (§36).
* **A layout `fn` is emitted into each stage that calls it.** Each stage is
  its own module. The name is shared.
* **A varying the vertex stage has assigned to is written from that var.**
  In three `positionLocal` *is* the varying, so the fragment stage reads the
  value after `positionLocal.assign( positionNode )`. The port writes
  `varyings.positionLocal = positionLocal;` after the assignment instead of
  three's `varyings.positionLocal = ( varyings.positionLocal + … )` in
  place: the text differs, but the value the fragment reads is the same. A
  varying that was only read keeps its old form.
### Shadow filters add no new class

The VSM and point-light-alpha modules differ from three's dumps only in the
classes above. The hooks' API shapes, the missing `shadowSide` and the
silent `PCFSoftShadowMap` are listed in §32.5.

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
`ComputeFlow::new( statements, count )` is `.compute( count )` with three's
default `[ 64 ]` workgroup; the other three are fields set afterwards.

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
holds them as [`uniform_settable`] pairs and writes them at the top of the
pass's `updateBefore()`, exactly where three does.

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

§45 added that `ContextNode`, and the page now uses the upstream shape:
`range_fog_factor( 2.7, 4.0 ).context( … )`. The argument form stays and
builds the same WGSL.

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
not owning it, so the pass's `updateBefore()` resizes through
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
  `KHR_texture_basisu` texture unless a `KTX2Loader` was set. `GltfLoader::load`
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
  `ColorSpace::Srgb` / `ColorSpace::LinearSrgb` by transfer function only,
  since the port has no P3 working space. No graded page uses a P3 file.
* **An unfilterable (`NearestFilter`) array texture is not supported.** Three
  would `textureLoad` it; the builder asserts instead. `KTX2Loader` only
  makes arrays of compressed (linear-filtered) textures, so only a hand-built
  array with both filters set to `Nearest` reaches the assertion.

[`Texture`]: ../src/textures/texture.rs
## 30. WebP and AVIF glTF textures (issue #179)

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

### 30.1 Divergences

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

## 31. `Data3DTexture` and storage textures — `webgpu_compute_texture`, `webgpu_volume_perlin`

Issue #166. Two texture kinds and the node plumbing around them:

* `Texture::storage( w, h )` is three's `StorageTexture`: no image, written by
  a kernel through `textureStore()`, sampled afterwards like any texture.
* `Data3DTexture` (`src/textures/data3d_texture.rs`) is three's
  `Data3DTexture`, and `Data3DTexture::storage( w, h, d )` its
  `Storage3DTexture`. They share a type because a `Storage3DTexture` *is* a
  3-D texture with `isStorageTexture` set, and the renderer path is the same.

`webgpu_compute_texture` grades the 2-D storage path and
`webgpu_volume_perlin` the sampled 3-D path. `Storage3DTexture` has no graded
rung (§31.6); `tests/nodes_texture_wgsl.rs` builds a kernel that writes one
and a material that samples it, and validates both modules with naga.

### 31.1 A storage binding is a different binding

`storageTexture( t )` and `texture( t )` of the same texture are two bindings
in three: `NodeStorageTexture` versus `NodeSampledTexture`, with different
layout entries and, in `WebGPUBindingUtils`, different views. The port keys a
texture binding by `(texture id, is_storage_binding)` in the builder and by
`(texture id, view dimension, storage)` in the renderer's view cache, so a
material that samples the texture a kernel writes gets its own view and
sampler.

The declaration is `texture_storage_2d<format, access>` (or `_3d`), with the
format from the texture (`wgsl::storage_format()`, which panics on a format
WGSL cannot store) and the access from the node. Outside a compute stage
three's `getNodeAccess()` forces `read` whatever the node says, and so does
the port (`TextureKind::declaration`), because a fragment or vertex stage cannot
declare a writable storage texture without a feature the ladder does not ask
for.

`textureStore( t, uvec2( posX, posY ), value )` is a statement, not a value:
`Node::TextureStore` pushes `textureStore( t, vec2<u32>( … ), … );` and
returns nothing to its reader. The coordinate is converted to `vec2<u32>` /
`vec3<u32>` as `generateTextureStore()` does.

### 31.2 The storage view has one mip, and a store makes the chain stale

A `StorageTexture` keeps `generateMipmaps = true`, so it is allocated with its
full chain (a 512² texture has ten levels), but WebGPU only binds one level as
a storage texture: `WebGPUBindingUtils` gives a storage binding a view with
`mipLevelCount: 1`. The port does the same (`mip_level_count:
storage.then_some(1)`).

The kernel writes level 0 only. Three then rebuilds the chain from it:
`Bindings._update()` marks a storage texture `needsMipmap` when it is bound
for a store, and the next time the texture is bound as an ordinary sampled
texture it generates the mips first. The port carries the same flag on its
2-D texture entry (`needs_mipmap`), set when a `Storage` binding is built and
cleared by `update_storage_mipmaps()` when a sampled binding next asks for the
view. This is the whole of `webgpu_compute_texture`'s frame: the plane is
minified, `LinearFilter` picks the nearest mip, and without the rebuild it
reads a zero-filled level 1 (15876 of 100000 pixels, most of the plane black).

A storage texture is created with `STORAGE_BINDING` added to its usages and is
never uploaded, and it is checked against
`format.guaranteed_format_features( device.features() )` before it is created
(`assert_storage_format`), with a panic that names the format. That is the
wasm32 concern the issue raises: Vulkan adapters report storage support for
formats a browser's WebGPU does not guarantee (`r8unorm`, and `bgra8unorm`
without a feature, among others), so a check against the adapter would pass
on the desktop and fail as a validation error on the web. The guaranteed set
is what wgpu promises on every backend, so a format that passes here passes
in the browser too. Both rungs use `rgba8unorm`; the pingpong page's
`rgba16float` is in the set as well.

Compute bind groups now take textures and samplers as well as buffers, through
the same `texture_view()` the render path uses.

### 31.3 `Data3DTexture` and `texture3D()`

`Data3DTexture` defaults to `NearestFilter` and `ClampToEdgeWrapping` on all
three axes, as three's does, and uploads its bytes as one `D3` texture with
`mipmaps` never generated (three's `generateMipmaps = false`). Its sampler key
adds the `r` wrap.

`texture3D( t, null, level )` is `Texture3DNode`: `sample( uv )` is
`textureSampleLevel( t, s, uv, level )`, and `.r` on it builds the node as a
`float`, which the builder writes as a `.x` on the sample, as three does.
`normal( uv )` is `Texture3DNode.normal()`: the central difference over six
taps at `±0.01`, emitted as three emits it — six nested `If`s, each tap a
var, and a normalised difference. Three has a second path for an
unfilterable volume (`textureLoad` against `textureDimensions`, no sampler);
no page on the ladder samples a `NearestFilter` volume, so the builder asserts
instead of porting it. A sampler-less `textureSampleLevel` would be a shader
compile error, not a wrong picture, so the assertion is the honest failure.

`webgpu_volume_perlin` sets `LinearFilter` on both filters, which makes it
filterable: `texture_3d<f32>` with a filtering sampler.

### 31.4 `Loop( { type: 'float' } )`, `Break()` and `bool` uniforms

`RaymarchingBox` (`src/addons/raymarching.rs`) needs three things the node
system did not have:

* **a float loop with a step.** `loop_float( name, start, end, update, body )`
  is `Loop( { type: 'float', start, end, update } )`: `for ( var i : f32 =
  start; i < end; i += update )`. `update` is a node, here the step size var.
* **`Break()`**, as `break_loop()`: a statement that emits `break;`.
* **a `bool` uniform.** `uniform( true )` is `refine` in the page. WGSL has no
  host-shareable `bool`, so three stores it as a `u32` member of the object
  struct and reads it back as `bool( object.nodeUniformN )`, into a var at the
  first read. The port does the same, and `programs.rs` writes the member
  through its existing `u32` path, as `0` / `1`.

`boolean( v )` is `bool( v )` of a literal, for the `false` / `true` the
raymarcher assigns to its hit flag.

### 31.5 What the WGSL gate compares

`tests/nodes_texture_wgsl.rs` compares:

* `webgpu_compute_texture`'s kernel as a whole module;
* its material's uniform block and flow;
* `webgpu_volume_perlin`'s fragment uniform block and flow (the slab test,
  the float loop, the bisection with its two `select`s, `normal()`'s nested
  `If`s, `break`);
* the vertex stage's two ray varyings, by expression.

Three §8 entries come from these dumps: "Usage-promoted temps",
"Whitespace-only lines" and the two unread `vec3` privates. The same file pins
`ImprovedNoise`'s volume: a checksum over all 2 097 152 bytes, computed by
running the page's fill under Node against three's `ImprovedNoise.js`. That
fill writes into a `Uint8Array`, whose `ToUint8` wraps rather than clamps, so a
noise value of exactly 1 becomes 0, not 255. `volume_data()` wraps the same
way (`trunc` then `rem_euclid( 256 )`).

### 31.6 What was left out

* **`webgpu_compute_texture_pingpong`.** Its first frame is a hash noise,
  `fract( sin( dot( uv, seed ) ) * 43758.5453 )`, whose low bits are GPU `sin`
  precision, not something the port controls. Its seed is `Math.random()`,
  redrawn once a second from `performance.now()`. And it blurs between two
  `HalfFloatType` storage textures, alternating which one `material.map`
  shows. The frame the grader sees is not a function of the page's source, so
  there is no rung to grade. The pieces it needs (a read-only storage binding
  through `.load()`, an `rgba16float` storage format) are in the port.
* **`webgpu_compute_texture_3d`.** It writes a `Storage3DTexture` from a
  kernel, which is here and tested (§31), but it also needs `CanvasTexture`
  for the sky's gradient and MaterialX's `mx_noise_vec3` with the time, which
  are other issues. The storage-3D path has a WGSL and naga test but no
  graded frame until that rung lands.
* **The unfilterable 3-D sample** (§31.3) and **`RenderTarget3D`** and
  **KTX2** volumes, the last two out of the issue's scope.

## 32. Shadow filters, `filterNode` / `shadowNode`, and VSM (`webgpu_shadowmap_vsm`, `webgpu_shadowmap_pointlight`)

`ShadowNode.setupShadow()` picks the filter as `shadow.filterNode ||
_shadowFilterLib[ renderer.shadowMap.type ]`, over `[ BasicShadowFilter,
PCFShadowFilter, null, VSMShadowFilter ]`. The port spells that
`ShadowFilter::of( type, filter_node )` in `src/lights/shadow_filter.rs`, with
`Renderer::shadow_map_type` (`ShadowMapType::{Basic, Pcf, PcfSoft, Vsm}`,
where `PcfSoft` resolves to `Pcf` as `Renderer.render()` rewrites it) and
`LightShadow::{filter_node, shadow_node}`.

### 32.1 What each type changes besides the filter

* **The depth texture's filtering.** Linear for PCF (the compare sampler
  interpolates), Nearest for Basic and VSM. The cube depth texture follows
  the same rule, which is why the `CubeDepth` sampler now reads the texture's
  own filters instead of a hard-coded Linear.
* **VSM (non-point lights).** There is no compare sampler. Two `RGFormat` /
  `HalfFloatType` targets with no depth buffer (`VSMVertical`,
  `VSMHorizontal`) get one `QuadMesh` pass each (`vsm_pass_vertical`,
  `vsm_pass_horizontal`), and the lit material's `VSMShadowFilter` reads the
  second target's `( mean, stdDev )`. The passes read `radius`,
  `blurSamples` and `mapSize` through the same `Shadow*( index )` render
  uniforms the lit material uses.
* **VSM's shadow pass.** The override material keeps `material.side` instead
  of `_shadowSide[ side ]`. `receiveShadow` objects are drawn into the map too
  (the `renderObject` function `ShadowNode` installs). Both apply to point
  lights under VSM as well, because the shadow pass does not know the light
  kind.
* **Point lights** use `BasicPointShadowFilter` when the type is Basic and
  `PointShadowFilter` otherwise, **VSM included**: `PointShadowNode` has no
  VSM path, so a VSM renderer gets PCF on its point lights. Reproduced.

### 32.2 An `RGFormat` texture is a `vec2` node

`NodeUtils.getTextureType()` types a texture node by its format's component
count. The port now does this for two-channel formats (`tsl::texture`,
`tsl::texture_uv`): the node is `Type::Vec2`, and the builder appends `.xy` to
the fetch, so the value is cached as `nodeVarN : vec2<f32> = textureSample(
… ).xy`, as in three's `VSMHorizontal` module and in `VSMShadowFilter`'s
`distribution`. The DFG LUT is `Rg16Float` too, so every physical material's
`dfg` sample moved from a `vec4` var read as `nodeVarN.xy.x` to three's own
`vec2` var read as `nodeVarN.x`. The arithmetic is identical and no rung moved.
Red-only formats stay `vec4` until a rung needs three's `float` typing.

### 32.3 `NodeBuilder.getOutputType()` for an RG target

A pass into an RG target declares `@location( 0 ) color : vec2<f32>` and
writes a `vec2` (`NodeBuilder::with_output_components`). The renderer keys the
program on the target's component count, because the same material drawn into
an RGBA target is a different module.

### 32.4 `alphaMap` and `alphaTest`

`webgpu_shadowmap_pointlight` needed both. `MaterialNode.OPACITY` with an
`alphaMap` is `opacity * texture( alphaMap )`, a `vec4` product, which the
`DiffuseColor.w` assign narrows to `.x`. Three's dump reads `( vec4<f32>(
DiffuseColor.w ) * ( vec4<f32>( opacity ) * texel ) ).x`, and so does the
port's. `alphaTest > 0` discards against the `materialAlphaTest` object
uniform, with `alphaTestNode` still taking precedence. The shadow pass copies
both onto its override material (`overrideMaterial.alphaTest` /
`.alphaMap`), so the cut-away bands cast no shadow. The override keeps its own
`opacity` of 1.

### 32.5 Divergences

* **`filterNode`'s signature.** Three calls `filterNode( { depthTexture,
  shadowCoord, shadow, depthLayer } )`, and for point lights `{ depthTexture,
  bd3D, dp, shadow }`. The port's hook is a Rust closure
  (`ShadowFilterFn`) over `ShadowFilterInputs { index, map, shadow_coord, dp
  }`. `index` stands in for `shadow`, because the light's shadow uniforms
  (`radius`, `mapSize`, …) are addressed by light index. A point light's
  `bd3D` arrives as `shadow_coord`. There is no `depthLayer`, because array
  and CSM shadows are out of scope.
* **`shadowNode`** is a `NodeRef` that replaces the whole `ShadowNode` for
  that light. It is still gated on `castShadow`, `receiveShadow` and
  `shadowMap.enabled`, no map is rendered, and no shadow position is pushed,
  exactly as `AnalyticLightNode` does it.
* **No `material.shadowSide`.** The override side is `_shadowSide[ side ]`,
  or `side` under VSM. A material that sets `shadowSide` would need the field.
* **`PCFSoftShadowMap` resolves silently.** Three logs a deprecation warning.
* **Var numbering in `VSMVertical`.** Three numbers the `textureLoad` result
  before the `textureDimensions` temp, and the port the other way round. This
  is the §24.6 class.
* **Fog in `webgpu_shadowmap_vsm`.** `new THREE.Fog()` is spelt as the
  constant `fog( color, rangeFogFactor( near, far ) )` node, as in
  `webgpu_shadowmap`. The colour and range are literals where three reads
  uniforms.
* Everything else in the lit modules is §8's `toConst` / var class and the
  uniform-slot numbering. The filter bodies line up statement for statement:
  `VSMShadowFilter`'s `step`, the `!= 1.0` branch, and the Chebyshev bound
  remapped by `( p - 0.3 ) / 0.65`.

## 33. Indirect draws, struct storage, atomics and workgroup memory (`webgpu_struct_drawindirect`, `webgpu_particles`)

Issue #167. The compute stage of §11 wrote flat arrays over a flat index. This
section adds what the rest of three's compute examples build on: a buffer the
GPU both computes into and draws from, typed views of it, atomics, and memory
shared inside a workgroup.

### 33.1 `IndirectStorageBufferAttribute` and the indirect draw

`IndirectStorageBufferAttribute::new( words, item_size )` is a `u32` array
with its own `BufferId`. The renderer creates one GPU buffer for it, with
`STORAGE | INDIRECT | COPY_SRC | COPY_DST` usage, keyed by that id. A kernel
that binds the attribute as storage and a draw that reads it as arguments
therefore use the same buffer, which is the whole point of the class.

`BufferGeometry::set_indirect( attr )` is `geometry.setIndirect( attr )`. A
draw whose geometry has one calls `draw_indexed_indirect` (indexed geometry)
or `draw_indirect` at offset 0, in place of the direct draw. The arguments
are WebGPU's: `[ vertexCount, instanceCount, firstVertex, firstInstance ]`,
or `[ indexCount, instanceCount, firstIndex, baseVertex, firstInstance ]`
when indexed.

`renderer.info()` records the CPU-side counts for an indirect draw, as
three's `WebGPUBackend.draw()` does (`info.update( object, vertexCount,
instanceCount )` after the indirect branch). What the GPU actually drew is
not known on the CPU, so neither counts it. `Renderer::read_indirect_buffer()`
reads the arguments back, for tests.

`Renderer::compute_indirect( flow, attr )` is `computeIndirect`: the dispatch
size comes from the attribute's first three words.

Two smaller pieces ride along:

* **`BufferGeometry::instance_count`** and
  **`BufferAttribute::new_instanced()`** (`InstancedBufferGeometry` and
  `InstancedBufferAttribute`). An instanced attribute's vertex buffer steps
  per instance. The program's cache key includes which attributes are
  instanced, because the same material on a plain geometry needs a
  per-vertex layout.
* **`Mesh.count`**. `SpriteNodeMaterial` draws `count` instances of a plain
  mesh with no `InstancedMesh`, as `webgpu_particles` does.

### 33.2 `struct` storage

`struct_type( name, members )` is `struct( { … }, name )`, and
`storage_struct( &attr, layout )` is `storage( attr, structType )`.
`.get( member )` reads one member.

```wgsl
// structs

struct DrawBuffer {
	vertexCount : u32,
	instanceCount : atomic< u32 >,
	firstVertex : u32,
	firstInstance : u32,
	offset : u32
};


// uniforms
@binding( 0 ) @group( 0 ) var<storage, read_write> NodeBuffer_0 : DrawBuffer;
```

These are three's spellings, reproduced exactly: the `// structs` section
of a compute module, a struct binding declared on one line, and
`atomic< u32 >` with spaces for a struct member. (An array of atomics is
`array< atomic<u32> >`, without them.) The layout has to be all 4-byte
members whose count matches the attribute's `item_size`; `storage_struct`
asserts both, because a mismatch would be a buffer the kernel reads past.

### 33.3 Atomics

`atomic_store`, `atomic_add`, `atomic_sub`, `atomic_max`, `atomic_min`,
`atomic_and`, `atomic_or`, `atomic_xor` and `atomic_load` take a pointer node:
an atomic struct member, an element of `StorageArray::to_atomic()`, or an
element of `WorkgroupArray::to_atomic()`. `AtomicFunctionNode` has two
spellings, and the port follows its rule:

* a call that is itself a statement of the flow, and is not used anywhere
  else, is a bare `atomicAdd( &…, 1u );`. This is three's
  `parents[ 0 ].isStackNode` test, which the builder answers with the key of
  the statement it is generating (`generate_statement`);
* anything else is a `let nodeConstN = atomicAdd( … );` whose name is the
  value.

A float value stored into a `u32` atomic is wrapped in `u32( … )`, as three's
`AtomicFunctionNode` does.

### 33.4 Workgroup memory and the compute builtins

`workgroup_array( ty, count )` is `workgroupArray( type, count )`. It declares
`var<workgroup> WorkgroupArray_N: array< u32, 64 >;` under `// locals`, and
`.element( i )` indexes it. `workgroup_barrier()` and `storage_barrier()` are
the bare statements. `invocation_local_index()`, `workgroup_id()`,
`local_id()`, `global_id()` and `num_workgroups()` are the compute builtins.
A kernel that reads `invocationLocalIndex` gets
`@builtin( local_invocation_index ) invocationLocalIndex : u32` first among
its entry point's parameters, where three puts it. Reading any of them
outside a compute kernel is a panic that names it.

The storage buffers of a stage are now declared before its uniform structs,
which is three's order (`m04` above has the struct binding, then
`objectStruct`). Every earlier WGSL gate still passes with it.

### 33.5 The varying a vertex stage reassigns

`positionLocal` is a varying (§1), and `setupPosition()` assigns
`positionNode` to it. Three's vertex stage writes every assignment to a
varying straight into `varyings.positionLocal`. The port's vertex stage holds
the value in a private var until the fragment stage asks for the varying. It
then wrote `varyings.positionLocal = position`, the unmoved geometry position.
It now writes the var's current value when the vertex stage has assigned to
it (`reassigned_varyings` in the builder). `webgpu_particles`' smoke colour
reads `positionLocal.y`, which is how this came up.

### 33.6 What the images cannot grade

Both rungs graded here have frames that do not show the feature.
`webgpu_struct_drawindirect` draws before its kernels run, so its graded
frame is the background. `webgpu_particles` is pinned to a time at which
every sprite's opacity is 0. Their progress notes list the WGSL and GPU
gates that stand in for the image. `tests/renderer_compute_indirect.rs` also
checks atomics, workgroup memory and an indirect dispatch in small kernels
whose answers are known in closed form.

`webgpu_compute_reduce`, the issue's third candidate, is not a rung. Its page
runs two renderers on two half-width canvases under a DOM thread display, and
its later reductions use `subgroupAdd`, which #167 leaves out with the rest
of the subgroup functions. Its workgroup-memory kernels are the source of
the spellings §33.4 asserts.

`texture.sample( uv ).offset( o ).gather( c )` is `SampleMode::Gather {
component, offset }` (`tsl::texture_gather`), and the same on a
`DepthTexture` with `.compare( z )` is `SampleMode::GatherCompare { compare,
offset }` (`tsl::depth_texture_gather_compare`), emitted as
`generateTextureGather()` / `generateTextureGatherCompare()` write them:
`textureGather( c, t, t_sampler, uv, offset )` and `textureGatherCompare( t,
t_sampler, uv, z, offset )`, the component built as an `int`, the offset as
an `ivec2` and the reference as a `float`. Both read mip level 0 and ignore
the texture's filters. Three drops the space before the closing parenthesis
when there is no offset, and so does the port. The depth form binds the
comparison sampler `shadow_map_compare` binds (`LessEqualCompare`), the one
`compareFunction` `webgpu_texturegather` sets; the port's `DepthTexture`
carries no `compareFunction` of its own.

One quirk kept: `TextureNode.generate()` types a gather's snippet from
`texture.type`, and a `DepthTexture` is `UnsignedIntType` by default, so three
takes the compare's result for a `uvec4` and formats it into the node's
`vec4`, writing `vec4<f32>( textureGatherCompare( … ) )` — a no-op cast. The
port writes the same cast when the depth texture's type is `UnsignedInt`.
`webgpu_texturegather`'s `m04` fragment matches from `// flow` to
`DiffuseColor = ` after renumbering
(`tests/nodes_texture_wgsl.rs::texturegather_fragment_matches_three`).

### Divergences specific to this section

The two new classes are in §8: one flat `instanceIndex` varying per
`range()`, and `varyings.positionLocal` written once from the var. Compute
modules also have the subgroup omission and the banner that §8 already
lists. Apart from those, both of `webgpu_struct_drawindirect`'s kernels
match three's dump line for line once generated names are renumbered.

[`Renderer::draw`]: ../src/renderer/mod.rs
[`materials::transmission`]: ../src/materials/transmission.rs
[`tsl::with_tangent_attribute`]: ../src/nodes/tsl.rs
[`tsl::bent_normal_view`]: ../src/nodes/tsl.rs
[`Scene::background_blurriness`]: ../src/objects/scene.rs

## 34. `webgpu_clearcoat` — the direct clearcoat lobe, and the coat's own normal

Four `MeshPhysicalMaterial` spheres with `clearcoat = 1`, the Pisa HDR cube as
background and (PMREM-filtered) environment, and one `PointLight` of intensity
30. It is the first graded page that lights a clearcoat: every earlier
clearcoat on the ladder (the barn lamp, §26) sat in a scene with no lights, so
only the coat's indirect half had ever run. Issue #171.

| sphere | base | coat normal | three's program |
| --- | --- | --- | --- |
| car paint | blue, metalness 0.9, `FlakesTexture` normal map at 0.15 | none | `m10` |
| fibers | carbon colour map and normal map, tiled 10× | none | `m12` |
| golf | golf-ball normal map | `Scratched_gold` normal map, scale `( 2, -2 )` | `m14` |
| red | water normal map at 0.15, metalness 1 | the same `Scratched_gold` map | `m14` |

### 34.1 `direct()`'s clearcoat branch

`PhysicalLightingModel.direct()` runs, between the sheen block and the base
lobes,

```js
const dotNLcc = clearcoatNormalView.dot( lightDirection ).clamp();
const ccIrradiance = dotNLcc.mul( lightColor );
this.clearcoatSpecularDirect.addAssign( ccIrradiance.mul( BRDF_GGX( { lightDirection,
    f0: clearcoatF0, f90: clearcoatF90, roughness: clearcoatRoughness,
    normalView: clearcoatNormalView } ) ) );
```

with `clearcoatF0 = vec3( 0.04 )` and `clearcoatF90 = 1`. The port's
`BRDF_GGX` used to read `roughness` and `normalView` directly; it is now
`physical::brdf_ggx_on( L, f0, f90, roughness, normal )`, with `brdf_ggx` the
default-argument call. `finish()` already blended `clearcoatSpecularDirect` in
through `Fcc`; the accumulator had simply never been written.

`irradiance` is a var (`nodeVar4` in `m10`) whenever clearcoat is on, as it is
with sheen: in three it is always `.toVar()`, and the port keeps it inline only
where no dump shows the var.

### 34.2 `clearcoatNormalView` without a clearcoat normal map is the *geometric* normal

`MaterialNode.CLEARCOAT_NORMAL` is `normalView` when the material has no
`clearcoatNormalMap`, but it is evaluated inside the `NORMAL` sub-build, where
`normalView` is `NORMAL_normalView` — the normal *before* the material's own
normal map. So the coat is smooth over a bumpy base: `m10` and `m12` both read
`clearcoatNormalView = NORMAL_normalView;`. The port had it as the outer,
normal-mapped `normalView`, which was invisible until a sphere had a normal
map and a coat without one; on this page it put the carbon weave into the
coat's reflection and highlight (141 pixels over the limit). Fixed in
`tsl::clearcoat_normal_view`. The barn lamp has a clearcoat normal map, so it
never took this branch.

### 34.3 `FlakesTexture`

`examples/jsm/textures/FlakesTexture.js` paints 4000 random round flakes on a
2D canvas. The port rasterises the same fills into RGBA8
(`addons::textures::FlakesTexture`), drawing from the grader's seeded
`Math.random()` after the inspector's five draws — the flakes are made in the
HDR loader's callback, after `init()` has built the inspector. The fill colour
is CSS-rounded, as the canvas parses it. The flake rim is antialiased with a
one-pixel distance ramp, the only approximation: against the canvas Chrome
paints (read back from the page, not from the screenshot) every flake interior
is exact and the mean difference is 0.7 levels in 255.

### 34.4 What the pixels found

| state | different pixels (of 100000) |
| --- | --- |
| direct lobe, coat normal = normal-mapped `normalView` | 141 |
| direct lobe, coat normal = `NORMAL_normalView` | **4** |

### 34.5 Divergences

* **Where `clearcoatNormalView` is assigned.** Three assigns it at its first
  read, inside the light loop after `irradiance`; the port assigns it right
  after `normalView`, before the loop. Both are straight-line assignments of
  the same value ahead of every read. Listed in §8.
* **`irradiance` after the clearcoat statement.** The port emits the
  `irradiance` var's assignment after `clearcoatSpecularDirect +=` rather than
  before it; nothing in the clearcoat statement reads it.
* Otherwise `m10`, `m12` and `m14` match `dump_wgsl`'s `clearcoat_car_paint`,
  `clearcoat_fibers` and `clearcoat_golf` statement for statement, modulo the
  naming classes §8 lists (`let nodeConstN` vs `nodeVarN`, uniform numbering).

## 35. Curves, shapes and `Flow` (`webgpu_modifier_curve`, issue #170)

`src/extras/` ports three's `Curve` family, `Path` / `Shape` / `ShapePath`,
`ShapeUtils` and Earcut. `src/geometries/` adds `ShapeGeometry`,
`ExtrudeGeometry` and `TubeGeometry`. On top of those sit `FontLoader` /
`TextGeometry` and `CurveModifierGPU`'s `Flow`. The geometry side is all `f64`,
in three's expression order, and `tests/geometries_shape_oracle.rs` checks it
against three's own output (from `tools/geometry_reference.mjs`) to 1e-6. The
spline texture is checked half-float for half-float. The node side adds only
`NodeRef::remap` (a `RemapNode` without `doClamp`, unfolded as three emits it)
and `Mesh::count`.

### 35.1 `Flow`'s shader

`positionNode` is three's `Fn()`, written as a `block` that assigns `mt` and
the `curveNormal` varying and then yields the bent position. `normalNode` is
`varyingProperty( 'vec3', 'curveNormal' )`. The generated WGSL matches three's
dump apart from `var` placement: the port declares `worldPos` inside the
`select` branch and again later. It is cosmetic and changes no value, like
§8's "Declaration order" and "Property-assignment temps". The texture reads are `textureSampleLevel( …, 0 )` in both.

### Divergences specific to this section

* **No arc-length cache on the leaf curves.** `Curve.getLengths()` memoises
  into `cacheArcLengths`. The port's leaf curves take `&self` and recompute
  on every call. The result is identical and only the cost differs.
  `CurvePath` keeps both of three's caches (see the next item).
* **`CurvePath`'s stale arc lengths, reproduced.** `cacheArcLengths` survives
  adding a curve, in three as here. `getPointAt` after a later `lineTo`
  therefore maps through the old path's lengths until `updateArcLengths()` is
  called.
* **`CurvePath::get_point` panics past the end** where three returns `null`.
  `ShapePath` likewise panics where three would throw on a `null`
  `currentPath`.
* **Frenet frames of a 2D curve are planar (`z = 0`).** three copies a
  `Vector2` tangent into a `Vector3`, which leaves `z` `undefined`, and the
  frames come out `NaN`. Nothing depends on them.
* **`TubeGeometry` takes 3D paths only.** Given a 2D path, three fills its
  buffers with `NaN`.
* **`UvGenerator` has no geometry argument.** three passes the half-built
  geometry first. `WorldUVGenerator` ignores it, and the port has nothing to
  read from it.
* **`Mesh::count`.** In three this is not a `Mesh` property. A page sets it
  ad hoc, and `RenderObject.getInstanceCount()` reads it off any object. Here
  it is an `Option<usize>` field, where `None` is `undefined`.
* **`Flow`'s `wrapY` typo, reproduced.** Upstream sets `wrapS` and then
  `wrapY` (not a `Texture` property), so the spline texture repeats in `s` and
  clamps in `t`. The port does the same.
* **`Flow` bends one mesh.** Upstream clones an `Object3D` and patches every
  `Mesh` / `InstancedMesh` under it. The port has no generic scene-graph
  clone, so `Flow::new` takes a geometry and a material and builds the single
  mesh. That covers every use the examples make of it.
* **`arcLengthDivisions` through an adapter.** `updateSplineTexture` mutates
  the caller's `curve.arcLengthDivisions`. The port's curves have no such
  field (`CatmullRomCurve3` has the trait default of 200), so a private
  `ArcLengthDivisions` wrapper presents the curve with 512 divisions instead.
  The caller's curve is left unchanged afterwards, where three's keeps 512.
* **A glyph the font lacks, with no `?` to fall back on, is skipped.** three
  logs and then throws on `ret.offsetX`.
* **No `TransformControls`.** `webgpu_modifier_curve`'s handles cannot be
  dragged. The graded frame never shows the gizmo.

## 36. `NodeCache`: one parent-chained cache for per-build data (issue #156)

Two rungs taught the builder the same rule on their own. The display nodes
(#148) found that a `Block` read twice, as `renderOutput()` reads its colour's
`.xyz` and `.w`, emitted its statements twice. They also found that each
nearest `textureLoad` declared its own `textureDimensions` var where three
shares one per texture per scope. MaterialX (#151) found the first of these
again in the raging sea's `elevation`, one inlined `Fn()` read by
`emissiveNode` and by `normalNode`. Both fixes wrote into the var cache,
which was a `Vec<HashMap<usize, String>>` per stage and another per layout
`fn`. The texture-size entry was keyed by hashing `("textureDimensions",
name)` into the same `usize` space as node addresses.

The builder now has three's shape instead. `NodeCache` (`NodeCache.js`) is a
map with an optional parent: `get` falls through to the parent, and `set`
writes only to the cache it is called on.

* **What it holds.** The key is a `CacheKey` enum. `Node(key)` is a node's
  generated snippet: the var, `let`, `if` result or block result it left.
  `TextureDimensions(name)` is `generateTextureDimension()`'s
  `textureData.dimensionsSnippet` for one texture binding. The two can no
  longer collide.
* **Where it lives.** Each stage owns one, and so does each layout `fn` body.
  A `fn` body's cache has no parent, because it sees nothing of its caller.
* **Scopes are children.** Opening a block (an `If` arm, a loop body, a
  `select` arm) makes the current cache a child of itself, and closing the
  block restores the parent. A snippet built before the arm is visible inside
  it. One built inside the arm is visible to the rest of the arm and not
  after it. A sibling arm builds its own. This is what the scope stack did
  before; the change is only in the shape.

This is step (1) of #155's plan. `isolate( node )` will be a node that swaps
in a child with or without a parent (`getCacheFromNode( node, parent )`),
and `subBuild` layers will join the key. Neither is added here, because no
rung needs them yet.

Nothing generated changed. `dump_wgsl`'s output is byte-identical before and
after, apart from one pointer printed in a `Debug` of an `ObjectUpdate`
closure. Every `tests/nodes_*` gate passes unchanged, and so does the full
ladder.

## 37. View offsets on both cameras, and render-pipeline hooks (issue #164)

The groundwork TRAA (#165) needs, from #154 decisions 2 and 3. No rung.

* **`set_view_offset` / `clear_view_offset` on `RenderCamera`.** Both of
  three's cameras have them, so a hook or pass node can jitter whichever
  camera it holds. `OrthographicCamera` gains `view: Option<CameraView>` and
  three's `updateProjectionMatrix()` branch: `scaleW = ( right - left ) /
  fullWidth / zoom`, then the window's planes. Unlike the perspective camera,
  the orthographic one leaves `aspect` alone because it has none.
  `tests/cameras_orthographic_camera.rs` ports three's QUnit file. That file
  has no view-offset case, so the offset tests compare against matrices three
  printed under node, and they match bit for bit.
* **`RenderPipeline::on_before_render` / `on_after_render`.** These are
  three's `OnBeforeRenderPipeline` / `OnAfterRenderPipeline`, typed
  `Box<dyn FnMut(&mut Renderer)>`. Before-hooks run after the output node is
  reassigned and before the renderer's tone mapping is neutralised.
  After-hooks run once it is restored. Within each list, hooks run in the
  order they were added. `tests/renderer_pipeline_hooks.rs` (GPU) checks the
  order against the draw count.
* **Divergence: hooks outlive a rebuild.** three collects the callbacks from
  `EventNode`s while the quad material builds, into a context that
  `_updateContext()` recreates, so a new `outputNode` drops them. The port has
  no builder context to collect into. The node that needs a hook adds it when
  it is built, and it stays for the pipeline's life.
* **`SsaaPassNode` is unchanged.** It jitters once per sample inside its own
  `render`, not once per pipeline render, so the hooks do not fit it. It
  still reads `PerspectiveCamera.view` directly.

## 38. `BuildContext`: one stack for `builder.context` (issue #160)

Eight thread-locals in `tsl.rs` held `builder.context` one key at a time.
Each had its own `with_…` function that swapped a value in and restored the
old one afterwards. #155 §2 lists them: `SUB_BUILD`, `OVERRIDE_NODES`,
`NORMAL_VALUE`, `FLAT_SHADING`, `MATERIAL_SIDE`, `HAS_TANGENT`,
`POSITION_VIEW_VALUE` and `CLEARCOAT_NORMAL_VALUE`. They are now fields of
one struct, `BuildContext` in `src/nodes/builder.rs`, kept on one stack.

| was | `BuildContext` field | three |
|---|---|---|
| `SUB_BUILD` | `sub_build` | `builder.subBuildLayers` (one layer deep) |
| `OVERRIDE_NODES` | `override_nodes` | `context.overrideNodes` (§27) |
| `NORMAL_VALUE` | `setup_normal` | `context.setupNormal` (§7) |
| `FLAT_SHADING` | `flat_shading` | `builder.isFlatShading()` |
| `MATERIAL_SIDE` | `material_side` | `builder.material.side` |
| `HAS_TANGENT` | `has_tangent` | `builder.geometry.hasAttribute( 'tangent' )` |
| `POSITION_VIEW_VALUE` | `setup_position_view` | `context.setupPositionView` |
| `CLEARCOAT_NORMAL_VALUE` | `setup_clearcoat_normal` | `context.setupClearcoatNormal` |

Core keys are typed fields. Addon keys will go in `extra`, a
`HashMap<&'static str, NodeRef>` (#155 decision 6), which nothing reads yet.
Three of the fields are not `builder.context` keys in three: the layer, the
flat-shading flag and the tangent flag live on the builder, its material and
its geometry. They are here because they have the same lifetime and the same
readers.

`push_context( |cx| … )` is `ContextNode`'s setup. It copies the top entry,
lets the caller change the keys it sets, pushes the copy and returns a
`ContextGuard`, which pops it when dropped. `current_context( |cx| … )` reads
the top entry, or the default one when nothing has been pushed. The `with_…`
functions keep their signatures and are each a push and a call now. The
change is that one stack holds all the keys, so a scope restores all of them
in one pop, and #161's `context( node, { … } )` has a place to push to.

**The stack is a thread-local beside the builder, not a field of it.** Three
calls `NodeMaterial.setup()` from inside `builder.build()`, so the context
object can live on the builder. The port builds the flow in
`materials::setup()` first and creates the `NodeBuilder` afterwards
(`Renderer::node_builder_state`), so there is no builder to hold it while the
keys are being read. Merging the two phases is a bigger change than this
issue. The stack is empty between material setups, and because the guard
pops on drop, a panic inside one setup can no longer leave its keys
installed for the next build on that thread.

**Not moved.** The accessor memo maps (`NORMAL_VIEW`, `TANGENT_VIEW`,
`NORMAL_WORLD`, `POSITION_VIEW`, `CLEARCOAT_NORMAL_VIEW`, …) and the
singleton `Lazy` cells stay thread-locals. They are process-wide
memoisation keyed on context values, not context: they give two builds with
the same context the same node, as three's per-build `nodeData` gives one
build one node. #155 proposes keying them on a hash of the context; that is
left for when `context()` can install arbitrary keys.

Nothing generated changed. `dump_wgsl`'s output is identical after each of
the eight migrations, apart from the one `ObjectUpdate` pointer noted in §36.
Every `tests/nodes_*` gate passes unchanged, and so does the full ladder.

## 40. A user `LightingModel` and `ArrayCamera` (`webgpu_lights_custom`, `webgpu_camera_array`)

### 40.1 `LightingModel` as a trait

`webgpu_lights_custom` subclasses `THREE.LightingModel` and hands the instance
to a material through `lights( [ … ] ).context( { lightingModel } )`.
`src/materials/lighting_model.rs` ports the base class as a trait:

* `start()`, `direct()`, `indirect()` and `finish()` default to three's base
  bodies. `start()` runs `builder.lightsNode.setupLights()`, which calls
  `direct()` once per direct light, and then calls `indirect()`.
* three's methods append to the builder's current stack implicitly. The
  port's methods push onto `LightingBuilder::stack` instead, and that stack is
  spliced into the material's fragment statements.
* `ReflectedLight`'s four accumulators are `vec3().toVar( name )`, as in
  `LightingContextNode`. Each one is therefore declared where it is first
  read, which is why `indirectDiffuse` is zeroed only in `totalDiffuse`'s
  line in three's dump as well as ours.
* `lights_node()` is `LightsNode.setup()`: `start()`, then the
  `totalDiffuse` / `totalSpecular` / `outgoingLight` tail, then `finish()`.

The material field `lighting_model` is the `lightingModel` from the
context. `LightingContextNode.setup()` reads `this.lightingModel ||
builder.context.lightingModel`, so it only takes effect on kinds with no model
of their own (Points, Sprite, and Basic standing in for a bare
`NodeMaterial`). The built-in models stay the `match` on `MaterialKind`. The
trait is how a model the port does not ship gets in; it does not replace the
built-in path.

Not ported: `directRectArea()`, because the port has no `RectAreaLight`, and
`ambientOcclusion()`, which is never called on a user model.

### 40.2 `ArrayCamera`: how three does it

With an `ArrayCamera`, `Camera.js` turns `cameraViewMatrix` and
`cameraProjectionMatrix` into
`uniformArray( matrices ).setGroup( renderGroup ).element( cameraIndex )`.
`cameraIndex` is a `u32` uniform in its own `sharedUniformGroup(
'cameraIndex' )`, read in the fragment stage through a flat `v_cameraIndex`
varying. That group takes `@group(1)`, and the object group moves to
`@group(2)`.

`WebGPUBackend.draw()` then loops over the sub-cameras for every object. For
each one it calls `setViewport( floor( vp * dpr ) )`, binds that
sub-camera's prebuilt `cameraIndex` bind group, and issues the draw. `dpr` is
the renderer's pixel ratio for the canvas and 1 for a user render target.
Culling goes through a `FrustumArray`: an object is drawn if any sub-camera's
frustum holds it, and it is then drawn for every sub-camera.

### 40.3 `ArrayCamera`: the port

* **Substitution at build time.** `NodeBuilder::with_array_cameras( n )`
  swaps the `CameraViewMatrix` / `CameraProjectionMatrix` uniform nodes for
  `BufferElement` nodes in `analyze()` and `generate()`. It does not rebuild
  them in `tsl`: the TSL singletons (`camera_view_matrix()` and the nodes
  built on it, such as `positionView` and `modelViewMatrix`) are cached
  process-wide and already hold the plain uniform. The element is
  `BufferSource::CameraViewMatrices[ v_cameraIndex ]`, where the varying wraps
  a `UniformGroup::CameraIndex` uniform.
* **Bind groups.** `UniformGroup::ORDER` is `[Render, CameraIndex, Object]`.
  Group numbers count the groups in use, so pages without an array camera
  keep `Render = 0, Object = 1` byte for byte. The two matrix arrays sit in
  the render group under three's fixed names `cameraViewMatrices` and
  `cameraProjectionMatrices`, not `NodeBuffer_N`.
* **The cache key.** `SetupContext::array_cameras` is the sub-camera count,
  so a material drawn through both kinds of camera builds two programs. The
  shadow passes set it to 0.
* **Draws.** `Renderer::sub_camera_draws()` makes one 16-byte slot buffer and
  bind group per sub-camera index (`SlotOwner::CameraIndex(i)`). `record_pass`
  sets the pipeline, bind groups and vertex buffers once. Then, for each
  sub-camera, it sets the viewport, rebinds the `cameraIndex` group and calls
  `issue_draw()` (the old draw body).
* **Culling.** `ProjectCamera::sub_frustums` is `FrustumArray`: an object is
  kept if any sub-frustum holds its bounding sphere.
* **`camera.viewport`.** `PerspectiveCamera::viewport` is
  `Option<Vector4>`, in CSS pixels with a top-left origin as in three.
  `ArrayCamera` is a base `PerspectiveCamera` (`Deref`) plus `cameras`.
  `RenderCamera::sub_cameras()` defaults to `&[]`.

### 40.4 Divergences

* **Binding numbers of the matrix arrays.** Three gives each stage its own
  `cameraViewMatrices` binding (the fragment's at 1, the vertex's at 2 and
  3). The port declares each array once per program. The WGSL indexes them
  the same way.
* **`v_cameraIndex` in the vertex stage.** Three reads
  `varyings.v_cameraIndex` back in the vertex stage. The port reads a private
  `v_cameraIndex` it assigned from the same uniform, which gives the same
  value. Varying locations are ordered differently, as in §8.
* **`subcamera.copy( camera )` is reproduced as the page wrote it.** It
  copies the `ArrayCamera`'s own defaults (fov 50, far 2000), not the sub-camera's
  constructor's 40 / 10. The example does the same, and the frame is
  pixel-identical to three's `actual_full.png`.
* **No XR, bundles or layer textures.** `WebGPUBackend`'s array-texture
  (multiview) path and its render-bundle path are not ported.
  `object.layers.test( subCamera.layers )` always passes, because the port
  has no layers.

## 41. Sprites: `userData`, `Sprite.count`, and fog in the material's scope (`webgpu_sprites`, `webgpu_instance_sprites`)

Both pages draw `SpriteNodeMaterial`, which the port already had for the
galaxy and the particles (§33). What they add is small, but one piece of it
is a correction to how every fog node was built.

### 41.1 `userData( name, type )`

`webgpu_sprites` gives two hundred `Sprite`s one material and sets
`material.rotationNode = userData( 'rotation', 'float' )`. Each sprite's
`userData.rotation` grows by a different step each frame. `UserDataNode` is a
`ReferenceNode` whose reference is `frame.object.userData`, with
`updateType = OBJECT`: an object-group `uniform()` rewritten before each draw.
The port already had that shape as `uniform_object` (§19), so
`tsl::user_data( name, ty )` is `uniform_object( ty, |object| … )` reading
`object.user_data[ name ]`. `Object3D.user_data` is three's open `userData`
object, a `serde_json::Map`. It is deep-copied on clone, as three's
`JSON.parse( JSON.stringify( … ) )` copies it. All two hundred draws share one
program and one pipeline, and each writes its own float into
`object.nodeUniform4` (`nodeUniform3` in the port's numbering).

### 41.2 Fog is built inside the material

`Fog.js`' factors are `Fn()`s. `rangeFogFactor( near, far )` reads
`getViewZNode( builder )`, which is `positionView.z` read **while the
material is being built**. `positionView` is
`builder.context.setupPositionView()`, and for a sprite that is
`SpriteNodeMaterial.setupPositionView()`, the billboarded `vec4`. Three's
sprite fragment therefore fogs by `v_positionView.xyz.z` of the one sprite
varying.

The port built the fog graph eagerly, when `scene.fogNode` (or `scene.fog`'s
cached node) was made. That is outside any material, so the factor held the
base class' `modelViewMatrix * positionLocal` varying. A sprite then carried
a second `v_positionView` (a `vec3`, of the un-billboarded quad corner), and
fogged by it. At `webgpu_sprites`' pinned time the group is not rotated, so
the two depths agreed and the frame was already at 0 pixels. They part as
soon as the group turns.

`range_fog_factor`, `density_fog_factor` and `exponential_height_fog_factor`
now return an inline, argument-less `Fn()` call. `NodeMaterial` setup runs
its body, through `tsl::resolve_fog_factor`, inside the material's
`with_material_position_view` scope, where `position_view()` is the
material's own. For a mesh, the scope's `positionView` is the base class' pair,
cached per context, so the node is the one it always was. The ladder's fog
rungs are unchanged to the pixel. A factor used anywhere else is inlined by
the builder with the base class' `positionView`, which is what it read
before. The `_with_view_z` forms (§24) are unchanged: their view z is
explicit.

### 41.3 `Sprite.count`

`webgpu_instance_sprites` draws ten thousand snowflakes as one `Sprite` with
`count = 10000`. `positionNode` is an `instancedBufferAttribute` and
`rotationNode` is `time.add( instanceIndex ).sin()`. `Sprite` gains three's
`count` (default 1), and `Payload::count()` returns it, which is
`RenderObject.getInstanceCount()`. The rest was in place: the instanced
attribute's `stepMode: 'instance'` buffer, `instanceIndex` cast to `f32` in
the add, `alphaMap` with its `vec4` product (§32.4), and `FogExp2`'s render-group
uniforms (§28).

### Divergences specific to this section

* **`userData` reads a flat key on the render object.** three's
  `ReferenceNode` walks a dotted path and also accepts an explicit
  `userData` object in place of the render object's. The ladder uses neither.
  A number or numeric array is read as the uniform's components. Anything
  else, or a missing key, reads as zero, where three would write `undefined`.
* **One uv-matrix uniform per texture, not per `texture()` node.**
  `webgpu_instance_sprites` samples the snowflake twice, as `map` and as
  `alphaMap`. Three's dump has two `mat3` members, `nodeUniform3` and
  `nodeUniform5`, one per `TextureNode`. The port's `transformed_uv` keys the
  member on the texture, so both samples read one member. The values are the
  same matrix, so no pixel can differ.
* The rest of the two pages' WGSL differs from the dumps only in §8's
  classes: `let nodeConstN` against `var nodeVarN`, uniform numbering and
  member order, and `@location` order. `dump_wgsl`'s `sprites` and
  `instance_sprites` sections print it.

## 42. `MeshToonNodeMaterial` and `toonOutlinePass` (`webgpu_materials_toon`)

**The lighting model.** `ToonLightingModel` extends `LightingModel` directly,
but its `indirect()` is Lambert's line for line: ambient irradiance times
`BRDF_Lambert`, multiplied by AO. The port therefore runs `MaterialKind::Toon`
through `setup_phong()`'s Lambert arm (`specular = false`). Only the per-light
term changes. `toon::direct_light()` replaces Lambert's `saturate( dotNL )`
with `getGradientIrradiance()`:

* **With a `gradientMap`.** The irradiance is `vec3( gradientMap.r )`,
  sampled at `vec2( dotNL * 0.5 + 0.5, 0 )`. Three reaches the map with
  `materialReference( 'gradientMap', 'texture' ).context( { getUV } )`.
  `getUV` replaces only the default `uv()`, and the texture node's
  `updateMatrix` stays on, so the coordinate still goes through the map's uv
  matrix. `tsl::texture_with_uv()` is that `.context( { getUV } )`: it is
  `texture()` with the uv argument given, and it uses the same
  `transformed_uv` and the same one uniform per texture.
* **Without one.** It is the two-step ramp `mix( vec3( 0.7 ), vec3( 1 ),
  smoothstep( 0.7 - fw.x, 0.7 + fw.x, coord.x ) )` with `fw = fwidth( coord )
  * 0.5`. No rung draws this branch yet.

**The gradient maps.** The page builds each ramp as `new DataTexture( colors,
n, 1, RedFormat )`. `Texture::data_r8()` is that texture: `R8Unorm`,
`flipY` false, no mipmaps, and `NearestFilter` for both min and mag. The last
of these makes it unfilterable, so its tap is a `textureLoad` with the
`tsl_coord_clampS_clampT_2d` helper, as three's dump has it. The page's loop
writes one byte past the end of the array, and that store is dropped. The
port simply builds `n` bytes.

**`toonOutlinePass`.** Three's `ToonOutlinePassNode` is a `PassNode`. Its
`updateBefore()` swaps the renderer's render-object function for one that
draws a toon object with its outline material, then with its own material.
The port's `ToonOutlinePassNode` wraps a `PassNode` and makes the same swap
through the renderer's `toon_outline` field. The render loop checks that
field at the point where three calls the function. For every draw whose
material is `MaterialKind::Toon`, it pushes a copy of the draw with the
outline material immediately before it. The outline material is three's
`_createMaterial()`: a back-side node material whose `vertexNode` pushes each
clip-space vertex along `normalize( pos - mvp * ( positionLocal -
normalLocal ) )` by `thickness * pos.w`, with `colorNode = vec4( color, alpha
)`.

The divergences:

* **One outline material, not one per toon material.** Three caches one
  outline material per source material (`_materialCache`). Every one of them
  is built from the same three nodes, so they differ only in identity. The
  port keeps the one template. Each outline draw's program key is the source
  material's key with a `VARIANT_TOON_OUTLINE` variant hashed with the
  template's id, which is the way shadow materials are keyed. This keeps the
  steady frame building nothing.
* **No wireframe case.** Three skips the outline for a
  `material.wireframe` toon material. The port's materials have no wireframe
  mode, so there is nothing to skip.
* **The outline is a `Basic` material.** Three's is a bare `NodeMaterial`
  with `lights = false`. The port's `Basic` kind with its default `lights =
  false` emits the same fragment flow (`m03`).
## 39. Occlusion queries and `frame.renderer.isOccluded()` (`webgpu_occlusion`)

A green Phong plane in front of a yellow Phong sphere. The sphere carries
`occlusionTest = true`, and the plane's `colorNode` is a custom node with
`updateType = NodeUpdateType.OBJECT`, as §19's is, whose `update( frame )`
asks the renderer:

```js
async update( frame ) {
    const isOccluded = frame.renderer.isOccluded( this.testObject );
    this.uniformNode.value.copy( isOccluded ? this.occludedColor : this.normalColor );
}
```

### What three does

`RenderList.push()` counts the runs of consecutive pushes of an object with
`occlusionTest`, and `beginRender()` creates an occlusion `GPUQuerySet` of that
size for the render context's pass. `WebGPUBackend.draw()` walks the draws with
`lastOcclusionObject`: where the object changes it ends the open query (if the
last object was tested, bumping `occlusionQueryIndex`) and begins one if the new
object is tested, recording the object at that index. `finishRender()` ends the
last one, resolves the set into a `QUERY_RESOLVE` buffer (cached by size),
copies it into a fresh `MAP_READ` buffer and calls `resolveOccludedAsync()`,
which maps **the previous `finishRender()`'s buffer**, not this one. When the
map lands, every object whose query counted zero samples goes into a `WeakSet`,
`renderContextData.occluded`, and `isOccluded( object )` reads that set for
`_currentRenderContext`.

So an answer reaches `update()` two frames after the draw it measured, at the
earliest. The graded frame is the first, and the plane is blue on it. Three's
screenshot shows exactly that, and so does the port.

### What the port has

| three.js | three-rs |
|---|---|
| `object.occlusionTest` (ad hoc) | `Object3D::occlusion_test` |
| `renderContextData.occlusionQuerySet` / `occlusionQueryBuffer` / `occluded` | `renderer::occlusion::OcclusionContext` |
| `occludedResolveCache` | `Occlusion::resolve_buffers` |
| `lastOcclusionObject` walk in `draw()` / `finishRender()` | `record_pass()`, counted by `occlusion::query_objects` |
| `resolveOccludedAsync()` | `Occlusion::finish()` issues the map; `Occlusion::collect()` reads it |
| `frame.renderer.isOccluded( object )` | [`NodeFrame::is_occluded`](../src/nodes/node.rs) |
| `Node` with `updateType = OBJECT` reading more than `frame.object` | [`tsl::uniform_frame( ty, \|frame\| … )`](../src/nodes/tsl.rs) |

`ObjectUpdate` now calls its callback with a `NodeFrame` (the object, plus the
render context's occlusion results) rather than a bare `&Object3D`.
`uniform_object` is unchanged for its callers: it wraps its closure to read
`frame.object`.

The render context is set by `render()` just before `render_list()`, and the
next `draw()` takes it, which is the scene pass. The shadow passes and the
output blit are draws of their own and never see it, as in three, where each is
its own render context.

The map's callback cannot touch the renderer, because natively it runs from
inside `device.poll()` and must be `Send`. It sets an atomic, and `render()`
polls without blocking whenever a map is outstanding and folds whatever has
landed into `occluded` before any draw is built. A renderer with no query in
flight never polls, so no other rung sees any of this.

`tests/renderer_occlusion.rs` checks what the graded frame cannot: the plane is
blue on frames one and two and green from frame three on, and it stays blue
for six frames once the sphere is moved in front of it.

### Divergences specific to this section

* **`occlusion_test` is a field.** §19 declined to add `mesh.color` because
  only the page reads it. `occlusionTest` is different: it is ad hoc in three
  too, but the renderer reads it (`RenderList`, `WebGPUBackend`), so it is
  renderer API. Being ad hoc, `Object3D.copy()` does not carry it, and the
  port's `Clone` does not either.
* **The render context has no camera.** three keys `RenderContexts` on the
  scene, the camera, the render target, the MRT and the call depth. The port
  keys occlusion on the scene and the render target only: `RenderCamera` is a
  trait over owned camera structs with no shared identity. Two cameras
  rendering one scene into one target share their occlusion results here.
* **The query set is reused.** three creates a query set in every
  `beginRender()` and destroys the previous one. The port keeps one per render
  context and makes a bigger one only when the count outgrows it. By the time
  it is reused, the previous frame's resolve has been submitted, so the queries
  are the same. The `MAP_READ` buffer is still fresh per frame, as in three.
* **Queries are counted in draw order.** three sizes the set by
  `RenderList.occlusionQueryCount`, which counts runs in push order, and then
  indexes it in the sorted draw order, which can have more runs. The port
  counts in draw order, so the index cannot overrun the set.
* **A split pass records queries only in its first segment.** When a
  framebuffer copy (transmission's, or a viewport node's, §61) splits the
  scene pass, three records into every part. No page on the ladder puts an
  occlusion test on a split pass.
* **Answers land on a poll, not on the event loop.** Natively the map
  callback fires on the next `render()`'s non-blocking poll, which on this
  machine is always in time for the next frame. That is the soonest three
  could have it too.
* **No queries in the browser.** wgpu 30's WebGPU backend never copies
  `occlusion_query_set` into the `GPURenderPassDescriptor`, so the first
  `beginOcclusionQuery()` invalidates the encoder ("The occlusionQuerySet in
  RenderPassDescriptor is not set") and the frame is never submitted.
  `occlusion::supported()` turns the queries off on
  `Backend::BrowserWebGpu`, so `isOccluded()` is always false there. That is
  three's answer until the first query lands, so the graded frame is the same.
  In the browser, `webgpu_occlusion`'s plane stays blue. It comes back once
  wgpu forwards the set.

## 43. `webgpu_fog_height` and `webgpu_shadowmap_opacity` — height fog under a lit, instanced material; transmitted shadows and AgX

36.1–36.3 are `webgpu_fog_height`, 36.4–36.8 `webgpu_shadowmap_opacity`.


The page is the first graded use of `exponentialHeightFogFactor` (§28.2). It
needed nothing new in `src/`: the factor, `scene.fogNode`, a `color()`
background node, a lit `InstancedMesh` and damped `OrbitControls` were all
in place. The rung is here so that it stays that way.

### 43.1 What three does

`Fog.js`' height factor is

```js
const distance = height.sub( positionWorld.y ).max( 0 ).toConst();
const m = distance.mul( viewZ ).toConst();
return density.mul( density, m, m ).negate().exp().oneMinus();
```

so the fog is zero above the world height `height` and thickens with the
depth below it times the view distance. `density` and `height` are the page's
`uniform( 0.04 )` / `uniform( 2 )`: plain `uniform()`s, so object group, after
the material's own members. That differs from `scene.fog`'s factors, whose
`reference()` uniforms are set to the render group (§28.1).

### 43.2 Checked against

`dump_wgsl`'s `fog_height` section (an instanced Phong material, one
directional and one ambient light, the page's fog node) against three's `m02`
/ `m03`. The fog lines match three's exactly, including the two `let` constants
and where the uniforms sit. The rest matches up to the naming classes in §8.

### 43.3 Divergences specific to this section

* **The GUI's uniforms are plain values.** The page's `Inspector` writes
  `density.value` / `height.value` from sliders. The port builds them with
  `uniform_value`, which has no handle to write through, because nothing in
  the graded frame or the viewer writes them. `uniform_settable` is the
  writeable form if a host ever wants the sliders.

### 43.4 `webgpu_shadowmap_opacity`: what three does

Two transmissive dragons (`DragonAttenuation.glb`, the second a clone with
`attenuationColor = 0xff0000`) cast coloured shadows onto the cloth backdrop.
Three pieces were missing from the port:

* **`renderer.shadowMap.transmitted`.** `ShadowNode.setupShadow()` samples
  `shadowMap.texture`, the shadow pass's `rgba8` colour target, which is
  cleared to `( 0, 0, 0, 0 )`, at the receiver's shadow coordinate. It then
  tints the filtered factor by it:

  ```js
  const shadowColor = texture( shadowMap.texture, shadowCoord );
  shadowOutput = mix( 1, shadowNode.rgb.mix( shadowColor, 1 ), shadowIntensity.mul( shadowColor.a ) );
  ```

  In three's WGSL the colour is sampled before the frustum branch, and the
  PCF filter keeps its five vogel-disk taps inside it. The port is
  `ShadowMap::Transmitted { map, color }`, wrapping the planar or filtered
  map the light would have had. `phong::shadow_factor_transmitted` builds the
  receiver side. `Renderer::render_shadows` wraps each spot or directional map
  in it when `shadow_map_transmitted` is set.
* **`material.castShadowNode`.** `Renderer._getShadowNodes()` makes the
  override material's colour `vec4( castShadowNode.rgb, castShadowNode.a )`.
  That alpha is then multiplied by the map's and the `colorNode`'s alphas, as
  before. For a `vec3` node, `.a` is `vec4( node, 1 ).w`, which is what three
  emits. The port has `MeshBasicNodeMaterial::cast_shadow_node` and
  `shadow_material_for`.
* **`AgXToneMapping`.** `ToneMappingFunctions.js`' `agxToneMapping` is
  `tsl::agx_tone_mapping`, statement for statement: the Rec.2020 matrices,
  the log2 encoding and the polynomial contrast approximation. The matrices
  are written column-major, so their constants read like three's.

The page also needed `MeshPhysicalMaterial.thicknessMap`: the dragon's
`KHR_materials_volume.thicknessTexture`. `MaterialNode.THICKNESS` is
`thickness * thicknessMap.g`, and the glTF loader now reads the texture as a
data map.

### 43.5 `webgpu_shadowmap_opacity`: checked against

`dump_wgsl`'s `dump_shadowmap_opacity()` was checked against three's dump:

* **`shadowmap_opacity_cast_shadow` against `m01`.** It is three's line for
  line: the constant `mix`, the `vec4( c, vec4( c, 1 ).w )` join, then opacity.
* **`shadowmap_opacity_backdrop` against `m05`.** The colour sample comes
  before the branch, there are five taps, and it is the same double `mix`.
* **`shadowmap_opacity_output_agx` against `m09`.** It differs only in how
  the constant is spelled: `mat3x3( 9 scalars )` where three writes
  `mat3x3( vec3, vec3, vec3 )`. The values are identical.

The dragon's transmission shader (`m07`) is not in `dump_wgsl`, because it
needs the transmission pass's context. The graded pixels cover it.

### 43.6 Divergences specific to `webgpu_shadowmap_opacity`

* **The tinted factor is `.xyz`'d.** Three's `mix` above is a `vec4`, which
  makes the light's `vec3` colour widen to `vec4( color, 1 )`. Its direct
  lighting then runs in `vec4` and ends in `.xyz`, which is visible in `m05`.
  The port takes `.xyz` of the tinted factor and keeps the light path in
  `vec3`. The rgb values are the same.
* **Point-light shadows ignore `transmitted`.** Three would sample the cube
  target's colour. The port's `ShadowMap::Cube` hands receivers only the
  depth cube, and nothing on the ladder asks for more.
* **No `shadow.autoUpdate`.** The page renders the shadow map once
  (`autoUpdate = false`, `needsUpdate = true`). The port renders it every
  frame. Nothing in the scene moves, so every frame renders the same map.

## 52. `renderer.setMRT()` on a user render target (`webgpu_multiple_rendertargets`, `webgpu_multiple_rendertargets_readback`)

Both pages draw a hardwood-textured torus knot into a two-attachment
`RenderTarget` of their own, `{ output, normal: normalWorld }`, and composite
the two attachments side by side at `screenUV.x = 0.5`. Before them, every
MRT on the ladder went through a `pass()` node (`webgpu_mrt`, `webgpu_deferred`,
the bloom pages), which names its attachments itself. Here the page does.

### 52.1 What three does

* **`count`.** `new RenderTarget( w, h, { count: 2 } )` clones
  `renderTarget.texture` once; the clone has the target's size, type and
  filters and an empty `name`. The page then sets `textures[ 0 ].name =
  'output'` and `textures[ 1 ].name = 'normal'`.
* **The names are the whole contract.** `MRTNode.setup()` looks every output
  name up in `renderer.getRenderTarget().textures` by `name`
  (`getTextureIndex()`), and skips a name it does not find ("Ignore if the
  output exists in the MRT but has never been used"). The readback page's
  512² `readbackTarget` is `{ count: 2 }` and its textures are never named.
  The graded frame is `'mrt'` and never renders into that target, so the dump
  does not show what three writes to it.
* **The clear value per attachment.** `WebGPUBackend.beginRender()` clears
  attachment 0 to `renderContext.clearColorValue` and every other attachment
  to `( 0, 0, 0, 1 )`, unless `MRTNode.setClearColor()` names one. The
  `normal` half of both pages is black off the knot for this reason: the
  scene's `0x222222` is only ever written into attachment 0.
* **What lands in `normal`.** `vec4( normalWorld, 1.0 )` into `rgba8unorm`,
  so every negative component clamps to 0. The attachments are the default
  `UnsignedByteType`, whatever the page's "Float buffers" comment says, and
  single-sampled: `antialias` does not reach a user render target.
* **`NearestFilter` both ways** makes both attachments unfilterable: the
  composite has no sampler and reads them with `textureLoad` (§23).
* **Where the canvas transform runs.** The first page composites through a
  `RenderPipeline`, so the sRGB transform is inline in the quad's shader
  (three's `m04_fragment_fragment_RenderPipeline`). The readback page uses a
  `QuadMesh` with a bare `NodeMaterial`, so under `antialias: true` the quad
  draws into the 4x `rgba16float` framebuffer target and a separate
  `outputColorTransform` pass follows (`m05` / `m06`).

### 52.2 What the port adds

* `RenderTarget::set_count()` and `RenderTarget::set_texture_name()`. They
  are setters, like `set_rg_format()`, so that `RenderTargetOptions` literals
  across the tree do not change. Attachment 0 is always `output` in this port,
  because the name lives on the target and not the texture. Naming it anything
  else panics, where it would otherwise silently leave `mrt( { output } )`
  unwritten.
* `record_pass()` now clears the extra attachments to `( 0, 0, 0, 1 )` rather
  than to the pass's clear colour. That was wrong since MRT first landed, and
  no rung could see it: `webgpu_mrt` has a skybox over every pixel, and the
  bloom and deferred passes clear to black or read only where geometry is.
  Without it the first page was at 42120 pixels, the whole right half.
* `Renderer::read_render_target_pixels( target, x, y, width, height,
  texture_index )` is `readRenderTargetPixelsAsync`. It blocks, as every
  native readback here does. It copies the whole attachment and cuts the
  rectangle out on the CPU, which costs more but gives the same bytes.
  `faceIndex` is not taken.
* `Texture::data_rgba8()` is `new DataTexture( Uint8Array, w, h )` with its
  own defaults: `flipY = false`, no mipmaps, `NearestFilter` both ways. The
  readback page's `pixelBufferTexture.image.data = …; needsUpdate = true` is
  `Texture::set_data()`.

### 52.3 Checked against

`dump_wgsl`'s `multiple_rendertargets_knot` against three's `m02`, and
`multiple_rendertargets_composite` against `m04_fragment_fragment_RenderPipeline`.
Both pages dump the same `m01` / `m02`. The composite matches line for line up
to the naming classes in §8 (`let nodeConstN` against `nodeVarN`, member
order). The two `textureLoad`s, the clamp-wrapping helpers, the
`fragCoord.xy / render.nodeUniform4` screen UV and the inline sRGB tail all
match exactly.

### 52.4 Divergences specific to this section

* **The knot's `NORMAL_normalView`.** Three's `m02` reads `normalWorld`
  through `normalViewGeometry → NORMAL_normalView → normalView`. The port's
  unlit material, which stands in for the bare `NodeMaterial` as in
  `webgpu_textures_2d-array_compressed`, goes `normalViewGeometry →
  normalView` without the `NORMAL` sub-build's copy. It is the same value,
  one private var shorter.
* **The readback branch is not graded,** and neither is the Inspector
  dropdown that reaches it. `App::options` is the dropdown. The example's
  `main()` takes `THREE_RS_SELECTION=diffuse|normal`, so the branch can be
  looked at. In the port, `'diffuse'` shows the knot, because attachment 0
  answers to `output` whatever the page calls it. `'normal'` is black: the
  unnamed attachment 1 gets only its `( 0, 0, 0, 1 )` clear, which is what
  `getTextureIndex()` implies. Whether three's attachment 0, whose name is
  empty, also stays unwritten there has not been checked against a dump of
  that mode.
## 44. Explicit-gradient and gathered taps, and two canvases on one (`webgpu_texturegrad`, `webgpu_texturegather`)

`textureNode.grad( gradX, gradY )` is `SampleMode::Grad( grad_x, grad_y )`,
emitted as `WGSLNodeBuilder.generateTextureGrad()` writes it for a 2-D
texture in the fragment stage: `textureSampleGrad( t, t_sampler, uv, gradX,
gradY )`, each gradient built as a `vec2`. Before this the mode carried no
nodes and baked two zero `vec2`s, for a `PMREMUtils` caller that no longer
exists in the tree; `tsl::texture_grad( map, uv, grad_x, grad_y )` now takes
the two gradients, and like `texture_uv` applies no uv matrix to the uv it is
given. The texture's result is a `nodeVarN` as every tap is.

`webgpu_texturegrad`'s `colorNode` is an inline `Fn()` (a `block`) over two
vars, two `If`s and four gradient taps, and matches three's `m02` fragment from
`// flow` to `DiffuseColor = ` after renumbering
(`tests/nodes_texture_wgsl.rs::texturegrad_fragment_matches_three`, against
the verbatim dump in `tests/fixtures/textures/texturegrad.fragment.wgsl`).
Two of three's `let`s come from its usage count, not the page, so the example
asks for them with `to_const` (§8, "Usage-promoted temps"): `blur`, read nine
times, and each tap's uv sum, which three names just before the tap —
`TextureNode.setup()` wraps the uv in an inline `Fn()` and the builder counts
the sum as read twice.

### Divergences specific to this section

* **Two canvases are two halves of one.** The page runs `init()` twice, for a
  WebGPU backend and a `forceWebGL` one, and each makes its own
  `WebGPURenderer` on an `innerWidth / 2` canvas, the WebGL one styled `left:
  50%` over a darker background. The port has one canvas and no WebGL
  backend: it keeps both `init()`s' scenes and cameras and renders them into
  the left and right halves of an 800 x 500 canvas through the viewport and
  the scissor, both through the WGSL path. The grader cannot tell — the page's
  two halves are the same picture apart from the background, and the port
  grades 0 pixels.
  `webgpu_texturegather` is drawn the same way, and grades 16 pixels: the
  corners of the depth-compare pentagon on the left half.
## 45. `Node::Custom`, `context()` and `isolate()` (issue #161)

Three's node set is open. Any class that `extends Node` and overrides
`setup( builder )` is a node, and the addons in `examples/jsm/tsl/` define
dozens that way. The port's `Node` is a closed enum, so until now every addon
node had to be written inside the crate. #155 settled how to open it: one
more variant, `Node::Custom(Rc<dyn CustomNode>)` (decision 1), whose `setup`
may only compose the variants that already exist (decision 2), shipped as an
additive 0.1.x change (decision 3). This section also adds the two core
nodes that act on the builder rather than on values: `ContextNode` and
`IsolateNode`.

### 45.1 `CustomNode`

```rust
pub trait CustomNode {
    fn type_name(&self) -> &'static str;   // static get type(): 'RGBShiftNode'
    fn node_type(&self) -> Type;           // getNodeType( builder )
    fn is_cacheable(&self) -> bool { true } // isCacheable( builder )
    fn setup(&self, builder: &NodeBuilder) -> NodeRef;
}
```

`tsl::custom( node )` wraps one in a `Node::Custom`. The builder treats it as
three treats a node whose `setup` returned an `outputNode`:

* **Setup once per build.** The first time `analyze` reaches the node, the
  builder calls `setup` and stores the result under the node's identity. This
  is three's `nodeProperties.outputNode`. The inlined `Fn()` bodies
  (`call_body`) have always been stored this way, and the two now share one
  map, `outputs`. `generate` builds the stored graph in the node's place.
  A second build calls `setup` again, because the per-build data starts empty.
* **Counted as a node, cached when shared.** `Node.analyze()` counts the
  node itself and walks its output only on the first reach. In r187dev
  `Node.build()` then gives *every* cacheable node with a value a var once
  its count passes one (`cacheResult`); this used to be `TempNode`'s job, and
  the display addons now `extend Node` and rely on it. So a custom node
  reached twice builds its output once into a var, unless it answers
  `is_cacheable() == false`. The difference shows as soon as `renderOutput()`
  reads a node as `.xyz` and `.w`. The first version of the gate below
  counted a custom node as a plain non-caching `Node` and failed there, with
  the join inlined twice.
* **Compose, never emit.** `setup` gets `&NodeBuilder`, not `&mut`, so it
  cannot generate, declare or bind anything. What it can ask the builder for
  is `builder.context( key )`. A node that needs a statement shape nothing
  composes into has to be a variant in the crate, where the exhaustive
  `match` and the dump gates see it. Three's addons live inside the same
  limit: they override `setup` and `updateBefore`, never `generate`.
  `updateBefore` and the rest of `NodeFrame` came with #162: a
  `CustomNode` can take part in all three update phases (§57).
* **Why `node_type` is explicit.** Three works a node's type out from its
  built output. The port's TSL methods type their results when they are
  called (`.mul()` has to know what it multiplies), which is before any
  builder exists, so the type has to be declared up front.

The sketch in #161 also had a `hash` method, for `customCacheKey()`. It is
not added because nothing in the port would read it. Per-build data is keyed
on identity (§1), and the program cache is keyed on the generated WGSL and
the binding descriptions, not on a hash of the graph. If #162's
per-object update deduplication turns out to need a hash, it can be added
then.

The proof that the trait is enough is in `tests/nodes_custom.rs`: an
`RGBShiftNode` written there, outside the crate, builds byte-identical WGSL
to the crate's `display::rgb_shift`, which is itself gated against three's
dump (#144). The crate's version stays.

### 45.2 `context( node, { … } )`

`ContextNode.js` merges its value into `builder.context`, builds its node,
and restores the previous context. It does this in `analyze` (on the first
reach only), in `setup` and in `generate`. `Node::Context { node, value }`
does the same with §37's stack: `push_context` in `analyze` and `generate`,
and the guard pops it again. `ContextValue` holds string keys mapped to
nodes, which lands in `BuildContext.extra`, the addon half of #155 decision
6. Three's values are arbitrary JS, often functions (`getViewZ: () =>
scenePassViewZ`). The port's are nodes, because every such key the ladder
has met is a function returning a node. The typed core keys are left out
of `ContextValue`: they are installed by the material's own setup.

**Who reads the keys.** Only code that runs *during the build* sees a
context node's keys. That means an inlined `Fn()` body (expanded in
`analyze`) and a `CustomNode::setup`. Most of the port's TSL runs eagerly
when the graph is constructed (§24.3, §37), and a node that already exists
when `.context()` wraps it was set up under whatever context was current at
its construction. That is the same rule three follows for a node whose
`nodeData` was filled by an earlier build.

**The material's own keys are not visible during the build.** §37 put the
context stack beside the builder because `materials::setup()` runs before
the builder exists, and the stack is empty again by the time `build()` runs.
So a `CustomNode::setup` that calls `normal_view()` gets the default
context, not its material's `setupNormal` or side. In three the material's
keys are there. Nothing on the ladder does this yet. The fix, when a rung
needs it, is to snapshot the material's context into `MaterialFlow` and
push it at the start of `build()`.

**Fog now reads `getViewZ`.** `Fog.js`' `getViewZNode( builder )` reads
`builder.context.getViewZ` and falls back to `positionView.z`. The port's
`range_fog_factor` and `density_fog_factor` are now inlined `Fn()`s, as
three's are. Each body reads `getViewZ` from the context when the builder
expands it. The fallback `positionView.z` is taken when the factor is
constructed, so it still sees a sprite or points material's
`setupPositionView`. `webgpu_custom_fog_background` now spells its fog as
the page does:

```rust
range_fog_factor(float(2.7), float(4.0))
    .context(ContextValue::new().set("getViewZ", scene_pass_view_z))
```

That closes §24.3's shape divergence. An inlined call is transparent to
`analyze` and `generate`, so the scene-fog programs did not change by a byte.

### 45.3 `isolate( node )`

`IsolateNode.build()` swaps in `getCacheFromNode( this, parent )`: a
`NodeCache` kept per isolate node, whose parent is the cache that was
current when the isolate was first built. Its node is built against it, and
then the previous cache is restored. `NodeCache.getData()` falls through to
the parent. `getDataFromNode()` creates a node's data in the current cache
only when the lookup finds none anywhere. So the data of a node the isolate
reaches *first* lives inside the isolate, and a node already known outside
is shared. The dump page shows both:

* `isolate( a ).add( a )`: `a` is counted inside the isolate first, so the
  reach outside starts a fresh count. Neither count reaches two, so there is
  no var, and `sin()` appears twice.
* `b.add( isolate( b.mul( 0.5 ) ) )`: `b` is counted outside first, the
  isolate finds that count through its parent, and the total reaches two.
  One var, one `cos()`.

The port's `NodeCache` (§36) holds only generated snippets, and it is shaped
by block scopes. `analyze` has no scopes at all. So the port keeps three's
per-node data in two places:

* **Counts and outputs** (`usage`, `outputs`) are keyed on `(data cache,
  node)`. Cache `0` belongs to the build. Each `Node::Isolate` is given its
  own cache the first time it is built, with the current one as its parent
  (`data_parents`). A lookup walks up the parents. The first write lands in
  the current cache, and later writes land wherever that first one did.
  This is three's rule exactly.
* **Snippets** go into a child `NodeCache` that `generate` pushes around the
  isolate's node, like a block scope without the indent. The child's entries
  are kept per isolate (and per stage and `fn` depth), so a second build of
  the same isolate finds its vars again.

**One divergence.** When three's lookup finds a node's data in the parent,
it hands back the parent's object, and the snippet written into it later is
visible outside. The port's snippet cache writes into the isolate's child.
The two differ only for a node that was counted outside the isolate but
first *generated* inside it. That would need `analyze` and `generate` to
visit the graph in different orders, which neither does today.
`cache( node, false )`, the deprecated parentless form, is not ported.

### 45.4 Gates

`tests/nodes_custom.rs`, against `tests/fixtures/nodes_custom/`:

* **`context`**: the composite quad of `webgpu_custom_fog_background`
  (three's `m10`), built with `.context( { getViewZ } )`. Its `smoothstep`
  must match three's after renaming, and its WGSL must be byte-identical to
  the argument form. A third test checks that outside a context the factor
  still reads `positionView.z`.
* **`isolate` and `context`**: `tools/dump-pages/isolate_context.html`, a
  one-material page written for this gate, since no example calls
  `.isolate()` itself (`EnvironmentNode` does, inside a graph far too large
  to gate on). The whole fragment body must match three's once
  `nodeVar`/`nodeConst`/`nodeUniform` are renamed in order of first
  appearance. A control test checks that without the isolate the same graph
  has one `sin()`.
* **`CustomNode`**: the out-of-crate `RGBShiftNode` above. A second test
  checks that `setup` runs once per build, is shared by three reaches, and
  reads a `context` key through `builder.context()`.

`dump_wgsl`'s output is byte-identical before and after, apart from §36's
`ObjectUpdate` pointer, and so is the full ladder.

On the way, `tools/dump-webgpu.mjs` had stopped working: a merge had left a
second read of the `--html` page behind, naming a variable that no longer
exists. Every dump died with a `ReferenceError` before Chrome started. The
stray line is gone.
## 42. `FXAANode`, `textureSampleBias`, and a float `uniformArray` (`webgpu_postprocessing_fxaa`, `webgpu_postprocessing`)

`fxaa( node )` is `examples/jsm/tsl/display/FXAANode.js`, ported line by
line into `src/nodes/display/fxaa.rs`. Like `sobel()`, it takes the texture
that `convertToTexture()` would have made. The page hands it the `RTTNode`
of `renderOutput( scenePass )`, because FXAA works on sRGB values.

**One real `fn`.** `ApplyFXAA` is the only `Fn` with a layout
(`FxaaPixelShader( uv, texSize )`), so three's `main()` is a single call. The
helpers are either plain arrow functions (`SampleLuminanceNeighborhood`,
`DetermineEdge`, …) or `Fn`s without a layout (`Sample`, `SampleLuminance`,
`SampleLuminanceOffset`), and both kinds inline at the call site. The port
keeps them as Rust closures and functions, called in the same order as in
the JS. Each helper that puts vars or `If`s on three's stack hands back its
statements, and the caller splices them into the `If( ShouldSkipPixel( l
).not() )` block in the order three pushes them. The eight neighbour taps
come out in first-use order, not declaration order: `max( s, e, n, w, m )`
builds the south tap first. That order falls out of the builder and needs no
code of its own.

**Checked against.** `tests/nodes_display_wgsl.rs` gains a
`Region::Function( name )`, which takes the fingerprint over the body of a
named WGSL `fn`, because `main()` here is only the call. With names
normalised, the port's `FxaaPixelShader` matches three's `m05` statement for
statement except for the divergence below.

**`textureSampleBias`.** `textureNode.bias( -100 )` pins every tap to the top
mip. This is `SampleMode::Bias` and `tsl::texture_bias( map, uv, bias )`. As
with `texture_uv`, the uv is taken as given, with no uv matrix. WGSL allows
`textureSampleBias` only under uniform control flow. Most of these taps sit
inside `if`s and loops, which is why the module keeps three's `diagnostic(
off, derivative_uniformity )`.

**`uniformArray( [ floats ] )`.** `tsl::uniform_array_f32` pads each float
to a `vec4`, and `UniformArray::element_x( index )` reads it back as
`NodeBuffer_N.value[ i ].x`. The index is a node, either `uint( 0 )` or the
loop's `i`, as in three.

**`Loop( { start: 1, end: float( 6 ) } )`.** Three writes the header as
`i < 6`, the bound's value in the index type, so the port passes
`loop_range( "i", int( 1 ), int( 6 ), … )`.

### 42.1 `webgpu_postprocessing` sits on three's own line

This page builds nothing new. `dot_screen` and `rgb_shift` were already gated
against its `m03` and `m05`, and the page wires them as `rgbShift( rtt(
dotScreen( passTexture ) ) )`. The port's 800×500 frame is byte-identical to
three's own frame on this machine (`tools/dump-webgpu.mjs`'
`actual_full.png`). Both score **107** of 100000 pixels against
`webgpu_postprocessing.jpg`, which is over the 100-pixel limit. The halftone
multiplies the channel average by 10 before adding the dot pattern, so a
difference of one LSB between two GPUs moves a dot's rim by a pixel. The 107
pixels are exactly those single-pixel dot rims, scattered across the frame.
Following `webgpu_instance_path`, the e2e test is `#[ignore]`d with the
reason, and the page stays in the steady-frame strip.

### 42.2 Divergences

* **`max( pixelBlend, edgeBlend )` gets a var in each arm.** In the final
  `If( edge.isHorizontal )` / `Else`, three inlines `finalBlend` into both
  `addAssign`s. The port declares it as a var at the top of each arm first.
  The port counts the one node's two uses, one per sibling arm, as two;
  three does not. The value, the calls and the literals are the same, and
  the frame is byte-identical to three's. The port's usage analysis is
  shared by every rung, so this is recorded rather than changed.

## 46. Sampler anisotropy, a transparent canvas, and `float()` on standard nodes (`webgpu_textures_anisotropy`, `webgpu_lights_selective`)

**Anisotropy needs all-linear filters.** `WebGPUTextureUtils.updateSampler()`
copies `texture.anisotropy` into the descriptor's `maxAnisotropy` only when
`magFilter`, `minFilter` and `mipmapFilter` are all `'linear'`. Otherwise it
leaves the reset descriptor's 1. `SamplerKey::of()` applies the same rule to
2-D and cube textures. It has to: wgpu's `anisotropy_clamp` fails validation
above 1 unless every filter is linear, so the old copy-through would have
failed on a `NearestFilter` texture that asked for anisotropy.
`Renderer::get_max_anisotropy()` is `WebGPUCapabilities.getMaxAnisotropy()`,
which returns 16 without asking the adapter. wgpu then clamps the value to
what the hardware supports, as Dawn does.

The key already carried `anisotropy_clamp`, and three's own sampler key
includes `texture.anisotropy`. So the two halves of
`webgpu_textures_anisotropy`, identical but for 16 against 1, get two
samplers in the port as in three's dump (samplers 30 and 73 there).

**Ungraded on this machine.** The port's frame is pixel-identical to three's
frame here (max channel difference 0), and three's frame scores 9234 of
100000 against its own reference. Both halves differ, including the one with
no anisotropy, so the reference's GPU minifies differently. The e2e test is
`#[ignore]`d, like `webgpu_instance_path`'s
(`docs/webgpu_textures_anisotropy-progress.md`).

**A transparent canvas is graded over the page.** With `alpha: true`, the
`WebGPURenderer` default, three's clear colour is `( 0, 0, 0, 0 )`.
`WebGPUBackend.getClearColor()` premultiplies it, and the canvas is configured
`alphaMode: 'premultiplied'`. Wherever nothing draws, Chrome shows the page
behind the canvas, and `page.screenshot()` captures that composite. Until now
this never mattered. Every graded page either covered its canvas (a
background, a full-screen quad) or had `example.css`'s black `<body>`, where
source-over of premultiplied colour onto black is the colour itself.
`webgpu_textures_anisotropy` sets `body { background-color: #f1f1f1 }` in its
own `<style>`, and the floor does not reach the horizon. Everything above the
far plane is that grey.

`testing::composite_over_page( pixels, 0xf1f1f1 )` is that composite: `c +
page * ( 1 - a )` per 8-bit channel, alpha set to 255. The colour is the
page's CSS, carried on the example as `PAGE_BACKGROUND`. It is not taken from
the reference image. The renderer is unchanged: the canvas it hands back
still has alpha 0 there, as three's does. Only the harness, standing in for
the browser's compositor, applies the page colour.

**`float()` around `metalnessNode` and `roughnessNode`.**
`MeshStandardNodeMaterial.setupVariants()` starts with

```js
const metalnessNode = this.metalnessNode ? float( this.metalnessNode ) : materialMetalness;
let roughnessNode = this.roughnessNode ? float( this.roughnessNode ) : materialRoughness;
```

and the port used the node as given. For a float node there is no
difference. For `texture( alphaTexture )`, a `vec4`, the conversion is
`.x`, and it matters where the node is reused. `diffuseContribution =
diffuseColor.rgb.mul( metalnessNode.oneMinus() )` became `( vec4( diffuse,
1 ) * ( 1 - map ) ).xyz`, one minus each channel of the map, instead of
`diffuse * ( 1 - map.x )`. `getRoughness()`'s clamp ran on four lanes before
`Roughness` took `.x`. Both now emit what three's dump has (`m02`, `m06` of
`webgpu_lights_selective`). The test texture is grey, so the frame could not
show the bug. The WGSL comparison found it.

## 48. `frontFacing` outside the fragment stage, and `vec4()` of a vec4 background (`webgpu_loader_gltf_compressed`, `webgpu_equirectangular`)

Two small gaps, each found by the first page that reached it.

**`FrontFacingNode.generate()` writes `true` outside the fragment stage.**
A double-sided material on a geometry with a `tangent` attribute builds
`bitangentView` in the vertex stage: `Bitangent.js` crosses
`normalView` and `tangentView` there and hands the product over as the
`NORMAL_v_bitangentView` varying. Both factors go through
`negateOnBackSide()`, which for `DoubleSide` multiplies by `faceDirection`,
which reads `frontFacing`. WGSL has no `@builtin( front_facing )` in a
vertex entry point, and three never asks for one there: its
`FrontFacingNode.generate()` returns the literal `'true'` whenever
`builder.shaderStage !== 'fragment'`, so three's dump reads
`( ( f32( true ) * 2.0 ) - 1.0 )` in the vertex stage and
`( ( f32( isFront ) * 2.0 ) - 1.0 )` in the fragment. The port declared the
builtin in whatever stage read it, and wgpu refused the vertex module
("Built-in FrontFacing is not available at this stage"). `NodeBuilder`'s
`Node::Builtin` arm now returns `true` for `FrontFacing` in any stage but the
fragment. No earlier rung took this path, because it needs all three: a
double-sided material, a normal map (only the normal map reads the tangent
frame), and a `tangent` attribute. The barn lamp has tangents and normal
maps but is single sided. `PrimaryIonDrive`'s double-sided materials have
tangents but no normal map. coffeemat has all three.

Three's `generate()` also returns `'false'` for a `BackSide` material. The
port does not need that branch: its `negate_on_back_side()` multiplies a
back-sided vector by `-1` and never reads `frontFacing`, as three's does.

**`vec4( backgroundNode )` of a node that is already a vec4 is the node.**
`Background.update()` colours the skybox with
`vec4( backgroundNode ).mul( backgroundIntensity )`. Every background node
the ladder had used before was a colour, a vec3, so
`background_node_color_node` always appended `1.0`.
`webgpu_equirectangular`'s node is `texture( map, equirectUV(), 0 )`, a vec4
sample, and appending to it would build a five-component join. The function
now keeps a vec4 node as it is and appends `1.0` to anything else. Three's
dump shows `DiffuseColor = ( nodeVar0 * vec4<f32>( render.nodeUniform2 ) )`,
the sample times the intensity with no constructor around it, and so does
the port's `dump_wgsl` section `background_equirect`.

Nothing else was missing. `equirect_uv`, `texture_level`, the meshopt
decoder, `KHR_mesh_quantization`, the `KHR_texture_basisu` transcode and
`KHR_texture_transform` were all already in place.
## 44. Compute skinning and points on a `Sprite` (`webgpu_skinning_points`)

### 44.1 A `ComputeNode` read as a value

`Fn( () => { …; return pointPositionArray.toAttribute() } )().compute( n )`
set as a material's `positionNode` does two things in three. Outside the
compute stage, `ComputeNode.generate()` returns its `outputComputeNode`, the
`Fn`'s return value, so the vertex stage reads the attribute. Its
`updateBefore` (`NodeUpdateType.FRAME`) runs `renderer.compute( this )`, once
per frame, before the draw that built it.

The port has `Node::Compute { flow, output }` and `tsl::compute_node( flow,
output )`. The builder's `analyze()` records the flow, deduplicated by
pointer, into `NodeProgram::update_before`, and generates `output` in its
place. `Renderer::draw()` runs each item's update-before list behind the
`FRAME` guard (§57), so the kernel is dispatched through the ordinary
`compute()` path at most once per frame. `onInit` needs no new code: the
existing `compute_dispatch` runs it the first time the kernel's pipeline is
built, so the order is `onInit`, the kernel, then the pass, as in three's
dump.

### 44.2 `computeSkinning( mesh )`

`SkinningNode` in compute mode is `getSkinnedPosition()` over storage copies
of the geometry's `position` (`vec3`, padded to 16 bytes), `skinIndex`
(`uvec4` as `u32`) and `skinWeight`, each indexed by `instanceIndex`. The
result is `bindMatrixInverse * Σ w·B·(bindMatrix * p)`, as a `vec3`.

* **Storage over a CPU array.** `BufferSource::StorageData { init,
  read_only }` is uploaded like `Struct`, at the storage stride
  (`storage_data` pads a `vec3` to four words). Three builds a new
  `InstancedBufferAttribute` per `computeSkinning()` call, and the page calls
  it once per kernel. So two sets of read-only buffers exist, as in three's
  dump.
* **Live uniforms.** `bindMatrix`, `bindMatrixInverse` and
  `objectWorldMatrix( child )` name a specific object, not the render item.
  A kernel has no render item. `UniformSource::Live( LiveValue )` wraps a
  closure over a weak `Node`, which is read whenever the kernel's object
  buffer is written. It is hashed and compared by identity.
* **Bone matrices without a draw.** The mesh is hidden
  (`child.visible = false`), so it never reaches the render list, and
  nothing else would call `skeleton.update()`. `BufferSource::SkeletonBoneMatrices`
  carries the skeleton. `Renderer::skeleton_bone_buffer()` updates it once per
  frame and writes the uniform array, as `SkinningNode.update()` does.

### 44.3 `.toAttribute()` and `Sprite.count`

`StorageArray::to_attribute()` is an `InstanceBuffer` with the storage node's
own `BufferId`. The renderer therefore finds the storage buffer the kernel
wrote, not a copy. Storage buffers now carry `VERTEX` usage. The stride is
the storage stride, so a `vec3` array is read 16 bytes apart (three's dump
declares it `float32x4`). `Sprite.count` (default 1) is the instance count
in `Payload::count()`.

### 44.4 `PointsNodeMaterial.setupVertexSprite()` and `shapeCircle()`

A points material on anything but `Points` offsets the clip position:
`mvp + vec4( positionGeometry.xy * sizeNode * screenDPR / ( viewport.zw / 2 )
* mvp.w, 0, 0 )`. `viewportSize` is `viewport.zw` here because that is what
three's dump reads. Only the `sizeNode` path with `sizeAttenuation = false`
is ported. The renderer's unsupported-field warning names the other two
(attenuation, or no `sizeNode`).

`shapeCircle()` branches on `material.alphaToCoverage &&
renderer.currentSamples > 0` at build time: `fwidth` smoothing, or a hard
`select`. Three reads both inside the `Fn` body. The port puts their
conjunction in `BuildContext::alpha_to_coverage_samples`. The renderer pushes
it around `setup()` and `build()` and adds it to the program's dynamic key.
`PointsNodeMaterial.alphaToCoverage` is `undefined` (false) by default, so
this page takes the hard edge.

### 44.5 Divergences

* **WGSL shape.** Three writes the skinned `bindMatrix * p` product and the
  world position as `let` constants. The port writes them as private vars.
  Its object members are numbered in a different order, and so are its
  storage bindings. The values are the same, and the frame is
  pixel-identical to three's `actual_full.png`.
* **`subgroup_size`.** Three's kernels declare `enable subgroups` and a
  `subgroup_size` builtin they never read. The port does not.

### 44.6 `webgpu_instance_points`: a `StorageInstancedBufferAttribute`

The second page has a kernel that writes each point's size into
`storage( new StorageInstancedBufferAttribute( sizes, 1 ), 'float', n )`. The
material reads the same attribute back with `instancedBufferAttribute()`.
In the port that is `storage_f32( &sizes, Type::F32 )` and
`.to_attribute()`. One `BufferId` means one GPU buffer, filled from `sizes`
and then overwritten by the kernel, which `animate()` dispatches with
`renderer.compute()` before each frame. A `float` array is packed at 4
bytes, as three's `float32` vertex attribute is. The page's `alphaToCoverage:
true` takes `shapeCircle()`'s smoothed edge and turns on the pipeline's
alpha-to-coverage (the frame is multisampled). `vertexColors: true` has no
effect, because the sprite quad has no `color` attribute and three checks
`geometry.hasAttribute( 'color' )`. The example leaves it off and says so.
The smoothed branch is built only under a renderer, from the frame's
sample count.

## 51. `alphaHash` and multi-material groups (`webgpu_materials_alphahash`, `webgpu_materials_arrays`)

Neither page is graded. Three.js itself fails its own reference for both
on this machine: 3782 and 251 pixels. The port's frames are pixel-identical
to three's frames here (see the two progress docs). This section records what
was ported so a machine where three hits the reference can grade them as they
stand.

### 51.1 `alphaHash`: what three does

`NodeMaterial.setupDiffuseColor()` runs after the alpha test and before the
opaque `diffuseColor.a = 1`:

```js
if ( this.alphaHash === true ) {
    diffuseColor.a.lessThan( getAlphaHashThreshold( positionLocal ) ).discard();
}
```

`getAlphaHashThreshold` (`nodes/functions/material/getAlphaHashThreshold.js`)
is Wyman and McGuire's hashed alpha test. The pixel scale comes from
`max( length( dpdx( position ) ), length( dpdy( position ) ) )`. It is
bracketed by the two neighbouring powers of two, and each is hashed with
`hash3D( floor( 2^n * position ) )`. `hash3D` is `hash2D( vec2( hash2D(
xy ), z ) )`, and `hash2D` is `fract( 1e4 * sin( 17 x + 0.1 y ) * ( 0.1 + abs(
sin( 13 y + x ) ) ) )`. The two hashes are mixed by `fract( log2( pixScale ) )`
and then passed through the uniform-distribution CDF. The CDF's three cases
are a nested `select`, and the result is clamped to `[ 1e-6, 1 ]`. The outer
function has a `setLayout`, so it becomes a real WGSL `fn`; the two hashes are
plain `Fn()`s, inlined at each call.

`src/nodes/alpha_hash.rs` builds the same graph: `shader_fn` with the layout,
and `inline_fn` for the hashes. `Material.alpha_hash` is the flag, and
`node_material.rs` emits the discard at three's point in `setup_diffuse_color`.
The two pages' materials pay nothing when the flag is off.

### 51.2 Multi-material groups: what three does

`Mesh.material` may be an array. `Renderer._projectObject()` handles it like
this:

```js
if ( Array.isArray( material ) ) {
    for ( const group of geometry.groups ) {
        const groupMaterial = material[ group.materialIndex ];
        if ( groupMaterial && groupMaterial.visible ) {
            renderList.push( object, geometry, groupMaterial, groupOrder, z, group, clippingContext );
        }
    }
}
```

Every render item carries its `group`. Opaque or transparent is decided per
group material. `RenderObject.getDrawParameters()` intersects the geometry's
`drawRange` with `[ group.start, group.start + group.count )` and clamps it to
the index or position count. Render objects are keyed by ( object, material,
… ), so two groups that share a material share a pipeline and bindings, and
differ only in the draw call.

The port keeps the array form in a separate field, `Mesh.materials`, created
by `Mesh::with_materials( geometry, materials )`, with `material: None`. The
alternative, making `Mesh.material` an enum, would have touched every call
site in the port for one page. `Payload::material_array()` exposes it.
`render_list.rs` pushes one `RenderItem` per group, with `group: Some( group )`.
`RenderItem::material( object )` is the single place that resolves `material[
group.materialIndex ]`, so the renderer never reads `object.material()`
directly for a drawable. The draw builder, the transmission probe, the
opaque/transparent split and both shadow passes (planar and point) all use it.
`Renderable.group` feeds the draw-range intersection in `src/renderer/mod.rs`,
which is three's arithmetic in `u64`. `geometry.addGroup()` already existed
(`BoxGeometry`, the cylinder, extrude); nothing downstream of it did.

### 51.3 Checked against

`dump_wgsl`'s `materials_alphahash` against three's `m13` (vertex) and `m14`
(fragment), for the instanced, instance-coloured standard material under a
PMREM environment. `getAlphaHashThreshold` has the same statements in the
same order: the log2/exp2/floor/ceil bracket, the two `hash3D`s, the mix, the
CDF and the clamp. The discard sits between the alpha test's place and the
opaque clamp, as in three. `webgpu_materials_arrays` builds no new WGSL. Each
group draws with the program of an ordinary `MeshStandardMaterial`, which the
ladder already checks.

### 51.4 Divergences specific to this section

* **Naming.** The port writes `var nodeVarN` where three writes `let
  nodeConstN`. This is the §8 class.
* **The CDF's `cases` vector.** Three writes `vec3( … )` inline in each branch
  of the nested `select`, as `.x`, `.y`, `.z`. The port's usage counter sees
  the node used more than once within the branch scope and promotes it to a
  var in each branch. The value and the pixels are the same.
* **Array materials only on `Mesh`.** `InstancedMesh`, `SkinnedMesh`,
  `BatchedMesh`, lines and points keep a single material (`materials` is
  empty). Three allows arrays there too, but nothing on the ladder uses them.
* **Not in raycasting.** Three's `Mesh.raycast` walks the groups, tests each
  with `material[ group.materialIndex ].side`, and reports that
  `materialIndex`. The port's raycaster still reads the single `material`, so
  a `with_materials` mesh is tested as one front-sided mesh over its whole
  draw range, with `face.materialIndex` 0. No graded page raycasts one.
## 56. `copyTextureToTexture` (`webgpu_textures_partialupdate`)

The page patches a loaded texture in place: every tenth of a second a 32 x 32
`DataTexture` is refilled with one random colour and
`renderer.copyTextureToTexture( dataTexture, diffuseMap, null, position )`
copies it into `Carbon.png` at a random multiple of 32 texels. Nothing in the
node graph changes. The shader is the plain `MeshBasicMaterial` with a `map`
that §5 already builds, and the dump's `m01` has nothing new in it.

### 56.1 The port

`Renderer::copy_texture_to_texture( src, dst, src_region, dst_position )` is
`Renderer.copyTextureToTexture()` and `WebGPUBackend.copyTextureToTexture()`
at `srcLevel = dstLevel = 0`:

* **Both textures are updated first.** Three calls
  `_textures.updateTexture()` on each, so the data texture's `needsUpdate`
  (its bytes were just rewritten) is uploaded before the copy reads it. The
  port calls `ensure_texture_2d()` on both, which is the same thing.
* **The region is in GPU texel rows.** The default region is the whole source
  image and the default position is the destination's origin. `Box2` gives
  `min` and `max`, and the extent is `max - min`. The coordinates are those of
  the GPU texture after the upload's `flipY`. `Carbon.png` is uploaded
  flipped and the data texture is not (`DataTexture.flipY = false`), so row
  `y` of the destination is `y` texels up from the bottom of the image. The
  e2e rung checks exactly that.
* **One encoder, one submit.** Then, if the destination has
  `generateMipmaps` and more than one level, its chain is regenerated. Three
  also checks `mipmapsAutoUpdate`, which the port does not have, so it is
  always on. The page turns `generateMipmaps` off, so the branch does not run
  here.
* **`COPY_SRC` on every uploaded texture.** `WebGPUTextureUtils.createTexture()`
  gives every texture `TEXTURE_BINDING | COPY_DST | COPY_SRC`. The port had
  left out `COPY_SRC`, which wgpu needs on a copy's source. Adding a usage
  flag changes no pixels, and the ladder confirms it.

`Texture::data_rgba8` is `new DataTexture( Uint8Array, w, h )`: `RGBAFormat`,
`UnsignedByteType`, and `DataTexture`'s own `flipY = false`,
`generateMipmaps = false` and `NearestFilter`.

### 56.2 What the grader sees, and what it does not

Under the pinned clock `timer.getElapsed()` is 0 on every frame, so
`elapsedTime - last > 0.1` never holds and neither three nor the port copies
anything before the graded frame. The screenshot is the untouched texture. The
copy path is gated by the rung's second half instead: the test moves the
clock to 150 ms and lets `animate()` make one copy from the seeded
`Math.random()` sequence (`randInt( 1, 16 )` twice, then the colour). It then
asserts that the pixels that changed are exactly the 32 x 32-texel block where
three's semantics put it, within two pixels, and that the block shows the
copied bytes. This checks the port against three's documented behaviour, not
against a reference image, so it is not a second comparator.

### 56.3 Page quirks reproduced

* **The bytes are linear.** `color.setHex( Math.random() * 0xffffff )` converts
  from sRGB, so the bytes written are the *linear* channels. They go into an
  sRGB texture, which decodes them once more.
* **The alpha is 1, not 255.** `data[ stride + 3 ] = 1` is kept. The
  material is opaque, so `DiffuseColor.w = 1.0` hides it.

## 50. `RotateNode`'s `vec3` branch, `wireframe`, and `CameraHelper` (`webgpu_layers`, `webgpu_camera`)

### 50.1 `rotate( vec3, vec3 )`

`tsl::rotate` now ports both branches of `RotateNode.setup()`. The `vec3`
branch builds one `mat4` per axis:

```js
const rotationXMatrix = mat4( vec4( 1, 0, 0, 0 ), vec4( 0, cos( rotation.x ), sin( rotation.x ), 0 ), … );
…
return matrixMap[ order[ 0 ] ].mul( matrixMap[ order[ 1 ] ] ).mul( matrixMap[ order[ 2 ] ] ).mul( vec4( positionNode, 1.0 ) ).xyz;
```

Only the default `'XYZ'` order is ported, because `rotate()`'s third argument
is not used on the ladder. Each `rotation.x`, `cos( rotation.x )` and
`sin( rotation.x )` is a new node in three, so the port builds each one anew
too. Sharing them would turn the inline `cos( nodeConst1.x )` calls in three's
WGSL into temps. Only `rotation` itself is one node, used twelve times, and the
builder hoists it: `nodeConst1` in three's `webgpu_layers` `m03`, a `nodeVar`
in the port (the same temp spelling difference as elsewhere).

### 50.2 `material.wireframe`

No shader reads it. In three it changes three things, all in the renderer:

* `WebGPUUtils.getPrimitiveTopology()`: `isLineSegments || ( isMesh &&
  wireframe )` is a `line-list`. The port's `Primitive::of` takes the
  material's flag and records it as `Primitive::wireframe`, which is true only
  for a mesh.
* `Geometries.getIndex()` swaps the geometry's index for
  `getWireframeIndex()`'s: each triangle `a b c` becomes `a b b c c a`, read
  from the index, or from the vertex order when there is no index. The port
  builds it once per geometry (`GeometryGpu::wireframe_index`) the first time
  a wireframe material draws that geometry. It is always `uint32`. Three
  builds a `Uint16BufferAttribute` below 65535 vertices, but
  `WebGPUAttributeUtils.createAttribute()` widens every non-normalized
  `Uint16Array` to `Uint32Array` on upload, and three's dump of
  `webgpu_camera` shows `format: "uint32"`.
* `RenderObject.getDrawParameters()`'s `rangeFactor = 2` scales `drawRange` to
  the doubled index count.

Divergence: three rebuilds the wireframe index when the geometry's index or
position version moves. The port's index is not versioned (a changed index
is a new geometry), so the wireframe index lives and dies with the geometry.

### 50.3 `CameraHelper` and a shared matrix

`CameraHelper` is a `LineSegments` of 50 vertices with vertex colours.
`update()` un-projects 21 named NDC points through
`camera.projectionMatrixInverse` alone, so the geometry is in the camera's
local space. Its matrix places it:

```js
this.matrix = camera.matrixWorld;
this.matrixAutoUpdate = false;
```

That line shares the matrix *object*, it does not copy it. Every
`updateMatrixWorld()` traversal that reaches the helper multiplies in whatever
the camera's world matrix holds at that moment. The port has no shared
`Matrix4`s, so `Object3D::matrix_alias` holds a `WeakNode` to the camera, and
`update_own_matrix_world` copies that node's `matrix_world` into `matrix` just
before composing. That is the point where three would read the shared object.

`webgpu_camera` shows why the timing has to be exact. The helper is added to
the scene before `cameraRig`, so within one traversal it is visited before its
camera. The first `render()` of a frame gives it the camera's world matrix
from before the rig's `lookAt()`. The second gives it the one the first
traversal computed. Only the second is visible (`activeHelper.visible` is
toggled between the two renders), and the port draws it where three does,
pixel for pixel.

`RenderCamera::node()` is how the helper finds the camera's node. The port's
`OrthographicCamera` is not a scene-graph node, so it returns `None`, and its
helper takes a one-time copy of the camera's world matrix instead. That is a
divergence, and `webgpu_camera` never shows it: the orthographic helper is
hidden on every frame the page draws.

`camera.reversedDepth` is not on the port's cameras, so `update()` takes
near/far from the coordinate system alone. `toneMapped: false` has no
counterpart, as with `GridHelper`.

### 50.4 A legacy `PointsMaterial` is opaque

`NodeLibrary.fromMaterial()` builds the node material and then assigns every
property of the legacy one over it. A `PointsMaterial` therefore gives a
`PointsNodeMaterial` with `transparent = false`, not the `true` that
`PointsNodeMaterial` inherits from `SpriteNodeMaterial`. Three's dump of
`webgpu_camera`'s points pipeline has no blend state, and its fragment ends
in the opaque `DiffuseColor.w = 1.0`. The page sets `transparent = false`
after `PointsNodeMaterial::points()`. `webgpu_postprocessing_ca` also passes a
legacy `PointsMaterial` but keeps `transparent`. That rung is green, and it
was not changed here. Moving it to the opaque list would be a separate
change, checked against that page's own dump.

## 58. `webgpu_multisampled_renderbuffers`, an ignored rung on §50's wireframe

The page draws two `InstancedMesh`es of fifty boxes, one with `wireframe: true`,
into a `RenderTarget` with `samples: 4`, then shows `renderTarget.texture`
through a `QuadMesh`. Both halves already exist in the port. The multisampled
target (an MSAA colour attachment resolved into `rgba8unorm`, and a
multisampled `depth24plus`) is the existing `RenderTarget` path. The wireframe
is §50.2's `Material.wireframe` from `webgpu_layers`. The rung adds no API.

Two details of three's wireframe that §50.2 leaves out do not reach this page:

* `NodeBuilder.isFlatShading()` is `flatShading && !wireframe`, but
  `MeshBasicMaterial` has no flat shading to turn off.
* The index is `uint16` below 65535 vertices in three and always `uint32` in
  the port. The format does not change which lines are drawn.

**Why the rung is ignored.** Three itself scores 2405 of 100000 pixels against
`screenshots/webgpu_multisampled_renderbuffers.jpg` on this machine (Intel Iris
Xe, Mesa 25.3.6). Every one of those pixels is on a wireframe line: Vulkan
leaves line rasterization to the implementation, and the reference came from
another GPU. The port's frame is measured against three's own frame for the
page (`tools/dump-webgpu.mjs`), and
`docs/webgpu_multisampled_renderbuffers-progress.md` gives the counts. As with
`webgpu_textures_anisotropy`, the e2e test is `#[ignore]`d with that reason and
the page stays in the steady-frame strip. It has no README, web or viewer
registration, and the grader is not loosened.
## 54. The EON diffuse lobe and `KHR_materials_diffuse_roughness` (`webgpu_loader_gltf_diffuse_roughness`)

`MeshPhysicalMaterial.diffuseRoughness` turns the Lambert diffuse lobe into
the Energy-preserving Oren-Nayar lobe (Portsmouth et al. 2025), as
`BRDF_EON.js` implements it. `webgpu_loader_gltf_diffuse_roughness` loads
Khronos' `DiffuseRoughnessParameterSweep.glb` under a PMREM of
`RoomEnvironment`. It is a grid of spheres whose
`KHR_materials_diffuse_roughness.diffuseRoughnessFactor` runs from 0 to 1.

### 54.1 What three does, and what the port does

* **`useDiffuseRoughness`.** `MeshPhysicalNodeMaterial` sets it when
  `diffuseRoughness > 0` (or there is a `diffuseRoughnessMap`).
  `setupVariants()` then writes the `DiffuseRoughness` property from the
  clamped `materialDiffuseRoughness` uniform. The port writes it in the same
  place, just before the clearcoat assignments, through
  `tsl::diffuse_roughness()` and `tsl::material_diffuse_roughness()`
  (`UniformSource::MaterialDiffuseRoughness`, object group).
* **`PhysicalLightingModel`.** With the flag on, three changes three terms:
  * `direct()` uses `BRDF_EON( lightDirection, diffuseColor, roughness )`
    times `1 - metalness` in place of `BRDF_Lambert`.
  * `indirectDiffuse()` scales irradiance by `EON_DirectionalAlbedo( ... )`
    / π in place of `diffuseColor`.
  * `indirectSpecular()` uses the same albedo where it would use
    `diffuseContribution`.

  `physical::Physical` gains a `diffuse_roughness` flag. It picks
  `brdf_eon()` and `eon_diffuse_albedo()` at the same three points.
* **The constants.** `FON_A = 1/2 - 2/(3π)`, `FON_AVG = 2/3 - 28/(15π)` and
  the `1e-7` epsilon are written to the same digits as three's, so the dump
  diffs clean.
* **`rho ≤ ε` falls back to Lambert.** Three does this through a
  `select()`, which the port writes as its usual if/else into a var.
* **`GLTFLoader`.** `GLTFMaterialsDiffuseRoughnessExtension` reads
  `diffuseRoughnessFactor` (default 0). Like the other physical extensions,
  its presence promotes the material to `MeshPhysicalMaterial` whatever the
  factor.

### 54.2 The page's winding flip

The page walks the scene and swaps index `i + 1` and `i + 2` of every
triangle, because "the draft sample asset currently uses clockwise triangle
winding". The port does the same with `Index::set_x` on a clone of each
distinct geometry. It keys a map by pointer, so a geometry shared by several
meshes is flipped once, as three's loop does through its `Set` of visited
geometries.

### 54.3 Checked against three's dump

`dump_wgsl`'s `gltf_diffuse_roughness_on` was diffed against three's
fragment shader for the sweep's material. The `DiffuseRoughness` assignment
comes in the same place, the two EON `if/else` blocks (direct and
directional albedo) have the same arithmetic and constants, and the
uniform order is the same (`diffuseRoughness` is `nodeUniform9` in both).
`gltf_diffuse_roughness_zero` is the Lambert shader, unchanged.

### 54.4 Divergences

* **`diffuseRoughnessMap` is not ported.** Nothing on the ladder has one.
  The loader reads only the factor and says so in a comment.
* **A hoisted temp.** In the `rho > ε` branch the port writes
  `nodeVar7 = clamp(...); nodeVar6 = nodeVar7;` where three assigns
  directly. The value is the same.
* **`indirectDiffuse`'s `diffuse` is inlined.** Three makes it a `toVar()`.
  This divergence was already there.
* **The grader cannot see the lobe.** The page grades at 0 pixels with the
  EON lobe switched off as well, because pixelmatch's 0.1 YIQ threshold is
  wider than anything the lobe moves. With it on, the frame differs from the
  Lambert frame by up to 15 levels in 255 over about 73000 pixels. Against
  three's `expected.jpg`, the mean absolute error in the sphere grid is
  1.35 levels with EON and 3.60 without. So the lobe is checked by the dump
  and by that measurement, not by the pixel count.

### 54.5 `scene.environmentNode` as a graph (`webgpu_cubemap_mix`)

`webgpu_cubemap_mix` sets `scene.environmentNode` to
`mix( pmremTexture( cube2 ), pmremTexture( cube1 ), oscSine( time.mul( .1 ) ) )`,
and `scene.backgroundNode` to the same node with `.context( { getTextureLevel:
() => float( .5 ) } )`.

Both `pmremTexture()` calls have no UV and no level of their own.
`PMREMNode.setup()` takes them from the build context:
* `EnvironmentNode.setup()` builds the whole graph twice, once under
  `createRadianceContext()` (the reflect vector, `roughness`) and once under
  `createIrradianceContext()` (`normalWorld`, 1). With a clearcoat it builds
  it a third time.
* `Background.update()` builds it under `backgroundRotation.mul(
  normalWorldGeometry )` and `backgroundBlurriness`.
* The inner `.context()` wins over both for the level.

The port has no node context. Before this page the environment was always a
`PmremHandle`, and `environment::setup` called its `sample( uv, level )`
directly. Now the environment is one of two things:

* `environment::EnvironmentNode`: an `Rc<dyn Fn( uv, level ) -> NodeRef>`
  with an identity. It is the graph as a function of the two values the
  context would supply, and the page's closure calls `PmremHandle::sample`
  for each `pmremTexture()` leaf. `with_texture_level( level )` is
  `.context( { getTextureLevel } )`, and it is a new node, as `.context()`
  is.
* `environment::Environment`, which is `Pmrem( PmremHandle )` or
  `Node( EnvironmentNode )`. `SetupContext::environment` and
  `environment::setup` take it.

`Scene::environment_node` is `scene.environmentNode`, and it wins over
`Scene::environment` as `NodeManager.getEnvironmentNode()` does.
`Background::EnvironmentNode` is the background case, which
`background_environment_color_node()` builds. Both are keyed by the node's
identity, as `Background::Node` is.

Each call of the closure builds the graph again, which matches the dump:
three's `m10` repeats the `mix` and the `oscSine` for `radiance` and for
`iblIrradiance`, and so does the port's `cubemap_mix_material`.

### 54.6 `webgpu_cubemap_mix`: checked against

* **`cubemap_mix_background` against `m08`.** It has the same two
  `textureSampleLevel`s, each with its own `materialEnvRotation` multiply and
  its own `clamp( 0.5 )`, and the same `mix` and `sin`. The only difference
  is that the port computes `backgroundRotation * vec4( normalWorldGeometry,
  1 )` once and shares it between the two reads. Three calls `getUV()` once
  per leaf and repeats it. The value is the same.
* **`cubemap_mix_material` against `m10`'s environment half.** The reflect
  vector is shared by both radiance reads and each read rotates it itself,
  as three does (`nodeConst12`, then `nodeConst13` and `nodeConst15`). The
  irradiance reads are the same.

### 54.7 Divergences specific to `webgpu_cubemap_mix`

* **The graded frame shows one cube.** The harness pins `time` to 0, where
  `oscSine` is 0, so the mix is all `cube2`, the Milky Way. The Pisa cube's
  PMREM is still generated and sampled, but it gets weight 0.
## 55. `reflector()` — planar mirrors as nested renders (`webgpu_mirror`)

`src/nodes/utils/ReflectorNode.js` is two nodes. `ReflectorNode` is a
`TextureNode` whose uv is `screenUV.flipX()` and whose uv matrix is off
(`setUpdateMatrix( false )`). `ReflectorBaseNode` is an `updateBefore` node of
type `RENDER`. Right before the object whose material carries it is drawn, it
renders the scene into a `HalfFloatType` target sized to the drawing buffer
times `resolutionScale`. It renders from a virtual camera, which is the real
camera mirrored in the plane of `reflector.target` (a point on the plane at
its world position, its world +Z the normal). The virtual camera's projection
has its third row replaced so that the near plane is the mirror plane
(Lengyel's oblique clip). It then points `textureNode.value` at that target.

The port is `src/nodes/reflector_node.rs` (the nodes and their state) and
`src/renderer/reflector.rs` (`updateBefore()`, which is a nested
`renderer.render()` and so has to live with the renderer). The
`updateBefore()` is a line-by-line port: the facing-away early-out and its
`hasOutput` clear, the reflected view, look target and up vector, the
oblique projection, hiding the carrier's material for the length of the
render, and saving and restoring the render target, MRT and `autoClear`.

### 55.1 What the dump shows, and why the order matters

`webgpu_mirror` has two mirrors, the floor (`R19`) and the back wall (`R20`).
Both use the default `bounces: true`, so each one also renders while the other
is rendering. Three's dump is 33 passes:

* pass 0 is the main pass;
* pass 1 is `R19` from `V1`, and pass 2 is `R20` rendering from `V2` *inside*
  pass 1;
* pass 30 is `R20` from `V3`, and pass 31 is `R19` rendering from `V4` inside
  pass 30.

The passes in between are mipmaps and the output transform. A nested render
is submitted before the render around it. Every draw binds whatever
`textureNode.value` is when the draw is *recorded*. That is the target of
`virtual( current camera )`, and it is not always the last target the
reflector rendered into. For example, the main pass's floor is recorded
after `R20`'s nested render has moved `R19.value` to `V4`'s target, but it
binds `V1`'s target, because it was recorded before that happened.

### 55.2 Divergence: the pre-pass and per-draw overrides

The port records a pass only once it has built every item, so it cannot
interleave a nested render between two draws of the same pass. Instead,
`render()` walks the items in draw order before it records anything
(`Renderer::update_reflectors`). For each item it looks up the program the
draw will use and scans its texture bindings for a reflector's default
texture. It fires each reflector's `updateBefore()` once per `render()` (three's
`renderId` check), and it snapshots `( default texture, value )` into the item
(`Renderable::texture_overrides`). `draw()` substitutes the snapshot when it
resolves that item's bind groups. The renders happen in the same order and
are submitted before the outer pass, and each draw sees the texture three's
would.

Because the graph only holds bindings, three's single module-level
`_defaultRT` becomes one default target per reflector. That texture's id is
how the renderer tells a reflector binding from any other texture. A
thread-local registry maps the id to the reflector. The registry holds the
reflector strongly, since in the port the material graph holds only the
texture node. It drops a reflector once nothing but the reflector itself
holds that default texture any more.

A render nested inside another neither resets `renderer.info` nor starts a
new frame (`Renderer::call_depth`). It is part of the frame around it.

`RenderCamera::id()` was added because `virtualCameras` is a `WeakMap` keyed
on the camera object. The virtual camera is taken out of its map for the
length of its render, so a nested update of the same reflector cannot alias
it.

### 55.3 WGSL

`dump_wgsl` sections `mirror_vertical` and `mirror_ground` match three's `m06`
and `m08` statement for statement: `nodeVar0 = fragCoord.xy /
render.nodeUniform1`, the sample at `vec2( 1.0 - nodeVar0.x, nodeVar0.y ) +
offset` with no uv matrix, and the floor's `mix( vec4( white, 1 ), reflection,
decal.w )`. The only difference is that the port parenthesises `( 1.0 -
nodeVar0.x )`.

### 55.4 Not ported

`generateMipmaps`, `depth` (the reflector's `getDepthNode()`) and
`reflector.forceUpdate` from outside are not ported, since no graded page
uses them. The clip bias is 0, as in three.
## 53. `webgpu_tsl_halftone` and `webgpu_tsl_earth` — a deferred `Fn()` as `outputNode`, and `bumpMap()` of an expression

The page mixes two halftone dot screens into every material's `output`:
`material.outputNode = halftones( output )`, where `halftones` and `halftone`
are TSL `Fn`s. Each screen is a rotated grid in screen space
(`screenCoordinate / screenSize.yy`), with dots sized by how far
`normalWorld` points along a light direction. The page applies it to a
default `MeshStandardNodeMaterial` on a torus knot and a sphere, and to every
material of the skinned `Michelle.glb`. Nothing new was needed in the builder:
`rotate`, `mod`, `remapClamp`, `step` and the split `Output.x = …` assign
were all there.

### 53.1 `outputNode` runs in its material's context

A TSL `Fn` body is not run where it is called. It runs inside the build of the
material that uses it, so `normalWorld` inside `halftones` reads *that*
material's `normalView`. For Michelle that is the normal-mapped normal,
negated on back faces because the GLTF material is `DoubleSide`. The port's
graph is eager. Built at the call site in `init()`, with no material in scope,
`normal_world()` keyed on the bare geometric normal. The fragment then
re-assigned `normalView = normalViewGeometry` just ahead of `normalWorld`,
which put 134 pixels on Michelle's hair and body and failed the rung.

The port already had a seam for exactly this: `scene.fogNode`'s factor is held
in an argument-less inline `Fn` call and run by `resolve_fog_factor` inside
`NodeMaterial` setup. That body is now `tsl::resolve_fn_call`, and
`resolve_fog_factor` delegates to it. The same function runs a deferred
`outputNode` in `node_material::setup_inner`. The example wraps `halftones`'
body in `call( &inline_fn( 0, … ), vec![] )`, so each material runs it once
with its own normal, and the body's normal is the one the lighting used.
Only an argument-less inline call is resolved. Every other `outputNode` is
used as it is, and no existing rung has one of those, so the ladder did not
move.

The one divergence is that three passes `output` as the `Fn`'s argument, while
the port's body closes over `output_property()`. It is the same node.

### 53.2 Checked against

`dump_wgsl`'s `dump_tsl_halftone()` was compared with three's dump:

* **`tsl_halftone_default` against `m00` / `m01`.** The halftone tail is
  three's, statement for statement. The lighting has the same terms in a
  different order. Three emits the DFG lookup and the dielectric scattering
  first and zero-initialises each accumulator where it is first used. The
  port zero-initialises them all up front (twice) and emits the directional
  light before the DFG. Three also writes `normalView` through
  `NORMAL_normalView = normalViewGeometry` where the port assigns it directly.
  The rest is spelling: `fragCoord` is the first fragment parameter rather
  than the last, and a `nodeVarN` stands where three writes
  `let nodeConstN`. None of it changes a value, and the frame shows that.
* **`tsl_halftone_body` against `m03` / `m04`** (`Ch03_Body`: physical,
  `DoubleSide`, normal map, 65 bones). After the fix, the halftone tail reads
  the TBN-mapped `normalView` exactly as three's does. The rest is the same
  reordering and spelling as the default material, plus the numbering of
  varyings and uniforms.
  The skinned vertex shader is the one the existing skinning path already
  emits. Three writes the bone-matrix sum out three times inside the
  `mat3x3` for `normalLocal`, and the port holds it in one var.

The port's 800x500 frame is pixel-identical to three's own
`actual_full.png`. Three itself scores 93 of 100000 pixels against the
reference JPEG on this machine, and so does the port.

### 53.3 `webgpu_tsl_earth`: `bumpMap()` of an expression

The globe's normal is `bumpMap( max( texture( map ).r, cloudsStrength ) )`,
a height built from two taps rather than one map's `.r`. Three's
`dHdxy_fwd` handles any `textureNode` by sampling it three times under a
`context( { getUV, forceUVContext: true } )`. That context moves every
default-uv texture tap inside the expression to `uv`, `uv + dFdx( uv )` and
`uv + dFdy( uv )`, and each tap still goes through its map's uv matrix. The
port's eager graph has no such context. `tsl::bump_map_with( height, scale )`
calls `height` once per tap and hands it a stand-in for `texture( map )`
that samples at that tap's uv. `bump_map( map, scale )` is now
`bump_map_with( |texture| texture( map ).x(), scale )`, which builds the same
nodes as before.

A node the closure captures instead of building through the stand-in is
shared by all three taps. That is what three does with `cloudsStrength`,
which samples at an explicit `uv()`: its dump has the same `nodeConst0` in
all three `max()`es, and so does the port's.

The rest of the page needed nothing new: `positionWorld`, `cameraPosition`,
`normalWorldGeometry`, `smoothstep`, `remap`, `step`, a `roughnessNode` and
an eager `outputNode` (it reads only `normalWorldGeometry`, which does not
depend on the material). The `BackSide` transparent atmosphere is a
`MeshBasicNodeMaterial` whose `outputNode` replaces its colour.

### 53.4 `webgpu_tsl_earth`: checked against

* **`tsl_earth_globe` against `m01` / `m02`.** The bump section is three's
  statement for statement, including the order of the taps: the `dFdx` tap,
  then `Hll`, then the `max()`, then the `dFdy` tap. One divergence: three
  gives each `texture()` call its own uv-matrix uniform (`nodeUniform5` for
  the roughness tap, `nodeUniform14` for the bump taps). The port keys the
  uniform by map and slot, so both read one. The matrices are the same
  identity. The lighting is reordered as in §53.2, and the `outputNode` tail
  is three's.
* **`tsl_earth_atmosphere` against `m03` / `m04`.** The `outputNode` is
  three's. Three's `BasicLightingModel` preamble writes `indirectDiffuse`
  through a `vec4` and the port's writes `Output` directly. That value never
  reaches the target, because `outputNode` replaces it.

The port scores 0 of 100000 pixels, and so does three's own frame. Against
three's 800x500 frame, 27781 pixels differ by one or two levels across the
globe, and 9 isolated ones near the sun's highlight differ by up to 75.
The likely source is the filtering of the three 4096x2048 JPEGs (mip
generation, 8x anisotropy). That was not pinned down, since none of it
reaches the grader's threshold.
## 57. The renderer-owned `NodeFrame` and its three update phases (issue #162)

### 57.1 What three does

A node can ask to be called around the draws that reach it. It says how
often with `updateBeforeType`, `updateType` and `updateAfterType`, each a
`NodeUpdateType`: `NONE`, `FRAME` (once per frame), `RENDER` (once per
`render()` call) or `OBJECT` (every draw). `Renderer._renderObjectDirect()`
runs `nodes.updateBefore( renderObject )`, then `updateForRender`, the draw,
and `nodes.updateAfter( renderObject )`. `NodeFrame` holds the clock
(`frameId`, `renderId`, `time`, `deltaTime`) and one map per phase that
stamps each node with the `frameId` / `renderId` it last ran in.

`updateBeforeNode()` writes the stamp *before* it calls the node and puts the
old one back if the call returns `false`. `updateNode()` and
`updateAfterNode()` write it only after a call that did not return `false`.
The early stamp is what stops a pass from recursing: a pass's own scene
render reaches the pass again and finds it done. `renderId` is `info.calls`
for the render that is running. A nested render puts the outer value back
when it returns.

`PassNode`, `RTTNode`, `BloomNode` and `ComputeNode` all use
`updateBeforeType = FRAME`. A pass renders the first time in a frame that a
draw samples its texture, from inside that draw. That is why three's
canvas `beginRenderPass` is recorded before the passes it samples, but
submitted after them.

### 57.2 The port

`nodes::frame::NodeFrameState` is the renderer's `NodeFrame`: the clock,
the render id and the stamp maps. `Renderer::node_frame()` reads it.
`nodes::NodeFrame<'a>`, the object and occlusion view a per-object uniform is
handed (§39), stays as it was. It is the per-draw slice of the same frame.
`NodeUpdateType` and the `NodeUpdate` trait (`update_before_type()` …
`update_after( &mut Renderer ) -> bool`) are the node half. A method is handed
the renderer rather than the frame, and reads the clock back through
`renderer.node_frame()`.

* **Collection.** The builder puts every update node a material reaches into
  `NodeProgram::update_before` / `update` / `update_after`, deduplicated by
  reference, in the order `analyze()` meets them. The lists are cached with
  the program, so a steady frame walks three short lists and no graph:
  `steady_frame_builds_nothing` and `STEADY_FRAME_CEILING` do not move. The
  nodes that take part are:
  * a `ComputeFlow` read as a value (§44.1, `FRAME`, before);
  * a `CustomNode` that overrides the new trait methods (§45);
  * a pass whose texture the material samples (below).
* **Order.** `Renderer::draw()` runs each item's update-before list, then its
  update list, before the item is encoded. It runs the update-after list
  once the item is recorded. A pass rendered from an update-before is a
  nested render with its own submit, which happens before the outer pass's
  encoder is submitted. So the GPU sees the order three's does.
* **Guards.** `NodeFrameState::claim()` and `settle()` split three's three
  `update*Node()` methods at the call. `claim` takes the before-stamp early,
  and `settle` puts it back on `false`, or writes the late stamp for the
  other two phases. `OBJECT` always runs.
* **Skeletons.** `skeleton.update()` (`SkinningNode.update()`) runs behind the
  `FRAME` guard of the update map, keyed on the skeleton. This replaces the
  renderer's `frame_skeletons` set, and the compute path replaces
  `frame_computes`. There is one frame notion, not three.

### 57.3 Where a frame starts and ends

Three's animation loop calls `nodeFrame.update()` once per display frame.
The port has no loop that it owns (the viewer, the web shell and the e2e
harness each drive their own). So a frame **opens at the first `render()` or
`render_quad()` after the last one closed**, and **closes at the end of a
render to the screen**: no render target, and not nested inside another
render. A render into a target, whether a pass's scene or a bloom's quads,
belongs to the frame it is part of. `frame_id` counts these frames, and the
`frameId` uniform now reads it too. Before, it counted every `render()`.
`time` and `deltaTime` advance once per frame, as three's do. Under the
pinned clock of the e2e harness they stay 0.

`renderId` is a counter that never resets, taken at the start of every
render and put back when a nested render returns. `info` resets only at the
start of an outermost render (`call_depth == 0`), as three's `info.reset()`
does, which never runs for a nested pass. A nested scene render also clears
`fullscreen_pass` for its own duration and restores it afterwards.

### 57.4 Passes render from `update_before`

`PassNode`, `RttNode` and `BloomNode` are now `Rc` handles. A clone shares
the node, and `Deref` reaches its state. `pass( scene, camera )` takes a
`SceneRef` (`Rc<RefCell<Scene>>`) and a `CameraRef`
(`Rc<RefCell<dyn RenderCamera>>`), and `set_scene` sets them later. That is
the `&mut Scene` the old explicit call borrowed, now shared the way three
shares it. Configuration setters take `&self`.

Three's `PassTextureNode` carries its `passNode`, and an `RTTNode` is its own
texture node. The port's texture node is a bare `Node::Texture`, so the link
sits beside the texture instead. `frame::register_texture_update( texture_id,
&node )` records which node fills which texture, as a `Weak`. After binding
layout, the builder looks up every sampled texture and adds that node to the
program's update-before list.

* A pass links its output and each MRT attachment. It links its depth
  texture only if it made that texture. The deferred example shares one
  depth texture between passes, and the pass that did not make it must not
  claim it.
* A bloom links `horizontal[0]`, the texture its `node()` samples. Its own
  blur also reads that target. The early before-stamp makes the bloom's own
  draw of it a no-op, as three's does for a pass that reaches itself.

**Divergence:** the link is on the texture, not on the node. Any draw that
samples a pass's texture, through any texture node, runs the pass once per
frame. In three, only a draw that samples the pass's `PassTextureNode` does
this. No ladder page samples a pass's target through a plain `texture()`.

The examples now do what their pages do: `renderPipeline.render()`, or
`renderer.render( scene, camera )` for a mesh that samples an RTT. They hold
the scene and camera as `Rc<RefCell<…>>`. `controls_and_camera` then lends
the camera as a `RefMut`. The viewer and the web shell take either kind
through `cameras::CameraMut` and hand `OrbitControls` a
`&mut PerspectiveCamera` as before.

### 57.5 The explicit `render()` is removed

`PassNode::render`, `RttNode::render` and `BloomNode::render` were
`#[deprecated]` forwards for one release and are removed in 0.2.0
(`docs/api.md` decision 10). Each forward did two things: opened the frame if
none was open and marked the node's update-before done for it, through
`Renderer::mark_update_before`, then rendered unconditionally, as the old
call did. Marking before the render — not after — mattered for a bloom's
blur, which samples its own `horizontal[0]` and would otherwise reach the
bloom again from inside itself; the early mark let the draw that samples the
pass later in the frame find it already done. `mark_update_before` and the
`NodeFrameState::mark` primitive it used existed only to support the
forward, and are removed with it.

`GaussianBlurNode`, `AfterImageNode`, `PixelationPassNode`,
`ToonOutlinePassNode` and `SsaaPassNode` are outside #162's list, and keep
their own explicit `render()` — it is a different node's method, not one of
the three this decision covers. `webgpu_postprocessing_afterimage` still
fires `AfterImageNode::render()` by hand for that reason; only its scene
pass moved to the renderer-owned path.

`webgpu_custom_fog_background` used to need the deprecated forward for a
real gap: its composite reads the pass's depth through `getViewZNode()`, and
that depth is multisampled. Three sets `renderTarget.samples =
renderer.samples` in `PassNode.setup()`, while the composite builds; the
port used to set it only in the pass's render, so a pass rendering from
`updateBefore()` found the composite's bind-group layout already made for a
single-sampled texture, and wgpu rejected the bind group. The fix, landed
with this removal: `NodeUpdate::sync_before_build`, a hook every registered
pass/RTT/bloom node gets before `Renderer::node_builder_state` builds a
material's program (the earliest point the builder can see a pass's
textures, and the only point that runs before a draw's bindings are
generated). `PassState::sync_before_build` sets `render_target.samples` from
it, matching what three's `setup()` does during graph analysis;
`nodes::frame::sync_before_build` walks the same `TEXTURE_UPDATES` registry
`texture_update` reads, so it costs nothing on a cache hit.

### 57.6 Gates

* `nodes::frame` unit tests run two frames of two renders with two draws
  each. They check the phase order, `FRAME` once per frame, `RENDER` once per
  render and `OBJECT` per draw. They also check that a phase returning
  `false` runs again, and that a nested render restores the render id.
* `tests/nodes_frame.rs` (GPU, not in CI's no-GPU list) drives the same
  pattern through a real `Renderer`. It uses one `CustomNode` shared by two
  meshes, a render into a target, then a render to the canvas, over two
  frames.
* The full ladder keeps every pixel count. The converted examples render
  their passes from the output quad's draw.

## 59. `SkyMesh`, `toVarIntent()` and `CubeCamera` (`webgpu_sky`)

`webgpu_sky` scales a `SkyMesh` to 450 000 and reflects it in a sphere
through a `CubeCamera` that re-renders the scene into a 256² half-float
`CubeRenderTarget` every frame, with the sphere hidden. `SkyMesh`
(`addons::objects`) is `examples/jsm/objects/SkyMesh.js` node for node. Its
`vertexNode` writes four varyings and pins the box to the far plane, and its
`colorNode` does the Preetham in-scattering, the sun disc and, under an
`If`, a cloud layer. The cloud layer is four octaves of gradient noise from
three inline `Fn()`s, which the port writes as Rust functions that build the
same nodes at each call site. `tests/nodes_sky_wgsl.rs` gates both stages
against three's dump.

### 59.1 `Node::VarIntent`

`pow()`, `mix()` and the other `nodeProxyIntent` functions wrap their result
in `toVarIntent()`. Three's intent var is transparent until an assignment
targets it. Then `VarNode.generate()` takes its `nodeVar.local` branch: a
function-scope `var nodeVarN : T = …;` written where the var is first built,
with no entry in `// vars`. `SkyMesh`'s `Lin` is a `pow()` that the page then
`mulAssign`s, so three's fragment has `var nodeVar0 : vec3<f32> = pow( … );`.

The port decides at construction, not at build. `tsl::to_var_intent()`
builds `Node::VarIntent`, which is always the assigned form. A caller uses
it only where the JS assigns to the result of one of those functions.
Everywhere else the plain node is already three's output. The var counts on
the same `nodeVarN` counter as a hoisted var, as three's `_var` counter
does.

### 59.2 `showSunDisc` is a `bool`

Upstream declares `this.showSunDisc = uniform( 1 )`. A `UniformNode` takes its
type from its value when the material is first built, and the page assigns
`true` before that happens. So three's dump stores it as `nodeUniformN : u32`,
reads it as `nodeVarN = bool( … )` (§31.4) and multiplies it in as
`f32( nodeVarN )`. The port's uniform types are fixed at construction, so
`SkyMesh::show_sun_disc` is a `bool` uniform from the start. That is the type
the one page that sets it gives it. Any non-zero value is `true`.

### 59.3 `CubeCamera` and `CubeRenderTarget`

`CubeRenderTarget::new( size, type )` is the cube texture plus one 2-D
face target with depth. `CubeCamera::new( near, far, renderTarget )` adds six
`PerspectiveCamera( -90, 1, near, far )` children in WebGPU's face
orientation (`coordinateSystem` is fixed, so the constructor sets the
orientation once). `update( renderer, scene )` renders each face into the face
target and copies it into its layer. That is the path
`fromEquirectangularTexture()` already used internally. It regenerates
mipmaps when the cube has them and restores the renderer's target.
`activeMipmapLevel` is not ported. As in three, nothing rendered into a
target is tone mapped, so the cube holds linear radiance. The sphere's
`MeshBasicNodeMaterial( { envMap } )` samples it directly (no PMREM), and the
sphere and the sky reach ACES Filmic together on the canvas pass.

### 59.4 Divergences

All of these are existing §8 classes, applied in the fixture test.

* **Varying names.** Upstream's four `varyingProperty()`s are unnamed, so three
  numbers them `nodeVarying5`–`8`. The port's `varyingProperty` takes a name,
  and `SkyMesh` passes upstream's JS identifiers (`vSunDirection`, `vSunE`,
  `vBetaR`, `vBetaM`). The fixture test maps one set to the other.
* **Usage-promoted temps.** Three emits `let nodeConstN` for every temp read
  twice: the sun direction in the vertex stage, and in the fragment the view
  direction, `cosTheta`, both phase terms, `g2`, the zenith angle, its
  inverse, `Fex`, `L0`, the sun disc colour, the horizon fade, the
  `floor`/`fract`/fade of each `noise()` call, and the region noise's
  argument. The port asks for each with `to_const`, with a comment.
* **`VERTEX_` sub-builds** and **render-struct member order**, as in every
  rung. The fixture test undoes the first on three's side and compares the
  uniform structs by membership.
## 60. `LightProbeNode` and `getShIrradianceAt()` (`webgpu_lightprobe`, `webgpu_lightprobe_cubecamera`)

A `LightProbe` is a light that only adds irradiance. three's
`LightProbeNode.setup()` is one line:

```js
builder.context.irradiance.addAssign( getShIrradianceAt( normalWorld, this.lightProbe ) );
```

where `this.lightProbe` is a `uniformArray()` of nine `vec3`s, laid out as
`vec4`s. Its `update()` copies `sh.coefficients[ i ] * intensity` into that
array every frame. Like an ambient or hemisphere light, it never reaches the
lighting model's `direct()`, so it has no shadow, direction or position.

### 60.1 Where it enters

Every lighting model reads its lights through `phong::setup_light`, which
ports `LightsNode.setupLightsNode()`'s per-light step. `LightKind::Probe`
takes the same early return as `Ambient` and `Hemisphere`: it pushes
`irradiance = irradiance + get_sh_irradiance_at( normal_world(),
light_probe_sh( index ) )` and gives the model no `( lightDirection,
lightColor )` pair. In the PBR flow this `irradiance` becomes
`PhysicalLightingModel.indirect()`'s diffuse term, `irradiance *
BRDF_Lambert( diffuseColor )`, the same place an ambient light goes. Phong,
Lambert and Toon do the same through their own `indirect()`.

`light_probe_sh( index )` is a 9-element `vec4` `uniformArray`. Its
`BufferSource::LightProbe( index )` is filled per draw from
`LightState.sh`, which the renderer fills from `LightObject::sh_intensity()`.
That multiplies by the intensity on the CPU, as `LightProbeNode.update()`
does. The `w` lane is padding.

`get_sh_irradiance_at` is a public TSL function, so `LightProbeHelper` can
call it too. It is Ramamoorthi and Hanrahan's quadratic form, and the
constants include the cosine lobe, so the sum is irradiance rather than
radiance. Each product is built in three's order and its `2.0 * k` constants
are folded the way JavaScript folds them, so the WGSL line matches three's
`webgpu_lightprobe` dump exactly, apart from the buffer's node id.
`tests/nodes_light_probe.rs` asserts that line and checks that it comes after
`irradiance = vec3( 0 )` and after `normalWorld`, never before.

### 60.2 A Phong-family fix that came with it

`setup_phong` used to emit `irradiance = vec3( 0 )` after the light loop
unless the scene had an ambient light. For an ambient light it hoists the
zero ahead of the loop, because `AmbientLightNode` reads `irradiance` before
anything else assigns it. A hemisphere light also adds from inside the loop,
so with a hemisphere light and no ambient light, a Phong, Lambert or Toon
material wiped the hemisphere's contribution. A probe would have been wiped
the same way. The zero now goes ahead of the loop when any
irradiance-only light (ambient, hemisphere, probe) is present.
`phong_hemisphere_without_ambient_keeps_its_irradiance` is the regression
test. No graded rung lit a Phong material that way, which is why the ladder
never saw it.

### 60.3 `LightProbeGenerator`

The addon lives in `src/addons/lights.rs`. Both entry points share one loop
(`Projection`). It weights each texel by its solid angle, projects the
texel onto the nine basis functions, and normalises the weights to sum to
4π. The entry points differ only in how they read a texel and where it sits
on the cube.

- `from_cube_texture()` reads the decoded 8-bit face bytes that
  `CubeTextureLoader` already holds. three draws each image onto a canvas and
  calls `getImageData()`. For the PNG and JPEG cubes this addon accepts, the
  bytes are the same. A cube that is not `UnsignedByteType`, or that has no
  pixels, returns an `Err`. `tools/light_probe_generator_reference.mjs` runs
  three's own generator under node, with a canvas stub that returns the same
  bytes. `tests/addons_light_probe_generator.rs` matches it to 1e-9 on the
  pisa cube and on a patterned cube, in both colour spaces.
- `from_cube_render_target()` reads a cube render target back one face at a
  time, as `readRenderTargetPixelsAsync( ..., faceIndex )` does, and uses
  three's WebGPU face table (`flip = 1`). A cube target's faces are stored
  as the cube camera drew them, which is not the orientation of a
  `CubeTexture`'s images, so its signs differ from `from_cube_texture()`'s.
  The pixel grader cannot see a wrong sign in either table: a wrong band-1
  sign tints the probe sphere by less than the colour threshold. So
  `tests/renderer_cube_camera.rs` (GPU) captures the pisa background with a
  `CubeCamera` and checks that `from_cube_render_target()` agrees with
  `from_cube_texture()` on the same cube, coefficient by coefficient, to 1%
  of the DC term. A sign or axis swapped in the face table fails it.

### 60.4 `CubeCamera`

`webgpu_lightprobe_cubecamera` captures the background with the `CubeCamera`
of §59.3 into a `CubeRenderTarget`, and
`LightProbeGenerator::from_cube_render_target` reads that target's cube back.

### Divergences specific to these rungs

- `LightProbe::copy()` copies the `Light` half (colour, intensity, `sh`) but
  not `Object3D.copy()`'s transform, since the port has no general form of
  that. Both pages copy from a probe the generator made at the origin, and
  then set the position themselves.
- `LightProbeHelper`'s `sh` and `intensity` uniforms are read from the probe
  every draw, as `onBeforeRender()` makes them. Its position and scale half
  is `update()`, which the page calls each frame, because a port object has
  no per-object render hook.
- Several probes are summed in light-list order, which is scene traversal
  order, as in three. Neither page has more than one probe.
## 61. Screen reads: the viewport texture nodes and the framebuffer copy (issue #169)

### 61.1 What three does

`ViewportTextureNode` is a texture read whose `updateBefore()` (update type
`RENDER`) calls `renderer.copyFramebufferToTexture( texture )`. The node
system runs `updateBefore()` from `renderObject()`, just before the first
draw of an object whose material reads the node. `WebGPUBackend
.copyFramebufferToTexture()` ends the open render pass, copies the colour
(or depth) attachment into the texture, and begins a new pass that loads
what is there. The reading object therefore sees everything drawn before it
in the opaque-then-transparent list, and nothing after.

* `viewportSharedTexture()` (`ViewportSharedTextureNode`) is one
  `FramebufferTexture` shared by every such node, `NearestFilter`.
* `viewportTexture()` has a `FramebufferTexture` of its own,
  `LinearMipmapLinearFilter`.
* `viewportDepthTexture()` (`ViewportDepthTextureNode`) is one shared
  `DepthTexture`, `_sharedDepthbuffer`.
* `viewportLinearDepth` is `viewportDepthTexture()` turned into an
  orthographic depth with `cameraNear` and `cameraFar`.
* `viewportSafeUV( uv )` (`ViewportUtils`) falls back to `screenUV` where
  the scene depth at `uv` is in front of the fragment, so a refraction
  offset does not pick up an object that sits in front of the refractor.
* `screenSize` and `screenCoordinate` are `ScreenNode`'s `SIZE` and
  `COORDINATE` scopes.

A material with `backdropNode` set goes into the transparent list. On a lit
material `LightsNode.setup()` replaces `totalDiffuse` with
`backdropAlpha ? mix( direct + indirect, backdrop, backdropAlpha ) :
backdrop`. On an unlit one `setupLighting()`'s `backdropNode` arm does the
same with the diffuse colour.

### 61.2 The port

`src/nodes/display/viewport_texture.rs` has the nodes and
`src/renderer/screen_reads.rs` has the copy. `screen_size` and
`screen_coordinate` are in `tsl.rs` next to `viewport_size`, and
`camera_near` and `camera_far` are render-group uniforms.

The port's `Renderer::draw` builds every draw of a pass before it records
any of them, so it cannot end a pass from inside a draw. Instead, a
viewport node's `update_before` makes a **request**: "copy the colour (or
depth) attachment into this texture before draw `n`", where `n` is the
index of the draw being built. `draw` then records the pass in segments.
It records the draws up to the first request and submits. It runs the
copies requested at that index, then records the next segment with
`LoadOp::Load` on colour and depth. It repeats until the draws run out.
The copy therefore lands where three's does, in three's opaque/transparent
order.

Transmission's `viewportOpaqueMipTexture()` copy, which the port already
had as a split of its own, is now one more request
(`FramebufferCopy::OpaqueFrame`) in the same sorted list. A transmissive
page with a backdrop sphere gets both copies at their own indices.

**A pass that reads nothing records one segment**, exactly as before. No
request means no split, no extra submit and no extra texture. The
steady-frame strip and every rung that does not read the screen are
unchanged.

The request allocates (or resizes) the destination texture, not the copy.
The reading draw's bind group is made right after the request returns, and
it has to name the texture the copy will fill.

### 61.3 The textures

* `viewport_shared_texture` binds a thread-local `FramebufferTexture`,
  `NearestFilter`, so it is unfilterable and the tap is a `textureLoad` at
  `uv * textureDimensions`. That matches three's WGSL.
* `viewport_texture` has a texture of its own with a sampler, read with
  `textureSample`. `generateMipmaps` stays false, so it has one level, as
  in three.
* `viewport_depth_texture` binds a thread-local `DepthTexture` as
  `texture_depth_2d`, read with `textureLoad`.

The guard is per node, as in three. Each `viewportSharedTexture()` call is
its own node, so `webgpu_backdrop`'s eight spheres copy the frame eight
times, and each sphere sees the spheres drawn before it.

### 61.4 `backdropNode`

`MeshBasicNodeMaterial` has `backdrop_node` and `backdrop_alpha_node`, and
the other node materials read them too. A backdrop material goes in the
transparent list (`in_transparent_list`). The blend is `backdrop_blend()` in
`node_material.rs`. On Physical, transmission's own backdrop still wins, as
`PhysicalLightingModel.start()` overwrites `context.backdrop`.

`MeshBasicNodeMaterial.lights` is true in three, so a backdrop on a Basic
material in a lit scene goes through `BasicLightingModel`. The port's Basic
path is lit when an `env_map` is set, or when a backdrop is set and the
scene has lights. That path now starts with three's
`indirectDiffuse = vec4( 0 ).xyz` store.

### 61.5 Divergences

* **One texture per node, not per render target.** Three keeps a texture
  per target the node is drawn into (`getTextureForReference`). The port
  resizes the node's one texture to whichever pass copies into it. A node
  read in two passes of different sizes in one frame reallocates twice a
  frame. A request keeps the `wgpu::Texture` it allocated, so the second
  resize cannot redirect the first copy.
* **A shared texture outlives its renderer.** The shared colour and depth
  textures are thread-locals, so a second renderer on the same thread sees
  the first one's GPU texture on the handle. Each renderer keeps the
  textures it made (`screen_reads::Destinations`, three's per-renderer
  `backend.get( texture )`), and it reuses the handle's texture only when it
  made it. The steady-frame strip, which renders every rung on one thread,
  is the gate.
* **No depth copy under MSAA.** WebGPU copies only between textures of
  equal sample count, and the destination is bound as a single-sampled
  `texture_depth_2d`. Under MSAA the depth copy is skipped, and the reading
  draw sees the depth texture's previous contents. No page on the ladder
  reads depth under MSAA.
* **Occlusion queries go only in the first segment** (§39,
  last bullet). No page on the ladder puts an occlusion test on a pass that
  splits.

### 61.6 Gates

* `tests/nodes_display_wgsl.rs`' `refraction_backdrop_matches_three`
  compares `webgpu_refraction`'s refractor fragment against three's `m06`.
* `hash_blur_loop_matches_three` (#148) now blurs
  `viewportSharedTexture()` at `screenUV`, as `webgpu_backdrop_area` does.
  The `textureLoad`-to-tap normalisation is gone, and the loop is compared
  exactly.
* `webgpu_backdrop` is graded green at 23 pixels, the same as three's own
  frame, and is pixel-identical to it.
* `webgpu_refraction` is ported but `#[ignore]`d: three itself scores 344
  against its own JPEG on this machine, over the limit. The port scores
  336, and 13 pixels differ by more than 2 from three's frame.
* The full ladder keeps every other pixel count. Every rung that reads
  nothing keeps its steady frame, because its pass is still one segment.
## 62. `VelocityNode` and the previous frame (`webgpu_postprocessing_motion_blur`, issue #163)

### 62.1 What three does

`velocity` is a `vec2`: this frame's NDC position minus last frame's.

- **Uniforms.** It owns four:
  - `previousModelWorldMatrix`, in the object group;
  - `currentProjectionMatrix`, `previousProjectionMatrix` and
    `previousCameraViewMatrix`, in the render group.
- **`update()`.** This is an `OBJECT` update. It copies the object's stored
  matrix into the uniform. Once per `frameId` per camera, it rotates the
  camera's current view and projection into the previous ones.
- **`updateAfter()`.** Also `OBJECT`. It stores `object.matrixWorld`.
- **Storage.** A module `WeakMap`. First sight seeds previous with current,
  so a first frame has no motion.
- **`setProjectionMatrix( m )`.** It substitutes the projection that
  `currentProjectionMatrix` records. TRAA uses it to keep its jitter out of
  the velocity.

The previous clip position goes through `positionPrevious`, a varying that
starts as `positionGeometry`. When `builder.needsPreviousData()` is true,
meaning the renderer's MRT has `velocity`, `SkinningNode` reassigns it,
before the current skinning. It skins with `previousBoneMatrices`, a second
storage buffer that `SkinningNode.update()` fills from last frame's
`skeleton.boneMatrices` before it updates the skeleton.

### 62.2 The port

The shader half, `nodes::velocity::velocity()`, builds three's `setup()`
graph. Its four matrices are new `UniformSource`s that the renderer fills.
They are not `ObjectUpdate` closures, because the values come from state the
renderer owns.

The bookkeeping half is `VelocityState`, a field of the renderer's
`NodeFrameState` (§57). This is #154's decision 1, option C. It keeps three
maps, each keyed by id:

- object → last `matrixWorld`;
- camera → `{ frameId, previous and current view and projection }`;
- skeleton → last bone matrices.

`Renderer::draw()` runs `update` before it writes the draw's bindings, and
`update_after` after it records the draw. Both run only when the program's
`reads_velocity` is set. The builder sets that flag when the program binds
any of the four uniforms. A frame with no velocity in it never touches the
store, and `steady_frame_builds_nothing` holds.

The camera history rolls when its `frameId` changes. A frame ends at a render
to the screen (§57.3). A test or tool that renders only into targets
therefore stays inside one frame, and its camera never rolls. The object
history has no such guard: as in three, it moves on every draw that reads
velocity.

**Skinning.** `SkinEntry` has a `previous` flag, part of the program key. The
main pass sets it when the renderer's MRT has `velocity`. With the flag set,
`skinning()` declares a `BufferSource::PreviousBoneMatrices` buffer and skins
`positionPrevious` with it. That code comes first, so the buffer takes the
lower binding, as in three's dump. `update_skeleton` calls
`VelocityState::rotate_bones` just before `skeleton.update()`, inside the
`FRAME` claim. It stores the bones the skeleton is about to overwrite.
`previous_bones` returns them, and seeds itself from the updated bones on
first sight.

**The override.** `Renderer::set_velocity_projection_matrix( Option<Matrix4> )`
is `velocity.setProjectionMatrix()`. It sits on the renderer because the
store does.

### 62.3 Not ported

- `positionPrevious` for instancing, batching and `Line2`.
- The bone-texture path.
- `useVelocity` on shadow casters. `docs/webgpu_postprocessing_motion_blur-progress.md`
  explains why no pixel depends on it.

## 63. `TRAANode` (`webgpu_postprocessing_traa`, issue #165)

### 63.1 What three does

`traa( beauty, depth, velocity, camera )` is a `TempNode` with
`updateBeforeType = FRAME`. It owns two half-float targets: the history,
which also carries a `DepthTexture`, and the resolve target. Its texture node
is `passTexture( this, resolve.texture )`.

- **Jitter.** `setup()` adds an `OnBeforeRenderPipeline` callback and an
  `OnAfterRenderPipeline` callback. They are guarded by
  `renderPipelineState.viewOffsetOwner`, so a pipeline with two TRAA nodes
  jitters only once. The before callback saves the unjittered projection into
  `velocity.setProjectionMatrix()`, then calls `camera.setViewOffset()` with
  the next of 32 Halton (2, 3) offsets, minus 0.5. The after callback clears
  both and advances the index.
- **`updateBefore()`.** It rolls `_previousCameraWorldMatrix` and
  `_previousCameraProjectionMatrixInverse` from last frame's values, then
  writes this frame's near/far, world, inverse world and inverse projection.
  On a size change it seeds the history by copying the beauty. It renders the
  resolve quad, copies the resolve into the history, and copies the scene's
  depth into the history's depth when the sizes match.
- **The resolve.** It finds the closest and farthest depth in the 3×3
  neighbourhood, and reads the velocity at the closest texel. It reprojects
  the history along that velocity. Disocclusion is a depth test of the
  reprojected history depth, rebuilt through the previous camera, and edges
  are exempt from it. It clips the history to the neighbourhood's mean ± γσ,
  then blends the result with a luminance-weighted (flicker-reducing) weight.
  That weight is 5% plus a sub-pixel term, rising with motion.

### 63.2 The port

`nodes::display::traa` builds the same graph, gated against three's dump. The
gate covers the resolve body and its three helper functions,
`subpixelCorrection`, `clipAABB` and `flickerReduction`
(`tests/nodes_display_wgsl.rs`). The graph needed four new pieces:

- **Struct values.** `Node::StructNew` is three's `struct( … )( values )`.
  `Node::StructGet` is `.get( name )`. `sampleCurrentDepth` returns its three
  results this way, as `StructType0`.
- **Texel loads.** `texture_load`, `texture_load_offset` and
  `depth_texture_load` are `textureLoad` at an integer coordinate. On a depth
  texture the result is a bare `f32`, with no `.x`.
- **`all( bvec )`.** The builder passes its argument unformatted, the way it
  already passes the arguments of `dpdx` and `inverseSqrt`, so a `bvec2` is
  not cut to its `.x`.
- **Helpers.** `view_z_to_perspective_depth` and `get_view_position`.

`TraaState` implements `NodeUpdate` and is registered as the updater of the
resolve texture, as `passTexture` makes it one in three. The jitter cannot be
installed from inside the node, because the port has no `setup()`-time handle
on the pipeline. So `TraaNode::attach( &mut RenderPipeline )` installs the two
hooks, behind `RenderPipeline::claim_view_offset()`, which is
`viewOffsetOwner`. `docs/api.md` §8 has the reason.

**Where the previous frame lives.** There are two histories:

- **The velocity attachment's.** This is the global `velocity`'s camera and
  object history in the renderer's `NodeFrameState` (§59). TRAA only holds
  the projection still, through `Renderer::set_velocity_projection_matrix`,
  for the length of the pipeline's render.
- **TRAA's own camera matrices.** These are `SettableValue` uniforms on
  `TraaState`. They roll at the top of `update_before()`, once per frame and
  before the new values are written, which is where three rolls them. The
  history colour and depth are GPU copies at the bottom of the same call,
  after the resolve has read them.

**Order within a frame.** In three, the scene pass is earlier in the frame's
update-before list than TRAA, because `setup()` builds the inputs first.
The port asks for the pass explicitly at the top of `update_before()`, through
`frame::texture_update( beauty )`. The frame guard turns the pass's own later
call into a no-op.

**Copies need usages.** WebGPU fixes a texture's usage at creation. Three's
backend gives every render target `COPY_DST`. Here, to keep every other
rung's textures exactly as they were, a render target opts in with
`set_copy_destination()` (the history only), and a depth texture with
`set_copyable()`, which adds `COPY_SRC | COPY_DST` (the pass's depth and the
history's). Both are `pub(crate)`. `Renderer::copy_render_texture` and
`copy_depth_texture` are whole-texture `copy_texture_to_texture` calls.
`Renderer::init_render_target` is three's `initRenderTarget`. It allocates
the targets after a resize so the restart copy has somewhere to land.

**The first frame is not quite unblended.** With the history seeded from the
beauty, the history and the current colour are equal everywhere. Variance
clipping still moves the history into the neighbourhood's mean ± σ. At a
one-pixel tip of a silhouette, that range does not reach the tip's own
colour, so the tip is blended on the first frame too. This is three's
arithmetic, and `tests/traa_frames.rs` allows for it.

### 63.3 Not ported

- An orthographic camera (`viewZToOrthographicDepth`).
- Logarithmic and reversed depth buffers.
- A beauty node that is an `RTTNode` rather than a pass attachment.
- A `velocity` other than the global one (`builder.context.velocity`).
- `useSubpixelCorrection = false`, `depthThreshold`, `edgeDepthDiff` and
  `maxVelocityLength` as settable properties. They are constants at three's
  defaults.

## 65. `SSRNode` and `SMAANode` (`webgpu_postprocessing_ssr`)

§64 is reserved for the GTAO and denoise nodes, whose branch is not merged.

### 65.1 What three does

`ssr( colorNode, depthNode, normalNode, { metalnessNode, roughnessNode,
camera } )` builds an `SSRNode`, which extends `Node` (not `TempNode`) and
has `updateBeforeType = FRAME`. `camera` is an option, inferred from the
colour pass when omitted. With `stochastic` left `false`, `updateBefore()`
draws up to three quads:

1. **`SSRNode.SSR`** draws into a half-float target. For each metallic pixel
   it reflects the view ray about the normal, clips the ray to the near plane
   and to `maxDistance`, and projects both ends to screen space. It then
   marches from one end to the other in `totalStep` equal steps, where
   `totalStep = max( |xLen|, |yLen| ) · quality` (truncated, at least 1) and
   `xLen`, `yLen` are the ray's screen-space extent. Once the ray is behind
   the depth buffer, a sample closer to the ray than `thickness` (or the
   view-space width of 3 texels, if larger) is a candidate. `Continue()`
   skips a candidate whose normal faces the same way as the reflected ray
   (`dot( viewReflectDir, vN ) >= 0`), `Break()` ends the march on a
   candidate further than `maxDistance` from the surface's plane, and any
   other candidate is the hit. The output is the hit's colour, scaled by
   `intensity`, metalness, a squared distance attenuation and a Fresnel-like
   term. Its alpha is the world-space distance from the surface to the hit.
2. **`SSRNode.Copy`** copies that target into mip 0 of the blur target.
3. **`SSRNode.Blur`** box-blurs the SSR target into mips 1–4. The tap spacing
   is the mip index, and the blur size is `blurQuality`, a build-time
   constant.

Passes 2 and 3 run only when `roughnessNode` is set. The texture node then
samples the blur target at level `roughness² · 4`.

`smaa( textureNode )` is iryoku's SMAA 1x: colour edge detection, then the
blending weights from four 8-step searches and a 160×560 area texture, then
the neighbourhood blend. Each runs into its own half-float target at the
size of the drawing buffer.

### 65.2 The port

`nodes::display::{ssr, smaa}` build the same graphs. The six fragment
shaders, and the page's `RTT` composite that reads the blur chain, are gated
against three's dumps (`tests/nodes_display_wgsl.rs`, fixtures
`webgpu_postprocessing_ssr_m21` … `m32`). Each long `Fn` in three
(the march, `SMAASearchXLeft` … `SMAAArea`) is a `#[inline(never)]` Rust
helper returning a `block`, not one large closure.

The port needed these new pieces:

- **`Node::Continue`** and `tsl::continue_loop()`, three's `Continue()`.
  The builder emits `continue;` exactly where `Break` emits `break;`.
- **`tsl::get_screen_position( viewPosition, projectionMatrix )`.**
- **Rendering into one mip of a render target.**
  - `RenderTarget::set_mip_level_count( n )` allocates the chain. It is the
    port of `blurRenderTarget.texture.mipmaps.push( {}, … )`.
  - `Renderer::set_render_target_level( rt, level )` is
    `setRenderTarget( rt, 0, level )`. The pass draws into a one-level view
    of that mip, and the viewport and scissor are scaled down to it.
  - `active_mipmap_level()` reads the level back, so a node can save it and
    restore it.
  - Sampling the target with `texture_level` reads across the whole chain.
- **`box_blur_with( map, options, sample )`.** SSR's blur pass is
  `boxBlur( ssrTexture, { size, separation } )`, with taps at
  `textureSample` level 0. The new variant takes the sample function. When
  `size` is a constant, the loop bound is now an integer literal
  (`i <= 1`), as three emits it, not `i32( 1.0 )`. The `dof_basic` box-blur
  gate, whose size is a uniform, is unaffected.
- **`Scene::environment_intensity`**, three's `scene.environmentIntensity`.
  It scales the scene environment's PMREM radiance and irradiance through
  the existing `material_env_intensity` uniform. A material's own
  `envMap` is not scaled by it, as in three.

**Order within a frame.** Both nodes run their input's updater at the top of
`update_before()`, as `TraaNode` does (§63). Three's `setup()` produces the
same order. The frame claim turns the input's own later run into a no-op.

**State save and restore.** Both nodes save and then restore these:

- the render target and its mip level;
- the MRT;
- the clear colour and alpha;
- `auto_clear`.

Three's `RendererUtils.resetRendererState()` / `restoreRendererState()` do
the same. SMAA resizes its three targets to `drawing_buffer_size()` every
frame. That is a no-op once the size is current.

**SMAA's lookup textures.** `SMAANode.js` embeds them as base64 PNGs.
`src/nodes/display/smaa_area.png` and `smaa_search.png` are those payloads
base64-decoded, byte-identical PNG files. The port includes them with
`include_bytes!` and decodes them to pixels at runtime, on first use, with
the crate's PNG decoder.

- The area texture is linear-filtered, with no mips.
- The search texture is `NearestFilter`, so its taps are `textureLoad`, as
  in three's dump.

### 65.3 Not ported

All of these are options the page leaves at their defaults:

- `stochastic`;
- `reflectNonMetals`, `binaryRefine` and `screenEdgeFadeBlack`;
- `setHistory()` and `diffuseNode`;
- `resolutionScale ≠ 1`;
- an orthographic camera;
- a logarithmic depth buffer.

`docs/webgpu_postprocessing_ssr-progress.md` has the rung.
