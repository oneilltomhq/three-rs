# three.js → three-rs parity

This matrix judges every exported name in three.js revision 5f610f5 (r187) — `src/`, the TSL exports and `examples/jsm` addons, 1315 rows — against three-rs `main` at 22d8630. A row counts as **Present** only when a QUnit port, a bit-exact oracle (the WGSL gates that diff generated shaders against three's own dumps count) or a graded e2e example verifies it, **Partial** when it exists but has a named gap or its only check is an ungraded example (a TSL function called by material or renderer code that graded examples render counts as verified), **Absent** when there is no definition (a name in a comment or TODO counts as Absent), and **N.A.** when it cannot apply to a native WebGPU port; coverage percentages leave N.A. rows out.

## Summary

| area | rows | Present | Partial | Absent | N.A. | Present of applicable |
|---|---|---|---|---|---|---|
| math | 29 | 26 | 2 | 1 | 0 | 90% |
| core | 29 | 11 | 7 | 8 | 3 | 42% |
| geometries | 21 | 19 | 0 | 2 | 0 | 90% |
| extras | 21 | 17 | 0 | 2 | 2 | 89% |
| animation | 14 | 13 | 1 | 0 | 0 | 93% |
| cameras | 6 | 5 | 0 | 1 | 0 | 83% |
| scenes | 3 | 3 | 0 | 0 | 0 | 100% |
| objects | 14 | 11 | 0 | 2 | 1 | 85% |
| lights | 11 | 7 | 1 | 3 | 0 | 64% |
| helpers | 13 | 1 | 1 | 11 | 0 | 8% |
| audio | 5 | 0 | 0 | 0 | 5 | — |
| materials | 36 | 4 | 19 | 11 | 2 | 12% |
| textures | 18 | 8 | 4 | 1 | 5 | 62% |
| loaders | 20 | 2 | 2 | 12 | 4 | 12% |
| renderers | 39 | 9 | 9 | 3 | 18 | 43% |
| nodes | 141 | 95 | 28 | 13 | 5 | 70% |
| tsl | 683 | 471 | 37 | 156 | 19 | 71% |
| addons/controls | 9 | 1 | 3 | 5 | 0 | 11% |
| addons/loaders | 71 | 6 | 2 | 63 | 0 | 8% |
| addons/postprocessing | 30 | 0 | 0 | 0 | 30 | — |
| addons/other | 102 | 11 | 4 | 64 | 23 | 14% |
| **total** | **1315** | **720** | **120** | **358** | **117** | **60%** |

TSL by family:

| family | rows | Present | Partial | Absent | N.A. |
|---|---|---|---|---|---|
| math | 109 | 108 | 1 | 0 | 0 |
| operators | 68 | 51 | 1 | 2 | 14 |
| conditionals/flow | 13 | 8 | 1 | 4 | 0 |
| textures | 23 | 17 | 1 | 5 | 0 |
| lighting/material | 121 | 75 | 9 | 37 | 0 |
| accessors | 95 | 76 | 8 | 10 | 1 |
| display/postprocessing | 75 | 40 | 8 | 27 | 0 |
| compute/storage | 53 | 19 | 3 | 31 | 0 |
| materialx | 49 | 48 | 0 | 1 | 0 |
| utils | 77 | 29 | 5 | 39 | 4 |

Graded examples: 87 (the README gallery plus `webgpu_textures_2d-array_compressed`, which has a live `rung!` but a hyphen in its name). 10 more are ported but `#[ignore]`d because three.js fails its own reference on this machine; a row whose only check is one of those is Partial.

## math

29 rows: 26 Present, 2 Partial, 1 Absent, 0 N.A.

| three.js name | verdict | three-rs symbol | verified by | note |
|---|---|---|---|---|
| `QuaternionLinearInterpolant` | Present | QuaternionLinearInterpolant alias (src/math/interpolants/quaternion_linear.rs:11) | tests/math_interpolants.rs |  |
| `LinearInterpolant` | Present | LinearInterpolant alias (src/math/interpolants/linear.rs:10) | tests/math_interpolants.rs |  |
| `DiscreteInterpolant` | Present | DiscreteInterpolant alias (src/math/interpolants/discrete.rs:11) | tests/math_interpolants.rs |  |
| `CubicInterpolant` | Present | CubicInterpolant alias (src/math/interpolants/cubic.rs:38) | tests/math_interpolants.rs |  |
| `BezierInterpolant` | Absent | — | — | not ported; Bezier tracks get no interpolant; comment only: src/animation/keyframe_track.rs:31 |
| `Interpolant` | Present | Interpolant<I> (src/math/interpolant.rs:110) | tests/math_interpolant.rs |  |
| `Triangle` | Present | Triangle (src/math/triangle.rs:10) | tests/math_triangle.rs |  |
| `MathUtils` | Partial | math_utils fns (src/math/math_utils.rs:16) | tests/math_math_utils.rs | no generateUUID/randInt/randFloat/randFloatSpread/seededRandom/setQuaternionFromProperEuler/normalize/denormalize |
| `Spherical` | Present | Spherical (src/math/spherical.rs:8) | tests/math_spherical.rs |  |
| `Cylindrical` | Present | Cylindrical (src/math/cylindrical.rs:7) | tests/math_cylindrical.rs |  |
| `Plane` | Present | Plane (src/math/plane.rs:9) | tests/math_plane.rs |  |
| `Frustum` | Present | Frustum (src/math/frustum.rs:9) | tests/math_frustum.rs |  |
| `FrustumArray` | Partial | crate-private ProjectCamera.sub_frustums (src/renderer/render_list.rs:152) | webgpu_camera_array | behaviour inlined in culling; no public type |
| `Sphere` | Present | Sphere (src/math/sphere.rs:9) | tests/math_sphere.rs |  |
| `Ray` | Present | Ray (src/math/ray.rs:8) | tests/math_ray.rs |  |
| `Matrix4` | Present | Matrix4 (src/math/matrix4.rs:24) | tests/math_matrix4.rs |  |
| `Matrix3` | Present | Matrix3 (src/math/matrix3.rs:9) | tests/math_matrix3.rs |  |
| `Matrix2` | Present | Matrix2 (src/math/matrix2.rs:11) | tests/math_matrix2.rs |  |
| `Box3` | Present | Box3 (src/math/box3.rs:11) | tests/math_box3.rs |  |
| `Box2` | Present | Box2 (src/math/box2.rs:8) | tests/math_box2.rs |  |
| `Line3` | Present | Line3 (src/math/line3.rs:9) | tests/math_line3.rs |  |
| `Euler` | Present | Euler (src/math/euler.rs:38) | tests/math_euler.rs |  |
| `Vector4` | Present | Vector4 (src/math/vector4.rs:11) | tests/math_vector4.rs |  |
| `Vector3` | Present | Vector3 (src/math/vector3.rs:13) | tests/math_vector3.rs |  |
| `Vector2` | Present | Vector2 (src/math/vector2.rs:10) | tests/math_vector2.rs |  |
| `Quaternion` | Present | Quaternion (src/math/quaternion.rs:10) | tests/math_quaternion.rs |  |
| `Color` | Present | Color (src/math/color.rs:29) | tests/math_color.rs |  |
| `ColorManagement` | Present | ColorManagement (src/math/color_management.rs:90) | tests/math_color_management.rs |  |
| `SphericalHarmonics3` | Present | SphericalHarmonics3 (src/math/spherical_harmonics3.rs:7) | tests/math_spherical_harmonics3.rs |  |

## core

29 rows: 11 Present, 7 Partial, 8 Absent, 3 N.A.

| three.js name | verdict | three-rs symbol | verified by | note |
|---|---|---|---|---|
| `Float32BufferAttribute` | Present | BufferAttribute::new, f32 storage (src/core/buffer_geometry.rs:136) | tests/core_buffer_attribute.rs | BufferAttribute is the Float32 case |
| `Float16BufferAttribute` | Absent | — | — | f32 storage only; no half-float attribute; comment only: tests/core_buffer_attribute.rs:7 |
| `Uint32BufferAttribute` | Partial | BufferAttribute::new_integer (src/core/buffer_geometry.rs:162) | no test | values held as f32, uploaded as u32; no typed array |
| `Int32BufferAttribute` | Partial | BufferAttribute::new (src/core/buffer_geometry.rs:136) | no test | no signed-int upload; values widened to f32 |
| `Uint16BufferAttribute` | Partial | BufferAttribute::new_integer (src/core/buffer_geometry.rs:162) | gltf skinIndex | values held as f32, uploaded as u32; no typed array/normalized |
| `Int16BufferAttribute` | Partial | BufferAttribute::new (src/core/buffer_geometry.rs:136) | no test | no signed-int/normalized storage; values widened to f32 |
| `Uint8ClampedBufferAttribute` | Absent | — | — | no clamped/normalized u8 attribute; comment only: tests/core_buffer_attribute.rs:7 |
| `Uint8BufferAttribute` | Partial | BufferAttribute::new_integer (src/core/buffer_geometry.rs:162) | no test | values held as f32, uploaded as u32; no normalized |
| `Int8BufferAttribute` | Partial | BufferAttribute::new (src/core/buffer_geometry.rs:136) | no test | no signed-int/normalized storage; values widened to f32 |
| `BufferAttribute` | Present | BufferAttribute (src/core/buffer_geometry.rs:102) | tests/core_buffer_attribute.rs | no usage/normalized/typed arrays |
| `RenderTarget` | Present | RenderTarget (src/renderer/render_target.rs:122) | webgpu_rtt, webgpu_multiple_rendertargets |  |
| `RenderTarget3D` | Absent | — | — | not ported |
| `Uniform` | N.A. | — | — | WebGL-only; TSL uniform() (src/nodes/tsl.rs:438) is the node counterpart |
| `UniformsGroup` | N.A. | — | — | WebGL-only |
| `InstancedBufferGeometry` | Present | BufferGeometry.instance_count (src/core/buffer_geometry.rs:532) | webgpu_struct_drawindirect, tests/nodes_instanced_attributes.rs | a field on BufferGeometry, not a type |
| `BufferGeometry` | Present | BufferGeometry (src/core/buffer_geometry.rs:504) | tests/core_buffer_geometry.rs |  |
| `InterleavedBufferAttribute` | Absent | — | — | geometry has no interleaved attributes; comment only: src/objects/sprite.rs:24, src/nodes/lines.rs:7 |
| `InstancedInterleavedBuffer` | Partial | nodes::node::InstanceBuffer item_size stride (src/nodes/node.rs:854) | tests/nodes_instanced_attributes.rs, webgpu_lines_fat | node-level only (instance matrix, Line2); no geometry-level type |
| `InterleavedBuffer` | Absent | — | — | geometry has no interleaved buffers; comment only: src/objects/sprite.rs:24 |
| `InstancedBufferAttribute` | Present | BufferAttribute::new_instanced (src/core/buffer_geometry.rs:149) + objects::InstancedBufferAttribute (src/objects/instanced_mesh.rs:17) | webgpu_struct_drawindirect, webgpu_instance_mesh |  |
| `GLBufferAttribute` | N.A. | — | — | WebGL-only |
| `Object3D` | Present | Object3D (src/core/object3d.rs:30) + scene-graph Node (src/core/node.rs:28) | tests/core_object3d.rs | QUnit hierarchy tests skipped |
| `Raycaster` | Present | Raycaster (src/core/raycaster.rs:179) | tests/core_raycaster.rs; webgpu_lines_fat_raycasting | no reversed-depth cases |
| `Layers` | Present | Layers (src/core/layers.rs:12) | tests/core_layers.rs; webgpu_layers |  |
| `EventDispatcher` | Absent | — | — | #153 #159; comment only: src/animation/animation_action.rs:28, docs/scene-graph.md:76 |
| `Clock` | Absent | — | — | deprecated r183; Timer ported instead |
| `Timer` | Present | Timer (src/core/timer.rs:26) | webgpu_shadowmap, webgpu_deferred, webgpu_loader_gltf | no connect()/page-visibility (DOM) |
| `RendererUtils` | Absent | — | — | state save/restore inlined privately; no public API; comment only: src/nodes/display/bloom.rs:468, src/nodes/display/rtt.rs:221 |
| `TSL` | Present | nodes::tsl module (src/nodes/tsl.rs:1) | webgpu_tsl_* graded examples | coverage per node rows |

## geometries

21 rows: 19 Present, 0 Partial, 2 Absent, 0 N.A.

| three.js name | verdict | three-rs symbol | verified by | note |
|---|---|---|---|---|
| `BoxGeometry` | Present | box_geometry (src/geometries/box_geometry.rs:31) | tests/geometry/batch1.rs (samples from three) |  |
| `CapsuleGeometry` | Present | capsule_geometry (src/geometries/capsule.rs:7) | tests/geometry/batch1.rs |  |
| `CircleGeometry` | Present | circle_geometry (src/geometries/circle.rs:8) | tests/geometry/batch1.rs |  |
| `ConeGeometry` | Present | cone_geometry (src/geometries/cone.rs:8) | tests/geometry/batch1.rs |  |
| `CylinderGeometry` | Present | cylinder_geometry (src/geometries/cylinder.rs:8) | tests/geometry/batch1.rs |  |
| `DodecahedronGeometry` | Present | dodecahedron_geometry (src/geometries/polyhedron.rs:335) | tests/geometry/batch1.rs |  |
| `EdgesGeometry` | Absent | — | — | not ported |
| `ExtrudeGeometry` | Present | extrude_geometry (src/geometries/extrude.rs:176) | tests/geometries_shape_oracle.rs | raw rs_symbol pointed at addons TextGeometry |
| `IcosahedronGeometry` | Present | icosahedron_geometry (src/geometries/polyhedron.rs:288) | tests/geometry/batch1.rs |  |
| `LatheGeometry` | Present | lathe_geometry (src/geometries/lathe.rs:16) | tests/geometry/batch1.rs |  |
| `OctahedronGeometry` | Present | octahedron_geometry (src/geometries/polyhedron.rs:308) | tests/geometry/batch1.rs |  |
| `PlaneGeometry` | Present | plane_geometry (src/geometries/plane.rs:6) | tests/geometry/batch1.rs |  |
| `PolyhedronGeometry` | Present | polyhedron_geometry (src/geometries/polyhedron.rs:9) | tests/geometry/batch2.rs |  |
| `RingGeometry` | Present | ring_geometry (src/geometries/ring.rs:8) | tests/geometry/batch1.rs |  |
| `ShapeGeometry` | Present | shape_geometry (src/geometries/shape.rs:20) | tests/geometries_shape_oracle.rs |  |
| `SphereGeometry` | Present | sphere_geometry (src/geometries/sphere.rs:8) | tests/geometry/batch0.rs |  |
| `TetrahedronGeometry` | Present | tetrahedron_geometry (src/geometries/polyhedron.rs:324) | tests/geometry/batch1.rs |  |
| `TorusGeometry` | Present | torus_geometry (src/geometries/torus.rs:8) | tests/geometry/batch1.rs |  |
| `TorusKnotGeometry` | Present | torus_knot_geometry (src/geometries/torus_knot.rs:7) | tests/geometry/batch0.rs |  |
| `TubeGeometry` | Present | tube_geometry (src/geometries/tube.rs:22) | tests/geometries_shape_oracle.rs |  |
| `WireframeGeometry` | Absent | — | — | not ported (material wireframe index exists, not the geometry class) |

## extras

21 rows: 17 Present, 0 Partial, 2 Absent, 2 N.A.

| three.js name | verdict | three-rs symbol | verified by | note |
|---|---|---|---|---|
| `ArcCurve` | Present | EllipseCurve::arc (src/extras/ellipse_curve.rs:78) | tests/extras_curves.rs | constructor on EllipseCurve, not a type |
| `CatmullRomCurve3` | Present | CatmullRomCurve3 (src/extras/catmull_rom_curve3.rs:99) | tests/extras_catmull_rom_curve3.rs |  |
| `CubicBezierCurve` | Present | CubicBezierCurve (src/extras/bezier_curves.rs:82) | tests/extras_curves.rs |  |
| `CubicBezierCurve3` | Present | CubicBezierCurve3 alias (src/extras/bezier_curves.rs:94) | tests/extras_curves.rs |  |
| `EllipseCurve` | Present | EllipseCurve (src/extras/ellipse_curve.rs:12) | tests/extras_curves.rs |  |
| `LineCurve` | Present | LineCurve (src/extras/line_curve.rs:10) | tests/extras_curves.rs |  |
| `LineCurve3` | Present | LineCurve3 alias (src/extras/line_curve.rs:18) | tests/extras_curves.rs |  |
| `QuadraticBezierCurve` | Present | QuadraticBezierCurve (src/extras/bezier_curves.rs:47) | tests/extras_curves.rs |  |
| `QuadraticBezierCurve3` | Present | QuadraticBezierCurve3 alias (src/extras/bezier_curves.rs:57) | tests/extras_curves.rs |  |
| `SplineCurve` | Present | SplineCurve (src/extras/spline_curve.rs:10) | tests/extras_curves.rs |  |
| `Shape` | Present | Shape (src/extras/shape.rs:18) | tests/extras_core.rs, tests/geometries_shape_oracle.rs |  |
| `Path` | Present | Path (src/extras/path.rs:22) | tests/extras_core.rs |  |
| `ShapePath` | Present | ShapePath (src/extras/shape_path.rs:25) | tests/extras_core.rs |  |
| `CurvePath` | Present | CurvePath (src/extras/curve_path.rs:35) | tests/extras_core.rs |  |
| `Curve` | Present | Curve trait (src/extras/curve.rs:126) | tests/extras_curves.rs, tests/extras_core.rs |  |
| `Controls` | Absent | — | — | no shared Controls base (connect/disconnect/update); no base type; OrbitControls, FirstPersonControls and FlyControls (src/addons/controls/) each stand alone |
| `DataUtils` | Present | to_half_float/from_half_float (src/extras/data_utils.rs:71) | tests/hdr_loader.rs (bit-exact vs three) |  |
| `ImageUtils` | N.A. | — | — | DOM canvas |
| `ShapeUtils` | Present | area/is_clock_wise/triangulate_shape (src/extras/shape_utils.rs:8) | tests/extras_core.rs, tests/geometries_shape_oracle.rs |  |
| `TextureUtils` | Absent | — | — | contain/cover/fill/getByteLength not ported |
| `PMREMGenerator` | N.A. | — | — | WebGL-only; WebGPU PmremGenerator (src/renderer/pmrem.rs:174) is Present: tests/pmrem.rs, webgpu_pmrem_* graded |

## animation

14 rows: 13 Present, 1 Partial, 0 Absent, 0 N.A.

| three.js name | verdict | three-rs symbol | verified by | note |
|---|---|---|---|---|
| `VectorKeyframeTrack` | Present | KeyframeTrack::vector / TrackValueType::Vector (src/animation/keyframe_track.rs:276) | tests/animation_keyframe_track.rs |  |
| `StringKeyframeTrack` | Present | KeyframeTrack::string / TrackValueType::String (src/animation/keyframe_track.rs:269) | tests/animation_keyframe_track.rs |  |
| `QuaternionKeyframeTrack` | Present | KeyframeTrack::quaternion / TrackValueType::Quaternion (src/animation/keyframe_track.rs:253) | tests/animation_keyframe_track.rs |  |
| `NumberKeyframeTrack` | Present | KeyframeTrack::number / TrackValueType::Number (src/animation/keyframe_track.rs:243) | tests/animation_keyframe_track.rs |  |
| `ColorKeyframeTrack` | Present | KeyframeTrack::color / TrackValueType::Color (src/animation/keyframe_track.rs:233) | tests/animation_keyframe_track.rs |  |
| `BooleanKeyframeTrack` | Present | KeyframeTrack::boolean / TrackValueType::Bool (src/animation/keyframe_track.rs:227) | tests/animation_keyframe_track.rs |  |
| `PropertyMixer` | Present | PropertyMixer (src/animation/property_mixer.rs:62) | tests/animation_property_mixer.rs |  |
| `PropertyBinding` | Partial | parse_track_name (src/animation/property_binding.rs:278) + SceneResolver (src/animation/object3d_target.rs:113) | tests/animation_property_binding.rs; webgpu_skinning | binds only position/quaternion/scale/morphTargetInfluences; no material.*, bones[n], by-name morph |
| `KeyframeTrack` | Present | KeyframeTrack (src/animation/keyframe_track.rs:177) | tests/animation_keyframe_track.rs | Bezier interpolation returns None |
| `AnimationUtils` | Present | animation_utils module fns (src/animation/animation_utils.rs:29) | tests/animation_animation_utils.rs | no convertArray/isTypedArray (JS-only) |
| `AnimationObjectGroup` | Present | AnimationObjectGroup (src/animation/animation_object_group.rs:303) | tests/animation_animation_object_group.rs |  |
| `AnimationMixer` | Present | AnimationMixer (src/animation/animation_mixer.rs:284) | tests/animation_animation_mixer.rs; webgpu_skinning | no events (#153) |
| `AnimationClip` | Present | AnimationClip (src/animation/animation_clip.rs:101) | tests/animation_animation_clip.rs |  |
| `AnimationAction` | Present | AnimationAction (src/animation/animation_action.rs:70) | tests/animation_animation_action.rs | no finished/loop events (#153) |

## cameras

6 rows: 5 Present, 0 Partial, 1 Absent, 0 N.A.

| three.js name | verdict | three-rs symbol | verified by | note |
|---|---|---|---|---|
| `StereoCamera` | Absent | — | — | not ported |
| `PerspectiveCamera` | Present | PerspectiveCamera (src/cameras/perspective_camera.rs:32) | tests/cameras_perspective_camera.rs |  |
| `OrthographicCamera` | Present | OrthographicCamera (src/cameras/orthographic_camera.rs:15) | tests/cameras_orthographic_camera.rs; webgpu_compute_points |  |
| `CubeCamera` | Present | CubeCamera (src/cameras/cube_camera.rs:27) | webgpu_sky, webgpu_lightprobe_cubecamera (graded); tests/renderer_cube_camera.rs | renders the six faces into a CubeRenderTarget, mips via generate_cube_mipmaps |
| `ArrayCamera` | Present | ArrayCamera (src/cameras/array_camera.rs:21) | webgpu_camera_array |  |
| `Camera` | Present | RenderCamera trait (src/cameras/mod.rs:88) | tests/cameras_perspective_camera.rs, tests/core_raycaster.rs | base is a sealed trait; no reversedDepth |

## scenes

3 rows: 3 Present, 0 Partial, 0 Absent, 0 N.A.

| three.js name | verdict | three-rs symbol | verified by | note |
|---|---|---|---|---|
| `FogExp2` | Present | FogExp2 (src/objects/fog.rs:62) | tests/scenes_fog_exp2.rs |  |
| `Fog` | Present | Fog (src/objects/fog.rs:20) | tests/scenes_fog.rs; webgpu_shadowmap |  |
| `Scene` | Present | Scene (src/objects/scene.rs:66) | tests/scenes_scene.rs |  |

## objects

14 rows: 11 Present, 0 Partial, 2 Absent, 1 N.A.

| three.js name | verdict | three-rs symbol | verified by | note |
|---|---|---|---|---|
| `Sprite` | Present | Sprite (src/objects/sprite.rs:53) | tests/objects_sprite.rs; webgpu_sprites |  |
| `LOD` | Absent | — | — | not ported (docs/scene-graph.md:453); comment only: src/core/raycaster.rs:290 |
| `SkinnedMesh` | Present | SkinnedMesh (src/objects/skinned_mesh.rs:29) | tests/objects_skinned_mesh.rs; webgpu_skinning |  |
| `Skeleton` | Present | Skeleton (src/objects/skeleton.rs:13) | tests/objects_skinned_mesh.rs; webgpu_skinning |  |
| `Bone` | Present | Bone (src/objects/bone.rs:10) | tests/objects_skinned_mesh.rs; webgpu_skinning |  |
| `Mesh` | Present | Mesh (src/objects/mesh.rs:15) | tests/objects_mesh.rs |  |
| `InstancedMesh` | Present | InstancedMesh (src/objects/instanced_mesh.rs:39) | tests/objects_instanced_mesh.rs; webgpu_instance_mesh |  |
| `BatchedMesh` | Present | BatchedMesh (src/objects/batched_mesh.rs:110) | tests/objects_batched_mesh.rs; webgpu_mesh_batch |  |
| `LineSegments` | Present | LineSegments (src/objects/line.rs:186) | tests/objects_line_segments.rs |  |
| `LineLoop` | N.A. | — | — | WebGPU-unsupported; deliberately absent: docs/scene-graph.md:155, docs/lines-progress.md:21 |
| `Line` | Present | Line (src/objects/line.rs:26) | tests/objects_line.rs |  |
| `Points` | Present | Points (src/objects/points.rs:21) | tests/objects_points.rs; webgpu_particles |  |
| `Group` | Present | Group (src/objects/group.rs:9) | tests/objects_group.rs | raw hit core::Group (geometry groups) is a different thing |
| `ClippingGroup` | Absent | — | — | no clipping context; comment only: docs/scene-graph.md:456 |

## lights

11 rows: 7 Present, 1 Partial, 3 Absent, 0 N.A.

| three.js name | verdict | three-rs symbol | verified by | note |
|---|---|---|---|---|
| `SpotLight` | Partial | SpotLight (src/lights/light_object.rs:224) | webgpu_shadowmap | no SpotLight.map (projected texture) |
| `PointLight` | Present | PointLight (src/lights/light_object.rs:182) | webgpu_lights_phong, webgpu_shadowmap_pointlight |  |
| `RectAreaLight` | Absent | — | — | not ported; comment only: src/materials/lighting_model.rs:117 |
| `HemisphereLight` | Present | HemisphereLight (src/lights/light_object.rs:204) | webgpu_lights_physical |  |
| `DirectionalLight` | Present | DirectionalLight (src/lights/light_object.rs:240) | webgpu_shadowmap, webgpu_fog_height |  |
| `AmbientLight` | Present | AmbientLight (src/lights/light_object.rs:166) | webgpu_shadowmap_pointlight, webgpu_camera_array |  |
| `Light` | Present | Light (src/lights/light.rs:10) + LightObject/LightKind (src/lights/light_object.rs:18) | webgpu_lights_phong |  |
| `LightShadow` | Present | LightShadow (src/lights/light_shadow.rs:132) | webgpu_shadowmap, webgpu_shadowmap_vsm, webgpu_shadowmap_pointlight | no autoUpdate/needsUpdate/mapType/biasNode |
| `LightProbe` | Present | LightProbe (src/lights/light_object.rs:288) | webgpu_lightprobe, webgpu_lightprobe_cubecamera (graded) |  |
| `IESSpotLight` | Absent | — | — | not ported |
| `ProjectorLight` | Absent | — | — | not ported |

## helpers

13 rows: 1 Present, 1 Partial, 11 Absent, 0 N.A.

| three.js name | verdict | three-rs symbol | verified by | note |
|---|---|---|---|---|
| `SpotLightHelper` | Absent | — | — | not ported |
| `SkeletonHelper` | Absent | — | — | not ported |
| `PointLightHelper` | Absent | — | — | not ported |
| `HemisphereLightHelper` | Absent | — | — | not ported |
| `GridHelper` | Present | GridHelper (src/helpers/grid_helper.rs:22) | tests/scene_webgpu_materials.rs; webgpu_materials, webgpu_particles |  |
| `PolarGridHelper` | Absent | — | — | not ported |
| `DirectionalLightHelper` | Absent | — | — | not ported |
| `CameraHelper` | Partial | CameraHelper (src/helpers/camera_helper.rs:27) | webgpu_camera (#[ignore]d, ungraded) | no graded check: webgpu_camera is ignored because three fails its own reference here |
| `BoxHelper` | Absent | — | — | not ported |
| `Box3Helper` | Absent | — | — | not ported |
| `PlaneHelper` | Absent | — | — | not ported |
| `ArrowHelper` | Absent | — | — | not ported |
| `AxesHelper` | Absent | — | — | not ported |

## audio

5 rows: 0 Present, 0 Partial, 0 Absent, 5 N.A.

| three.js name | verdict | three-rs symbol | verified by | note |
|---|---|---|---|---|
| `AudioListener` | N.A. | — | — | audio DOM |
| `PositionalAudio` | N.A. | — | — | audio DOM |
| `AudioContext` | N.A. | — | — | audio DOM |
| `AudioAnalyser` | N.A. | — | — | audio DOM |
| `Audio` | N.A. | — | — | audio DOM |

## materials

36 rows: 4 Present, 19 Partial, 11 Absent, 2 N.A.

| three.js name | verdict | three-rs symbol | verified by | note |
|---|---|---|---|---|
| `ShadowMaterial` | Absent | — | — | no shadow-catcher material / ShadowMaskModel; no definition (shadow_material src/materials/node_material.rs:175 is the shadow-pass override, not this) |
| `SpriteMaterial` | Present | MaterialKind::Sprite, MeshBasicNodeMaterial::sprite (src/materials/mod.rs:794) | tests/objects_sprite.rs, tests/renderer_sprites.rs; webgpu_sprites | same impl as SpriteNodeMaterial |
| `RawShaderMaterial` | N.A. | — | — | GLSL shaders |
| `ShaderMaterial` | N.A. | — | — | GLSL shaders |
| `PointsMaterial` | Partial | MaterialKind::Points, ::points (src/materials/mod.rs:811) | tests/renderer_points.rs; webgpu_instance_points, webgpu_compute_points | no size scalar (materialPointSize) or attenuated sprite-size branch (node_material.rs:880) |
| `MeshPhysicalMaterial` | Partial | MaterialKind::Physical, ::physical (src/materials/mod.rs:939) | webgpu_clearcoat, webgpu_loader_gltf_sheen, webgpu_furnace_test | no iridescence (#229), dispersion, sheen/diffuseRoughness maps, anisotropic direct BRDF; transmission off (#228) |
| `MeshStandardMaterial` | Partial | MaterialKind::Standard, ::standard (src/materials/mod.rs:925) | webgpu_lights_physical, webgpu_loader_gltf | no displacementMap, lightMap, per-material envMapIntensity/Rotation; raw envMap ignored (needs pmrem_env) |
| `MeshPhongMaterial` | Partial | MaterialKind::Phong, ::phong (src/materials/mod.rs:884) | webgpu_lights_phong, webgpu_shadowmap | no lightMap, specularMap, displacementMap; envMap and aoMap ignored (docs/api.md §7) |
| `MeshToonMaterial` | Partial | MaterialKind::Toon, ::toon (src/materials/mod.rs:914) | webgpu_materials_toon | no lightMap, displacementMap; aoMap ignored |
| `MeshNormalMaterial` | Partial | MaterialKind::Normal, ::normal (src/materials/mod.rs:874) | webgpu_materials | no displacementMap or normalMapType |
| `MeshLambertMaterial` | Partial | MaterialKind::Lambert, ::lambert (src/materials/mod.rs:898) | tests/room_environment.rs (kind only) | no graded render; no lightMap/specularMap/displacementMap, envMap/aoMap ignored |
| `MeshDepthMaterial` | Absent | — | — | not ported |
| `MeshDistanceMaterial` | Absent | — | — | not ported |
| `MeshBasicMaterial` | Partial | MeshBasicNodeMaterial (src/materials/mod.rs:204), MaterialKind::Basic | webgpu_materials_basic, webgpu_materials | no lightMap, specularMap, combine; aoMap ignored (api.md §7) |
| `MeshMatcapMaterial` | Absent | — | — | not ported |
| `LineDashedMaterial` | Absent | — | — | not ported; comment only: src/objects/line.rs:13 |
| `LineBasicMaterial` | Present | ::line (src/materials/mod.rs:838), LineBasicNodeMaterial alias (src/materials/mod.rs:981) | tests/renderer_lines.rs, tests/renderer_vertex_colors.rs; webgpu_modifier_curve | same impl as LineBasicNodeMaterial |
| `Material` | Partial | MeshBasicNodeMaterial (src/materials/mod.rs:204) carries Material fields | all graded examples | no clippingPlanes, stencil*, polygonOffset, dithering, shadowSide, toJSON; no QUnit port |
| `NodeMaterialObserver` | Absent | — | — | no change detection; set_needs_update + per-frame uniform upload instead |
| `NodeMaterial` | Partial | MeshBasicNodeMaterial node fields (src/materials/mod.rs:204), setup (src/materials/node_material.rs:413) | tests/nodes_*_wgsl.rs, webgpu_materials | no envNode, aoNode, backdropNode, geometryNode, receivedShadowNode |
| `LineBasicNodeMaterial` | Present | LineBasicNodeMaterial alias (src/materials/mod.rs:981), ::line (src/materials/mod.rs:838) | tests/renderer_lines.rs; webgpu_modifier_curve |  |
| `LineDashedNodeMaterial` | Absent | — | — | not ported; comment only: src/objects/line.rs:13 |
| `Line2NodeMaterial` | Partial | Line2NodeMaterial alias (src/materials/mod.rs:977), src/materials/line2.rs | tests/nodes_line2_layout.rs; webgpu_lines_fat, webgpu_lines_fat_raycasting | dashes not ported (line2.rs:23) |
| `MeshNormalNodeMaterial` | Partial | MeshNormalNodeMaterial alias (src/materials/mod.rs:973) | webgpu_materials | no displacementMap or normalMapType |
| `MeshBasicNodeMaterial` | Partial | MeshBasicNodeMaterial (src/materials/mod.rs:204) | webgpu_materials_basic, webgpu_materials | no lightMap, specularMap, combine; aoMap ignored (api.md §7) |
| `MeshLambertNodeMaterial` | Partial | MeshLambertNodeMaterial alias (src/materials/mod.rs:954) | tests/room_environment.rs (kind only) | no graded render; no lightMap/specularMap/displacementMap, envMap/aoMap ignored |
| `MeshPhongNodeMaterial` | Partial | MeshPhongNodeMaterial alias (src/materials/mod.rs:950), src/materials/phong.rs | webgpu_lights_phong | no lightMap, specularMap, displacementMap; envMap and aoMap ignored (docs/api.md §7) |
| `MeshStandardNodeMaterial` | Partial | MeshStandardNodeMaterial alias (src/materials/mod.rs:966), src/materials/physical.rs | webgpu_lights_physical, webgpu_deferred | no displacementMap, lightMap, per-material envMapIntensity/Rotation |
| `MeshPhysicalNodeMaterial` | Partial | MeshPhysicalNodeMaterial alias (src/materials/mod.rs:969), src/materials/physical.rs | webgpu_clearcoat | no iridescence (#229), dispersion, sheen/diffuseRoughness maps, anisotropic direct BRDF; transmission off (#228) |
| `MeshSSSNodeMaterial` | Absent | — | — | not ported |
| `MeshToonNodeMaterial` | Partial | MeshToonNodeMaterial alias (src/materials/mod.rs:958), src/materials/toon.rs | webgpu_materials_toon | no lightMap, displacementMap; aoMap ignored |
| `MeshMatcapNodeMaterial` | Absent | — | — | not ported |
| `PointsNodeMaterial` | Partial | PointsNodeMaterial alias (src/materials/mod.rs:963) | tests/nodes_compute_indirect_wgsl.rs; webgpu_instance_points, webgpu_compute_points | no materialPointSize scalar; attenuated sprite-size branch not ported |
| `SpriteNodeMaterial` | Present | SpriteNodeMaterial alias (src/materials/mod.rs:961), ::sprite (src/materials/mod.rs:794) | tests/renderer_sprites.rs; webgpu_sprites, webgpu_instance_sprites |  |
| `ShadowNodeMaterial` | Absent | — | — | no ShadowMaskModel |
| `VolumeNodeMaterial` | Absent | — | — | not ported |

## textures

18 rows: 8 Present, 4 Partial, 1 Absent, 5 N.A.

| three.js name | verdict | three-rs symbol | verified by | note |
|---|---|---|---|---|
| `VideoTexture` | N.A. | — | — | DOM video |
| `VideoFrameTexture` | N.A. | — | — | DOM video |
| `FramebufferTexture` | Partial | framebuffer_texture (src/nodes/display/viewport_texture.rs:53) | webgpu_backdrop (graded) | crate-private copy behind viewportSharedTexture; no public class or copyFramebufferToTexture |
| `DataTexture` | Present | DataTexture (src/textures/data_texture.rs:42), Texture::data_* (src/textures/texture.rs:314) | webgpu_materials_toon, webgpu_textures_partialupdate |  |
| `DataArrayTexture` | Partial | DataArrayTexture (src/textures/data_array_texture.rs:24) | tests/nodes_morph.rs; webgpu_morphtargets | rgba32float morph store only; no sampled/layer-update use |
| `Data3DTexture` | Present | Data3DTexture (src/textures/data3d_texture.rs:60) | webgpu_volume_perlin |  |
| `CompressedTexture` | Present | Texture::compressed (src/textures/compressed_texture.rs:33) | tests/ktx2_loader.rs; webgpu_loader_gltf_compressed |  |
| `CompressedArrayTexture` | Present | Texture::compressed_array (src/textures/compressed_texture.rs:58) | tests/ktx2_loader.rs; webgpu_textures_2d-array_compressed |  |
| `CompressedCubeTexture` | Partial | Ktx2Class::CompressedCubeTexture (src/loaders/ktx2_loader.rs:115) | tests/ktx2_loader.rs | parsed only; into_texture errors, no upload path (docs/nodes.md:2988) |
| `CubeTexture` | Present | CubeTexture (src/textures/cube_texture.rs:109) | webgpu_cubemap_mix, webgpu_materials_envmaps |  |
| `CanvasTexture` | N.A. | — | — | DOM canvas; no definition (FlakesTexture builds a Texture) |
| `HTMLTexture` | N.A. | — | — | DOM element |
| `DepthTexture` | Present | DepthTexture (src/textures/depth_texture.rs:89) | tests/nodes_shadow_filters.rs; webgpu_depth_texture |  |
| `CubeDepthTexture` | Present | CubeDepthTexture (src/textures/cube_depth_texture.rs:33) | webgpu_shadowmap_pointlight | internal point-shadow use |
| `ExternalTexture` | Partial | Texture::external (src/textures/texture.rs:262) | tests/renderer_external_device.rs | constructor on Texture; no graded/oracle check |
| `Texture` | Present | Texture (src/textures/texture.rs:151) | tests/renderer_textures.rs; webgpu_textures_2d, webgpu_materials_texture_manualmipmap | no QUnit port |
| `TextureSource` | Absent | — | — | no shared source; clone_texture re-uploads; comment only: src/textures/texture.rs:466 (nodes::TextureSource is unrelated) |
| `Source` | N.A. | — | — | deprecated alias; no definition (raw error.rs `source` is unrelated) |

## loaders

20 rows: 2 Present, 2 Partial, 12 Absent, 4 N.A.

| three.js name | verdict | three-rs symbol | verified by | note |
|---|---|---|---|---|
| `AnimationLoader` | Absent | — | — | not ported |
| `CompressedTextureLoader` | Absent | — | — | not ported (Ktx2Loader is not one) |
| `CubeTextureLoader` | Present | CubeTextureLoader (src/loaders/cube_texture_loader.rs:16) | webgpu_cubemap_mix, webgpu_materials_envmaps | synchronous, path-based |
| `DataTextureLoader` | Partial | folded into HdrLoader::load (src/loaders/hdr_loader.rs:113) | tests/hdr_loader.rs; webgpu_pmrem_equirectangular | no generic base loader |
| `TextureLoader` | Present | TextureLoader (src/loaders/texture_loader.rs:21) | tests/loaders_webp.rs; webgpu_textures_2d, webgpu_sprites | synchronous; no AVIF |
| `ObjectLoader` | Absent | — | — | not ported |
| `MaterialLoader` | Absent | — | — | not ported |
| `BufferGeometryLoader` | Partial | BufferGeometryLoader (src/loaders/buffer_geometry_loader.rs:13) | webgpu_instance_mesh | no interleaved attrs, morphAttributes, groups, drawRange, bounds |
| `DefaultLoadingManager` | Absent | — | — | loaders are synchronous; no manager |
| `LoadingManager` | Absent | — | — | loaders are synchronous; no manager |
| `ImageLoader` | N.A. | — | — | DOM image; comment only: src/loaders/cube_texture_loader.rs |
| `ImageBitmapLoader` | N.A. | — | — | DOM API; comment only: src/loaders/gif.rs |
| `FileLoader` | N.A. | — | — | browser fetch; no definition (src/io.rs is the byte seam) |
| `Loader` | Absent | — | — | no common Loader trait; no definition (per-loader set_path only) |
| `LoaderUtils` | Absent | — | — | not ported |
| `Cache` | Absent | — | — | not ported; no definition (raw builder `cache` is unrelated) |
| `AudioLoader` | N.A. | — | — | audio DOM |
| `NodeLoader` | Absent | — | — | no JSON node loading |
| `NodeObjectLoader` | Absent | — | — | no JSON node loading |
| `NodeMaterialLoader` | Absent | — | — | no JSON node loading |

## renderers

39 rows: 9 Present, 9 Partial, 3 Absent, 18 N.A.

| three.js name | verdict | three-rs symbol | verified by | note |
|---|---|---|---|---|
| `WebGLArrayRenderTarget` | N.A. | — | — | WebGL-only |
| `WebGL3DRenderTarget` | N.A. | — | — | WebGL-only |
| `WebGLRenderTarget` | N.A. | — | — | WebGL-only |
| `WebXRController` | N.A. | — | — | WebXR |
| `WebGLRenderer` | N.A. | — | — | WebGL-only |
| `WebGLCubeRenderTarget` | N.A. | — | — | WebGL-only |
| `ShaderLib` | N.A. | — | — | WebGL-only |
| `UniformsLib` | N.A. | — | — | WebGL-only |
| `UniformsUtils` | N.A. | — | — | WebGL-only |
| `ShaderChunk` | N.A. | — | — | WebGL-only |
| `WebGLUtils` | N.A. | — | — | WebGL-only |
| `WebGPURenderer` | Partial | Renderer (src/renderer/mod.rs:649) | tests/e2e/main.rs (all graded examples) | native backend fixed to Vulkan, no backend/powerPreference choice (#48) |
| `WebGPUBackend` | Present | folded into Renderer over wgpu (src/renderer/mod.rs:649) | tests/e2e/main.rs | no separate backend object; wgpu is the backend |
| `WebGLBackend` | N.A. | — | — | WebGL-only; comment only: src/renderer/mod.rs |
| `Renderer` | Partial | Renderer (src/renderer/mod.rs:649) | tests/e2e/main.rs, tests/renderer_*.rs | no clipping planes, copyFramebufferToTexture, compileAsync; caches never evict (#237) |
| `Backend` | N.A. | — | — | single backend |
| `WebGLCapabilities` | N.A. | — | — | WebGL-only |
| `Lighting` | Partial | Renderer.lighting_enabled (src/renderer/mod.rs:714) | webgpu_deferred, webgpu_lights_selective | flag only; no swappable Lighting object (TiledLighting) |
| `BundleGroup` | Absent | — | — | not ported; comment only: docs/scene-graph.md:453 |
| `QuadMesh` | Present | QuadMesh (src/objects/quad_mesh.rs:12), Renderer::render_quad | webgpu_rtt, webgpu_deferred, webgpu_depth_texture |  |
| `PMREMGenerator` | Present | PmremGenerator (src/renderer/pmrem.rs:174) | tests/pmrem.rs, tests/pmrem_scene.rs, tests/pmrem_equirect.rs; webgpu_pmrem_test, webgpu_pmrem_scene |  |
| `RenderPipeline` | Present | RenderPipeline (src/renderer/render_pipeline.rs:31) | tests/renderer_pipeline_hooks.rs; webgpu_postprocessing_bloom |  |
| `DirectRenderPipeline` | Present | DirectRenderPipeline (src/renderer/direct_render_pipeline.rs:36) | webgpu_postprocessing_direct |  |
| `PostProcessing` | N.A. | — | — | deprecated alias; comment only: src/renderer/render_pipeline.rs |
| `ReadbackBuffer` | Partial | Renderer::read_storage_buffer (src/renderer/mod.rs:3863) | tests/renderer_compute_points.rs | synchronous one-shot read; no reusable ReadbackBuffer object |
| `CubeRenderTarget` | Present | CubeRenderTarget (src/renderer/cube_render_target.rs:70) | webgpu_sky, webgpu_lightprobe_cubecamera (graded); tests/renderer_cube_camera.rs | layered cube target for CubeCamera plus from_equirectangular_texture |
| `StorageTexture` | Present | Texture::storage (src/textures/texture.rs:225), tsl::storage_texture (src/nodes/tsl.rs:3169) | webgpu_compute_texture |  |
| `Storage3DTexture` | Partial | Data3DTexture::storage (src/textures/data3d_texture.rs:103) | tests/nodes_texture_wgsl.rs (naga only) | no graded rung or oracle |
| `StorageArrayTexture` | Absent | — | — | not ported |
| `StorageBufferAttribute` | Partial | StorageArray via tsl::storage_data (src/nodes/tsl.rs:3708) | webgpu_compute_points | no attribute class; storage buffer is a node-side StorageArray |
| `StorageInstancedBufferAttribute` | Partial | StorageArray via tsl::instanced_array / storage_data (src/nodes/tsl.rs:3633) | webgpu_instance_points, webgpu_particles | no attribute class; storage buffer is a node-side StorageArray |
| `IndirectStorageBufferAttribute` | Present | IndirectStorageBufferAttribute (src/core/indirect_storage_buffer_attribute.rs:22) | tests/renderer_compute_indirect.rs; webgpu_struct_drawindirect |  |
| `InspectorBase` | Absent | — | — | not ported |
| `CanvasTarget` | Partial | private CanvasTarget (src/renderer/mod.rs:228) | tests/renderer_viewport.rs | private; one canvas per renderer, no setCanvasTarget |
| `BlendMode` | Partial | pub(crate) BlendMode (src/materials/blending.rs:124), MrtNode::set_blend_mode (src/nodes/mrt.rs:124) | tests/renderer_blending.rs | not public; MRT blend takes a Blending preset only |
| `GLSLNodeBuilder` | N.A. | — | — | WebGL-only |
| `BasicNodeLibrary` | N.A. | — | — | static dispatch |
| `StandardNodeLibrary` | N.A. | — | — | static dispatch |
| `WGSLNodeBuilder` | Present | NodeBuilder (src/nodes/builder.rs:650) | tests/nodes_*_wgsl.rs vs three dumps |  |

## nodes

141 rows: 95 Present, 28 Partial, 13 Absent, 5 N.A.

| three.js name | verdict | three-rs symbol | verified by | note |
|---|---|---|---|---|
| `NodeShaderStage` | Partial | Stage (src/nodes/builder.rs:50, pub(crate)) | all *_wgsl gates | crate-private; users cannot name it |
| `NodeUpdateType` | Present | NodeUpdateType (src/nodes/frame.rs:30) | tests/nodes_frame.rs |  |
| `NodeType` | Present | Type (src/nodes/node.rs:22) | all *_wgsl gates |  |
| `NodeAccess` | Present | StorageAccess (src/nodes/node.rs:943) | webgpu_compute_texture |  |
| `ArrayNode` | Partial | const_array / Node::ConstArray (src/nodes/tsl.rs:323) | webgpu_depth_texture; webgpu_postprocessing_godrays (graded, vec2 elements via const_array_of) | float or vector literals only; no array(type,count) or arrays of nodes |
| `AssignNode` | Present | Node::Assign (src/nodes/node.rs:1403) | webgpu_compute_points |  |
| `AttributeNode` | Present | Node::Attribute (src/nodes/node.rs:1346), attribute (src/nodes/tsl.rs:345) | all *_wgsl gates |  |
| `BypassNode` | Absent | — | — | no bypass() |
| `ConstNode` | Present | Node::Const (src/nodes/node.rs:1313) | all *_wgsl gates |  |
| `ContextNode` | Present | Node::Context (src/nodes/node.rs:1669), context (src/nodes/tsl.rs:145) | tests/nodes_custom.rs |  |
| `IndexNode` | Partial | Builtin::VertexIndex/InstanceIndex (src/nodes/node.rs:1053) | webgpu_compute_points | no drawIndex or subgroup indices |
| `InputNode` | Present | Node::Const/Uniform (src/nodes/node.rs:1313,1337) | all *_wgsl gates | role only (no base class) |
| `InspectorNode` | N.A. | — | — | DOM inspector |
| `IsolateNode` | Present | Node::Isolate (src/nodes/node.rs:1679), isolate (src/nodes/tsl.rs:158) | tests/nodes_custom.rs |  |
| `LightingModel` | Partial | trait LightingModel (src/materials/lighting_model.rs:119) | webgpu_lights_custom (ungraded) | no directRectArea/ambientOcclusion; only ungraded example |
| `MRTNode` | Present | MrtNode (src/nodes/mrt.rs:70) | tests/renderer_mrt.rs, webgpu_mrt |  |
| `Node` | Present | NodeRef / enum Node (src/nodes/node.rs:1687,1311) | all *_wgsl gates | name clashes with core::Node #250 |
| `NodeAttribute` | Present | AttributeSlot (src/nodes/builder.rs:180) | all *_wgsl gates |  |
| `NodeBuilder` | Present | NodeBuilder (src/nodes/builder.rs:650) | all *_wgsl gates |  |
| `NodeCache` | Present | NodeCache (src/nodes/builder.rs:439) | all *_wgsl gates | r187 nodeConstN sharing not followed #156 |
| `NodeCode` | Present | CodeDef / StageState.codes (src/nodes/code.rs:31, src/nodes/builder.rs:617) | webgpu_tsl_interoperability |  |
| `NodeError` | N.A. | — | — | JS diagnostics |
| `NodeFrame` | Present | NodeFrame (src/nodes/node.rs:459), frame.rs | tests/nodes_frame.rs | some passes not yet on NodeFrame #210 |
| `NodeFunctionInput` | Present | CodeDef.params (src/nodes/code.rs:31) | webgpu_tsl_interoperability |  |
| `NodeUniform` | Present | UniformMember (src/nodes/builder.rs:106) | all *_wgsl gates |  |
| `NodeVar` | Present | VarDef (src/nodes/node.rs:1111) | all *_wgsl gates |  |
| `NodeVarying` | Present | VaryingDef (src/nodes/node.rs:1124) | tests/nodes_wgsl_varying.rs |  |
| `OutputStructNode` | Present | MrtNode output struct (src/nodes/mrt.rs:70) | tests/renderer_mrt.rs |  |
| `OverrideContextNode` | Partial | OverrideNodes / with_override_nodes (src/nodes/tsl.rs:65,89) | webgpu_deferred | fixed to positionView/positionViewDirection/normalView |
| `ParameterNode` | Present | Node::Param (src/nodes/node.rs:1396) | webgpu_tsl_interoperability |  |
| `PropertyNode` | Present | Node::Property (src/nodes/node.rs:1389), property (src/nodes/tsl.rs:500) | all *_wgsl gates |  |
| `StackNode` | Present | Node::Block (src/nodes/node.rs:1523) + builder stacks | all *_wgsl gates |  |
| `StackTrace` | N.A. | — | — | JS diagnostics |
| `StructNode` | Absent | — | — | no struct value constructor; struct_type only declares storage layouts |
| `StructTypeNode` | Present | struct_type (src/nodes/tsl.rs:3837) | webgpu_struct_drawindirect |  |
| `SubBuildNode` | Present | in_sub_build (src/nodes/tsl.rs:168) | webgpu_materials |  |
| `TempNode` | Partial | increase_usage (src/nodes/builder.rs:845) | all *_wgsl gates | r187 let nodeConstN temp rule not followed #156 |
| `UniformGroupNode` | Present | UniformGroup (src/nodes/node.rs:130) | all *_wgsl gates |  |
| `UniformNode` | Present | UniformNode (src/nodes/node.rs:594), uniform (src/nodes/tsl.rs:438) | all *_wgsl gates |  |
| `VarNode` | Present | Node::Var/Let (src/nodes/node.rs:1380,1384), to_var (src/nodes/tsl.rs:454) | all *_wgsl gates |  |
| `VaryingNode` | Present | Node::Varying (src/nodes/node.rs:1386), to_varying (src/nodes/tsl.rs:489) | tests/nodes_wgsl_varying.rs |  |
| `BufferAttributeNode` | Partial | instanced_buffer_attribute (src/nodes/tsl.rs:4395), to_attribute (3758) | tests/nodes_instanced_attributes.rs | no plain/dynamic bufferAttribute() |
| `BufferNode` | Present | BufferNode (src/nodes/node.rs:869) | webgpu_skinning |  |
| `BuiltinNode` | Partial | enum Builtin (src/nodes/node.rs:1053) | tests/nodes_compute_wgsl.rs | closed enum; no generic builtin(name) |
| `ClippingNode` | Absent | — | — | no clippingPlanes; docs/nodes.md s6 deferred |
| `CubeTextureNode` | Present | cube_texture (src/nodes/tsl.rs:3581) | webgpu_materials_envmaps |  |
| `MaterialNode` | Partial | material_color etc. (src/nodes/tsl.rs:2143) | webgpu_lights_physical | no iridescence/lightMap/dash accessors |
| `MaterialReferenceNode` | Partial | UniformSource::Material* (src/nodes/node.rs:172) | webgpu_lights_physical | fixed sources; no materialReference(name) |
| `ModelNode` | Partial | model_world_matrix etc. (src/nodes/tsl.rs:2123,2133,2335) | all *_wgsl gates | no modelPosition/Scale/Direction/ViewPosition/Radius |
| `Object3DNode` | Partial | object_world_matrix (src/nodes/tsl.rs:3789) | webgpu_skinning_points | only objectWorldMatrix |
| `PointUVNode` | N.A. | — | — | GLSL-only |
| `ReferenceBaseNode` | Absent | — | — | no reference() |
| `ReferenceElementNode` | Absent | — | — | no referenceBuffer() |
| `ReferenceNode` | Partial | user_data (src/nodes/tsl.rs:419) | webgpu_sprites | no reference()/referenceBuffer() |
| `RendererReferenceNode` | Partial | tone_mapping_exposure (src/nodes/tsl.rs:1409) | tests/nodes_custom.rs | fixed exposure only; no rendererReference(name) |
| `StorageBufferNode` | Present | instanced_array (src/nodes/tsl.rs:3633) | webgpu_compute_points, webgpu_particles |  |
| `StorageTexture3DNode` | Present | storage_texture_3d (src/nodes/tsl.rs:3183) | tests/nodes_texture_wgsl.rs |  |
| `StorageTextureNode` | Present | StorageTextureNode (src/nodes/tsl.rs:3164) | webgpu_compute_texture |  |
| `Texture3DNode` | Present | Texture3DNode (src/nodes/tsl.rs:3086) | webgpu_volume_perlin, tests/nodes_texture_wgsl.rs |  |
| `TextureNode` | Present | Node::Texture (src/nodes/node.rs:1469), texture (src/nodes/tsl.rs:3056) | all *_wgsl gates |  |
| `TextureSizeNode` | Present | Node::TextureSize (src/nodes/node.rs:1480), texture_size (src/nodes/tsl.rs:4303) | tests/nodes_display_wgsl.rs, webgpu_mrt |  |
| `UniformArrayNode` | Partial | UniformArray (src/nodes/tsl.rs:4013) | webgpu_postprocessing_bloom, webgpu_postprocessing_fxaa | f32/vec3 elements only |
| `UserDataNode` | Present | user_data (src/nodes/tsl.rs:419) | webgpu_sprites |  |
| `VelocityNode` | Present | velocity (src/nodes/velocity.rs:64) | webgpu_postprocessing_motion_blur (graded); tests/velocity_frames.rs | previous model, camera and bone matrices tracked per frame; issue 163 closed |
| `VertexColorNode` | Present | vertex_color (src/nodes/tsl.rs:2020) | webgpu_lines_fat, tests/renderer_vertex_colors.rs | no white fallback (deliberate, docs/nodes.md s8) |
| `CodeNode` | Present | Node::Code (src/nodes/node.rs:1510), wgsl_fn (src/nodes/code.rs:59) | webgpu_tsl_interoperability |  |
| `ExpressionNode` | Partial | Node::Discard/Return/Break (src/nodes/node.rs:1601,1605,1553) | webgpu_compute_points | no expression(snippet); no Continue |
| `FunctionCallNode` | Present | Node::CodeCall (src/nodes/node.rs:1513), call_wgsl (src/nodes/tsl.rs:4443) | webgpu_tsl_interoperability |  |
| `FunctionNode` | Present | emit_code_fn (src/nodes/builder.rs:2766) | webgpu_tsl_interoperability |  |
| `BumpMapNode` | Present | bump_map (src/nodes/tsl.rs:3312) | webgpu_tsl_earth, webgpu_lights_physical |  |
| `ColorSpaceNode` | Partial | srgb_to_working (src/nodes/tsl.rs:4654), render_output | tests/nodes_custom.rs | sRGB/linear only; no convertColorSpace or Display-P3 |
| `FrontFacingNode` | Present | front_facing / Builtin::FrontFacing (src/nodes/tsl.rs:2064) | webgpu_loader_gltf_compressed |  |
| `NormalMapNode` | Partial | normal_map (src/nodes/tsl.rs:1456) | webgpu_lights_phong, webgpu_clearcoat | tangent space only; no object-space normal map |
| `PassNode` | Present | pass / PassNode (src/renderer/pass.rs:117,54) | webgpu_postprocessing_bloom |  |
| `RenderOutputNode` | Present | render_output (src/materials/node_material.rs:1040) | tests/nodes_custom.rs |  |
| `ScreenNode` | Partial | screen_uv, viewport_size (src/nodes/tsl.rs:2362,2301) | webgpu_mrt, webgpu_tsl_halftone | no viewportUV/viewportCoordinate |
| `ToneMappingNode` | Partial | tone_mapping_node (src/materials/node_material.rs:1058) | tests/nodes_custom.rs, webgpu_custom_fog_background | no Cineon or Custom tone mapping |
| `ToonOutlinePassNode` | Present | toon_outline_pass (src/nodes/display/toon_outline_pass.rs:40) | webgpu_materials_toon |  |
| `ViewportDepthNode` | Partial | perspective_depth_to_view_z (src/nodes/tsl.rs:845) | webgpu_depth_texture | no viewportDepth/linearDepth #169 |
| `ViewportDepthTextureNode` | Partial | viewport_depth_texture (src/nodes/display/viewport_texture.rs:173) | — | defined; no gate or graded example reads the depth copy yet |
| `ViewportSharedTextureNode` | Present | ViewportTextureNode, Framebuffer::Shared (src/nodes/display/viewport_texture.rs:70) | webgpu_backdrop (graded); tests/nodes_display_wgsl.rs refraction_backdrop_matches_three | issue 169 closed |
| `ViewportTextureNode` | Present | ViewportTextureNode (src/nodes/display/viewport_texture.rs:70) | webgpu_backdrop (graded) | one node type behind viewportTexture, viewportSharedTexture and viewportDepthTexture; transmission's viewportOpaqueMipTexture stays separate |
| `RangeNode` | Present | instanced_range (src/materials/node_material.rs:1017) | tests/nodes_range_buffers.rs, webgpu_instance_mesh |  |
| `AtomicFunctionNode` | Present | Node::Atomic (src/nodes/node.rs:1646), atomic_add (src/nodes/tsl.rs:3942) | webgpu_struct_drawindirect, tests/nodes_compute_wgsl.rs |  |
| `BarrierNode` | Present | Node::Barrier (src/nodes/node.rs:1659), workgroup_barrier (src/nodes/tsl.rs:3997) | tests/nodes_compute_indirect_wgsl.rs |  |
| `ComputeBuiltinNode` | Present | Builtin::WorkgroupId/LocalId/GlobalId/NumWorkgroups (src/nodes/node.rs:1053) | tests/nodes_compute_wgsl.rs |  |
| `ComputeNode` | Present | compute_node (src/nodes/tsl.rs:3778) | webgpu_compute_points |  |
| `SubgroupFunctionNode` | Absent | — | — | no subgroup ops; comment only: src/nodes/builder.rs:3375 |
| `WorkgroupInfoNode` | Present | Node::Workgroup (src/nodes/node.rs:1656), workgroup_array (src/nodes/tsl.rs:3967) | tests/nodes_compute_indirect_wgsl.rs |  |
| `AmbientLightNode` | Present | ambient_lights (src/materials/phong.rs:386,223) | webgpu_materials_toon |  |
| `AnalyticLightNode` | Present | setup_light (src/materials/phong.rs:214) | webgpu_lights_phong |  |
| `AONode` | Partial | aoMap path (src/materials/physical.rs:870) | webgpu_loader_gltf | PBR only; ao_map loud on Basic/Phong |
| `BasicEnvironmentNode` | Partial | env_map on MeshBasic (src/materials/environment.rs) | webgpu_materials_envmaps | MeshBasic only; env_map loud on Phong/Lambert |
| `BasicLightMapNode` | Absent | — | — | no lightMap |
| `DirectionalLightNode` | Present | setup_light Directional arm (src/materials/phong.rs:280) | webgpu_shadowmap |  |
| `EnvironmentNode` | Present | EnvironmentNode (src/materials/environment.rs:94) | webgpu_pmrem_scene |  |
| `HemisphereLightNode` | Present | setup_light Hemisphere arm (src/materials/phong.rs:229) | webgpu_lights_physical |  |
| `IESSpotLightNode` | Absent | — | — | no IES lights |
| `IrradianceNode` | Absent | — | — | no lightMap |
| `LightingContextNode` | Present | lighting context (src/materials/node_material.rs:1509) | webgpu_lights_physical | role only |
| `LightingNode` | Present | setup_light / lights_node (src/materials/phong.rs:214) | webgpu_lights_phong | base role only |
| `LightProbeNode` | Present | BufferSource::LightProbe (src/nodes/node.rs:739), LightKind::Probe in lights_node (src/materials/node_material.rs:1156) | webgpu_lightprobe (graded); tests/nodes_light_probe.rs |  |
| `LightsNode` | Present | lights_node (src/materials/lighting_model.rs:140) | webgpu_lights_physical |  |
| `PointLightNode` | Present | setup_light Point arm (src/materials/phong.rs:255) | webgpu_lights_physical |  |
| `PointShadowNode` | Present | point_shadow (src/lights/point_shadow.rs:157) | webgpu_shadowmap_pointlight |  |
| `ProjectorLightNode` | Absent | — | — | no ProjectorLight |
| `RectAreaLightNode` | Absent | — | — | no RectAreaLight |
| `ShadowBaseNode` | Present | shadow_node (src/materials/phong.rs:171) | webgpu_shadowmap | role merged into ShadowNode |
| `ShadowNode` | Present | shadow_node (src/materials/phong.rs:171), shadow_filter.rs | webgpu_shadowmap, tests/nodes_shadow_filters.rs |  |
| `SpotLightNode` | Present | setup_light Spot arm (src/materials/phong.rs:264) | webgpu_shadowmap, webgpu_shadowmap_vsm |  |
| `BitcastNode` | Present | bitcast / float_bits_to_int (src/nodes/tsl/wrappers.rs:454,475) | tests/nodes_tsl_batch.rs |  |
| `BitcountNode` | Present | count_one_bits (src/nodes/tsl/wrappers.rs:1114) | tests/nodes_tsl_batch.rs |  |
| `ConditionalNode` | Present | Node::Select/IfVar (src/nodes/node.rs:1624,1590), select (src/nodes/tsl.rs:1835) | webgpu_compute_points |  |
| `MathNode` | Present | Node::Math (src/nodes/node.rs:1421) | all *_wgsl gates |  |
| `OperatorNode` | Present | Node::Op (src/nodes/node.rs:1410) | all *_wgsl gates |  |
| `PackFloatNode` | Present | pack_snorm_2x16 … unpack_unorm_4x8 (src/nodes/tsl/wrappers.rs:589), generate_packing (src/nodes/builder.rs) | tests/nodes_tsl_batch.rs |  |
| `Packed4x8IntegerNode` | Present | pack_4x_i8 … dot_4i8_packed (src/nodes/tsl/wrappers.rs:614), generate_packing (src/nodes/builder.rs) | tests/nodes_tsl_batch.rs | native builtins only; the no-feature emulation is not ported |
| `UnpackFloatNode` | Present | unpack_snorm_2x16 … unpack_unorm_4x8 (src/nodes/tsl/wrappers.rs:589), generate_packing (src/nodes/builder.rs) | tests/nodes_tsl_batch.rs |  |
| `GLSLNodeParser` | N.A. | — | — | GLSL-only |
| `PMREMNode` | Present | pmrem_texture (src/nodes/pmrem_node.rs:157) | tests/pmrem*.rs, webgpu_pmrem_scene |  |
| `ArrayElementNode` | Present | Node::Element (src/nodes/node.rs:1460), element (src/nodes/tsl.rs:1796) | webgpu_compute_points |  |
| `ConvertNode` | Present | Node::Cast (src/nodes/node.rs:1439), convert_type (src/nodes/tsl/wrappers.rs:77) | all *_wgsl gates |  |
| `CubeMapNode` | Present | from_equirectangular_texture (src/renderer/cube_render_target.rs:61) | webgpu_materials_envmaps |  |
| `DebugNode` | Absent | — | — | no debug() |
| `EventNode` | Partial | NodeUpdate trait (src/nodes/frame.rs:51), on_before_render (src/renderer/render_pipeline.rs:84) | tests/renderer_pipeline_hooks.rs | no OnObjectUpdate/OnMaterialUpdate/OnFrameUpdate TSL |
| `FlipNode` | Present | flip_y (src/nodes/tsl.rs:1859) | webgpu_materials |  |
| `FunctionOverloadingNode` | Partial | by_position (src/nodes/mx_noise.rs:833) | tests/nodes_mx_noise.rs | internal MaterialX use only; no overloadingFn() |
| `JoinNode` | Present | Node::Join (src/nodes/node.rs:1453), vec4 (src/nodes/tsl.rs:298) | all *_wgsl gates |  |
| `LoopNode` | Present | Node::Loop (src/nodes/node.rs:1530), loop_index (src/nodes/tsl.rs:4266) | webgpu_volume_perlin, webgpu_tsl_raging_sea |  |
| `MaxMipLevelNode` | Present | max_mip_level (src/nodes/tsl.rs:4039) | tests/nodes_tsl_batch.rs | a live object uniform, read when the buffer is written |
| `MemberNode` | Present | Node::StructMember (src/nodes/node.rs:1636), .get (src/nodes/tsl.rs:3904) | webgpu_struct_drawindirect |  |
| `ReflectorNode` | Present | ReflectorNode (src/nodes/reflector_node.rs:183) | webgpu_mirror |  |
| `RotateNode` | Present | rotate (src/nodes/tsl.rs:733) | tests/nodes_tsl_batch.rs, webgpu_layers |  |
| `RTTNode` | Present | rtt (src/nodes/display/rtt.rs:73) | webgpu_procedural_texture |  |
| `SampleNode` | Absent | — | — | no sample() |
| `SetNode` | Partial | set_z (src/nodes/tsl.rs:1779) | webgpu_depth_texture | only .set_z; no general set() |
| `SplitNode` | Present | Node::Swizzle (src/nodes/node.rs:1430) | all *_wgsl gates |  |
| `StorageArrayElementNode` | Present | StorageArray.element (src/nodes/tsl.rs:3644) | webgpu_compute_points |  |
| `PhongLightingModel` | Present | brdf_blinn_phong (src/materials/phong.rs:90) | webgpu_lights_phong |  |
| `PhysicalLightingModel` | Partial | direct_light (src/materials/physical.rs:915) | webgpu_lights_physical, webgpu_loader_gltf | no iridescence, dispersion, or direct anisotropic GGX |
| `NodeUtils` | Present | align_of/size_of (src/nodes/wgsl.rs:453,463), CacheKey (src/nodes/builder.rs:409) | all *_wgsl gates | role only |

## addons/controls

9 rows: 1 Present, 3 Partial, 5 Absent, 0 N.A.

| three.js name | verdict | three-rs symbol | verified by | note |
|---|---|---|---|---|
| `ArcballControls` | Absent | — | — |  |
| `DragControls` | Absent | — | — |  |
| `FirstPersonControls` | Present | addons::controls::FirstPersonControls (src/addons/controls/first_person_controls.rs:53) | tests/addons_first_person_controls.rs (three's class under node, 7 scenarios, 1e-9) | DOM listener bookkeeping (connect/disconnect/dispose, contextmenu) replaced by input methods; see docs/controls.md |
| `FlyControls` | Partial | addons::controls::FlyControls (src/addons/controls/fly_controls.rs:88) | tests/addons_fly_controls.rs (three's class under node, 5 scenarios, 1e-9) | no change event (update() returns its condition); DOM listener bookkeeping replaced by input methods; the `domElement === document` branch of _getContainerDimensions (window.innerWidth/innerHeight, zero offset) dropped, the host passes the size via set_element_size |
| `MapControls` | Partial | three_rs_controls::MapControls (addons/controls/src/map_controls.rs:216) | addons/controls unit tests | own ground-grab design, not a port; JS preset (OrbitControls screenSpacePanning=false, LEFT=PAN) not provided |
| `OrbitControls` | Partial | addons::controls::OrbitControls (src/addons/controls/orbit_controls.rs:209) | tests/addons_orbit_controls.rs; webgpu_loader_gltf et al. | no touch gestures, no OrthographicCamera branch, no change/start/end events |
| `PointerLockControls` | Absent | — | — |  |
| `TrackballControls` | Absent | — | — |  |
| `TransformControls` | Absent | — | — | comment only: examples/webgpu_modifier_curve.rs:20 ("Not ported") |

## addons/loaders

71 rows: 6 Present, 2 Partial, 63 Absent, 0 N.A.

| three.js name | verdict | three-rs symbol | verified by | note |
|---|---|---|---|---|
| `3DMLoader` | Absent | — | — |  |
| `3MFLoader` | Absent | — | — |  |
| `AMFLoader` | Absent | — | — |  |
| `BVHLoader` | Absent | — | — |  |
| `ColladaLoader` | Absent | — | — |  |
| `DDSLoader` | Absent | — | — |  |
| `DRACOLoader` | Partial | loaders::draco (src/loaders/draco.rs:1, doc(hidden)) | tests/gltf_draco.rs | glTF-driven half only; no standalone .drc load/parse to BufferGeometry |
| `EXRLoader` | Absent | — | — |  |
| `FBXLoader` | Absent | — | — |  |
| `FontLoader` | Present | FontLoader, Font (src/loaders/font_loader.rs:14) | tests/geometries_shape_oracle.rs; webgpu_materials_toon |  |
| `GCodeLoader` | Absent | — | — |  |
| `GLTFGaussianSplatLoaderExtension` | Absent | — | — |  |
| `GLTFLoader` | Partial | GltfLoader (src/loaders/gltf_loader.rs:456) | tests/gltf_loader.rs, gltf_draco.rs, gltf_meshopt.rs; webgpu_loader_gltf | no cameras, KHR_lights_punctual, CUBICSPLINE (linearised), AVIF, primitive modes/dedup, unlit/iridescence/dispersion/instancing exts; #229 #230 |
| `GaussianSplatPLYLoader` | Absent | — | — |  |
| `HDRCubeTextureLoader` | Present | HdrCubeTextureLoader (src/loaders/hdr_cube_texture_loader.rs:38) | tests/hdr_loader.rs; webgpu_pmrem_cubemap |  |
| `HDRLoader` | Present | HdrLoader (src/loaders/hdr_loader.rs:75) | tests/hdr_loader.rs (bit-exact oracle); webgpu_pmrem_test |  |
| `IESLoader` | Absent | — | — |  |
| `KMZLoader` | Absent | — | — |  |
| `KSPLATLoader` | Absent | — | — |  |
| `KTX2Loader` | Present | Ktx2Loader (src/loaders/ktx2_loader.rs:103) | tests/ktx2_loader.rs (byte oracle); webgpu_loader_gltf_compressed | cube/3D results parse but into_texture refuses them (no renderer path) |
| `KTXLoader` | Absent | — | — |  |
| `LDrawLoader` | Absent | — | — |  |
| `LUT3dlLoader` | Absent | — | — |  |
| `LUTCubeLoader` | Absent | — | — |  |
| `LUTImageLoader` | Absent | — | — |  |
| `LWOLoader` | Absent | — | — |  |
| `MD2Loader` | Absent | — | — |  |
| `MDDLoader` | Absent | — | — |  |
| `MTLLoader` | Absent | — | — |  |
| `MaterialXLoader` | Absent | — | — | mx_* TSL library exists (src/nodes/materialx), loader does not |
| `NRRDLoader` | Absent | — | — |  |
| `OBJLoader` | Absent | — | — |  |
| `PCDLoader` | Absent | — | — |  |
| `PDBLoader` | Absent | — | — |  |
| `PLYLoader` | Absent | — | — |  |
| `PVRLoader` | Absent | — | — |  |
| `RGBELoader` | Present | HdrLoader (src/loaders/hdr_loader.rs:75) | tests/hdr_loader.rs | deprecated alias of HDRLoader (r180); no RgbeLoader alias, use HdrLoader |
| `SPLATLoader` | Absent | — | — |  |
| `SPZLoader` | Absent | — | — |  |
| `STLLoader` | Absent | — | — |  |
| `SVGLoader` | Absent | — | — |  |
| `TDSLoader` | Absent | — | — |  |
| `TGALoader` | Absent | — | — |  |
| `TIFFLoader` | Absent | — | — |  |
| `TTFLoader` | Absent | — | — | sdf-text reads TTF via ttf-parser but is not a TTFLoader |
| `USDLoader` | Absent | — | — |  |
| `USDZLoader` | Absent | — | — |  |
| `UltraHDRLoader` | Present | UltraHdrLoader (src/loaders/ultra_hdr_loader.rs:190) | webgpu_loader_gltf, webgpu_custom_fog_background | ICC profile ignored (as upstream) |
| `VOXLoader` | Absent | — | — |  |
| `VRMLLoader` | Absent | — | — |  |
| `VTKLoader` | Absent | — | — |  |
| `XYZLoader` | Absent | — | — |  |
| `ColladaComposer` | Absent | — | — |  |
| `ColladaParser` | Absent | — | — |  |
| `IFFParser` | Absent | — | — |  |
| `LWO2Parser` | Absent | — | — |  |
| `LWO3Parser` | Absent | — | — |  |
| `MaterialXArchive` | Absent | — | — |  |
| `MaterialXDocument` | Absent | — | — |  |
| `MaterialXHextile` | Absent | — | — |  |
| `MaterialXInterfaceValidation` | Absent | — | — |  |
| `MaterialXLog` | Absent | — | — |  |
| `MaterialXNodeInterfaceRegistry` | Absent | — | — |  |
| `MaterialXNodeLibrary` | Absent | — | — |  |
| `MaterialXSurfaceMappings` | Absent | — | — |  |
| `MaterialXUtils` | Absent | — | — |  |
| `MaterialXCompileRegistry` | Absent | — | — |  |
| `MaterialXParser` | Absent | — | — |  |
| `USDAParser` | Absent | — | — |  |
| `USDCParser` | Absent | — | — |  |
| `USDComposer` | Absent | — | — |  |

## addons/postprocessing

30 rows: 0 Present, 0 Partial, 0 Absent, 30 N.A.

| three.js name | verdict | three-rs symbol | verified by | note |
|---|---|---|---|---|
| `AfterimagePass` | N.A. | — | — | WebGL composer; AfterimagePass -> AfterImageNode afterImage(): Present; after_image (src/nodes/display/after_image.rs:37); tests/nodes_display_wgsl.rs; webgpu_postprocessing_afterimage (ungraded) |
| `BloomPass` | N.A. | — | — | WebGL composer; BloomPass -> nearest BloomNode bloom(): Present; bloom (src/nodes/display/bloom.rs:219); webgpu_postprocessing_bloom |
| `BokehPass` | N.A. | — | — | WebGL composer; BokehPass -> DepthOfFieldNode dof(): Absent |
| `ClearPass` | N.A. | — | — | WebGL composer; ClearPass -> no TSL node (renderer clear / autoClear): Present; Renderer clear (src/renderer); webgpu_postprocessing_masking |
| `CubeTexturePass` | N.A. | — | — | WebGL composer; CubeTexturePass -> no TSL node (scene.background cube): Present; CubeTexture background; webgpu_materials_envmaps |
| `DotScreenPass` | N.A. | — | — | WebGL composer; DotScreenPass -> DotScreenNode dotScreen(): Present; dot_screen (src/nodes/display/dot_screen.rs:15); tests/nodes_display_wgsl.rs; webgpu_postprocessing (ungraded) |
| `EffectComposer` | N.A. | — | — | WebGL composer; EffectComposer -> RenderPipeline (core): Present; RenderPipeline (src/renderer/render_pipeline.rs:31); webgpu_postprocessing_masking |
| `FXAAPass` | N.A. | — | — | WebGL composer; FXAAPass -> FXAANode fxaa(): Present; fxaa (src/nodes/display/fxaa.rs:26); webgpu_postprocessing_fxaa |
| `FilmPass` | N.A. | — | — | WebGL composer; FilmPass -> FilmNode film(): Absent |
| `GTAOPass` | N.A. | — | — | WebGL composer; GTAOPass -> GTAONode ao(): Absent |
| `GlitchPass` | N.A. | — | — | WebGL composer; GlitchPass -> no TSL counterpart: Absent |
| `HalftonePass` | N.A. | — | — | WebGL composer; HalftonePass -> no TSL node (inline TSL in webgpu_tsl_halftone, graded); webgpu_tsl_halftone |
| `LUTPass` | N.A. | — | — | WebGL composer; LUTPass -> Lut3DNode lut3D(): Absent |
| `MaskPass` | N.A. | — | — | WebGL composer; MaskPass -> pass().a mix pattern: Present; PassNode (src/renderer/pass.rs:54); webgpu_postprocessing_masking |
| `OutlinePass` | N.A. | — | — | WebGL composer; OutlinePass -> OutlineNode outline(): Absent |
| `OutputPass` | N.A. | — | — | WebGL composer; OutputPass -> renderOutput() (core): Present; render_output (src/materials/node_material.rs:1040); webgpu_postprocessing_bloom_emissive |
| `Pass` | N.A. | — | — | WebGL composer; Pass -> PassNode/TempNode (core): Present; PassNode (src/renderer/pass.rs:54); webgpu_postprocessing_masking |
| `RenderPass` | N.A. | — | — | WebGL composer; RenderPass -> pass(scene,camera) (core): Present; pass (src/renderer/pass.rs:117); webgpu_postprocessing_masking |
| `RenderPixelatedPass` | N.A. | — | — | WebGL composer; RenderPixelatedPass -> PixelationPassNode pixelationPass(): Present (WGSL gate only, no graded rung); pixelation_pass (src/nodes/display/pixelation_pass.rs:35); tests/nodes_display_wgsl.rs |
| `RenderTransitionPass` | N.A. | — | — | WebGL composer; RenderTransitionPass -> TransitionNode transition(): Present; transition (src/nodes/display/transition.rs:17); webgpu_postprocessing_transition |
| `SAOPass` | N.A. | — | — | WebGL composer; SAOPass -> SSAONode/GTAONode: Absent |
| `SMAAPass` | N.A. | — | — | WebGL composer; SMAAPass -> SMAANode smaa(): Absent |
| `SSAARenderPass` | N.A. | — | — | WebGL composer; SSAARenderPass -> SSAAPassNode ssaaPass(): Present; SsaaPassNode (src/renderer/ssaa_pass.rs:130); webgpu_postprocessing_ssaa |
| `SSAOPass` | N.A. | — | — | WebGL composer; SSAOPass -> SSAONode ssao(): Absent |
| `SSRPass` | N.A. | — | — | WebGL composer; SSRPass -> SSRNode ssr(): Absent |
| `SavePass` | N.A. | — | — | WebGL composer; SavePass -> no TSL node (rtt()/convertToTexture()): Present; rtt (src/nodes/display/rtt.rs:73); webgpu_postprocessing_anamorphic |
| `ShaderPass` | N.A. | — | — | WebGL composer; ShaderPass -> no TSL node (Fn on RenderPipeline.outputNode): Present; RenderPipeline (src/renderer/render_pipeline.rs:31); webgpu_postprocessing_radial_blur |
| `TAARenderPass` | N.A. | — | — | WebGL composer; TRAANode traa() is the WebGPU counterpart (Present) |
| `TexturePass` | N.A. | — | — | WebGL composer; TexturePass -> no TSL node (texture() in outputNode): Present; RenderPipeline (src/renderer/render_pipeline.rs:31); webgpu_postprocessing_masking |
| `UnrealBloomPass` | N.A. | — | — | WebGL composer; UnrealBloomPass -> BloomNode bloom(): Present; bloom (src/nodes/display/bloom.rs:219); webgpu_postprocessing_bloom |

## addons/other

102 rows: 11 Present, 4 Partial, 64 Absent, 23 N.A.

| three.js name | verdict | three-rs symbol | verified by | note |
|---|---|---|---|---|
| `BoxLineGeometry` | Absent | — | — |  |
| `ConvexGeometry` | Absent | — | — |  |
| `DecalGeometry` | Absent | — | — |  |
| `LoftGeometry` | Absent | — | — |  |
| `ParametricFunctions` | Absent | — | — |  |
| `ParametricGeometry` | Absent | — | — |  |
| `RoundedBoxGeometry` | Present | rounded_box_geometry (src/geometries/rounded_box.rs:98) | tests/geometry/rounded_box.rs (bit-exact) |  |
| `TeapotGeometry` | Present | teapot_geometry (src/geometries/teapot.rs:105) | tests/geometry/teapot.rs; webgpu_materials |  |
| `TextGeometry` | Present | text_geometry, TextGeometryOptions (src/addons/text_geometry.rs:51) | tests/geometries_shape_oracle.rs; webgpu_materials_toon |  |
| `Line2` | N.A. | — | — | WebGL-only; LineMaterial GLSL pairing, webgpu/Line2 is the port; counterpart addons::lines::Line2 (src/addons/lines.rs:250) |
| `LineGeometry` | Partial | addons::lines::LineGeometry (src/addons/lines.rs:173) | tests/nodes_line2_layout.rs; webgpu_lines_fat | no setFromPoints, fromLine |
| `LineMaterial` | N.A. | — | — | GLSL-only; Line2NodeMaterial is the WebGPU counterpart (dashes not ported); counterpart Line2NodeMaterial (src/materials/mod.rs:977) |
| `LineSegments2` | N.A. | — | — | WebGL-only; webgpu/LineSegments2 is the port; counterpart addons::lines::LineSegments2 (src/addons/lines.rs:214) |
| `LineSegmentsGeometry` | Partial | addons::lines::LineSegmentsGeometry (src/addons/lines.rs:36) | tests/addons_lines_raycast.rs; webgpu_lines_fat_raycasting | no fromWireframeGeometry/fromEdgesGeometry/fromMesh/fromLineSegments, applyMatrix4, toJSON |
| `Wireframe` | N.A. | — | — | WebGL-only; no WebGPU Wireframe either |
| `WireframeGeometry2` | Absent | — | — | needs LineSegmentsGeometry.fromWireframeGeometry and core WireframeGeometry, both absent |
| `Line2` | Present | addons::lines::Line2 (src/addons/lines.rs:250) | webgpu_lines_fat, webgpu_lines_fat_raycasting |  |
| `LineSegments2` | Partial | addons::lines::LineSegments2 (src/addons/lines.rs:214) | tests/addons_lines_raycast.rs; webgpu_lines_fat_raycasting | no computeLineDistances (no dashes); #5 |
| `Wireframe` | Absent | — | — |  |
| `GaussianSplat` | Absent | — | — |  |
| `GroundedSkybox` | Absent | — | — |  |
| `Lensflare` | N.A. | — | — | WebGL-only; LensflareMesh counterpart Absent |
| `LensflareMesh` | Absent | — | — |  |
| `MarchingCubes` | Absent | — | — |  |
| `Reflector` | N.A. | — | — | WebGL-only; TSL reflector() Present; counterpart reflector() / ReflectorNode (src/nodes/reflector_node.rs:183); webgpu_mirror |
| `ReflectorForSSRPass` | N.A. | — | — | WebGL-only (SSRPass helper) |
| `Refractor` | N.A. | — | — | WebGL-only |
| `ShadowMesh` | Absent | — | — |  |
| `Sky` | N.A. | — | — | WebGL-only; SkyMesh is the WebGPU counterpart (Present) |
| `SkyMesh` | Present | SkyMesh (src/addons/objects/sky_mesh.rs:44) | webgpu_sky (graded); tests/nodes_sky_wgsl.rs |  |
| `Water` | N.A. | — | — | WebGL-only; WaterMesh counterpart Absent |
| `Water2` | N.A. | — | — | WebGL-only; Water2Mesh counterpart Absent |
| `Water2Mesh` | Absent | — | — |  |
| `WaterMesh` | Absent | — | — |  |
| `BufferGeometryUtils` | Absent | — | — |  |
| `CameraUtils` | Absent | — | — |  |
| `ColorUtils` | Absent | — | — |  |
| `GaussianSplatUtils` | Absent | — | — |  |
| `GeometryCompressionUtils` | Absent | — | — |  |
| `GeometryUtils` | Partial | hilbert_3d (src/addons/geometry_utils.rs:20) | tests/extras_spline_oracle.rs; webgpu_lines_fat | no hilbert2D, gosper |
| `LDrawUtils` | Absent | — | — |  |
| `SceneOptimizer` | Absent | — | — |  |
| `SceneUtils` | Absent | — | — |  |
| `ShadowMapViewer` | N.A. | — | — | WebGL-only |
| `ShadowMapViewerGPU` | Absent | — | — |  |
| `SkeletonUtils` | Absent | — | — |  |
| `SortUtils` | Present | radix_sort (src/utils/sort_utils.rs:153) | webgpu_mesh_batch |  |
| `UVsDebug` | N.A. | — | — | DOM canvas |
| `WebGLTextureUtils` | N.A. | — | — | WebGL-only |
| `WebGPUTextureUtils` | Absent | — | — | addon decompress() absent; comment only (core WebGPUTextureUtils refs): src/renderer/bindings.rs |
| `WorkerPool` | N.A. | — | — | Web Workers |
| `CurveModifier` | N.A. | — | — | WebGL-only (onBeforeCompile GLSL); counterpart Flow (src/addons/curve_modifier_gpu.rs:259) |
| `CurveModifierGPU` | Present | addons::curve_modifier_gpu::Flow (src/addons/curve_modifier_gpu.rs:259) | tests/geometries_shape_oracle.rs; webgpu_modifier_curve | Flow takes one mesh, not an Object3D clone |
| `EdgeSplitModifier` | Absent | — | — |  |
| `SimplifyModifier` | Absent | — | — |  |
| `TessellateModifier` | Absent | — | — |  |
| `Capsule` | Absent | — | — | raw hit is core CapsuleGeometry, not the math Capsule |
| `ColorConverter` | Absent | — | — |  |
| `ColorSpaces` | Absent | — | — | Display P3/Rec2020/extended sRGB absent; ColorSpace enum has NoColorSpace/Srgb/LinearSrgb only (src/math/color_management.rs:55) |
| `ConvexHull` | Absent | — | — |  |
| `ImprovedNoise` | Present | addons::improved_noise::ImprovedNoise (src/addons/improved_noise.rs:55) | webgpu_volume_perlin |  |
| `Lut` | Absent | — | — |  |
| `MeshSurfaceSampler` | Absent | — | — |  |
| `OBB` | Absent | — | — |  |
| `Octree` | Absent | — | — |  |
| `SimplexNoise` | Absent | — | — |  |
| `ConvexObjectBreaker` | Absent | — | — |  |
| `GPUComputationRenderer` | N.A. | — | — | WebGL-only |
| `Gyroscope` | Absent | — | — |  |
| `MD2Character` | Absent | — | — |  |
| `MD2CharacterComplex` | Absent | — | — |  |
| `MorphAnimMesh` | Absent | — | — |  |
| `MorphBlendMesh` | Absent | — | — |  |
| `ProgressiveLightMap` | N.A. | — | — | WebGL-only |
| `ProgressiveLightMapGPU` | Absent | — | — |  |
| `RollerCoaster` | Absent | — | — |  |
| `Sculptor` | Absent | — | — |  |
| `SculptorMesh` | Absent | — | — |  |
| `SculptorTools` | Absent | — | — |  |
| `SculptorUtils` | Absent | — | — |  |
| `TileCreasedNormalsPlugin` | N.A. | — | — | JS-library plugin (3d-tiles-renderer) |
| `TubePainter` | Absent | — | — |  |
| `Volume` | Absent | — | — | raw hit webgpu_volume_perlin does not use Volume |
| `VolumeSlice` | Absent | — | — |  |
| `AnimationPathHelper` | Absent | — | — |  |
| `LightProbeGridHelper` | Absent | — | — |  |
| `LightProbeGridHelperWebGL` | N.A. | — | — | WebGL-only |
| `LightProbeHelper` | N.A. | — | — | WebGL-only |
| `LightProbeHelperGPU` | Present | LightProbeHelper (src/addons/helpers.rs:21) | webgpu_lightprobe, webgpu_lightprobe_cubecamera (graded) |  |
| `LightProbeGenerator` | Present | LightProbeGenerator (src/addons/lights.rs:14) | webgpu_lightprobe, webgpu_lightprobe_cubecamera (graded); tests/addons_light_probe_generator.rs | fromCubeTexture and fromCubeRenderTarget; row added after the first extraction missed it |
| `OctreeHelper` | Absent | — | — |  |
| `PositionalAudioHelper` | N.A. | — | — | audio DOM |
| `RapierHelper` | Absent | — | — |  |
| `RectAreaLightHelper` | Absent | — | — |  |
| `TextureHelper` | N.A. | — | — | WebGL-only |
| `TextureHelperGPU` | Absent | — | — |  |
| `VertexNormalsHelper` | Absent | — | — |  |
| `VertexTangentsHelper` | Absent | — | — |  |
| `ViewHelper` | Absent | — | — |  |
| `ColorEnvironment` | Absent | — | — |  |
| `DebugEnvironment` | Absent | — | — |  |
| `RoomEnvironment` | Present | RoomEnvironment (src/environments/room_environment.rs:38) | tests/room_environment.rs; webgpu_postprocessing_sobel |  |

### Supplementary: `examples/jsm/tsl/display/*.js`

These 48 files are absent from raw.csv (the extraction skipped `tsl/display`) and are not counted above. 19 Present, 1 Partial, 28 Absent.

| file | status | three-rs port and check |
|---|---|---|
| `AfterImageNode.js` | Present | after_image; tests/nodes_display_wgsl.rs; webgpu_postprocessing_afterimage (ported, ungraded) |
| `AnaglyphPassNode.js` | Absent | — |
| `BilateralBlurNode.js` | Present | bilateral_blur/BilateralBlurNode (src/nodes/display/bilateral_blur.rs:50); tests/nodes_display_wgsl.rs (horizontal and vertical gates); webgpu_postprocessing_godrays (graded); two materials instead of one with a swapped texture; no dispose() |
| `BleachBypass.js` | Absent | — |
| `BloomNode.js` | Present | bloom/BloomNode; webgpu_postprocessing_bloom, _bloom_emissive, _bloom_selective, _anamorphic |
| `boxBlur.js` | Present | box_blur; tests/nodes_display_wgsl.rs (WGSL gate only, no graded rung) |
| `ChromaticAberrationNode.js` | Present | chromatic_aberration; webgpu_postprocessing_ca |
| `CRT.js` | Absent | — |
| `DenoiseNode.js` | Absent | — |
| `depthAwareBlend.js` | Present | depth_aware_blend (src/nodes/display/depth_aware_blend.rs:96); tests/nodes_display_wgsl.rs; webgpu_postprocessing_godrays (graded); perspective camera only, as in three; a baseNode with its own uvNode is not ported |
| `depthAwareBlur.js` | Absent | — |
| `DepthOfFieldNode.js` | Absent | — |
| `DotScreenNode.js` | Present | dot_screen; tests/nodes_display_wgsl.rs; webgpu_postprocessing (ported, ungraded) |
| `FilmNode.js` | Absent | — |
| `FSR1Node.js` | Absent | — |
| `FXAANode.js` | Present | fxaa; webgpu_postprocessing_fxaa |
| `GaussianBlurNode.js` | Present | gaussian_blur; tests/nodes_display_wgsl.rs; webgpu_procedural_texture |
| `GodraysNode.js` | Partial | godrays/GodraysNode (src/nodes/display/godrays.rs:74); tests/nodes_display_wgsl.rs; webgpu_postprocessing_godrays (graded); point lights only: the DirectionalLight branch, log depth and dispose() are not ported |
| `GTAONode.js` | Absent | — |
| `hashBlur.js` | Present | hash_blur; tests/nodes_display_wgsl.rs (loop gated against three's webgpu_backdrop_area dump, taps through viewportSharedTexture) |
| `ImportanceSampledEnvironment.js` | Absent | — |
| `LensflareNode.js` | Present | lensflare/LensflareNode (src/nodes/display/lensflare.rs:84); tests/nodes_display_wgsl.rs; webgpu_postprocessing_lensflare (graded); takes a texture: the caller writes convertToTexture()'s rtt(); no dispose() |
| `Lut3DNode.js` | Absent | — |
| `MotionBlur.js` | Present | motion_blur; tests/nodes_display_wgsl.rs; webgpu_postprocessing_motion_blur (graded) |
| `OITPassNode.js` | Absent | — |
| `OutlineNode.js` | Absent | — |
| `ParallaxBarrierPassNode.js` | Absent | — |
| `PixelationPassNode.js` | Present | pixelation_pass; tests/nodes_display_wgsl.rs (WGSL gate only, no graded rung) |
| `radialBlur.js` | Present | radial_blur; webgpu_postprocessing_radial_blur |
| `RecurrentDenoiseNode.js` | Absent | — |
| `RetroPassNode.js` | Absent | — |
| `RGBShiftNode.js` | Present | rgb_shift; tests/nodes_display_wgsl.rs; webgpu_postprocessing (ported, ungraded) |
| `Sepia.js` | Absent | — |
| `Shape.js` | Absent | (circle(); core shapeCircle exists in src/nodes/tsl.rs:3813, a different function) |
| `SharpenNode.js` | Absent | — |
| `SMAANode.js` | Absent | — |
| `SobelOperatorNode.js` | Present | sobel; webgpu_postprocessing_sobel |
| `SSAAPassNode.js` | Present | SsaaPassNode (src/renderer/ssaa_pass.rs:130); webgpu_postprocessing_ssaa |
| `SSAONode.js` | Absent | — |
| `SSGINode.js` | Absent | — |
| `SSRNode.js` | Absent | — |
| `SSSNode.js` | Absent | — |
| `StereoCompositePassNode.js` | Absent | — |
| `StereoPassNode.js` | Absent | — |
| `TAAUNode.js` | Absent | — |
| `TemporalReprojectNode.js` | Absent | — |
| `TRAANode.js` | Present | traa/TraaNode; tests/nodes_display_wgsl.rs (resolve, subpixel correction, clip AABB, flicker reduction gates), tests/traa_frames.rs; webgpu_postprocessing_traa (ported, ungraded: three's own e2e exception list) |
| `TransitionNode.js` | Present | transition; webgpu_postprocessing_transition |

## TSL

683 names exported through `three/tsl`, grouped into families by the file that defines each name. Many Present rows are verified indirectly: the function is called by material or renderer code that graded examples render (shown as "indirect: used by …"). Methods count: `mixElement` is `NodeRef::mix`.

### math

108 of 109 applicable present (1 Partial, 0 Absent, 0 N.A.).

Partial: `bitcast`.

| three.js name | verdict | three-rs symbol | verified by | note |
|---|---|---|---|---|
| `bitcast` | Partial | bitcast (src/nodes/tsl/wrappers.rs:454, private) | tests/nodes_tsl_batch.rs | private; reachable only via floatBitsToInt/intBitsToFloat family |
| `floatBitsToInt` | Present | float_bits_to_int (src/nodes/tsl/wrappers.rs:475) | tests/nodes_tsl_batch.rs |  |
| `floatBitsToUint` | Present | float_bits_to_uint (src/nodes/tsl/wrappers.rs:480) | tests/nodes_tsl_batch.rs |  |
| `intBitsToFloat` | Present | int_bits_to_float (src/nodes/tsl/wrappers.rs:485) | tests/nodes_tsl_batch.rs |  |
| `uintBitsToFloat` | Present | uint_bits_to_float (src/nodes/tsl/wrappers.rs:490) | tests/nodes_tsl_batch.rs |  |
| `countTrailingZeros` | Present | count_trailing_zeros (src/nodes/tsl/wrappers.rs:314) | tests/nodes_tsl_batch.rs |  |
| `countLeadingZeros` | Present | count_leading_zeros (src/nodes/tsl/wrappers.rs:318) | tests/nodes_tsl_batch.rs |  |
| `countOneBits` | Present | count_one_bits (src/nodes/tsl/wrappers.rs:309) | tests/nodes_tsl_batch.rs |  |
| `hash` | Present | hash (src/nodes/tsl/wrappers.rs:550) | tests/nodes_tsl_batch.rs |  |
| `parabola` | Present | parabola (src/nodes/tsl/wrappers.rs:400) | tests/nodes_tsl_batch.rs |  |
| `gain` | Present | gain (src/nodes/tsl/wrappers.rs:408) | tests/nodes_tsl_batch.rs |  |
| `pcurve` | Present | pcurve (src/nodes/tsl/wrappers.rs:418) | tests/nodes_tsl_batch.rs |  |
| `sinc` | Present | sinc (src/nodes/tsl/wrappers.rs:428) | tests/nodes_tsl_batch.rs |  |
| `packSnorm2x16` | Present | pack_snorm_2x16 (src/nodes/tsl/wrappers.rs:536) | tests/nodes_tsl_batch.rs |  |
| `packUnorm2x16` | Present | pack_unorm_2x16 (src/nodes/tsl/wrappers.rs:541) | tests/nodes_tsl_batch.rs |  |
| `packHalf2x16` | Present | pack_half_2x16 (src/nodes/tsl/wrappers.rs:547) | tests/nodes_tsl_batch.rs |  |
| `packSnorm4x8` | Present | pack_snorm_4x8 (src/nodes/tsl/wrappers.rs:553) | tests/nodes_tsl_batch.rs |  |
| `packUnorm4x8` | Present | pack_unorm_4x8 (src/nodes/tsl/wrappers.rs:559) | tests/nodes_tsl_batch.rs |  |
| `dot4U8Packed` | Present | dot_4u8_packed (src/nodes/tsl/wrappers.rs:608) | tests/nodes_tsl_batch.rs | native builtin only; the no-feature emulation is not ported |
| `dot4I8Packed` | Present | dot_4i8_packed (src/nodes/tsl/wrappers.rs:614) | tests/nodes_tsl_batch.rs | native builtin only; the no-feature emulation is not ported |
| `pack4xI8` | Present | pack_4x_i8 (src/nodes/tsl/wrappers.rs:619) | tests/nodes_tsl_batch.rs | native builtin only |
| `pack4xU8` | Present | pack_4x_u8 (src/nodes/tsl/wrappers.rs:624) | tests/nodes_tsl_batch.rs | native builtin only |
| `pack4xI8Clamp` | Present | pack_4x_i8_clamp (src/nodes/tsl/wrappers.rs:630) | tests/nodes_tsl_batch.rs | native builtin only |
| `pack4xU8Clamp` | Present | pack_4x_u8_clamp (src/nodes/tsl/wrappers.rs:636) | tests/nodes_tsl_batch.rs | native builtin only |
| `unpack4xI8` | Present | unpack_4x_i8 (src/nodes/tsl/wrappers.rs:641) | tests/nodes_tsl_batch.rs | native builtin only |
| `unpack4xU8` | Present | unpack_4x_u8 (src/nodes/tsl/wrappers.rs:646) | tests/nodes_tsl_batch.rs | native builtin only |
| `unpackSnorm2x16` | Present | unpack_snorm_2x16 (src/nodes/tsl/wrappers.rs:565) | tests/nodes_tsl_batch.rs |  |
| `unpackUnorm2x16` | Present | unpack_unorm_2x16 (src/nodes/tsl/wrappers.rs:571) | tests/nodes_tsl_batch.rs |  |
| `unpackHalf2x16` | Present | unpack_half_2x16 (src/nodes/tsl/wrappers.rs:577) | tests/nodes_tsl_batch.rs |  |
| `unpackSnorm4x8` | Present | unpack_snorm_4x8 (src/nodes/tsl/wrappers.rs:583) | tests/nodes_tsl_batch.rs |  |
| `unpackUnorm4x8` | Present | unpack_unorm_4x8 (src/nodes/tsl/wrappers.rs:589) | tests/nodes_tsl_batch.rs |  |
| `triNoise3D` | Present | tri_noise_3d (src/nodes/tsl/wrappers.rs:615) | tests/nodes_tsl_batch.rs |  |
| `EPSILON` | Present | epsilon (src/nodes/tsl/wrappers.rs:225) | tests/nodes_tsl_batch.rs |  |
| `INFINITY` | Present | infinity (src/nodes/tsl/wrappers.rs:231) | tests/nodes_tsl_batch.rs |  |
| `PI` | Present | pi (src/nodes/tsl/wrappers.rs:209) | tests/nodes_tsl_batch.rs |  |
| `PI2` | Present | pi2 (src/nodes/tsl/wrappers.rs:215) | tests/nodes_tsl_batch.rs |  |
| `TWO_PI` | Present | two_pi (src/nodes/tsl.rs:713) | webgpu_compute_points (graded) |  |
| `HALF_PI` | Present | half_pi (src/nodes/tsl/wrappers.rs:220) | tests/nodes_tsl_batch.rs |  |
| `all` | Present | all (src/nodes/tsl.rs:619) | tests/nodes_tsl_batch.rs |  |
| `any` | Present | any (src/nodes/tsl/wrappers.rs:455) | tests/nodes_tsl_batch.rs |  |
| `radians` | Present | radians (src/nodes/tsl/wrappers.rs:301) | tests/nodes_tsl_batch.rs |  |
| `degrees` | Present | degrees (src/nodes/tsl/wrappers.rs:297) | tests/nodes_tsl_batch.rs |  |
| `exp` | Present | exp (src/nodes/tsl.rs:684) | tests/nodes_fog.rs, webgpu_skinning_points (graded) |  |
| `exp2` | Present | exp2 (src/nodes/tsl.rs:663) | examples/dump_wgsl.rs (manual WGSL diff) |  |
| `log` | Present | log (src/nodes/tsl.rs:677) | tests/nodes_frame.rs |  |
| `log2` | Present | log2 (src/nodes/tsl.rs:670) | tests/pmrem.rs |  |
| `sqrt` | Present | sqrt (src/nodes/tsl.rs:1484) | webgpu_compute_texture (graded) |  |
| `inverseSqrt` | Present | inverse_sqrt (src/nodes/tsl.rs:1419) | indirect: used by src/nodes/pmrem_utils.rs |  |
| `floor` | Present | floor (src/nodes/tsl.rs:642) | tests/nodes_compute_indirect_wgsl.rs, webgpu_instance_uniform (graded) |  |
| `ceil` | Present | ceil (src/nodes/tsl.rs:628) | indirect: used by src/nodes/alpha_hash.rs |  |
| `normalize` | Present | .normalize() (src/nodes/tsl.rs:1621) | webgpu_deferred (graded) |  |
| `fract` | Present | fract (src/nodes/tsl.rs:649) | tests/nodes_tsl_batch.rs, webgpu_tsl_interoperability (graded) |  |
| `sin` | Present | .sin() (src/nodes/tsl.rs:1629) | tests/nodes_compute_indirect_wgsl.rs, webgpu_clearcoat (graded) |  |
| `sinh` | Present | sinh (src/nodes/tsl/wrappers.rs:265) | tests/nodes_tsl_batch.rs |  |
| `cos` | Present | .cos() (src/nodes/tsl.rs:1625) | tests/nodes_custom.rs, webgpu_clearcoat (graded) |  |
| `cosh` | Present | cosh (src/nodes/tsl/wrappers.rs:269) | tests/nodes_tsl_batch.rs |  |
| `tan` | Present | tan (src/nodes/tsl/wrappers.rs:261) | tests/nodes_tsl_batch.rs, webgpu_loader_gltf (graded) |  |
| `tanh` | Present | tanh (src/nodes/tsl/wrappers.rs:273) | tests/nodes_tsl_batch.rs |  |
| `asin` | Present | asin (src/nodes/tsl/wrappers.rs:257) | tests/nodes_tsl_batch.rs |  |
| `asinh` | Present | asinh (src/nodes/tsl/wrappers.rs:277) | tests/nodes_tsl_batch.rs |  |
| `acos` | Present | acos (src/nodes/tsl/wrappers.rs:253) | tests/nodes_tsl_batch.rs |  |
| `acosh` | Present | acosh (src/nodes/tsl/wrappers.rs:281) | tests/nodes_tsl_batch.rs |  |
| `atan` | Present | atan (src/nodes/tsl/wrappers.rs:248) | tests/nodes_tsl_batch.rs, webgpu_pmrem_test (graded) |  |
| `atanh` | Present | atanh (src/nodes/tsl/wrappers.rs:285) | tests/nodes_tsl_batch.rs |  |
| `abs` | Present | abs (src/nodes/tsl.rs:784) | tests/nodes_range_buffers.rs, webgpu_compute_points (graded) |  |
| `sign` | Present | sign (src/nodes/tsl.rs:656) | webgpu_tsl_vfx_flames (graded) |  |
| `length` | Present | length (src/nodes/tsl.rs:698) | tests/scene_webgpu_materials.rs, webgpu_compute_points (graded) |  |
| `negate` | Present | .negate() (src/nodes/tsl.rs:1612) | tests/scene_webgpu_materials.rs, webgpu_compute_points (graded) |  |
| `oneMinus` | Present | .one_minus() (src/nodes/tsl.rs:1607) | webgpu_deferred (graded) |  |
| `dFdx` | Present | dpdx (src/nodes/tsl.rs:818) | indirect: used by src/nodes/alpha_hash.rs |  |
| `dFdy` | Present | dpdy (src/nodes/tsl.rs:827) | indirect: used by src/nodes/alpha_hash.rs |  |
| `round` | Present | round (src/nodes/tsl/wrappers.rs:289) | tests/nodes_tsl_batch.rs, webgpu_instance_points (graded) |  |
| `reciprocal` | Present | .reciprocal() (src/nodes/tsl.rs:1684) | indirect: used by src/materials/physical.rs |  |
| `trunc` | Present | trunc (src/nodes/tsl/wrappers.rs:293) | tests/nodes_tsl_batch.rs, webgpu_volume_perlin (graded) |  |
| `fwidth` | Present | fwidth (src/nodes/tsl.rs:792) | indirect: used by sdf-text/src/batched_text.rs |  |
| `transpose` | Present | transpose (src/nodes/tsl/wrappers.rs:305) | tests/nodes_tsl_batch.rs |  |
| `determinant` | Present | determinant (src/nodes/tsl/wrappers.rs:324) | tests/nodes_tsl_batch.rs |  |
| `inverse` | Present | inverse (src/nodes/tsl/wrappers.rs:331) | tests/nodes_tsl_batch.rs |  |
| `min` | Present | min_of (src/nodes/tsl.rs:691) | indirect: used by src/nodes/pmrem_utils.rs |  |
| `max` | Present | max (src/nodes/tsl.rs:1491) | tests/nodes_instanced_attributes.rs, webgpu_compute_points (graded) |  |
| `step` | Present | step (src/nodes/tsl.rs:4893) | tests/nodes_display_wgsl.rs, webgpu_mrt (graded) |  |
| `reflect` | Present | reflect (src/nodes/tsl.rs:635) | indirect: used by src/materials/environment.rs |  |
| `distance` | Present | distance (src/nodes/tsl.rs:707) | webgpu_layers (graded) |  |
| `difference` | Present | difference (src/nodes/tsl/wrappers.rs:370) | tests/nodes_tsl_batch.rs |  |
| `dot` | Present | dot (src/nodes/tsl.rs:609) | tests/nodes_dot_widening.rs, webgpu_materials (graded) |  |
| `cross` | Present | cross (src/nodes/tsl.rs:614) | webgpu_struct_drawindirect (graded) |  |
| `pow` | Present | .pow() (src/nodes/tsl.rs:1674) | webgpu_instance_mesh (graded) |  |
| `pow2` | Present | pow2 (src/nodes/tsl/wrappers.rs:353) | tests/nodes_tsl_batch.rs |  |
| `pow3` | Present | .pow3() (src/nodes/tsl.rs:1679) | tests/nodes_tsl_batch.rs, webgpu_tsl_galaxy (graded) |  |
| `pow4` | Present | pow4 (src/nodes/tsl/wrappers.rs:364) | tests/nodes_tsl_batch.rs |  |
| `transformDirection` | Present | transform_direction (src/nodes/tsl.rs:4918) | webgpu_tsl_raging_sea (graded) |  |
| `transformNormalByViewMatrix` | Present | transform_normal_by_view_matrix (src/nodes/tsl/wrappers.rs:463) | tests/nodes_tsl_batch.rs |  |
| `transformNormalByInverseViewMatrix` | Present | transform_normal_by_inverse_view_matrix (src/nodes/tsl/wrappers.rs:513) | tests/nodes_tsl_batch.rs |  |
| `cbrt` | Present | cbrt (src/nodes/tsl/wrappers.rs:376) | tests/nodes_tsl_batch.rs |  |
| `lengthSq` | Present | length_sq (src/nodes/tsl/wrappers.rs:382) | tests/nodes_tsl_batch.rs |  |
| `mix` | Present | mix (src/nodes/tsl.rs:598) | tests/nodes_custom.rs, webgpu_cubemap_mix (graded) |  |
| `clamp` | Present | .clamp() (src/nodes/tsl.rs:1701) | webgpu_particles (graded) |  |
| `saturate` | Present | .saturate() (src/nodes/tsl.rs:1659) | tests/renderer_sprites.rs, webgpu_shadowmap (graded) |  |
| `refract` | Present | refract (src/nodes/tsl.rs:621) | indirect: used by src/materials/transmission.rs |  |
| `smoothstep` | Present | smoothstep (src/nodes/tsl.rs:807) | tests/nodes_custom.rs, webgpu_tsl_earth (graded) |  |
| `faceForward` | Present | face_forward (src/nodes/tsl/wrappers.rs:388) | tests/nodes_tsl_batch.rs |  |
| `rand` | Present | rand (src/nodes/tsl/wrappers.rs:564) | tests/nodes_tsl_batch.rs |  |
| `mixElement` | Present | NodeRef::mix (src/nodes/tsl.rs:1713) | webgpu_cubemap_mix (graded) | method form |
| `smoothstepElement` | Present | NodeRef::smoothstep (src/nodes/tsl/wrappers.rs:1154) | tests/nodes_tsl_batch.rs | method form |
| `stepElement` | Present | NodeRef::step (src/nodes/tsl/wrappers.rs:1160) | tests/nodes_tsl_batch.rs | method form |
| `faceforward` | Present | faceforward (src/nodes/tsl/wrappers.rs:438) | tests/nodes_tsl_batch.rs | deprecated alias of faceForward |
| `inversesqrt` | Present | inversesqrt (src/nodes/tsl/wrappers.rs:449) | tests/nodes_tsl_batch.rs | deprecated alias of inverseSqrt |
| `checker` | Present | checker (src/nodes/tsl.rs:835) | webgpu_lights_phong (graded) |  |
| `shapeCircle` | Present | shape_circle (src/nodes/tsl.rs:3813) | webgpu_instance_points (graded) |  |

### operators

51 of 54 applicable present (1 Partial, 2 Absent, 14 N.A.).

Missing (Absent): `Switch`, `Stack`.

Partial: `mat4`.

| three.js name | verdict | three-rs symbol | verified by | note |
|---|---|---|---|---|
| `assign` | Present | .assign() (src/nodes/tsl.rs:1933) | tests/nodes_compute_indirect_wgsl.rs, webgpu_compute_points (graded) |  |
| `addMethodChaining` | N.A. | — | — | JS metaprogramming; TSLCore.js |
| `defined` | N.A. | — | — | JS metaprogramming; TSLCore.js |
| `getConstNodeType` | N.A. | — | — | JS metaprogramming; TSLCore.js |
| `ShaderNode` | N.A. | — | — | JS metaprogramming; TSLCore.js |
| `nodeObject` | N.A. | — | — | JS metaprogramming; TSLCore.js |
| `nodeObjectIntent` | N.A. | — | — | JS metaprogramming; TSLCore.js |
| `nodeObjects` | N.A. | — | — | JS metaprogramming; TSLCore.js |
| `nodeArray` | N.A. | — | — | JS metaprogramming; TSLCore.js |
| `nodeProxy` | N.A. | — | — | JS metaprogramming; TSLCore.js |
| `nodeImmutable` | N.A. | — | — | JS metaprogramming; TSLCore.js |
| `nodeProxyIntent` | N.A. | — | — | JS metaprogramming; TSLCore.js |
| `nodeProxyConstructor` | N.A. | — | — | JS metaprogramming; TSLCore.js |
| `Fn` | Present | shader_fn (src/nodes/tsl.rs:4473) | tests/nodes_mx_library.rs |  |
| `setCurrentStack` | N.A. | — | — | JS metaprogramming; TSLCore.js |
| `getCurrentStack` | N.A. | — | — | JS metaprogramming; TSLCore.js |
| `If` | Present | if_else (src/nodes/tsl.rs:4845) | webgpu_texturegather (graded) |  |
| `Switch` | Absent | — | — | comment only: src/animation/animation_action.rs (false positive) |
| `Stack` | Absent | — | — | bodies are Vec<NodeRef>; block() at src/nodes/tsl.rs:4725 |
| `color` | Present | color (src/nodes/tsl/wrappers.rs:112) | tests/nodes_tsl_batch.rs | takes hex only |
| `float` | Present | float (src/nodes/tsl.rs:267) | tests/nodes_custom.rs, webgpu_compute_texture (graded) |  |
| `int` | Present | int (src/nodes/tsl.rs:272) | tests/nodes_mx_library.rs, webgpu_shadowmap (graded) |  |
| `uint` | Present | uint (src/nodes/tsl.rs:4900) | tests/nodes_compute_indirect_wgsl.rs, webgpu_compute_texture (graded) |  |
| `bool` | Present | boolean (src/nodes/tsl.rs:277) | webgpu_volume_perlin (graded) |  |
| `vec2` | Present | vec2 (src/nodes/tsl.rs:282), vec2_join (:308) | tests/nodes_custom.rs |  |
| `ivec2` | Present | ivec2 (src/nodes/tsl.rs:4297) | webgpu_texturegather (graded) |  |
| `uvec2` | Present | uvec2 (src/nodes/tsl/wrappers.rs:136) | tests/nodes_tsl_batch.rs |  |
| `bvec2` | Present | bvec2 (src/nodes/tsl/wrappers.rs:156) | tests/nodes_tsl_batch.rs |  |
| `vec3` | Present | vec3 (src/nodes/tsl.rs:287), vec3_join (:313) | tests/nodes_custom.rs |  |
| `ivec3` | Present | ivec3 (src/nodes/tsl/wrappers.rs:148) | tests/nodes_tsl_batch.rs |  |
| `uvec3` | Present | uvec3 (src/nodes/tsl/wrappers.rs:140) | tests/nodes_tsl_batch.rs |  |
| `bvec3` | Present | bvec3 (src/nodes/tsl/wrappers.rs:160) | tests/nodes_tsl_batch.rs |  |
| `vec4` | Present | vec4 (src/nodes/tsl.rs:298), vec4_join (:318) | tests/nodes_custom.rs |  |
| `ivec4` | Present | ivec4 (src/nodes/tsl/wrappers.rs:152) | tests/nodes_tsl_batch.rs |  |
| `uvec4` | Present | uvec4 (src/nodes/tsl/wrappers.rs:144) | tests/nodes_tsl_batch.rs |  |
| `bvec4` | Present | bvec4 (src/nodes/tsl/wrappers.rs:164) | tests/nodes_tsl_batch.rs |  |
| `mat2` | Present | mat2 (src/nodes/tsl/wrappers.rs:174) | tests/nodes_tsl_batch.rs |  |
| `mat3` | Present | mat3 (src/nodes/tsl/wrappers.rs:179) | indirect: used by src/nodes/display/sobel.rs |  |
| `mat4` | Partial | mat4 (src/nodes/tsl/wrappers.rs:184) | webgpu_multisampled_renderbuffers (#[ignore]d) | no graded or test check |
| `element` | Present | StorageArray::element (src/nodes/tsl.rs:3644) | tests/nodes_compute_indirect_wgsl.rs, webgpu_compute_points (graded) |  |
| `convert` | Present | NodeRef::to(Type) (src/nodes/tsl.rs) | tests/nodes_custom.rs | method form |
| `split` | Present | NodeRef::swizzle (src/nodes/tsl.rs) | tests/nodes_custom.rs | method form |
| `array` | Present | const_array (src/nodes/tsl.rs:323) | indirect: used by src/nodes/display/bloom.rs |  |
| `add` | Present | .add() (src/nodes/tsl.rs:1528) | tests/nodes_compute_indirect_wgsl.rs, webgpu_clearcoat (graded) |  |
| `sub` | Present | .sub() (src/nodes/tsl.rs:1532) | tests/nodes_custom.rs, webgpu_compute_points (graded) |  |
| `mul` | Present | .mul() (src/nodes/tsl.rs:1536) | tests/nodes_custom.rs, webgpu_compute_points (graded) |  |
| `div` | Present | .div() (src/nodes/tsl.rs:1540) | tests/nodes_texture_wgsl.rs, webgpu_compute_texture (graded) |  |
| `mod` | Present | mod_ (src/nodes/tsl/wrappers.rs:523) | tests/nodes_tsl_batch.rs, webgpu_tsl_halftone (graded) |  |
| `equal` | Present | .equal() (src/nodes/tsl.rs:1549) | webgpu_tsl_interoperability (graded) |  |
| `notEqual` | Present | .not_equal() (src/nodes/tsl.rs:1553) | indirect: used by src/nodes/morph.rs |  |
| `lessThan` | Present | .less_than() (src/nodes/tsl.rs:1569) | tests/nodes_tsl_batch.rs, webgpu_texturegrad (graded) |  |
| `greaterThan` | Present | .greater_than() (src/nodes/tsl.rs:1561) | tests/nodes_mx_noise.rs, webgpu_shadowmap (graded) |  |
| `lessThanEqual` | Present | .less_than_equal() (src/nodes/tsl.rs:1557) | webgpu_compute_points (graded) |  |
| `greaterThanEqual` | Present | .greater_than_equal() (src/nodes/tsl.rs:1565) | webgpu_compute_points (graded) |  |
| `and` | Present | .and() (src/nodes/tsl.rs:1573) | tests/nodes_frame.rs, webgpu_texturegrad (graded) |  |
| `or` | Present | .or() (src/nodes/tsl.rs:1577) | tests/e2e/main.rs |  |
| `not` | Present | .not() (src/nodes/tsl.rs:1581) | tests/nodes_compute_wgsl.rs, webgpu_tsl_angular_slicing (graded) |  |
| `xor` | Present | xor (src/nodes/tsl/wrappers.rs:441) | tests/nodes_tsl_batch.rs |  |
| `bitAnd` | Present | .bit_and() (src/nodes/tsl.rs:1594) | indirect: used by src/nodes/materialx/mx_noise.rs |  |
| `bitNot` | Present | bit_not (src/nodes/tsl/wrappers.rs:446) | tests/nodes_tsl_batch.rs |  |
| `bitOr` | Present | .bit_or() (src/nodes/tsl.rs:1598) | indirect: used by src/nodes/materialx/mx_noise.rs |  |
| `bitXor` | Present | .bit_xor() (src/nodes/tsl.rs:1602) | indirect: used by src/nodes/materialx/mx_noise.rs |  |
| `shiftLeft` | Present | .shift_left() (src/nodes/tsl.rs:1586) | indirect: used by src/nodes/materialx/mx_noise.rs |  |
| `shiftRight` | Present | .shift_right() (src/nodes/tsl.rs:1590) | indirect: used by src/nodes/materialx/mx_noise.rs |  |
| `incrementBefore` | Present | increment_before (src/nodes/tsl/wrappers.rs:509) | tests/nodes_tsl_batch.rs |  |
| `decrementBefore` | Present | decrement_before (src/nodes/tsl/wrappers.rs:514) | tests/nodes_tsl_batch.rs |  |
| `increment` | Present | increment (src/nodes/tsl/wrappers.rs:496) | tests/nodes_tsl_batch.rs |  |
| `decrement` | Present | decrement (src/nodes/tsl/wrappers.rs:503) | tests/nodes_tsl_batch.rs |  |

### conditionals/flow

8 of 13 applicable present (1 Partial, 4 Absent, 0 N.A.).

Missing (Absent): `parameter`, `stack`, `Continue`, `VarIntent`.

Partial: `overloadingFn`.

| three.js name | verdict | three-rs symbol | verified by | note |
|---|---|---|---|---|
| `parameter` | Absent | — | — | Fn parameters are Rust closures |
| `stack` | Absent | — | — | bodies are Vec<NodeRef>; block() at src/nodes/tsl.rs:4725 |
| `overloadingFn` | Partial | by_position (src/nodes/materialx/mx_noise.rs:834, internal) | — | internal dispatch in MaterialX only |
| `Loop` | Present | loop_n (src/nodes/tsl.rs:4730) | webgpu_volume_perlin (graded) |  |
| `Continue` | Absent | — | — | comment only: src/geometries/teapot.rs (false positive) |
| `Break` | Present | break_loop (src/nodes/tsl.rs:4831) | webgpu_volume_perlin (graded) |  |
| `call` | Present | call (src/nodes/tsl.rs:4511) | tests/nodes_custom.rs |  |
| `select` | Present | wgsl_select (src/nodes/tsl.rs:4887) | webgpu_volume_perlin (graded) |  |
| `Var` | Present | NodeRef::to_var (src/nodes/tsl.rs:454) | tests/nodes_custom.rs | method form |
| `Const` | Present | NodeRef::to_const (src/nodes/tsl.rs:466) | tests/nodes_custom.rs | method form |
| `VarIntent` | Absent | — | — |  |
| `Discard` | Present | discard (src/nodes/tsl.rs:4875) | webgpu_deferred (graded) |  |
| `Return` | Present | return_statement (src/nodes/tsl.rs:4870) | tests/nodes_custom.rs |  |

### textures

17 of 23 applicable present (1 Partial, 5 Absent, 0 N.A.).

Missing (Absent): `cubeTextureBase`, `uniformCubeTexture`, `uniformTexture`, `sampler`, `samplerComparison`.

Partial: `textureLoad`.

| three.js name | verdict | three-rs symbol | verified by | note |
|---|---|---|---|---|
| `equirectUV` | Present | equirect_uv (src/nodes/tsl.rs:3464) | webgpu_equirectangular (graded) |  |
| `equirectDirection` | Present | equirect_direction (src/nodes/tsl.rs:4001) | tests/nodes_tsl_batch.rs |  |
| `matcapUV` | Present | matcap_uv (src/nodes/tsl.rs:4020) | tests/nodes_tsl_batch.rs |  |
| `maxMipLevel` | Present | max_mip_level (src/nodes/tsl.rs:4039) | tests/nodes_tsl_batch.rs | takes the Texture, not a texture node |
| `spritesheetUV` | Present | spritesheet_uv (src/nodes/tsl.rs:4056) | tests/nodes_tsl_batch.rs |  |
| `triplanarTextures` | Present | triplanar_textures (src/nodes/tsl.rs:4078) | tests/nodes_tsl_batch.rs | takes Textures, not texture nodes |
| `triplanarTexture` | Present | triplanar_texture (src/nodes/tsl.rs:3279) | webgpu_materials (graded) |  |
| `cubeTextureBase` | Absent | — | — |  |
| `cubeTexture` | Present | cube_texture (src/nodes/tsl.rs:3581) | tests/renderer_textures.rs, webgpu_instance_uniform (graded) |  |
| `uniformCubeTexture` | Absent | — | — |  |
| `textureBicubicLevel` | Present | texture_bicubic_level (src/nodes/tsl.rs:4118) | tests/nodes_tsl_batch.rs | takes the Texture and uv, not a texture node |
| `textureBicubic` | Present | texture_bicubic (src/nodes/tsl.rs:4109) | tests/nodes_tsl_batch.rs | takes the Texture and uv, not a texture node |
| `texture` | Present | texture (src/nodes/tsl.rs:3056) | tests/nodes_texture_wgsl.rs, webgpu_materials (graded) |  |
| `uniformTexture` | Absent | — | — |  |
| `textureLoad` | Partial | texture_load_texel (src/nodes/tsl.rs:4315), texture_load_array (:4331) | webgpu_mesh_batch (graded) | DataTexture/DataArrayTexture only; no general texture(map).load() |
| `textureLevel` | Present | texture_level (src/nodes/tsl.rs:3422) | webgpu_equirectangular (graded) |  |
| `sampler` | Absent | — | — | comment only: src/error.rs |
| `samplerComparison` | Absent | — | — | comment only: src/nodes/builder.rs |
| `textureSize` | Present | texture_size (src/nodes/tsl.rs:4303) | indirect: used by src/nodes/display/box_blur.rs |  |
| `texture3D` | Present | texture_3d (src/nodes/tsl.rs:3092) | tests/nodes_texture_wgsl.rs, webgpu_volume_perlin (graded) |  |
| `texture3DLoad` | Present | texture_3d_load (src/nodes/tsl.rs:4221) | tests/nodes_tsl_batch.rs | filterable volumes only, as texture3D; mip 0 only, no level argument |
| `texture3DLevel` | Present | texture_3d_level (src/nodes/tsl.rs:4233) | tests/nodes_tsl_batch.rs |  |
| `pmremTexture` | Present | PmremEnvironment::new (src/nodes/pmrem_node.rs:64) | tests/nodes_compute_indirect_wgsl.rs, webgpu_clearcoat (graded) |  |

### lighting/material

75 of 121 applicable present (9 Partial, 37 Absent, 0 N.A.).

Missing (Absent): `iridescence`, `iridescenceIOR`, `iridescenceThickness`, `dashSize`, `gapSize`, `pointWidth`, `dispersion`, `retroreflectivity`, `materialSpecularStrength`, `materialNormal`, `materialClearcoatNormal`, `materialAnisotropy`, `materialIridescence`, `materialIridescenceIOR`, `materialIridescenceThickness`, `materialLineScale`, `materialLineDashSize`, `materialLineGapSize`, `materialLineDashOffset`, `materialPointSize`, `materialDispersion`, `materialRetroreflectivity`, `materialLightMap`, `materialAO`, `materialReference`, `lightProjectionUV`, `lights`, `lightingContext`, `directPointLight`, `shadow`, `D_GGX_Anisotropic`, `Schlick_to_F0`, `V_GGX_SmithCorrelated_Anisotropic`, `LTC_Evaluate`, `LTC_Evaluate_Volume`, `LTC_Uv`, `getParallaxCorrectNormal`.

Partial: `transmission`, `thickness`, `attenuationDistance`, `pointShadow`, `BRDF_GGX`, `D_GGX`, `DFGLUT`, `EnvironmentBRDF`, `V_GGX_SmithCorrelated`.

| three.js name | verdict | three-rs symbol | verified by | note |
|---|---|---|---|---|
| `property` | Present | property (src/nodes/tsl.rs:500) | indirect: used by src/nodes/materialx/mx_noise.rs |  |
| `varyingProperty` | Present | varying_property (src/nodes/tsl.rs:4325) | tests/nodes_wgsl_varying.rs, webgpu_struct_drawindirect (graded) |  |
| `diffuseColor` | Present | diffuse_color (src/nodes/tsl.rs:2749) | webgpu_deferred (graded) |  |
| `diffuseContribution` | Present | diffuse_contribution (src/nodes/tsl.rs:2857) | indirect: used by src/materials/physical.rs |  |
| `diffuseRoughness` | Present | diffuse_roughness (src/nodes/tsl.rs:2874) | webgpu_loader_gltf_diffuse_roughness (graded) |  |
| `emissive` | Present | emissive_color (src/nodes/tsl.rs:2783) | webgpu_mrt (graded) |  |
| `roughness` | Present | roughness (src/nodes/tsl.rs:2844) | tests/e2e/main.rs, webgpu_clearcoat (graded) |  |
| `metalness` | Present | metalness (src/nodes/tsl.rs:2839) | tests/e2e/main.rs, webgpu_clearcoat (graded) |  |
| `clearcoat` | Present | clearcoat (src/nodes/tsl.rs:2912) | webgpu_clearcoat (graded) |  |
| `clearcoatRoughness` | Present | clearcoat_roughness (src/nodes/tsl.rs:2915) | webgpu_clearcoat (graded) |  |
| `sheen` | Present | sheen (src/nodes/tsl.rs:2865) | webgpu_loader_gltf_sheen (graded) |  |
| `sheenRoughness` | Present | sheen_roughness (src/nodes/tsl.rs:2868) | webgpu_loader_gltf_sheen (graded) |  |
| `iridescence` | Absent | — | — | issue 229; no definition on main; code on branch rung-gltf-iridescence |
| `iridescenceIOR` | Absent | — | — | issue 229; no definition on main; code on branch rung-gltf-iridescence |
| `iridescenceThickness` | Absent | — | — | issue 229; no definition on main; code on branch rung-gltf-iridescence |
| `alphaT` | Present | alpha_t (src/nodes/tsl.rs:2903) | indirect: used by src/materials/node_material.rs |  |
| `anisotropy` | Present | anisotropy (src/nodes/tsl.rs:2900) | webgpu_loader_gltf_anisotropy (graded) | IBL only; direct anisotropic BRDF not ported |
| `anisotropyT` | Present | anisotropy_t (src/nodes/tsl.rs:2906) | indirect: used by src/materials/node_material.rs |  |
| `anisotropyB` | Present | anisotropy_b (src/nodes/tsl.rs:2909) | indirect: used by src/materials/node_material.rs |  |
| `specularColor` | Present | specular_color (src/nodes/tsl.rs:2780) | indirect: used by src/materials/physical.rs |  |
| `specularColorBlended` | Present | specular_color_blended (src/nodes/tsl.rs:2847) | indirect: used by src/materials/physical.rs |  |
| `specularF90` | Present | specular_f90 (src/nodes/tsl.rs:2850) | indirect: used by src/materials/physical.rs |  |
| `shininess` | Present | shininess (src/nodes/tsl.rs:2777) | webgpu_lights_phong (graded) |  |
| `output` | Present | output_property (src/nodes/tsl.rs:2771) | tests/renderer_mrt.rs, webgpu_mrt (graded) |  |
| `dashSize` | Absent | — | — | no dashed line material (docs/lines-progress.md:100); comment only: src/materials/line2.rs |
| `gapSize` | Absent | — | — | no dashed line material (docs/lines-progress.md:100); comment only: src/materials/line2.rs |
| `pointWidth` | Absent | — | — |  |
| `ior` | Present | ior (src/nodes/tsl.rs:2854) | webgpu_furnace_test (graded) |  |
| `transmission` | Partial | transmission (src/nodes/tsl.rs:2920) | webgpu_materials_transmission (0.198% off) | no graded rung; issue 228 |
| `thickness` | Partial | thickness (src/nodes/tsl.rs:2923) | webgpu_materials_transmission (0.198% off) | no graded rung; issue 228 |
| `attenuationDistance` | Partial | attenuation_distance (src/nodes/tsl.rs:2926) | webgpu_materials_transmission (0.198% off) | no graded rung; issue 228 |
| `attenuationColor` | Present | attenuation_color (src/nodes/tsl.rs:2929) | webgpu_shadowmap_opacity (graded) |  |
| `dispersion` | Absent | — | — | comment only: src/materials/physical.rs |
| `retroreflectivity` | Absent | — | — |  |
| `ambientOcclusion` | Present | ambient_occlusion (src/nodes/tsl.rs:2826) | indirect: used by src/materials/physical.rs |  |
| `materialAlphaTest` | Present | material_alpha_test (src/nodes/tsl.rs:2161) | indirect: used by src/renderer/mod.rs |  |
| `materialColor` | Present | material_color (src/nodes/tsl.rs:2141) | indirect: used by src/renderer/mod.rs |  |
| `materialShininess` | Present | material_shininess (src/nodes/tsl.rs:2183) | indirect: used by src/renderer/mod.rs |  |
| `materialEmissive` | Present | material_emissive (src/nodes/tsl.rs:2203) | indirect: used by src/renderer/mod.rs |  |
| `materialOpacity` | Present | material_opacity (src/nodes/tsl.rs:2151) | indirect: used by src/renderer/mod.rs |  |
| `materialSpecular` | Present | material_specular (src/nodes/tsl.rs:2193) | indirect: used by src/renderer/mod.rs |  |
| `materialSpecularIntensity` | Present | material_specular_intensity (src/nodes/tsl.rs:1233) | indirect: used by src/renderer/mod.rs |  |
| `materialSpecularColor` | Present | material_specular_color (src/nodes/tsl.rs:1243) | indirect: used by src/renderer/mod.rs |  |
| `materialSpecularStrength` | Absent | — | — |  |
| `materialReflectivity` | Present | material_reflectivity (src/nodes/tsl.rs:2244) | indirect: used by src/renderer/mod.rs |  |
| `materialRoughness` | Present | material_roughness (src/nodes/tsl.rs:1389) | indirect: used by src/renderer/mod.rs |  |
| `materialDiffuseRoughness` | Present | material_diffuse_roughness (src/nodes/tsl.rs:1287) | indirect: used by src/renderer/mod.rs |  |
| `materialMetalness` | Present | material_metalness (src/nodes/tsl.rs:1212) | indirect: used by src/renderer/mod.rs |  |
| `materialNormal` | Absent | — | — |  |
| `materialClearcoat` | Present | material_clearcoat (src/nodes/tsl.rs:1310) | indirect: used by src/renderer/mod.rs |  |
| `materialClearcoatRoughness` | Present | material_clearcoat_roughness (src/nodes/tsl.rs:1320) | indirect: used by src/renderer/mod.rs |  |
| `materialClearcoatNormal` | Absent | — | — |  |
| `materialRotation` | Present | material_rotation (src/nodes/tsl.rs:2171) | indirect: used by src/renderer/mod.rs |  |
| `materialSheen` | Present | material_sheen (src/nodes/tsl.rs:1256) | indirect: used by src/renderer/mod.rs |  |
| `materialSheenRoughness` | Present | material_sheen_roughness (src/nodes/tsl.rs:1276) | indirect: used by src/renderer/mod.rs |  |
| `materialAnisotropy` | Absent | — | — | comment only: src/materials/node_material.rs |
| `materialIridescence` | Absent | — | — | issue 229; no definition on main; code on branch rung-gltf-iridescence |
| `materialIridescenceIOR` | Absent | — | — | issue 229; no definition on main; code on branch rung-gltf-iridescence |
| `materialIridescenceThickness` | Absent | — | — | issue 229; no definition on main; code on branch rung-gltf-iridescence |
| `materialTransmission` | Present | material_transmission (src/nodes/tsl.rs:1330) | indirect: used by src/renderer/mod.rs |  |
| `materialThickness` | Present | material_thickness (src/nodes/tsl.rs:1340) | indirect: used by src/renderer/mod.rs |  |
| `materialIOR` | Present | material_ior (src/nodes/tsl.rs:1223) | indirect: used by src/renderer/mod.rs |  |
| `materialAttenuationDistance` | Present | material_attenuation_distance (src/nodes/tsl.rs:1350) | indirect: used by src/renderer/mod.rs |  |
| `materialAttenuationColor` | Present | material_attenuation_color (src/nodes/tsl.rs:1360) | indirect: used by src/renderer/mod.rs |  |
| `materialLineScale` | Absent | — | — | no dashed line material (docs/lines-progress.md:100) |
| `materialLineDashSize` | Absent | — | — | no dashed line material (docs/lines-progress.md:100) |
| `materialLineGapSize` | Absent | — | — | no dashed line material (docs/lines-progress.md:100) |
| `materialLineWidth` | Present | material_line_width (src/nodes/tsl.rs:2343) | examples/dump_wgsl.rs (manual WGSL diff) |  |
| `materialLineDashOffset` | Absent | — | — | no dashed line material (docs/lines-progress.md:100) |
| `materialPointSize` | Absent | — | — | comment only: src/materials/node_material.rs |
| `materialDispersion` | Absent | — | — |  |
| `materialRetroreflectivity` | Absent | — | — |  |
| `materialLightMap` | Absent | — | — |  |
| `materialAO` | Absent | — | — | comment only: src/materials/node_material.rs |
| `materialAnisotropyVector` | Present | material_anisotropy_vector (src/nodes/tsl.rs:1300) | indirect: used by src/materials/node_material.rs |  |
| `materialRefractionRatio` | Present | material_refraction_ratio (src/nodes/tsl.rs:2361) | tests/nodes_tsl_batch.rs (reflect_refract_match), tests/nodes_accessor_uniforms.rs | `MeshBasicNodeMaterial::refraction_ratio`: 0.98 for Basic, Lambert and Phong, 0 for the other constructors (docs/nodes.md §67.3) |
| `materialEnvIntensity` | Present | material_env_intensity (src/nodes/tsl.rs:2234) | indirect: used by src/renderer/programs.rs |  |
| `materialEnvRotation` | Present | material_env_rotation (src/nodes/tsl.rs:2254) | webgpu_instance_uniform (graded) |  |
| `materialReference` | Absent | — | — | comment only: src/materials/toon.rs |
| `rangeFogFactor` | Present | range_fog_factor_with_view_z (src/nodes/tsl.rs:924) | tests/nodes_custom.rs |  |
| `densityFogFactor` | Present | density_fog_factor (src/nodes/tsl.rs:943) | indirect: used by src/objects/fog.rs |  |
| `exponentialHeightFogFactor` | Present | exponential_height_fog_factor (src/nodes/tsl.rs:974) | webgpu_fog_height (graded) |  |
| `fog` | Present | fog (src/nodes/tsl.rs:1001) | tests/nodes_custom.rs, webgpu_fog_height (graded) |  |
| `lightShadowMatrix` | Present | shadow_matrix (src/nodes/tsl.rs:1121) | indirect: used by src/lights/light_shadow.rs |  |
| `lightProjectionUV` | Absent | — | — |  |
| `lightPosition` | Present | light_world_position (src/nodes/tsl.rs:1081) | webgpu_lights_phong (graded) |  |
| `lightTargetPosition` | Present | light_target_position (src/nodes/tsl.rs:1091) | indirect: used by src/nodes/tsl.rs |  |
| `lightViewPosition` | Present | light_view_position (src/nodes/tsl.rs:1059) | indirect: used by src/materials/phong.rs |  |
| `lightTargetDirection` | Present | light_target_direction (src/nodes/tsl.rs:4926) | indirect: used by src/materials/phong.rs |  |
| `lights` | Absent | — | — | comment only: src/error.rs |
| `lightingContext` | Absent | — | — |  |
| `shadowPositionWorld` | Present | shadow_position_world (src/nodes/tsl.rs:4908) | indirect: used by src/lights/point_shadow.rs |  |
| `BasicPointShadowFilter` | Present | basic_point_shadow_filter (src/lights/point_shadow.rs:88) | indirect: used by src/lights/shadow_filter.rs |  |
| `PointShadowFilter` | Present | point_shadow_filter (src/lights/point_shadow.rs:107) | indirect: used by src/lights/shadow_filter.rs |  |
| `pointShadow` | Partial | point_shadow (src/lights/point_shadow.rs:157, pub(crate)) | webgpu_shadowmap_pointlight graded via renderer | internal only |
| `directPointLight` | Absent | — | — |  |
| `getDistanceAttenuation` | Present | distance_attenuation (src/materials/phong.rs:29) | indirect: used by src/materials/phong.rs |  |
| `shadow` | Absent | — | — | comment only: src/core/object3d.rs |
| `BasicShadowFilter` | Present | basic_shadow_filter (src/lights/shadow_filter.rs:186) | indirect: used by src/lights/mod.rs |  |
| `PCFShadowFilter` | Present | pcf_shadow_filter (src/lights/shadow_filter.rs:194) | indirect: used by src/lights/mod.rs |  |
| `VSMShadowFilter` | Present | vsm_shadow_filter (src/lights/shadow_filter.rs:227) | indirect: used by src/lights/mod.rs |  |
| `BRDF_EON` | Present | brdf_eon (src/materials/physical.rs:364) | indirect: used by src/materials/physical.rs |  |
| `EON_DirectionalAlbedo` | Present | eon_directional_albedo (src/materials/physical.rs:339) | indirect: used by src/materials/physical.rs |  |
| `BRDF_GGX` | Partial | brdf_ggx (src/materials/physical.rs:125) | webgpu_clearcoat (graded) | anisotropic and iridescence branches missing (issue 229) |
| `BRDF_Lambert` | Present | brdf_lambert (src/materials/phong.rs:66) | indirect: used by src/materials/toon.rs |  |
| `BRDF_Sheen` | Present | brdf_sheen (src/materials/physical.rs:228) | indirect: used by src/materials/physical.rs |  |
| `D_GGX` | Partial | d_ggx (src/materials/physical.rs:102) | — | internal only (not public) |
| `D_GGX_Anisotropic` | Absent | — | — | anisotropic BRDF_GGX not ported (src/renderer/mod.rs:7186) |
| `DFGLUT` | Partial | dfg_lut (src/materials/dfg_lut.rs:47) | — | internal only (not public) |
| `EnvironmentBRDF` | Partial | Physical::environment_brdf (src/materials/physical.rs:569) | — | internal only (not public) |
| `F_Schlick` | Present | f_schlick (src/materials/phong.rs:72) | indirect: used by src/materials/physical.rs |  |
| `Schlick_to_F0` | Absent | — | — | issue 229; no definition on main; code on branch rung-gltf-iridescence |
| `V_GGX_SmithCorrelated` | Partial | v_ggx_smith_correlated (src/materials/physical.rs:63) | — | internal only (not public) |
| `V_GGX_SmithCorrelated_Anisotropic` | Absent | — | — | anisotropic BRDF_GGX not ported (src/renderer/mod.rs:7186) |
| `LTC_Evaluate` | Absent | — | — | no RectAreaLight (docs/nodes.md:3745) |
| `LTC_Evaluate_Volume` | Absent | — | — | no RectAreaLight (docs/nodes.md:3745) |
| `LTC_Uv` | Absent | — | — | no RectAreaLight (docs/nodes.md:3745) |
| `getGeometryRoughness` | Present | geometry_roughness (src/materials/physical.rs:35) | indirect: used by src/materials/physical.rs |  |
| `getParallaxCorrectNormal` | Absent | — | — |  |
| `getRoughness` | Present | get_roughness (src/materials/physical.rs:51) | indirect: used by src/materials/node_material.rs |  |
| `getShIrradianceAt` | Present | get_sh_irradiance_at (src/nodes/tsl.rs:4280) | webgpu_lightprobe (graded); tests/nodes_light_probe.rs |  |

### accessors

76 of 94 applicable present (8 Partial, 10 Absent, 1 N.A.).

Missing (Absent): `bufferAttribute`, `dynamicBufferAttribute`, `instancedDynamicBufferAttribute`, `clipping`, `clippingAlpha`, `hardwareClipping`, `buffer`, `cameraIndex`, `rendererReference`, `reference`.

Partial: `tangentViewFrame`, `bitangentViewFrame`, `builtin`, `cameraViewport`, `instance`, `instancedMesh`, `batchIndirectIndex`, `referenceBuffer`.

| three.js name | verdict | three-rs symbol | verified by | note |
|---|---|---|---|---|
| `bufferAttribute` | Absent | — | — | comment only: src/core/buffer_geometry.rs |
| `dynamicBufferAttribute` | Absent | — | — |  |
| `instancedBufferAttribute` | Present | instanced_buffer_attribute (src/nodes/tsl.rs:4395) | indirect: used by src/nodes/lines.rs |  |
| `instancedDynamicBufferAttribute` | Absent | — | — |  |
| `TBNViewMatrix` | Present | tbn_view_matrix (src/nodes/tsl.rs:1427) | indirect: used by src/materials/node_material.rs |  |
| `parallaxDirection` | Present | parallax_direction (src/nodes/tsl.rs:3173) | tests/nodes_tsl_batch.rs (parallax_matches) |  |
| `parallaxUV` | Present | parallax_uv (src/nodes/tsl.rs:3183) | tests/nodes_tsl_batch.rs (parallax_matches) | returns a vec3, as three's does |
| `bentNormalView` | Present | bent_normal_view (src/nodes/tsl.rs:2990) | indirect: used by src/materials/environment.rs |  |
| `uniformArray` | Present | uniform_array_f32 (src/nodes/tsl.rs:4036) | indirect: used by src/nodes/display/fxaa.rs |  |
| `bitangentGeometry` | Present | bitangent_geometry (src/nodes/tsl.rs:3118) | tests/nodes_tsl_batch.rs (bitangent_geometry_matches) | three `.once( [ 'NORMAL' ] )`s `getBitangent`, so every bitangent in one shader shares the first one built; the port builds each from its own normal and tangent |
| `bitangentLocal` | Present | bitangent_local (src/nodes/tsl.rs:3136) | tests/nodes_tsl_batch.rs (bitangent_local_matches) | see bitangentGeometry |
| `bitangentView` | Present | bitangent_view (src/nodes/tsl.rs:3042) | indirect: used by src/nodes/tsl.rs |  |
| `bitangentWorld` | Present | bitangent_world (src/nodes/tsl.rs:3154) | tests/nodes_tsl_batch.rs (bitangent_world_matches) | see bitangentGeometry |
| `clipping` | Absent | — | — | deferred (docs/nodes.md §6) |
| `clippingAlpha` | Absent | — | — | deferred (docs/nodes.md §6) |
| `hardwareClipping` | Absent | — | — | deferred (docs/nodes.md §6) |
| `tangentViewFrame` | Partial | inline in tangent_attribute_frame (src/nodes/tsl.rs:2983, internal) | — | internal only |
| `bitangentViewFrame` | Partial | inline in tangent_attribute_frame (src/nodes/tsl.rs:2983, internal) | — | internal only |
| `buffer` | Absent | — | — |  |
| `builtin` | Partial | Builtin enum (src/nodes/node.rs) | — | closed enum; no arbitrary builtin(name) |
| `cameraIndex` | Absent | — | — | UniformGroup::CameraIndex only (internal) |
| `cameraNear` | Present | camera_near (src/nodes/tsl.rs:2453) | tests/nodes_tsl_batch.rs (camera_near_far_and_normal_matrix_match) |  |
| `cameraFar` | Present | camera_far (src/nodes/tsl.rs:2463) | tests/nodes_tsl_batch.rs (camera_near_far_and_normal_matrix_match) |  |
| `cameraProjectionMatrix` | Present | camera_projection_matrix (src/nodes/tsl.rs:2079) | webgpu_materials (graded) |  |
| `cameraProjectionMatrixInverse` | Present | camera_projection_matrix_inverse (src/nodes/tsl.rs:2320) | examples/dump_wgsl.rs (manual WGSL diff) |  |
| `cameraViewMatrix` | Present | camera_view_matrix (src/nodes/tsl.rs:2089) | webgpu_tsl_raging_sea (graded) |  |
| `cameraWorldMatrix` | Present | camera_world_matrix (src/nodes/tsl.rs:2099) | examples/dump_wgsl.rs (manual WGSL diff) |  |
| `cameraNormalMatrix` | Present | camera_normal_matrix (src/nodes/tsl.rs:2480) | tests/nodes_tsl_batch.rs (camera_near_far_and_normal_matrix_match) | identity, as WebGPURenderer writes it; no ArrayCamera element |
| `cameraPosition` | Present | camera_position (src/nodes/tsl.rs:2109) | webgpu_tsl_earth (graded) |  |
| `cameraViewport` | Partial | camera_viewport (src/materials/transmission.rs:164, internal) | — | internal only |
| `vertexColor` | Present | vertex_color (src/nodes/tsl.rs:2020) | indirect: used by src/materials/node_material.rs |  |
| `instanceColor` | Present | instance_color (src/nodes/tsl.rs:4133) | examples/dump_wgsl.rs (manual WGSL diff) |  |
| `instance` | Partial | renderer builds instancing (src/renderer/mod.rs) | — | internal only; no public instance() node |
| `instancedMesh` | Partial | renderer builds instancing (src/renderer/mod.rs) | — | internal only; graded via webgpu_instance_uniform |
| `batchColor` | Present | batch_color (src/nodes/batch.rs:36) | indirect: used by src/materials/node_material.rs |  |
| `batchIndirectIndex` | Partial | batch_indirect_index (src/nodes/batch.rs:42, internal) | — | internal only |
| `batch` | Present | batch (src/nodes/batch.rs:63) | examples/dump_wgsl.rs (manual WGSL diff) |  |
| `rendererReference` | Absent | — | — |  |
| `morphReference` | Present | morph_reference (src/nodes/morph.rs:162) | indirect: used by src/materials/node_material.rs |  |
| `modelDirection` | Present | model_direction (src/nodes/tsl.rs:2630) | tests/nodes_tsl_batch.rs (model_scopes_match) |  |
| `modelWorldMatrix` | Present | model_world_matrix (src/nodes/tsl.rs:2121) | tests/nodes_tsl_batch.rs |  |
| `modelPosition` | Present | model_position (src/nodes/tsl.rs:2636) | tests/nodes_tsl_batch.rs (model_scopes_match) |  |
| `modelScale` | Present | model_scale (src/nodes/tsl.rs:2642) | tests/nodes_tsl_batch.rs (model_scopes_match) |  |
| `modelViewPosition` | Present | model_view_position (src/nodes/tsl.rs:2648) | tests/nodes_tsl_batch.rs (model_scopes_match) |  |
| `modelRadius` | Present | model_radius (src/nodes/tsl.rs:2654) | tests/nodes_tsl_batch.rs (model_scopes_match) |  |
| `modelNormalMatrix` | Present | model_normal_matrix (src/nodes/tsl.rs:2131) | webgpu_tsl_raging_sea (graded) |  |
| `modelWorldMatrixInverse` | Present | model_world_matrix_inverse (src/nodes/tsl.rs:2332) | examples/dump_wgsl.rs (manual WGSL diff) |  |
| `modelViewMatrix` | Present | model_view_matrix (src/nodes/tsl.rs:2380) | examples/dump_wgsl.rs (manual WGSL diff) |  |
| `mediumpModelViewMatrix` | Present | mediump_model_view_matrix (src/nodes/tsl.rs:2573) | tests/nodes_tsl_batch.rs (model_view_precision_matches) | a fresh product per call (the port counts uses across both stages) |
| `highpModelViewMatrix` | Present | highp_model_view_matrix (src/nodes/tsl.rs:2585) | tests/nodes_tsl_batch.rs (model_view_precision_matches) | no ArrayCamera element |
| `highpModelNormalViewMatrix` | Present | highp_model_normal_view_matrix (src/nodes/tsl.rs:2601) | tests/nodes_tsl_batch.rs (model_view_precision_matches) | no ArrayCamera element |
| `modelViewProjection` | Present | model_view_projection (src/nodes/tsl.rs:2729) | indirect: used by src/materials/node_material.rs |  |
| `normalGeometry` | Present | normal_geometry (src/nodes/tsl.rs:1971) | indirect: used by src/nodes/tsl.rs |  |
| `normalLocal` | Present | normal_local (src/nodes/tsl.rs:2375) | webgpu_materials (graded) |  |
| `normalFlat` | Present | normal_flat (src/nodes/tsl.rs:2795) | indirect: used by src/nodes/tsl.rs |  |
| `normalViewGeometry` | Present | normal_view_geometry (src/nodes/tsl.rs:2465) | indirect: used by src/materials/physical.rs |  |
| `normalWorldGeometry` | Present | normal_world_geometry (src/nodes/tsl.rs:2713) | webgpu_pmrem_cubemap (graded) |  |
| `normalView` | Present | normal_view (src/nodes/tsl.rs:2532) | webgpu_deferred (graded) |  |
| `normalWorld` | Present | normal_world (src/nodes/tsl.rs:2698) | tests/renderer_textures.rs, webgpu_instance_mesh (graded) |  |
| `clearcoatNormalView` | Present | clearcoat_normal_view (src/nodes/tsl.rs:2968) | indirect: used by src/materials/environment.rs |  |
| `transformNormal` | Present | transform_normal (src/nodes/tsl/wrappers.rs:478) | tests/nodes_tsl_batch.rs (transform_normal_matches) | also `NodeRef::transform_normal` |
| `transformNormalToView` | Present | transform_normal_to_view (src/nodes/tsl/wrappers.rs:496) | tests/nodes_tsl_batch.rs (transform_normal_to_view_matches; webgpu_tsl_raging_sea (graded)) | reads `modelNormalViewMatrix` from the context at construction |
| `objectDirection` | Present | object_direction (src/nodes/tsl.rs:4557) | tests/nodes_tsl_batch.rs (object_scopes_match) | takes `&Node`; refreshes the world matrix and negates for a camera, as getWorldDirection() |
| `objectWorldMatrix` | Present | object_world_matrix (src/nodes/tsl.rs:3789) | webgpu_skinning_points (graded) |  |
| `objectPosition` | Present | object_position (src/nodes/tsl.rs:4574) | tests/nodes_tsl_batch.rs (object_scopes_match) | takes `&Node` |
| `objectScale` | Present | object_scale (src/nodes/tsl.rs:4583) | tests/nodes_tsl_batch.rs (object_scopes_match) | takes `&Node` |
| `objectViewPosition` | Present | object_view_position (src/nodes/tsl.rs:4592) | tests/nodes_tsl_batch.rs (object_scopes_match) | takes `&Node` |
| `objectRadius` | Present | object_radius (src/nodes/tsl.rs:4603) | tests/nodes_tsl_batch.rs (object_scopes_match) | takes `&Node`; the drawn object's geometry, the target's matrixWorld, as three |
| `pointUV` | N.A. | — | — | GLSL-only; PointUVNode.js |
| `clipSpace` | Present | clip_space (src/nodes/tsl.rs:2674) | tests/nodes_tsl_batch.rs (clip_space_matches) | fragment stage only; reads the `clipSpace` build-context key (docs/nodes.md §67) |
| `positionGeometry` | Present | position_geometry (src/nodes/tsl.rs:1966) | webgpu_deferred (graded) |  |
| `positionLocal` | Present | position_local (src/nodes/tsl.rs:2366) | tests/nodes_mx_library.rs, webgpu_layers (graded) |  |
| `positionPrevious` | Present | position_previous (src/nodes/tsl.rs:2551) | webgpu_postprocessing_motion_blur (graded); tests/velocity_frames.rs | skinning reassigns it under last frame's bones when the MRT has a velocity output |
| `positionWorld` | Present | position_world (src/nodes/tsl.rs:2396) | webgpu_materials (graded) |  |
| `positionWorldDirection` | Present | position_world_direction (src/nodes/tsl.rs:2407) | webgpu_equirectangular (graded) |  |
| `positionView` | Present | position_view (src/nodes/tsl.rs:2390) | webgpu_deferred (graded) |  |
| `positionViewDirection` | Present | position_view_direction (src/nodes/tsl.rs:2773) | webgpu_deferred (graded) |  |
| `reference` | Absent | — | — | uniform_object / uniform_settable / user_data cover the use; shadow_normal_bias hit is a false positive |
| `referenceBuffer` | Partial | bone_matrices (src/nodes/skinning.rs:33, internal) | — | internal only; skinning |
| `reflectView` | Present | reflect_view (src/nodes/tsl.rs:3224) | tests/nodes_tsl_batch.rs (reflect_refract_match) |  |
| `refractView` | Present | refract_view (src/nodes/tsl.rs:3235) | tests/nodes_tsl_batch.rs (reflect_refract_match) |  |
| `reflectVector` | Present | reflect_vector (src/nodes/tsl.rs:3250) | webgpu_instance_uniform (graded) |  |
| `refractVector` | Present | refract_vector (src/nodes/tsl.rs:3263) | tests/nodes_tsl_batch.rs (reflect_refract_match) |  |
| `skinning` | Present | skinning (src/nodes/skinning.rs:50) | webgpu_skinning_points (graded) |  |
| `computeSkinning` | Present | compute_skinning (src/nodes/skinning.rs:156) | webgpu_skinning_points (graded) |  |
| `backgroundBlurriness` | Present | background_blurriness (src/nodes/tsl.rs:2274) | webgpu_loader_gltf_anisotropy (graded) |  |
| `backgroundIntensity` | Present | background_intensity (src/nodes/tsl.rs:2284) | indirect: used by src/renderer/programs.rs |  |
| `backgroundRotation` | Present | background_rotation (src/nodes/tsl.rs:2264) | indirect: used by src/renderer/programs.rs |  |
| `tangentGeometry` | Present | tangent_geometry (src/nodes/tsl.rs:3049) | tests/nodes_tsl_batch.rs (bitangent_geometry_matches) |  |
| `tangentLocal` | Present | tangent_local (src/nodes/tsl.rs:3054) | tests/nodes_tsl_batch.rs (bitangent_local_matches) |  |
| `tangentView` | Present | tangent_view (src/nodes/tsl.rs:3037) | indirect: used by src/nodes/tsl.rs |  |
| `tangentWorld` | Present | tangent_world (src/nodes/tsl.rs:3082) | tests/nodes_tsl_batch.rs (tangent_world_matches, bitangent_world_matches) |  |
| `uv` | Present | uv (src/nodes/tsl.rs) | tests/nodes_custom.rs, webgpu_tsl_earth (graded) |  |
| `userData` | Present | user_data (src/nodes/tsl.rs:419) | webgpu_sprites (graded) |  |
| `velocity` | Present | velocity (src/nodes/velocity.rs:64) | webgpu_postprocessing_motion_blur (graded); tests/velocity_frames.rs | issue 163 closed |

### display/postprocessing

40 of 75 applicable present (8 Partial, 27 Absent, 0 N.A.).

Missing (Absent): `outputStruct`, `getTextureIndex`, `viewportSafeUV`, `getViewPosition`, `getScreenPosition`, `getScreenPositionFromClip`, `getNormalFromDepth`, `workingToColorSpace`, `convertColorSpace`, `blendBurn`, `blendDodge`, `blendScreen`, `blendOverlay`, `blendColor`, `grayscale`, `vibrance`, `cdl`, `posterize`, `directionToFaceDirection`, `screenSize`, `viewportCoordinate`, `viewportUV`, `viewportTexture`, `viewportMipTexture`, `viewportOpaqueMipTexture`, `viewportSharedTexture`, `viewportDepthTexture`, `viewZToOrthographicDepth`, `viewZToReversedOrthographicDepth`, `orthographicDepthToViewZ`, `viewZToPerspectiveDepth`, `viewZToReversedPerspectiveDepth`, `viewZToLogarithmicDepth`, `logarithmicDepthToViewZ`, `depth`, `linearDepth`, `viewportLinearDepth`, `depthPass`, `cineonToneMapping`.

Partial: `colorSpaceToWorking`, `negateOnBackSide`, `passTexture`.

| three.js name | verdict | three-rs symbol | verified by | note |
|---|---|---|---|---|
| `outputStruct` | Absent | — | — |  |
| `getTextureIndex` | Absent | — | — |  |
| `mrt` | Present | mrt (src/nodes/mrt.rs:79) | tests/renderer_mrt.rs, webgpu_deferred (graded) |  |
| `viewportSafeUV` | Present | viewport_safe_uv (src/nodes/display/viewport_texture.rs:200) | webgpu_backdrop (graded) |  |
| `reflector` | Present | reflector (src/nodes/reflector_node.rs:217) | webgpu_mirror (graded) |  |
| `rtt` | Present | rtt (src/nodes/display/rtt.rs:73) | webgpu_postprocessing_anamorphic (graded) |  |
| `convertToTexture` | Present | convert_to_texture (src/nodes/display/rtt.rs:86) | webgpu_postprocessing_ca (graded) |  |
| `getViewPosition` | Present | get_view_position (src/nodes/tsl.rs:900) | indirect: used by src/nodes/display/traa.rs (tests/nodes_display_wgsl.rs traa gates) |  |
| `getScreenPosition` | Absent | — | — |  |
| `getScreenPositionFromClip` | Absent | — | — |  |
| `getNormalFromDepth` | Absent | — | — |  |
| `interleavedGradientNoise` | Present | interleaved_gradient_noise (src/nodes/tsl.rs:4945) | indirect: used by src/nodes/display/radial_blur.rs |  |
| `vogelDiskSample` | Present | vogel_disk_sample (src/nodes/tsl.rs:4966) | indirect: used by src/lights/point_shadow.rs |  |
| `workingToColorSpace` | Absent | — | — | CPU-side ColorManagement only; comment only: src/math/color_management.rs |
| `colorSpaceToWorking` | Partial | srgb_to_working (src/nodes/tsl.rs:4654) | — | sRGB source only; no test calls it |
| `convertColorSpace` | Absent | — | — | CPU-side ColorManagement only |
| `toneMapping` | Present | tone_mapping_node (src/materials/node_material.rs:1058) | tests/nodes_custom.rs, webgpu_custom_fog_background (graded) |  |
| `toneMappingExposure` | Present | tone_mapping_exposure (src/nodes/tsl.rs:1409) | webgpu_clearcoat (graded) |  |
| `renderOutput` | Present | render_output (src/materials/node_material.rs:1040) | tests/nodes_custom.rs, webgpu_mrt (graded) |  |
| `blendBurn` | Absent | — | — |  |
| `blendDodge` | Absent | — | — |  |
| `blendScreen` | Absent | — | — |  |
| `blendOverlay` | Present | blend_overlay (src/nodes/tsl.rs:4866) | webgpu_backdrop (graded) |  |
| `blendColor` | Absent | — | — |  |
| `bumpMap` | Present | bump_map_with (src/nodes/tsl.rs:3329) | webgpu_tsl_earth (graded) |  |
| `grayscale` | Present | grayscale (src/nodes/tsl.rs:4851) | webgpu_backdrop, webgpu_tsl_vfx_flames (graded) |  |
| `saturation` | Present | saturation (src/nodes/tsl.rs:4529) | webgpu_postprocessing_difference (graded) |  |
| `vibrance` | Absent | — | — |  |
| `hue` | Present | hue (src/nodes/tsl.rs:4545) | webgpu_postprocessing_bloom_selective (graded) |  |
| `luminance` | Present | luminance (src/nodes/tsl.rs:4523) | tests/nodes_dot_widening.rs, webgpu_postprocessing_difference (graded) |  |
| `cdl` | Absent | — | — |  |
| `posterize` | Present | posterize (src/nodes/tsl.rs:4858) | webgpu_backdrop (graded) |  |
| `frontFacing` | Present | front_facing (src/nodes/tsl.rs:2062) | webgpu_tsl_angular_slicing (graded) |  |
| `faceDirection` | Present | face_direction (src/nodes/tsl.rs:2067) | indirect: used by src/nodes/tsl.rs |  |
| `negateOnBackSide` | Partial | negate_on_back_side (src/nodes/tsl.rs:215, internal) | — | internal only |
| `directionToFaceDirection` | Absent | — | — |  |
| `normalMap` | Present | normal_map_scaled (src/nodes/tsl.rs:1470) | indirect: used by src/materials/node_material.rs |  |
| `premultiplyAlpha` | Present | premultiply_alpha (src/nodes/tsl.rs:4689) | indirect: used by src/nodes/display/gaussian_blur.rs |  |
| `unpremultiplyAlpha` | Present | unpremultiply_alpha (src/nodes/tsl.rs:4703) | examples/dump_wgsl.rs (manual WGSL diff) |  |
| `screenDPR` | Present | screen_dpr (src/nodes/tsl.rs:2315) | examples/dump_wgsl.rs (manual WGSL diff) |  |
| `screenUV` | Present | screen_uv (src/nodes/tsl.rs:2354) | webgpu_layers (graded) |  |
| `screenSize` | Absent | — | — | comment only: src/materials/transmission.rs |
| `screenCoordinate` | Present | frag_coord (src/nodes/tsl.rs:2073) | webgpu_tsl_halftone (graded) |  |
| `viewport` | Present | viewport (src/nodes/tsl.rs:2309) | tests/renderer_half_float_target.rs, webgpu_postprocessing_anamorphic (graded) |  |
| `viewportSize` | Present | viewport_size (src/nodes/tsl.rs:2299) | webgpu_tsl_halftone (graded) |  |
| `viewportCoordinate` | Absent | — | — |  |
| `viewportUV` | Absent | — | — |  |
| `viewportTexture` | Partial | viewport_texture (src/nodes/display/viewport_texture.rs:153) | — | defined; no gate or graded example reads the per-draw copy yet |
| `viewportMipTexture` | Absent | — | — | issue 169 |
| `viewportOpaqueMipTexture` | Absent | — | — | issue 169; transmission uses an internal opaque frame texture; comment only: src/materials/node_material.rs |
| `viewportSharedTexture` | Present | viewport_shared_texture (src/nodes/display/viewport_texture.rs:132) | webgpu_backdrop (graded); tests/nodes_display_wgsl.rs refraction_backdrop_matches_three | issue 169 closed |
| `viewportDepthTexture` | Partial | viewport_depth_texture (src/nodes/display/viewport_texture.rs:173) | — | defined; no gate or graded example reads it yet |
| `viewZToOrthographicDepth` | Partial | view_z_to_orthographic_depth (src/nodes/tsl.rs:916) | — | only caller is linear_depth, itself unverified |
| `viewZToReversedOrthographicDepth` | Absent | — | — |  |
| `orthographicDepthToViewZ` | Absent | — | — |  |
| `viewZToPerspectiveDepth` | Present | view_z_to_perspective_depth (src/nodes/tsl.rs:885) | indirect: used by src/nodes/display/traa.rs (tests/nodes_display_wgsl.rs traa gates) |  |
| `viewZToReversedPerspectiveDepth` | Absent | — | — |  |
| `perspectiveDepthToViewZ` | Present | perspective_depth_to_view_z (src/nodes/tsl.rs:845) | indirect: used by src/renderer/pass.rs |  |
| `viewZToLogarithmicDepth` | Absent | — | — |  |
| `logarithmicDepthToViewZ` | Absent | — | — |  |
| `depth` | Absent | — | — | comment only: src/error.rs |
| `linearDepth` | Partial | linear_depth, linear_depth_of (src/nodes/tsl.rs:928,941) | — | defined; no gate or graded example renders it yet |
| `viewportLinearDepth` | Partial | viewport_linear_depth (src/nodes/display/viewport_texture.rs:193) | — | defined; no gate or graded example renders it yet |
| `toonOutlinePass` | Present | toon_outline_pass (src/nodes/display/toon_outline_pass.rs:40) | webgpu_materials_toon (graded) |  |
| `pass` | Present | pass (src/renderer/pass.rs:117) | tests/nodes_custom.rs, webgpu_custom_fog_background (graded) |  |
| `passTexture` | Partial | PassNode::texture_node (src/renderer/pass.rs:355) | webgpu_mrt (graded) | method form; no free passTexture(pass, texture) |
| `depthPass` | Absent | — | — |  |
| `sRGBTransferEOTF` | Present | srgb_transfer_eotf (src/nodes/tsl.rs:4627) | indirect: used by src/nodes/tsl.rs |  |
| `sRGBTransferOETF` | Present | srgb_transfer_oetf (src/nodes/tsl.rs:4578) | indirect: used by src/materials/node_material.rs |  |
| `linearToneMapping` | Present | linear_tone_mapping (src/nodes/tsl.rs:4605) | indirect: used by src/materials/node_material.rs |  |
| `reinhardToneMapping` | Present | reinhard_tone_mapping (src/nodes/tsl.rs:4665) | indirect: used by src/materials/node_material.rs |  |
| `cineonToneMapping` | Absent | — | — |  |
| `acesFilmicToneMapping` | Present | aces_filmic_tone_mapping (src/nodes/tsl.rs:5001) | indirect: used by src/materials/node_material.rs |  |
| `agxToneMapping` | Present | agx_tone_mapping (src/nodes/tsl.rs:5053) | indirect: used by src/materials/node_material.rs |  |
| `neutralToneMapping` | Present | neutral_tone_mapping (src/nodes/tsl.rs:5160) | indirect: used by src/materials/node_material.rs |  |

### compute/storage

19 of 53 applicable present (3 Partial, 31 Absent, 0 N.A.).

Missing (Absent): `storageElement`, `attributeArray`, `storageTexture3D`, `subgroupSize`, `textureBarrier`, `atomicFunc`, `subgroupElect`, `subgroupBallot`, `subgroupAdd`, `subgroupInclusiveAdd`, `subgroupExclusiveAdd`, `subgroupMul`, `subgroupInclusiveMul`, `subgroupExclusiveMul`, `subgroupAnd`, `subgroupOr`, `subgroupXor`, `subgroupMin`, `subgroupMax`, `subgroupAll`, `subgroupAny`, `subgroupBroadcastFirst`, `quadSwapX`, `quadSwapY`, `quadSwapDiagonal`, `subgroupBroadcast`, `subgroupShuffle`, `subgroupShuffleXor`, `subgroupShuffleUp`, `subgroupShuffleDown`, `quadBroadcast`.

Partial: `globalId`, `localId`, `storageBarrier`.

| three.js name | verdict | three-rs symbol | verified by | note |
|---|---|---|---|---|
| `storageElement` | Absent | — | — |  |
| `computeKernel` | Present | ComputeFlow::workgroup_size (src/nodes/builder.rs:239) | tests/nodes_compute_indirect_wgsl.rs | field on ComputeFlow |
| `compute` | Present | ComputeFlow (src/nodes/builder.rs:239), Renderer::compute (src/renderer/mod.rs:3670) | webgpu_compute_texture (graded) | struct, not .compute(count) |
| `attributeArray` | Absent | — | — |  |
| `instancedArray` | Present | instanced_array (src/nodes/tsl.rs:3633) | tests/nodes_compute_indirect_wgsl.rs, webgpu_compute_points (graded) |  |
| `storage` | Present | storage_struct (src/nodes/tsl.rs:3874) | webgpu_struct_drawindirect (graded) |  |
| `storageTexture3D` | Absent | — | — |  |
| `storageTexture` | Present | storage_texture (src/nodes/tsl.rs:3169) | webgpu_compute_texture (graded) |  |
| `textureStore` | Present | texture_store (src/nodes/tsl.rs:3224) | tests/nodes_texture_wgsl.rs, webgpu_compute_texture (graded) |  |
| `numWorkgroups` | Present | num_workgroups (src/nodes/tsl.rs:2057) | indirect: used by src/nodes/builder.rs |  |
| `workgroupId` | Present | workgroup_id (src/nodes/tsl.rs:2042) | tests/nodes_compute_indirect_wgsl.rs |  |
| `globalId` | Partial | global_id (src/nodes/tsl.rs:2052) | — | no test or example calls it |
| `localId` | Partial | local_id (src/nodes/tsl.rs:2047) | — | no test or example calls it |
| `subgroupSize` | Absent | — | — | comment only: src/nodes/builder.rs |
| `workgroupBarrier` | Present | workgroup_barrier (src/nodes/tsl.rs:3997) | tests/nodes_compute_indirect_wgsl.rs |  |
| `storageBarrier` | Partial | storage_barrier (src/nodes/tsl.rs:4002) | — | no test or example calls it |
| `textureBarrier` | Absent | — | — | comment only: src/nodes/node.rs |
| `workgroupArray` | Present | workgroup_array (src/nodes/tsl.rs:3967) | tests/nodes_compute_indirect_wgsl.rs |  |
| `atomicFunc` | Absent | — | — |  |
| `atomicLoad` | Present | atomic_load (src/nodes/tsl.rs:3958) | tests/nodes_compute_indirect_wgsl.rs |  |
| `atomicStore` | Present | atomic_store (src/nodes/tsl.rs:3938) | webgpu_struct_drawindirect (graded) |  |
| `atomicAdd` | Present | atomic_add (src/nodes/tsl.rs:3938) | tests/nodes_compute_indirect_wgsl.rs |  |
| `atomicSub` | Present | atomic_sub (src/nodes/tsl.rs:3938) | indirect: used by src/nodes/tsl.rs |  |
| `atomicMax` | Present | atomic_max (src/nodes/tsl.rs:3938) | tests/renderer_compute_indirect.rs |  |
| `atomicMin` | Present | atomic_min (src/nodes/tsl.rs:3938) | indirect: used by src/nodes/tsl.rs |  |
| `atomicAnd` | Present | atomic_and (src/nodes/tsl.rs:3938) | indirect: used by src/nodes/tsl.rs |  |
| `atomicOr` | Present | atomic_or (src/nodes/tsl.rs:3938) | indirect: used by src/nodes/tsl.rs |  |
| `atomicXor` | Present | atomic_xor (src/nodes/tsl.rs:3938) | indirect: used by src/nodes/tsl.rs |  |
| `subgroupElect` | Absent | — | — |  |
| `subgroupBallot` | Absent | — | — |  |
| `subgroupAdd` | Absent | — | — |  |
| `subgroupInclusiveAdd` | Absent | — | — |  |
| `subgroupExclusiveAdd` | Absent | — | — |  |
| `subgroupMul` | Absent | — | — |  |
| `subgroupInclusiveMul` | Absent | — | — |  |
| `subgroupExclusiveMul` | Absent | — | — |  |
| `subgroupAnd` | Absent | — | — |  |
| `subgroupOr` | Absent | — | — |  |
| `subgroupXor` | Absent | — | — |  |
| `subgroupMin` | Absent | — | — |  |
| `subgroupMax` | Absent | — | — |  |
| `subgroupAll` | Absent | — | — |  |
| `subgroupAny` | Absent | — | — |  |
| `subgroupBroadcastFirst` | Absent | — | — |  |
| `quadSwapX` | Absent | — | — |  |
| `quadSwapY` | Absent | — | — |  |
| `quadSwapDiagonal` | Absent | — | — |  |
| `subgroupBroadcast` | Absent | — | — |  |
| `subgroupShuffle` | Absent | — | — |  |
| `subgroupShuffleXor` | Absent | — | — |  |
| `subgroupShuffleUp` | Absent | — | — |  |
| `subgroupShuffleDown` | Absent | — | — |  |
| `quadBroadcast` | Absent | — | — |  |

### materialx

48 of 49 applicable present (0 Partial, 1 Absent, 0 N.A.).

Missing (Absent): `mx_frame`.

Partial: none.

| three.js name | verdict | three-rs symbol | verified by | note |
|---|---|---|---|---|
| `mx_aastep` | Present | mx_aastep (src/nodes/materialx/mx_nodes.rs:428) | tests/nodes_mx_library.rs |  |
| `mx_ramplr` | Present | mx_ramplr (src/nodes/materialx/mx_nodes.rs:437) | tests/nodes_mx_library.rs |  |
| `mx_ramptb` | Present | mx_ramptb (src/nodes/materialx/mx_nodes.rs:446) | tests/nodes_mx_library.rs |  |
| `mx_ramp4` | Present | mx_ramp4 (src/nodes/materialx/mx_nodes.rs:456) | tests/nodes_mx_library.rs |  |
| `mx_splitlr` | Present | mx_splitlr (src/nodes/materialx/mx_nodes.rs:472) | tests/nodes_mx_library.rs |  |
| `mx_splittb` | Present | mx_splittb (src/nodes/materialx/mx_nodes.rs:482) | tests/nodes_mx_library.rs |  |
| `mx_transform_uv` | Present | mx_transform_uv (src/nodes/materialx/mx_nodes.rs:492) | tests/nodes_mx_library.rs |  |
| `mx_safepower` | Present | mx_safepower (src/nodes/materialx/mx_nodes.rs:579) | tests/nodes_mx_library.rs |  |
| `mx_contrast` | Present | mx_contrast (src/nodes/materialx/mx_nodes.rs:585) | tests/nodes_mx_library.rs |  |
| `mx_noise_float` | Present | mx_noise_float (src/nodes/materialx/mx_nodes.rs:41) | tests/nodes_mx_library.rs, webgpu_tsl_raging_sea (graded) |  |
| `mx_noise_vec3` | Present | mx_noise_vec3 (src/nodes/materialx/mx_nodes.rs:52) | tests/nodes_mx_library.rs |  |
| `mx_noise_vec4` | Present | mx_noise_vec4 (src/nodes/materialx/mx_nodes.rs:64) | tests/nodes_mx_library.rs |  |
| `mx_smoothstep` | Present | mx_smoothstep (src/nodes/materialx/mx_nodes.rs:601) | tests/nodes_mx_library.rs |  |
| `mx_cell_noise_vec3` | Present | mx_cell_noise_vec3 (src/nodes/materialx/mx_nodes.rs:84) | tests/nodes_mx_library.rs |  |
| `mx_worley_noise_float_2d` | Present | mx_worley_noise_float_2d (src/nodes/materialx/mx_nodes.rs:227) | tests/nodes_mx_library.rs |  |
| `mx_worley_noise_float_3d` | Present | mx_worley_noise_float_3d (src/nodes/materialx/mx_nodes.rs:236) | tests/nodes_mx_library.rs |  |
| `mx_unifiednoise2d` | Present | UnifiedNoise::new_2d (src/nodes/materialx/mx_nodes.rs:341) | tests/nodes_mx_library.rs |  |
| `mx_unifiednoise3d` | Present | UnifiedNoise::new_3d (src/nodes/materialx/mx_nodes.rs:346) | tests/nodes_mx_library.rs |  |
| `mx_worley_noise_float` | Present | mx_worley_noise_float (src/nodes/materialx/mx_nodes.rs:218) | tests/nodes_mx_library.rs |  |
| `mx_worley_noise_vec2` | Present | mx_worley_noise_vec2 (src/nodes/materialx/mx_nodes.rs:246) | tests/nodes_mx_library.rs |  |
| `mx_worley_noise_vec3` | Present | mx_worley_noise_vec3 (src/nodes/materialx/mx_nodes.rs:252) | tests/nodes_mx_library.rs |  |
| `mx_worley_noise_vec3_style` | Present | mx_worley_noise_vec3_style (src/nodes/materialx/mx_nodes.rs:262) | tests/nodes_mx_library.rs |  |
| `mx_cell_noise_float` | Present | mx_cell_noise_float (src/nodes/materialx/mx_nodes.rs:79) | tests/nodes_mx_library.rs |  |
| `mx_fractal_noise_float_2d` | Present | mx_fractal_noise_float_2d (src/nodes/materialx/mx_nodes.rs:115) | tests/nodes_mx_library.rs |  |
| `mx_fractal_noise_float` | Present | mx_fractal_noise_float (src/nodes/materialx/mx_nodes.rs:134) | tests/nodes_mx_library.rs, webgpu_shadowmap (graded) |  |
| `mx_fractal_noise_vec2` | Present | mx_fractal_noise_vec2 (src/nodes/materialx/mx_nodes.rs:153) | tests/nodes_mx_library.rs |  |
| `mx_fractal_noise_vec3` | Present | mx_fractal_noise_vec3 (src/nodes/materialx/mx_nodes.rs:172) | tests/nodes_mx_library.rs, webgpu_shadowmap (graded) |  |
| `mx_fractal_noise_vec4` | Present | mx_fractal_noise_vec4 (src/nodes/materialx/mx_nodes.rs:191) | tests/nodes_mx_library.rs |  |
| `mx_add` | Present | mx_add (src/nodes/materialx/mx_nodes.rs:620) | tests/nodes_mx_library.rs |  |
| `mx_subtract` | Present | mx_subtract (src/nodes/materialx/mx_nodes.rs:625) | tests/nodes_mx_library.rs |  |
| `mx_multiply` | Present | mx_multiply (src/nodes/materialx/mx_nodes.rs:630) | tests/nodes_mx_library.rs |  |
| `mx_divide` | Present | mx_divide (src/nodes/materialx/mx_nodes.rs:635) | tests/nodes_mx_library.rs |  |
| `mx_modulo` | Present | mx_modulo (src/nodes/materialx/mx_nodes.rs:641) | tests/nodes_mx_library.rs |  |
| `mx_power` | Present | mx_power (src/nodes/materialx/mx_nodes.rs:647) | tests/nodes_mx_library.rs |  |
| `mx_atan2` | Present | mx_atan2 (src/nodes/materialx/mx_nodes.rs:652) | tests/nodes_mx_library.rs |  |
| `mx_timer` | Present | mx_timer (src/nodes/materialx/mx_nodes.rs:657) | tests/nodes_mx_library.rs |  |
| `mx_frame` | Absent | — | — |  |
| `mx_invert` | Present | mx_invert (src/nodes/materialx/mx_nodes.rs:662) | tests/nodes_mx_library.rs |  |
| `mx_ifgreater` | Present | mx_ifgreater (src/nodes/materialx/mx_nodes.rs:667) | tests/nodes_mx_library.rs |  |
| `mx_ifgreatereq` | Present | mx_ifgreatereq (src/nodes/materialx/mx_nodes.rs:677) | tests/nodes_mx_library.rs |  |
| `mx_ifequal` | Present | mx_ifequal (src/nodes/materialx/mx_nodes.rs:687) | tests/nodes_mx_library.rs |  |
| `mx_separate` | Present | mx_separate (src/nodes/materialx/mx_nodes.rs:709) | tests/nodes_mx_library.rs |  |
| `mx_place2d` | Present | mx_place2d (src/nodes/materialx/mx_nodes.rs:502) | tests/nodes_mx_library.rs |  |
| `mx_heighttonormal` | Present | mx_heighttonormal (src/nodes/materialx/mx_nodes.rs:551) | tests/nodes_mx_library.rs |  |
| `mx_rotate2d` | Present | mx_rotate2d (src/nodes/materialx/mx_core.rs:12) | tests/nodes_mx_library.rs |  |
| `mx_rotate3d` | Present | mx_rotate3d (src/nodes/materialx/mx_core.rs:24) | tests/nodes_mx_library.rs |  |
| `mx_hsvtorgb` | Present | mx_hsvtorgb (src/nodes/materialx/mx_nodes.rs:389) | tests/nodes_mx_library.rs |  |
| `mx_rgbtohsv` | Present | mx_rgbtohsv (src/nodes/materialx/mx_nodes.rs:394) | tests/nodes_mx_library.rs |  |
| `mx_srgb_texture_to_lin_rec709` | Present | mx_srgb_texture_to_lin_rec709 (src/nodes/materialx/mx_nodes.rs:399) | tests/nodes_mx_library.rs |  |

### utils

29 of 73 applicable present (5 Partial, 39 Absent, 4 N.A.).

Missing (Absent): `defaultShaderStages`, `defaultBuildStages`, `shaderStages`, `vectorComponents`, `bypass`, `uniformFlow`, `setName`, `builtinShadowContext`, `builtinAOContext`, `builtinGIContext`, `label`, `overrideNodes`, `subgroupIndex`, `invocationSubgroupIndex`, `drawIndex`, `uniformGroup`, `sharedUniformGroup`, `frameGroup`, `objectGroup`, `vertexStage`, `unpackRGBToNormal`, `unpackNormal`, `directionToColor`, `colorToDirection`, `replaceDefaultUV`, `sample`, `OnObjectUpdate`, `OnMaterialUpdate`, `OnFrameUpdate`, `OnAfterObjectUpdate`, `OnBeforeObjectUpdate`, `OnBeforeMaterialUpdate`, `OnBeforeFrameUpdate`, `OnBeforeRenderPipeline`, `OnAfterRenderPipeline`, `expression`, `debug`, `addNodeElement`, `wgsl`.

Partial: `NodeShaderStage`, `cache`, `overrideNode`, `renderGroup`, `subBuild`.

| three.js name | verdict | three-rs symbol | verified by | note |
|---|---|---|---|---|
| `NodeShaderStage` | Partial | Stage (src/nodes/builder.rs:50, pub(crate)) | — | internal only |
| `NodeUpdateType` | Present | NodeUpdateType (src/nodes/frame.rs:30) | tests/nodes_frame.rs |  |
| `NodeType` | Present | Type enum (src/nodes/node.rs:22) | tests/nodes_custom.rs | closed enum, not strings |
| `NodeAccess` | Present | StorageAccess (src/nodes/node.rs:943) | tests/nodes_compute_indirect_wgsl.rs | renamed |
| `defaultShaderStages` | Absent | — | — | comment only: src/nodes/builder.rs |
| `defaultBuildStages` | Absent | — | — |  |
| `shaderStages` | Absent | — | — |  |
| `vectorComponents` | Absent | — | — |  |
| `attribute` | Present | attribute (src/nodes/tsl.rs:345) | tests/nodes_wgsl_varying.rs, webgpu_struct_drawindirect (graded) |  |
| `bypass` | Absent | — | — |  |
| `isolate` | Present | isolate (src/nodes/tsl.rs:158) | tests/nodes_custom.rs |  |
| `cache` | Partial | NodeBuilder::cache (src/nodes/builder.rs:1121) | — | internal only (not public) |
| `context` | Present | context (src/nodes/tsl.rs:145) | tests/nodes_custom.rs, webgpu_custom_fog_background (graded) |  |
| `uniformFlow` | Absent | — | — | comment only: src/nodes/materialx/mx_noise.rs |
| `setName` | Absent | — | — | comment only: src/nodes/builder.rs |
| `builtinShadowContext` | Absent | — | — |  |
| `builtinAOContext` | Absent | — | — |  |
| `builtinGIContext` | Absent | — | — |  |
| `label` | Absent | — | — | comment only: src/bin/viewer_app.rs |
| `overrideNode` | Partial | override_node (src/nodes/tsl.rs:99) | — | internal only (not public) |
| `overrideNodes` | Absent | — | — | comment only: src/materials/node_material.rs |
| `vertexIndex` | Present | vertex_index (src/nodes/tsl.rs:2026) | indirect: used by src/nodes/morph.rs |  |
| `instanceIndex` | Present | instance_index (src/nodes/tsl.rs:2031) | tests/nodes_compute_indirect_wgsl.rs, webgpu_compute_points (graded) |  |
| `subgroupIndex` | Absent | — | — |  |
| `invocationSubgroupIndex` | Absent | — | — |  |
| `invocationLocalIndex` | Present | invocation_local_index (src/nodes/tsl.rs:2036) | tests/nodes_compute_indirect_wgsl.rs |  |
| `drawIndex` | Absent | — | — |  |
| `struct` | Present | struct_type (src/nodes/tsl.rs:3837) | webgpu_struct_drawindirect (graded) |  |
| `uniformGroup` | Absent | — | — |  |
| `sharedUniformGroup` | Absent | — | — | comment only: src/nodes/builder.rs |
| `frameGroup` | Absent | — | — |  |
| `renderGroup` | Partial | BufferSource::group_and_name (src/nodes/node.rs:726) | — | internal only (not public) |
| `objectGroup` | Absent | — | — | comment only: src/nodes/node.rs |
| `uniform` | Present | uniform_value (src/nodes/tsl.rs:350) | tests/nodes_custom.rs, webgpu_compute_points (graded) |  |
| `varying` | Present | NodeRef::to_varying (src/nodes/tsl.rs) | tests/nodes_wgsl_varying.rs | method form |
| `vertexStage` | Absent | — | — |  |
| `oscSine` | Present | osc_sine (src/nodes/tsl.rs:4569) | tests/nodes_tsl_batch.rs, webgpu_cubemap_mix (graded) |  |
| `oscSquare` | Present | osc_square (src/nodes/tsl/wrappers.rs:684) | tests/nodes_tsl_batch.rs |  |
| `oscTriangle` | Present | osc_triangle (src/nodes/tsl/wrappers.rs:679) | tests/nodes_tsl_batch.rs |  |
| `oscSawtooth` | Present | osc_sawtooth (src/nodes/tsl/wrappers.rs:689) | tests/nodes_tsl_batch.rs |  |
| `packNormalToRGB` | Present | pack_normal_to_rgb (src/nodes/tsl.rs:4660) | webgpu_mesh_batch (graded) |  |
| `unpackRGBToNormal` | Absent | — | — |  |
| `unpackNormal` | Absent | — | — |  |
| `directionToColor` | Absent | — | — | deprecated r185 alias of packNormalToRGB |
| `colorToDirection` | Absent | — | — | deprecated r185 alias of unpackRGBToNormal |
| `remap` | Present | remap (src/nodes/tsl/wrappers.rs:734) | tests/nodes_tsl_batch.rs, webgpu_tsl_earth (graded) |  |
| `remapClamp` | Present | remap_clamp (src/nodes/tsl/wrappers.rs:753) | tests/nodes_tsl_batch.rs, webgpu_tsl_halftone (graded) |  |
| `replaceDefaultUV` | Absent | — | — |  |
| `rotateUV` | Present | rotate_uv (src/nodes/tsl/wrappers.rs:695) | tests/nodes_tsl_batch.rs |  |
| `spherizeUV` | Present | spherize_uv (src/nodes/tsl/wrappers.rs:712) | tests/nodes_tsl_batch.rs, webgpu_tsl_vfx_flames (graded) |  |
| `billboarding` | Present | billboarding (src/nodes/tsl/wrappers.rs:860) | webgpu_tsl_vfx_flames (graded) |  |
| `rotate` | Present | rotate (src/nodes/tsl.rs:733) | webgpu_layers (graded) |  |
| `time` | Present | time (src/nodes/tsl.rs:2294) | tests/nodes_custom.rs, webgpu_cubemap_mix (graded) |  |
| `deltaTime` | Present | delta_time (src/nodes/tsl/wrappers.rs:791) | tests/nodes_tsl_batch.rs |  |
| `frameId` | Present | frame_id (src/nodes/tsl/wrappers.rs:807) | tests/nodes_frame.rs |  |
| `sample` | Absent | — | — | no SampleNode callback form; PmremHandle::sample etc. are texture methods (false positive) |
| `OnObjectUpdate` | Absent | — | — | comment only: src/nodes/node.rs |
| `OnMaterialUpdate` | Absent | — | — |  |
| `OnFrameUpdate` | Absent | — | — |  |
| `OnAfterObjectUpdate` | Absent | — | — |  |
| `OnBeforeObjectUpdate` | Absent | — | — |  |
| `OnBeforeMaterialUpdate` | Absent | — | — |  |
| `OnBeforeFrameUpdate` | Absent | — | — |  |
| `OnBeforeRenderPipeline` | Absent | — | — | comment only: src/cameras/mod.rs |
| `OnAfterRenderPipeline` | Absent | — | — | comment only: src/renderer/render_pipeline.rs |
| `expression` | Absent | — | — | comment only: src/lights/point_shadow.rs |
| `debug` | Absent | — | — | comment only: src/bin/viewer_app.rs |
| `subBuild` | Partial | in_sub_build (src/nodes/tsl.rs:168) | — | internal only (not public) |
| `inspect` | N.A. | — | — | DOM inspector; InspectorNode.js |
| `addNodeElement` | Absent | — | — |  |
| `code` | Present | code (src/nodes/tsl.rs:4430) | tests/nodes_custom.rs, webgpu_materials (graded) |  |
| `js` | N.A. | — | — | JS codegen; CodeNode.js |
| `wgsl` | Absent | — | — | code() covers raw WGSL; TextureKind::declaration hit is a false positive |
| `glsl` | N.A. | — | — | WebGL-only; CodeNode.js |
| `glslFn` | N.A. | — | — | WebGL-only; FunctionNode.js |
| `wgslFn` | Present | wgsl_fn (src/nodes/code.rs:59) | tests/nodes_wgsl_varying.rs, webgpu_materials (graded) |  |
| `range` | Present | range (src/nodes/tsl.rs:4209) | tests/nodes_range_buffers.rs |  |

## What this matrix says about the next work

Every graded example passes, so graded examples cannot rank the gaps. The ranking below uses need instead: how many of three.js r187's 231 `examples/webgpu_*.html` pages call each absent piece, counted by grep, plus how often it turns up in ordinary scenes.

The first refresh of this matrix closed the previous top five. Velocity and TRAA (`velocity`, `positionPrevious`, `VelocityNode`, `TRAANode`, `MotionBlur`), the screen reads (`viewportSharedTexture`, `viewportTexture`, `viewportDepthTexture`, `viewportLinearDepth`, `linearDepth`, `viewportSafeUV`), `SkyMesh`, `CubeCamera` with a layered `CubeRenderTarget`, and the light probes (`LightProbe`, `LightProbeGenerator`, `LightProbeNode`, `LightProbeHelperGPU`, `getShIrradianceAt`) are Present. So is `transformNormalToView` (6 pages), with the rest of the accessors batch; `webgpu_tsl_raging_sea` now calls it. Of the screen reads only the shared copy has a graded page behind it; `viewportTexture`, `viewportDepthTexture`, `viewportLinearDepth` and `linearDepth` are defined but no gate or graded example renders them yet, so they stay Partial until a depth-reading page is ported.

1. **The screen-space effect nodes** (`GTAONode`, `SSRNode`, `SSGINode`, `SSSNode`, `DenoiseNode`, `DepthOfFieldNode`; `ao()` 2 pages, `ssr()` 2, `ssgi()` 2, `sss()` 1, `dof()` 1, `denoise` 6). Each needs a velocity target and a temporal resolve, and both now exist; these pages are the direct payoff of the velocity work. Porting them also gives the depth reads (`viewportDepthTexture`, `linearDepth`, `getViewPosition`) their first graded consumer.
2. **EventDispatcher** (issues 153, 159). 8 pages subscribe to `change` or `finished` events. In ordinary use these are render-on-demand behind OrbitControls and chaining animation clips when one ends, and neither works in the port today. `AnimationMixer` and `AnimationAction` fire nothing.
3. **Controls** (`TransformControls`, 3 pages). `FirstPersonControls` (8 pages) and `FlyControls` (1) are now ported and gated against three's classes (`docs/controls.md`); `TransformControls` is the remaining one, and it is bigger: a gizmo with its own scene, raycasting and materials.
4. **`WaterMesh`** (2 pages) and the remaining display files (`OutlineNode`, `SMAANode`, `Lut3DNode`, `FilmNode`, 1 to 2 pages each). Small, self-contained, and each unlocks one page.

**Runners-up, and why they rank lower:**
- RectAreaLight and LTC (3 pages).
- Clipping planes (2 pages, but common in CAD-style viewers).
- The TSL long tail: 156 absent `three/tsl` names, most of them unused by any r187 page. They port cheaply in batches against WGSL dump gates.
- One Partial matters more than its page count suggests: `GLTFLoader` has no `KHR_lights_punctual` and no cameras. No r187 page needs them, but arbitrary glTF assets from users will.
