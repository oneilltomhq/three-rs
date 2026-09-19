# The environment-map family of `webgpu_*` examples

Scouted 2026-09-19 against `~/src/vendor/three.js` @ r186. Companion to
`PLAN.md`, which plans the first of these as a rung.

"Environment map" here means: the example's look comes from `scene.environment`
or `material.envMap` (or a `scene.background`/`backgroundNode` fed from the same
texture), by one of four routes —

* **`fromScene`** — `PMREMGenerator.fromScene( scene, sigma )` renders a scene
  (often `examples/jsm/environments/RoomEnvironment.js`) into the cube-UV atlas.
* **`fromEquirectangular`** — an equirect HDR/LDR image, either called
  explicitly or reached implicitly by assigning an equirect texture to
  `scene.environment` (`NodeManager.updateEnvironment` →`EnvironmentNode` →
  `PMREMNode` → `fromEquirectangular`).
* **`fromCubemap`** — a six-face cube texture through the same implicit path.
* **non-PMREM cube reflection** — `envMap` on a `MeshBasicMaterial`, sampled
  through `CubeMapNode`/`reflectVector` with no prefiltering. This route is
  already ported (`webgpu_materials_basic` is green on the ladder) and is listed
  only so it is not mistaken for IBL.

A fifth, `CubeCamera`, renders the scene to a live cube texture each frame;
those examples are in the table but are a different rung family.

## 1. The table

70 `webgpu_*` examples match. `env path` is what the page does, `exc` is Three's
own `exceptionList` in `test/e2e/puppeteer.js` (those cannot be graded here and
are not candidates). `Inspector`, `OrbitControls`, `FirstPersonControls` and the
`WebGPU` capability check are omitted from "other heavy deps" — every example
has them and none of them affects the graded frame.

| example | env path | loader / format | other heavy deps | exc |
|---|---|---|---|---|
| `webgpu_clearcoat` | cubeTexLoader+scene.env | 6× Radiance .hdr, TextureLoader | HDRCubeTextureLoader, FlakesTexture | no |
| `webgpu_compute_water` | scene.env | DRACOLoader, GLTFLoader, Radiance .hdr | DRACOLoader, GLTFLoader, HDRLoader, SimplexNoise | no |
| `webgpu_cubemap_adjustments` | scene.env | GLTFLoader, Radiance .hdr | GLTFLoader, HDRLoader | no |
| `webgpu_cubemap_dynamic` | cubeTexLoader+CubeCamera+envMap-prop+scene.env | 6× Radiance .hdr, TextureLoader | HDRCubeTextureLoader | no |
| `webgpu_custom_fog` | fromScene+scene.env | — | ForestGenerator, TerrainGenerator, SkyMesh | no |
| `webgpu_custom_fog_background` | scene.env | GLTFLoader, UltraHDR .hdr.jpg | GLTFLoader, UltraHDRLoader | no |
| `webgpu_deferred` | scene.env | UltraHDR .hdr.jpg | TeapotGeometry, UltraHDRLoader | no |
| `webgpu_display_stereo` | cubeTexLoader+envMap-prop | 6× .jpg/.png | AnaglyphPassNode, ParallaxBarrierPassNode, StereoPassNode | no |
| `webgpu_furnace_test` | fromScene+envMap-prop | — | — | no |
| `webgpu_generator_building` | fromScene+scene.env | — | SkyscraperGenerator, SunLight, SunLightNode, SkyMesh | no |
| `webgpu_geometry_loft` | RoomEnvironment+fromScene+scene.env | — | RoomEnvironment, LoftGeometry, SunLight, SunLightNode | no |
| `webgpu_instance_path` | RoomEnvironment+fromScene+scene.env | — | RoomEnvironment | no |
| `webgpu_instance_uniform` | cubeTexLoader | 6× .jpg/.png | TeapotGeometry | no |
| `webgpu_lightprobe` | cubeTexLoader+envMap-prop | 6× .jpg/.png | LightProbeHelperGPU, LightProbeGenerator | no |
| `webgpu_lightprobe_cubecamera` | cubeTexLoader+CubeCamera | 6× .jpg/.png | LightProbeHelperGPU, LightProbeGenerator | no |
| `webgpu_lights_sunlight` | fromScene+scene.env | — | SunLight, SunLightNode, SkyMesh | no |
| `webgpu_loader_gltf` | scene.env | GLTFLoader, UltraHDR .hdr.jpg | GLTFLoader, UltraHDRLoader | no |
| `webgpu_loader_gltf_anisotropy` | scene.env | GLTFLoader, UltraHDR .hdr.jpg | GLTFLoader, UltraHDRLoader | no |
| `webgpu_loader_gltf_dispersion` | scene.env | GLTFLoader, Radiance .hdr | GLTFLoader, HDRLoader | no |
| `webgpu_loader_gltf_iridescence` | scene.env | GLTFLoader, Radiance .hdr | GLTFLoader, HDRLoader | no |
| `webgpu_loader_gltf_sheen` | scene.env | GLTFLoader, UltraHDR .hdr.jpg | GLTFLoader, UltraHDRLoader | no |
| `webgpu_loader_gltf_transmission` | scene.env | DRACOLoader, GLTFLoader, UltraHDR .hdr.jpg | DRACOLoader, GLTFLoader, UltraHDRLoader | no |
| `webgpu_loader_materialx` | scene.env | GLTFLoader, Radiance .hdr, MaterialXLoader | GLTFLoader, HDRLoader, MaterialXLoader, MaterialXInterfaceValidation, BufferGeometryUtils | no |
| `webgpu_materials_alphahash` | RoomEnvironment+fromScene+scene.env | — | RoomEnvironment, SSAAPassNode | no |
| `webgpu_materials_basic` | cubeTexLoader+envMap-prop | 6× .jpg/.png | — | no |
| `webgpu_materials_cubemap_mipmaps` | cubeTexLoader+envMap-prop | 6× .jpg/.png | — | no |
| `webgpu_materials_displacementmap` | cubeTexLoader+envMap-prop | 6× .jpg/.png, OBJLoader, TextureLoader | OBJLoader | no |
| `webgpu_materials_envmaps` | cubeTexLoader+envMap-prop | 6× .jpg/.png, TextureLoader | — | no |
| `webgpu_materials_envmaps_bpcem` | CubeCamera | TextureLoader | RectAreaLightHelper, RectAreaLightTexturesLib | no |
| `webgpu_materials_envmaps_groundprojected` | fromEquirect+envMap-prop+scene.env | DRACOLoader, GLTFLoader, Radiance .hdr, TextureLoader | DRACOLoader, GLTFLoader, HDRLoader, GroundedSkybox | no |
| `webgpu_materials_retroreflection` | fromScene+scene.env | — | BloomNode, hashBlur | no |
| `webgpu_materials_transmission` | envMap-prop | UltraHDR .hdr.jpg | UltraHDRLoader | no |
| `webgpu_materialx_noise` | cubeTexLoader+scene.env | FontLoader, 6× Radiance .hdr | TextGeometry, FontLoader, HDRCubeTextureLoader, MaterialXHextile | no |
| `webgpu_mrt` | scene.env | GLTFLoader, UltraHDR .hdr.jpg | GLTFLoader, UltraHDRLoader | no |
| `webgpu_ocean` | fromScene+scene.env | TextureLoader | SkyMesh, WaterMesh, BloomNode | no |
| `webgpu_parallax_uv` | scene.env | Radiance .hdr, TextureLoader | HDRLoader | no |
| `webgpu_performance` | scene.env | DRACOLoader, GLTFLoader, UltraHDR .hdr.jpg | DRACOLoader, GLTFLoader, UltraHDRLoader | no |
| `webgpu_pmrem_cubemap` | cubeTexLoader+envMap-prop | 6× Radiance .hdr | HDRCubeTextureLoader | no |
| `webgpu_pmrem_equirectangular` | envMap-prop | UltraHDR .hdr.jpg | UltraHDRLoader | no |
| `webgpu_pmrem_scene` | fromScene+cubeTexLoader | 6× .jpg/.png | — | no |
| `webgpu_pmrem_test` | fromEquirect+envMap-prop | Radiance .hdr | HDRLoader | no |
| `webgpu_postprocessing_bloom_emissive` | scene.env | GLTFLoader, Radiance .hdr | GLTFLoader, HDRLoader, BloomNode | no |
| `webgpu_postprocessing_ca` | RoomEnvironment+fromScene+scene.env | — | RoomEnvironment, ChromaticAberrationNode | no |
| `webgpu_postprocessing_dof_basic` | envMap-prop+scene.env | DRACOLoader, GLTFLoader, UltraHDR .hdr.jpg | DRACOLoader, GLTFLoader, UltraHDRLoader, FXAANode, boxBlur | no |
| `webgpu_postprocessing_lensflare` | scene.env | GLTFLoader, UltraHDR .hdr.jpg | GLTFLoader, UltraHDRLoader, BloomNode, GaussianBlurNode, LensflareNode | no |
| `webgpu_postprocessing_retro` | scene.env | GLTFLoader, Radiance .hdr, TextureLoader | GLTFLoader, HDRLoader, CRT, RetroPassNode, Shape, Bayer | no |
| `webgpu_postprocessing_sobel` | RoomEnvironment+fromScene+scene.env | GLTFLoader | RoomEnvironment, GLTFLoader, SobelOperatorNode | no |
| `webgpu_postprocessing_ssr` | RoomEnvironment+fromScene+scene.env | DRACOLoader, GLTFLoader | RoomEnvironment, DRACOLoader, GLTFLoader, SMAANode, SSRNode | no |
| `webgpu_reflection_roughness` | scene.env | TextureLoader, UltraHDR .hdr.jpg | UltraHDRLoader | no |
| `webgpu_skinning_instancing_individual` | RoomEnvironment+fromScene+scene.env | GLTFLoader | RoomEnvironment, SunLight, SunLightNode, GLTFLoader | no |
| `webgpu_sky` | CubeCamera+envMap-prop | — | SkyMesh | no |
| `webgpu_tonemapping` | scene.env | DRACOLoader, GLTFLoader, Radiance .hdr | DRACOLoader, GLTFLoader, HDRLoader | no |
| `webgpu_tsl_angular_slicing` | scene.env | DRACOLoader, GLTFLoader, UltraHDR .hdr.jpg | SunLight, SunLightNode, DRACOLoader, GLTFLoader, UltraHDRLoader | no |
| `webgpu_tsl_procedural_terrain` | scene.env | Radiance .hdr | SunLight, SunLightNode, HDRLoader | no |
| `webgpu_tsl_wood` | scene.env | FontLoader, Radiance .hdr | RoundedBoxGeometry, TextGeometry, FontLoader, HDRLoader, WoodNodeMaterial | no |
| `webgpu_upscaling_fsr1` | RoomEnvironment+fromScene+scene.env | DRACOLoader, GLTFLoader | RoomEnvironment, DRACOLoader, GLTFLoader, FSR1Node | no |
| `webgpu_upscaling_taau` | RoomEnvironment+fromScene+scene.env | DRACOLoader, GLTFLoader | RoomEnvironment, DRACOLoader, GLTFLoader, SharpenNode, TAAUNode | no |
| `webgpu_compute_cloth` | scene.env | UltraHDR .hdr.jpg | UltraHDRLoader | **yes** |
| `webgpu_compute_particles_fluid` | scene.env | UltraHDR .hdr.jpg | UltraHDRLoader, BufferGeometryUtils | **yes** |
| `webgpu_compute_rasterizer_ibl` | scene.env | GLTFLoader, UltraHDR .hdr.jpg | GLTFLoader, UltraHDRLoader | **yes** |
| `webgpu_cubemap_mix` | cubeTexLoader+scene.env | 6× .jpg/.png, GLTFLoader, 6× Radiance .hdr | GLTFLoader, HDRCubeTextureLoader | **yes** |
| `webgpu_generator_city` | fromScene+scene.env | — | CityGenerator, LightProbeGridHelper, LightProbeGrid, SkyMesh, BloomNode | **yes** |
| `webgpu_materials_matcap` | — | OpenEXR, GLTFLoader, TextureLoader | EXRLoader, GLTFLoader | **yes** |
| `webgpu_materials_texture_html` | RoomEnvironment+fromScene+scene.env | — | RoomEnvironment, RoundedBoxGeometry, InteractionManager | **yes** |
| `webgpu_morphtargets_face` | RoomEnvironment+fromScene+scene.env | GLTFLoader, KTX2 | RoomEnvironment, GLTFLoader, KTX2Loader | **yes** |
| `webgpu_postprocessing_ao` | RoomEnvironment+fromScene+scene.env | DRACOLoader, GLTFLoader | RoomEnvironment, RoundedBoxGeometry, DRACOLoader, GLTFLoader, GTAONode, SSAONode, TRAANode | **yes** |
| `webgpu_postprocessing_dof` | cubeTexLoader | 6× .jpg/.png | DepthOfFieldNode | **yes** |
| `webgpu_postprocessing_ssr_denoise` | scene.env | GLTFLoader, Radiance .hdr | SunLight, SunLightNode, GLTFLoader, HDRLoader, RecurrentDenoiseNode, SSRNode, SharpenNode, TRAANode, TemporalReprojectNode | **yes** |
| `webgpu_tsl_graph` | scene.env | GLTFLoader, Radiance .hdr, TSLGraphLoader | GLTFLoader, HDRLoader | **yes** |
| `webgpu_water` | scene.env | DRACOLoader, GLTFLoader, TextureLoader, UltraHDR .hdr.jpg | DRACOLoader, GLTFLoader, UltraHDRLoader, Water2Mesh, BloomNode, FXAANode | **yes** |

## 2. Reading the table

* **Off the exception list: 57 of the 70.** Of those, **16** use `fromScene`,
  **2** call `fromEquirectangular` explicitly (`webgpu_pmrem_test`,
  `webgpu_materials_envmaps_groundprojected`) and **~30 more** reach it
  implicitly through `scene.environment = <equirect texture>`.
* **The gate is almost never PMREM — it is the loader and the model.**
  `UltraHDRLoader` (JPEG + gain map) appears in 17 gradable examples;
  `GLTFLoader` with full PBR in 22; `DRACOLoader` in 8. The examples with *no*
  loader at all are the `fromScene` ones, which is what makes `fromScene`
  attractive despite being the bigger generator path.
* **Radiance `.hdr` is the cheap HDR format** and `rung-pmrem-a` already decodes
  it bit-exactly (`src/loaders/hdr_loader.rs`, gated against the vendor's own
  `HDRLoader.parse`). Ten gradable examples use it directly.
* **EXR is irrelevant to this family.** The only `webgpu_*` `EXRLoader` user is
  `webgpu_materials_matcap`, which is on the exception list. Do not spend a
  rung on EXR. If it is ever needed, port Three's `EXRLoader` rather than take a
  crate — the port's gate is bit-exactness against Three's decoder, and
  `rung-pmrem-a` already found that Three's half-float conversion *truncates*
  where any sane crate rounds.
* **UltraHDR is the loader worth a rung of its own, later.**
  `examples/jsm/loaders/UltraHDRLoader.js` parses a JPEG with an embedded XMP
  gain map and a second JPEG stream, and reconstructs a `HalfFloatType`
  equirect. The port already decodes JPEG for `TextureLoader`, so the new work
  is the XMP/MPF parsing and the gain-map maths — and those must match Three
  texel for texel. Landing it converts 17 examples from "impossible" to "glTF +
  PBR away".
* **`RoomEnvironment` is 185 lines of pure scene construction** and needs no
  assets: a `BackSide` `MeshStandardMaterial` room box, an `InstancedMesh` of 6
  boxes, one `PointLight( 0xffffff, 900, 28, 2 )`, and six emissive-only
  `MeshLambertMaterial` panels (`color: 0x000000, emissive: 0xffffff,
  emissiveIntensity: 17…100`). Because the panels' diffuse colour is black, the
  Lambert lighting model contributes nothing and they are pure emission — a
  `MeshLambertNodeMaterial` shim is needed for the WGSL to match, but its
  lighting term is provably dead. The construction is trivially gated
  numerically: assert the 13 transforms, the light parameters and the six
  emissive intensities against the JS.

## 3. Recommended order

Each step is one rung, and each one's *new* machinery is the only thing the next
one does not already have. Everything here assumes `rung-pmrem-b`
(`fromCubemap` + cube-UV sampling + `webgpu_pmrem_cubemap`) has landed.

| # | example | grade | what it adds |
|---|---|---|---|
| 1 | **`webgpu_pmrem_test`** | 0.0% ×2 | `fromEquirectangular`: `equirectUV( outputDirection )`, the equirect `NodeMaterial`, `_setSize( w / 4 )`. Plus `HDRLoader.load()` (the `DataTextureLoader` wrapper over the already-ported `parse()`), **`flipY` on a data-texture upload**, and `scene.background` accepting a cube-UV render-target texture. ~250–350 lines. Planned in `PLAN.md`. |
| 2 | **`webgpu_furnace_test`** | 0.0% ×2 | `fromScene`, in its minimal form: the env scene is `new Scene()` with `background = new Color( 0xcccccc )` and nothing else, so `_sceneToCubeUV` is the `BackgroundBox` (`BoxGeometry`, `BackSide`, depth off) plus six 90° cube-camera renders into viewport tiles with `autoClear` false. One renderer capability is new: *render a scene into a viewport slice of a target without clearing*. The consumer side is an 11×11 `MeshPhysicalMaterial` roughness×metalness grid — and a uniform white furnace is the strongest numeric gate in the family: with a constant environment the shaded result must equal the environment for every roughness/metalness pair, so a wrong energy term shows up as a visible band rather than a plausible picture. No loaders, no assets, no controls, no `Math.random`. ~150–200 lines on top of #1. |
| 3 | **`webgpu_pmrem_scene`** | 0.0% ×2 | `fromScene` over a *real* scene: six `MeshBasicMaterial` spheres plus a **cube-texture background** rendered inside the cube camera (the `useSolidColor = false` branch), which means the background sphere mesh must work while rendering into the atlas. Also the only example that uses `pmremTexture( target.texture, normalWorld, uniform( 0.5 ) )` directly as a `colorNode`, so it exercises cube-UV sampling with no PBR in the way. `CubeTextureLoader` with 6 JPEGs — already ported. |
| 4 | **`webgpu_postprocessing_ca`** | ungraded here (sibling of the postprocessing family) | `RoomEnvironment` + `fromScene( env, 0 )` + `scene.environment` (i.e. `EnvironmentNode` fed from a cube-UV texture rather than `material.envMap`), on top of a one-node `ChromaticAberrationNode` pass. This is where `RoomEnvironment` itself lands — the room box, the instanced boxes, the point light and the Lambert emissive shim. Pairs naturally with whatever the postprocessing-family scout recommends. |
| 5 | **`webgpu_materials_alphahash`** | ungraded here | Same `RoomEnvironment` + `fromScene`, and it reuses `SSAAPassNode` from the already-landed `rung-ssaa`. Adds alpha-hash dithering only. Cheap once #4 exists. |
| 6 | **`webgpu_instance_path`** | 0.0% ×2 | The first example needing `fromScene( scene, **0.04** )` — i.e. the `_blur()` / `sphericalGaussianBlur` spiral-kernel path, which every rung above skips because `sigma` is 0. Also `instancedBufferAttribute`, `NeutralToneMapping`, a `screenUV` `backgroundNode`, animated `time` nodes, and **4000 `Math.random()` draws before the frame** (`Renderer::skip_random_draws`). Worth doing last of the six: it is the only one that needs the blur shader, and its randomness makes it the fiddliest to line up. |

Two honourable mentions, deliberately not in the list: `webgpu_materials_envmaps`
and `webgpu_materials_cubemap_mipmaps` (both 0.0% ×2) are cheap and
asset-light, but their `envMap` goes through the non-PMREM `CubeMapNode`
reflection path that `webgpu_materials_basic` already proves — they would add
refraction mapping and cube mip selection, not environment lighting. Keep them
as filler rungs, not as part of this ladder.

## 4. Files here

| file | what |
|---|---|
| `PLAN.md` | the rung plan for `webgpu_pmrem_test` |
| `ENVIRONMENT-FAMILY.md` | this file |
| `e2e-env-candidates-2026-09-19.log` | both grader runs, six examples |
| `webgpu_pmrem_test.jpg` | Three's reference screenshot (12 KB) |
| `dump-pmrem_test/` | `dump-webgpu.mjs` output: `dump.json`, 11 `.wgsl` modules, `actual_full.png`, `actual.jpg` (440 KB total) |
