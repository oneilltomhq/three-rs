# webgpu_postprocessing_motion_blur

Branch `velocity-traa`, issue #163.

**Green at 5 pixels of 100000** (limit 0.1%), steady frame 6.2 ms, 9 draw
calls, 100553 triangles.

## What the score says, and what it does not

The rung proves the page **builds and composes**. It does not prove that
anything is blurred.

Only the first frame is graded. On that frame `VelocityNode` has never seen
any object or camera, so it seeds every previous matrix with the current one,
as three's does. The velocity attachment is zero everywhere. `motion_blur`
then takes seventeen taps of the same texel, and the vignette darkens the
corners. So the score checks these things:

- the scene pass with `mrt( { output, velocity } )` builds;
- its second attachment is created and sampled;
- the skinned Xbot compiles with `positionPrevious` skinned by a second bone
  buffer;
- the blur loop and vignette are the page's;
- the scene under them is right.

It says nothing about the motion itself. `tests/velocity_frames.rs` checks
that on the GPU, over several frames, against hand-computed NDC distances:

- a plane moved one unit;
- the camera moved one unit;
- a skinned plane whose only bone moved one unit, which proves that last
  frame's bones reach `positionPrevious`;
- `set_velocity_projection_matrix()` hiding a view-offset jitter.

Each case also checks a zero on the first frame and on a frame after the
motion stops. The page's output quad is checked against three's dump
(`m14`) by `motion_blur_matches_three` in `tests/nodes_display_wgsl.rs`.

## What this rung adds

| area | what |
|---|---|
| `src/nodes/velocity.rs` | `velocity`, and `VelocityState`, the previous-frame store: per object, per camera and per skeleton |
| `src/nodes/frame.rs` | `NodeFrameState.velocity`, swept with the other per-frame maps |
| `src/nodes/node.rs` | `UniformSource::{PreviousModelWorldMatrix, VelocityProjectionMatrix, PreviousProjectionMatrix, PreviousCameraViewMatrix}`, `BufferSource::PreviousBoneMatrices` |
| `src/nodes/tsl.rs` | `positionPrevious` |
| `src/nodes/skinning.rs` | `positionPrevious` skinned with last frame's bones when the MRT has `velocity` (`SkinEntry.previous`) |
| `src/nodes/builder.rs` | `NodeProgram.reads_velocity`, which gates every bit of the bookkeeping |
| `src/renderer/mod.rs` | the two update steps in `draw()`; the bone roll in `update_skeleton`; `set_velocity_projection_matrix()`; skinned shadow casters |
| `src/nodes/display/motion_blur.rs` | `MotionBlur.js` |
| `src/materials/node_material.rs` | the Phong-family hemisphere fix (below) |

## Two things the scene found

**Hemisphere lights under Phong were lost.** The page lights a Phong floor
and Phong walls with two `HemisphereLight`s and a sun. The port pushed each
hemisphere's `irradiance +=` during the direct-light loop. When the scene had
no `AmbientLight`, the tail then reset `irradiance` to zero, which wiped those
adds. The right wall came out nearly black. Three's order is: direct lights,
then the reset, then the hemisphere adds. `setup_phong` now follows it. This
fixes every Phong, Lambert and Toon material lit by a hemisphere light.

**Skinned shadow casters were drawn unskinned.** The Xbot casts the sun's
shadow. The shadow pass now runs `update_skeleton` and binds the bones as the
main pass does. Point-light shadows of skinned meshes are still unskinned.

## Divergences

- The history is keyed by object id and camera id in the renderer's
  `NodeFrameState`, not held in a `WeakMap` on the node (#154, decision 1).
  An object's or a skeleton's entry is held weakly and goes when the object
  is dropped, as a `WeakMap` entry would. A camera's entry goes after
  `CACHE_GRACE_FRAMES` frames with no draw through it.
- `velocity.setProjectionMatrix( m )` is
  `Renderer::set_velocity_projection_matrix( Some( m ) )`, because the store
  belongs to the renderer.
- A frame ends at a render to the screen (`docs/nodes.md` §57.3), so a
  camera's history rolls once per presented frame, as three's does once per
  `frameId`.

## Left out

These are not on this page:

- `positionPrevious` for `InstancedMesh`, `BatchedMesh` and `Line2`;
- the bone-texture fallback for skeletons too large for a storage buffer;
- point-shadow skinning.

Three's per-object `useVelocity` mark is not ported either.
`ShadowBaseNode` sets it on a shadow caster drawn while the MRT has
`velocity`, so that the caster's shadow draw also builds the
`positionPrevious` path. Here a shadow draw never builds it. Nothing reads
velocity in a shadow map, so no pixel depends on it.
