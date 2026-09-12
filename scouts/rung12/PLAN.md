# Rung 12 scout — `webgpu_compute_points`

Scouted 2026-09-13. Example: `~/src/vendor/three.js/examples/webgpu_compute_points.html`
(184 lines) at `148ef33` (r186) + `handoff/rung0/grader-flags.patch`.
Everything here is read off the real page; nothing is inferred from the reference image.

Files next to this plan:

| file | what |
|---|---|
| `precompute_velocity.compute-r186.wgsl` | the `onInit` compute module (module label `compute`, no `setName`) |
| `update_particles.compute-r186.wgsl` | the per-frame compute module (`setName( 'Update Particles' )`) |
| `PointsNodeMaterial.vert-r186.wgsl` / `.frag-r186.wgsl` | the only scene program Three compiles |
| `outputColorTransform.vert-r186.wgsl` / `.frag-r186.wgsl` | the output pass — **byte-identical to `scouts/rung6/outputColorTransform.*.wgsl`** (verified with `diff`, banner excluded), so the port already emits these |
| `dump-r186.json` | modules, render + compute pipelines, pipeline layouts, bind-group layouts, every `createBuffer`/`createTexture`/`createSampler`/`createBindGroup` descriptor, the pass/cmd list, the submission `order` array, and `requestedLimits` |
| `webgpu_compute_points.jpg` | the grader's reference (copy of `examples/screenshots/`), 400×250 |
| `actual_full-r186.png` | Three's own 800×500 render of the deterministic frame (pre-downscale) |
| `centre-crop-r186.png` | left: reference ×16 crop around (192,117)–(208,133); right: Three's 800×500 ×8 crop around (384,234)–(416,266). This is the entire graded content of the example. |
| `e2e-compute_points-r186.log` | the three e2e runs of §1 |
| `e2e-compute-family-r186.log` | the twelve-example compute-family grading of §1.2 |

Dump method: a temporary `test/e2e/_dump_rung12.mjs` modelled on `puppeteer.js`
(same flags, `--use-angle=vulkan`, no `--disable-vulkan-surface`, port 1277,
profile `.puppeteer_profile_rung12`), same `deterministic-injection.js` +
`clean-page.js` + the three `buildInjection` build-file rewrites, networkidle →
single RAF, plus an `evaluateOnNewDocument` spy wrapping
`GPUDevice.prototype.{createShaderModule,createBindGroupLayout,createPipelineLayout,createRenderPipeline(Async),createComputePipeline(Async),createBuffer,createTexture,createSampler,createBindGroup}`,
`GPUCommandEncoder.prototype.{beginRenderPass,beginComputePass,finish}`,
`GPURenderPassEncoder`/`GPUComputePassEncoder`
`{setPipeline,setBindGroup,setVertexBuffer,setIndexBuffer,draw,drawIndexed,dispatchWorkgroups,end}`,
`GPUQueue.prototype.{writeBuffer,submit}` and `GPUAdapter.prototype.requestDevice`
(for the granted limits). Script and profile deleted afterwards; the vendor tree
is back to only `M test/e2e/puppeteer.js` (the `_dump_rung13.mjs` /
`.puppeteer_profile_rung13` entries belong to a concurrent scout, not to this one).

Result: **6 shader modules, 2 compute pipelines, 2 render pipelines, 4 pipeline
layouts, 5 bind-group layouts, 10 buffers, 4 textures, 1 sampler, 2 compute
passes + 2 render passes, 4 separate command encoders / `queue.submit` calls.**

---

## 1. Is it 0.0%?

**Yes — and the number is meaningless. Read §1.1 before anything else.**

```
cd ~/src/vendor/three.js
npm run test-e2e-webgpu -- webgpu_compute_points
```

Three runs, all identical (`e2e-compute_points-r186.log`):

```
Diff 0.0% in file: webgpu_compute_points (3.7s / 3.5s / 3.6s)
TEST PASSED! 1 screenshots rendered correctly.
```

Matches `handoff/rung0/RUNG0.md` and `rung0/REPIN-r186.md`. No alternative pick
is forced by the letter of the brief: `webgpu_compute_points` *is* the smallest
0.0% example that forces a compute pass + storage buffers + points rendering
(184 lines, one addon which is pure UI, no external assets).

### 1.1 The graded image is four pixels, and Three itself misses all four

The reference is **pure black except a 2×2 grey blob at the dead centre**:

```
reference webgpu_compute_points.jpg, red channel, rows 122..127 × cols 196..203

[[  0   0   2   0   1   3   0   0]
 [  0   0   0   4   1   0   0   2]
 [  0   1   0 141 154   4   1   0]     <- rows 124..125,
 [  0   1   0 159 147   0   3   2]        cols 199..200
 [  1   2   0   0   2   0   0   0]
 [  0   0   1   2   0   3   1   1]]
```

Twelve pixels in the whole 100 000 are non-zero; eight of those are JPEG ringing
at value ≤ 4. `image.js`'s `compare()` (unchanged at r186, `test/e2e/image.js:105`)
counts a pixel as different when `(dr²+dg²+db²)/(255²·3) > 0.1²`, i.e. RGB
distance > 44.17. Consequences, all measured:

* **A pure-black 400×250 render scores 4/100 000 = 0.004% and PASSES**
  (budget is 0.1% = 100 pixels). The only four pixels that can fail are the
  blob's.
* **Three's own 800×500 frame, box-downscaled by `image.js`'s `scale()`
  (`test/e2e/image.js:~150`, a plain 2×2 average at factor ½), also differs from
  its own reference JPEG on exactly those four pixels, at max distance 61.3**:
  `111.5 127.5 / 127.5 111.8` against the JPEG's `141 154 / 159 147`. q95 JPEG
  quantisation of an isolated bright 2×2 blob on black moves it by ~30 per
  channel.

So the graded diff of this example cannot distinguish: the correct render, a
render with the two compute dispatches in the wrong order, a render where the
`onInit` precompute never ran at all (velocities stay 0 → every particle sits at
exactly `(0,0)`), or **a black frame**. Everything passes at 4/100 000.

**Therefore: the graded image must not be this rung's gate.** The gates in §6 are
structural (WGSL text diffs against the dumps next to this file) and numeric
(GPU→CPU readback of the storage buffers compared against a CPU recomputation).
The director should treat "rung 12 green" as meaning "the readback gates pass and
the WGSL matches", not "0.0%".

### 1.2 Compute-family numbers, for a second example with real content

Graded at r186 with the patched flags (`e2e-compute-family-r186.log`), plus the
"how much content is there" measure — the number of reference pixels a pure-black
render would fail, i.e. the real grading budget each example spends:

| example | diff | black-frame fails | lines | extra addons |
|---|---|---|---|---|
| webgpu_compute_points | **0.0%** | **4** (0.0%) | 184 | Inspector (UI only) |
| webgpu_compute_geometry | **0.0%** | 100 000 (100%) | 240 | OrbitControls, GLTFLoader |
| webgpu_compute_particles_rain | **0.0%** | 28 893 (28.9%) | 371 | OrbitControls, BufferGeometryUtils |
| webgpu_compute_particles_snow | **0.0%** | 17 733 (17.7%) | 370 | OrbitControls, TeapotGeometry, GaussianBlurNode |
| webgpu_compute_texture | **0.0%** | 15 876 (15.9%) | 148 | capabilities/WebGPU only |
| webgpu_compute_texture_3d | **0.0%** | 100 000 (100%) | 242 | OrbitControls, Raymarching |
| webgpu_compute_water | **0.0%** | 100 000 (100%) | 643 | DRACOLoader, GLTFLoader, HDRLoader, SimplexNoise |
| webgpu_compute_birds | 0.5% FAIL | — | — | — |
| webgpu_compute_particles | 0.5% FAIL | — | — | — |
| webgpu_compute_rasterizer | 0.4% FAIL | — | — | — |
| webgpu_compute_reduce | 2.9% FAIL | — | — | — |
| webgpu_compute_texture_pingpong | 11.2% FAIL | — | — | — |
| webgpu_storage_buffer | 0.8% FAIL (also on Three's exception list) | — | — | — |

`webgpu_compute_birds`, `_cloth`, `_particles_fluid`, `_sort_bitonic`, `_audio`,
`_rasterizer_ibl` are on Three's own exception list or fail; `webgpu_compute_particles`
(0.5%) is out, so the "obvious" alternative is not available.

**Recommendation to the director:** keep `webgpu_compute_points` as rung 12 — it
is the minimal forcing function for the compute machinery — and add
**`webgpu_compute_texture`** (0.0%, 148 lines, no real addons, 15.9% of pixels
non-black, `OrthographicCamera` + `PlaneGeometry` + `texture()`) as the rung's
*image* gate. It costs one more compute dispatch and a `StorageTexture`
(`texture_storage_2d<rgba8unorm, write>` + `textureStore`), reuses the already-ported
`texture()`/`MeshBasicNodeMaterial` path, and its reference has 4 000× more
gradable content. If a storage *buffer* with teeth is wanted instead,
`webgpu_compute_geometry` (0.0%, 100% content) writes a compute result into a
geometry attribute, but needs GLTFLoader (lands at rung 10) and OrbitControls.

---

## 2. The page

### 2.1 Scene

* `camera = new THREE.OrthographicCamera( -1, 1, 1, -1, 0, 1 )`, `camera.position.z = 1`.
  Note **`near = 0`, `far = 1`**, and the frustum is a unit square regardless of
  the 800×500 aspect, so the NDC square is stretched to the viewport: x spans the
  full 800 px, y the full 500 px. `updateProjectionMatrix()` is only called from
  `onWindowResize` (never fires).
* `scene = new THREE.Scene()`, **`scene.background` is never set** → `null`. The
  scene pass clears to `{ r:0, g:0, b:0, a:0 }` (`dump-r186.json` → `passes[2].colorAttachments[0].clearValue`),
  depth clear 1.0.
* Particle count **300 000**.
* Geometry: a `BufferGeometry` with a single `position` attribute of
  `new Float32Array( 3 )` (one vertex at the origin, 12 bytes) and
  `pointsGeometry.drawRange.count = 1`.
* `mesh = new THREE.Points( pointsGeometry, pointsMaterial )`, `mesh.count = 300000`,
  `scene.add( mesh )`. `mesh.count` is not a standard `Points` property; the
  renderer picks it up at `src/renderers/common/RenderObject.js:623-627`
  (`else if ( object.count !== undefined ) instanceCount = Math.max( 0, object.count )`),
  which with `drawRange.count = 1` gives **`draw( 1, 300000, 0, 0 )`**.
* `renderer = new THREE.WebGPURenderer( { antialias: true, requiredLimits: { maxStorageBuffersInVertexStage: 1 } } )`,
  `setPixelRatio( window.devicePixelRatio )` (in-page DPR is 1 — rung 1's note),
  `setSize( 800, 500 )`. Confirmed by the four 800×500 textures in the dump.
* `renderer.setAnimationLoop( animate )`; `animate()` is `renderer.compute( computeNode ); renderer.render( scene, camera )`.
* `renderer.inspector = new Inspector()` + `createParameters( 'Settings' )` with
  two sliders on `scaleVector`. Pure UI, hidden by `clean-page.js`, `trackTimestamp`
  neutered by the e2e build injection. **Do not port.** No `OrbitControls`.

### 2.2 The compute Fns

**`Update Particles`** (per frame), workgroup size `[64,1,1]` (the default from
`ComputeNode`'s `computeKernel( node, workgroupSize = [ 64 ] )`, padded to
`[64,1,1]`), `count = 300000` → dispatch `ceil(300000/64) = ` **4688 × 1 × 1**
(`src/renderers/webgpu/WebGPUBackend.js:1935-1985`). 4688 × 64 = 300 032
invocations, 32 over, hence the early-return guard. Per invocation:

```
if ( instanceIndex >= count ) { return; }            // ComputeNode.generate, allowEarlyReturns
position  = particle[i] + velocity[i]                // .toVar() -> nodeVar0 : vec2
velocity[i].x = select( abs(position.x) >= limit.x, -velocity[i].x, velocity[i].x )
velocity[i].y = select( abs(position.y) >= limit.y, -velocity[i].y, velocity[i].y )
position  = max( min( position, limit ), -limit )    // AFTER the velocity flips
particle[i] = select( length( pointer - position ) <= 0.1, vec2(0,0), position )
```

`limit` = `uniform( scaleVector )` = `(1,1)`; `pointer` = `uniform( pointerVector )`
= `(-10,-10)` (the mouse handler never fires). The two `select()`s lower to
**statement-level if/else writing a `var<private>`**, not to WGSL `select()` —
see `update_particles.compute-r186.wgsl`.

**`precomputeShaderNode`** (`computeNode.onInit`), same workgroup size and the
same 4688 dispatch, same guard:

```
angle = f32(instanceIndex) * 0.005 * 6.283185307179586
speed = f32(instanceIndex) * 1e-8 + 1e-7
velocity[i] = vec2( sin(angle) * speed, cos(angle) * speed )
```

Literals are printed by JS: `6.283185307179586`, `1e-8`, `1e-7`, `0.005`, `0.1`.
Match those digit strings if diffing WGSL text.

### 2.3 The storage buffers

```js
const particleArray = instancedArray( particlesCount, 'vec2' );  // NodeBuffer_992
const velocityArray = instancedArray( particlesCount, 'vec2' );  // NodeBuffer_993
```

`instancedArray` (`src/nodes/accessors/Arrays.js:47`) → `StorageInstancedBufferAttribute(300000, 2, Float32Array)`
→ `storage( buffer, 'vec2', 300000 )` → `StorageBufferNode`
(`src/nodes/accessors/StorageBufferNode.js`), `access = NodeAccess.READ_WRITE`,
`isAtomic = false`, `global = true`.

GPU buffers (`dump-r186.json` → `buffers[0]`, `buffers[9]`): **2 400 000 bytes each**
(300 000 × 2 × f32), `usage = COPY_SRC | COPY_DST | VERTEX | STORAGE` (`0xAC`, from
`WebGPUBackend.js:2897`), **`mappedAtCreation: true`** — so they are
zero-initialised from the JS `Float32Array` at creation and never `writeBuffer`-ed.
`VERTEX` is there only because a `StorageBufferAttribute` is a `BufferAttribute`.

WGSL shape (runtime-sized array, no count, because `uniform.type === 'storageBuffer'`
suppresses the `, N` suffix at `WGSLNodeBuilder.js:2231`):

```wgsl
struct NodeBuffer_992Struct {
	value : array< vec2<f32> >
};
@binding( 0 ) @group( 0 )
var<storage, read_write> NodeBuffer_992 : NodeBuffer_992Struct;
```

Element access is `NodeBuffer_992.value[ instanceIndex ]`; component assignment is
`NodeBuffer_993.value[ instanceIndex ].x = nodeVar1;` (single-component swizzle
assign; `supports.swizzleAssign` is `false` in WGSL, but one component is legal
and `velocity.xy = vec2(..)` on a `vec2` collapses to a whole-element store).

Access mode per stage, `WGSLNodeBuilder.getNodeAccess()` (`:1256-1280`): in the
compute stage it is the node's own `access` (`read_write`); in **any other stage
it is forced to `read`** regardless (the atomic exception does not apply here).
Hence `var<storage, read>` in the vertex and fragment modules, and
`read-only-storage` in the render bind-group layout.

Binding usage per module:

| module | group 0 | group 1 |
|---|---|---|
| `precompute_velocity.compute` | b0 `NodeBuffer_993` storage read_write; b1 `object` uniform (count, 16 B) | — |
| `update_particles.compute` | b0 `NodeBuffer_992` rw; b1 `NodeBuffer_993` rw; b2 `object` uniform (limit vec2, pointer vec2, count u32; 32 B) | — |
| `PointsNodeMaterial.vert` | b0 `render` uniform (128 B) | b1 `object` uniform (80 B); **b2 `NodeBuffer_992` storage read** |
| `PointsNodeMaterial.frag` | — | **b0 `NodeBuffer_992` storage read**; b1 `object` uniform |

The same GPU buffer appears **twice** in the render pipeline's group-1 layout, at
binding 0 (visibility `FRAGMENT`) and binding 2 (visibility `VERTEX`) — see
`dump-r186.json` → `bindGroupLayouts[3]` (`id: 26`) and `bindGroups` `id: 27`.
Root cause: `NodeBuilder.getSharedDataFromNode()`
(`src/nodes/core/NodeBuilder.js:3304-3316`) reads `sharedNodeData` but **nothing
in the file ever writes to it** (`grep -n sharedNodeData src/nodes/core/NodeBuilder.js`
→ lines 40 and 3306 only), so `sharedData.buffer` is always `undefined` and a
fresh `NodeStorageBuffer` is allocated per stage. `getBindings()` then dedupes by
object identity (`:752`) and keeps both. A single wgpu `BindGroupLayout` cannot
give one resource two binding numbers anyway, so **the port should emit one
binding with visibility `VERTEX | FRAGMENT` and accept the binding-number
divergence** (same class as the rung-2 instance-buffer divergence in
`port/docs/nodes.md §7`). Pixel-neutral.

### 2.4 The material and the render pipeline

`pointsMaterial = new THREE.PointsNodeMaterial()` with
`colorNode = particleArray.element( instanceIndex ).add( color( 0xFFFFFF ) )` and
`positionNode = particleArray.element( instanceIndex )`.

`PointsNodeMaterial extends SpriteNodeMaterial`
(`src/materials/nodes/PointsNodeMaterial.js`). Two things follow:

* `SpriteNodeMaterial`'s constructor sets **`this.transparent = true`**
  (`:94`), and `PointsMaterial`'s defaults do not override it, so the object goes
  in the **transparent** render list and the pipeline gets a blend state:
  `color { src-alpha, one-minus-src-alpha, add }`, `alpha { one, one-minus-src-alpha, add }`
  (`WebGPUPipelineUtils.js:103` gates on `blending !== NormalBlending || transparent !== false`;
  `premultipliedAlpha` is false, so it is the non-premultiplied `NormalBlending` pair).
  `depthWriteEnabled` is still `true`, `depthCompare` `less-equal`.
* `setupVertex( builder )` returns `super.setupVertex( builder )` when
  `builder.object.isPoints` (`:160-170`), i.e. **the plain NodeMaterial MVP path —
  no sprite quad expansion, no `sizeNode`, no `screenDPR`, no `viewportSize`**.
  WebGPU point primitives are 1 px, which is why the example uses `Points` and not
  `Sprite`.
* `setupPositionView()` is overridden to `modelViewMatrix.mul( vec3( positionNode || positionLocal ) ).xyz`
  (`:81-87`) — the vertex WGSL therefore uses `vec3<f32>( NodeBuffer_992.value[ instanceIndex ], 0.0 )`
  directly rather than reading the `positionLocal` var a second time.

Pipeline `renderPipeline_PointsNodeMaterial_17`:

```
topology point-list, frontFace ccw, cullMode back   (cullMode is inert for points but is in the descriptor)
vertex buffers: one, arrayStride 12, @location(0) position : float32x3, stepMode vertex
target rgba16float, blend as above, writeMask 15
depthStencil depth24plus / depthWriteEnabled true / less-equal
multisample count 4, mask 0xffffffff, alphaToCoverageEnabled false
```

`material.alphaToCoverage` defaults true on `SpriteNodeMaterial` but the dump
shows `alphaToCoverageEnabled: false`, so do not turn it on.

Vertex stage in words (`PointsNodeMaterial.vert-r186.wgsl`):

```wgsl
positionLocal = position;                                     // the 1-vertex attribute, (0,0,0)
positionLocal = vec3<f32>( NodeBuffer_992.value[ instanceIndex ], 0.0 );   // positionNode
varyings.nodeVarying4 = instanceIndex;                        // @interpolate(flat, either) u32
modelViewMatrix = render.cameraViewMatrix * object.nodeUniform5;          // nodeUniform5 = modelWorldMatrix
v_positionView  = ( modelViewMatrix * vec4<f32>( vec3<f32>( NodeBuffer_992.value[ instanceIndex ], 0.0 ), 1.0 ) ).xyz;
clip            = render.cameraProjectionMatrix * vec4<f32>( v_positionView, 1.0 );
```

Fragment stage:

```wgsl
DiffuseColor = vec4<f32>( vec3<f32>( NodeBuffer_992.value[ nodeVarying4 ], 0.0 ) + vec3<f32>( 1.0, 1.0, 1.0 ), 1.0 );
DiffuseColor.w = DiffuseColor.w * object.nodeUniform2;        // nodeUniform2 = materialOpacity = 1
Output = max( vec4<f32>( DiffuseColor.xyz, DiffuseColor.w ), vec4<f32>( 0.0 ) );
```

Note `instanceIndex` reaches the fragment stage as a **flat `u32` varying**:
`IndexNode.generate()` (`src/nodes/core/IndexNode.js:88-102`) returns the raw
builtin in `vertex`/`compute` and wraps it in `varying( this )` elsewhere. There is
**no `DiffuseColor.w = 1.0` forcing line** here (the material is transparent), unlike
rungs 1–6.

Uniform members and sizes (all from `dump-r186.json`):

| buffer | size | members |
|---|---|---|
| `bindingBuffer0_object` (precompute) | 16 | `nodeUniform1 : u32` = 300000 (the `countNode`) |
| `bindingBuffer1_object` (update) | 32 | `nodeUniform2 : vec2` = limit `(1,1)`; `nodeUniform3 : vec2` = pointer `(-10,-10)`; `nodeUniform4 : u32` = 300000 |
| `bindingBuffer3_render` (points) | 128 | `cameraProjectionMatrix : mat4x4`, `cameraViewMatrix : mat4x4` |
| `bindingBuffer2_object` (points) | 80 | `nodeUniform2 : f32` = materialOpacity; `nodeUniform5 : mat4x4` = modelWorldMatrix |
| `bindingBuffer5_render` / `bindingBuffer6_object` | 144 / 64 | the output pass (identical to rung 6) |

Member order is **first-use order in the generated code**, not declaration order:
`limit` (`nodeUniform2`) is allocated before `pointer` (`nodeUniform3`) even though
`pointer` is declared first in the Fn, because the `abs(position.x) >= limit.x`
line is emitted first.

### 2.5 Timing, `Math.random`, and how many compute steps run

* **`Math.random` is never called by this example** (`grep -c Math.random` → 0).
  The "seeded PRNG consumption order" problem that bit rungs 1–2 does not exist
  here: the initial velocities come from `instanceIndex` arithmetic, and the
  initial positions come from the zero-filled storage buffer. If the port's
  `range()` or anything else draws from the seeded PRNG on this rung's path, it
  means something is running that Three does not run.
* `performance.now` / `Date.now` return 0; nothing in the example reads them.
* **`init()` does not call compute.** `renderer.compute` appears exactly twice in
  the source: inside `animate()` (the outer node) and inside `computeNode.onInit`
  (the precompute). `await renderer.init()` happens before the RAF.
* RAF fires once ⇒ `animate()` runs once ⇒ `renderer.compute( computeNode )` runs
  once. Because the pipeline cache is cold, `Renderer.compute()`
  (`src/renderers/common/Renderer.js:2877-2975`) calls `onInitFn` inside the loop,
  **after `backend.beginCompute( outer )` has already opened the outer pass**
  (`:2924`), and the re-entrant `renderer.compute( precompute )` opens, dispatches,
  ends and **submits its own command encoder first** (`backend.finishCompute`,
  `WebGPUBackend.js:1994-2002`, one encoder + one `submit` per compute call).

The dumped submission order (`dump-r186.json` → `order`) is therefore:

```
0  beginComputePass computeGroup_998          (outer, "Update Particles")
1  beginComputePass computeGroup_1010         (inner, precompute)
2  setPipeline computePipeline_compute
3  dispatchWorkgroups 4688x1x1 in computeGroup_1010
4  end / 5 finish / 6 queue.submit            <- velocities written FIRST
7  setPipeline computePipeline_compute_Update Particles
8  dispatchWorkgroups 4688x1x1 in computeGroup_998
9  end / 10 finish / 11 queue.submit          <- one particle step
12 beginRenderPass (scene, clear 0,0,0,0 + depth 1.0)
13 setPipeline renderPipeline_PointsNodeMaterial_17
14 draw 1,300000,0,0
15 end / 16 finish / 17 queue.submit
18 beginRenderPass (output, loadOp load)
19 setPipeline renderPipeline_outputColorTransform_18
20 draw 3,1,0,0
21 end / 22 finish / 23 queue.submit
```

**Exactly two compute dispatches in the graded frame: precompute, then one
particle step.** `ComputeNode.getUpdateBeforeType()` returns `NONE` while
`frame.compute === this`, so the `FRAME` update type does not cause a second
dispatch; and the points material's node graph does not contain the ComputeNode.

So the graded state is: `velocity[i] = (sin(a)·s, cos(a)·s)` with
`s = i·1e-8 + 1e-7` ∈ `[1e-7, 3.0e-3]`, and `particle[i] = velocity[i]` (the
velocity flip never triggers because `|velocity| < 1`, the clamp to `±1` is a
no-op, and `|pointer − position| ≈ 14.1 > 0.1`). Every particle is within
**3.0e-3 of the origin in a ±1 NDC square**, i.e. within ±1.2 px of the centre of
an 800-wide viewport — which is exactly the 2×2 white core plus faint halo in
`actual_full-r186.png`:

```
(399,249) (400,249) (399,250) (400,250) = 255 ; (398,250),(401,249) = 191 ; (398,249),(401,250) = 127/128
```

300 000 src-alpha blends of white at alpha 1 saturate every covered sample, so the
core is pure white; the halo is MSAA partial coverage.

---

## 3. Gap list

Against `port/docs/nodes.md` and `port/src/{nodes,renderer,materials,objects,cameras}`
at `port` head `cdf834a` (rung 4 + merged side branches + treewalk). Rung 5 is
green on branch `rung5` but not merged; nothing below depends on it.

**Headline: there is no compute anything in the port.**
`grep -rn 'ComputePass|compute_pipeline|begin_compute|ShaderStages::COMPUTE|STORAGE|Stage::Compute' src/` → zero hits.
`port/docs/nodes.md:251` claims "`Stage::Compute` is in the stage enum and
unreachable" — that is **false**; `src/nodes/builder.rs:23-36` is
`pub enum Stage { Fragment, Vertex }`. Fix the doc line as part of this rung.

### 3.1 NodeBuilder: a third stage

| missing | port location | note |
|---|---|---|
| `Stage::Compute` + `Stage::index() = 2` | `src/nodes/builder.rs:23-36` | `NodeBuilder.stages` is `[StageState; 2]` (`:146`, init loop `:194-198`) → `[StageState; 3]`. `StageState` itself (`:116-128`) needs no new fields except directives (below). |
| `Visibility::compute` | `builder.rs:38-63` (`add`, `stages`, `Self::visible` at `:1100-1105`) | `stages()` currently ORs only `VERTEX`/`FRAGMENT`. The `match stage` arms are exhaustive, so adding the variant breaks loudly — good. |
| compute arm in `assemble()` | `builder.rs:1116-1214` — three places: header block `:1120-1125`, param list `:1173-1188` (compute takes neither attributes nor varyings), and `(attr, ret)` `:1190-1193` → `("@compute @workgroup_size( 64, 1, 1 )", none)` with no `return` and no `VaryingsStruct`/`OutputStruct` | Three's template is `WGSLNodeBuilder._getWGSLComputeCode()` (`src/renderers/webgpu/nodes/WGSLNodeBuilder.js:2673-2716`). |
| the `// system` section | new, between `// structs` and `// uniforms` in Three's order; `var<private> instanceIndex : u32;` | plus the generated first flow line `instanceIndex = globalId.x + globalId.y * ( 64 * numWorkgroups.x ) + globalId.z * ( 64 * numWorkgroups.x ) * ( 1 * numWorkgroups.y );` — literal `64`/`1` are the workgroup dims. |
| a `// locals` (scopedArrays) section | — | empty here; emit the empty section only if matching Three's text exactly matters. |
| compute builtins | `Builtin` enum `src/nodes/node.rs:197-221` | `GlobalInvocationId`, `WorkgroupId`, `LocalInvocationId`, `NumWorkgroups` (all `vec3<u32>`) and `SubgroupSize` (`u32`). **`Type` has no `UVec3`** (`node.rs:18-31`, `wgsl::type_name` `nodes/wgsl.rs:11-26`), and `Type::vector_of` (`node.rs:58-68`) panics for `(U32,3)`. |
| `Builtin::InstanceIndex` must lower differently in compute | `builder.rs:594-600`, `:1161-1172` | vertex/fragment: an entry-point `@builtin(instance_index)` param. Compute: the `var<private> instanceIndex` assigned from `globalId` — i.e. `Node::Builtin` generation branches on `self.stage`. This mirrors `IndexNode.generate()` (`src/nodes/core/IndexNode.js:88-102`). |
| a compute entry to `build()` | `builder.rs:892-987` (`MaterialFlow`, `NodeBuilder::build`) | `build()` is hard-wired to "fragment flow + vertex flow + one `output` + one `position`", and collects only `[UniformGroup::Render, UniformGroup::Object]` (`:958-965`) with `group_index()` hardcoding Render=0/Object=0-or-1 (`:991-1007`). Needs a `ComputeFlow { statements, workgroup_size, count }` → `ComputeProgram { wgsl, groups, cache_key }` path with a single group (Three puts the compute bindings in **group 0**, named `object`, because `bindingsIndexes` is per-builder). |
| a directives mechanism | `builder.rs:1124` emits a literal empty `// directives` | only needed if the port wants `enable subgroups;`. **It should not**: no subgroup op is used, the `subgroup_size` param is dead, and emitting it would require `wgpu::Features::SUBGROUP`. Record as a deliberate divergence. |
| early-return guard | new | `if ( instanceIndex >= object.nodeUniformN ) { return; }` prepended to the flow when `count` is a number (`ComputeNode.generate()` + `allowEarlyReturns = true`, `WGSLNodeBuilder.js:353`). The count is a **`u32` uniform**, not a literal. |
| `Node::Select` lowering inside compute | `builder.rs` (already lowers to if/else writing a result var, per RUNGS rung-5 note) | confirm it works when the result is then stored to a storage element, and that it emits the `( a >= b )` / `( a <= b )` comparison forms in the dump. |
| `.abs()`, `.length()`, `.min()/.max()` on vec2, unary negate on a uniform vec2 | `src/nodes/tsl.rs` | `floor/sign/exp2/length/smoothstep/...` landed on `rung5`; check `abs` and vec2 `min`/`max`. Emitted forms: `abs( x )`, `length( ( a - b ) )`, `max( min( a, b ), ( - b ) )`. |
| single-component swizzle **assignment** into a storage element | new | `NodeBuffer_993.value[ instanceIndex ].x = nodeVar1;` |

### 3.2 Storage buffers

| missing | port location | note |
|---|---|---|
| a storage `BindingDesc` | `builder.rs:76-101` — `BindingDesc::{Uniforms,Texture,Sampler,Buffer}` | `Buffer` is the rung-2 path and is `var<uniform>` only. Needs either a new `Storage { name, source, element_ty, read_only_per_stage }` or a flag on `Buffer`. |
| runtime-sized array declaration | `builder.rs:1068-1071` emits `value : array< mat4x4<f32>, 1000 >` | storage wants `value : array< vec2<f32> >` (no count) and `var<storage, read>` / `var<storage, read_write>` chosen **per stage** from one descriptor (Three: compute = node access, everything else forced `read`). |
| `BufferBindingType::Storage { read_only }` in the layout | `src/renderer/programs.rs:176-220` `layout_entry`, line `:178` lumps `Uniforms | Buffer` into one arm with `BufferBindingType::Uniform` (`:183`) | a storage arm is new. |
| `wgpu::BufferUsages::STORAGE` + renderer-owned persistent buffers | `src/renderer/mod.rs:684-694` (`bind_groups()` calls `node_buffer()` **every draw**), `:740-744` (InstanceMatrix, fresh buffer per frame), `:747-750` (`self.buffers` memoises only `Range`) | a compute-written storage buffer **must never be re-uploaded per draw**. Three's usage set is `COPY_SRC | COPY_DST | VERTEX | STORAGE`; for the port `STORAGE | COPY_SRC | COPY_DST` suffices (`COPY_SRC` is what makes the §6 readback gate possible). |
| zero-initialisation semantics | — | Three uses `mappedAtCreation: true` with the JS typed array, so the buffers are **exactly zero** at first use. wgpu zero-fills by default; an explicit zero upload is fine. The initial positions being exactly `(0,0)` is load-bearing for the graded frame. |
| `instancedArray( count, 'vec2' )` equivalent | `src/nodes/node.rs:159-175` (`BufferSource::{InstanceMatrix, Range}`), `src/nodes/tsl.rs:843-868` | a new `BufferSource::Storage(handle)` plus a `storage_element( handle, index )` TSL entry. |
| the `vec3 → vec4` padding rule | — | not needed here (`itemSize 2`), but `WebGPUAttributeUtils.createAttribute` pads `itemSize === 3` storage attributes to 4 ("WGSL does not support packed vec3 data in storage buffers"). Worth encoding now so rung 13 / `instancedArray(n,'vec3')` does not surprise. |
| **no atomics** | — | `isAtomic` is false everywhere in this example; no `atomic<T>`, no `atomicAdd`, no `var<workgroup>`. Skip them. |

Binding-number divergence (accept, document): the port allocates binding indices
**globally per group** (the index is the position in `GroupState::bindings`,
`builder.rs:138-142`, used by `uniform_declarations()` `:1019,1056` and
`programs.rs:54-58` / `mod.rs:704-715`), and it derives visibility from use
(`Visibility::add(self.stage)` at `:376,416,444-455,488-496`). So the port will
emit **one** `read-only-storage` entry with visibility `VERTEX|FRAGMENT` where
Three emits two. Also note `docs/nodes.md:173`'s claim that uniform buffers get
visibility `7` is already stale — the code emits only the bits used.

### 3.3 Renderer: compute pipelines and passes

| missing | port location | note |
|---|---|---|
| `Renderer::compute( &mut self, node )` | new, beside `render_quad` `src/renderer/mod.rs:362-378` | structurally the same shape as `draw()` `:496-565`: one `create_command_encoder`, `encoder.begin_compute_pass`, `set_pipeline` / `set_bind_group` / `dispatch_workgroups( ceil(count/64), 1, 1 )`, `queue.submit`. Three uses **one encoder + one submit per `renderer.compute()` call**; the precompute's submit precedes the outer one (§2.5). |
| `wgpu::ComputePipeline` cache | `mod.rs:127-128` (`programs: HashMap<u64, Program>` keyed on `cache_key`, `pipelines: HashMap<PipelineKey, RenderPipeline>`) | parallel `compute_programs` / `compute_pipelines` maps. `Program` (`programs.rs:31-37`) has no compute module slot. Precedent for a second pipeline family: `src/renderer/mipmap.rs` + `generate_mipmaps()` (`mod.rs:1030-1137`, `mipmap_pipelines` `:133-134`). |
| a once-only `on_init` hook | new | the port's example code can simply call `renderer.compute(precompute)` before `renderer.compute(update)` on the first frame — but it must be **once**, and in that order, and the rung must prove it (§6 step 4). |
| dispatch-size maths | new | `ceil(count / (wx*wy*wz))`, then the `maxComputeWorkgroupsPerDimension` wrap (`WebGPUBackend.js:1958-1970`). 300000/64 → 4688. |

### 3.4 Points rendering

| missing | port location | note |
|---|---|---|
| `PrimitiveTopology::PointList` | **hardcoded** `TriangleList` at `src/renderer/programs.rs:138-151`; `cull_mode: Some(Face::Back)` at `:147` | `RenderState` (`programs.rs:13-27`) has **no topology field**, so the pipeline cache would alias a points and a triangle pipeline from the same program. Add topology (and blend, §3.5) to `RenderState`. |
| a `Points` object | `src/objects/payload.rs:22-29` `Payload::{None,Mesh,InstancedMesh}`; `render()` asserts "the render list only holds meshes" (`mod.rs:294-296`); `project_mesh` `render_list.rs:205-267` | a `Payload::Points` (or a topology on the material) + a `project_points`. |
| `draw( 1, 300000 )` from a 1-vertex geometry | `ensure_geometry` derives `vertex_count` from `position` and yields 0 without it (`mod.rs:1172-1175`); `GeometryGpu::attribute()` panics outside `position`/`normal`/`uv` (`mod.rs:45-53`) | the instanced draw path itself exists (`Renderable::instance_count` `mod.rs:78`, `pass.draw(0..vertex_count, 0..instance_count)` `:555-561`). |
| `BufferGeometry::draw_range` honoured by the renderer | exists at `src/core/buffer_geometry.rs:209,243-244,352-354`, **never read** by the renderer | `drawRange.count = 1` is how the example forces 1 vertex out of a 1-vertex attribute; with the attribute being exactly 1 vertex it is redundant here, but wiring it is cheap and rung 13 will want it. |
| `object.count` → `instance_count` | — | Three's rule is `RenderObject.js:617-631`: instanced geometry's `instanceCount`, else `object.count` if defined, else 1. |

### 3.5 Blending, transparency, camera

| missing | port location | note |
|---|---|---|
| blend state | `programs.rs:131-136` hardcodes `blend: None` ("Opaque material: three.js emits no blend state") | needs `NormalBlending` non-premultiplied: color `(SrcAlpha, OneMinusSrcAlpha, Add)`, alpha `(One, OneMinusSrcAlpha, Add)`; plus a field on `RenderState`. |
| the unconditional `DiffuseColor.w = 1.0` | `src/materials/node_material.rs:78-80` forces it on an `isOpaque()` assumption | must be dropped for a transparent material — Three emits `DiffuseColor.w = DiffuseColor.w * materialOpacity;` and nothing else. |
| transparent render list | **already there** — `render_list.rs:66-69,82-89,101-103,106-109` (reverse painter sort, opaque then transparent). `docs/nodes.md`'s "no transparent list" gap line is stale. | one object, so ordering is untested here. |
| `OrthographicCamera` through `Renderer::render` | the camera **exists** (`src/cameras/orthographic_camera.rs`, already used as `quad_camera` `mod.rs:142-143`), but `Renderer::render` takes `&mut PerspectiveCamera` (`mod.rs:256`), as do `project_scene` (`:346-357`) and `ProjectCamera::new` (`render_list.rs:140`) | needs a camera abstraction or a second entry point. This is shared with rung 13 and with any later ortho example. |
| `PointsNodeMaterial` / `SpriteNodeMaterial` | `src/materials` has no sprite/points material | for rung 12 only the `Points` branch is needed: `setupVertex` = the plain MVP, `setupPositionView` = `modelViewMatrix * vec3(positionNode)`, `transparent = true`, `alphaToCoverage` **off**, no `sizeNode`. The sprite branch (quad expansion, `screenDPR`, `viewportSize`) is rung 13's. |

### 3.6 Already done / nothing needed

Flat `u32` varyings work end to end (`VaryingDef.flat` `node.rs:232-241`,
`tsl::to_varying` auto-flats `U32`/`I32` at `tsl.rs:117-125`, `@interpolate(flat, either)`
emitted at `builder.rs:1135-1139,1181-1187`) — this is the rung-2 instanced-range
path. `OrthographicCamera` maths, the opaque/transparent lists, the instanced draw
range, use-derived binding visibility, the rgba16float MSAA frame-buffer target +
output pass (`mod.rs:393-421,572-591,1239-1260`) and the output-pass WGSL itself
(byte-identical to rung 6's) all exist. No textures, no samplers beyond the output
pass, no JPEG/PNG decode, no lights, no fog, no tone mapping, no shadows, no
`Math.random`, no external assets.

---

## 4. Traps

* **The image proves nothing (§1.1).** A black frame scores 4/100 000 and passes.
  Three's own frame also misses all four graded pixels. Do not let a green diff
  stand in for a working compute stage — §6's gates 3, 4 and 5 are the rung.
* **`depthCompare` is `less-equal`, and the points land exactly on the far
  plane.** Ortho `near = 0, far = 1`, camera at `z = 1`, particles at `z = 0` ⇒
  view `z = −1` ⇒ NDC depth exactly `1.0`, against a depth clear of `1.0`. With
  `less` instead of `less-equal` **nothing draws at all** — and the resulting
  black frame still passes the grader. Assert the blob's pixels explicitly.
* **Dispatch order: precompute first, then the particle step.** It is not the
  source order; it falls out of `onInit` firing inside the outer
  `Renderer.compute()` after `beginCompute()`, with the inner call submitting its
  own encoder first (§2.5). Running the update before the precompute, or never
  running the precompute, leaves the particles at exactly `(0,0)` — and still
  passes. This is exactly the "one-time `renderer.compute` in init vs per-frame
  compute" trap, and the image cannot see it.
* **Exactly one precompute, ever.** `onInit` fires only when the pipeline cache
  misses (`Renderer.js:2925-2946`). A port that re-runs it every frame is
  pixel-identical here and wrong everywhere else.
* **`Math.random` is never called.** There is no PRNG consumption order to match.
  If the port draws from the seeded PRNG on this rung's path (`range()`,
  anything), something is running that Three does not run — investigate rather
  than shrug, because rungs 1–2 showed how that hides.
* **Storage buffers must be zero at first use.** Three's `mappedAtCreation: true`
  upload of a fresh `Float32Array` guarantees it. `particleArray` is never written
  by the CPU, so the initial positions are exactly `(0,0)` and the first step's
  `position = 0 + velocity`.
* **Float determinism is a non-issue for pixels, not for the readback gate.** The
  velocities are `sin/cos` of `i · 0.005 · 2π` times `i·1e-8 + 1e-7`, magnitudes
  ≤ 3.0e-3 in a ±1 NDC square — under 1.2 px at 800 wide, so no plausible `sin`
  precision difference moves a pixel. But the §6 step-4 readback compares
  300 000 f32 pairs: allow a small relative tolerance there (WGSL does not pin
  `sin` precision), and compare the *CPU* reference in f32, not f64.
* **The same storage buffer is bound twice in Three's render group 1** (binding 0
  visibility FRAGMENT, binding 2 visibility VERTEX) because of the
  `getSharedDataFromNode` bug (§2.3). Do not copy it; emit one binding with
  `VERTEX | FRAGMENT` and record the binding-number divergence.
* **`read_write` in compute, `read` everywhere else.** `getNodeAccess()` forces
  `read` outside compute regardless of the node's access. A `read_write` storage
  binding in a vertex stage also needs `maxStorageBuffersInVertexStage` — which is
  why the example passes `requiredLimits: { maxStorageBuffersInVertexStage: 1 }`.
  On wgpu, check `Limits::max_storage_buffers_per_shader_stage` and that the
  vertex stage is allowed a storage binding at all on this adapter; the default
  downlevel/WebGL-ish limits set it to 0.
* **Runtime-sized array, not a fixed count.** `array< vec2<f32> >` with no `, N`.
  The rung-2 uniform path emits `array< T, N >`; reusing it here gives a uniform
  buffer, a 64 KiB limit blowout at 2.4 MB, and a validation error.
* **`transparent = true` on `PointsNodeMaterial`.** Blending on, `depthWrite`
  still on, `alphaToCoverage` off, and **no `DiffuseColor.w = 1.0` line**. With
  blending off the 300 000 overlapping points still saturate to white, so this
  one is pixel-neutral *here* — which means it will go unnoticed until rung 13.
* **`point-list` + `cullMode: back`.** Three leaves the cull mode in the
  descriptor; it is inert for points. Do not "fix" it into `None` if you are
  diffing descriptors, but do not let `Face::Back` reach a triangle pipeline it
  should not.
* **`enable subgroups;` and the `subgroup_size` / `local_invocation_id` params are
  dead code in both compute modules.** Three emits them because the adapter
  reports the feature. Omit them; record as divergence.
* **Uniform member order is first-use order**, and the compute bindings live in
  **group 0** while the render object bindings live in group 1. Pixel-neutral so
  long as the writer and the struct come from the same list (`docs/nodes.md §4`).
* **`docs/nodes.md:251` is wrong** about `Stage::Compute` existing. Fix it.
* **One GPU, two worktrees.** `handoff/RUNGS.md`: run e2e serially. A concurrent
  d33 e2e run was in flight during this scout (`EADDRINUSE :1234` on the first
  attempt — `puppeteer.js:84` hardcodes the port, so two grader runs cannot
  overlap at all).

---

## 5. Addons and external assets

* `three/addons/inspector/Inspector.js` — `renderer.inspector = new Inspector()`
  plus `createParameters( 'Settings' )` and two sliders on `scaleVector`. Pure UI:
  `clean-page.js` hides `.three-inspector`, and the e2e build injection replaces
  `this.trackTimestamp = ( parameters.trackTimestamp === true );` with a
  permanently-false getter so the profiler cannot crash in software mode. The
  sliders' `onChange` never fires, so `scaleVector` stays `(1,1)`. **Do not port.**
* **No `OrbitControls`, no loaders, no textures, no models, no fonts.** Nothing in
  `examples/` outside the HTML is read. The entire cost of this rung is the
  compute stage, storage buffers and point rendering.

---

## 6. Step ladder

Six steps. Gates 3, 4 and 5 are the real ones; gate 6's diff is a formality
(§1.1). Nothing here needs rung 5.

1. **Points, ortho, blend — no compute.** Port the scene with the particle
   positions coming from the **existing rung-2 uniform buffer path**
   (`BufferSource`), filled on the CPU with a small, spread-out pattern
   (e.g. 64 positions on a known grid, `mesh.count = 64`). Add:
   `Payload::Points` + `project_points`, `PrimitiveTopology::PointList` and
   `blend` on `RenderState`/`create_pipeline`, `PointsNodeMaterial`'s `Points`
   branch, `draw_range`/`object.count` → `draw(1, N)`, `OrthographicCamera`
   through `Renderer::render`, and the dropped `DiffuseColor.w = 1.0`.
   **Gate (ungraded):** a test that renders 800×500 and asserts each of the 64
   points lands on its predicted pixel, computed from the ortho projection in the
   test itself — not from the image. Then set one position to `z = 0` exactly and
   assert it still draws: that is the `less-equal` far-plane trap, caught before
   any compute exists.

2. **Storage buffers, read-only, CPU-written.** Replace the uniform buffer with a
   renderer-owned `wgpu::Buffer` (`STORAGE | COPY_SRC | COPY_DST`), declared
   `var<storage, read>` with a runtime-sized `array< vec2<f32> >`, bound once with
   visibility `VERTEX | FRAGMENT`, written once from the CPU, and never
   re-uploaded per draw.
   **Gate:** step 1's pixels are unchanged, *and* the generated vertex/fragment
   WGSL diffs clean against `PointsNodeMaterial.vert-r186.wgsl` /
   `.frag-r186.wgsl` modulo (a) the banner, (b) the binding numbers, (c) the
   `VERTEX_nodeVar*` / `VERTEX_v_modelViewProjection` sub-build temps already
   listed in `docs/nodes.md §7`. If it goes black, it is a validation error
   (visibility, `read` vs `read_write`, `maxStorageBuffersInVertexStage`), not a
   shading bug.

3. **Compute stage in the builder, no GPU.** `Stage::Compute`, the compute
   builtins, `Type::UVec3`, the `// system` section, the `instanceIndex` lowering,
   the `u32` count uniform + early-return guard, and a `ComputeFlow`/`ComputeProgram`
   entry to `build()`. Generate the precompute Fn.
   **Gate (no GPU, a string test):** the generated WGSL diffs clean against
   `precompute_velocity.compute-r186.wgsl` modulo the banner, `enable subgroups;`,
   the `subgroup_size`/`local_invocation_id` params, and binding numbers. Literals
   `6.283185307179586`, `1e-8`, `1e-7`, `0.005` must print identically.

4. **Compute pipeline, pass, dispatch, readback.** `Renderer::compute`, the
   compute pipeline cache, `begin_compute_pass` + `dispatch_workgroups(4688,1,1)`
   + its own encoder and submit. Run the precompute against a 300 000-element
   `velocityArray`.
   **Gate (the real one):** copy the buffer to a `MAP_READ` staging buffer and
   compare all 300 000 `vec2` against a CPU reference computed in **f32** with the
   same expression, within a small relative tolerance. Then assert
   `ceil(300000/64) == 4688` and that 300 032 − 300 000 = 32 tail invocations wrote
   nothing past the end. This is the only thing that can tell a working compute
   stage from a no-op, because the graded image cannot.

5. **`Update Particles`.** The `select()` lowering to if/else, `read_write` access,
   single-component swizzle stores into a storage element, `abs`/`length`/vec2
   `min`/`max`/negate, the two `vec2` uniforms in first-use order.
   **Gate:** WGSL diffs clean against `update_particles.compute-r186.wgsl` (same
   exclusions as step 3); **and** after precompute-then-one-update, a readback of
   `particleArray` equals the CPU reference `velocity[i]` elementwise, and
   `velocityArray` is **unchanged** (no flip triggers). Swap the two dispatches on
   purpose once and watch the readback fail while the graded diff stays at 0.0% —
   that is the §4 ordering trap, demonstrated.

6. **Wire the example and grade it — then grade something that can fail.**
   `examples/webgpu_compute_points.rs` + the e2e entry;
   `cargo test -p three-rs --test e2e -- --nocapture --test-threads=1` (serially;
   rungs 1–4 must stay at 0 / 45 / 0 / 1, and rung 5 at 4 if it has merged).
   **Gate:** not just "≤ 100/100 000". Assert explicitly that the downscaled
   400×250 output is black everywhere except a 2×2 block at rows 124–125 × cols
   199–200 whose values lie in 100–200 (Three's own box-downscaled frame is
   `111.5 127.5 / 127.5 111.8`; the JPEG reference is `141 154 / 159 147`).
   Then, for an honest image gate on the same machinery, take
   **`webgpu_compute_texture`** (§1.2: 0.0%, 148 lines, 15.9% non-black) as a
   companion: it adds only a `StorageTexture`
   (`texture_storage_2d<rgba8unorm, write>` + `textureStore`) and a second compute
   dispatch, and reuses the already-ported `texture()` /
   `MeshBasicNodeMaterial` / `PlaneGeometry` path. The director decides whether
   that lands inside rung 12 or becomes rung 12b.
