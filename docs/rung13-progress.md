# Rung 13 — `webgpu_tsl_galaxy`

**Result: 40 of 100000 pixels different (0.04%), limit 0.1%.** Passed on the
first render, with no iteration against the reference image.

The example is a pure-TSL `SpriteNodeMaterial`: one `InstancedMesh(
PlaneGeometry( 1, 1 ), material, 20000 )`, four `range()` nodes, additive
blending, `depthWrite: false`. The scene has no lights, no textures and no
post-processing, so everything new here is in the node system.

## What this rung needed

Step 1 and step 2 of the scout plan (blend state, the `isOpaque()` gate, the
transparent list, `range()` / `instanceMatrix` as instanced vertex attributes,
`Type::Mat2`) were already on port from the blend branch. Verified, not rebuilt.
What was actually missing:

### 1. The `setupPositionView` seam

`docs/nodes.md` §6 claimed `NodeMaterial` already routed `setup_position_view`
through `builder.context`. It did not: `position_view()` was a fixed
`thread_local` singleton equal to `modelViewMatrix * vec4( positionLocal, 1 )`,
and `model_view_projection()` was another one built on top of it.

Both are now built by one context-keyed function, `position_view_pair()`, keyed on
the `NodeRef::key()` of the value `with_material_position_view()` installed —
the same shape `normal_view()` / `with_material_normal()` already had. See
`docs/nodes.md` §10 for the full seam.

This is the only change in this rung that touches a path every other material
takes, so it is the one to watch in a merge. It is behaviour-preserving when no
override is installed, and every rung's pixel count is unchanged.

### 2. `SpriteNodeMaterial`

`MaterialKind::Sprite` plus five material fields (`position_node`, `scale_node`,
`rotation_node`, `rotation`, `size_attenuation`) and
`MeshBasicNodeMaterial::sprite()`, which sets `transparent: true` the way
`SpriteNodeMaterial`'s constructor does. The fragment flow is `Basic`'s,
unchanged. `node_material::setup_position_view_sprite()` is the vertex shader.

`setupPosition()` gained its last step,
`positionLocal.assign( vec3( positionNode ) )`, which no earlier rung used.

### 3. TSL additions

`two_pi()`, `rotate()` (`RotateNode`'s `vec2` branch), `pow3()`, `.zw()`,
`material_rotation()` + `UniformSource::MaterialRotation`.

### 4. `range()`'s `w` component

`BufferSource::Range` carried `min: Color, max: Color`, and the renderer filled
the fourth component from a hardcoded `1.0`. Three's `RangeNode.setup()` widens
each end to a `Vector4` by three different rules, and only the `Color` rule puts
1 in `w`; `range( vec3( -1 ), vec3( 1 ) )` has `w = 0` at both ends. It now
carries `[f64; 4]`, built by `tsl::RangeValue`, and `instanced_range()` also
applies the narrowing `convert()` Three ends with — so `range( 0, 1 )` reads
`….x` and a `vec3` range reads `….xyz`, and callers no longer swizzle by hand.
`docs/nodes.md` §9.3.

### 5. `InstancedMesh::new` filled zero matrices

three.js' constructor ends with `for ( … ) this.setMatrixAt( i, _identity )`.
The port left the array at zeros. Harmless for rung 2 (which overwrites every
matrix) and for this rung (whose `positionNode` replaces `positionLocal`, making
the instance block dead), but it would silently collapse any mesh that relies on
the default. Fixed.

## Determinism: the PRNG offset, and the fill order

Two things have to be right or every instance gets the wrong numbers, and nothing
errors.

**Offset.** `Math.random` is shared across the page, and the example makes five
draws after the material's `range()` nodes are built but before the frame that
fills them — `new Inspector()`'s tabs each construct a `List`, whose constructor
ends `this.id = \`list-${Math.random().toString( 36 ).slice( 2, 11 )}\``
(`examples/jsm/inspector/ui/List.js:11`). three.js' own `generateUUID()` does not
count: the harness rewrites it to the unseeded `Math._random`.
`Renderer::skip_random_draws( 5 )` is how the example spends them.

**Order.** Three fills each buffer once, inside `RangeNode.setup()`, so the order
is setup order. The port fills lazily, in `NodeProgram::vertex_buffers()` order,
which is attribute-allocation order, which is generate order:

| # | buffer | reached by |
|---|---|---|
| 1 | `range( 0, branches )` | `positionNode` → `position` → `cos( angle )` → `angle` → `branchAngle` |
| 2 | `range( 0, 1 )` radiusRatio | `angle`'s second operand, `time.mul( radiusRatio.oneMinus() )` |
| 3 | `range( vec3( -1 ), vec3( 1 ) )` | `positionNode`'s second operand, `randomOffset` |
| 4 | `range( 0, 1 )` scaleNode | the vertex flow only: `modelViewProjection` → `positionView` → `setupPositionView()` → `scaleNode` |

That matches Three's order because `NodeBuilder.build()` flows
`context.position` — the `setupPosition()` stack — before either shader stage in
every build stage, and `shaderStages` is `[ 'fragment', 'vertex', 'compute' ]`,
so the `positionNode` chain is set up first and `scaleNode`, reachable only
through `setupPositionView()`, last.

Both facts are implicit in the port's traversal, so they are pinned by
`tests/nodes_range_buffers.rs`, which drives `fill_range()` through the real
program's vertex buffers and checks the first four floats of each against the
buffers three.js uploads (`handoff/scouts/rung13/PLAN.md` §2.4). All sixteen
numbers agree, including the `vec3` range's zero `w` — which is independent
confirmation of both the offset and the widening rule.

## WGSL

`cargo run --example dump_wgsl` emits `tsl_galaxy_sprite` from the same
`galaxy_material()` the example builds. Statement for statement it matches
`handoff/scouts/rung13/{vertex,fragment}-r186.wgsl`; the only differences are the
ones `docs/nodes.md` §8 already lists (generated name numbering, attribute
`@location` order, `m[ 0u ]`, the absent `VERTEX_` temps, `var<private>`
declaration order). Uniform layout matches exactly: `renderStruct` 144 bytes
(`time` @0, projection @16, view @80); `objectStruct` 112 bytes (two `vec3`
colours @0/@16, `materialOpacity` @**28** in the second colour's padding,
`modelWorldMatrix` @32, `materialRotation` @96, the example's `size` @100).

## Gates

* `cargo build --workspace --all-targets` — clean, no new warnings.
* `cargo test --workspace -- --test-threads=1` — 838 passed, 0 failed (835 at
  `4fadfc1`, + 2 in `tests/nodes_range_buffers.rs`, + 1 e2e).
* e2e, serial: depth_texture 0, instance_mesh 60, lights_phong 31,
  materials_basic 0, postprocessing_masking 18, rtt 1, **tsl_galaxy 40** — all
  of 100000, limit 100.

## Not done

* `alphaToCoverage` multisample plumbing and `colorWrite` (inherited gap —
  nothing in the ladder sets either).
* `RotateNode`'s `vec3` / `vec4` branch.
* `Sprite` as an object: `object.center` is a `Sprite` field, so
  `setupPositionView()`'s `alignedPosition.sub( center.sub( 0.5 ) )` step is not
  ported. An `InstancedMesh` has no `center`.
* `OrbitControls`. The page constructs one with `enableDamping`, but on the first
  frame `controls.update()` leaves the camera where `position.set()` put it and
  aims it at the default target — which `camera.look_at( Vector3::ZERO )` already
  does.
