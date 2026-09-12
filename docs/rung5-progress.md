# rung 5 — `webgpu_lights_phong`, in progress

Status at handoff (session ended at its hard stop, ~40 min): **Three's WGSL is
dumped and read; no three-rs code written yet.** e2e is unchanged — rungs 1–4
still 0 / 45 / 0 / 1, no fifth test registered. Nothing in the tree is red.

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

### 1. Only ONE sphere program is created, and its colour is white

`DiffuseColor = vec4<f32>( vec3<f32>( 1.0, 1.0, 1.0 ), 1.0 );` — a baked
constant, and the module is created once even though the four sphere materials
have `colorNode = color( 0x0040ff / 0xffffff / 0x80ff80 / 0xffaa00 )`. So the
program cache key does not separate `ConstNode`s by value and **all four light
spheres draw white** in the graded frame. Verify against the reference
screenshot before reproducing it — but reproduce it if it holds, it is Three's
own behaviour at this commit, not a fudge. (Check `ConstNode`/`InputNode`
`getCacheKey`/`getHash` in the vendor tree for the mechanism.)

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

## What remains (suggested order)

1. **Lights.** `src/lights/{light.rs, point_light.rs}` (Object3D + colour,
   intensity, distance, decay, `power` getter/setter = `intensity * 4π`).
   The renderer must collect lights while walking the scene (they are
   `scene.add( light )` with the sphere mesh as a **child of the light** — the
   first rung that needs the renderer to walk nested children, which
   docs/scene-graph.md flags as deferred: fold `Child` into the tree here).
2. **`LightsNode`** in the node builder: per-light uniforms in the render group
   with the member order of §2, the `direct()` call per light, and the
   `lights([...])` selective form (a material-level override of the scene list).
3. **`MeshPhongNodeMaterial`** + `PhongLightingModel` (`direct`/`indirect` per
   §4), `shininess`/`specular`/`emissive` uniforms, `specularNode`,
   `normalNode`, and `lights = false` (§1's sphere program).
4. **Fog** (§5) on `Scene` — `fog()` / `rangeFogFactor()` TSL, applied in
   `setup_output` of every material.
5. **TSL additions:** `checker`, `mix` on colours (exists), `smoothstep`,
   `dpdx`/`dpdy`, `cross` (exists), `pow`, `exp2`, `clamp`, `length`,
   `normal_map()`, `position_view()`, `position_view_direction()`,
   `normal_view()`, `transformed_normal_view`.
6. `examples/webgpu_lights_phong.rs` + register in `tests/e2e/main.rs`
   (fifth entry, mirroring the HTML: camera 50° / 0.01 / 100, z = 7;
   SphereGeometry(0.1, 16, 8); TeapotGeometry(0.8, 18); teapots at x = -3/0/3,
   y = -1, `rotation.y = -π/2`; `antialias: true`).
7. Textures: `textures/water/Water_1_M_Normal.jpg` and
   `textures/roughness_map.jpg`, both `RepeatWrapping` — **repeat wrapping is
   new** (rung 4 noted ClampToEdge only).

## Traps noted

- `antialias: true` here; check what sample count rungs 1–4 used and that the
  internal rgba16float MSAA target path (rung 2's finding) is the same.
- The JPEG-decode residue from rung 4 (≤3/channel vs libjpeg-turbo) now applies
  to a *normal* map, where it perturbs shading rather than albedo. Watch it.
- Three's own run of this example scores `Diff 0.0%`, so the reference is a
  faithful target on this machine.
