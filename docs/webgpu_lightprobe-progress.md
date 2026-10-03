# `webgpu_lightprobe`

Status: **green.** 2 of 100000 pixels against three.js' own
`test/e2e/image.js`, threshold 0.1%. Intel Iris Xe, Mesa 25.3.6, wgpu on
Vulkan. Steady frame 1.3 ms, 4 draw calls, 6913 triangles; the steady frame
builds nothing.

A white, roughness-0 standard sphere in front of the pisa cube. It is lit by
one directional light, by the cube's PMREM, and by a `LightProbe` whose nine
spherical-harmonic coefficients `LightProbeGenerator.fromCubeTexture()`
projects out of the same cube. A `LightProbeHelper` to the left draws the
probe's irradiance on its own small sphere.

## Reconciling with the plans

There is no scout plan for this rung. It was ported from
`~/src/vendor/three.js/examples/webgpu_lightprobe.html`,
`src/lights/LightProbe.js`, `src/nodes/lighting/LightProbeNode.js`, the
`getShIrradianceAt` in `src/nodes/functions/material/getShIrradianceAt.js`,
`examples/jsm/lights/LightProbeGenerator.js` and
`examples/jsm/helpers/LightProbeHelperGPU.js`. Three's WGSL was dumped by
`tools/dump-webgpu.mjs` into `target/dumps/webgpu_lightprobe/` (uncommitted).
The probe's line in the sphere's fragment (`m10`) is now a unit test.

## What was added

| area | what |
| --- | --- |
| `src/lights/light_object.rs` | `LightKind::Probe`, the light's `sh` field, `LightProbe::new` / `copy` |
| `src/nodes/tsl.rs` | `get_sh_irradiance_at` (public), the probe's 9-element `uniformArray` |
| `src/nodes/node.rs`, `src/renderer/` | `BufferSource::LightProbe( index )`, filled per draw from `LightState.sh`, which is premultiplied by the intensity; `BufferSource::Live` for the helper |
| `src/materials/phong.rs` | the `Probe` arm of `setup_light`: `irradiance += getShIrradianceAt( normalWorld, sh )` |
| `src/materials/node_material.rs` | Phong's `irradiance = vec3( 0 )` goes ahead of the light loop for any irradiance-only light (a fix, see below) |
| `src/addons/lights.rs` | `LightProbeGenerator::from_cube_texture` / `from_cube_render_target` |
| `src/addons/helpers.rs` | `LightProbeHelper` |
| `tools/light_probe_generator_reference.mjs` | three's own generator under node, as the oracle for the unit test |
| `tests/` | `addons_light_probe_generator.rs` (the SH projection against three, to 1e-9), `nodes_light_probe.rs` (the WGSL against three's dump) |

`docs/nodes.md` §60 is the long form.

## What the pixels found, and what they could not

**The pixel grader cannot see the SH maths.** A sign flipped in a band-1
term of the face table moves the probe's irradiance by a few levels on
the sphere. That stays well inside the colour threshold, and the frame still
passed. So the projection is checked where it can be:
`tests/addons_light_probe_generator.rs` runs three's `LightProbeGenerator.js`
under node, against the same face bytes, with a canvas stub. It compares all
27 numbers on the pisa cube and on a patterned cube, in sRGB and in
`NoColorSpace`, to a relative 1e-9. Two closed-form cases (a uniform cube
projects to c0 only, and its irradiance is π·L) pin the normalisation.

**The WGSL is three's.** `get_sh_irradiance_at` builds its products in
three's order and folds `2.0 * k` the way JavaScript does, so the line
matches three's dump exactly, apart from the buffer's node id.
`tests/nodes_light_probe.rs` asserts it, and asserts that it lands after
`irradiance = vec3( 0 )` and `normalWorld`.

**A Phong-family bug turned up on the way.** With a hemisphere light and no
ambient light, `setup_phong` zeroed `irradiance` after the light loop. That
wiped the hemisphere's contribution, and a probe's would have gone the same
way. The zero now goes ahead of the loop. No graded rung lit a Phong,
Lambert or Toon material that way. `nodes_light_probe.rs` has the regression
test.

The 2 remaining pixels were not chased.

## Divergences

- `fromCubeTexture()` reads the decoded face bytes that `CubeTextureLoader`
  holds. three reads them from a canvas, which gives the same 8-bit values
  for the PNG and JPEG cubes the addon accepts. A cube that is not
  `UnsignedByteType` returns an `Err`.
- `LightProbe::copy()` copies the `Light` half, not the transform (see
  §60's divergences).
- `LightProbeHelper::update()` is the position and scale half of three's
  `onBeforeRender()`, and the page calls it every frame. The `sh` and
  `intensity` uniforms are already read live.
