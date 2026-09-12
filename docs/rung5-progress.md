# rung 5 — `webgpu_lights_phong`, done

**Status: passing, and merged onto the tree walk.** `webgpu_lights_phong` differs
in 4 of 100000 pixels (0.004%, limit 0.1%); rungs 1–4 are unchanged at
0 / 45 / 0 / 1. The generated WGSL is structurally identical to the dumps — same
bindings, same uniform layout, same expression order — modulo the cosmetic set
listed in `docs/nodes.md` §8.

Rung 5 landed first against the old flat `Scene.children: Vec<Child>` list and
was then merged onto `port`'s real scene-graph render path. The merge is
pixel-neutral and byte-neutral in the shaders: `cargo run --example dump_wgsl`
is identical before and after, because nothing in `src/nodes` or the material
flow moved — only where the renderer gets its objects and its lights from.

This note keeps the reading of Three's dumps (§§1–7 below) because the next
lighting rungs (6, 7, 8) build on it. Two of its warnings turned out to be
wrong; both are corrected in place.

### What rung 5 added

| area | what |
|---|---|
| lights | `src/lights/{light,point_light}.rs` — `Payload::Light( PointLight )` with `object.is_light` set, added with plain `scene.add( &light )`; the bulb sphere is an ordinary child and draws through the tree walk |
| node system | `Node::If`/`Select` lowering for `getDistanceAttenuation`, `LightsNode` (render-group per-light uniforms, selective `lights([…])` from `material.lights_node`), sub-build layers (`docs/nodes.md` §7) |
| materials | `MaterialKind::Phong` on the one `NodeMaterial` struct, `src/materials/phong.rs` (`BRDF_Lambert`, `F_Schlick`, `D_BlinnPhong`, `BRDF_BlinnPhong`, `PhongLightingModel`), `specularNode`, `normalNode`, `lights = false` |
| TSL | `floor`, `sign`, `exp2`, `length`, `smoothstep`, `dpdx`, `dpdy`, `tsl_mod_float`, `checker()`, `fog()` / `range_fog_factor()`, `normal_map()`, `tangent_view()` / `bitangent_view()` / `tbn_view_matrix()` |
| textures | `Wrapping::Repeat` → `wgpu::AddressMode::Repeat`, `Texture::set_wrapping()` |
| geometry | `teapot_geometry` re-exported from the crate root |
| example | `examples/webgpu_lights_phong.rs` + the fifth `tests/e2e/main.rs` entry |

### The two bugs the pixels found

Both were invisible to the WGSL diff, because neither is in the shader text.

1. **`object.rotation.y = …` needs `set_rotation()`.** The first e2e run was
   2.41% different: every teapot drew side-on with its spout and handle in
   silhouette. Writing `Object3D.rotation` directly leaves `quaternion`
   untouched, and the world matrix is composed from the quaternion — three.js
   syncs them in `Euler.onChange`. `set_rotation( x, y, z )` is that pair.
   Fixing it took the diff to 0.175%.

2. **`MeshPhongMaterial.specular` default was not colour-space converted.**
   The remaining 0.175% was 168 pixels in one place: the specular hotspot of
   the *centre* teapot, the only one that does not override `specularNode`.
   `Default for MeshBasicNodeMaterial` hard-coded `Color::new( 0x11 / 255, … )`
   = 0.0667, but `new THREE.Color( 0x111111 )` is `setHex( hex, SRGBColorSpace
   )`, i.e. the linear 0.0056 — **11.9× dimmer**. `Color::from_hex( 0x111111 )`
   took the diff to 0.004%.

   The shape of this failure is worth keeping: a specular term is `pow( dotNH,
   80 )`, so an error confined to the specular colour shows up only where the
   highlight is, as a small very-bright cluster, and everything else in the
   frame stays under the comparator threshold. Per-region mean distance was
   left 0.63 / centre 2.76 / right 0.37 — the region that is wrong is the
   region whose mean moves, and the two teapots with a `specularNode` were
   the control group that ruled out geometry, normals, UVs and the lighting
   model in one step.

### Ruled out along the way

* **JPEG decode.** zune-jpeg vs libjpeg-turbo on `Water_1_M_Normal.jpg`: 3316
  of 786432 channels differ, max 3, mean 0.005. Far too small to move a
  highlight; the rung-4 note about the decode residue perturbing a *normal* map
  is real but immaterial here.
* **Mipmaps.** `src/renderer/shaders/mipmap.wgsl` is byte-for-byte the dumped
  `00_mipmap.wgsl` shader, and the dump confirms Three takes the same
  `2d-array` fallback path for a plain 2D texture.
* **Colour space of the maps.** Neither texture sets `colorSpace`, so both stay
  `NoColorSpace`; `TextureLoader` already matches.

### Still open

* The cosmetic WGSL deltas in `docs/nodes.md` §8 (temp hoisting, var/varying/
  uniform numbering, the `VERTEX_` sub-build prefix, uniform member order).
  Two of Three's choices there are still unexplained: why `dot( normalView,
  lightDirection )` is given a var when its usage count looks like 1, and why
  `bitangentViewFrame = ( a * b )` skips the `vec3<f32>()` widening that
  `tangentViewFrame` gets.
* Only `PointLight`. `AmbientLight` / `DirectionalLight` / `SpotLight` and
  shadows belong to rungs 6–8.

## Merging onto the tree walk (2026-09-13)

`git merge port` (cdf834a) into rung5 (0e197ec): two conflicted files,
`src/objects/scene.rs` and `src/renderer/mod.rs`. What the resolution did:

* **Deleted** `Scene.lights`, `Scene::add_light()`, `Scene::drawables()` and
  rung 5's `Scene::update_matrix_world` light-children loop, plus the
  `render_list_order()` / `apply_matrix4_vector4()` pair in `src/renderer/mod.rs`
  that the real `RenderList` replaces. `Scene` keeps only the one field rung 5
  added, `fog_node`.
* **Lights became a payload.** `PointLight::new()` returns a `Node` with
  `Payload::Light( PointLight )` and `object.is_light = true`; `Light` and
  `PointLight` no longer own an `Object3D`, and `PointLight.children` /
  `PointLight::add()` are gone — a bulb is `light.add( &mesh )` through
  `Object3DNode`. `Object3D::light()` / `light_mut()` reach the payload.
  `PointLight::world_position( &matrix_world )` is an associated function now,
  because the matrix lives on the node.
* **`LightState` maps from `render_list.lights`**, not `scene.lights` — that is
  `lightsArray`, i.e. scene-traversal order, which is what
  `LightsNode.setLights()` sees and what `UniformSource::Light*( i )` indexes.
  The example adds its four lights before the three teapots, exactly as
  `webgpu_lights_phong.html` does, so traversal order is add order and the
  uniform triples land in Three's slots. `SetupContext.light_count` is
  `render_list.lights.len()`.
* **`Renderable`'s rung-5 fields** (`fog` from `scene.fog_node`, and
  `light_count` inside `SetupContext`) were reapplied at the rewritten
  construction site, which now reads `item.matrix_world` and
  `item.node.borrow().mesh()`.

A second-order effect worth knowing: the bulb spheres are now reached *through*
the lights during traversal, so they enter the opaque list before the teapots
rather than after them, as `drawables()` had it. The pixels do not move because
`painterSortStable` sorts on `z` and tie-breaks on `Object3D.id`, and the ids are
still allocated in the same order (each `add_light` builds its mesh, then its
light).

`dump_wgsl` needed no change at all: it drives `setup()` + `NodeBuilder` on
materials directly and never builds a scene.

## Where the dumps are and how to get them again

`rung5/target/dumps/webgpu_lights_phong/` (untracked, as rung 4's were):
17 files, `NN_<label>.wgsl`, in `createShaderModule` call order, plus the
`createRenderPipeline` descriptors as `NN__pipeline_*.wgsl` (JSON).

Recipe (the temporary vendor hook — **removed again**; `git diff` in
`~/src/vendor/three.js` is back to the two-line grader-flags patch alone):

1. In `test/e2e/puppeteer.js`, `preparePage()`, after
   `await page.evaluateOnNewDocument( injection );` add a second
   `evaluateOnNewDocument` that monkey-patches `GPUDevice.prototype.createShaderModule`
   (push `{ label, code }` onto `window.__dumps`) and
   `createRenderPipeline` (push the descriptor, JSON-stringified with
   `module`/`layout` replaced by `'[obj]'`).
2. In `checkFile()`, right after the `const screenshot = ...` line, when
   `process.env.DUMP_DIR` is set, `page.evaluate( () => window.__dumps )` and
   write one file per entry.
3. `DUMP_DIR=<...>/rung5/target/dumps node test/e2e/puppeteer.js webgpu_lights_phong`
   — takes ~30 s and also confirms the example passes for Three itself
   (`Diff 0.0%`).

Restore `puppeteer.js` from a copy afterwards; only the grader-flags patch may
stay.

## What the dumps say

Five programs (vertex+fragment pairs) plus the mipmap module:

| pair | program |
|---|---|
| 02/03 | `leftObject` — Phong, `lightsNode = lights([light1])` (one light), `specularNode = texture( alphaTexture )` |
| 05/06 | `centerObject` — Phong, **all four** scene lights, `normalNode = normalMap( texture( normalMapTexture ) )`, `shininess = 80` |
| 08/09 | `rightObject` — Phong, `lights([light2])`, `specularNode = mix( color, color, checker( uv*5 ) )`, `shininess = 90` |
| 11/12 | the light spheres — `lights = false`, so no lighting flow at all |
| 14/15 | `outputColorTransform` — unchanged from rung 4 |
| 00 | `mipmap` — unchanged (raw WGSL in Three too) |

### 1. Only ONE sphere program is created — ~~and its colour is white~~

**Corrected.** The dump has one sphere module,
`DiffuseColor = vec4<f32>( vec3<f32>( 1.0, 1.0, 1.0 ), 1.0 );`, which reads
like a program-cache collision that would make all four light spheres draw
white. It is not. At `lightTime = 0` only light2 — the white one — is inside
the frustum: light1 projects to ndc y ≈ 2.14 and lights 3 and 4 (which
coincide) to ndc y ≈ 1.225, all off-screen even allowing for the sphere's
~0.03–0.05 ndc radius. Three creates one sphere program because it draws one
sphere. Per-sphere colours stay correct, and the reference screenshot shows the
single white dot above the right-hand teapot.

### 2. Light uniforms live in the **render** group (group 0)

Per point light, in light order, three members: `color * intensity` (`vec3`),
`cutoffDistance` (`f32`), `decay` (`f32`). Then every light's **view-space
position** (`vec3`) is appended after all of those, in light order. The
centre teapot's render struct:

```
cameraProjectionMatrix, cameraViewMatrix,
u12:vec3 u13:f32 u14:f32   // light1 colour, distance, decay
u16:vec3 u17:f32 u18:f32   // light2
u20:vec3 u21:f32 u22:f32   // light3
u24:vec3 u25:f32 u26:f32   // light4
u11:vec3 u15:vec3 u19:vec3 u23:vec3   // the four view positions, appended last
```

The selective-lights teapots have the same shape with one light
(`u11 colour, u12 distance, u13 decay, u9 position`).

`light.power = 1700` → `intensity = power / (4π)`; `distance = 100`; decay
defaults to 2. The uniform is `color.multiplyScalar( intensity )` in linear
space, and the position is the light's world position transformed by the camera
view matrix.

### 3. `Date.now`/`performance.now` are 0, so the light positions are t = 0

`lightTime = 0` ⇒ light1 (0, 4, 3), light2 (3, 0, 0), light3 (0, 4, 0),
light4 (0, 4, 0). Lights 3 and 4 coincide.

### 4. The per-light direct flow (`PhongLightingModel`)

```wgsl
nodeVar1 = ( lightViewPosition - v_positionView );   // lVector
nodeVar2 = normalize( nodeVar1 );                    // L
nodeVar3 = dot( normalView, nodeVar2 );
if ( cutoffDistance > 0.0 ) {
    d  = length( nodeVar1 );
    t  = d / cutoffDistance;
    f  = clamp( 1.0 - t*t*t*t, 0.0, 1.0 );
    att = ( 1.0 / max( pow( d, decay ), 0.01 ) ) * ( f * f );
} else {
    att = ( 1.0 / max( pow( length( nodeVar1 ), decay ), 0.01 ) );
}
lightColor  = lightColorIntensity * vec3<f32>( att );
irradiance  = vec3<f32>( clamp( dot( normalView, L ), 0.0, 1.0 ) ) * lightColor;
// direct diffuse: BRDF_Lambert
directDiffuse += irradiance * ( DiffuseColor.xyz * vec3<f32>( 0.3183098861837907 ) );
// direct specular: BRDF_BlinnPhong
H  = normalize( L + positionViewDirection );
VdotH = clamp( dot( positionViewDirection, H ), 0.0, 1.0 );
fr = exp2( ( ( VdotH * -5.55473 ) - 6.98316 ) * VdotH );          // F_Schlick
spec = ( ( SpecularColor * vec3<f32>( 1.0 - fr ) + vec3<f32>( 1.0 * fr ) )
        * vec3<f32>( 0.25 )
        * vec3<f32>( ( ( Shininess * 0.5 + 1.0 ) * 0.3183098861837907 )
                     * pow( clamp( dot( normalView, H ), 0.0, 1.0 ), Shininess ) ) );
directSpecular += ( irradiance * spec ) * vec3<f32>( 1.0 );
```

and the tail, identical in every lit material:

```wgsl
indirectDiffuse = vec3(0); irradiance = vec3(0);
indirectDiffuse = ( vec4( indirectDiffuse, 1.0 ) + vec4( irradiance, 1.0 ) * ( DiffuseColor * vec4( 0.3183098861837907 ) ) ).xyz;
ambientOcclusion = 1.0;
indirectDiffuse = indirectDiffuse * vec3( ambientOcclusion );
totalDiffuse = directDiffuse + indirectDiffuse;
indirectSpecular = vec3(0);
totalSpecular = directSpecular + indirectSpecular;
outgoingLight = totalDiffuse + totalSpecular;
Output = max( vec4( outgoingLight + EmissiveColor, DiffuseColor.w ), vec4( 0.0 ) );
```

Material prologue, every Phong material (order matters):

```wgsl
DiffuseColor = vec4<f32>( materialColor, 1.0 );
DiffuseColor.w = DiffuseColor.w * materialOpacity;
DiffuseColor.w = 1.0;                       // isOpaque
Shininess = max( materialShininess, 0.0001 );
SpecularColor = <specularNode or materialSpecular>;
EmissiveColor = materialEmissive * vec3<f32>( materialEmissiveIntensity );
```

### 5. Fog — new at this rung, and it is on every material

`scene.fogNode = fog( color( 0xFF00FF ), rangeFogFactor( 12, 30 ) )` becomes,
after `Output`:

```wgsl
nodeVarN = vec4<f32>( mix( Output.xyz, vec3<f32>( 1.0, 0.0, 1.0 ),
                           smoothstep( 12.0, 30.0, ( - v_positionView.z ) ) ), Output.w );
Output = nodeVarN;
```

i.e. the magenta is a baked const (sRGB 0xFF00FF is exactly (1,0,1) in linear),
`rangeFogFactor( near, far )` is `smoothstep( near, far, -positionView.z )`, and
`fog()` mixes over `Output.xyz` only, leaving `Output.w`.

### 6. New varyings / vertex stage

```wgsl
modelViewMatrix = render.cameraViewMatrix * modelWorldMatrix;
varyings.v_positionView = ( modelViewMatrix * vec4( positionLocal, 1.0 ) ).xyz;
varyings.v_normalViewGeometry = normalize( ( cameraViewMatrix * vec4( modelNormalMatrix * normalLocal, 0.0 ) ).xyz );
varyings.nodeVarying6 = uv;
varyings.v_positionViewDirection = - varyings.v_positionView;
```

Note `positionViewDirection` is a **varying** (`-positionView`, interpolated,
then `normalize()`d in the fragment), not recomputed in the fragment. Varying
locations differ between the left teapot (0 positionView, 1 positionViewDirection,
2 normalViewGeometry, 3 uv) and the centre one (0, 1 normalViewGeometry,
2 positionViewDirection, 3 uv) — first-use order, as §3 of docs/nodes.md says.

### 7. `normalMap` (centre teapot) — derivative TBN, no tangent attribute

```wgsl
normalViewGeometry = normalize( v_normalViewGeometry );
NORMAL_normalView  = normalViewGeometry;
q0 = cross( - dpdy( v_positionView ), NORMAL_normalView );   // note the minus
q1 = cross( NORMAL_normalView, dpdx( v_positionView ) );
st0 = dpdx( uv ); st1 = - dpdy( uv );
... (Three's TBN-from-derivatives; the intermediate nodeVar4..nodeVar7 lines are
in the dump at 06_fragment.wgsl lines 194–216)
tangentViewFrame   = nodeVar4 * vec3<f32>( nodeVar5 );
bitangentViewFrame = nodeVar6 * nodeVar5;
NORMAL_TBNViewMatrix = mat3x3<f32>( tangentView, bitangentView, normalView );
normalView = normalize( NORMAL_TBNViewMatrix * ( texel * vec4(2.0) - vec4(1.0) ).xyz );
```

`normalMap()` with no explicit scale: no `mix`/scale term appears.

## Traps that turned out to matter

- `antialias: true` here (4× MSAA), same internal path as rungs 1–4 — no change
  needed.
- The JPEG-decode residue from rung 4 now lands on a *normal* map. Measured on
  `Water_1_M_Normal.jpg` it is 3 / 255 worst case and does not move the
  highlight; see "Ruled out along the way" above.
- Three's own run of this example scores `Diff 0.0%`, so the reference is a
  faithful target on this machine — which is why a 2.4% diff was worth reading
  as a bug rather than as tolerance.
- The vendor tree moved from `3d010ef` to r186 (`148ef33`) during this rung.
  `git diff 3d010ef..r186` touches `IndexNode`, `PassNode`, `BRDF_Sheen`, the
  compute/subgroup/workgroup nodes, `EnvironmentNode`, `Packed4x8IntegerNode`,
  `LoopNode`, `RTTNode`, `WebGPUBackend` and `WGSLNodeBuilder` (scoped-array
  atomics, array/3D texture component prefix) — nothing this example uses, and
  `examples/screenshots/webgpu_lights_phong.jpg` is unchanged. The dumps taken
  at `3d010ef` are still valid.
