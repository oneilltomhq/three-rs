# `webgpu_fog_height`

Status: **green.** 0 of 100000 pixels against three.js 5f610f5's own
`test/e2e/image.js`, threshold 0.1%. Intel Iris Xe, Mesa 25.3.6, wgpu on
Vulkan. Steady frame 2.8 ms (release, `viewer --headless --frames 40`), 3 draw
calls, 3185 triangles.

A hundred `1 x 25 x 1` boxes, one `InstancedMesh` of `MeshPhongNodeMaterial`
on a 10 x 10 grid, lit by a pink `DirectionalLight` and an `AmbientLight`,
standing in a peach height fog:

```js
scene.fogNode = fog( color( 0xffdfc1 ), exponentialHeightFogFactor( uniform( 0.04 ), uniform( 2 ) ) );
scene.backgroundNode = color( 0xffdfc1 );
```

## Reconciling with the plans

There is no scout `PLAN.md` for this rung. It was ported from
`~/src/vendor/three.js/examples/webgpu_fog_height.html` directly, with three's
WGSL dumped by `tools/dump-webgpu.mjs` into `target/dumps/webgpu_fog_height/`
(uncommitted). Every piece the page uses was already in the port:
`tsl::exponential_height_fog_factor` (§28), `scene.fog_node`, a node
background (`Background::Node`), `InstancedMesh` under a lit material, and
`OrbitControls` with damping.

## What was added

| area | what |
| --- | --- |
| `examples/` | `webgpu_fog_height.rs`; a `dump_wgsl` section `fog_height` (three's `m02` / `m03`) |
| `tools/dump-webgpu.mjs` | a stray read of the old `pageFile` option, left by a merge, made every dump throw; removed |
| registrations | the `rung!` and `#[test]` entries, the README row and gallery thumbnail, the web trio and manifest, the viewer row |

`docs/nodes.md` §36 is the long form.

## What the pixels found

Nothing to find: the first graded frame was 0 pixels. The fog statement in
`dump_wgsl`'s `fog_height` is three's line for line, with `density` and
`height` in the object group after the material's own uniforms, as in the
dump (`object.nodeUniform15` / `16` there). The rest of the Phong shader
differs from three's only in the classes §8 already lists (uniform numbering,
varying order, `nodeVarN` where three has `let nodeConstN`).
