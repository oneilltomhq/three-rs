# Rung 6 scout — `webgpu_morphtargets`

Scouted 2026-09-12. Example: `~/src/vendor/three.js/examples/webgpu_morphtargets.html`.
Everything here is read off the real page; nothing is inferred from the reference image.

Files next to this plan:

| file | what |
|---|---|
| `MeshPhongNodeMaterial.vert.wgsl` / `.frag.wgsl` | the only scene program Three compiles |
| `outputColorTransform.vert.wgsl` / `.frag.wgsl` | the output pass (identical in shape to rung 2/4's) |
| `pipelines.json` | both `createRenderPipeline` descriptors, verbatim |
| `bind-group-layouts.json` | every `createBindGroupLayout` descriptor, verbatim |
| `webgpu_morphtargets.jpg` | the grader's reference (copy of `examples/screenshots/`) |
| `three_actual_800x500.png` | Three's own full-size render of the deterministic frame (pre-downscale) |

Dump method: a temporary `test/e2e/_dump_rung6.mjs` modelled on `puppeteer.js`
(same flags, `--use-angle=vulkan`, no `--disable-vulkan-surface`, profile
`.puppeteer_profile_rung6`), same `deterministic-injection.js` +
`clean-page.js` + the three build-file rewrites, plus an
`evaluateOnNewDocument` spy that wraps `GPUDevice.prototype.createShaderModule`
/ `createBindGroupLayout` / `createPipelineLayout` / `createRenderPipeline(Async)`
/ `createComputePipeline(Async)`. Script and profile deleted; the vendor tree is
back to only `M test/e2e/puppeteer.js`.

Result: **4 shader modules, 2 render pipelines, 3 bind-group layouts, 0 compute
pipelines.** No background pipeline (`scene.background` is a `Color`, so it is a
clear colour, not a skybox mesh — unlike rung 3). No mipmap pipeline.

---

## 1. What Three compiles

### 1.1 `renderPipeline_MeshPhongMaterial_17`

Vertex buffers: one, `arrayStride 12`, `@location(0) position : float32x3`,
`stepMode vertex`. **`normal` is not an attribute of either stage** — the
material is `flatShading: true`, so the fragment stage derives the normal from
screen-space derivatives of the view position.

Target `rgba16float` (the internal frame-buffer target), `multisample.count 4`,
depth `depth24plus` / `depthWriteEnabled true` / `less-equal`, primitive
`triangle-list` / `ccw` / cull `back`.

Bind groups:

```
group 0 "render"  binding 0  visibility 7 (VERTEX|FRAGMENT|COMPUTE)  uniform buffer
group 1 "object"  binding 0  visibility 7                            uniform buffer
                  binding 1  visibility 1 (VERTEX)                   uniform buffer  (morph influences)
                  binding 2  visibility 1 (VERTEX)  texture float 2d-array           (morph data)
```

Note the two **vertex-only** visibilities — every texture in rungs 1–4 was
fragment-only (`visibility 2`). This is also the first `2d-array` texture view.

Uniform members, decoded:

`renderStruct` (group 0, binding 0, `UpdateType::Render`/`Frame`):

| member | source | Three node |
|---|---|---|
| `cameraProjectionMatrix : mat4x4` | camera | `cameraProjectionMatrix` |
| `cameraViewMatrix : mat4x4` | camera | `cameraViewMatrix` |
| `nodeUniform9 : vec3` | `AmbientLight.color × intensity` in working space | `AmbientLightNode.colorNode` (`AnalyticLightNode`, `renderGroup`) |
| `nodeUniform13 : vec3` | `PointLight.color × intensity` | `AnalyticLightNode.colorNode` |
| `nodeUniform14 : f32` | `PointLight.distance` (0 here) | `PointLightNode` cutoff distance |
| `nodeUniform15 : f32` | `PointLight.decay` (2 here) | `PointLightNode` decay exponent |
| `nodeUniform12 : vec3` | point light position **in view space** | `lightViewPosition` / `Object3DNode` `VIEW_POSITION` |

Member order is allocation order, and it is not grouped by light: the ambient
colour (`9`) is allocated before the point light's (`13`) but the point light's
view position (`12`) lands last, after the scalars. That order comes from
fragment-stage traversal order inside `LightsNode`, not from a declaration list.

`objectStruct` (group 1, binding 0, `UpdateType::Object`):

| member | source |
|---|---|
| `nodeUniform0 : f32` | **morph base influence** (`morphReference`'s `base`; 1 here) |
| `nodeUniform3 : vec3` | `materialColor` (0xff0000 → linear `(1,0,0)`) |
| `nodeUniform4 : f32` | `materialOpacity` (1) |
| `nodeUniform5 : f32` | `materialShininess` (30) |
| `nodeUniform6 : vec3` | `materialSpecular` (0x111111 → linear) |
| `nodeUniform7 : vec3` | `materialEmissive` (black) |
| `nodeUniform8 : f32` | `materialEmissiveIntensity` (1) |
| `nodeUniform11 : mat4x4` | `modelWorldMatrix` |

There is **no `modelViewMatrix` uniform**: the vertex stage computes
`modelViewMatrix = render.cameraViewMatrix * object.nodeUniform11` itself. Also
no `modelNormalMatrix` — flat shading needs none.

Group 1 binding 1 is a `BufferNode`, not a struct member:

```wgsl
struct NodeBuffer_1015Struct { value : array< vec4<f32>, 2 > };
@binding( 1 ) @group( 1 ) var<uniform> NodeBuffer_1015 : NodeBuffer_1015Struct;
```

That is `uniformArray( mesh.morphTargetInfluences, 'float' )` —
`UniformArrayNode` pads each `float` to its own `vec4` and reads `.x`. Two
targets → `array<vec4<f32>,2>`, 32 bytes.

Group 1 binding 2 is the morph data texture: `texture_2d_array<f32>`,
sampled with a bare `textureLoad( tex, coord, layer, level )` — **no sampler
binding at all** (same non-filterable path rung 1's depth quad used, but with
the array-layer argument, which the port has never emitted).

### 1.2 `renderPipeline_outputColorTransform_19`

Identical in shape to what the port already emits at rung 2/4: one full-screen
triangle, `rgba8unorm` target, `multisample.count 1`, sampler+texture+uniform in
group 1, unpremultiply → sRGB OETF → premultiply. Diff the two `.wgsl` files
against the port's output-pass dump; no change expected.

### 1.3 The vertex program in words

```
positionLocal = position;
positionLocal = positionLocal * base;              // morphReference, base = 1
for ( i in 0..2 ) {                                 // Loop( morphTargetsCount )
    influence = 0.0;                                // float(0).toVar()
    influence = NodeBuffer.value[i].x;              // influences.element(i).toVar()
    if ( influence != 0.0 ) {                       // If( influence.notEqual(0) )
        texelIndex = int(vertexIndex) * 1 + 0;      // stride 1 (position only), offset 0
        y  = texelIndex / 4096;                     // width = 4096
        xy = ivec2( texelIndex - y*4096, y );
        texel = textureLoad( morphMap, xy, i, 0u );
        positionLocal = positionLocal + texel.xyz * influence;
    }
}
modelViewMatrix          = cameraViewMatrix * modelWorldMatrix;
v_positionView           = ( modelViewMatrix * vec4(positionLocal,1) ).xyz;
v_positionViewDirection  = -v_positionView;
clip                     = cameraProjectionMatrix * vec4( v_positionView, 1 );
```

Two varyings only: `@location(0) v_positionView`, `@location(1)
v_positionViewDirection`. The `VERTEX_nodeVar34` / `VERTEX_v_modelViewProjection`
pair is Three's `subBuild('VERTEX')` cosmetic temp, already noted as a
deliberate divergence in `docs/nodes.md §7`.

**The loop body never executes in the graded frame**: both influences are 0
(the GUI never fires). It must still compile, the texture and buffer must still
be bound and non-degenerate, and `base` must be 1. Getting the loop wrong is
invisible at t=0; getting the *bindings* wrong is a validation error or silent
black.

### 1.4 The fragment program in words

`MeshPhongNodeMaterial` + `PhongLightingModel` + `LightsNode` over
`[AmbientLightNode, PointLightNode]`:

```
DiffuseColor  = vec4( materialColor, 1 );  DiffuseColor.w *= opacity;  DiffuseColor.w = 1;
Shininess     = max( materialShininess, 1e-4 );
SpecularColor = materialSpecular;
EmissiveColor = materialEmissive * emissiveIntensity;

irradiance    = 0;  irradiance += ambientColor;          // AmbientLightNode.setup
directDiffuse = 0;
normalFlat          = normalize( cross( dpdx(v_positionView), -dpdy(v_positionView) ) );
normalViewGeometry  = normalFlat;  normalView = normalViewGeometry;   // flatShading

// PointLightNode.setup → getDistanceAttenuation
lVector   = lightViewPosition - v_positionView;
L         = normalize( lVector );
dotNL     = dot( normalView, L );
if ( cutoffDistance > 0 ) {                                // never taken: distance = 0
    d  = length(lVector);  s = d / cutoffDistance;
    t  = clamp( 1 - s*s*s*s, 0, 1 );
    att = ( 1 / max( pow(d, decay), 0.01 ) ) * t*t;
} else {
    att = 1 / max( pow( length(lVector), decay ), 0.01 );
}
lightColor    = pointColor * att;
irradianceDir = clamp(dotNL,0,1) * lightColor;
directDiffuse += irradianceDir * ( DiffuseColor.rgb * RECIPROCAL_PI );      // BRDF_Lambert

// BRDF_BlinnPhong
H        = normalize( L + positionViewDirection );
VdotH    = clamp( dot( positionViewDirection, H ), 0, 1 );
fresnel  = exp2( ( VdotH * -5.55473 - 6.98316 ) * VdotH );                  // F_Schlick
F        = SpecularColor*(1-fresnel) + 1.0*fresnel;
D        = ( ( Shininess*0.5 + 1 ) * RECIPROCAL_PI ) * pow( clamp(dot(normalView,H),0,1), Shininess );
directSpecular += irradianceDir * ( F * 0.25 * D ) * 1.0;                   // G = 0.25 implicit

indirectDiffuse  = 0;
indirectDiffuse += ( vec4(irradiance,1) * ( DiffuseColor * RECIPROCAL_PI ) ).xyz;   // BRDF_Lambert, vec4 form
ambientOcclusion = 1.0;   indirectDiffuse *= ambientOcclusion;
totalDiffuse   = directDiffuse + indirectDiffuse;
totalSpecular  = directSpecular + indirectSpecular(=0);
outgoingLight  = totalDiffuse + totalSpecular;
Output = max( vec4( outgoingLight + EmissiveColor, DiffuseColor.w ), vec4(0) );
```

`RECIPROCAL_PI` is printed as the literal `0.3183098861837907` — match that
digit string, it is `f64` `1/PI` printed by JS.

Two odd shapes to reproduce exactly, both from `LightingContextNode` /
`PhongLightingModel`:

* the indirect-diffuse line is done in **`vec4`** (`vec4(irradiance,1) * (DiffuseColor * 0.3183…)`)
  and then `.xyz`-ed, while the direct-diffuse line is `vec3`;
* the specular has a redundant `* vec3<f32>( 1.0 )` (the `specularStrength`
  slot) and `directSpecular`/`directDiffuse` are read-modify-write properties,
  not accumulated in temps.

---

## 2. The reference frame

`webgpu_morphtargets.jpg`, 400×250 (the grader's ×½ downscale of the 800×500
render).

Flat field of `#8FBCD4`-ish light blue — that is `scene.background`, converted
sRGB→linear on the CPU, used as the clear colour of the `rgba16float` MSAA
target, and converted back through the output pass's sRGB OETF. Dead centre, a
saturated red square roughly 70×70 px in the reference (≈140×140 at full size),
with hard edges and a slight radial falloff (brightest in the middle, darker toward the corners). That is the
`BoxGeometry(2,2,2,32,32,32)` seen exactly face-on: the camera is a
`PerspectiveCamera(45, 800/500 = 1.6, 1, 20)` at `(0,0,10)` with no rotation, so
only the +Z face is visible and it fills a square. Both morph influences are 0
(the GUI `onChange` never fires under the deterministic RAF), so **the cube is
still a cube** — no sphere, no twist.

The red is saturated because the `PointLight(0xffffff, 200)` is parented to the
camera, so it sits at world `(0,0,10)`, view-space `(0,0,0)`; the front face is
at view z = −9, giving attenuation `1/max(pow(9,2),0.01) = 1/81` and
`lightColor ≈ 2.47`, times `NdotL = 1`, times `albedo/π ≈ 0.318` ⇒ ≈ 0.79 in
linear before the ambient term, which the sRGB OETF pushes to ≈ 0.91. The
`AmbientLight(0x8FBCD4, 1.5)` adds a blue-ish `irradiance × diffuse/π`, which on
a pure-red albedo only lifts the red channel further — hence the red reads as
`#FF2222`-ish rather than pure `#FF0000`, and there is no blue spill on the
face. Specular contributes a little: `SpecularColor` is `MeshPhongMaterial`'s
default `0x111111`, shininess 30, and the half-vector is ≈ the normal at the
centre.

The faint gradient is real, not JPEG, and it is radial rather than vertical:
the face is flat but the light is a point at the camera, so `NdotL` and the
attenuation both fall off with the off-axis distance — and because `flatShading` derives the normal per-triangle,
the 32×32 tessellation of the face makes it a staircase of constant patches, not
a smooth ramp. Any smooth-shading mistake shows up as a different gradient, not
as a different colour.

Note there is **no visible silhouette detail to chase**: almost all of the
pixels are flat background, so a wrong clear colour fails instantly and a wrong
lighting term fails on ~5% of pixels. 5% is 50× the 0.1% budget, so there is no
hiding.

Timing: `Date.now`/`performance.now` = 0, RAF fires once, and **the example
never calls `Math.random`** — `createGeometry()` is pure arithmetic and
`BoxGeometry` is deterministic. The seeded-PRNG consumption order that bit
rungs 1–2 is a non-issue here.

---

## 3. Gap list

Against `port/docs/nodes.md` and `port/src/{nodes,renderer,materials}` at the
current `port` head (rung 4 + the merged side branches). Items marked
**[rung 5]** are expected to land from the parallel `webgpu_lights_phong` rung;
everything else is rung 6's own work.

### 3.1 Nodes

| missing | Three source | why |
|---|---|---|
| `Node::Loop` — a statement `for ( var i : i32 = 0; i < N; i++ )` with the loop var in scope | `src/nodes/utils/LoopNode.js` | `morphReference` is a `Loop( morphTargetsCount, … )`. The port's enum (`src/nodes/node.rs`) has no loop variant at all. |
| `Node::If` as a **statement** (with optional `else`) | `src/nodes/math/ConditionalNode.js` (`If`/`Else` in `TSLBase`) | needed twice: the `influence != 0` guard and the point light's `cutoffDistance > 0` branch. The port only has `Node::Select`, which lowers a *value* select into an if/else writing a result var — the morph guard has no result value and the light branch assigns an outer `toVar()`. **[rung 5 will need the light one]** |
| `.mul_assign` / `.add_assign` on a var (`positionLocal.mulAssign(base)`) | `src/nodes/core/Node.js` (`mulAssign`, `addAssign`) | trivial sugar over `Assign` + `Op`, but the emitted text is `x = ( x * y )`, matching the dump. |
| `.not_equal( 0 )` | `src/nodes/math/OperatorNode.js` | emits `( a != b )` as a bool. |
| `BufferNode` / `UniformArrayNode` as a *uniform* buffer with `array<vec4<f32>, N>` and `.element(i).x` | `src/nodes/accessors/UniformArrayNode.js`, `src/nodes/accessors/BufferNode.js` | the port has `Node::BufferElement` + `BufferSource` (used for the instance matrix at rung 2) — check whether that path emits the `NodeBuffer_NStruct { value : array<…> }` wrapper and a *dynamic* `i32` index; the instance case indexed by `instanceIndex`, this one indexes by a loop variable. |
| `textureLoad` with an **array layer and explicit level**: `textureLoad( t, ivec2, layer, 0u )` | `src/nodes/accessors/TextureNode.js` (`.depth(i)` sets `depthNode`) | `SampleMode::Load` in `src/nodes/node.rs` exists but emits the 2-D `textureDimensions`-clamped form with no layer. Needs a `Load { layer: Option<NodeRef>, level }` shape and `Type` support for `texture_2d_array<f32>`. |
| `TextureSource::DataArray` (a `DataArrayTexture`) | `src/textures/DataArrayTexture.js` | the port's `TextureSource` is `Texture2D | Depth | Cube`. |
| `morphReference` itself, called from `setupPosition` | `src/nodes/accessors/Morph.js` | the whole §1.3 body. `docs/nodes.md §6` already names `setup_position` as the hook, next to the existing `instanced_mesh()` call in `src/materials/node_material.rs`. |
| `vertexIndex` in an `i32` context (`int(vertexIndex).mul(stride).add(offset)`) | `src/nodes/core/IndexNode.js` | the port has `Builtin::VertexIndex` (u32); needs the `i32(...)` cast that the dump shows as `( ( i32( vertexIndex ) * 1 ) + 0 )`. |
| `normalFlat` = `normalize( cross( dpdx(positionView), -dpdy(positionView) ) )` | `src/nodes/accessors/Normal.js` (`normalFlat`), `src/nodes/display/ScreenNode.js`-adjacent `dFdx`/`dFdy` in `src/nodes/math/MathNode.js` | `dpdx`/`dpdy` are new WGSL builtins for the port, and they are why the fragment shader carries `diagnostic( off, derivative_uniformity )` (already emitted at rung 4). **[rung 5 may not need this — `webgpu_lights_phong` is smooth-shaded]** — treat `flatShading` as rung 6's own. |
| `Property` nodes `Shininess`, `SpecularColor`, `EmissiveColor`, `irradiance`, `directDiffuse`, `directSpecular`, `indirectDiffuse`, `indirectSpecular`, `totalDiffuse`, `totalSpecular`, `ambientOcclusion`, `normalFlat`, `normalViewGeometry`, `normalView`, `positionViewDirection` | `src/nodes/core/PropertyNode.js`, `src/nodes/accessors/Normal.js`, `src/nodes/accessors/Position.js` | the port has `Node::Property` but only with rung-1–4's names. **[rung 5]** for all but `normalFlat`. |
| `positionViewDirection` varying + `v_positionView` varying | `src/nodes/accessors/Position.js` | **[rung 5]** |

### 3.2 Lighting

| missing | Three source | note |
|---|---|---|
| `LightsNode` (collect lights from the scene, per-light `setup`, the `LightingContext`) | `src/nodes/lighting/LightsNode.js`, `LightingContextNode.js`, `LightingNode.js` | **[rung 5]** — but rung 5 has only a `PointLight`, so its `LightsNode` may hard-code a single direct light. Rung 6 needs **two lights of different classes in one material**, with the ambient one contributing to `irradiance` and not to the direct loop. |
| `AmbientLightNode` | `src/nodes/lighting/AmbientLightNode.js` | one line: `context.irradiance.addAssign( colorNode )`. Rung 6's own. |
| `AnalyticLightNode.update()`: `color.copy(light.color).multiplyScalar(light.intensity)`, `updateType = FRAME`, uniform in the **render** group | `src/nodes/lighting/AnalyticLightNode.js:299` | the port's `UpdateType` has `Render`/`Frame`/`Object`; needs `UniformSource::LightColor(idx)` etc. **[rung 5]** |
| `PointLightNode` (view position, cutoff distance, decay) + `getDistanceAttenuation` | `src/nodes/lighting/PointLightNode.js`, `src/nodes/lighting/LightUtils.js` | **[rung 5]** |
| `PhongLightingModel` (`direct`, `indirect`, `ambientOcclusion`, `finish`) | `src/nodes/functions/PhongLightingModel.js` | **[rung 5]** |
| `BRDF_Lambert`, `BRDF_BlinnPhong`, `F_Schlick`, `D_BlinnPhong`, `G_BlinnPhong_Implicit` | `src/nodes/functions/BSDF/BRDF_Lambert.js`, `src/nodes/functions/BSDF/F_Schlick.js`, `BRDF_BlinnPhong.js` | **[rung 5]** |
| `MeshPhongNodeMaterial` + `setup_lighting_model` returning a `LightingModel` | `src/materials/nodes/MeshPhongNodeMaterial.js`, `src/materials/MeshPhongMaterial.js` | **[rung 5]**. Rung 6 adds `flatShading` on top: `MeshPhongNodeMaterial.setupVariants` is unchanged, but `NodeMaterial.setupNormal` picks `normalFlat` when `material.flatShading === true` (`src/materials/nodes/NodeMaterial.js`). |
| `AONode` / `ambientOcclusion = 1.0` default | `src/nodes/lighting/AONode.js` (default path in `LightingContextNode`) | **[rung 5]** |
| Light **uniform member order** inside `renderStruct` | — | the dump's order (`9, 13, 14, 15, 12`) is the one to reproduce; a different order is only a byte-layout question and is pixel-neutral so long as the writer and the struct are generated from the same list (`docs/nodes.md §4`). |

### 3.3 Scene graph / renderer

| missing | Three source | note |
|---|---|---|
| **Nested objects.** `scene.add(camera)` and `camera.add(pointLight)` — the light's world matrix comes from the camera's. | `src/core/Object3D.js` `updateMatrixWorld` | `handoff/RUNGS.md` flags exactly this: *"Renderer still walks the flat `Child` list; folding `Child` into the tree is left for the first rung that nests."* **Rung 6 is that rung.** The `scene-graph` branch already merged `Node = Rc<RefCell<Object3D>>` + `Object3DNode` (see `port/docs/scene-graph.md`); the renderer's traversal is what has to change. |
| Light traversal: gathering lights while walking the graph, in tree order | `src/renderers/common/RenderList.js` (`pushLight`), `src/renderers/common/Renderer.js` `_renderObjects` | **[rung 5]**, but must survive a light that is *not* a direct child of the scene. |
| `Mesh.morphTargetInfluences` / `morphTargetDictionary` on the object, initialised from `geometry.morphAttributes` | `src/objects/Mesh.js` `updateMorphTargets()` | `scene-graph` already landed `BufferGeometry.morphAttributes`; the `Mesh` side is missing. |
| The morph **data-array texture build**: width `= position.count × stride` capped at 4096, height `= ceil(w/4096)`, `Float32Array(w*h*4*count)`, one layer per target, RGBA with `a = 0` | `src/nodes/accessors/Morph.js` `getEntry()` | For `BoxGeometry(2,2,2,32,32,32)`: `position.count = 6 × 33² = 6534`, stride 1 ⇒ width 4096, height 2, 2 layers, `rgba32float`. Note the buffer is **larger than the vertex count** — the tail is zeros. |
| `rgba32float` 2-D-array texture upload + a non-filterable, sampler-less bind | `src/renderers/webgpu/utils/WebGPUTextureUtils.js` | the port's texture path is `rgba8unorm(-srgb)` 2-D and cube only. |
| Vertex-stage texture/buffer **visibility** (`ShaderStages::VERTEX`) in the bind-group layout | `src/renderers/webgpu/utils/WebGPUBindingUtils.js` | `docs/nodes.md §3` says the port emits `7` for uniform buffers and `2` for textures; `2` here is wrong and will fail validation or read garbage. Visibility must be derived from which stages actually reference the binding. |
| `BoxGeometry` with 32 segments per axis | — | already in the merged `geometries` branch; just confirm the vertex order matches Three's (the morph texture is indexed by `vertexIndex`, so *any* vertex-order divergence corrupts morphing — invisible at t=0, but do not let that hide a real bug). |
| MSAA `count 4` on the scene pass | `src/renderers/common/Renderer.js` | the renderer already does this from rung 2 (`antialias: true` there too); confirm, don't rebuild. |
| Clear colour from `scene.background` as a `Color` | `src/renderers/common/Background.js` | rung 1 already had `scene.background` as a colour. Confirm the sRGB→linear conversion is on the clear value, not on the shader output. |

### 3.4 Nothing needed

No `TextureLoader`, no JPEG/PNG decode, no cube map, no render target beyond the
frame-buffer target the port already has, no tone mapping (`NoToneMapping`), no
fog, no transparency, no sorting (one opaque object), no `Math.random`.

---

## 4. Step order for the rung worker

This rung most resembles **rung 5** (`webgpu_lights_phong`) — it is the same
material family and the same lighting model — with rung 2's `InstanceNode`
as the template for "a node that expands inside `setup_position` and pulls a
per-object buffer + texture with it".

Take rung 5's merge first if it has landed. If it has not, build rung 6's own
half (steps 1–3) against a `MeshBasicNodeMaterial` first; the morph work is
entirely in the vertex stage and is independent of the lighting model.

1. **Clear colour only.** Port the example scene (`examples/webgpu_morphtargets.rs`)
   with the box drawn by the existing `MeshBasicNodeMaterial`. Green check: the
   background is `#8FBCD4` and a *black* square sits in the middle. This proves
   camera, aspect, `BoxGeometry(2,2,2,32,32,32)`, the nested
   `scene.add(camera)` / `camera.add(pointLight)` traversal, and the output pass.
   Diff will be ≈5% — that is the square, and it is the budget for the rest.

2. **Flat material, right size.** Give the box a basic red material. Check the
   square's **extent** against the reference to sub-pixel accuracy before doing
   any lighting: the projection, the aspect and the near/far are all verified by
   the silhouette, and every later step is colour-only. If the square is the
   wrong size, stop — nothing downstream will fix it.

3. **Scene-graph nesting.** Make the renderer walk the `Object3D` tree instead
   of the flat `Child` list and gather lights during the walk. Verify the point
   light's *world* position is `(0,0,10)` and its view position `(0,0,0)` with a
   unit test on `updateMatrixWorld`, not with the image — the image cannot tell
   `(0,0,10)` from `(0,0,9.9)`.

4. **Morph plumbing with the loop still dead.** Add `Mesh.morphTargetInfluences`,
   the `DataArrayTexture` build, the `rgba32float` 2-D-array upload, the
   `uniformArray` influences buffer, the vertex-visibility fix, and
   `morphReference()` in `setup_position`. At t=0 the image **must not change**
   from step 2. That is the check: the WGSL grew a loop and a texture binding
   and the pixels are identical. If it goes black, it is a validation error
   (wrong visibility, wrong `sampleType`, missing layer arg), not a shading bug.

5. **Prove the loop actually works** off the graded path: a second, ungraded
   run of the same scene with `morphTargetInfluences = [1, 0]` should give a
   sphere, and `[0, 1]` a twist. Nothing derived from that goes in the tree; it
   is the only way to know step 4 is real, because the graded frame never takes
   the branch.

6. **Ambient only.** `AmbientLightNode` + `irradiance` + the `vec4`-shaped
   indirect diffuse. The square goes from black to a dim desaturated red.

7. **Point light diffuse.** `PointLightNode` + `getDistanceAttenuation` +
   `BRDF_Lambert` + `normalFlat`. This is where almost all the red comes from.
   Expect to land within a few percent immediately; if it is far off, check the
   `1/max(pow(d,decay),0.01)` form and that `decay` is 2.

8. **Specular.** `BRDF_BlinnPhong` + `F_Schlick`. A small lift; get it in before
   judging the diff.

9. **Diff, then look at the gradient.** If the flat colour matches and only the
   gradient is wrong, the suspect is `normalFlat` (smooth vs flat) or the
   `dpdy` sign — the dump has `-dpdy`, and dropping the negation gives inverted
   facets that survive a flat-field check.

### Traps

* **The loop is dead at t=0.** Everything in §1.3 inside the `If` is untested by
  the grader. Step 5 is not optional.
* **`-dpdy`, not `dpdy`.** WGSL's y derivative has the opposite sign to the
  GLSL convention Three's WebGL path uses; the node emits the negation
  explicitly and it is load-bearing for the facet gradient.
* **Bind-group visibility.** Group 1 bindings 1 and 2 are `VERTEX` only
  (`visibility 1`). The port hard-codes `2` for textures (`docs/nodes.md §3`).
  wgpu will reject the mismatch — loudly, for once — but the fix must be
  general (derive from usage), because rung 10's skinning hits the same thing.
* **`base` influence.** `morphTargetsRelative` is false here, so
  `base = 1 - Σ influences = 1`. If it is left at 0 the box collapses to a
  point and the image is pure background — which looks like "the mesh did not
  draw", not like "morphing is wrong".
* **Morph texture width is 4096, not the vertex count.** 6534 vertices wrap to
  a 4096×2 texture. Hard-coding width = `position.count` produces a texture that
  is only correct for the first 4096 vertices — again invisible at t=0.
* **`vec4` padding in the influences buffer.** `uniformArray(…, 'float')` is
  `array<vec4<f32>, 2>` with the value in `.x`, not `array<f32, 2>`. 32 bytes,
  not 8.
* **No `normal` attribute.** Do not bind one. The pipeline has exactly one
  vertex buffer, `arrayStride 12`. Binding a second buffer whose location is
  unused is harmless in wgpu but means the port's attribute allocator is
  disagreeing with the shader it generated.
* **Colour spaces.** `0xff0000` and `0x8FBCD4` are sRGB, converted to the linear
  working space on the CPU (`ColorManagement`, already ported on `math2`). The
  scene pass is `rgba16float` linear; the output pass applies the OETF. Same as
  rung 2 — do not convert twice.
* **MSAA count 4 on the scene pass, 1 on the output pass.** The square's edges
  are ~4 px of the 400×250 image; getting MSAA wrong costs ~0.3% of pixels,
  three times the budget, on the edges alone.
* **`Math.random` is never called.** If the port's `range()` or anything else
  draws from the seeded PRNG on this rung's path, the seed state is irrelevant
  here but it means something is running that Three does not run.
* **Sort order / transparency.** One opaque mesh; no transparent list needed
  (`docs/nodes.md` lists "no transparent list" as a rung-4 gap — still fine).
* **`DiffuseColor.w = 1.0`.** Three emits it here (see the dump), unlike
  `webgpu_depth_texture`. The port emits it uniformly — matches.
* **Two worktrees, one GPU.** `handoff/RUNGS.md`: run e2e serially. Rung 5 is
  running in parallel; do not run both e2e suites at once.

---

## 5. Addons and external assets

The example imports two addons:

* `three/addons/inspector/Inspector.js` — `renderer.inspector = new Inspector()`
  and `renderer.inspector.createParameters('Morph Targets')` for the GUI. This
  is **pure UI**: `clean-page.js` hides `.three-inspector` and the e2e build
  injection neuters `trackTimestamp` precisely so the Inspector cannot crash in
  software mode. Nothing it does reaches the image. **Do not port it.** The
  ported scene simply omits `initGUI()`.
* `three/addons/controls/OrbitControls.js` — constructed with
  `enableZoom = false` and never interacted with; it does not move the camera in
  a single frame. **Do not port it.**

**No external assets at all**: no textures, no models, no fonts. The geometry is
`BoxGeometry` plus arithmetic, both morph targets are computed in the example's
own `createGeometry()`. Nothing in `examples/` outside the HTML is read. This is
the cheapest rung on the ladder from an asset point of view — the entire cost is
in the node system.

The only thing the HANDOFF rules make awkward is that the example's *purpose*
(the morph GUI) is invisible to the grader, so the rung's correctness rests on
step 5's ungraded sanity run rather than on the graded image.
