# `webgpu_lights_selective`

Status: **green.** 9 of 100000 pixels against three.js 5f610f5's own
`test/e2e/image.js`, threshold 0.1%. Three's own frame for the page also
scores 9 against the same JPEG. Intel Iris Xe, Mesa 25.3.6, wgpu on Vulkan.
Steady frame 4.6 ms, 5 draw calls, 62001 triangles.

This is `webgpu_lights_phong`'s scene with `MeshStandardNodeMaterial`
teapots. There are four point lights with unlit spheres, the magenta range
fog and a normal-mapped centre teapot lit by all four lights. The two outer
teapots are each lit by one light through `material.lightsNode = lights( [
light ] )`: red on the left, with `roughnessNode = texture( alphaTexture )`,
and blue on the right, with `metalnessNode = texture( alphaTexture )`.

## What was added

| area | what |
| --- | --- |
| `src/materials/node_material.rs` | `setupVariants()` wraps an explicit `metalnessNode` / `roughnessNode` in `float()`, as three does |
| `examples/` | `webgpu_lights_selective.rs`; `dump_wgsl` sections `selective_left`, `_centre`, `_right`, `_light_sphere` against three's `m02`, `m04`, `m06`, `m08` |

Selective lights (`lights_node`), `TeapotGeometry`, `normalMap`, the point
lights' `power` and `fog( color, rangeFogFactor )` were all already ported for
`webgpu_lights_phong`.

## What the pixels found

The first grade was already 9 pixels. The WGSL was not yet right, though.
The dump comparison found that the port skipped three's `float()` around an
explicit metalness or roughness node. A bare `texture()` node is a `vec4`, so
the right teapot's `DiffuseContribution` was `( vec4( diffuse, 1 ) * ( 1 -
map ) ).xyz`, one minus each of the map's channels, where three has
`diffuse * ( 1 - map.x )`. `roughness_map.jpg` is grey, so the pixels could
not tell. §40 of `docs/nodes.md` has the details.

The rest of the Standard lighting flow differs from three's dump in order and
in a few duplicated terms: `directDiffuse = vec3( 0 )` is assigned twice, and
the irradiance term gets its own `singleScattering` / `multiScattering` pair
where three reuses the dielectric one. These already appear in every Standard
and Physical rung, and they compute the same values. This rung leaves them
alone.
