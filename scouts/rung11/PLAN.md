# Rung 11 scout — BatchedMesh (`webgpu_mesh_batch`)

Scouted 2026-09-13. Vendor tree `~/src/vendor/three.js` @ `148ef33` (tag r186),
grader-flags patch applied (`git status` = `M test/e2e/puppeteer.js` only; the
`.puppeteer_profile_rung12/13` and `_dump_rung12/13.mjs` entries belong to the
rung 12 / rung 13 scouts running in parallel and were left alone).

**Headline: `webgpu_mesh_batch` is confirmed 0.0% at r186, four runs, and it is
much smaller than the ladder implies.** Under `WebGPURenderer` there is **no
multi-draw, no indirect draw and no storage buffer anywhere in this example**.
Three issues 453 ordinary `drawIndexed()` calls in one pass against one
pipeline and one bind group, varying only `(indexCount, firstIndex,
firstInstance)`. Everything per-instance lives in three `DataTexture`s read with
`textureLoad`. Two pipelines total, and the second one (`outputColorTransform`)
is already ported and byte-identical to rung 8's minus tone mapping.

---

## 1. Confirming the pick

```
cd ~/src/vendor/three.js
npm run test-e2e-webgpu -- webgpu_mesh_batch webgpu_shadowmap_array
```

Log: `e2e-rung11-candidates.log` next to this file.

```
Diff 0.0% in file: webgpu_mesh_batch (3.7s)
Diff 0.0% in file: webgpu_shadowmap_array (3.7s)
TEST PASSED! 2 screenshots rendered correctly.
```

Repeated three more times (`e2e-repeat-x3.log`): **0.0%, 0.0%, 0.0%**. Four
independent runs, all 0.0%. Rung 0's number stands.

*(Note: the first attempt died with `EADDRINUSE :::1234` because the rung 10 and
rung 12 scouts were driving `test/e2e/puppeteer.js` at the same moment. The
HANDOFF rule "run e2e serially" applies to the grader's fixed port as much as to
the GPU — retry, don't reinterpret.)*

**Alternatives, for the record.** There is exactly one other WebGPU example in
the tree that touches `BatchedMesh`: `webgpu_shadowmap_array` (also 0.0%), but
it uses BatchedMesh only as scenery under a four-light shadow-array setup — it
is a shadow example, and it would drag in `LightShadow` arrays, texture-array
shadow maps and `MeshStandardNodeMaterial` on top of the batching. The other two
`BatchedMesh` examples, `webgl_mesh_batch` and `webgl_batch_lod_bvh`, are WebGL
and out of scope (`renderers/webgl*` is out of scope per HANDOFF §Sources).
`examples/jsm/{interactive/SelectionBox,lighting/vxgi/VXGISceneCollector,utils/SceneOptimizer,misc/Sculptor}.js`
mention BatchedMesh but none is a graded example on its own. **Keep
`webgpu_mesh_batch`.**

---

## 2. The page

Source: `~/src/vendor/three.js/examples/webgpu_mesh_batch.html`, 368 lines.
Reference: `webgpu_mesh_batch.jpg` (copied next to this file).

### 2.1 Scene

| thing | value |
|---|---|
| camera | `PerspectiveCamera( 70, 800/500, 1, 100 )`, `position.z = 30` (line 220–221) |
| renderer | `WebGPURenderer( { antialias: true, forceWebGL: false } )` — **MSAA ×4**, `setPixelRatio(1)`, `setSize(800,500)` |
| background | `new THREE.Color( 0xc1c1ff )` (the `forceWebGL ? 0xffc1c1 : 0xc1c1ff` ternary takes the WebGPU branch). Clear value in the dump is `(0.5332764040016892, 0.5332764040016892, 1.0, 1.0)` — the **linear** value, because the frame target is `rgba16float` |
| lights | **none**, `lights === false` throughout |
| tone mapping | none (`NoToneMapping`); the output pass is un-premultiply → sRGB OETF only |
| geometries | `ConeGeometry(1.0, 2.0)`, `BoxGeometry(2,2,2)`, `SphereGeometry(1.0, 16, 8)` — all three already in the port's `geometries` module |
| the mesh | `new BatchedMesh( 512, 3*512 = 1536, 3*1024 = 3072, material )`, `frustumCulled = false` (line 179) |
| instances | 512, geometry id cycling `i % 3` → cone, box, sphere, cone, … |
| material | one `MeshBasicNodeMaterial` with **`outputNode`** (line 132–136): `vec4( diffuseColor.mul( packNormalToRGB( normalView ).y.add( 0.5 ) ).rgb, diffuseColor.a )`. Not `colorNode`, not `fragmentNode` — a third slot the port does not have |
| addons | `OrbitControls` (**used, see §2.3**), `Inspector` (GUI only but **not pixel-neutral, see §2.4**), `SortUtils.radixSort` (used as the custom sort) |

Draw ranges Three actually assigns (confirmed in the dump's `firstIndex` /
`indexCount` sets — these are the first checkable numbers for the port):

| geometryId | geometry | `start` (index elements) | `count` |
|---|---|---|---|
| 0 | Cone(1,2) | 0 | 192 |
| 1 | Box(2,2,2) | 192 | 36 |
| 2 | Sphere(1,16,8) | 228 | 672 |

`baseVertex` is always 0: `setGeometryAt()` rebases every index as
`vertexStart + srcIndex.getX(i)` (`BatchedMesh.js:781`), so the batch geometry's
index buffer is already absolute.

### 2.2 GUI settings that apply at load

`api` (lines 66–84) is the defaults object; nothing in it is changed before the
single frame. What reaches the render:

- `count = 512` → 512 instances built at load.
- `dynamic = 16` → `animateMeshes()` post-multiplies the **first 16** instance
  matrices by their per-instance rotation matrix, once, before the render.
- `sortObjects = true` → `mesh.sortObjects = true` in `animate()` (line 309).
- `perObjectFrustumCulled = true` (line 310). **453 of 512 instances survive**
  the cull; 59 are dropped.
- `useCustomSort = true` → `mesh.setCustomSort( sortFunction )` (line 311), i.e.
  the `radixSort` path, not `list.sort(sortOpaque)`.
- `opacity = 1` → the `onChange` never fires, so `material.transparent` stays
  `false`, `depthWrite` stays `true`. The material is opaque; `DiffuseColor.w`
  is forced to `1.0` in the shader.
- `webgpu = true`, `randomizeGeometry` — never invoked.

The GUI itself is hidden by `clean-page.js` and contributes no pixels.

### 2.3 `performance.now` pinned to 0 — what still moves

`deterministic-injection.js` makes `Date.now`/`performance.now` return 0 and
fires RAF exactly once. Two things still change state before that one render:

1. **`animateMeshes()`** (line 319): `loopNum = min(512, 16) = 16`. For
   `i in 0..16`: `getMatrixAt(id) → matrix.multiply(rotationSpeeds[i]) →
   setMatrixAt(id)`. Time-independent — it happens once because RAF fires once.
   Do **not** skip it: instances 0–15 are visibly rotated relative to their
   construction matrices.
2. **`controls.update()`** (line 305) with `autoRotate = true`,
   `autoRotateSpeed = 1.0`, called with **no argument**, so
   `_getAutoRotationAngle(null)` returns the frame-rate-independent fallback
   `2π/60/60 * 1.0 = 0.0017453292519943296` rad
   (`examples/jsm/controls/OrbitControls.js:932–944`). One `_rotateLeft` step is
   applied, then `lookAt(target)`.

   Net effect on the camera (verified arithmetic): spherical from `(0,0,30)` is
   `radius 30, phi π/2, theta 0`; theta becomes `-0.0017453292519943296`; the
   camera lands at

   ```
   position = ( -0.052359850976949264, 1.83697019872103e-15, 29.99995430739863 )
   ```

   then `lookAt(0,0,0)`. A ~0.1° yaw. Small, but it rotates *every* pixel of a
   full-frame scene — reproduce it, do not simplify to `position.z = 30` +
   `lookAt(origin)`.

### 2.4 `Math.random` — the trap of this rung

`Math.random` is the seeded `x = sin(seed++) * 10000; x - floor(x)`, seed `π/4`.
Per instance, in creation order, the page draws **11** values:

```
randomizeMatrix : position.x, position.y, position.z,          (3)
                  rotation.x, rotation.y, rotation.z,          (3)
                  scale (0.5 + r*0.5)                          (1)
setColorAt      : new Color( Math.random() * 0xffffff )        (1)
randomizeRotationSpeed : rotation.x, rotation.y, rotation.z    (3)
```

**But the instance loop does not start at draw 0. It starts at draw 5.** Five
`Math.random()` calls are consumed before it by `new Inspector()` (line 228,
constructed before `initMesh()` at line 241): each `Inspector` UI `List` widget
takes an id from `Math.random().toString(36)` —
`examples/jsm/inspector/ui/List.js:11` — and the `Inspector` constructor builds
five of them.

This was verified against the real uploaded texture bytes. With the five leading
draws discarded, a from-scratch replay of the 512×11 sequence (Euler XYZ →
quaternion → `Matrix4.compose`, then `M * R` for `i < 16`) reproduces the whole
`_matricesTexture` payload to **max |Δ| = 9.5e-7** (f32 storage rounding) and the
whole `_colorsTexture` payload to **max |Δ| = 2.9e-8**. Without them, nothing
matches. First instance, for a unit test:

```
matrix[0]  = [-0.579944, 0.209340, -0.199118, 0,
               0.225518, 0.607128, -0.018541, 0,
               0.180590, -0.085901, -0.616291, 0,
             -12.960941, 6.530896, -0.369800, 1]      (column-major, post-animate)
color[0]   = (0.617207, 0.009721, 0.194618, 1.0)      (linear)
color[1]   = (0.341914, 0.376262, 0.806952, 1.0)
```

The colours are **linear**: `new Color(hex)` decodes sRGB→linear
(`ColorManagement`), and `_colorsTexture` is `rgba32float` with
`colorSpace = workingColorSpace`, so no format-level decode happens on the GPU.
The port's `Color::from_hex` already does the sRGB→linear step.

---

## 3. Three's real page, dumped

Taken with a temporary `~/src/vendor/three.js/test/e2e/_dump_rung11.mjs`
(**deleted**; profile dir `.puppeteer_profile_rung11/` deleted), modelled on
`puppeteer.js`: same flags including `--use-angle=vulkan` and no
`--disable-vulkan-surface`, the same `buildInjection` rewrites of
`build/three.{core,module,webgpu}.js`, the same
`deterministic-injection.js` + `clean-page.js` + `networkidle0` + `_videosReady`
+ single-RAF sequence, on its own port (1239) and profile so it cannot collide
with the grader. It hooks
`GPUDevice.prototype.{createShaderModule,createBindGroupLayout,createPipelineLayout,createRenderPipeline*,createBuffer,createTexture,createSampler,createBindGroup}`,
`GPUQueue.prototype.{writeTexture,writeBuffer}`,
`GPUCommandEncoder.prototype.beginRenderPass` and every
`GPURenderPassEncoder` draw/bind/set call, plus `navigator.gpu.requestAdapter`
for the adapter's feature list.

Files next to this plan:

```
vertex-r186.wgsl                            the batch vertex shader
fragment-r186.wgsl                          the batch fragment shader
vertex_outputColorTransform-r186.wgsl       output pass (already ported)
fragment_outputColorTransform-r186.wgsl     output pass (already ported)
dump.json                                   everything, incl. the 453 draws in order
                                            and the base64 payloads of the three DataTextures
actual_full-r186.png                        Three's own 800×500 frame
webgpu_mesh_batch.jpg                       the reference the grader uses
e2e-rung11-candidates.log, e2e-repeat-x3.log
```

Totals: **4 shader modules, 2 render pipelines, 3 bind-group layouts, 8 buffers,
7 textures, 1 sampler, 2 passes.** No compute pipeline. No storage buffer
(`dump.json` → every `createBuffer` usage is `VERTEX`, `INDEX` or `UNIFORM`).
No `drawIndirect` / `drawIndexedIndirect` anywhere.

### 3.1 Passes and draw order

**Pass 0** — the scene, into the `rgba16float` ×4 MSAA target resolving to the
`rgba16float` single-sample target, depth `depth24plus` ×4,
`clear (0.5332764, 0.5332764, 1.0, 1.0)`, depth clear 1.0.

```
setPipeline   renderPipeline_MeshBasicNodeMaterial_17
setBindGroup  0 -> { 0: render uniform buffer (128 B) }
setBindGroup  1 -> { 0: object uniform buffer (128 B),
                     1: _indirectTexture  (r32uint 23x23),
                     2: _matricesTexture  (rgba32float 48x48),
                     3: _colorsTexture    (rgba32float 23x23) }
setIndexBuffer  uint32, offset 0
setVertexBuffer 0 -> position (stride 12, float32x3, @location 0)
setVertexBuffer 1 -> normal   (stride 12, float32x3, @location 1)
drawIndexed( 192, 1,   0, 0, 0 )
drawIndexed( 672, 1, 228, 0, 1 )
drawIndexed( 672, 1, 228, 0, 2 )
drawIndexed(  36, 1, 192, 0, 3 )
...  453 draws in total, firstInstance = 0,1,2,…,452 monotonically
drawIndexed(  36, 1, 192, 0, 452 )
```

**Pass 1** — the output pass: `setPipeline renderPipeline_outputColorTransform_18`,
two bind groups, one vertex buffer (the `QuadMesh` `position`, 36 B),
`draw(3,1,0,0)`, `loadOp: load` on both colour and depth, target `rgba8unorm`,
`sampleCount 1`. Structurally identical to rungs 1–4.

**This is the whole mechanism.** The "batch" is a `for` loop in
`src/renderers/webgpu/WebGPUBackend.js:2124–2145`:

```js
if ( object.isBatchedMesh === true ) {
    const starts = object._multiDrawStarts;
    const counts = object._multiDrawCounts;
    const drawCount = object._multiDrawCount;
    const bytesPerElement = object._multiDrawBytesPerElement;
    for ( let i = 0; i < drawCount; i ++ ) {
        if ( hasIndex === true ) {
            passEncoderGPU.drawIndexed( counts[ i ], 1, starts[ i ] / bytesPerElement, 0, i );
        } else {
            passEncoderGPU.draw( counts[ i ], 1, starts[ i ], i );
        }
        info.update( object, counts[ i ], 1 );
    }
}
```

`firstInstance = i` is the *ordinal in the visible-sorted list*, and that is
exactly what `@builtin(instance_index)` reads in the vertex shader (WebGPU's
`instance_index` starts at `firstInstance`). `RenderObject.getDrawParameters()`
returns early for a batched mesh (`src/renderers/common/RenderObject.js:633`) —
the usual `instanceCount`/`firstVertex` path is bypassed.

### 3.2 Bind-group layouts

```
bgl#7   (group 0, shared by both pipelines)
  0  visibility VERTEX|FRAGMENT|COMPUTE (7)  buffer {}
bgl#13  (group 1, the batch material)
  0  visibility 7  buffer {}
  1  visibility 1 (VERTEX)  texture { sampleType: 'uint'  }   _indirectTexture
  2  visibility 1 (VERTEX)  texture { sampleType: 'float' }   _matricesTexture
  3  visibility 1 (VERTEX)  texture { sampleType: 'float' }   _colorsTexture
bgl#26  (group 1, the output pass)
  0  visibility 2 sampler {}   1  visibility 2 texture {}   2  visibility 7 buffer {}
```

Note: **no samplers for the three data textures** — they are `textureLoad`-only,
and the entries are vertex-visibility-only. `sampleType: 'uint'` is new for the
port's layout builder.

Pipeline state, `renderPipeline_MeshBasicNodeMaterial_17`:
`topology triangle-list`, `frontFace ccw`, `cullMode back`, target
`rgba16float` writeMask 15, depth `depth24plus` / `depthWriteEnabled true` /
`less-equal`, **`multisample { count: 4, alphaToCoverageEnabled: false }`**.

### 3.3 The three DataTextures

All created by `BatchedMesh`'s `_init*Texture()` methods
(`src/objects/BatchedMesh.js:335`, `:355`, `:367`) and uploaded with one
`queue.writeTexture` each per frame, `bytesPerRow = width * bytesPerTexel`
(unaligned — 92 / 768 / 368 bytes; `writeTexture` allows that, `copyBufferToTexture`
would not).

| texture | size rule | for `maxInstanceCount = 512` | format | contents |
|---|---|---|---|---|
| `_matricesTexture` | `size = max(4, ceil(sqrt(n*4)/4)*4)` | `sqrt(2048)=45.25 → 48` → **48×48** | `rgba32float` (`RGBAFormat`+`FloatType`) | 4 texels per matrix = the four **columns** in order, `matrix.toArray(data, instanceId*16)` |
| `_indirectTexture` | `size = ceil(sqrt(n))` | `sqrt(512)=22.6 → 23` → **23×23** | `r32uint` (`RedIntegerFormat`+`UnsignedIntType`) | `indirectArray[k] = instanceId` for the k-th surviving draw; entries past `_multiDrawCount` are stale |
| `_colorsTexture` | same as indirect | **23×23** | `rgba32float`, `colorSpace = workingColorSpace` | `color.toArray(data, instanceId*4)`, white-filled at init |

Verified from the dump: 452 of the 529 indirect entries are non-zero and one
legitimate zero sits at k=5, giving exactly the 453 draws. First entries:
`[270, 8, 497, 508, 383, 0, 254, 251, 416, 509, 64, 211, …]` — that is the
front-to-back ordering, not identity.

### 3.4 What the vertex shader reads (`vertex-r186.wgsl`)

Generated by `src/nodes/accessors/Batch.js` (there is **no `BatchNode.js`** in
r186; the file was renamed to `Batch.js` and is a set of TSL `Fn`s, not a Node
subclass). `NodeMaterial.setupPosition()` calls `batch( object )` at
`src/materials/nodes/NodeMaterial.js:792–794`, and `setupDiffuseColor()` does
`colorNode = batchColor.mul( colorNode )` at `:853–855`.

`WGSLNodeBuilder.getDrawIndex()` returns **`null`**
(`src/renderers/webgpu/nodes/WGSLNodeBuilder.js:1615`), so
`Batch.js:128`'s `batchingIdNode` is `instanceIndex`, never `drawIndex`. The
`drawIndex` / `gl_DrawID` / `WEBGL_multi_draw` machinery is the **WebGL fallback
path only** (`GLSLNodeBuilder.js:1295`, `:1379`) and is out of scope.

The emitted body, verbatim from the dump:

```wgsl
let nodeConst0 = i32( textureDimensions( nodeUniform0, 0 ).x );        // indirect width
let nodeConst1 = ( i32( instanceIndex ) % nodeConst0 );
let nodeConst2 = ( i32( instanceIndex ) / nodeConst0 );
nodeVar0 = textureLoad( nodeUniform0, vec2<i32>( nodeConst1, nodeConst2 ), u32( 0u ) ).x;  // indirectId : u32
varyings.vBatchIndirectId = nodeVar0;

let nodeConst3 = i32( textureDimensions( nodeUniform1, 0 ).x );        // matrices width (48)
let nodeConst4 = i32( ( f32( nodeVar0 ) * 4.0 ) );                     // NB: via f32, not integer *4
let nodeConst5 = ( nodeConst4 % nodeConst3 );
let nodeConst6 = ( nodeConst4 / nodeConst3 );

let nodeConst7 = i32( textureDimensions( nodeUniform2, 0 ).x );        // colors width (23)
let nodeConst8 = ( i32( nodeVar0 ) % nodeConst7 );
let nodeConst9 = ( i32( nodeVar0 ) / nodeConst7 );
nodeVar1 = textureLoad( nodeUniform2, vec2<i32>( nodeConst8, nodeConst9 ), u32( 0u ) );
varyings.vBatchColor = nodeVar1;

positionLocal = position;
nodeVar2..nodeVar5 = textureLoad( nodeUniform1, vec2<i32>( nodeConst5 + k, nodeConst6 ), 0u );  // k = 0..3
nodeVar6 = mat4x4<f32>( nodeVar2, nodeVar3, nodeVar4, nodeVar5 );
positionLocal = ( nodeVar6 * vec4<f32>( positionLocal, 1.0 ) ).xyz;
normalLocal   = normal;
normalLocal   = ( mat3( nodeVar6 ) * ( normalLocal / vec3( dot(c0,c0), dot(c1,c1), dot(c2,c2) ) ) );
varyings.v_normalViewGeometry = normalize( ( render.cameraViewMatrix * vec4( object.nodeUniform6 * normalLocal, 0.0 ) ).xyz );
modelViewMatrix = ( render.cameraViewMatrix * object.nodeUniform8 );
v_positionView  = ( modelViewMatrix * vec4( positionLocal, 1.0 ) ).xyz;
VERTEX_v_modelViewProjection = ( render.cameraProjectionMatrix * vec4( v_positionView, 1.0 ) );
```

Points that matter:

- The batch matrix is applied in **local** space, before the object's
  `modelMatrix` (`object.nodeUniform8`) and `normalMatrix` (`nodeUniform6`).
  Same slot as `instancedMesh()` and `skinning()` in `setupPosition`.
- The normal transform is the **inverse-scale trick**, not `inverse(transpose())`
  as the port's `InstanceNode` path uses: divide by the squared column lengths,
  then multiply by the 3×3. `Batch.js:166–170`. Three re-emits
  `mat3x3<f32>( nodeVar6[0].xyz, nodeVar6[1].xyz, nodeVar6[2].xyz )` **seven
  times inline** in that one statement rather than hoisting it — a node-builder
  divergence worth matching or at least noting.
- `f32( nodeVar0 ) * 4.0` then `i32(...)`: the multiply goes through f32. Exact
  for ids ≤ 2^22; do not "fix" it to an integer multiply silently.
- Varyings: `@location(0) v_normalViewGeometry : vec3<f32>`,
  `@location(1) @interpolate(flat, either) vBatchIndirectId : u32`,
  `@location(2) vBatchColor : vec4<f32>` (**not** flat — interpolating a
  per-draw constant is exact, so this is fine).
- `vBatchIndirectId` is declared and written but **never read** by this
  material's fragment shader. Emit it anyway; it is in the dump.
- There is **no `uv` attribute** in the vertex buffers — only `position` (loc 0)
  and `normal` (loc 1), in that order. (Rung 8's dump had uv first; the
  attribute order is per-program, driven by what the graph reads.)

### 3.5 The fragment shader (`fragment-r186.wgsl`)

```wgsl
DiffuseColor = ( vBatchColor * vec4<f32>( object.nodeUniform3, 1.0 ) );   // materialColor
DiffuseColor.w = ( DiffuseColor.w * object.nodeUniform4 );                 // materialOpacity
DiffuseColor.w = 1.0;                                                      // isOpaque()
Output = max( vec4<f32>( DiffuseColor.xyz, DiffuseColor.w ), vec4<f32>( 0.0 ) );
normalViewGeometry = normalize( v_normalViewGeometry );
normalView = normalViewGeometry;
output.color = vec4<f32>( ( DiffuseColor * vec4<f32>( ( ( ( normalView * vec3<f32>( 0.5 ) ) + vec3<f32>( 0.5 ) ).y + 0.5 ) ) ).xyz, DiffuseColor.w );
```

The `Output = max(...)` line is `NodeMaterial.js:536–542`: the *default*
`basicOutput` is still assigned to the `Output` property even though
`this.outputNode !== null` replaces the returned value
(`NodeMaterial.js:547–549`). The port's `MaterialFlow.emit_output_property`
already models that flag — `output_node` keeps it `true`, unlike `fragment_node`.
Note also the ordering: `normalViewGeometry`/`normalView` are emitted **after**
`Output`, because the custom output node's dependencies are resolved last.

Both stages declare the same `objectStruct`
`{ vec3 materialColor, f32 opacity, mat3x3 normalMatrix, mat4x4 modelMatrix }`
(128 B, matching `buffer#9`), even though the fragment uses only the first two.

---

## 4. Gap list against the port

Against `port` @ `cdf834a` (post-`treewalk`): `src/renderer/mod.rs`,
`src/renderer/{render_list,programs}.rs`, `src/nodes/{node,builder,tsl,wgsl}.rs`,
`src/materials/node_material.rs`, `src/objects/{payload,instanced_mesh}.rs`,
`src/textures/texture.rs`.

### 4.1 Storage buffers — **not needed**

No storage buffer appears anywhere in the dump. The rung needs none. (Rung 12,
`webgpu_compute_points`, is where storage buffers actually arrive.)

### 4.2 Multi-draw indirect — **not needed, and Three does not use it**

Three's WebGPU backend never calls `drawIndexedIndirect` for a `BatchedMesh`; it
loops (§3.1). For completeness, probed from a throwaway crate in the scratch dir
(the port tree was not touched), `wgpu @ ~/src/vendor/wgpu` v30, Vulkan:

```
=== IntegratedGpu Intel(R) Iris(R) Xe Graphics (RPL-U) (Vulkan) Mesa 25.3.6
  MULTI_DRAW_INDIRECT_COUNT                  true
  INDIRECT_FIRST_INSTANCE                    true
  FLOAT32_FILTERABLE                         true
  Rgba32Float: usages=COPY_SRC|COPY_DST|TEXTURE_BINDING|STORAGE_BINDING|RENDER_ATTACHMENT|TRANSIENT_ATTACHMENT
               flags=FILTERABLE|MULTISAMPLE_X2..X16|MULTISAMPLE_RESOLVE|STORAGE_*|BLENDABLE
  R32Uint:     usages=… |STORAGE_ATOMIC  flags=MULTISAMPLE_*|STORAGE_*   (no FILTERABLE — textureLoad only, which is all we need)
  max_texture_dimension_2d = 16384
```

(This wgpu has no `Features::MULTI_DRAW_INDIRECT` constant at all —
`RenderPass::multi_draw_indirect` is always available and emulated as a loop
where the driver lacks it; only the `_COUNT` variants are feature-gated, and
that is `true` here. So even if a later rung wants it, this machine has it.)
Chrome's adapter also lists `chromium-experimental-multi-draw-indirect`, unused.

**The port's gap is not a capability gap. It is that a `Renderable` currently
maps to exactly one draw call.** `Renderer::draw()` (`src/renderer/mod.rs:548–561`)
does `pass.draw_indexed(0..*count, 0, 0..draw.instance_count)` — whole index
buffer, instances `0..N`. BatchedMesh needs a per-renderable list of sub-draws:

```rust
struct SubDraw { first_index: u32, index_count: u32, first_instance: u32 }
```

and `pass.draw_indexed(first_index..first_index+index_count, 0, first_instance..first_instance+1)`.
That is the single load-bearing renderer change, and it is small.

### 4.3 Data textures — **the real new work**

`src/textures/texture.rs` is RGBA8-only: `TextureInner.data: Option<Vec<u8>>`
plus a `wgpu::TextureFormat` used only for render targets, and every path
assumes mipmaps + a sampler. The rung needs a `DataTexture` with:

- typed payloads (`Vec<f32>` and `Vec<u32>`) and formats `Rgba32Float` and
  `R32Uint`;
- `generateMipmaps = false`, `flipY = false`, no mip blit;
- **no sampler at all** — the binding is the texture alone;
- upload via `queue.write_texture` with `bytes_per_row = width * texel_size`
  (no 256 alignment needed, unlike `copy_buffer_to_texture`);
- re-upload when `needsUpdate` is set (once per frame here).

`BindingDesc::Texture` / `TextureSource` (`src/nodes/builder.rs:431–436`) must
grow a data-texture source, and `TextureKind` (`src/nodes/wgsl.rs:59–71`) must
grow `Uint2D → texture_2d<u32>` alongside `Float2D`, `Depth2D`, `Cube`. The
bind-group layout builder in `src/renderer/programs.rs` must emit
`sampleType: Uint` and **vertex-only visibility** for these three entries.

### 4.4 Node-system gaps

- **`texture_size()` / `textureDimensions( t, 0 )`** as a first-class node.
  `src/nodes/wgsl.rs:101` already has a `textureDimensions( t, u32(0) )` helper,
  but it is internal to the nearest-filter `texture_load` path; Three emits
  `textureDimensions( t, 0 ).x` with an `i32(...)` around it and hoists it into a
  `let nodeConstN`.
- **`textureLoad( t, vec2<i32>(x,y), 0u )` with explicit integer texel coords.**
  The port's existing `texture_load` (`src/nodes/wgsl.rs:91–95`) derives texels
  from a clamped UV in `vec2<u32>` — a different formulation. A direct
  integer-coordinate load node is needed.
- **`mat4x4<f32>( v0, v1, v2, v3 )` from four `vec4` loads** — the port has
  `join(Type::Mat3, …)`; needs the Mat4 case.
- **`.mod()` and `.div()` on ints** emitting `%` and `/` with i32 semantics.
- **`output_node`** as a third material slot beside `color_node`,
  `vertex_node`, `fragment_node` in `MeshBasicNodeMaterial`
  (`src/materials/mod.rs:27–50`) and in `materials::setup`
  (`src/materials/node_material.rs:64–120`): keep the whole default fragment
  flow and `emit_output_property = true`, but return the custom node.
- **`packNormalToRGB( n )` = `n * 0.5 + 0.5`** — trivial, but it must be emitted
  in Three's exact shape `( ( normalView * vec3<f32>( 0.5 ) ) + vec3<f32>( 0.5 ) )`.
- `varyingProperty('vec4','vBatchColor')` and `varyingProperty('uint','vBatchIndirectId')`:
  named varyings written from the vertex flow. The port's `to_varying`
  (`src/nodes/tsl.rs:123`) already picks `@interpolate(flat, either)` for
  `U32`/`I32` and smooth otherwise — matches.
- `normalViewGeometry` / `normalView` already exist (`src/nodes/tsl.rs:663–683`).

### 4.5 Per-instance data not via uniform buffers

The port's only per-instance path is `BufferSource::InstanceMatrix`
(`src/renderer/mod.rs:733–742`), a **UNIFORM** buffer of `count*16` floats
indexed by `instance_index`, capped at ~1024 instances by wgpu's
`max_uniform_buffer_binding_size`. Nothing in that path survives here: the
matrices come from a texture, and the index is an indirection through a second
texture. Keep `InstanceMatrix` for rung 2; add a parallel `Batch*` source set.

### 4.6 `BatchedMesh` API surface the example needs

A faithful subset of `src/objects/BatchedMesh.js` (1696 lines; the port needs
maybe 400):

| member | line | needed |
|---|---|---|
| `constructor(maxInstanceCount, maxVertexCount, maxIndexCount, material)` | 192 | yes |
| `_initMatricesTexture / _initIndirectTexture / _initColorsTexture` | 335 / 355 / 367 | yes |
| `_initializeGeometry`, `_validateGeometry` | 380 / 418 | yes |
| `addGeometry(geometry)` | 627 | yes |
| `setGeometryAt(geometryId, geometry)` — attribute copy + **index rebasing** + zero padding | 710 | yes |
| `addInstance(geometryId)` | 561 | yes |
| `setMatrixAt` / `getMatrixAt` | 1071 / 1092 | yes |
| `setColorAt` (lazily creates `_colorsTexture`) | 1109 | yes |
| `getBoundingBoxAt` / `getBoundingSphereAt` — lazily computed per geometryId by walking the batch index buffer | 976 / 1015 | yes (culling depends on them) |
| `setCustomSort` | 486 | yes |
| `onBeforeRender( renderer, scene, camera, geometry, material )` — the cull + sort + `_multiDrawStarts/Counts/_indirectTexture` fill | 1523–1685 | yes, this is the core |
| `MultiDrawRenderList`, `sortOpaque`, `sortTransparent` | 33 / 21 / 27 | yes |
| `setVisibleAt`, `setGeometryIdAt`, `deleteGeometry`, `deleteInstance`, `optimize`, `raycast`, `copy`, `clone`, `dispose`, `toJSON` | 1163, 1201, 826, 879, 1384, 1445, … | **no** |
| `onBeforeShadow` | 1688 | no (no shadows here) |

`onBeforeRender` is called **per render item at draw time** from
`Renderer.renderObject()` (`src/renderers/common/Renderer.js:3721`), i.e. after
the render list is built and sorted — so the `_indirectTexture` upload and the
draw happen in the same frame, in that order. The port's `Renderer::draw()` builds
all `Draw`s first and then records the pass; the `onBeforeRender` equivalent must
run in the build phase, before the bind group is created.

`radixSort` (`examples/jsm/utils/SortUtils.js`, 175 lines) must be ported for
`setCustomSort`: `factor = (2**32-1)/camera.far = 42949672.95`, `list[i].z *= factor`,
`options.reversed = material.transparent` (false here), `get = el => el.z`,
`aux = new Array(maxInstanceCount)`. Keys go through JS `>>>` (ToUint32).
**For an opaque material with depth testing the resulting image is
order-independent**, so an exact radix port is a correctness nicety, not a pixel
requirement — but the *set* of surviving draws is absolutely pixel-critical.

### 4.7 Renderer / backend details

- `Payload::BatchedMesh(BatchedMesh)` alongside `Mesh` / `InstancedMesh`
  (`src/objects/payload.rs:22–29`), with `is_mesh() == true`.
- `Object3D.frustumCulled = false` on the mesh: the scene-level cull in
  `render_list.rs` must be skipped, the per-object cull inside
  `onBeforeRender` must not.
- Index format: Three's backend up-converts `Uint16Array`/`Uint8Array` index data
  to `uint32` on upload (`src/renderers/webgpu/utils/WebGPUAttributeUtils.js:89–91`)
  — the dump's index buffer is 12288 B for 3072 Uint16 elements. The port keeps
  U16 as `IndexFormat::Uint16` (`src/renderer/mod.rs:1162–1169`). Pixel-neutral,
  but `firstIndex` is in *elements* either way; don't accidentally multiply by
  `bytesPerElement` twice. (`_multiDrawStarts` stores **bytes**;
  `WebGPUBackend` divides by `_multiDrawBytesPerElement` to get elements.)
- MSAA ×4 is already in place from rung 2; `antialias: true` here.
- Output pass: `vertex_outputColorTransform-r186.wgsl` /
  `fragment_outputColorTransform-r186.wgsl` diff against rung 8's
  `outputColorTransform_26.*` as *only* the tone-mapping block plus uniform
  renumbering — i.e. it is exactly the port's existing rung-1..4 output program.
  Nothing to do.

---

## 5. Traps

1. **The five Inspector `Math.random()` draws.** §2.4. Without them nothing
   matches, and the failure mode is a completely different (but equally
   plausible-looking) cloud of shapes — the classic silent-wrong-output on this
   stack. State the reason in a comment; it is derived from the *page*, not the
   reference image.

2. **`controls.update()` really moves the camera.** `autoRotate` + the
   `deltaTime === null` fallback rotate by `2π/3600` rad. Rung 1 got away with
   "OrbitControls constructor = `lookAt`" ; rung 11 does not.

3. **`animateMeshes()` mutates the first 16 matrices before the render** —
   `matrix * rotationMatrix`, post-multiply, not pre-.

4. **`firstInstance` is the draw ordinal, not the instance id.** The shader's
   `instanceIndex` indexes the *indirect* texture; the instance id comes back
   out of it. Getting this backwards renders 453 shapes in the wrong places with
   the wrong colours and no error.

5. **Per-object frustum culling is load-bearing: 453 of 512.** The cull runs in
   the mesh's **local** frame:
   `_matrix = projectionMatrix * matrixWorldInverse * this.matrixWorld`, then
   `Frustum.setFromProjectionMatrix(_matrix, coordinateSystem, reversedDepth)`,
   tested against `getBoundingSphereAt(geometryId).applyMatrix4(instanceMatrix)`
   (`BatchedMesh.js:1558–1620`). The bounding sphere is **not** the geometry's
   own `boundingSphere`: it is computed by walking the *batch* index buffer over
   `[start, start+count)` (`:1015–1063`), so it depends on the rebased indices.
   A sphere radius that is off by a rounding step changes the draw count and
   therefore the image.

6. **`WebGPURenderer` never touches `drawIndex` / multi-draw.** If the worker
   reads `Batch.js:128` and `GLSLNodeBuilder.js:1295` without checking
   `WGSLNodeBuilder.getDrawIndex()` (which returns `null`,
   `WGSLNodeBuilder.js:1615`), they will build machinery that Three's own
   WebGPU path does not have and diverge from the dumped WGSL.

7. **There is no `BatchNode.js` at r186.** The file is
   `src/nodes/accessors/Batch.js` and it is a set of TSL `Fn`s, not a Node class.
   Anything found online about `BatchNode` is pre-r1xx.

8. **`outputNode` is not `fragmentNode`.** The default fragment flow *and* the
   `Output = max(...)` assignment still run; only the returned value is
   replaced. The port's `fragment_node` path suppresses both. Emitting the
   custom node through `fragment_node` gives a shader that is missing the
   `Output` property and, more importantly, skips `setupDiffuseColor`'s
   `batchColor.mul(colorNode)` — so every shape comes out white.

9. **Colours are linear, in an `rgba32float` texture with no GPU decode.**
   `new Color(hex)` does the sRGB→linear conversion in JS. If the port uploads
   raw 0–255 sRGB it will be bright and wrong everywhere.

10. **Lots of thin silhouette edges.** The reference is ~450 small shapes on a
    lavender ground — this is the failure class that dropped `webgpu_camera` at
    rung 0. It measured 0.0% four times (Three grading itself), and MSAA ×4 is
    on, but the margin is not as forgiving as rung 8's smooth falloffs. Expect
    the final residue to be silhouette pixels, and do not chase them with
    per-example fudges.

11. **`setGeometryAt` zero-fills the reserved tail** of each attribute and fills
    spare index slots with `vertexStart`. Here `reservedVertexCount ==
    geometry.position.count` (the `-1` default), so there is no intra-geometry
    padding — but the batch geometry's buffers are sized for
    `maxVertexCount = 1536` / `maxIndexCount = 3072` and the unused tail is
    uploaded as zeros. Size the GPU buffers from the maxima, not from what is
    used (the dump's 18432 / 18432 / 12288 byte buffers confirm it).

12. **`f32(id) * 4.0` then `i32()`** in the matrix texel address. Match it.

13. **The 7× inline `mat3x3<f32>( nodeVar6[0].xyz, … )` repetition** in the
    normal transform is what Three emits. If the port hoists it into a temp, the
    arithmetic is identical but the WGSL diff against the dump becomes noisy —
    decide once and record it in `docs/nodes.md §7/§8` like rungs 4 and 5 did.

14. **Don't run the grader concurrently with another worktree.** Both the GPU
    and port 1234 are shared; this scout hit `EADDRINUSE` from a sibling scout.

---

## 6. Step ladder

Six steps. Grade (diff image, not draw counts) at 3, 5 and 6.

**Step 1 — `BatchedMesh` as data, no GPU.**
Port `BatchedMesh` constructor, the three `_init*Texture` size rules,
`_initializeGeometry`/`_validateGeometry`, `addGeometry`, `setGeometryAt`
(attribute copy + index rebasing + zero fill), `addInstance`, `setMatrixAt`,
`getMatrixAt`, `setColorAt`, `getBoundingBoxAt`, `getBoundingSphereAt`. Plus
`DeterministicRandom`-driven scene construction for the example.

*Gate (no GPU, unit tests):*
- texture sizes for `maxInstanceCount = 512` are **48×48 rgba32float**,
  **23×23 r32uint**, **23×23 rgba32float**;
- draw ranges are cone `(0,192)`, box `(192,36)`, sphere `(228,672)`;
- with **five leading `Math.random()` draws discarded**, the 512 composed
  matrices (post-`animateMeshes` for `i<16`) match the dumped
  `_matricesTexture` payload in `dump.json` (`writeTexture[1].bytesBase64`,
  little-endian f32) to ≤ 1e-6, and the 512 colours match
  `writeTexture[2].bytesBase64` to ≤ 1e-7. Instance 0's matrix and colours 0/1
  are printed in §2.4 as a smoke test.

**Step 2 — `onBeforeRender`: cull, sort, indirect fill.**
Port `MultiDrawRenderList`, the local-frame frustum setup, the sort branch, the
`_multiDrawStarts` (bytes) / `_multiDrawCounts` / `indirectArray` fill, and
`radixSort` from `SortUtils.js` behind `setCustomSort`. Camera comes from
step 2's `OrbitControls` one-step `autoRotate` (§2.3).

*Gate:* camera position is
`(-0.052359850976949264, 1.83697019872103e-15, 29.99995430739863)`;
`_multiDrawCount == 453`; the first twelve `indirectArray` entries are
`[270, 8, 497, 508, 383, 0, 254, 251, 416, 509, 64, 211]`; the multiset of
`(firstIndex, indexCount)` pairs matches the 453 `drawIndexed` entries in
`dump.json` → `passes[0].cmds`.

**Step 3 — data textures + `texture_2d<u32>` + the sub-draw loop, on the GPU.**
`DataTexture` (f32/u32 payloads, `Rgba32Float` / `R32Uint`, no mips, no
sampler), `TextureKind::Uint2D`, layout entries with `sampleType: Uint` and
vertex-only visibility, `write_texture` upload, and `SubDraw` in
`Renderer::draw()`. Wire the batch nodes (`textureDimensions`, integer
`textureLoad`, `mat4` from four `vec4`, the two varyings) into
`setupPosition`/`setupDiffuseColor`.

*Gate:* generated `vertex-r186.wgsl` matches Three's line for line except for
documented divergences; the bind-group layout is
`{0: buffer vis 7, 1: uint tex vis 1, 2: float tex vis 1, 3: float tex vis 1}`;
the scene renders 453 correctly placed, correctly coloured shapes. **Grade:**
should already be close — the only thing still missing is the output node, so
expect a flat-shaded (no normal modulation) image that is otherwise right.

**Step 4 — `output_node`.**
Third material slot; keep the default fragment flow and the `Output = max(...)`
property, replace only the returned value. Add `packNormalToRGB`.

*Gate:* generated fragment WGSL matches `fragment-r186.wgsl` including the
statement order (`DiffuseColor`, `Output`, `normalViewGeometry`, `normalView`,
then the output expression).

**Step 5 — the example + e2e entry, first real grade.**
`examples/webgpu_mesh_batch.rs` + the `tests/e2e` entry against
`~/src/vendor/three.js/examples/screenshots/webgpu_mesh_batch.jpg`. MSAA ×4,
clear `Color(0xc1c1ff)`, the existing no-tone-mapping output pass.

*Gate:* diff < 0.1% of 100000 px, and actual/expected/diff looked at by the
director. Compare `actual_full-r186.png` (Three's own 800×500 frame, in this
directory) against the port's pre-downscale frame first — it isolates
geometry/placement errors from the box-filter downscale and JPEG residue.

**Step 6 — tidy and record.**
Fold the divergences into `docs/nodes.md §7/§8` (the inline `mat3x3` repetition,
the `f32 * 4.0` texel address, the u16-vs-u32 index format). Confirm rungs 1–10
are unmoved, run the whole e2e suite **serially**, and note in `RUNGS.md` that
this rung needs neither storage buffers nor indirect draws — rung 12 is where
those actually arrive.

---

## 7. What the HANDOFF rules make hard

- **Addons.** Three are imported. `OrbitControls` is used for one `autoRotate`
  step — port that arithmetic inline (§2.3), not a controls implementation.
  `SortUtils.radixSort` is genuinely used and should be ported (small, 175
  lines, pure). `Inspector` contributes **no pixels but five `Math.random()`
  draws** — the honest port is a documented constant, not an Inspector port.
- **Loaders.** None. No textures on disk, no JPEG decode, no GLTF. This is the
  first rung since rung 1 with zero image assets, which removes the
  zune-jpeg-vs-libjpeg residue entirely.
- **Geometry.** `ConeGeometry`, `BoxGeometry`, `SphereGeometry` — all in the
  merged `geometries` branch. Nothing new.
- **Hand-written WGSL.** None needed; everything comes out of the node builder,
  as the rules require from rung 4 on.

## 8. Vendor tree state

`test/e2e/_dump_rung11.mjs` and `.puppeteer_profile_rung11/` deleted.
`~/src/vendor/three.js` ends at `M test/e2e/puppeteer.js` and nothing else. The
wgpu feature probe was a throwaway crate in the session scratch dir; the `port`
tree was not modified. Nothing committed.
