# Rung 13 scout — `webgpu_tsl_galaxy`

Scouted 2026-09-13. Example: `~/src/vendor/three.js/examples/webgpu_tsl_galaxy.html`
(vendor at `148ef33`, tag r186, `grader-flags.patch` applied). Everything here is
read off the real page; nothing is inferred from the reference image.

Files next to this plan:

| file | what |
|---|---|
| `e2e-webgpu_tsl_galaxy.log` | Three's own e2e run of this example on this machine |
| `vertex-r186.wgsl` / `fragment-r186.wgsl` | the only scene program Three compiles (`SpriteNodeMaterial`) |
| `vertex_outputColorTransform-r186.wgsl` / `fragment_outputColorTransform-r186.wgsl` | the output pass |
| `dump.json` | every `createShaderModule` / `createBindGroupLayout` / `createPipelineLayout` / `createRenderPipeline` / `createTexture` / `createSampler` / `createBuffer` descriptor, the two render passes with their command streams, every `queue.writeBuffer`, the head/tail floats of every `mappedAtCreation` buffer, and the device limits |
| `webgpu_tsl_galaxy.jpg` | the grader's reference (copy of `examples/screenshots/`) |
| `actual_full-r186.png` | Three's own 800×500 render of the deterministic frame (pre-downscale) |

Dump method: a temporary `test/e2e/_dump_rung13.mjs` modelled on `puppeteer.js`
(same flags incl. `--use-angle=vulkan`, same `deterministic-injection.js` +
`clean-page.js` + the three `buildInjection` rewrites, networkidle → single RAF,
profile `.puppeteer_profile_rung13`, server on port 1237), plus an
`evaluateOnNewDocument` spy wrapping the `GPUDevice` / `GPUQueue` /
`GPUCommandEncoder` / `GPURenderPassEncoder` / `GPUBuffer` methods above. A
second temporary `test/e2e/_probe_rung13.mjs` counted `Math.random` calls and
captured stack traces for the first eight (§2.4). Both scripts and the profile
are deleted; the vendor tree is back to only `M test/e2e/puppeteer.js`
(verified with `git status --short`).

Result: **4 shader modules, 2 render pipelines, 2 bind-group layouts, 0 compute
pipelines, 4 textures, 1 sampler, 14 buffers, 2 render passes, 1 draw call.**

---

## 1. It is 0.0% at r186

```
cd ~/src/vendor/three.js && npm run test-e2e-webgpu -- webgpu_tsl_galaxy
```

```
Diff 0.0% in file: webgpu_tsl_galaxy (4.1s)
TEST PASSED! 1 screenshots rendered correctly.
```

Full log: `e2e-webgpu_tsl_galaxy.log`. Same number rung 0 recorded
(`handoff/rung0/RUNG0.md`) and the re-pin confirmed. **No alternative example is
needed.**

(Note for whoever runs this: `puppeteer.js` binds port 1234 unconditionally. A
first attempt died with `EADDRINUSE` because another worktree's session was
mid-run on the same vendor tree. `handoff/RUNGS.md`'s "run e2e serially" is not
just about the GPU — it is one TCP port.)

---

## 2. The page

### 2.1 Scene

* `PerspectiveCamera( 50, 800/500 = 1.6, 0.1, 100 )` at `(4, 2, 5)`.
  `OrbitControls( camera, dom )` is constructed with `enableDamping = true`,
  `minDistance 0.1`, `maxDistance 50`, and its initial `update()` points the
  camera at its default target `(0,0,0)`; `animate()` calls `update()` again,
  which with no input moves nothing. The dumped view matrix is exactly
  `lookAt(0,0,0)` with up `(0,1,0)` — column 3 is `(0, 0, -6.708204, 1)`,
  `|(4,2,5)| = sqrt(45) = 6.708204`. **Port it as `camera.lookAt(0,0,0)`; do not
  port OrbitControls.**
* `scene.background = new Color( 0x201919 )` → clear value
  `(0.014443843592229466, 0.00972121731707524, 0.00972121731707524, 1)` — the
  sRGB→linear conversion on the CPU, as at rungs 1/6. **Alpha of the clear is 1**,
  which matters in §4.6.
* One object: `new InstancedMesh( new PlaneGeometry( 1, 1 ), material, 20000 )`.
  **20000 particles, drawn as one `drawIndexed( 6, 20000 )`.**
  `setMatrixAt` is never called, but `InstancedMesh`'s constructor fills
  `instanceMatrix` with identity (`src/objects/InstancedMesh.js:102-106`), and
  the dumped 1 280 000-byte instance buffer is identity matrices.
* `renderer.inspector = new Inspector()` plus `createParameters('Parameters')`
  and three `gui.add*` calls. Pure UI — but **not pixel-neutral**, see §2.4.
* `WebGPURenderer({ antialias: true })`, `setPixelRatio(1)` (the grader sets no
  `deviceScaleFactor`), `setSize(800, 500)`.

### 2.2 The TSL graph, node by node

Imports actually used: `color, cos, float, mix, range, sin, time, uniform, uv,
vec3, vec4, TWO_PI`. **No `hash`, no `smoothstep`, no colour-ramp node, no
`Fn()`, no `If`, no `Loop`, no texture of any kind.** The whole example is
arithmetic over four `range()` attributes.

```js
const size          = uniform( 0.08 );                       // float uniform, object group
material.scaleNode  = range( 0, 1 ).mul( size );             // range #4 (see §2.4)

const radiusRatio   = range( 0, 1 );                         // range #2
const radius        = radiusRatio.pow( 1.5 ).mul( 5 ).toVar();

const branches      = 3;
const branchAngle   = range( 0, branches ).floor().mul( TWO_PI.div( branches ) );  // range #1
const angle         = branchAngle.add( time.mul( radiusRatio.oneMinus() ) );

const position      = vec3( cos( angle ), 0, sin( angle ) ).mul( radius );
const randomOffset  = range( vec3( -1 ), vec3( 1 ) ).pow3().mul( radiusRatio ).add( 0.2 );  // range #3
material.positionNode = position.add( randomOffset );

const colorInside   = uniform( color( '#ffa575' ) );         // vec3 uniform
const colorOutside  = uniform( color( '#311599' ) );         // vec3 uniform
const colorFinal    = mix( colorInside, colorOutside, radiusRatio.oneMinus().pow( 2 ).oneMinus() );
const alpha         = float( 0.1 ).div( uv().sub( 0.5 ).length() ).sub( 0.2 );
material.colorNode  = vec4( colorFinal, alpha );
```

Node inventory, by TSL name: `uniform`, `color`, `float`, `vec3`, `vec4`,
`range`, `mix`, `cos`, `sin`, `time`, `uv`, `TWO_PI`, and the methods
`.mul .add .sub .div .pow .pow3 .floor .oneMinus .toVar .length`. Plus what
`SpriteNodeMaterial` pulls in behind the example's back (§2.3):
`modelViewMatrix`, `modelWorldMatrix`, `positionGeometry`, `materialRotation`,
`rotate` (`RotateNode`, vec2 path → `mat2x2`), `vec2`, and
`NodeMaterial.setupPosition`'s `instancedMesh()`.

The generated vertex stage (verbatim shape, from `vertex-r186.wgsl`):

```wgsl
positionLocal = position;
nodeVar0      = mat4x4<f32>( nodeAttribute0, nodeAttribute1, nodeAttribute2, nodeAttribute3 );
positionLocal = ( nodeVar0 * vec4<f32>( positionLocal, 1.0 ) ).xyz;          // dead
normalLocal   = normal;
normalLocal   = normalize( ( transpose( tsl_inverse_mat3( mat3x3<f32>( nodeVar0[0].xyz, nodeVar0[1].xyz, nodeVar0[2].xyz ) ) ) * normalLocal ) );  // dead
nodeVar1      = ( ( floor( nodeAttribute4.x ) * ( 6.283185307179586 / 3.0 ) ) + ( render.nodeUniform5 * ( 1.0 - nodeAttribute6.x ) ) );
nodeVar2      = ( pow( nodeAttribute6.x, 1.5 ) * 5.0 );
nodeVar3      = ( ( vec3<f32>( cos( nodeVar1 ), 0.0, sin( nodeVar1 ) ) * vec3<f32>( nodeVar2 ) )
                + ( ( ( ( nodeAttribute7.xyz * nodeAttribute7.xyz ) * nodeAttribute7.xyz ) * vec3<f32>( nodeAttribute6.x ) ) + vec3<f32>( 0.2 ) ) );
positionLocal = nodeVar3;                                                     // dead
varyings.nodeVarying4 = nodeAttribute6;    // the whole vec4, not just .x
varyings.nodeVarying5 = uv;
modelViewMatrix = ( render.cameraViewMatrix * object.nodeUniform13 );
nodeVar5 = ( modelViewMatrix * vec4<f32>( nodeVar3, 1.0 ) );
nodeVar6 = cos( object.nodeUniform14 );    // materialRotation = 0
nodeVar7 = sin( object.nodeUniform14 );
v_positionView = vec4<f32>( ( nodeVar5.xy + ( mat2x2<f32>( nodeVar6, nodeVar7, ( - nodeVar7 ), nodeVar6 )
                 * ( position.xy * ( vec2<f32>( length( object.nodeUniform13[ 0u ].xyz ), length( object.nodeUniform13[ 1u ].xyz ) ) * vec2<f32>( ( nodeAttribute15.x * object.nodeUniform16 ) ) ) ) ) ), nodeVar5.zw );
VERTEX_nodeVar8 = ( render.cameraProjectionMatrix * v_positionView );
varyings.builtinClipSpace = VERTEX_v_modelViewProjection;
```

Points to copy exactly:

* `TWO_PI.div( branches )` is emitted as the **expression**
  `( 6.283185307179586 / 3.0 )`, not folded to a constant.
* `.pow3()` is `( ( a * a ) * a )`, not `pow( a, 3.0 )`.
* `.pow(1.5)` / `.pow(2.0)` are WGSL `pow`.
* A scalar × vector is an explicit splat: `vec3<f32>( nodeVar2 )`,
  `vec2<f32>( x )`.
* `v_positionView` here is a **`vec4`** `var<private>` (not a varying — the
  fragment never reads it), because `setupPositionView` returns a vec4 for
  sprites and `setupModelViewProjection` multiplies it directly.
* The instance-matrix block is **dead code that Three still emits**: `positionLocal`
  is reassigned from `positionNode` two lines later and never read again. Emit
  it anyway (it is what fixes the attribute locations and forces the four
  instance-matrix vertex buffers to be bound), and note it is exactly the
  r186 `setupPosition` ordering bug the sdf-text scout flagged — here it cancels
  out because `positionNode` overwrites `positionLocal` wholesale.

The generated fragment stage, complete:

```wgsl
DiffuseColor = vec4<f32>( mix( object.nodeUniform8, object.nodeUniform9,
                 ( 1.0 - pow( ( 1.0 - nodeVarying4.x ), 2.0 ) ) ),
                 ( ( 0.1 / length( ( nodeVarying5 - vec2<f32>( 0.5 ) ) ) ) - 0.2 ) );
DiffuseColor.w = ( DiffuseColor.w * object.nodeUniform10 );      // materialOpacity = 1
nodeVar4 = max( vec4<f32>( DiffuseColor.xyz, DiffuseColor.w ), vec4<f32>( 0.0 ) );
output.color = nodeVar4;
```

**There is no `DiffuseColor.w = 1.0` line.** `NodeBuilder.isOpaque()`
(`src/nodes/core/NodeBuilder.js:527`) is `transparent === false && blending ===
NormalBlending && alphaToCoverage === false`; this material fails on both of the
first two, so `NodeMaterial.setupDiffuseColor` skips the opaque branch
(`src/materials/nodes/NodeMaterial.js:899-903`). The port currently emits that
line unconditionally (`src/materials/node_material.rs:80`) — see §4.3.

Also note `max( …, 0 )` clamps the alpha too: `0.1/|uv-0.5| - 0.2` is negative
near the quad corners (`0.1/0.7071 - 0.2 = -0.0586`), and the clamp is what stops
sprites from *subtracting* light.

### 2.3 `SpriteNodeMaterial` under `WebGPURenderer`

`src/materials/nodes/SpriteNodeMaterial.js`. It is a `NodeMaterial` subclass
whose only override is `setupPositionView( builder )` (line 109):

```js
const mvPosition = modelViewMatrix.mul( vec3( positionNode || 0 ) );          // 115
let scale = vec2( modelWorldMatrix[0].xyz.length(), modelWorldMatrix[1].xyz.length() );  // 117
if ( scaleNode !== null ) scale = scale.mul( vec2( scaleNode ) );             // 119-123
if ( camera.isPerspectiveCamera && sizeAttenuation === false )                // 125-129
    scale = scale.mul( mvPosition.z.negate() );
let alignedPosition = positionGeometry.xy;                                    // 131
if ( object.center && object.center.isVector2 === true )                      // 133-139
    alignedPosition = alignedPosition.sub( center.sub( 0.5 ) );
alignedPosition = alignedPosition.mul( scale );                               // 141
const rotation = float( rotationNode || materialRotation );                   // 143
const rotatedPosition = rotate( alignedPosition, rotation );                  // 145
return vec4( mvPosition.xy.add( rotatedPosition ), mvPosition.zw );           // 147
```

Facts for this example:

* **Billboarding is implicit.** There is no billboard matrix and no camera-right/up
  vector: the quad corners are added in **view space** (`mvPosition.xy + rotated
  corner`), which is what makes the sprite face the camera. The centre goes
  through `modelViewMatrix`, the corners do not.
* `sizeAttenuation` defaults to `true` (`_useSizeAttenuation = true`, line 44),
  so the `mvPosition.z.negate()` branch is **absent** from the dump. Sprites get
  smaller with distance simply because the projection divides by w.
* `rotationNode` is null, so `materialRotation` (a `MaterialNode` uniform,
  `MaterialNode.ROTATION`, `SpriteMaterial.rotation = 0`) is used. It is
  `nodeUniform14 = 0` in the object struct, and `RotateNode`'s vec2 path
  (`src/nodes/utils/RotateNode.js:112-121`) emits
  `mat2x2( cos, sin, -sin, cos )`. **Zero rotation, but the mat2 is still
  emitted** — the port needs `Type::Mat2` for this.
* `object.center` — `InstancedMesh` has no `center`, so that block is skipped.
  (`Sprite` has one; this example is not a `Sprite`.)
* `modelWorldMatrix[0].xyz.length()` / `[1]` is the object's world X/Y scale, `1`
  here, and it is emitted as two `length()` calls in the shader — a `Vec3::length`
  and matrix-column indexing the port needs.
* `SpriteNodeMaterial` sets `this.transparent = true` in its constructor (line 94)
  and inherits `SpriteMaterial`'s defaults via `setDefaultValues`. The example
  then passes `{ depthWrite: false, blending: AdditiveBlending }`.

`NodeMaterial.setup` wires this through `builder.context.setupPositionView`
(`src/materials/nodes/NodeMaterial.js:472`), so the hook the port's
`docs/nodes.md §6` already names (`setup_position_view`) is the right seam —
except that the port does not actually have it yet (§4.1).

### 2.4 Determinism — and the one real landmine

* `performance.now` / `Date.now` return 0. `NodeFrame.update`
  (`src/nodes/core/NodeFrame.js:310-316`) computes `deltaTime` from
  `performance.now()`, so **`time` is exactly 0** and
  `angle = branchAngle + 0`. The `time` uniform is still allocated and written
  (`renderStruct.nodeUniform5 = 0`); do not optimise it away, it changes the
  uniform layout.
* `Math.random` is the seeded `sin(seed++)*10000` PRNG, seed `PI/4`.
  `RangeNode.setup` (`src/nodes/geometry/RangeNode.js:159-168`) fills
  `stride(4) × count` floats with `lerp(min[i%4], max[i%4], Math.random())` — so
  **every range buffer draws 4 randoms per instance, 80000 each, even for a
  scalar range**, and the w component of a `vec3` range still consumes a draw
  (its min = max = 0, so the value is 0; see buffer 6's head in `dump.json`).
* **The `Inspector` consumes the first 5 draws.** `examples/jsm/inspector/ui/List.js:11`
  does `` `list-${Math.random().toString(36).slice(2,11)}` `` and `new Inspector()`
  builds five `List`s (Parameters, Viewer, Performance, Memory, Settings —
  stack traces in the probe run). Total draws for the frame: **320005 = 5 + 4 × 80000.**
  So the first range value is drawn at **sequence index 5, not 0**.
* **Consumption order is the setup traversal order, not the source order:**

  | order | draws | node | attribute | vertex buffer slot / `@location` |
  |---|---|---|---|---|
  | 1 | idx 5 … 80004 | `range( 0, 3 )` (branchAngle) | `nodeAttribute4` | slot 4 / loc 7 |
  | 2 | idx 80005 … 160004 | `range( 0, 1 )` (radiusRatio) | `nodeAttribute6` | slot 5 / loc 8 |
  | 3 | idx 160005 … 240004 | `range( vec3(-1), vec3(1) )` | `nodeAttribute7` | slot 6 / loc 9 |
  | 4 | idx 240005 … 320004 | `range( 0, 1 )` (scaleNode) | `nodeAttribute15` | slot 7 / loc 10 |

  That is `setupPosition`'s depth-first walk of `positionNode` (cos → angle →
  branchAngle → the branches range; then `time.mul(radiusRatio.oneMinus())`;
  then `randomOffset`), followed by `setupPositionView`'s `scaleNode`. The
  example's *source* order is the opposite (scaleNode is written first).
  Verified against the replayed PRNG: buffer heads invert exactly to sequence
  indices 5 / 80005 / 160005 / 240005.

  Checkable constants (f64 sequence from seed `PI/4`):

  ```
  r[0] = 0.067811865475050581   r[3] = 0.56506990257366851
  r[1] = 0.61263899475670769    r[4] = 0.63986756874874118
  r[2] = 0.10126532103868158    r[5] = 0.17597648546689015
  ```

  and the first four floats the GPU actually saw (`dump.json` → `bufferHeads`):

  ```
  branches   0.527929485  1.98981714   1.47226501    2.93593931    (= 3 × r[5..8])
  radiusRatio 0.781111181 0.94367677   0.36973241    0.372152746
  offset      0.231912121 0.736554921  0.0717008635  0            (= 2×r-1, w forced 0)
  scale       0.61644882  0.782270014  0.00294871163 0.231343001
  ```

* **The two `range( 0, 1 )` calls are two different buffers with two different
  random draws.** The port's `range()` cache is keyed on
  `format!("range:{min:?}:{max:?}:{count}")` (`src/renderer/mod.rs:747`), which
  would collapse them into one. That is a silent, total-image failure.
* Why nobody has hit the 5-draw offset before: rung 2 (`webgpu_instance_mesh`) has
  the same `Inspector` and the same offset, but its
  `mix( normalWorld, randomColors, oscSine( time.mul(.1) ) )` evaluates
  `oscSine(0) = sin(0.75·2π)·0.5+0.5 = 0` (`src/nodes/utils/Oscillators.js:11`),
  so **the random colours contribute nothing to rung 2's graded frame**. Rung 13
  is the first rung where `range()` values reach pixels at all. The port's
  `DeterministicRandom` (`src/testing.rs`) starts at index 0 and has never been
  checked against the page.

---

## 3. What Three compiles

### 3.1 `renderPipeline_SpriteNodeMaterial_17`

**Blend state, exactly** (`AdditiveBlending`, `premultipliedAlpha = false`,
`src/renderers/webgpu/utils/WebGPUPipelineUtils.js:552-655`):

```json
"blend": {
  "color": { "srcFactor": "src-alpha", "dstFactor": "one", "operation": "add" },
  "alpha": { "srcFactor": "one",       "dstFactor": "one", "operation": "add" }
}
```

`writeMask 15`. Target `rgba16float` (the internal frame-buffer target).
`multisample.count 4`. Depth `depth24plus`, **`depthWriteEnabled false`**,
`depthCompare "less-equal"`. Primitive `triangle-list` / `ccw` / cull `back`.

A pipeline gets a blend state at all only when
`material.blending !== NoBlending && ( material.blending !== NormalBlending ||
material.transparent !== false )` (`WebGPUPipelineUtils.js:103`) — which is why
rungs 1–4 saw `blend: undefined`.

**Vertex buffers — eight of them, four `stepMode: "instance"`:**

| slot | stride | attributes | step |
|---|---|---|---|
| 0 | 12 | loc 0 `position` float32x3 | vertex |
| 1 | 12 | loc 1 `normal` float32x3 | vertex |
| 2 | 8 | loc 2 `uv` float32x2 | vertex |
| 3 | **64** | loc 3 @0, loc 4 @16, loc 5 @32, loc 6 @48, all float32x4 | **instance** |
| 4 | 16 | loc 7 float32x4 | **instance** |
| 5 | 16 | loc 8 float32x4 | **instance** |
| 6 | 16 | loc 9 float32x4 | **instance** |
| 7 | 16 | loc 10 float32x4 | **instance** |

Slot 3 is the **interleaved** instance matrix: `createInstanceMatrixNode`
(`src/nodes/accessors/Instance.js:40-68`) wraps `instanceMatrix.array` in an
`InstancedInterleavedBuffer( array, 16, 1 )` and takes four
`instancedBufferAttribute( interleaved, 'vec4', 16, offset )` views, because
`20000 × 16 × 4 = 1 280 000 > maxUniformBufferBindingSize (65536)`. Same test in
`RangeNode.setup` (`:173`): `20000 × 4 × 4 = 320 000 > 65536`, so every `range()`
is an `InstancedBufferAttribute` registered on the geometry as
`'__range' + node.id` — **not** a uniform buffer.

Bind groups — **one layout object, reused for both groups**
(`pipelineLayouts[14].bindGroupLayouts = [ bgl 10, bgl 10 ]`):

```
group 0 "render"  binding 0  visibility 7 (VERTEX|FRAGMENT|COMPUTE)  uniform buffer, 144 B
group 1 "object"  binding 0  visibility 7                            uniform buffer, 112 B
```

`renderStruct` (`UpdateType::Render`), 144 bytes:

| member | offset | source |
|---|---|---|
| `nodeUniform5 : f32` | 0 | `time` (= 0) |
| `cameraProjectionMatrix : mat4x4` | 16 | camera |
| `cameraViewMatrix : mat4x4` | 80 | camera |

`time` is allocated **first**, before the camera matrices, because the vertex
traversal reaches it first. The projection is the WebGPU-depth form:
`m22 = far/(near-far) = -1.001001`, `m32 = far·near/(near-far) = -0.1001`,
`m00 = 1.3403167`, `m11 = 2.1445069`.

`objectStruct` (`UpdateType::Object`), 112 bytes — note the **scalar packed into
the vec3's padding**:

| member | offset | source | value |
|---|---|---|---|
| `nodeUniform8 : vec3` | 0 | `uniform( color('#ffa575') )` | `(1, 0.376262, 0.177888)` linear |
| `nodeUniform9 : vec3` | 16 | `uniform( color('#311599') )` | `(0.030713, 0.007499, 0.318547)` linear |
| `nodeUniform10 : f32` | **28** | `materialOpacity` | 1 |
| `nodeUniform13 : mat4x4` | 32 | `modelWorldMatrix` | identity |
| `nodeUniform14 : f32` | 96 | `materialRotation` | 0 |
| `nodeUniform16 : f32` | 100 | `uniform( 0.08 )` (size) | 0.08 |

Member order is fragment-stage-first allocation order (the two colours and the
opacity, then the vertex stage's world matrix, rotation, size).
`queue.writeBuffer` is called three times on this buffer (offsets 0, 16, 100) —
one per uniform group update; irrelevant to the port, which can write it whole.

Varyings: `@location(0) nodeVarying4 : vec4<f32>` (the *whole* radiusRatio vec4
attribute), `@location(1) nodeVarying5 : vec2<f32>` (uv). The fragment stage
declares **no group 0 at all** — it only uses `object`.

### 3.2 `renderPipeline_outputColorTransform_18`

Identical in shape to rungs 2/4/6: full-screen triangle (36-byte position
buffer, `(-1,3,0) (-1,-1,0) (3,-1,0)`), `rgba8unorm` target, **no blend**,
`multisample.count 1`, `depth24plus` attachment with `depthWriteEnabled true`,
sampler + texture in group 1 bindings 0/1 and a 64-byte object struct at binding 2,
`renderStruct` = projection + view + `nodeUniform1 : vec2` viewport `(800, 500)`.
`unpremultiply → sRGB OETF → premultiply`. Diff the two `.wgsl` files against the
port's output-pass dump; no change expected.

### 3.3 Textures, buffers, passes

Textures: `rgba16float` 800×500 sample 1 (frame-buffer target), `-msaa`
`rgba16float` sample 4, `depth24plus` sample 4, `depthBuffer` `depth24plus`
sample 1. One sampler (linear/linear/nearest, clamp) for the output pass.

Buffers (`dump.json`): 48 B position, 48 B normal, 32 B uv, 24 B uint32 index
(6 indices), 1 280 000 B instance matrix, **four × 320 000 B range buffers**,
144 B + 112 B scene uniforms, 36 B quad position, 144 B + 64 B output-pass
uniforms.

Pass 1: clear to the background colour, depth clear 1, one
`setPipeline / setBindGroup 0 / setBindGroup 1 / setIndexBuffer(uint32) /
setVertexBuffer × 8 / drawIndexed( 6, 20000 )`.
Pass 2: `loadOp: "load"`, the output quad.

---

## 4. Gap list against the port

Against `port` at `cdf834a` (rung 4 + the merged side branches + treewalk).
Items marked **[shared]** are the same renderer work the sdf-text plan's §4 and
rung 9 need.

### 4.1 `SpriteNodeMaterial`

| missing | where |
|---|---|
| any material other than `MeshBasicNodeMaterial` | `src/materials/mod.rs:27-51` is one struct; `src/materials/node_material.rs:33` is one free `setup()` function. There is no subclass seam at all — `docs/nodes.md §3` promises "`setupX()` … as methods on the material port so a subclass overrides one of them", but that is not what the code does. |
| `position_node` on the material | `src/materials/mod.rs` has `color_node`, `vertex_node`, `fragment_node` only. |
| `setup_position_view` hook | `position_view()` is a fixed `thread_local` accessor (`src/nodes/tsl.rs:~640`) equal to `modelViewMatrix * vec4(positionLocal,1)`.xyz. `docs/nodes.md §6` claims `NodeMaterial` "already routes through `builder.context`"; it does not. This is the real seam to build. |
| the sprite billboard path itself | new: `mvPosition`, world-scale from `modelWorldMatrix` columns, `scaleNode`, `positionGeometry.xy`, `rotate()`, the vec4 return. |
| `materialRotation` uniform + `Material.rotation` | new `UniformSource::MaterialRotation`. |
| `sizeAttenuation` flag | needed only to prove the `false` branch is *not* emitted. |
| `Material.blending` / `transparent` defaults for sprites | `transparent` exists on the material struct; `blending` does not exist at all. |

### 4.2 Blend state in `programs.rs` **[shared with sdf-text §4 and rung 9]**

`src/renderer/programs.rs:131-136` hardcodes `blend: None` ("Opaque material:
three.js emits no blend state"). Needs:

* a `Blending` enum (`No`, `Normal`, `Additive`, `Subtractive`, `Multiply`,
  `Custom`) + `premultiplied_alpha` on the material;
* `RenderState` (`programs.rs:13-21`) gains the blend state so the pipeline cache
  key separates an additive pipeline from an opaque one;
* the `_getBlending` mapping of §3.1 — for this rung only the
  non-premultiplied `AdditiveBlending` row is needed:
  colour `(SrcAlpha, One, Add)`, alpha `(One, One, Add)`;
* the gate `blending !== NoBlending && ( blending !== NormalBlending ||
  transparent !== false )`, so rungs 1–4 keep emitting `blend: None`.

### 4.3 Transparent list, `depthWrite: false`, per-fragment alpha **[shared]**

* The transparent list **is already drawn**: `RenderList::items()`
  (`src/renderer/render_list.rs:106-110`) chains `opaque.iter()` then
  `transparent.iter()`, and `Renderer::render` iterates it
  (`src/renderer/mod.rs:292`). Sorting is right too
  (`reverse_painter_sort_stable`). **What is missing is only the blend state** —
  today a transparent object draws opaquely.
* `depth_write` / `depth_test` already flow into the pipeline
  (`programs.rs:152-164`, set at `mod.rs:459`), so `depthWrite: false` is free.
* `src/materials/node_material.rs:80` unconditionally emits
  `DiffuseColor.w = 1.0`. It must become `if is_opaque()` — `transparent ==
  false && blending == Normal && alpha_to_coverage == false`. **This single line
  is the difference between a galaxy and a wall of solid quads.**
  `docs/nodes.md §7` lists the unconditional emission as a known divergence
  "pixel-neutral here"; rung 13 is where it stops being neutral.

### 4.4 TSL nodes missing from `src/nodes/tsl.rs`

By Three's TSL name (port head; `[rung5]` = already built on the unmerged rung5
branch):

| TSL | port state |
|---|---|
| `length` | missing on `port` **[rung5]** — `Node::Math` call, `Type::F32` result |
| `pow3` | missing (trivial: `(a*a)*a`, and it must emit that form, not `pow`) |
| `TWO_PI` | missing (and it must stay an unfolded `( 6.283185307179586 / 3.0 )` division) |
| `vec2( x )` splat / `vec2( a, b )` from nodes | `vec2()` takes two `f64`s only; there is no node-valued `vec2`/`vec3` join helper for vec2 (`vec3_join`/`vec4_join` exist, no `vec2_join`) |
| `mat2` construction | **`Type::Mat2` does not exist** (`src/nodes/node.rs:18-31`) — needed by `RotateNode` |
| `rotate( vec2, float )` | missing entirely (`src/nodes/utils/RotateNode.js`) |
| `range( float, float )`, `range( vec3, vec3 )` | `range()` (`src/nodes/tsl.rs:866`) takes `Color` min/max only, always `Type::Vec4`, always a **uniform** buffer, and has no `.convert(nodeType)` → `.x` / `.xyz` narrowing |
| `materialRotation` | missing |
| `uniform( color )` as a *value* uniform | `uniform_value()` (`tsl.rs:92`) + `UniformSource::Value` exist — usable as is |
| `time` | exists (`tsl.rs`, `UniformSource::Time`) |
| `uv`, `positionGeometry`, `modelWorldMatrix`, `modelViewMatrix` | exist |
| `mix`, `cos`, `sin`, `floor`, `pow`, `one_minus`, `max`, `to_var`, `div`, `sub`, `add`, `mul` | exist |
| matrix **column** indexing `m[0].xyz` | `element(0)` exists (`tsl.rs:386`) and already prints `m[ 0u ]` where Three prints `m[ 0 ]` (`docs/nodes.md §7`) |

Not needed, despite the rung's reputation: no `hash`, no `smoothstep`, no
`If`/`Loop`, no `Fn()`, no texture node, no compute.

### 4.5 `range()` / instanced vertex attributes — the biggest piece **[shared with sdf-text §4]**

Today: `range()` and the instance matrix are **uniform buffers**
(`src/nodes/tsl.rs:843-868`, `src/renderer/mod.rs:736-772`), indexed by
`@builtin(instance_index)`, capped at 64 KiB ≈ 1024 instances. This rung needs
20000. Concretely:

1. `Node::Attribute` needs an instanced variant carrying `step_mode`, a
   `buffer id`, an `array_stride` and a byte `offset`, so that four `vec4`s can
   come out of one 64-byte interleaved buffer.
2. `programs.rs:104-116` builds one `VertexBufferLayout` per attribute with
   `array_stride = components * 4` and `step_mode: Vertex`. It needs to group
   attributes by source buffer and carry per-attribute `offset` + per-buffer
   `step_mode`.
3. The renderer's geometry cache (`ensure_geometry`, `src/renderer/mod.rs`)
   must hold the per-instance buffers as well as the geometry's own, and
   `pass.set_vertex_buffer` must bind eight slots (`mod.rs:551-556` binds one per
   named attribute).
4. `RangeNode`'s own decision — uniform below `maxUniformBufferBindingSize`,
   instanced attribute above — should be ported literally, including the
   `stride = 4` fill and the `.convert(nodeType)` narrowing, so rung 2 keeps its
   uniform path and rung 13 gets the attribute path from the same code.
5. The instance-matrix attribute path (`InstancedInterleavedBuffer`, 4 × vec4 at
   offsets 0/16/32/48, stride 64) is the same mechanism and lifts rung 2's 1024
   cap for free.
6. A varying carrying a whole instanced `vec4` attribute
   (`varyings.nodeVarying4 = nodeAttribute6`) replaces the port's
   `to_varying(None, instance_index())` trick
   (`src/materials/node_material.rs:191-194`).

Also: the `range()` buffer cache key
(`format!("range:{min:?}:{max:?}:{count}")`, `mod.rs:747`) must become
**per-node identity**, or the example's two `range(0,1)` calls share one buffer
and the image is wrong everywhere.

And `DeterministicRandom` (`src/testing.rs`) must start **5 draws in** (§2.4),
with the reason recorded in a comment: five `Inspector` `List` ids, not a fudge.
Best shape: an explicit `random.skip(5)` in the example's setup with a comment
naming `examples/jsm/inspector/ui/List.js:11`, so the constant lives next to the
example it belongs to, not inside the renderer.

### 4.6 Premultiplied alpha in the output

The output pass does `clamp(a, 0, 1)` → `unpremultiplyAlpha` → sRGB OETF →
`premultiplyAlpha` (`fragment_outputColorTransform-r186.wgsl`), which the port
already emits (`src/materials/node_material.rs:205-213`).

For **this** example it is a no-op, and it is worth knowing why before chasing
it: the scene pass clears alpha to **1**, the sprite alpha is clamped `≥ 0` by
`max(…, 0)`, and the additive alpha equation is `One + One`, so the target alpha
is `≥ 1` at every pixel and the clamp pins it to exactly 1. Divide-by-alpha and
multiply-by-alpha both cancel. **But** if the port gets the clear alpha wrong
(0 instead of 1) or emits `DiffuseColor.w = 1.0`, the unpremultiply turns into a
huge per-pixel amplification and the failure looks like a colour-space bug rather
than a blending bug. `material.premultipliedAlpha` is `false` here and must stay
false — it selects a different blend row (§3.1).

### 4.7 Nothing needed

No textures, no loaders, no lights, no shadows, no fog, no tone mapping
(`NoToneMapping`), no render target beyond the frame-buffer target the port
already has, no `Fn()`, no compute, no morph/skin/batch, no addons (drop
`Inspector` and `OrbitControls` — but keep the Inspector's five PRNG draws).
`PlaneGeometry(1,1)` and `InstancedMesh` both exist.

---

## 5. Traps

* **The 5-draw PRNG offset.** Off by one draw and every one of the 20000 sprites
  moves. There is no partial credit: ~11% of the reference's pixels are above
  background (11240 of 100000), against a 100-pixel budget. Verify the offset
  with a unit test on the first four floats of each range buffer (§2.4), not
  with the image.
* **Two `range(0,1)` are two buffers.** Cache by node identity, never by value.
* **Consumption order is `branchAngle → radiusRatio → randomOffset → scaleNode`**,
  which is *not* the order the JS writes them. Get it from `setupPosition`'s
  traversal, then `setupPositionView`'s.
* **`DiffuseColor.w = 1.0` must not be emitted.** It is the whole image.
* **`max( …, 0 )` clamps alpha as well as colour.** Without it the quad corners
  have negative alpha and *subtract* through the additive blend.
* **`depthWrite: false`, `depthTest` still on, depth cleared to 1.** Nothing
  writes depth, so every sprite passes; if the port leaves `depthWrite` true the
  first sprites occlude the rest and the galaxy thins out plausibly rather than
  obviously.
* **Alpha blending must be on the *scene* pass (`rgba16float`, MSAA 4), not the
  output pass.** The output pass has no blend state.
* **Four instanced vertex buffers plus one interleaved instance-matrix buffer.**
  wgpu will reject a stride/step mismatch loudly, but a *correct-looking* layout
  with `step_mode: Vertex` will silently read the first 6 values for all 20000
  instances and draw six sprites' worth of garbage.
* **The dead instance-matrix code.** Three emits it; it costs the `normal`
  attribute and `tsl_inverse_mat3`. Omitting it is pixel-neutral but changes the
  attribute locations and the vertex-buffer count — decide once, and say so in
  `docs/nodes.md §7` either way.
* **`time` is 0 but the uniform is still there**, first in `renderStruct`.
  Dropping it shifts every offset in that struct.
* **`TWO_PI.div(3)` is not folded**, `.pow3()` is not `pow(x,3)`. If the port
  constant-folds, the arithmetic differs in the last f32 bits — probably
  invisible, but it makes a WGSL diff against the dump noisy, which is the
  cheapest gate this rung has.
* **The vec3 range's w component still consumes a random draw** and is written
  as 0. Fill 4 components per instance always.
* **`v_positionView` is a `vec4` here**, and is a `var<private>`, not a varying.
* **Frustum culling.** `InstancedMesh.boundingSphere` is null →
  `computeBoundingSphere()` unions identity-transformed `PlaneGeometry`
  spheres → centre origin, radius ≈ 0.7071, well inside the frustum. The
  *particles* live out to radius 5 and are far outside that sphere; that is fine
  because Three culls on the sphere, not the sprites. A port that culls on
  something cleverer will draw nothing.
* **One TCP port, one GPU.** `puppeteer.js` hardcodes 1234; do not run two e2e
  suites at once (this bit the first run of §1).
* **Silent failure is the norm on this stack.** Trust the diff image.

---

## 6. Step ladder

Six steps. Steps 2 and 3 are the **shared renderer work**: the blend state, the
material `blending`/`transparent` plumbing, the `isOpaque()` gate on
`DiffuseColor.w`, and the instanced-vertex-attribute path. Rung 9
(`webgpu_postprocessing_masking`) and the sdf-text ladder's step 4 both need the
first of those; sdf-text's `BatchedText` needs the second, and it also lifts
rung 2's 1024-instance cap. Do them on a branch that can merge to `port`
independently of the galaxy example.

1. **Blend state + the opaque gate.** Add `Blending` and `premultiplied_alpha`
   to the material, the `_getBlending` table, `blend` in `RenderState`/the
   pipeline key, and make `DiffuseColor.w = 1.0` conditional on `is_opaque()`.
   **Gate:** rungs 1–4 still score `0 / 45 / 0 / 1`, and a unit test asserts that
   a material with `AdditiveBlending` produces exactly
   `color (SrcAlpha, One, Add)` / `alpha (One, One, Add)` while a default
   material still produces `blend: None`. Pixel-neutral by construction.

2. **Instanced vertex attributes.** `Node::Attribute` gains step mode + buffer
   identity + stride + offset; `programs.rs` groups attributes into buffers;
   the renderer binds them. Port `RangeNode`'s uniform-vs-attribute branch and
   `createInstanceMatrixNode`'s interleaved branch literally, both keyed on
   `maxUniformBufferBindingSize`. **Gate:** rung 2 is unchanged at 45 px
   (it stays on the uniform path, `1000 × 64 = 64 000 ≤ 65536` — check that
   number before assuming), *and* a scratch test with 2000 instances draws
   through the attribute path without a validation error. Still no new example.

3. **`SpriteNodeMaterial`, geometry only.** Introduce the material seam
   (`setup_position_view`), `position_node`, `Type::Mat2`, `rotate()`, `length()`,
   `materialRotation`, and the four `range()` variants — with a *constant* colour
   and no blending yet, drawn opaque. **Gate:** generated WGSL diffed line by
   line against `vertex-r186.wgsl`; expected divergences are only the ones
   `docs/nodes.md §7` already lists (`nodeVarN` numbering, `m[0u]`, the
   `VERTEX_` sub-build temps). The vertex program is where all the risk is, and
   this gate does not need a single correct pixel.

4. **The PRNG offset and the four range buffers.** `random.skip(5)`, per-node
   buffer identity, the documented consumption order. **Gate:** a unit test
   asserting the first four floats of each of the four buffers against the
   numbers in §2.4, to f32. No image involved. If this test is red, nothing
   downstream can be judged.

5. **The full material, blended.** `colorNode`, `scaleNode`, `depthWrite: false`,
   `AdditiveBlending`, `transparent`. Port `examples/webgpu_tsl_galaxy.rs` and
   add the e2e entry. **Gate:** the diff image. Expect to land near 0 or near
   20% — there is very little in between. If it is near 20%, dump the port's
   range buffers and compare against `dump.json`'s `bufferHeads` before touching
   the shader.

6. **Diff, then look.** If the shape is right but the brightness is off, the
   suspects in order are: `DiffuseColor.w = 1.0` still being emitted, the
   `max(…, 0)` clamp missing on alpha, the clear alpha not being 1 (§4.6), and
   MSAA `count 4` on the scene pass. If the shape is right but the sprites are
   the wrong *size*, it is `scaleNode` vs `sizeAttenuation` — the dump has no
   `mvPosition.z.negate()`. Director looks at actual/expected/diff.
