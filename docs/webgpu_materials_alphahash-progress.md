# `webgpu_materials_alphahash`

Status: **ported, not graded.** Three.js itself fails its own reference for
this page on this machine: three's frame differs from
`webgpu_materials_alphahash.jpg` in 3782 of 100000 pixels (3.782%, over the
0.1% limit; `rung0/e2e-run6-rung1-candidates2.log` recorded 3.5% on the same
page). The port's full-resolution frame is **pixel-identical** to
three's: 0 differing pixels, max channel difference 0, against
`tools/dump-webgpu.mjs`' `actual_full.png`. Against the reference the port
scores 3637 pixels (3.637%). The e2e test is `#[ignore]`d with that reason,
following the `webgpu_instance_path` / `webgpu_lights_custom` precedent; the
page stays in the steady-frame ladder (`rung!`). Intel Iris Xe, Mesa, wgpu on
Vulkan.

27 instanced icosphere balls, each a random `setColorAt()` colour, share one
`MeshStandardMaterial` with `alphaHash: true` and `opacity: 0.5`. The only
light is `scene.environment`, `PMREMGenerator.fromScene( new RoomEnvironment(),
0.04 )`, and the frame is `ssaaPass( scene, camera )` at `sampleLevel` 3
(eight jittered scene renders) through a `RenderPipeline`.

## Reconciling with the task

The task described the page as a `MeshPhysicalNodeMaterial` under a TRAA
pass. The page at 5f610f5 is neither: it is a plain `MeshStandardMaterial`,
and its post-processing is `ssaaPass`, which the port has had since
`webgpu_postprocessing_ssaa`. No TRAA port was needed, so none was made.

## What was added

| area | what |
| --- | --- |
| `src/nodes/alpha_hash.rs` | `getAlphaHashThreshold( position )`, a real WGSL `fn` via `setLayout`, with `hash2D` / `hash3D` as inlined `Fn()`s |
| `src/materials/mod.rs`, `node_material.rs` | `Material.alphaHash`; `setupDiffuseColor()` discards `diffuseColor.a < getAlphaHashThreshold( positionLocal )` after the alpha test and before the opaque clamp |
| `examples/` | `webgpu_materials_alphahash.rs`; `dump_wgsl` section `materials_alphahash` (three's `m13` + `m14`) |

`docs/nodes.md` §51 has the design.

## What the pixels found

Nothing to fix in the port: the first run's frame was already byte-identical
to three's own frame on this GPU. The whole 3.6–3.8% is the reference. The
hash is `fract( 10000 * sin( … ) )` of `floor( positionLocal * 2^n )`, with
`n` from `dpdx` / `dpdy` of the position. A last-bit difference in `sin`,
`log2` or the derivatives on the machine that rendered the reference moves
which texels a fragment falls into, and so which fragments are discarded. The
diff image is red grain spread evenly over every ball, not edges or shading.
No correct port can match that grain on this GPU, and loosening the grader or
copying anything from the screenshot is not allowed. The rung stays ungraded
until the ladder runs where three's own frame hits the reference.

## Checked against three's WGSL

`dump_wgsl`'s `materials_alphahash` against `m14`: `getAlphaHashThreshold`
has the same statements in the same order, and the discard line and its
position in `main` match. Differences are the naming classes of §8 (`var
nodeVarN` for three's `let nodeConstN`, uniform numbering). One is new to this
section: three writes the CDF's `cases` vector inline in each branch of the
nested `select`, where the port's usage counter promotes it to a var per
branch (§51.3). Same value, and the pixels are identical.
