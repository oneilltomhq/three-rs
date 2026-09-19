# Scout plan — `webgpu_materials`

Scouted 2026-09-19. Vendor tree `~/src/vendor/three.js` @ r186 (148ef33) with
`rung0/grader-flags.patch` applied. Port read at
`/home/tom/src/projects/three-rs/main` (tip after all 2026-09-13 merges: rungs
1–9 + 13 green, `lines` merged).

**Verdict up front: one rung, but a large one (~1300 lines of Rust), and it is
not a "material zoo".** The brief's premise — Basic/Lambert/Phong/Standard/
Physical/Normal/Toon/Matcap, wireframe, flat shading, vertex colours, env maps,
side — describes the *WebGL* `webgl_materials.html`. The WebGPU example of that
name is a different page: **16 of its 17 materials are `MeshBasicNodeMaterial`
with a `colorNode`**, and the 17th is `MeshNormalMaterial`. There is no PBR, no
light, no env map, no wireframe, no `side`, no flat shading. It is a **TSL
breadth** example. So "split by material family" has nothing to split on; the
only meaningful split axis is *node-builder feature*, and §8 works that out.

---

## 1. Grade confirmation

| run | command | result |
|---|---|---|
| 1 | `npm run test-e2e-webgpu -- webgpu_materials` | `Diff 0.0% in file: webgpu_materials (3.2s)` PASS |
| 2 | same | `Diff 0.0% in file: webgpu_materials (3.2s)` PASS |

Logs: `grade-run1.log`, `grade-run2.log` beside this file. Both runs serial
under `flock /run/user/1000/three-rs-gpu.lock`.

`webgpu_materials` is **not** on `exceptionList` in
`~/src/vendor/three.js/test/e2e/puppeteer.js` (checked line 8–82; the nearby
entries `webgpu_materials_texture_html`, `webgpu_materials_matcap`,
`webgpu_materials_video` are, this one is not). This matches rung 0's
2026-09-12 table (`rung0/RUNG0.md`: `webgpu_materials | 0.0% | pass`) and the
2026-09-13 r186 re-pin.

Threshold is the stock one: `pixelThreshold 0.1`, `maxDifferentPixels 0.1` %
of the scaled image, i.e. ~100 px of 100000.

Three siblings graded once each at the same time, for §8's split discussion:

| example | diff | note |
|---|---|---|
| `webgpu_tsl_interoperability` | 0.0% | the only other `wgslFn` example |
| `webgpu_instance_uniform` | 0.0% | `GridHelper` + `TeapotGeometry` |
| `webgpu_sandbox` | **0.1%** | on the line — rung 0's policy drops these |

---

## 2. What the example does

Source: `~/src/vendor/three.js/examples/webgpu_materials.html`, ~300 lines of
scene code. Reference: `reference.jpg` beside this file (copied from
`examples/screenshots/webgpu_materials.jpg`).

### Scene

- `PerspectiveCamera( 45, 800/500, 1, 2000 )`, `position (0, 200, 800)`.
  `animate()` then overwrites it: `timer = 0.0001 * Date.now()`; the grader
  freezes `Date.now()` to `0`, so `position.x = cos(0)*1000 = 1000`,
  `position.z = sin(0)*1000 = 0`, `y` stays 200, then `lookAt( 0,0,0 )`.
- `scene.background = new THREE.Color( 0x000000 )` — this is
  `Background.update()`'s `isColor` branch: it becomes `clearColorValue`
  `{0,0,0,1}` with `forceClear`, **not** a skybox mesh. The dump confirms: no
  background draw, `clearValue: {r:0,g:0,b:0,a:1}`. The port's
  `Background::Color` already takes that branch.
- `GridHelper( 1000, 40, 0x303030, 0x303030 )` at `y = -75`. `GridHelper`
  extends `LineSegments`; 41 lines each way = **82 segments / 164 vertices**,
  `position` + `color` `Float32BufferAttribute`s, all four colours equal
  (`0x303030` twice), material `LineBasicMaterial({ vertexColors: true,
  toneMapped: false })`.
- Geometry for every mesh: **one shared** `TeapotGeometry( 50, 18 )` from
  `examples/jsm/geometries/TeapotGeometry.js` (689 lines; 32 bicubic Bézier
  patches). Dump: index buffer 247104 B = **61776 indices**, position buffer
  138624 B. Deterministic, no RNG.
- `renderer = new WebGPURenderer( { antialias: true } )` → **sampleCount 4**,
  `rgba16float` MSAA colour + resolve, `depth24plus` MSAA depth.
- `renderer.inspector = new Inspector()`. Checked
  `examples/jsm/inspector/Inspector.js` (684 lines) and
  `src/renderers/common/Renderer.js` `set inspector`: it installs a debug UI
  and sets `backend.trackTimestamp = true`; `clean-page.js` hides it by CSS
  (`.three-inspector`). `InspectorNode` only alters a graph for nodes
  explicitly wrapped in `inspector()`, which this page never does. **No effect
  on pixels.** Unlike `webgpu_tsl_galaxy`, this page never calls
  `createParameters()`/`gui.add`, so the Inspector draws **no** `Math.random`
  — `INSPECTOR_RANDOM_DRAWS` is 0 here.
- No `window.TESTING` branch, no loader beyond `TextureLoader`, no
  post-processing, no compute, no addons besides `TeapotGeometry`,
  `capabilities/WebGPU.js` and the Inspector.

### `Math.random` draws

`addMesh()` draws three (`rotation.x/y/z = random()*200 - 100`), once per
mesh, **17 meshes = 51 consecutive draws**, in material-creation order. Nothing
else on the page draws. The grader's generator is
`seed = Math.PI/4; x = sin(seed++) * 10000; return x - floor(x)`.

All 51 values, plus the resulting transforms, are dumped in
`scene_t0.json` (§5).

### `animate()` before the single graded frame

`requestAnimationFrame` fires exactly once, so `animate()` runs once *before*
the render: every mesh gets `rotation.x += 0.01; rotation.y += 0.005` on top of
its random Euler. Rung 5 lost 2.41% to exactly this class of bug (writing
`rotation` without syncing the quaternion), so `scene_t0.json` pins the
post-step quaternions and `matrixWorld`s.

### Textures

| file | size | format in the dump | wrap | mips |
|---|---|---|---|---|
| `textures/uv_grid_opengl.jpg` | 1024×1024 | `rgba8unorm` (**not** `-srgb`) | `RepeatWrapping` S+T | 11 levels |
| `textures/alphaMap.jpg` | 512×512 | `rgba8unorm` | `RepeatWrapping` S+T | 10 levels |

Neither gets a colour space (the page never sets `.colorSpace`, and
`material.colorNode = texture(...)` bypasses `map` handling), so **no sRGB
decode, no colour-space node** — simpler than rung 3. Both go through Three's
2d-array mipmap blit: 19 `mipmapEncoder` passes in the dump.

### The 17 materials, in creation order

Three object ids 18…34; `LineBasicMaterial` is id 17 (grid, created first).

| id | what the page sets | TSL used |
|---|---|---|
| 18 | `colorNode = positionLocal` | `positionLocal` |
| 19 | `colorNode = positionWorld` | `positionWorld` |
| 20 | `colorNode = normalLocal` | `normalLocal` |
| 21 | `colorNode = normalWorld` | `normalWorld` |
| 22 | `colorNode = normalView` | `normalView` |
| 23 | `colorNode = texture( uvTexture )` | `texture` (with the texture matrix) |
| 24 | `colorNode = color(0x0099FF)`, `opacityNode = texture(uvTexture)`, `transparent = true` | `color`, **`opacityNode`**, blending |
| 25 | `colorNode = texture(uvTexture)`, `opacityNode = texture(opacityTexture)`, `alphaTestNode = 0.5` | **`alphaTestNode`** → `discard` |
| 26 | `colorNode = cameraProjectionMatrix.mul( positionLocal )` | `cameraProjectionMatrix` |
| 27 | `MeshNormalMaterial`, `opacity = .5`, `transparent = true` | **`MeshNormalNodeMaterial`** |
| 28 | `colorNode = desaturateShaderNode({ color: texture(uvTexture) })` | **`Fn` with a named-object input** |
| 29 | `colorNode = desaturateNoInputsShaderNode()` | **`Fn` with no inputs** |
| 30 | `colorNode = someWGSLFn({ color: texture(uvTexture) })` | **`wgslFn` with an `includes` list** |
| 31 | `colorNode = getWGSLTextureSample({ tex: textureNode, tex_sampler: textureNode, uv: uv() })` | **`wgslFn` taking a texture + its sampler** |
| 32 | `colorNode = triplanarTexture( texture(uvTexture), null, null, float(.01) )` | **`triplanarTexture`** |
| 33 | `colorNode = texture( uvTexture, screenUV.flipY() )` | **`screenUV`**, **`.flipY()`** |
| 34 | `colorNode = Loop( 10, ({i}) => { … } )` | **`Loop`**, `.toVar()`, `.assign()`, `oscSine` |

Meshes are laid out `x = (i%4)*200 - 400`, `z = floor(i/4)*200 - 200`.

---

## 3. Dump reading

Produced with today's tool:

```sh
cd /home/tom/src/projects/three-rs/tools-dump && \
flock /run/user/1000/three-rs-gpu.lock node tools/dump-webgpu.mjs webgpu_materials \
    --out /home/tom/src/projects/three-rs/scouts/scouts/webgpu_materials/dump
```

It printed one benign page error, `Failed to load resource: 404`, and still
produced a correct frame (`dump/actual_full.png` matches the reference; the
grader scores the same page 0.0% twice). Both textures exist on disk; the
missing resource is not used by the render.

### Totals

| | count |
|---|---|
| shader modules | **35** (`m00`…`m34`) |
| render pipelines | **19** = 17 scene + 1 `mipmap-rgba8unorm-2d-array` + 1 `outputColorTransform` |
| compute pipelines | 0 |
| passes | **21** = 1 scene + 19 `mipmapEncoder` + 1 output |
| draws in the scene pass | **18** (17 meshes + the grid) |
| bind-group layouts | **3** |
| pipeline layouts | 18 |
| bind groups | 53 |
| buffers | 32 |
| textures | 6 |
| samplers | 4 |

**17 scene pipelines for 18 draws**: materials **28 and 29 share pipeline 67**
(labelled `renderPipeline_MeshBasicNodeMaterial_29`). The `Fn`-with-named-input
and the `Fn`-with-no-inputs desaturate filters generate **byte-identical
WGSL**, because `Fn` without a layout is inlined. Likewise Three's shader-module
cache emits only a *fragment* module for materials 23 and 30 (`m17`, `m28`),
reusing an earlier identical vertex module. (Rung 8's 2026-09-12 note said "18
render pipelines"; on r186 it is 17 + 2. Use today's number.)

### The three bind-group layouts

| BGL | entries | used by |
|---|---|---|
| 9 | `@binding(0)` uniform buffer, VERTEX+FRAGMENT+COMPUTE | group 0 (`bindGroup_render`) everywhere; also group 1 for the six materials with no texture |
| 50 | 0 sampler (F), 1 `texture_2d<f32>` (F), 2 uniform buffer (VFC) | every single-texture material + the output pass |
| 97 | 0 sampler, 1 texture, 2 uniform, 3 sampler, 4 texture | **only material 25** (colour map + opacity map) |

Group 0 (`render`) is one of three buffers depending on what the graph needs:
128 B (`cameraProjectionMatrix` + `cameraViewMatrix`), 144 B (+ a `vec2`
viewport size, or + an `f32` time). Group 1 (`object`) is 80/112/128/160 B.
Uniform *ordering inside the struct is declaration order of the node
uniforms*, e.g. `m23`'s `objectStruct { nodeUniform2 : f32, nodeUniform5 :
mat4x4<f32> }` — a scalar first, then the model matrix.

### Draw order in the single scene pass

Three's `RenderList` sort: opaque front-to-back, then transparent back-to-front.
Recorded order (material ids):

```
33, 29, 25, 21, 20, 32, 28(shares pipeline 67), 17=grid, 19, 23, 31, 18, 34,
26, 22, 30,   |   27, 24
```

Note the **grid sits in the middle of the opaque list**, and the two
transparent materials (27 `MeshNormalMaterial` and 24 the opacity teapot) are
last — the only two pipelines with a `blend` state
(`src-alpha / one-minus-src-alpha`, alpha `one / one-minus-src-alpha`). Every
pipeline is `triangle-list`/`ccw`/`cull:back`/`depth24plus`/`less-equal`/
`depthWrite:true`/`sampleCount:4` except the grid (`line-list`) and the output
pass (`sampleCount:1`, `rgba8unorm`).

### Module → material map (`.wgsl` file names)

| files | material |
|---|---|
| `m00_vertex_fragment_mipmap.wgsl` | Three's own raw-WGSL mipmap blit |
| `m01`/`m02` | 33 `screenUV.flipY()` |
| `m03`/`m04` | 29 (and 28) desaturate `Fn` |
| `m05`/`m06` | 25 alphaTest (the only `discard`) |
| `m07`/`m08` | 21 `normalWorld` |
| `m09`/`m10` | 20 `normalLocal` |
| `m11`/`m12` | 32 `triplanarTexture` |
| `m13`/`m14` | 17 `LineBasicNodeMaterial` (grid, vertex colours) |
| `m15`/`m16` | 19 `positionWorld` |
| `m17` (frag only) | 23 `texture` |
| `m18`/`m19` | 31 `wgslFn` texture+sampler |
| `m20`/`m21` | 18 `positionLocal` |
| `m22`/`m23` | 34 `Loop` |
| `m24`/`m25` | 26 `cameraProjectionMatrix` |
| `m26`/`m27` | 22 `normalView` |
| `m28` (frag only) | 30 `wgslFn` with an include |
| `m29`/`m30` | 27 `MeshNormalMaterial` |
| `m31`/`m32` | 24 opacity/transparent |
| `m33`/`m34` | `outputColorTransform` |

### The surprises

**(a) Three's `Loop` result is dropped — `m23` is a compiled bug, and the port
must reproduce it.** `material.colorNode = Loop( 10, … )`. `LoopNode.toStack()`
appends the `for` as a *statement*; its `generate()` returns an empty snippet,
so `setupDiffuseColor`'s `diffuseColor.assign( colorNode )` compiles to:

```wgsl
for ( var i : i32 = 0; i < 10; i ++ ) { … nodeVar0 = ( nodeVar0 + nodeVar2 ); … }

DiffuseColor = vec4<f32>(  );          // <-- empty constructor: the loop's value
DiffuseColor.w = ( DiffuseColor.w * object.nodeUniform2 );
DiffuseColor.w = 1.0;
```

`vec4<f32>()` is WGSL's zero value, so **that teapot renders opaque black** and
only shows up by occluding grid lines behind it. A *correct* `Loop` return
value would produce a visible textured teapot and **fail** the pixel diff. This
must be written down in `docs/nodes.md` and pinned by a WGSL fixture test, or a
later well-meaning fix will silently break the rung.

**(b) `oscSine` at `t = 0` is 0.** `((sin((render.nodeUniform1 + 0.75) * 6.283185307179586) * 0.5) + 0.5)` with the frozen clock = 0, so the loop's `scale` is 0 anyway. The `time` uniform still has to exist and land in the 144-byte render struct.

**(c) `texture(map)` carries a `mat3x3` uv matrix; `texture(map, uv)` does not.**
Materials 23, 25, 24, 28/29, 30 emit
`textureSample( t, t_sampler, ( object.nodeUniformN * vec3<f32>( uv, 1.0 ) ).xy )`;
materials 31, 32, 33 (explicit uv) sample the raw uv. The port already splits
these as `tsl::texture()` vs `tsl::texture_uv()`.

**(d) `wgslFn` body text is passed through verbatim**, indentation and trailing
whitespace included, with only the function *name* substituted. `m28` shows the
include emitted first (`fn desaturate`) then `fn someFn`, both with the page's
original tabbing. A WGSL-diff gate has to match that byte for byte.

**(e) `wgslFn`'s texture+sampler pair.** `{ tex: textureNode, tex_sampler:
textureNode }` builds the *same* `TextureNode` twice with different output
types; `TextureNode.generate` returns `<property>` for the texture and
`<property> + '_sampler'` for the sampler. Result:
`getWGSLTextureSample( nodeUniform0, nodeUniform0_sampler, nodeVarying4 )`.

**(f) `MeshNormalMaterial` emits a real `sRGBTransferEOTF` WGSL function**
(`m30`), because `MeshNormalNodeMaterial.setupDiffuseColor` wraps
`packNormalToRGB(normalView)` in `colorSpaceToWorking(…, SRGBColorSpace)`. The
port already emits `sRGBTransferOETF` the same way (`tsl.rs:2219`,
`shader_fn`), so this is a 15-line mirror.

**(g) The alpha-test material is not `transparent`**, so after the `discard` it
still gets `DiffuseColor.w = 1.0` and no blend state.

---

## 4. Gap list against the port

`MAIN` = `/home/tom/src/projects/three-rs/main`. "Shared path" marks work in a
file every existing rung compiles through — the merge-conflict and regression
risks.

### Have

| thing | Rust item |
|---|---|
| `MeshBasicNodeMaterial` + `colorNode` | `MAIN/src/materials/mod.rs:338`, `color_node` field, `setup_diffuse_color` |
| `LineBasicNodeMaterial`, `LineSegments`, `line-list` topology | `MAIN/src/objects/line.rs`, `MeshBasicNodeMaterial::line()`; merged 2026-09-13 ("lines") |
| `vertexColors` (widened to `vec4` with alpha 1) | `mod.rs:191` + `tsl::vertex_color()` — matches `m14` exactly |
| `TeapotGeometry(50,18)` | `MAIN/src/geometries/teapot.rs` (419 lines, bit-exact vs three) |
| `positionLocal/World`, `normalLocal/World/View`, `positionView` | `tsl::position_local/…/normal_view` |
| `cameraProjectionMatrix` (render group) | `tsl::camera_projection_matrix()` |
| `texture()` with matrix, `texture_uv()` without | `tsl.rs:1745` / `:1791` |
| `uv()`, `.dot/.mul/.add/.div/.negate/.xyz`, `to_var`/`assign` | `tsl.rs`, `node.rs` |
| `Loop` node | `tsl::loop_n`, `Node::Loop` (rung 6) |
| `Fn` inlined, no WGSL fn emitted | `tsl::inline_fn` / `call` |
| `oscSine`, `time` uniform | `tsl::osc_sine` (`tsl.rs:2210`) |
| sorted transparent list, painter sort | `MAIN/src/renderer/render_list.rs` |
| blending (`src-alpha`/`one-minus-src-alpha` table) | `MAIN/src/materials/blending.rs` |
| `transparent`, `opacity` uniform, `is_opaque()` → `DiffuseColor.w = 1.0` | `mod.rs:239/380`, `node_material.rs:131` |
| MSAA 4x + `rgba16float` internal target + `outputColorTransform` | `renderer/mod.rs:559`, rung 2 |
| `scene.background = Color` → clear value, no mesh | `Background::Color` (`objects/scene.rs:14`) |
| `RepeatWrapping` S/T samplers, JPEG decode, 2d-array mipmap blit | `renderer/mod.rs:2018`, `loaders/texture_loader.rs`, `renderer/mipmap.rs` |
| `frag_coord()`, `viewport_size()` (render group) | `tsl.rs:1195`, `:1380` |
| named WGSL `fn` emission (for `sRGBTransfer*`) | `tsl::shader_fn` + `FnDef` |
| `discard` + `If` statement machinery | `Node::Discard`, `tsl::discard_if` |

### Missing

| # | thing | Three source (JS lines) | lands in | Rust est. | kind | shared path? |
|---|---|---|---|---|---|---|
| 1 | `wgslFn` — `Node::Code`/`FunctionNode`, the regex WGSL declaration parser, named-parameter binding, `includes` ordering, the `_sampler` output form, `// codes` emission | `src/nodes/code/FunctionNode.js` 178, `CodeNode.js` 181, `FunctionCallNode.js` 187, `renderers/webgpu/nodes/WGSLNodeFunction.js` 188 (**734**, ~450 of them relevant) | new `MAIN/src/nodes/code.rs`; `node.rs` (+`Node::Code`, `Node::CodeCall`), `builder.rs` (make `add_code` reachable, dedupe by name, ordering) | **380–450** | node-system | **yes** — `builder.rs`, `node.rs` |
| 2 | `opacityNode` + `alphaTest`/`alphaTestNode` in `setupDiffuseColor` | `materials/nodes/NodeMaterial.js:858-903` (**45**) | `MAIN/src/materials/mod.rs` (2 fields), `MAIN/src/materials/node_material.rs::setup_diffuse_color` | **70** | material | **yes** — every material compiles through `setup_diffuse_color` |
| 3 | `MeshNormalNodeMaterial` (`MaterialKind::Normal`), `packNormalToRGB`, `colorSpaceToWorking`/`sRGBTransferEOTF` | `MeshNormalNodeMaterial.js` 67 + `PackingNode.js` + `ColorSpaceFunctions.js` (**~140**) | `MAIN/src/materials/mod.rs` (kind + alias), `node_material.rs` (a `setup_diffuse_color` branch that *bypasses* the opacity/alphaTest path), `tsl.rs` (`srgb_transfer_eotf`, `pack_normal_to_rgb`) | **150** | material | partly (`node_material.rs`) |
| 4 | `triplanarTexture` | `src/nodes/utils/TriplanarTextures.js` (**65**) | `MAIN/src/nodes/tsl.rs` | **70** | node-system | no |
| 5 | `screenUV` + `.flipY()` — the `FlipNode` var-caching + `vec2(v.x, 1.0 - v.y)` reconstruction | `display/ScreenNode.js` 249 (UV scope only) + `utils/FlipNode.js` 106 (**~90** relevant) | `tsl.rs` (`screen_uv()`), `node.rs` (`Node::Flip` or reuse `to_var` + `vec2_join`) | **60** | node-system | no |
| 6 | `Loop` **as an expression that generates nothing** — the `vec4<f32>( )` behaviour of §3(a), plus `VarNode`'s loop hoisting (`var<private>` declared outside, assigned inside) | `utils/LoopNode.js` 390 + `core/VarNode.js` build branch (**~60** relevant) | `MAIN/src/nodes/builder.rs`, `tsl::loop_n` | **40** | node-system | **yes** — `builder.rs` |
| 7 | `GridHelper` | `src/helpers/GridHelper.js` (**84**) | new `MAIN/src/helpers/grid_helper.rs` (+ `src/helpers/mod.rs`) | **80** | object/helper | no |
| 8 | `color()` named TSL fn; node-valued `vec2/3/4` ergonomics (`vec3_join` spelling), `.mod`-free but `.x/.y` writes | — | `tsl.rs` | **40** | node-system | no |
| 9 | `Fn` called with a *named* input map (`{ color: … }`) vs no inputs — port's `inline_fn` is positional only; must produce **byte-identical** WGSL for both spellings | `tsl/TSLCore.js:505-599`, `NodeUtils.js:492` (**~95**) | `tsl.rs` (an ergonomic wrapper; the inlining itself is already right) | **30** | node-system | no |
| 10 | The example + e2e registration + oracle test | — | `MAIN/examples/webgpu_materials.rs`, `MAIN/tests/e2e/main.rs`, `MAIN/examples/dump_wgsl.rs`, `MAIN/src/bin/viewer.rs` | **220** | harness | **yes** — `tests/e2e/main.rs`, `dump_wgsl.rs`, `viewer.rs` are touched by every rung |
| 11 | WGSL fixtures + unit gates (§5) | — | `MAIN/tests/fixtures/webgpu_materials/`, `MAIN/tests/nodes_materials_zoo.rs` | **180** | tests | no |

**Total: ~1300 lines of Rust**, of which item 1 (`wgslFn`) is ~35% and is the
only piece with real design freedom.

### Not needed (contrary to the brief's premise)

No Lambert/Phong/Standard/Physical/Toon/Matcap, no `wireframe` (the port's
`polygon_mode` stays `Fill`), no `flatShading`, no `side` other than the
default, no env map, no lights, no tone mapping, no `alphaMap`/`alphaHash`,
no PNG in `TextureLoader`. All of that is either already done (rungs 5/8) or
simply absent from this page.

### Dependencies on rungs 10/11/12

**None.** No skinning, no `BatchedMesh`, no compute, no storage buffers, no
`Points`. Nothing here conflicts with those three branches except the shared
harness files in item 10 and `builder.rs` in items 1 and 6.

---

## 5. Gates beyond the pixel diff

This rung is unusually well-gated without the image, which is what makes a
one-sitting attempt realistic.

**(a) WGSL diff, 32 modules.** `dump/m01…m32_*.wgsl` are Three's exact output
for the 17 materials + the grid. The established workflow
(`MAIN/examples/dump_wgsl.rs`, `docs/dumping.md` step 2) compares the port's
printed WGSL per material against these line for line. Each material is an
independent gate, so the rung can be walked one material at a time. Copy the
files the fixture test needs into `MAIN/tests/fixtures/webgpu_materials/` —
not the whole dump (issue #70).

**(b) Program-dedup assertion.** Materials 28 and 29 must produce **byte-identical**
WGSL, and the program cache must therefore hand out one pipeline for both. This
is a sharp, cheap test of `Fn` inlining across the two call spellings, and it is
invisible to the pixel diff.

**(c) The `vec4<f32>( )` fixture.** Pin `m23_fragment_fragment.wgsl` verbatim.
It is the one place where "correct" and "matches Three" diverge (§3a).

**(d) Numeric oracle — `scene_t0.json` (13 KB, beside this file).** Generated
now, from three.js r186's own `src/` in node with no GPU
(`node oracle.mjs`, source inlined in that file's `note`). Format:

```
{ note, randomSeedStart: 0.7853981633974483, randomDrawCount: 51,
  randomDraws: [ 51 floats, in mesh order, x/y/z per mesh ],
  camera: { fov, aspect, near, far, position,
            matrixWorld[16], matrixWorldInverse[16], projectionMatrix[16] },
  meshes: [ 17 × { index, position[3], rotation[3], quaternion[4],
                   matrixWorld[16] } ],   // AFTER the one animate() step
  gridHelper: { vertexCount: 164, positionFirst12, positionLast12,
                colorFirst12, colorDistinct, positionSum } }
```

Assert the port's `DeterministicRandom` reproduces `randomDraws` exactly, then
each mesh's `matrixWorld` and the two camera matrices to ~1e-12. This is the
rung-5 class of bug (Euler → quaternion sync, `lookAt` before/after
`updateMatrixWorld`) caught before any pixel is drawn. `gridHelper` pins the
new helper's buffers without a GPU.

**(e) Draw-order assertion.** `RenderList` must emit the order in §3 —
specifically the grid at position 8 of 18 and materials 27, 24 last. A unit
test over `render_list.items()` catches a sort regression that the image would
only show as z-fighting.

**(f) Bind-group-layout assertion.** Exactly three distinct BGLs, with layout 97
(two sampler+texture pairs) used by material 25 alone, and group-0 buffers of
128/144 B. The dump's `bindGroupLayouts` / `bindGroups` sections are the
expected values.

**(g) Three's own QUnit tests.** `test/unit/src/helpers/GridHelper.tests.js`
exists upstream and is worth porting alongside item 7; there is no unit test
for the node pieces.

---

## 6. Order of work

Each step leaves the ladder (11 green rungs) green; only step 8 turns
`webgpu_materials` itself green.

1. **`GridHelper` + the oracle test.** New `src/helpers/`, `scene_t0.json`
   checked in as a fixture, `DeterministicRandom` + transforms + grid buffers
   asserted. No renderer change, no pixel risk. *(~90 lines Rust, ~80 test.)*
2. **`opacityNode` + `alphaTest`/`alphaTestNode` in `setup_diffuse_color`.**
   The highest-regression-risk edit, done early and alone so the 11 existing
   rungs re-run against it immediately. Gate: the ladder unchanged
   (0/60/31/0/0/18/1/40/…) **and** `m06`'s `discard` block matches. *(~70.)*
3. **`MeshNormalNodeMaterial` + `sRGBTransferEOTF` + `packNormalToRGB`.**
   Gate: `m29`/`m30` byte-exact. Self-contained new `MaterialKind`. *(~150.)*
4. **The six accessor materials (18–23, 26) + the grid material.** Nothing new
   is needed — they are the "does the port already do this" sweep. Gate:
   `m07`–`m10`, `m13`–`m17`, `m20`, `m21`, `m24`–`m27` byte-exact. Expect
   fallout only in uniform ordering inside `objectStruct` and `@location`
   order (a known §8 divergence — decide here whether to close it or record
   it, because 17 modules make the divergence very visible). *(~60.)*
5. **`screenUV` + `.flipY()`, then `triplanarTexture`.** Both are pure
   additions to `tsl.rs`. Gates: `m02`, `m12`. *(~130.)*
6. **`Fn` named-input ergonomics + the dedup assertion.** Gate: `m04` byte-exact
   **and** one pipeline for materials 28+29. *(~30.)*
7. **`Loop` as a value: the `vec4<f32>( )` fidelity fix + `VarNode` hoisting.**
   Gate: `m23` byte-exact, with the divergence written into `docs/nodes.md`.
   *(~40.)*
8. **`wgslFn`.** The big one, deliberately last so that if it overruns, steps
   1–7 are already merged and green. Gates: `m19` (texture+sampler), then `m28`
   (includes ordering + verbatim body text). Then the example and the pixel
   gate. *(~450 + 220 harness.)*

If the sitting runs out at step 7, everything merged is still green and the
remaining work is one well-defined file.

---

## 7. What this rung unlocks

Grepped over `~/src/vendor/three.js/examples/webgpu_*.html`:

| feature this rung adds | other examples that use it |
|---|---|
| `Loop(` | **223 examples** — already partly there from rung 6, but this rung's expression-form fix is the remaining half |
| `screenUV` | **36** (`webgpu_backdrop*`, `webgpu_compute_particles_*`, `webgpu_animation_retargeting*`, …) |
| `opacityNode` | **14** (`webgpu_hdr`, `webgpu_particles_soft`, `webgpu_instance_points`, `webgpu_compute_particles*`, …) |
| `oscSine` | 10 |
| `GridHelper` | 9 (`webgpu_instance_uniform`, `webgpu_particles`, `webgpu_lightprobes*`, `webgpu_compute_particles`) |
| `TeapotGeometry` | 10 (already ported) |
| `wgslFn` | **2** — this one and `webgpu_tsl_interoperability` |
| `triplanarTexture` | 2 — this one and `webgpu_backdrop_water` |
| `alphaTestNode` | 2 — this one and `webgpu_sandbox` (0.1%, not gradeable here) |
| `MeshNormalMaterial` | 3 (`webgpu_compile_async`, `webgpu_shadow_contact`) |

The cheapest immediate follow-ons:

- **`webgpu_instance_uniform`** (graded 0.0% today): `GridHelper` +
  `TeapotGeometry` + `MeshStandardNodeMaterial` (rung 8) + per-instance
  uniforms. After this rung, it is a small rung.
- **`webgpu_tsl_interoperability`** (graded 0.0% today): reuses this rung's
  `wgslFn` wholesale, and needs on top of it only `varyingProperty` inside raw
  WGSL (`varyings.vUv = uv;`), a WGSL *vertex* shader as `positionNode`, PNG in
  `TextureLoader`, an `OrthographicCamera` and
  `outputColorSpace = LinearSRGBColorSpace`. ~250 lines once `wgslFn` exists.

`triplanarTexture` and `alphaTestNode` unlock almost nothing else; they are
paid for here because the image demands them.

---

## 8. Verdict

**One rung, sized at the top of what one Opus worker can do in a sitting
(~1300 lines of Rust), with §6's eight internally-gated steps.** Not two.

Why not split by material family: **there are no families.** 16 of 17 materials
are the same class (`MeshBasicNodeMaterial`) with a different `colorNode`. The
real axis is node-builder feature, and on that axis eleven of the seventeen
materials cost *nothing* — the port already builds them (step 4 is a sweep, not
a port).

Why not split into two rungs at all: **the pixel gate on this example is
all-or-nothing.** One black teapot or one missing grid line and the diff blows
past 0.1%. A "half" of this example is not renderable, so a first rung would
have to be graded on a *different* image, and no sibling is a strict subset:

- `webgpu_sandbox` is the closest overlap (`alphaTestNode`, `opacityNode`,
  `oscSine`) but grades **0.1%** on this machine — rung 0's own policy drops
  candidates on that line, so it cannot carry a rung.
- `webgpu_tsl_interoperability` isolates `wgslFn` and grades 0.0%, but it is
  *more* `wgslFn` machinery, not less: raw-WGSL vertex stage plus
  `varyingProperty` access from inside WGSL. Making it the prerequisite rung
  trades one hard item for a harder one and adds PNG loading and a second
  output colour space.

So the correct split is *inside* the rung, against the WGSL dumps, which is
exactly how rungs 4–8 were run. §5 gives eight independent gates that need no
image, and §6 orders them so that the two shared-path edits
(`setup_diffuse_color`, `builder.rs`) land early, under the protection of the
eleven already-green rungs, and the one open-ended item (`wgslFn`) lands last,
where overrunning costs a follow-up commit rather than the rung.

**Top three risks**

1. **`wgslFn` (item 1, ~450 lines).** A regex WGSL-declaration parser, a global
   `// codes` section with include ordering, name-collision renaming, verbatim
   body passthrough (whitespace included), and the `texture`/`texture_2d<f32>`
   → `<prop>` / `<prop>_sampler` dual build of one `TextureNode`. It touches
   `builder.rs` and `node.rs`, which rungs 10/11/12 are also editing. Mitigated
   by doing it last, behind `m19` then `m28`.
2. **`setup_diffuse_color` (item 2).** Every material in the port compiles
   through it; adding `opacityNode` and the alpha-test branch there can move
   any of the eleven green rungs by a pixel. Mitigated by doing it second,
   alone, with a full ladder re-run as its gate.
3. **Reproducing Three's dropped `Loop` value (§3a).** The port has to emit
   `DiffuseColor = vec4<f32>( );` — deliberately wrong-looking code — or that
   teapot renders textured instead of black and the diff fails. It also has to
   survive a future reviewer. Mitigated by a verbatim `m23` fixture plus a
   `docs/nodes.md` entry.

Lesser risks, for completeness: the 17 modules will expose the existing
`@location`-order and uniform-ordering divergences much more loudly than any
previous rung (decide once, in step 4); and `zune-jpeg` vs libjpeg-turbo
(≤3/channel on `uv_grid_opengl.jpg`, worst RGB distance 3.46 vs a 44 threshold)
now affects **nine** of the eighteen draws rather than one, so the residue
budget is thinner than on rung 3.

---

## Files written

| file | size |
|---|---|
| `PLAN.md` | this |
| `scene_t0.json` | 13 KB — the numeric oracle of §5(d) |
| `reference.jpg` | 33 KB — `examples/screenshots/webgpu_materials.jpg` |
| `grade-run1.log`, `grade-run2.log` | 210 B each |
| `dump/dump.json` | 155 KB |
| `dump/m00…m34_*.wgsl` | 35 files, 47 KB total |
| `dump/actual_full.png` | 97 KB — the raw 800×500 frame |
| `dump/actual.jpg` | 33 KB — the frame as the grader scales it |

Total 432 KB under `dump/`, nothing over a few MB.
