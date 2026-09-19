# Scout — `webgpu_lines_fat` (fat lines: `Line2` / `LineGeometry` / `Line2NodeMaterial`)

Scouted 2026-09-19. Example: `~/src/vendor/three.js/examples/webgpu_lines_fat.html`
(vendor r186, `rung0/grader-flags.patch` applied). Everything here is read off the
HTML, off three.js' own source, or off the GPU dump. Nothing is inferred from the
reference image.

Files next to this plan:

| file | bytes | what |
|---|---|---|
| `e2e-webgpu_lines_fat.log` | 773 | Three's own grader on this machine, both runs, plus the siblings |
| `dump/dump.json` | 44 246 | every descriptor, all six passes, the chronological `order` log |
| `dump/m00_vertex_vertex.wgsl` | 4 731 | `Line2NodeMaterial`'s vertex stage — the whole rung |
| `dump/m01_fragment_fragment.wgsl` | 1 798 | `Line2NodeMaterial`'s fragment stage |
| `dump/m02..m03_*_outputColorTransform.wgsl` | 1 299 / 1 641 | the output pass (unchanged from rungs 6/13) |
| `dump/m04..m05_*_Background.material.wgsl` | 1 439 / 1 108 | the inset's `scene.backgroundNode` skybox |
| `dump/actual_full.png` | 35 015 | Three's own 800×500 deterministic frame |
| `dump/actual.jpg` | 37 050 | the same frame through `test/e2e/image.js` — what the grader compares |
| `webgpu_lines_fat.jpg` | 37 044 | the grader's reference (copy of `examples/screenshots/`) |
| `make-oracle.mjs` | 3 533 | regenerates the oracle below from three's `src/` (no bundle, no DOM) |
| `spline_oracle.json` | 54 273 | **the numeric gate** — the 64 Hilbert corners, the 768 spline points and the 768 HSL colours, as `f32` (§5) |

Dump result: **6 shader modules, 3 render pipelines, 0 compute pipelines,
2 bind-group layouts, 6 render passes, 6 submits, 3 draws of scene geometry.**

---

## 1. Grade confirmation — 0.0%, twice, not on the exception list

```
$ cd ~/src/vendor/three.js && flock /run/user/1000/three-rs-gpu.lock \
      npm run test-e2e-webgpu -- webgpu_lines_fat
Diff 0.0% in file: webgpu_lines_fat (3.0s)
TEST PASSED! 1 screenshots rendered correctly.
```

Second run, batched with both siblings:

```
Diff 0.0% in file: webgpu_lines_fat (3.0s)
Diff 0.1% in file: webgpu_lines_fat_raycasting (3.0s)
Diff 0.1% in file: webgpu_lines_fat_wireframe (3.0s)
TEST PASSED! 3 screenshots rendered correctly.
```

`webgpu_lines_fat` is **not** on `test/e2e/puppeteer.js`'s `exceptionList` (which
at r186 contains no `lines` entry of any kind). 0.0% on both runs. **This is a
gradeable rung.** The two siblings sit at 0.1%, i.e. exactly on
`maxDifferentPixels` — they pass, but with no margin; see §7.

Budget: the grader compares at 400×250 = 100 000 pixels with
`maxDifferentPixels = 0.1%` → **100 pixels**.

---

## 2. What the example does

### 2.1 Page, renderer, cameras

* `WebGPURenderer( { antialias: true } )` → `samples = 4`. `setPixelRatio(
  window.devicePixelRatio )`; the grader sets no `deviceScaleFactor`, so
  **DPR = 1**. `setClearColor( 0x000000 )` (alpha defaults to 1 here — see the
  pass-0 `clearValue` below). `setSize( innerWidth, innerHeight )`.
* `puppeteer.js` sets the page viewport to `{ width: 400*2, height: 250*2 }`, so
  **`innerWidth = 800`, `innerHeight = 500`**, canvas 800×500, and the screenshot
  is scaled by `1/2` before comparison. The dump's textures are 800×500 — confirmed.
* `camera = PerspectiveCamera( 40, 800/500, 1, 1000 )` at `( -40, 0, 60 )`.
* `camera2 = PerspectiveCamera( 40, 1, 1, 1000 )`, position and quaternion copied
  from `camera` every frame.
* `OrbitControls( camera, dom )`, `enableDamping = true`, `minDistance 10`,
  `maxDistance 500`. The constructor's `update()` aims the camera at the default
  target `(0,0,0)`; `animate()`'s `update()` with no input moves nothing.
  **Port it as `camera.lookAt( 0, 0, 0 )`; do not port OrbitControls** (same call
  the rung-13 plan makes).
* `Stats` and `lil-gui` are UI only. Unlike rungs 2/13 there is **no `Inspector`**,
  so there is no five-draw PRNG offset here.

### 2.2 The geometry, exactly

```js
const points   = GeometryUtils.hilbert3D( new Vector3(0,0,0), 20.0, 1, 0,1,2,3,4,5,6,7 );
const spline   = new CatmullRomCurve3( points );
const divisions = Math.round( 12 * points.length );
for ( i < divisions ) { spline.getPoint( i/divisions, point ); positions.push(…);
                        lineColor.setHSL( i/divisions, 1.0, 0.5, SRGBColorSpace ); colors.push(…); }
```

`hilbert3D` with `iterations = 1` recurses once into eight sub-cubes of eight
corners: **64 points**, so `divisions = 768`. Verified two ways — the oracle
script reproduces 64/768, and the dump's instanced vertex buffers are 18 408 B
= 767 × 24, with `drawIndexed( 18, 767 )`. `LineGeometry.setPositions` converts a
768-point polyline to 767 start/end **pairs** (`length = array.length - 3`, output
`2 * length` floats = 4602 = 767 × 6).

`CatmullRomCurve3` defaults: `closed = false`, `curveType = 'centripetal'`,
`tension = 0.5`. `Curve.getPoint( t )` maps `t` over `points.length - 1` spans.
**The port has no curve classes at all** (§4.6).

### 2.3 The objects and materials

```js
const geometry = new LineGeometry();        // extends LineSegmentsGeometry (InstancedBufferGeometry)
geometry.setPositions( positions );          // instanceStart/instanceEnd, one interleaved buffer, stride 6
geometry.setColors( colors );                // instanceColorStart/End, one interleaved buffer, stride 6

matLine = new Line2NodeMaterial( { color: 0xffffff, linewidth: 5, vertexColors: true,
                                   dashed: false, alphaToCoverage: false } );
line = new Line2( geometry, matLine );       // extends LineSegments2 extends Mesh
line.computeLineDistances();                 // instanceDistanceStart/End — UNUSED, see below
line.scale.set( 1, 1, 1 );
scene.add( line );

// the second object is invisible in the graded frame:
line1 = new Line( geo, new LineBasicNodeMaterial( { vertexColors: true } ) );
line1.visible = false;                       // toggled only from the GUI
```

`LineSegmentsGeometry`'s fixed base geometry is 8 vertices / 18 indices:

```
position = [ -1,2,0,  1,2,0,  -1,1,0,  1,1,0,  -1,0,0,  1,0,0,  -1,-1,0,  1,-1,0 ]
uv       = [ -1,2, 1,2, -1,1, 1,1, -1,-1, 1,-1, -1,-2, 1,-2 ]
index    = [ 0,2,1, 2,3,1, 2,4,3, 4,5,3, 4,6,5, 6,7,5 ]
```

Dump: buffer 6 = 96 B (8×3×4), buffer 8 = 64 B (8×2×4), buffer 10 = 72 B
(18 × uint32). Exact.

**`computeLineDistances()` is dead weight for the graded frame.** `dashed` is
false, so `mvpLine` never reads `instanceDistanceStart/End`, and the dump's
pipeline has only **four** vertex buffers — the distance buffer is never created.
A port may call it or skip it; it changes nothing. (It is still worth porting for
the dashed path the workspace consumer wants.)

### 2.4 The compiled material variant

`Line2NodeMaterial` (three's **`src/materials/nodes/Line2NodeMaterial.js`**, 634
lines — note: *core*, not `examples/jsm/`) branches at setup time on three flags:

| flag | value here | effect |
|---|---|---|
| `_useDash` | `false` (from `dashed: false`) | drops `instanceDistance*`, `lineDistance`, `dashSize`/`gapSize`, the `mod`-discard, and **keeps** the world-units cap extension |
| `_useWorldUnits` | `false` (default) | takes the **screen-space** branch of `mvpLine` and the **round-endcap** branch of `alphaLine` |
| `_useAlphaToCoverage` | `false` (explicit; the class default is `true`) | takes the `discard` branch, not the `fwidth`/`smoothstep` branch |

So of the 634 lines, the graded variant needs roughly **220 lines of TSL**:
`mvpLine`'s screen-space path, `trimSegmentAlpha`, the non-A2C `alphaLine`, and
`setupDiffuseColor`'s `instanceColor` block. `closestLineToLine`,
`viewportOpaqueMipTexture`, the dash machinery and the world-units block are all
absent from the dump. The rung can land the full material, but only the
screen-space / no-dash / no-A2C variant is on the pixel hook.

`Line2NodeMaterial` also sets **`this.blending = NoBlending`** in its constructor
and `setDefaultValues( new LineDashedMaterial() )` (so `transparent = false`,
`opacity = 1`, `side = FrontSide`, depth test/write on).

### 2.5 `animate()` — two renders, a scissor and a depth clear

```js
renderer.setClearColor( 0x000000 );
renderer.setViewport( 0, 0, 800, 500 );
controls.update();
renderer.autoClear = true;
scene.backgroundNode = null;
renderer.render( scene, camera );              // pass 0 + output pass 1

const posY = 500 - 125 - 20;                   // = 355
renderer.clearDepth();                         // pass 2 (+ a redundant output pass 3)
renderer.setScissorTest( true );
renderer.setScissor ( 20, 355, 125, 125 );
renderer.setViewport( 20, 355, 125, 125 );
camera2.position.copy( camera.position ); camera2.quaternion.copy( camera.quaternion );
renderer.autoClear = false;
scene.backgroundNode = color( 0x222222 );
renderer.render( scene, camera2 );              // pass 4 (background + line) + output pass 5
renderer.setScissorTest( false );
```

`insetWidth = insetHeight = innerHeight/4 = 125`. Three's viewport origin is
bottom-left, so `(20, 355, 125, 125)` is a 125×125 box **20 px from the left and
20 px from the top** — in wgpu/WebGPU's top-left coordinates, `y = 500 - 355 -
125 = 20`.

**The inset is not optional.** 125×125 of 800×500 is 62.5×62.5 ≈ **3900 of the
grader's 100 000 pixels — 39× the 100-pixel budget.** A port that draws only the
main view fails by a factor of forty.

### 2.6 Determinism

* No `window.TESTING` branch anywhere in the page.
* **No `Math.random` reaches pixels.** `Line2`'s and `LineSegments2`'s *default*
  material argument calls `Math.random()`, but the example supplies both
  materials, so the default is never evaluated. There is no `Inspector` and no
  `range()` node. `Renderer::skip_random_draws` is irrelevant to this rung —
  a first for the ladder since rung 2.
* `performance.now`/`Date.now` are frozen at 0. The material has no `time` node
  and the dump's render struct has no time member, so nothing depends on it.

---

## 3. Dump reading

### 3.1 The three pipelines

| id | label | topology / face | depth | MSAA | target | blend |
|---|---|---|---|---|---|---|
| 19 | `renderPipeline_Line2NodeMaterial_17` | `triangle-list`, ccw, cull **back** | `depth24plus`, write **true**, `less-equal` | **4**, `alphaToCoverageEnabled: false` | `rgba16float`, mask 15 | **none** |
| 47 | `renderPipeline_Background.material_21` | `triangle-list`, **cw**, cull back | write **false**, `always` | 4 | `rgba16float` | none |
| 36 | `renderPipeline_outputColorTransform_20` | `triangle-list`, ccw, cull back | write true, `less-equal` | 1 | `rgba8unorm` | none |

`blend` is absent on the line pipeline even though the material is blended-looking:
`WebGPUPipelineUtils` emits a blend state only when `blending !== NoBlending && (
blending !== NormalBlending || transparent !== false )`, and `Line2NodeMaterial`
sets `blending = NoBlending`. **The port's existing `blend: None` path is already
right here** — `Blending::No` must map to "no blend state", not to a
`(One, Zero)` blend.

`alphaToCoverageEnabled` is `false`, which is what the port hardcodes today
(`src/renderer/programs.rs:212`). No change needed for *this* example.

### 3.2 Vertex buffers — four, two of them interleaved instanced

| slot | stride | step | attributes |
|---|---|---|---|
| 0 | 12 | vertex | loc 0 `position` `float32x3` @0 |
| 1 | **24** | **instance** | loc 1 `instanceStart` `float32x3` @0, loc 2 `instanceEnd` `float32x3` @**12** |
| 2 | 8 | vertex | loc 3 `uv` `float32x2` @0 |
| 3 | **24** | **instance** | loc 4 `instanceColorStart` @0, loc 5 `instanceColorEnd` @**12** |

Buffers: 6 = 96 B position, 7 = 18 408 B instanceStart/End, 8 = 64 B uv,
9 = 18 408 B instanceColor, 10 = 72 B uint32 index.
`drawIndexed( indexCount 18, instanceCount 767 )`.

This is exactly the shape `InstancedInterleavedBuffer( array, 6, 1 )` + two
`InterleavedBufferAttribute( buf, 3, 0 | 3 )` produce — and exactly what the
port's `AttributeSource::Instance { buffer, offset }` grouping
(`src/nodes/builder.rs:214-232`) already emits, with `array_stride =
item_size * 4 = 24` and offsets `0` / `12`. **Rung 13's machinery covers this
verbatim; nothing new is needed in `programs.rs`.**

### 3.3 Uniform layouts — record them, don't derive them

One bind-group layout object (id 12: binding 0, visibility `VERTEX|FRAGMENT|
COMPUTE`, uniform buffer) is reused for **both** groups of both scene pipelines.

`renderStruct`, **288 B** (buffer 11):

| member | offset | source |
|---|---|---|
| `nodeUniform8 : f32` | 0 | `screenDPR` (= 1) |
| `cameraWorldMatrix : mat4x4` | 16 | camera |
| `cameraProjectionMatrixInverse : mat4x4` | 80 | camera |
| `cameraViewMatrix : mat4x4` | 144 | camera |
| `cameraProjectionMatrix : mat4x4` | 208 | camera |
| `nodeUniform6 : vec4` | 272 | `viewport` |

`objectStruct`, **160 B** (buffer 14):

| member | offset | source |
|---|---|---|
| `nodeUniform1 : mat4x4` | 0 | `modelWorldMatrixInverse` |
| `nodeUniform5 : mat4x4` | 64 | `modelWorldMatrix` (inside `modelViewMatrix`) |
| `nodeUniform7 : f32` | 128 | `materialLineWidth` (= 5) |
| `nodeUniform9 : vec3` | 144 | `materialColor` (white) |
| `nodeUniform10 : f32` | **156** | `materialOpacity` (= 1) — packed into the vec3's padding |

Background: render 144 B (`nodeUniform0 : f32` backgroundIntensity @0, view @16,
projection @80), object 80 B (`nodeUniform1 : f32` opacity @0, `nodeUniform4 :
mat4x4` @16). Output pass: render 144 B, object 64 B — byte-identical to rungs
6/9/13.

**`screenDPR` is allocated *first* in the render group even though it has the
highest node id.** That ordering does not follow WGSL generation order, and the
port's group allocator (traversal order) will not reproduce it. It is
pixel-neutral — the port writes its own buffer to its own layout — but it makes
a struct-for-struct diff against the dump noisy. Add it to `docs/nodes.md` §8 as
a new divergence class ("uniform-struct member order") rather than chasing it.

### 3.4 The pass sequence

| pass | encoder | attachments | draws |
|---|---|---|---|
| 0 | `renderContext_0` | rgba16float MSAA4 + resolve, `clear` to `(0,0,0,1)`; depth `clear` 1 | line, `drawIndexed(18, 767)` |
| 1 | `renderContext_1` | canvas, `load` | output quad |
| 2 | `clear` | colour `load`, **depth `clear` 1** | *(nothing)* — `renderer.clearDepth()` |
| 3 | `renderContext_1` | canvas, `load` | output quad again — a redundant re-blit that `clearDepth()` triggers; pixel-neutral |
| 4 | `renderContext_0` | colour `load`, depth `load` | background `drawIndexed(5952, 1)`, then line `drawIndexed(18, 767)` |
| 5 | `renderContext_1` | canvas, `load` | output quad |

Six `queue.submit`s. Note the clear alpha of pass 0 is **1**, not 0.

The background geometry is 13 068 B of position (1089 vertices = 33×33) and
23 808 B of uint32 index (5952) — `SphereGeometry( 1, 32, 32 )`, which the port
already builds for `Background::Node`.

**The dumper does not spy `setViewport` / `setScissorRect`.** They are genuinely
issued by `WebGPUBackend` and are load-bearing here; they simply do not appear in
`dump.json`. Everything in §2.5 about the viewport rect comes from the page
source plus `ScreenNode.update()` (`renderer.getViewport( v ).multiplyScalar(
pixelRatio )`), not from the dump. Worth a one-line addition to the dump tool's
pass-command wrapper (`tools/dump-webgpu.mjs`) for whoever touches it next.

### 3.5 The generated WGSL, and what it tells the port

`m00_vertex_vertex.wgsl` is the rung. Its spine, in order:

```wgsl
positionLocal   = position;
modelViewMatrix = ( render.cameraViewMatrix * object.nodeUniform5 );
start = ( modelViewMatrix * vec4<f32>( instanceStart, 1.0 ) );
end   = ( modelViewMatrix * vec4<f32>( instanceEnd,   1.0 ) );
if ( render.cameraProjectionMatrix[ 2u ][ 3u ] == -1.0 ) {          // perspective
    if ( start.z < 0.0 && end.z > 0.0 )      { end   = vec4( mix( start.xyz, end.xyz, fn2( start, end ) ), end.w ); }
    else { if ( end.z < 0.0 && start.z >= 0.0 ) { start = vec4( mix( end.xyz, start.xyz, fn2( end, start ) ), start.w ); } }
}
nodeVar0 = cameraProjectionMatrix * end;    nodeVar1 = cameraProjectionMatrix * start;
nodeVar2 = ( nodeVar0.xyz / vec3( nodeVar0.w ) ).xy - ( nodeVar1.xyz / vec3( nodeVar1.w ) ).xy;   // dir
nodeVar3 = render.nodeUniform6.z / render.nodeUniform6.w;                                        // aspect
nodeVar2.x = nodeVar2.x * nodeVar3;   nodeVar2 = normalize( nodeVar2 );
nodeVar4 = vec4( 0.0, 0.0, 0.0, 1.0 );                                                           // clip = vec4()
offset   = vec2( nodeVar2.y, -nodeVar2.x );
nodeVar2.x = nodeVar2.x / nodeVar3;   offset.x = offset.x / nodeVar3;
if ( position.x < 0.0 ) { nodeVar5 = -offset; } else { nodeVar5 = offset; }    offset = nodeVar5;
if ( position.y < 0.0 ) { offset = offset - nodeVar2; }
else { if ( position.y > 1.0 ) { offset = offset + nodeVar2; } }
offset = offset * vec2( object.nodeUniform7 );                                 // linewidth
offset = offset / vec2( render.nodeUniform6.w / render.nodeUniform8 );         // viewport.w / screenDPR
if ( position.y < 0.5 ) { nodeVar6 = nodeVar1; } else { nodeVar6 = nodeVar0; }  nodeVar4 = nodeVar6;
offset   = offset * vec2( nodeVar4.w );
nodeVar4 = nodeVar4 + vec4( offset, 0.0, 0.0 );
nodeVar7 = ( ( object.nodeUniform1 * render.cameraWorldMatrix ) * render.cameraProjectionMatrixInverse ) * nodeVar4;
positionLocal = nodeVar7.xyz / vec3( nodeVar7.w );
varyings.nodeVarying4 = uv;  varyings.nodeVarying5 = position;
varyings.nodeVarying6 = instanceColorStart;  varyings.nodeVarying7 = instanceColorEnd;
v_positionView = ( modelViewMatrix * vec4( positionLocal, 1.0 ) ).xyz;
VERTEX_nodeVar12 = render.cameraProjectionMatrix * vec4( v_positionView, 1.0 );
VERTEX_v_modelViewProjection = VERTEX_nodeVar12;
varyings.builtinClipSpace    = VERTEX_v_modelViewProjection;
```

with one emitted helper:

```wgsl
fn fn2 ( start : vec4<f32>, end : vec4<f32> ) -> f32 {
    var nodeVar0 : f32;
    if ( render.cameraProjectionMatrix[ 2u ][ 2u ] > 0.0 ) {
        nodeVar0 = ( -render.cameraProjectionMatrix[ 3u ][ 2u ] ) / ( render.cameraProjectionMatrix[ 2u ][ 2u ] + 1.0 );
    } else {
        nodeVar0 = ( render.cameraProjectionMatrix[ 3u ][ 2u ] * -0.5 ) / render.cameraProjectionMatrix[ 2u ][ 2u ];
    }
    return ( nodeVar0 - start.z ) / ( end.z - start.z );
}
```

Six things to copy exactly:

1. **The whole fat-line transform is a `positionLocal` round-trip.**
   `setupPosition` computes a clip-space position and then pushes it *back*
   through `modelWorldMatrixInverse * cameraWorldMatrix * cameraProjectionMatrixInverse`
   and a perspective divide, so the standard MVP that follows re-does the
   forward transform. That is why the render struct carries both the projection
   and its inverse, and why `v_positionView` / `VERTEX_v_modelViewProjection`
   still appear at the end. **Do not "optimise" the round-trip away** — it is not
   algebraically the identity in f32, and it is the only thing that lets the fat
   line reuse the ordinary MVP tail.
2. `fn2` is a real WGSL function with a layout, called twice — the port's
   `shader_fn` + `call` (`src/nodes/tsl.rs:2122`, `:2152`), not `inline_fn`.
3. Every `.select()` is a `nodeVarN` plus an `if/else`, which is exactly what
   `Node::Select` emits (`src/nodes/builder.rs:1083`). No WGSL `select()` builtin
   anywhere.
4. `If(…).ElseIf(…)` is a **nested** `if/else` with the inner `if` in the else
   block, and a trailing blank line inside the else. The port's `Node::If` has no
   else arm (§4.2).
5. Scalar × vector is an explicit splat: `vec2<f32>( object.nodeUniform7 )`,
   `vec3<f32>( nodeVar0.w )`. A `vec3` divided by its own `w` splats to `vec3`,
   *then* `.xy` is taken.
6. Matrix element-of-element is `m[ 2u ][ 3u ]` — the port's `element(i).element(j)`
   already prints `[ 2u ]`, and here Three prints `2u` too, so for once there is
   no divergence.

The fragment (`m01_fragment_fragment.wgsl`), complete:

```wgsl
DiffuseColor   = vec4<f32>( object.nodeUniform9, 1.0 );
DiffuseColor.w = DiffuseColor.w * object.nodeUniform10;
alpha          = 1.0;
if ( abs( nodeVarying4.y ) > 1.0 ) {
    if ( nodeVarying4.y > 0.0 ) { nodeVar8 = nodeVarying4.y - 1.0; } else { nodeVar8 = nodeVarying4.y + 1.0; }
    if ( ( nodeVarying4.x * nodeVarying4.x + nodeVar8 * nodeVar8 ) > 1.0 ) { discard; }
}
DiffuseColor.w = DiffuseColor.w * alpha;
if ( nodeVarying5.y < 0.5 ) { nodeVar9 = nodeVarying6; } else { nodeVar9 = nodeVarying7; }
nodeVar10 = DiffuseColor.xyz * nodeVar9;
DiffuseColor.x = nodeVar10[ 0 ];  DiffuseColor.y = nodeVar10[ 1 ];  DiffuseColor.z = nodeVar10[ 2 ];
nodeVar11 = max( vec4<f32>( DiffuseColor.xyz, DiffuseColor.w ), vec4<f32>( 0.0 ) );
Output = nodeVar11;  output.color = nodeVar11;
```

* **There is no `DiffuseColor.w = 1.0`.** `builder.isOpaque()` is
  `transparent === false && blending === NormalBlending && alphaToCoverage ===
  false`; `Line2NodeMaterial` sets `blending = NoBlending`, so the opaque branch
  is skipped. The port's `is_opaque()` (`src/materials/mod.rs:381`) already
  encodes this and already gates the line — **no change needed**, but check it,
  because `docs/nodes.md` §8 records the port emitting it in a case where Three
  does not.
* **`vertexColors: true` does *not* produce the base `vertexColor()` multiply.**
  `super.setupDiffuseColor()`'s `vertexColors === true && geometry.hasAttribute(
  'color' )` fails (the geometry has `instanceColorStart`, not `color`), and the
  `Line2` override then does its own `positionGeometry.y.lessThan( 0.5 ).select(
  instanceColorStart, instanceColorEnd )` multiply — in the **fragment**, through
  four varyings. The port's `vertexColor()` has "no no-attribute fallback"
  (`docs/nodes.md` §8) — confirm it stays silent here.
* `rgb.mulAssign` is emitted as **three component assignments**
  (`DiffuseColor.x = nodeVar10[ 0 ]` …), not one `.xyz =`. Copy the form.
* `positionGeometry` crosses as a `vec3` varying and `uv` as a `vec2`, both read
  only in the fragment. `instanceColorStart/End` are *instanced attributes read
  as varyings* — the port's `AttributeNode.generate()` rule already turns an
  attribute read outside the vertex stage into a varying (`docs/nodes.md` §9.2).

---

## 4. Gap list against the port

Against `main` at `2cf90bd` (rungs 0–9 + 13 + lines + controls merged; rungs 10/11/12
in flight on their own branches). "have" names the Rust item; "missing" names the
Three source, the Rust file, and a size.

### 4.1 The good news first — what rung 13 and the hairline-lines work already paid for

| the example needs | the port has |
|---|---|
| interleaved **instanced** vertex attributes, stride 24, two `vec3` views | `AttributeSource::Instance { buffer, offset }` + `NodeProgram::vertex_buffers()` grouping by `Rc` identity — `src/nodes/builder.rs:202-237`, `src/nodes/tsl.rs:2082` `instanced_data_attribute( data, item_size, offset, ty )`. `item_size = 6`, offsets `0` and `3`, `Type::Vec3` is a literal fit. Gated by `tests/nodes_instanced_attributes.rs`. |
| an instanced draw without an instance matrix | `Renderable::instance_count` is already separate from `SetupContext::instance_count` (which is what inserts the `InstanceNode` transform) — `src/renderer/mod.rs:227` vs `src/materials/node_material.rs:26` |
| a `setupX()` seam a subclass overrides | `MaterialKind` + `with_material_position_view` — `src/materials/mod.rs:88-105`, `src/materials/node_material.rs:158-166` (built for `Sprite` at rung 13) |
| `NoBlending` → no blend state in the pipeline | `Blending::No` + `blend_state()` — `src/materials/blending.rs`, `src/renderer/programs.rs` |
| `isOpaque()` gating `DiffuseColor.w = 1.0` | `MeshBasicNodeMaterial::is_opaque()` — `src/materials/mod.rs:381` |
| `scene.backgroundNode = color( … )` → the skybox sphere, `cw`/`always`/no depth write | `Background::Node( Color )` + `background_node_color_node()` — `src/objects/scene.rs:21`, `src/renderer/mod.rs:693` |
| MSAA 4, `rgba16float` frame-buffer target, the output colour transform | `samples`, `frame_buffer_target()`, `render_output()` |
| `Fn()` with a layout, called twice | `shader_fn` / `call` — `src/nodes/tsl.rs:2122`, `:2152` |
| `.select()` → `nodeVarN` + `if/else` | `Node::Select` — `src/nodes/builder.rs:1083` |
| `discard`, `abs`, `length`, `normalize`, `mix`, `max`, `to_var`, `assign`, `add_assign`, `mul_assign`, `element`, swizzles | all present in `src/nodes/tsl.rs` |
| `SphereGeometry( 1, 32, 32 )` for the background | `src/geometries/sphere.rs` |

### 4.2 Node system — missing

| thing | Three source | lands in | size | tier |
|---|---|---|---|---|
| `If(…).ElseIf(…)` / an `else` arm on `Node::If` | `StackNode.js` `ElseIf`/`Else` | `src/nodes/node.rs` (`Node::If { cond, body, else_body }`), `src/nodes/builder.rs:1147` + `:409` | ~40 lines Rust | node-system, **shared path** — every existing `if_then`/`discard_if` call site must keep emitting the current text |
| `.sub_assign( v )` | `PropertyNode` assign ops | `src/nodes/tsl.rs:1105` neighbourhood | ~6 lines | node-system |
| `viewport` (vec4) | `src/nodes/display/ScreenNode.js:224` (`ScreenNode.VIEWPORT`) | `UniformSource::Viewport`, `src/nodes/node.rs:121`; value in `UniformContext` | ~25 lines | node-system + renderer |
| `screenDPR` | `ScreenNode.js:190` | `UniformSource::ScreenDpr` (render group) | ~15 lines | node-system + renderer |
| `cameraProjectionMatrixInverse` | `src/nodes/accessors/Camera.js` | `UniformSource::CameraProjectionMatrixInverse` + `Matrix4::invert` (the port has it) | ~20 lines | node-system |
| `modelWorldMatrixInverse` | `src/nodes/accessors/ModelNode.js` | `UniformSource::ModelWorldMatrixInverse` (**object** group) | ~20 lines | node-system |
| `materialLineWidth` | `MaterialNode.js` (`MaterialNode.LINE_WIDTH`) | `UniformSource::MaterialLineWidth` + `Material.linewidth` | ~15 lines | material |
| `materialLineScale` / `…DashSize` / `…GapSize` / `…DashOffset` | same | same | ~40 lines | material — **dashed path only, not on the pixel hook** |
| `varyingProperty( type, name )` | `PropertyNode.js` | `to_varying( Some( name ), … )` exists (`src/nodes/tsl.rs:263`) — **have**, but the world-units path needs `.assign()` on a varying before it is read, which is a new shape | 0–20 lines | node-system |
| `smoothstep`, `fwidth` | — | **have** (`tsl.rs:489`, `:476`) — needed only by the A2C path | 0 | — |

`viewport`, `screenDPR`, `cameraProjectionMatrixInverse` and
`modelWorldMatrixInverse` are four new `UniformSource` variants plus four
`UniformContext` fields plus four writers. Nothing else in the tree reads them,
so the blast radius is one `match` arm each in `src/nodes/builder.rs` and
`src/renderer/mod.rs`. Low risk.

### 4.3 `Line2NodeMaterial` itself — the heart

| piece | Three source | lands in | size |
|---|---|---|---|
| `mvpLine`, screen-space branch + `trimSegmentAlpha` | `Line2NodeMaterial.js:57-69`, `:112-291` (screen branch `:251-287`) | new `src/materials/line2.rs` | ~130 lines JS read → ~200 lines Rust |
| `mvpLine`, world-units branch | `:134-139`, `:207-250` | same | ~50 JS → ~90 Rust — **off the pixel hook**, but it is the workspace consumer's stated want |
| `alphaLine`, round-endcap non-A2C branch | `:300-388` (`:370-382`) | same | ~15 JS → ~30 Rust |
| `alphaLine`, A2C + world-units branches | `:323-368` | same | ~45 JS → ~80 Rust — off the hook |
| `setupDiffuseColor` override (`alpha` multiply + `instanceColor` select) | `:495-518` | `src/materials/node_material.rs` match arm | ~25 JS → ~45 Rust |
| `setupPosition` override (the inverse round-trip) | `:526-540` | `src/materials/node_material.rs` — a **new seam**: `setupPosition` is not currently overridable, only `setupPositionView` is | ~15 JS → ~30 Rust + ~20 lines of seam |
| `MaterialKind::Line2` + `linewidth` / `dashed` / `world_units` / `dash_*` fields, `blending = NoBlending` default | `:396-631` | `src/materials/mod.rs` | ~60 Rust |

**The new seam is the one design decision.** rung 13 installed
`with_material_position_view` for `SpriteNodeMaterial`. `Line2NodeMaterial`
overrides `setupPosition` instead, which in `setup_inner`
(`src/materials/node_material.rs:169-215`) is the *pre-vertex* block. The
cheapest faithful shape is a `with_material_position(…)`-style hook, or simply a
`match material.kind { Line2 => pre_vertex.extend( line2_setup_position( … ) ), _ => … }`
arm ahead of the existing `position_node` / `instance_count` block — `Line2`
geometry is never an `InstancedMesh`, so the two cannot collide. Note the port's
recorded divergence (positionNode applied *before* the instance transform, r186
does the reverse, `docs/nodes.md` §10) does not bite here.

### 4.4 Where the material gets `instanceStart` — the one real design question

Three writes `attribute( 'instanceStart' )` and resolves it against
`builder.geometry`. The port's instanced-attribute node **carries its data**
(`instanced_data_attribute( &Rc<Vec<f32>>, item_size, offset, ty )`), because
"the node graph is built from the material and the renderer has no attribute-name
table" (`src/nodes/tsl.rs:2075`). Two ways to bridge:

* **(a) `SetupContext` carries the buffers** — add
  `line_segments: Option<LineSegmentsAttributes>` holding two `Rc<Vec<f32>>`
  (positions, colours) next to the existing `morph: Option<MorphEntry>`, which is
  the exact precedent for geometry-derived setup input. Hashes by `Rc` pointer,
  so the cache key stays cheap and correct. **~60 lines, reuses everything.**
  Cost: `SetupContext` (core) gains a fat-lines-shaped field, and the renderer
  must populate it, so core has to know the geometry convention.
* **(b) give `BufferGeometry` real interleaved/instanced attributes** —
  `BufferAttribute` gains `stride`/`offset`/`instanced`, `VertexBufferDesc::Geometry`
  gains stride/offset/step-mode and grouping by underlying array identity,
  `InstancedBufferGeometry::instanceCount` becomes a geometry field, and
  `attribute( name, ty )` resolves as today. **~250 lines, and it touches
  `ensure_geometry` / `vertex_buffers()` / `programs.rs` — the path every example
  uses.** This is the real three.js shape and makes `Line2NodeMaterial` carry
  zero geometry knowledge.

**Recommendation: (a) for this rung, (b) as a follow-up issue.** (a) fits one
sitting and cannot regress another example; (b) is the right end state and is
worth doing once `Points`/`BatchedMesh` (rungs 11/12) have shown what else wants
it.

### 4.5 Renderer — viewport, scissor, autoClear, clearDepth

**Entirely missing.** `grep -rn "set_viewport\|set_scissor\|auto_clear\|clear_depth" src/` returns nothing.

| thing | Three source | lands in | size |
|---|---|---|---|
| `Renderer.setViewport/getViewport` (`Vector4`, bottom-left origin, × pixelRatio) | `src/renderers/common/Renderer.js` | `src/renderer/mod.rs` state + `pass.set_viewport()` in `draw()` (`:1514`), with the y-flip | ~60 lines |
| `setScissorTest` / `setScissor` | same | `pass.set_scissor_rect()`, same y-flip | ~40 lines |
| `autoClear` (and `autoClearColor`/`Depth`) | same | `render_list( …, clear )` already takes an `Option`; thread `auto_clear` through `render()` (`:834`, `:888`) | ~25 lines |
| `clearDepth()` | `Renderer.clear( color, depth, stencil )` | a depth-only pass on the frame-buffer target | ~40 lines |
| two `render()` calls into the same cached frame-buffer target | — | already works: `frame_buffer_target()` memoises (`:2776`) and `render_output()` is per-call | 0 |

This is **renderer work on a path every example uses** — `draw()` and
`render_list()`. It is the highest-regression-risk item in the rung, and it is
also the single most reusable thing the rung produces (every multi-view,
picture-in-picture or split-screen consumer wants it; `webgl_lines_fat`,
`webgpu_lines_fat_wireframe` and a dozen WebGL examples use the same inset).

Two details that will bite:

* Three's viewport/scissor origin is **bottom-left**; wgpu's is **top-left**.
  `y_wgpu = target_height - y_three - height`. Get it wrong and the inset lands
  at the bottom-left instead of the top-left — a 3.9% diff that looks like a
  *correct* render.
* The `viewport` **uniform** must be the *three.js* rect (`20, 355, 125, 125`),
  not the flipped one: only `viewport.z`/`.w` are read by the shader, but writing
  the flipped `y` into `.y` would be wrong for anything that later reads
  `viewportCoordinate`.
* `clearDepth()` runs **before** `setScissorTest( true )`, so it is a full-target
  depth clear. Do not scissor it.

### 4.6 Geometry / objects / curves

| thing | Three source | lands in | size | tier |
|---|---|---|---|---|
| `CatmullRomCurve3` (+ the `Curve.getPoint` base) | `src/extras/core/Curve.js`, `src/extras/curves/CatmullRomCurve3.js` | `src/extras/` (new module) | ~260 JS → ~230 Rust | **core** (three ships it in `src/`) |
| `LineSegmentsGeometry` (base quad, `setPositions`, `setColors`, `computeBoundingBox/Sphere`) | `examples/jsm/lines/LineSegmentsGeometry.js` | addon (§9) | 298 JS → ~180 Rust | addon |
| `LineGeometry` (`setPositions`/`setColors`/`setFromPoints` pair conversion) | `examples/jsm/lines/LineGeometry.js` | addon | 157 JS → ~70 Rust | addon |
| `LineSegments2` (`computeLineDistances`, `onBeforeRender` resolution, `raycast`) | `examples/jsm/lines/webgpu/LineSegments2.js` | addon | 419 JS, of which **~280 is raycasting** → ~90 Rust without raycast | addon |
| `Line2` | `examples/jsm/lines/webgpu/Line2.js` | addon | 46 JS → ~25 Rust | addon |
| `hilbert3D` | `examples/jsm/utils/GeometryUtils.js` | the example file itself | ~40 JS → ~40 Rust | example |
| `Color.setHSL( h, s, l, SRGBColorSpace )` | `src/math/Color.js` | `src/math/color.rs` — **check**: the port has `Color`; confirm `set_hsl` with a colour-space argument exists | 0–30 Rust | core |

**`LineSegmentsGeometry::computeBoundingSphere` is not optional cosmetics.** The
port's `bounding_sphere_in()` computes from the `position` attribute, which for a
fat line is the fixed 8-vertex ±2 quad box around the origin — a radius-2.45
sphere instead of the curve's ~26. It happens to stay inside both frusta here, so
the graded frame survives either way, but a port that culls on the quad box will
drop the line the moment a consumer moves the camera. Port the real one.

`Raycaster` does not exist in the port at all (`grep -rln raycast src/` → only
`src/cameras/mod.rs`), so `LineSegments2::raycast` is out of scope for this rung
and takes `webgpu_lines_fat_raycasting` with it (§7).

### 4.7 Dependencies on rungs 10 / 11 / 12

**None.** No skinning, no `BatchedMesh`, no compute, no storage buffers, no
`Points`. The only shared surface is `src/renderer/programs.rs` (rung 11 will be
in the vertex-layout code too) and `src/renderer/mod.rs::draw()` (rung 12's
compute passes). Land §4.5 early and rebase the rest on top.

### 4.8 Nothing needed

No textures, no loaders, no lights, no shadows, no fog, no tone mapping
(`NoToneMapping`), no post-processing, no `viewportOpaqueMipTexture` (the
`transparent` branch is off), no `Timer`, no `Inspector`, no compute, no
`Math.random`, no render target beyond the frame-buffer target the port has.

---

## 5. Gates beyond the pixel diff

Four, in the order they should go green. **Three of them need no GPU at all.**

### 5.1 `spline_oracle.json` — the geometry oracle (dumped, next to this plan)

Format:

```jsonc
{ "hilbertPointCount": 64, "divisions": 768, "instanceCount": 767,
  "hilbert":   [ 64 × 3 f32 ],    // the CatmullRomCurve3 control points
  "positions": [ 768 × 3 f32 ],   // spline.getPoint( i/768 )
  "colors":    [ 768 × 3 f32 ] }  // Color.setHSL( i/768, 1, 0.5, SRGBColorSpace ), linear-srgb
```

First values, as f32:

```
hilbert  [0..6]  = -15, 15, -15,  -15, 5, -15
positions[0..9]  = -15, 15, -15,  -15.030885696411133, 14.148801803588867, -15,
                   -15.112503051757812, 13.246871948242188, -15
colors   [0..9]  = 1, 0, 0,  1, 0.0006046826601959765, 0,  1, 0.001209365320391953, 0
```

Regenerate with `node make-oracle.mjs [three-js-root] > spline_oracle.json`. The
script imports `src/math/Vector3.js`, `src/math/Color.js`,
`src/extras/curves/CatmullRomCurve3.js` and `src/constants.js` directly — no
bundle, no DOM, no vendor-tree modification — and inlines `hilbert3D` verbatim
because `GeometryUtils.js` imports the bare specifier `three`.

This pins the `CatmullRomCurve3` port, `Color::set_hsl` in sRGB, and the
`LineGeometry` pair conversion (the derived `instanceStart/End` buffer is
`positions` with each interior point duplicated; its length must be 767 × 6 =
4602 floats = **18 408 bytes**, the dump's buffer-7 size). A unit test asserting
the 768 points to f32 fails long before any pixel does.

### 5.2 WGSL diff against `dump/m00_vertex_vertex.wgsl` and `m01_fragment_fragment.wgsl`

`examples/dump_wgsl.rs` gains a `line2` section. Expected divergences are only
the classes `docs/nodes.md` §8 already lists — `nodeVarN`/`nodeUniformN`
numbering, `var<private>` declaration order, the `VERTEX_` sub-build temps,
attribute `@location` order — **plus one new class to add: uniform-struct member
order** (§3.3). Anything else is a real difference. This gate covers the entire
vertex transform, which is where all the risk is, and needs no correct pixel.

### 5.3 Vertex-layout readback / descriptor assertion

A unit test in the shape of `tests/nodes_instanced_attributes.rs`: a
`Line2NodeMaterial` over a 3-segment `LineGeometry` produces exactly four
`VertexBufferDesc`s — strides 12 / 24 / 8 / 24, step modes vertex / instance /
vertex / instance, attribute offsets 0 / (0, 12) / 0 / (0, 12) — and
`instance_count == 3`. Pure CPU.

### 5.4 A viewport/scissor readback

`tests/renderer_viewport.rs` in the shape of `tests/renderer_lines.rs`: render a
full-screen quad into 64×64 with `set_viewport( 8, 8, 16, 16 )` +
`set_scissor_test( true )` and read the framebuffer back — the lit box must be at
**top-left** `(8, 40)`–`(24, 56)` in wgpu rows, i.e. three.js' bottom-left
`(8, 8)`. This is the cheapest possible catch for the y-flip, which is the single
most likely way this rung fails while looking right.

### 5.5 Three's own unit tests

`test/unit/` has no `Line2`/`LineSegmentsGeometry` suite (they are addons).
`test/unit/src/extras/curves/CatmullRomCurve3.tests.js` **does** exist and is a
direct QUnit port for §4.6's curve work — take it.

---

## 6. Order of work

Six steps; each leaves the ladder green.

1. **Viewport, scissor, `autoClear`, `clearDepth`.** Renderer only — `Vector4`
   viewport/scissor state, the bottom-left→top-left flip, `pass.set_viewport` /
   `set_scissor_rect` in `draw()`, `auto_clear` threaded into
   `render_list( …, clear )`, and a depth-only clear pass.
   **Gate:** the existing ten-rung ladder is bit-identical (nothing sets a
   viewport, so every pass keeps the full-target default), plus
   `tests/renderer_viewport.rs` (§5.4). No new example.
2. **`CatmullRomCurve3` + `Curve`.** `src/extras/`, ported with the QUnit suite
   (`test/unit/src/extras/curves/CatmullRomCurve3.tests.js`).
   **Gate:** the QUnit port, and a test asserting `spline_oracle.json`'s 768
   points to f32. No GPU.
3. **The four camera/screen uniforms and the node-system bits.** `viewport`,
   `screenDPR`, `cameraProjectionMatrixInverse`, `modelWorldMatrixInverse`,
   `materialLineWidth`, `sub_assign`, and the `else` arm on `Node::If`.
   **Gate:** `cargo run --example dump_wgsl` output for every existing material
   is unchanged (the `else` arm is the only risk here — it must not alter what
   `if_then` and `discard_if` emit today).
4. **`addons/lines`: the geometries and the objects.** `LineSegmentsGeometry`,
   `LineGeometry`, `LineSegments2`, `Line2`, `computeLineDistances`, the real
   `computeBoundingSphere`. No material yet.
   **Gate:** §5.1's oracle test extended to the derived interleaved buffers —
   767 instances, 18 408 bytes, the pair duplication — and `cargo test -p
   three-rs-lines`. No GPU.
5. **`Line2NodeMaterial`, screen-space / no-dash / no-A2C.** `MaterialKind::Line2`,
   the `setup_position` seam, `mvpLine`'s screen branch, `trimSegmentAlpha` as a
   `shader_fn`, `alphaLine`'s round endcaps, the `instanceColor` select, and the
   `SetupContext::line_segments` bridge of §4.4(a).
   **Gate:** §5.2's WGSL diff against `dump/m00`/`m01`, line by line, and §5.3's
   vertex-layout assertion. **Do not look at a pixel before this gate is green** —
   the vertex program is the whole rung and the diff is free.
6. **The example and the image.** `addons/lines/examples/webgpu_lines_fat.rs`
   (inlined `hilbert3D`, `lookAt(0,0,0)` instead of OrbitControls, the two-render
   `animate()` of §2.5) plus `addons/lines/tests/webgpu_lines_fat.rs`.
   **Gate:** the diff image, ceiling 100 pixels.
   If it is wrong, work down this list before touching the shader: **(i)** is the
   inset in the *top*-left? (y-flip); **(ii)** is the line width 4× wider inside
   the inset than outside? (`viewport.w` is the viewport *height*, 125 vs 500 —
   if both look the same the `viewport` uniform is not being updated per render);
   **(iii)** is `DiffuseColor.w = 1.0` being emitted? (it must not be:
   `blending = NoBlending` ⇒ `!is_opaque()`); **(iv)** are the two interleaved
   buffers `stepMode: instance` with stride 24? (a `Vertex` step mode draws
   eight segments' worth of garbage, silently); **(v)** is the depth cleared
   between the two renders?

Steps 1 and 3 are the shared renderer/node work and should go on a branch that
can merge independently of the lines work; they are also what rungs 11/12 are
most likely to conflict with.

---

## 7. What this rung unlocks

Three webgpu examples in the vendor tree import the fat-line stack
(`grep -ln "lines/webgpu\|Line2NodeMaterial" examples/*.html`):

| example | grade here | what it still needs after this rung | estimate |
|---|---|---|---|
| **`webgpu_lines_fat`** | **0.0%** | — | this rung |
| `webgpu_lines_fat_wireframe` | 0.1% | `WireframeGeometry2` (49 JS) + `Wireframe` (86 JS) + core `WireframeGeometry` + the **alphaToCoverage** path (`fwidth`/`smoothstep` branch in `alphaLine` **and** `alphaToCoverageEnabled: true` in `programs.rs:212`) + `LineDashedNodeMaterial` for the hairline comparison object (hidden in the graded frame) + drop `Inspector` but keep its five PRNG draws. `IcosahedronGeometry` is already in `src/geometries/polyhedron.rs`. Same inset/scissor `animate()`. | **cheap — ~250 lines of Rust on top**, one short follow-up rung |
| `webgpu_lines_fat_raycasting` | 0.1% | the **world-units** branch of `mvpLine` + `closestLineToLine` + the A2C path, a `Raycaster` (**the port has none**), `Raycaster.params.Line2`, `LineSegments2::raycast` (~280 JS of screen-space segment maths), `Timer`, `alpha: true`, `MeshBasicMaterial` with `depthTest: false`, and the `Inspector` PRNG offset | **not cheap — a rung of its own**, gated on a `Raycaster` port |

Also unlocked *outside* the ladder: fat lines are what the 3D-workspace consumer
asked for (width + dashes), and `Renderer::set_viewport`/`set_scissor`/`autoClear`/
`clearDepth` (§4.5) is generic multi-view support that `three-rs-controls`'
overview mode and every picture-in-picture consumer wants. The four new
camera/screen uniforms (`viewport`, `screenDPR`, `cameraProjectionMatrixInverse`,
`modelWorldMatrixInverse`) are prerequisites for a large slice of
`examples/jsm/postprocessing` too.

`webgl_lines_fat*` are the WebGL twins and are not on this ladder.

---

## 8. Verdict

**Rung-sized, but at the top of the range: one Opus worker, one long sitting —
and only because rung 13 already built the interleaved-instanced-attribute path
and the `MaterialKind` seam.** Roughly **1 100–1 300 lines of Rust** across
`src/renderer` (~170), `src/nodes` (~130), `src/materials/line2.rs` (~300 for the
graded variant, ~470 for the whole material), `src/extras/curves` (~230) and
`addons/lines` (~350), plus ~200 lines of tests and the example.

If the director wants it smaller, the clean cut is to land **steps 1–3 as their own
rung** (viewport/scissor/clearDepth + `CatmullRomCurve3` + the four uniforms,
~500 lines, all gated without a new image) and the material + addon crate as the
next one. That split is natural because step 1 is generic renderer work that
several other things want, and it is the piece most likely to conflict with rungs
11/12.

Top three risks:

1. **The viewport/scissor y-flip** (§4.5). Wrong by a flip and the inset is in
   the bottom-left: a ~3900-pixel diff on a 100-pixel budget, in an image that
   otherwise looks completely correct. §5.4 is the gate; write it first.
2. **The `positionLocal` round-trip through `modelWorldMatrixInverse *
   cameraWorldMatrix * cameraProjectionMatrixInverse`** (§3.5). It reads like
   dead arithmetic and is not; simplifying it, or getting the multiply order
   wrong, yields a line that is *nearly* right and impossible to debug from the
   image. The WGSL diff (§5.2) catches it for free.
3. **`SetupContext` growing a fat-lines-shaped field** (§4.4a). It is the
   pragmatic choice and it is a wart in a core struct that every material's cache
   key derives from. Get the `Rc`-pointer hashing right, write the follow-up issue
   for the geometry-attribute path (b) at the same time, and do not let (b) creep
   into this rung — `ensure_geometry` / `vertex_buffers()` / `programs.rs` are on
   every example's path.
## 9. Where it belongs in the port (appendix — asked for separately)

Three tiers, and the split falls out of where three.js itself keeps each file.

**`Line2NodeMaterial` → `src/materials/line2.rs` (core).** three.js ships it in
`src/materials/nodes/Line2NodeMaterial.js`, not in `examples/jsm/`. The README's
Addons rule is explicit — `addons/` holds crates "that are **not** ports of
anything in three.js' `src/`" — so a core file stays core. It also *has* to:
it needs a new `MaterialKind` variant, a new `setup_position` seam in
`src/materials/node_material.rs`, and six new `UniformSource` variants in
`src/nodes/node.rs`. None of that is reachable from a downstream crate.
`CatmullRomCurve3` is core for the same reason (`src/extras/curves/`).

**`LineSegmentsGeometry` / `LineGeometry` / `LineSegments2` / `Line2` (and later
`WireframeGeometry2` / `Wireframe`) → `addons/lines`, crate `three-rs-lines`.**
These are `examples/jsm/lines/`, which is exactly the tier the README maps onto
`addons/`. The crate is thin — a geometry builder, a pair-conversion, a `Mesh`
wrapper and `computeLineDistances` — precisely because the material is core. It
depends on `three-rs` the way `three-rs-controls` does, and nothing in `three-rs`
depends on it.

**The graded example and its e2e test go in the addon crate**, following
`sdf-text`'s `examples/d33_treemap_labels.rs` + `tests/d33_treemap_labels.rs`,
which already call `three_rs::testing::{write_png, compare, three_js_dir}` from
outside the root crate. Add `"addons/lines"` to the workspace members.

The cost, and the director should decide it explicitly: **`cargo test --test e2e`
in the root crate will not cover this rung.** The root ladder
(`tests/e2e/main.rs`) pulls its examples in with `#[path = "../../examples/…"]`
and cannot reach a member crate. `sdf-text` already lives with that; a second
crate doing the same makes "the ladder" two commands. If the director would
rather keep one ladder, the alternative is to put `LineSegmentsGeometry` in
`src/geometries/line_segments.rs` and `LineSegments2`/`Line2` in
`src/objects/line2.rs` next to `src/objects/line.rs` — cheaper operationally,
but it puts `examples/jsm/` code in `src/`, which is the thing the README's
Addons section was written to prevent. **My recommendation is the addon crate**,
with a one-line note in `README.md`'s Addons section and a `CONTRIBUTING.md`
line that the full ladder is `cargo test --test e2e` plus
`cargo test -p sdf-text -p three-rs-lines`.

---

