# `webgpu_loader_gltf_compressed`

Status: **green.** 4 of 100000 pixels against three.js 5f610f5's own
`test/e2e/image.js`, threshold 0.1%. Intel Iris Xe, Mesa 25.3.6, wgpu on
Vulkan. Steady frame 6.1 ms, 3 draw calls,
332259 triangles.

`coffeemat.glb`, a mug with an Oreo-dunking arm, is compressed every way glTF
allows:
- geometry: `EXT_meshopt_compression` bufferViews over `KHR_mesh_quantization`
  accessors (uint16 positions and uvs, int8 normals and tangents);
- textures: five `KHR_texture_basisu` KTX 2.0 maps, transcoded by
  `KTX2Loader` after `detectSupport( renderer )`, each behind a
  `KHR_texture_transform` into a shared atlas.

The only light is a 1300 lm `PointLight` parented to the camera. The
background is a flat `0xEEEEEE`, and the output uses `ReinhardToneMapping`.

## Reconciling with the plans

There is no scout plan. The page was ported from
`~/src/vendor/three.js/examples/webgpu_loader_gltf_compressed.html`
directly. Three's WGSL was dumped by `tools/dump-webgpu.mjs` into
`target/dumps/webgpu_loader_gltf_compressed/` (uncommitted). The task brief
expected the RoomEnvironment PMREM, but this page has none. Its fragment
shaders zero every indirect term, as the port's do.

## What was added

| area | what |
| --- | --- |
| `src/nodes/builder.rs` | `FrontFacingNode.generate()`'s `true` outside the fragment stage |
| `examples/` | `webgpu_loader_gltf_compressed.rs`; `dump_wgsl` section `loader_gltf_compressed_coffee` |

`docs/nodes.md` §43 is the long form.

## What the pixels found

**The first frame did not compile.** `Material.001` and `OREO.001` are
double sided and the asset carries tangents. That combination builds
`bitangentView` in the vertex stage, where `negateOnBackSide()` reads
`frontFacing`. The port declared `@builtin( front_facing )` in the vertex
entry point, and wgpu rejected it. Three writes the literal `true` there.
With that one branch the frame was at 4 pixels, all on antialiased edges.

`dump_wgsl`'s `loader_gltf_compressed_coffee` matches three's `m00`/`m01`
structurally. The differences are the ones every rung has: the port's
`nodeVar` for three's `let nodeConst`, and declaration order. There is also
one deliberate divergence. Three binds the quantized `uint16` attributes as
`vec2<u32>` / `vec3<u32>` and converts them in the shader with
`vec3<f32>( position )`. The port's loader widens every attribute to `f32`
on the CPU, as `gltf_loader.rs` says, so it reads the same values from a
`vec3<f32>`.
