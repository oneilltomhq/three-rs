//! Port of `three.js/src/nodes/core/` — the node graph itself.
//!
//! Three's nodes are mutable JS objects that grow builder state as they are
//! visited. Here a node is an immutable `Rc<Node>`; `Rc::as_ptr` gives it a
//! stable identity, and every piece of per-build state the builder would have
//! hung off the node (usage count, property name, varying slot, uniform slot)
//! lives in the builder keyed by that identity. See `docs/nodes.md` §1.

use std::cell::{Cell, RefCell};
use std::hash::Hash;
use std::rc::Rc;

use crate::math::{Color, Matrix4, Vector2, Vector3};
use crate::textures::{
    CubeDepthTexture, CubeTexture, Data3DTexture, DataArrayTexture, DataTexture, DepthTexture,
    Texture,
};

/// A WGSL value type. Three carries these as strings (`'vec3'`); the closed set
/// is the part of `NodeBuilder`'s type vocabulary the ladder has reached.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Type {
    /// No value — a statement's type, or a `fn` with no return.
    Void,
    /// `bool`.
    Bool,
    /// `f32`.
    F32,
    /// `i32`.
    I32,
    /// `u32`.
    U32,
    /// `vec2<f32>`.
    Vec2,
    /// `vec3<f32>`.
    Vec3,
    /// `vec4<f32>`.
    Vec4,
    /// `vec2<u32>`.
    UVec2,
    /// `vec2<i32>`.
    IVec2,
    /// `vec3<u32>` — MaterialX's `mx_hash_vec3` packs its three byte hashes
    /// into one.
    UVec3,
    /// `vec4<u32>` — the `skinIndex` attribute, which three.js declares
    /// `attribute( 'skinIndex', 'uvec4' )` and uploads as a `Uint32Array`.
    UVec4,
    /// `vec3<i32>` / `vec4<i32>` — TSL's `ivec3()` / `ivec4()`.
    IVec3,
    /// `vec4<i32>` — TSL's `ivec4()`.
    IVec4,
    /// `vec2<bool>` / `vec3<bool>` / `vec4<bool>` — TSL's `bvec2()` … `bvec4()`,
    /// and what a component-wise comparison of two vectors is.
    BVec2,
    /// `vec3<bool>`.
    BVec3,
    /// `vec4<bool>`.
    BVec4,
    /// `mat2` — `RotateNode`'s vec2 path emits `mat2x2<f32>( cos, sin, -sin, cos )`.
    Mat2,
    /// `mat3x3<f32>`.
    Mat3,
    /// `mat4x4<f32>`.
    Mat4,
}

impl Type {
    /// `NodeBuilder.getTypeLength()`.
    pub fn components(self) -> usize {
        match self {
            Type::Void => 0,
            Type::Bool | Type::F32 | Type::I32 | Type::U32 => 1,
            Type::Vec2 | Type::UVec2 | Type::IVec2 | Type::BVec2 => 2,
            Type::Vec3 | Type::UVec3 | Type::IVec3 | Type::BVec3 => 3,
            Type::Vec4 | Type::UVec4 | Type::IVec4 | Type::BVec4 => 4,
            Type::Mat2 => 4,
            Type::Mat3 => 9,
            Type::Mat4 => 16,
        }
    }

    /// `NodeBuilder.getComponentType()`.
    pub fn component_type(self) -> Type {
        match self {
            Type::UVec2 | Type::UVec3 | Type::UVec4 => Type::U32,
            Type::IVec2 | Type::IVec3 | Type::IVec4 => Type::I32,
            Type::BVec2 | Type::BVec3 | Type::BVec4 => Type::Bool,
            Type::Vec2 | Type::Vec3 | Type::Vec4 | Type::Mat2 | Type::Mat3 | Type::Mat4 => {
                Type::F32
            }
            other => other,
        }
    }

    /// The vector type with `self`'s component type and `n` components.
    pub fn vector_of(component: Type, n: usize) -> Type {
        match (component, n) {
            (_, 1) => component,
            (Type::U32, 2) => Type::UVec2,
            (Type::I32, 2) => Type::IVec2,
            (Type::U32, 3) => Type::UVec3,
            (Type::I32, 3) => Type::IVec3,
            (Type::I32, 4) => Type::IVec4,
            (Type::Bool, 2) => Type::BVec2,
            (Type::Bool, 3) => Type::BVec3,
            (Type::Bool, 4) => Type::BVec4,
            (Type::F32, 2) => Type::Vec2,
            (Type::F32, 3) => Type::Vec3,
            (Type::F32, 4) => Type::Vec4,
            (Type::U32, 4) => Type::UVec4,
            other => panic!("three-rs: the node builder only makes vectors of {other:?}"),
        }
    }

    /// Whether this is one of the matrix types.
    pub fn is_matrix(self) -> bool {
        matches!(self, Type::Mat2 | Type::Mat3 | Type::Mat4)
    }
}

/// Which of the generated shader's uniform blocks a uniform lives in.
///
/// Three has a `UniformGroupNode` per uniform; the ladder uses `renderGroup`
/// (per render call — camera, time, viewport), `objectGroup` (per render
/// object — model matrices, material values) and, under an `ArrayCamera`,
/// `sharedUniformGroup( 'cameraIndex' )` — the one `u32` the backend swaps
/// per sub-camera (`docs/nodes.md` §40).
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum UniformGroup {
    /// `renderGroup` — per render call: camera, time, viewport.
    Render,
    /// `cameraIndex`'s group, which three sorts between the two others
    /// (`@group( 1 )` in `webgpu_camera_array`'s dump).
    CameraIndex,
    /// `objectGroup` — per render object: model matrices, material values.
    Object,
}

impl UniformGroup {
    /// The order the groups take their `@group( n )` indices in, each one
    /// only when it has a binding.
    pub(crate) const ORDER: [UniformGroup; 3] = [
        UniformGroup::Render,
        UniformGroup::CameraIndex,
        UniformGroup::Object,
    ];

    pub(crate) fn struct_name(self) -> &'static str {
        match self {
            UniformGroup::Render => "render",
            UniformGroup::CameraIndex => "cameraIndex",
            UniformGroup::Object => "object",
        }
    }
}

/// How often a uniform block has to be rewritten — `Node.updateType`.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum UpdateType {
    /// Written once per render call.
    Render,
    /// Written once per render object.
    Object,
}

/// Where a uniform's bytes come from. The renderer owns the values; the node
/// graph only names them. This is the whole of `NodeUpdateType` /
/// `UniformNode.update()` for the ladder so far.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub enum UniformSource {
    /// `cameraProjectionMatrix` — `camera.projectionMatrix`.
    CameraProjectionMatrix,
    /// `cameraViewMatrix` — `camera.matrixWorldInverse`.
    CameraViewMatrix,
    /// `cameraIndex` — `uniform( 0, 'uint' ).setName( 'u_cameraIndex' )`, the
    /// sub-camera an `ArrayCamera` draw is for. The backend binds one group
    /// per sub-camera rather than rewriting it, so the value written here is
    /// never read; see `Draw::sub_cameras` in the renderer.
    CameraIndex,
    /// `cameraWorldMatrix` — `camera.matrixWorld`.
    CameraWorldMatrix,
    /// `cameraPosition` — `camera.matrixWorld`'s translation, which
    /// `getIBLVolumeRefraction` takes the world-space view vector from.
    CameraPosition,
    /// `modelWorldMatrix` — `object.matrixWorld`.
    ModelWorldMatrix,
    /// `modelNormalMatrix` — `object.normalMatrix`.
    ModelNormalMatrix,
    /// `cameraProjectionMatrixInverse` — `camera.projectionMatrixInverse`.
    /// `Line2NodeMaterial.setupPosition()` needs it to push a clip-space
    /// position back through the camera.
    CameraProjectionMatrixInverse,
    /// `modelWorldMatrixInverse` — `uniform( new Matrix4() ).onObjectUpdate( (
    /// { object }, self ) => self.value.copy( object.matrixWorld ).invert() )`.
    ModelWorldMatrixInverse,
    /// `materialColor` — `MeshBasicMaterial.color` in the working space.
    MaterialColor,
    /// `materialOpacity` — `Material.opacity`.
    MaterialOpacity,
    /// `materialAlphaTest` — `Material.alphaTest`.
    MaterialAlphaTest,
    /// `materialReflectivity` — `MeshLambertMaterial.reflectivity` and
    /// friends.
    MaterialReflectivity,
    /// `materialEnvIntensity` — `MeshStandardMaterial.envMapIntensity`, the
    /// scale `EnvironmentNode` puts on both IBL terms.
    MaterialEnvIntensity,
    /// `materialRotation` — `SpriteMaterial.rotation`.
    MaterialRotation,
    /// `MeshPhongMaterial.shininess` / `.specular` / `.emissive` /
    /// `.emissiveIntensity`.
    MaterialShininess,
    /// `MeshPhongMaterial.specular`.
    MaterialSpecular,
    /// `MeshPhongMaterial.emissive` / `MeshStandardMaterial.emissive`.
    MaterialEmissive,
    /// `emissiveIntensity` — the scale on [`UniformSource::MaterialEmissive`].
    MaterialEmissiveIntensity,
    /// `materialAnisotropyVector` — `MeshPhysicalMaterial.anisotropy` and
    /// `.anisotropyRotation` as one `vec2( a * cos( r ), a * sin( r ) )`.
    MaterialAnisotropyVector,
    /// `MeshPhysicalMaterial.clearcoat` / `.clearcoatRoughness` /
    /// `.clearcoatNormalScale`.
    MaterialClearcoat,
    /// `MeshPhysicalMaterial.clearcoatRoughness`.
    MaterialClearcoatRoughness,
    /// `MeshPhysicalMaterial.clearcoatNormalScale`.
    MaterialClearcoatNormalScale,
    /// `MeshPhysicalMaterial.transmission` / `.thickness` /
    /// `.attenuationDistance` / `.attenuationColor` — `KHR_materials_transmission`
    /// and `KHR_materials_volume`.
    MaterialTransmission,
    /// `MeshPhysicalMaterial.thickness`.
    MaterialThickness,
    /// `MeshPhysicalMaterial.attenuationDistance`.
    MaterialAttenuationDistance,
    /// `MeshPhysicalMaterial.attenuationColor`, in the working space.
    MaterialAttenuationColor,
    /// `materialEnvRotation` — the env map's rotation matrix.
    EnvRotationMatrix,
    /// `backgroundRotation` — `scene.backgroundRotation`.
    BackgroundRotation,
    /// `backgroundBlurriness` — `scene.backgroundBlurriness`.
    BackgroundBlurriness,
    /// `backgroundIntensity` — `scene.backgroundIntensity`.
    BackgroundIntensity,
    /// `reference( 'color' | 'near' | 'far' | 'density', …, scene.fog )
    /// .setGroup( renderGroup )` — the classic `scene.fog`'s parameters, which
    /// `NodeManager.updateFog()` binds as render-group uniforms so that a fog
    /// whose values change never rebuilds a program. The colour is in the
    /// working space.
    FogColor,
    /// `scene.fog.near` — `Fog`'s linear near distance.
    FogNear,
    /// `scene.fog.far` — `Fog`'s linear far distance.
    FogFar,
    /// `scene.fog.density` — `FogExp2`'s density.
    FogDensity,
    /// `time` — `TimerNode.GLOBAL`, seconds since the renderer started.
    Time,
    /// `deltaTime` — `TimerNode.DELTA`, `frame.deltaTime`: the seconds since
    /// the previous `NodeFrame.update()`.
    DeltaTime,
    /// `frameId` — `TimerNode.FRAME`, `uniform( 0, 'uint' )` updated from
    /// `frame.frameId`: the count of `NodeFrame.update()` calls.
    FrameId,
    /// `viewportSize` — the render target's pixel dimensions.
    ViewportSize,
    /// `viewport` — `ScreenNode.VIEWPORT`, the whole rectangle as
    /// `( x, y, width, height )` in physical pixels. `ScreenNode.update()`
    /// takes it from the bound render target, or from
    /// `renderer.getViewport()` times the pixel ratio.
    Viewport,
    /// `screenDPR` — `uniform( 1 ).onRenderUpdate( ( { renderer } ) =>
    /// renderer.getPixelRatio() )`.
    ScreenDpr,
    /// `cameraNear` — `uniform( 'float' ).setName( 'cameraNear' )
    /// .setGroup( renderGroup ).onRenderUpdate( ( { camera } ) => camera.near )`.
    CameraNear,
    /// `cameraFar` — as [`UniformSource::CameraNear`], from `camera.far`.
    CameraFar,
    /// `LightsNode`'s per-light members, by index into the renderer's light
    /// list for the pass. The dumps put all four in the **render** group:
    /// `light.color * light.intensity` (linear), the cutoff distance, the decay
    /// exponent, and the light's position through the camera view matrix.
    LightColorIntensity(usize),
    /// The cutoff distance — see [`UniformSource::LightColorIntensity`].
    LightCutoffDistance(usize),
    /// The decay exponent — see [`UniformSource::LightColorIntensity`].
    LightDecay(usize),
    /// The light's position through the camera view matrix — see
    /// [`UniformSource::LightColorIntensity`].
    LightViewPosition(usize),
    /// `Morph.js`' `base = uniform( 1 )`, updated per object to
    /// `1 - Σ morphTargetInfluences` (or 1 when the targets are relative).
    MorphBase,
    /// `lightPosition( light )` / `lightTargetPosition( light )` — the world
    /// positions `lightTargetDirection` differences.
    LightWorldPosition(usize),
    /// `lightTargetPosition( light )` — see [`UniformSource::LightWorldPosition`].
    LightTargetPosition(usize),
    /// `HemisphereLightNode`: `light.groundColor * light.intensity` (linear).
    LightGroundColor(usize),
    /// `SpotLightNode`'s `coneCosNode` / `penumbraCosNode`.
    LightConeCos(usize),
    /// `SpotLightNode.penumbraCosNode` — see [`UniformSource::LightConeCos`].
    LightPenumbraCos(usize),
    /// `ShadowNode`'s per-shadow references: `lightShadowMatrix( light )`, the
    /// shadow camera's near and far planes (`PointShadowNode`) and
    /// `reference( …, shadow )` for the five scalars.
    ShadowMatrix(usize),
    /// The shadow camera's near plane — see [`UniformSource::ShadowMatrix`].
    ShadowCameraNear(usize),
    /// The shadow camera's far plane — see [`UniformSource::ShadowMatrix`].
    ShadowCameraFar(usize),
    /// `reference( 'bias', 'float', shadow )`.
    ShadowBias(usize),
    /// `reference( 'normalBias', 'float', shadow )`.
    ShadowNormalBias(usize),
    /// `reference( 'radius', 'float', shadow )` — the blur radius.
    ShadowRadius(usize),
    /// `reference( 'blurSamples', 'float', shadow )` — the VSM blur passes'
    /// tap count.
    ShadowBlurSamples(usize),
    /// `reference( 'mapSize', 'vec2', shadow )` — the shadow map's resolution.
    ShadowMapSize(usize),
    /// `reference( 'intensity', 'float', shadow )`.
    ShadowIntensity(usize),
    /// `materialLineWidth` — `MaterialNode.LINE_WIDTH`, i.e.
    /// `material.linewidth`. Only a fat-line material reads it; a hairline
    /// `Line` ignores it, as WebGL and WebGPU both do.
    MaterialLineWidth,
    /// `materialMetalness` / `materialRoughness` / `materialBumpScale`.
    MaterialMetalness,
    /// `MeshStandardMaterial.roughness`.
    MaterialRoughness,
    /// `MeshStandardMaterial.bumpScale`.
    MaterialBumpScale,
    /// `MeshPhysicalMaterial`'s `ior` / `specularIntensity` / `specularColor`,
    /// and `MeshStandardMaterial.normalScale`.
    MaterialIor,
    /// `MeshPhysicalMaterial.specularIntensity`.
    MaterialSpecularIntensity,
    /// `MeshPhysicalMaterial.specularColor`, in the working space.
    MaterialSpecularColor,
    /// `MeshPhysicalMaterial.sheen` / `.sheenColor` / `.sheenRoughness`.
    MaterialSheen,
    /// `MeshPhysicalMaterial.sheenColor`, in the working space.
    MaterialSheenColor,
    /// `MeshPhysicalMaterial.sheenRoughness`.
    MaterialSheenRoughness,
    /// `MeshPhysicalMaterial.diffuseRoughness`.
    MaterialDiffuseRoughness,
    /// `MeshStandardMaterial.normalScale`.
    MaterialNormalScale,
    /// `MeshStandardMaterial.aoMapIntensity` — the scale in `materialAO`'s
    /// `tex.r.sub( 1 ).mul( aoMapIntensity ).add( 1 )`.
    MaterialAoMapIntensity,
    /// `MeshLambertMaterial.lightMapIntensity` — the scale in
    /// `materialLightMap`'s `tex.rgb.mul( lightMapIntensity )`.
    MaterialLightMapIntensity,
    /// `PointsMaterial.size` — `materialPointSize`.
    MaterialPointSize,
    /// `toneMappingExposure` — `renderer.toneMappingExposure`.
    ToneMappingExposure,
    /// `reference( 'bindMatrix', 'mat4' )` / `reference( 'bindMatrixInverse',
    /// 'mat4' )` — `SkinnedMesh`'s two bind matrices, in the object group.
    BindMatrix,
    /// `bindMatrixInverse` — see [`UniformSource::BindMatrix`].
    BindMatrixInverse,
    /// `reference( 'center', 'vec2', object )` — `Sprite.center`, read by
    /// `SpriteNodeMaterial.setupPositionView()`, in the object group.
    ObjectCenter,
    /// `VelocityNode.previousModelWorldMatrix` — the object's `matrixWorld`
    /// as the last velocity draw of it left it (`getPreviousMatrix( object
    /// )`), in the object group. See [`crate::nodes::velocity`].
    PreviousModelWorldMatrix,
    /// `VelocityNode.currentProjectionMatrix` — the camera's projection, or
    /// the unjittered one `setProjectionMatrix()` handed the node, in the
    /// render group.
    VelocityProjectionMatrix,
    /// `VelocityNode.previousProjectionMatrix` — last frame's
    /// `currentProjectionMatrix` for this camera.
    PreviousProjectionMatrix,
    /// `VelocityNode.previousCameraViewMatrix` — last frame's
    /// `camera.matrixWorldInverse` for this camera.
    PreviousCameraViewMatrix,
    /// A plain `uniform( value )` the example supplies.
    Value(Vec<f64>),
    /// `uniform( value )` whose `.value` is written between draws — three.js'
    /// `UniformNode` is always this; [`UniformSource::Value`] is the special
    /// case of one that never moves. `SSAAPassNode.sampleWeight` is the port's
    /// first: one node, one program, eight different values across the eight
    /// accumulation draws of a frame.
    Settable(SettableValue),
    /// A `uniform()` inside a node whose `updateType` is
    /// `NodeUpdateType.OBJECT`: three calls `node.update( frame )` with
    /// `frame.object` set to the render object about to be drawn, and the
    /// callback writes `uniformNode.value` before that object's bindings are
    /// built. `webgpu_instance_uniform`'s `InstanceUniformNode` is the ladder's
    /// first — twelve meshes share one material, one program and one pipeline,
    /// and differ only in the three floats this uniform resolves to.
    ///
    /// The port has no place to hang a `mesh.color` the way the page does, so
    /// the callback receives the object itself and answers from whatever the
    /// application keyed to it. See `docs/nodes.md` §18.
    ObjectUpdate(ObjectUpdate),
    /// A uniform that holds a *reference* to an application object and reads
    /// it when the buffer is written: three's `uniform( skinnedMesh.bindMatrix
    /// )` (the `Matrix4` itself, not a copy of its elements) and
    /// `objectWorldMatrix( object3d )` (`Object3DNode` with an explicit object,
    /// whose `OBJECT` update reads `object3d.matrixWorld`). Unlike
    /// [`UniformSource::ObjectUpdate`] it does not need a render object, so a
    /// compute kernel can read it: `computeSkinning()` and
    /// `webgpu_skinning_points` are the first. See `docs/nodes.md` §44.
    Live(LiveValue),
    /// `cameraNormalMatrix` — `uniform( camera.normalMatrix )`. Nothing in
    /// `WebGPURenderer` ever writes a camera's `normalMatrix` (only
    /// `WebGLRenderer` updates it, and only for the objects it draws), so
    /// three uploads the identity it was constructed with, and so does this.
    CameraNormalMatrix,
    /// `materialRefractionRatio` — `uniform( 0 ).onObjectUpdate( ( { material
    /// } ) => material.refractionRatio )`.
    MaterialRefractionRatio,
    /// `highpModelViewMatrix` — `uniform( 'mat4' ).onObjectUpdate( … )`:
    /// `camera.matrixWorldInverse * object.matrixWorld`, multiplied on the CPU
    /// in double precision.
    HighpModelViewMatrix,
    /// `highpModelNormalViewMatrix` — the normal matrix of
    /// [`UniformSource::HighpModelViewMatrix`]'s product, per object.
    HighpModelNormalViewMatrix,
    /// `Object3DNode`'s vector and scalar scopes: [`Object3DScope`] of the
    /// object drawn (`ModelNode`, `object` `None`) or of an explicit one
    /// (`objectPosition( object3d )` and friends), whose `matrixWorld` the
    /// [`LiveValue`] reads when the buffer is written.
    Object3D {
        /// Which value of the object.
        scope: Object3DScope,
        /// The explicit object's `matrixWorld`, or `None` for the object the
        /// draw is for.
        object: Option<LiveValue>,
    },
}

/// `Object3DNode`'s scopes other than `WORLD_MATRIX`, which has a uniform
/// source of its own ([`UniformSource::ModelWorldMatrix`], or
/// [`UniformSource::Live`] for an explicit object).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Object3DScope {
    /// `POSITION` — `setFromMatrixPosition( matrixWorld )`, a `vec3`.
    Position,
    /// `SCALE` — `setFromMatrixScale( matrixWorld )`, a `vec3`.
    Scale,
    /// `DIRECTION` — `getWorldDirection()`, the normalised `+z` column of
    /// `matrixWorld`, a `vec3`.
    Direction,
    /// `VIEW_POSITION` — the world position through the rendering camera's
    /// `matrixWorldInverse`, a `vec3`.
    ViewPosition,
    /// `RADIUS` — the bounding sphere of the *drawn* object's geometry,
    /// through the scoped object's `matrixWorld`, a `float`. three reads
    /// `frame.object.geometry` even for an explicit object, and so does this.
    Radius,
}

/// The reader behind [`UniformSource::Live`]. Compares and hashes by
/// identity, like [`SettableValue`]: the value moves, the program does not.
#[derive(Clone)]
pub struct LiveValue(Rc<dyn Fn() -> Vec<f64>>);

impl LiveValue {
    pub(crate) fn new(read: impl Fn() -> Vec<f64> + 'static) -> Self {
        Self(Rc::new(read))
    }

    /// The value now.
    pub(crate) fn get(&self) -> Vec<f64> {
        (self.0)()
    }
}

impl std::fmt::Debug for LiveValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("LiveValue")
            .field(&Rc::as_ptr(&self.0).cast::<u8>())
            .finish()
    }
}

impl PartialEq for LiveValue {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}

impl std::hash::Hash for LiveValue {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        (Rc::as_ptr(&self.0).cast::<u8>() as usize).hash(state);
    }
}

/// The callback behind [`UniformSource::ObjectUpdate`] — `Node.update( frame )`
/// narrowed to what an `OBJECT` update can read: `frame.object`, and the one
/// question a node on the ladder asks `frame.renderer`, `isOccluded()`.
///
/// Two callbacks with the same behaviour are still two uniforms, so, like
/// [`SettableValue`], this compares by identity: the program cache must not
/// merge two per-object uniforms that happen to be spelled alike.
#[derive(Clone)]
pub struct ObjectUpdate(Rc<ObjectUpdateFn>);

/// The body of an [`ObjectUpdate`]: three's `update( frame )`.
pub(crate) type ObjectUpdateFn = dyn Fn(&NodeFrame) -> Vec<f64>;

/// `NodeFrame` as a node's `update( frame )` sees it, narrowed to what the
/// port's object-update uniforms read.
///
/// three hands the node the whole frame, renderer included; the port hands it
/// the render object and the renderer's occlusion results for the render
/// context being drawn, which is all `frame.renderer.isOccluded( object )`
/// reads (`webgpu_occlusion`, `docs/nodes.md` §39).
///
/// This is the per-object view of the frame. The frame itself — `frameId`,
/// `renderId`, the clock and the update maps — is
/// [`NodeFrameState`](crate::nodes::NodeFrameState), which the renderer owns
/// (`docs/nodes.md` §57).
#[derive(Clone, Copy)]
pub struct NodeFrame<'a> {
    /// `frame.object` — the render object about to be drawn.
    pub object: &'a crate::core::Object3D,
    /// `renderContextData.occluded` for the current render context: the ids of
    /// the objects whose last resolved occlusion query drew no samples. `None`
    /// until a query has resolved, as three's is `undefined`.
    pub(crate) occluded: Option<&'a std::collections::HashSet<u32>>,
}

impl<'a> NodeFrame<'a> {
    /// `frame.renderer.isOccluded( object )`: whether the last occlusion query
    /// the current render context resolved for `object` drew no samples.
    /// Results arrive asynchronously, a frame or more after the draw, so this
    /// is `false` until then, as it is in three.
    pub fn is_occluded(&self, object: &crate::core::Object3D) -> bool {
        self.occluded.is_some_and(|set| set.contains(&object.id))
    }
}

impl ObjectUpdate {
    pub(crate) fn new(update: impl Fn(&crate::core::Object3D) -> Vec<f64> + 'static) -> Self {
        Self(Rc::new(move |frame: &NodeFrame| update(frame.object)))
    }

    /// An update that reads more of the frame than its object — see
    /// [`NodeFrame`].
    pub(crate) fn with_frame(update: impl Fn(&NodeFrame) -> Vec<f64> + 'static) -> Self {
        Self(Rc::new(update))
    }

    /// `node.update( frame )` — the value for one render object.
    pub(crate) fn value(&self, frame: &NodeFrame) -> Vec<f64> {
        (self.0)(frame)
    }
}

impl std::fmt::Debug for ObjectUpdate {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("ObjectUpdate")
            .field(&Rc::as_ptr(&self.0).cast::<u8>())
            .finish()
    }
}

impl PartialEq for ObjectUpdate {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}

impl std::hash::Hash for ObjectUpdate {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        (Rc::as_ptr(&self.0).cast::<u8>() as usize).hash(state);
    }
}

/// The cell behind [`UniformSource::Settable`]. Two cells with equal contents
/// are still two uniforms, so this compares and hashes by identity — a value
/// that changes must not be part of any cache key.
#[derive(Clone, Debug)]
pub struct SettableValue(Rc<RefCell<Vec<f64>>>);

impl SettableValue {
    /// Wraps a starting value.
    pub fn new(values: Vec<f64>) -> Self {
        Self(Rc::new(RefCell::new(values)))
    }

    /// `uniformNode.value = …`.
    pub fn set(&self, values: Vec<f64>) {
        *self.0.borrow_mut() = values;
    }

    /// `uniformNode.value`.
    pub fn get(&self) -> Vec<f64> {
        self.0.borrow().clone()
    }
}

impl PartialEq for SettableValue {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}

impl std::hash::Hash for SettableValue {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        (Rc::as_ptr(&self.0) as *const u8 as usize).hash(state);
    }
}

impl UniformSource {
    pub(crate) fn update_type(&self) -> UpdateType {
        match self {
            UniformSource::ModelWorldMatrix
            | UniformSource::ModelNormalMatrix
            | UniformSource::MaterialColor
            | UniformSource::MaterialOpacity
            | UniformSource::MaterialAlphaTest
            | UniformSource::MaterialReflectivity
            | UniformSource::MaterialEnvIntensity
            | UniformSource::MaterialShininess
            | UniformSource::MaterialSpecular
            | UniformSource::MaterialEmissive
            | UniformSource::MaterialEmissiveIntensity
            | UniformSource::ModelWorldMatrixInverse
            | UniformSource::MaterialLineWidth
            | UniformSource::MaterialMetalness
            | UniformSource::MaterialRoughness
            | UniformSource::MaterialBumpScale
            | UniformSource::MaterialIor
            | UniformSource::MaterialSpecularIntensity
            | UniformSource::MaterialSpecularColor
            | UniformSource::MaterialSheen
            | UniformSource::MaterialSheenColor
            | UniformSource::MaterialSheenRoughness
            | UniformSource::MaterialDiffuseRoughness
            | UniformSource::MaterialNormalScale
            | UniformSource::MaterialAoMapIntensity
            | UniformSource::MaterialLightMapIntensity
            | UniformSource::MaterialPointSize
            | UniformSource::EnvRotationMatrix
            | UniformSource::MorphBase
            | UniformSource::BindMatrix
            | UniformSource::BindMatrixInverse
            | UniformSource::ObjectCenter
            | UniformSource::PreviousModelWorldMatrix
            | UniformSource::Value(_)
            | UniformSource::Settable(_)
            | UniformSource::ObjectUpdate(_)
            | UniformSource::Live(_) => UpdateType::Object,
            _ => UpdateType::Render,
        }
    }
}

/// `UniformNode`.
#[derive(Debug)]
pub struct UniformNode {
    /// Where the uniform's bytes come from.
    pub source: UniformSource,
    /// The uniform's WGSL type.
    pub ty: Type,
    /// Which uniform block the uniform is bound in.
    pub group: UniformGroup,
    /// Three names camera uniforms explicitly and numbers the rest
    /// `nodeUniformN`.
    pub name: Option<&'static str>,
}

/// Where an array-typed uniform buffer's contents come from — `BufferNode`.
#[derive(Clone, PartialEq)]
#[non_exhaustive]
pub enum BufferSource {
    /// `InstancedMesh.instanceMatrix`.
    InstanceMatrix,
    /// `InstancedMesh.instanceColor` — `setColorAt()`'s three floats per
    /// instance, always a `stepMode: 'instance'` vertex buffer (three.js wraps
    /// it in a fresh `InstancedBufferAttribute( colors.array, 3 )` and never
    /// takes the uniform branch for it).
    InstanceColor,
    /// `RangeNode` resolved per instance:
    /// `lerp( min[c], max[c], Math.random() )`. `min`/`max` are the `Vector4`s
    /// `RangeNode.setup()` builds out of the min/max values: a scalar splats
    /// into all four components, a `Color` fills `xyz` and leaves `w` at 1, and
    /// any other vector takes `x`, `y`, `z || 0`, `w || 0` — so a `vec3` range
    /// has `w` **0** at both ends, not 1.
    Range {
        /// The low end of the range, one component per channel.
        min: [f64; 4],
        /// The high end of the range, one component per channel.
        max: [f64; 4],
    },
    /// `Morph.js`' `uniformArray( mesh.morphTargetInfluences, 'float' )` — one
    /// `vec4` per morph target with the influence in `.x`.
    MorphInfluences,
    /// `uniformArray( values )` — a constant array the application supplies,
    /// already padded to one `vec4` per element the way
    /// `UniformArrayNode.updateBuffer()` pads it. `BloomNode.bloomTintColors`
    /// is the ladder's first; `GodraysNode`, `SSAONode`, `GTAONode` and
    /// `SSRNode` want the same node.
    ///
    /// The values travel in an `Rc` so the buffer can be cached on the node's
    /// identity and uploaded once, like [`BufferSource::Attribute`].
    UniformArray(Rc<Vec<f32>>),
    /// `Camera.js`' `uniformArray( matrices ).setGroup( renderGroup )
    /// .setName( 'cameraViewMatrices' )` — every `ArrayCamera` sub-camera's
    /// `matrixWorldInverse`, rewritten per render. A render-group buffer, the
    /// only one.
    CameraViewMatrices,
    /// The same for `cameraProjectionMatrices` — the sub-cameras'
    /// `projectionMatrix`.
    CameraProjectionMatrices,
    /// `buffer( previousBoneMatrices, 'mat4', bones )` — `Skinning.js`'
    /// `getPreviousSkinnedPosition()`: the skeleton's bone matrices as they
    /// were before this frame's `skeleton.update()`, for the
    /// `positionPrevious` of a draw into a `velocity` MRT.
    PreviousBoneMatrices,
    /// `referenceBuffer( 'skeleton.boneMatrices', 'mat4', bones )` — the
    /// skeleton's bone matrices as one `array< mat4x4<f32>, N >`. Three falls
    /// back to a bone *texture* when `bones * 64` passes the uniform buffer
    /// limit; the ladder's skeletons fit (Michelle is 65 bones, 4160 bytes).
    BoneMatrices,
    /// A per-instance attribute the caller fills itself — three.js'
    /// `new InstancedBufferAttribute( array, itemSize )` on the geometry, e.g.
    /// `BatchedText`'s `aGlyphUV` / `aGlyphBounds` / `aColor` / `aOpacity`.
    /// Only ever a vertex buffer: the uniform path has no equivalent.
    Attribute(Rc<Vec<f32>>),
    /// `instancedArray( count, type )` — a GPU-only *storage* buffer
    /// (`StorageBufferNode`, `StorageInstancedBufferAttribute` with no array
    /// behind it). Zero-filled once by the renderer and never re-uploaded: the
    /// only thing that ever writes it is a compute pass.
    ///
    /// Declared `array< T >` with **no** element count, because
    /// `WGSLNodeBuilder.getStorageAccess()` emits a runtime-sized array; the
    /// `count` on the [`BufferNode`] is only what the renderer allocates.
    Storage,
    /// `instancedArray( count, 'uint' ).toAtomic()` — [`BufferSource::Storage`]
    /// whose elements are declared `atomic< T >`
    /// (`WGSLNodeBuilder.getUniforms()`'s `bufferNode.isAtomic` arm), so the
    /// only way to touch one is an [`atomic function`](Node::Atomic).
    AtomicStorage,
    /// `storage( indirectAttribute, struct( … ), count )` — an
    /// `IndirectStorageBufferAttribute` read and written through a named WGSL
    /// struct (`StructTypeNode`). Declared as the struct itself rather than
    /// wrapped in `{ value : array< … > }`, which is
    /// `WGSLNodeBuilder.isCustomStruct()`'s single-struct case. `init` is the
    /// attribute's `Uint32Array`, uploaded once when the GPU buffer is made;
    /// see [`crate::core::IndirectStorageBufferAttribute`].
    Struct {
        /// The struct's name and member list.
        layout: Rc<StructLayout>,
        /// The attribute's initial contents, uploaded once.
        init: Rc<Vec<u32>>,
    },
    /// `storage( attribute, type, count )` over an attribute that *has* a CPU
    /// array — `computeSkinning()`'s `storage( new InstancedBufferAttribute(
    /// position.array, 3 ), 'vec3' )`, or a `StorageInstancedBufferAttribute(
    /// array, itemSize )`. Uploaded once, when the GPU buffer is made, then
    /// left to the kernels. `init` is the array's bits already laid out at the
    /// storage stride (a `vec3` padded to 16 bytes, as
    /// `WebGPUAttributeUtils.createAttribute()` pads it).
    ///
    /// `read_only` is `.toReadOnly()`: `var<storage, read>` in a kernel too.
    StorageData {
        /// The array's bits, already laid out at the storage stride.
        init: Rc<Vec<u32>>,
        /// `.toReadOnly()` — `var<storage, read>` rather than `read_write`.
        read_only: bool,
    },
    /// `buffer( skeleton.boneMatrices, 'mat4', bones )` — `computeSkinning()`'s
    /// bone matrices: a plain uniform `BufferNode` over the skeleton's own
    /// array rather than `SkinningNode`'s `referenceBuffer`, so it is resolved
    /// from the skeleton it names and not from a render object. The
    /// skeleton's `OnObjectUpdate` (`skeleton.update()`, once per frame) runs
    /// when the buffer is written.
    SkeletonBoneMatrices(SkeletonRef),
    /// `LightProbeNode.lightProbe` — `uniformArray( 9 × Vector3 )` holding the
    /// probe at this index of the renderer's light list, each coefficient
    /// already multiplied by `light.intensity` (`LightProbeNode.update()`).
    /// One `vec4` per coefficient, rewritten per draw like
    /// [`BufferSource::MorphInfluences`], because the probe can change between
    /// frames while the program does not.
    LightProbe(usize),
    /// `uniformArray( values )` over an array the application keeps changing
    /// — `LightProbeHelper`'s `uniformArray( lightProbe.sh.coefficients )`,
    /// which shares the probe's own `Vector3`s. The reader returns the
    /// elements already padded to four floats each; the buffer is rewritten
    /// per draw. Compares by identity, as [`UniformSource::Live`] does.
    Live(LiveValue),
}

impl BufferSource {
    /// A `var<storage>` binding rather than a `var<uniform>` one — every
    /// `StorageBufferNode` shape the port has.
    pub(crate) fn is_storage(&self) -> bool {
        matches!(
            self,
            BufferSource::Storage
                | BufferSource::AtomicStorage
                | BufferSource::Struct { .. }
                | BufferSource::StorageData { .. }
        )
    }

    /// The group the binding joins and the name three gives it: the two
    /// camera arrays are `renderGroup` buffers named by `setName()`, and
    /// everything else is an object-group `NodeBuffer_N` (`None`).
    pub(crate) fn group_and_name(&self) -> (UniformGroup, Option<&'static str>) {
        match self {
            BufferSource::CameraViewMatrices => (UniformGroup::Render, Some("cameraViewMatrices")),
            BufferSource::CameraProjectionMatrices => {
                (UniformGroup::Render, Some("cameraProjectionMatrices"))
            }
            _ => (UniformGroup::Object, None),
        }
    }
}

/// The skeleton behind [`BufferSource::SkeletonBoneMatrices`], compared by
/// identity: one skeleton is one buffer however many kernels read it.
#[derive(Clone)]
pub struct SkeletonRef(pub(crate) Rc<RefCell<crate::objects::Skeleton>>);

impl PartialEq for SkeletonRef {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}

/// One member of a [`StructLayout`] — an entry of `struct( { … } )`'s object.
#[derive(Clone, Debug, PartialEq)]
pub struct StructMember {
    /// The member's name in WGSL.
    pub name: &'static str,
    /// The member's WGSL type.
    pub ty: Type,
    /// `{ type: 'uint', atomic: true }` — declared `atomic< u32 >`.
    pub atomic: bool,
}

/// `struct( members, name )` — `StructTypeNode`: a named WGSL struct whose
/// members keep their declaration order.
#[derive(Clone, Debug, PartialEq)]
pub struct StructLayout {
    /// The struct's name in WGSL.
    pub name: &'static str,
    /// The struct's members, in declaration order.
    pub members: Vec<StructMember>,
}

impl StructLayout {
    /// `struct DrawBuffer {\n\tvertexCount : u32,\n … };` —
    /// `WGSLNodeBuilder.getStructMembers()`'s spelling, `atomic< u32 >` with
    /// the spaces three puts inside it.
    pub(crate) fn wgsl(&self) -> String {
        let members: Vec<String> = self
            .members
            .iter()
            .map(|m| {
                let ty = crate::nodes::wgsl::type_name(m.ty);
                if m.atomic {
                    format!("\t{} : atomic< {ty} >", m.name)
                } else {
                    format!("\t{} : {ty}", m.name)
                }
            })
            .collect();
        format!("struct {} {{\n{}\n}};", self.name, members.join(",\n"))
    }

    /// The member's index, by name — `.get( name )`.
    pub(crate) fn member(&self, name: &str) -> usize {
        self.members
            .iter()
            .position(|m| m.name == name)
            .unwrap_or_else(|| panic!("three-rs: struct {} has no member {name}", self.name))
    }
}

/// `workgroupArray( type, count )` — `WorkgroupInfoNode`: a `var<workgroup>`
/// array every invocation of one workgroup shares.
///
/// Its identity is the array: two `workgroupArray()` calls are two arrays,
/// each named `WorkgroupArray_N` by the builder in first-use order.
#[derive(Debug)]
pub struct WorkgroupArrayDef {
    /// The array's element type.
    pub element_ty: Type,
    /// The number of elements.
    pub count: usize,
    /// `.toAtomic()`.
    pub atomic: bool,
}

/// The identity of one `BufferNode` / `InstanceBuffer`, from a never-reused
/// counter — the same shape as `BufferGeometry.id` and `Material.id`.
///
/// The renderer caches a `range()` buffer's one-and-only random fill under
/// this. It used to be `Rc::as_ptr( &buffer )`, which a *later* buffer
/// inherits the moment this one's material is dropped, and which would then be
/// served the dead buffer's fill: the same freed-address bug as issue #58's
/// geometry cache, silent rather than loud because the contents are random
/// either way.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct BufferId(usize);

impl BufferId {
    pub(crate) fn next() -> Self {
        thread_local! {
            static BUFFER_ID: Cell<usize> = const { Cell::new(0) };
        }
        BUFFER_ID.with(|id| {
            let next = id.get();
            id.set(next + 1);
            BufferId(next)
        })
    }

    /// The number itself, for keying on.
    pub fn get(&self) -> usize {
        self.0
    }
}

/// The CPU-side buffer behind one or more *instanced vertex attributes* —
/// three.js' `InstancedBufferAttribute` / `InstancedInterleavedBuffer`.
///
/// `RangeNode.setup()` and `createInstanceMatrixNode()` both branch on
/// `uniformBufferSize <= builder.getUniformBufferLimit()`: under the limit the
/// data is a uniform buffer indexed by `instanceIndex` ([`BufferNode`]), over it
/// an instanced attribute, which is what lifts the ~1024-instance cap the
/// 64 KiB uniform binding imposes. `Rc` identity is the buffer's identity, so
/// two `range( 0, 1 )` calls are two buffers with two different random fills,
/// exactly as two `RangeNode`s are in three.js.
#[derive(Debug)]
pub struct InstanceBuffer {
    /// This buffer's identity — see [`BufferId`].
    pub id: BufferId,
    /// Where the buffer's contents come from.
    pub source: BufferSource,
    /// The instance count — `InstancedBufferAttribute.count`.
    pub count: usize,
    /// Floats per instance, i.e. the attribute stride in components: 16 for the
    /// interleaved instance matrix (`new InstancedInterleavedBuffer( array, 16,
    /// 1 )`), 4 for a `range()`.
    pub item_size: usize,
}

/// `BufferNode` — `buffer( array, type, count )`.
#[derive(Debug)]
pub struct BufferNode {
    /// This buffer's identity — see [`BufferId`].
    pub id: BufferId,
    /// Where the buffer's contents come from.
    pub source: BufferSource,
    /// The type of one element.
    pub element_ty: Type,
    /// The number of elements.
    pub count: usize,
}

/// The texture a `TextureNode` / `CubeTextureNode` samples.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub enum TextureSource {
    /// `texture( Texture )` — a sampled `texture_2d<f32>`.
    Texture2D(Texture),
    /// `texture( DepthTexture )` — a sampled `texture_depth_2d`.
    Depth(DepthTexture),
    /// A `DepthTexture` with `compareFunction` set, bound as
    /// `texture_depth_2d` + `sampler_comparison` and read with
    /// `textureSampleCompare` — `ShadowNode`'s shadow map.
    ShadowMap(DepthTexture),
    /// `cubeTexture( CubeTexture )` — a sampled `texture_cube<f32>`.
    Cube(CubeTexture),
    /// `DataArrayTexture` — the morph-target data texture.
    DataArray(DataArrayTexture),
    /// `DataTexture` — `BatchedMesh`' matrices / colours / indirect tables.
    Data(DataTexture),
    /// A point light's shadow map — `cubeTexture( CubeDepthTexture )`.
    CubeDepth(CubeDepthTexture),
    /// `texture3D( Data3DTexture | Storage3DTexture )` — a sampled
    /// `texture_3d<f32>`.
    Texture3D(Data3DTexture),
    /// `storageTexture( StorageTexture )` — a `texture_storage_2d<format,
    /// access>`, what `textureStore()` writes and `.load()` reads.
    Storage(Texture, StorageAccess),
    /// `storageTexture( Storage3DTexture )` — `texture_storage_3d`.
    Storage3D(Data3DTexture, StorageAccess),
}

impl TextureSource {
    /// The texture's id — `Texture.id`, whichever class it is.
    pub(crate) fn id(&self) -> usize {
        match self {
            TextureSource::Texture2D(t) | TextureSource::Storage(t, _) => t.id(),
            TextureSource::Depth(t) | TextureSource::ShadowMap(t) => t.id(),
            TextureSource::Cube(t) => t.id(),
            TextureSource::DataArray(t) => t.id(),
            TextureSource::Data(t) => t.id(),
            TextureSource::CubeDepth(t) => t.id(),
            TextureSource::Texture3D(t) | TextureSource::Storage3D(t, _) => t.id(),
        }
    }

    /// Bound as a storage texture rather than a sampled one. A texture a
    /// kernel stores into and a material samples is two bindings of one
    /// texture, so the builder keys its binding names on this as well as on
    /// [`id`](Self::id).
    pub(crate) fn is_storage_binding(&self) -> bool {
        matches!(
            self,
            TextureSource::Storage(..) | TextureSource::Storage3D(..)
        )
    }
}

/// `NodeAccess` for a `StorageTextureNode` — the `access` of its
/// `texture_storage_*` declaration.
///
/// `StorageTextureNode`'s default is `WRITE_ONLY`; `WGSLNodeBuilder
/// .getStorageAccess()` forces `READ_ONLY` outside the compute stage, which the
/// builder applies when it declares the binding.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum StorageAccess {
    /// `texture_storage_2d<format, write>` — `StorageTextureNode`'s default.
    #[default]
    WriteOnly,
    /// `texture_storage_2d<format, read>` — forced outside the compute stage.
    ReadOnly,
    /// `texture_storage_2d<format, read_write>`.
    ReadWrite,
}

impl StorageAccess {
    /// `WGSLNodeBuilder`'s `accessNames`.
    pub(crate) fn wgsl(self) -> &'static str {
        match self {
            StorageAccess::WriteOnly => "write",
            StorageAccess::ReadOnly => "read",
            StorageAccess::ReadWrite => "read_write",
        }
    }

    /// The `wgpu` spelling, for the bind group layout.
    pub(crate) fn wgpu(self) -> wgpu::StorageTextureAccess {
        match self {
            StorageAccess::WriteOnly => wgpu::StorageTextureAccess::WriteOnly,
            StorageAccess::ReadOnly => wgpu::StorageTextureAccess::ReadOnly,
            StorageAccess::ReadWrite => wgpu::StorageTextureAccess::ReadWrite,
        }
    }
}

impl TextureSource {
    /// The texture's liveness, for the renderer's view cache to sweep on; see
    /// [`TextureOwner`](crate::textures::TextureOwner).
    pub(crate) fn owner(&self) -> crate::textures::TextureOwner {
        match self {
            TextureSource::Texture2D(texture) => texture.owner(),
            TextureSource::Depth(depth) | TextureSource::ShadowMap(depth) => depth.owner(),
            TextureSource::Cube(cube) => cube.owner(),
            TextureSource::DataArray(data) => data.owner(),
            TextureSource::Data(data) => data.owner(),
            TextureSource::CubeDepth(cube) => cube.owner(),
            TextureSource::Texture3D(data) | TextureSource::Storage3D(data, _) => data.owner(),
            TextureSource::Storage(texture, _) => texture.owner(),
        }
    }
}

/// How a `TextureNode` reads its texture — `WGSLNodeBuilder.generateTexture*`.
#[derive(Clone, Debug)]
pub enum SampleMode {
    /// `textureSample( t, t_sampler, uv )`.
    Sample,
    /// `textureSampleLevel( t, t_sampler, uv, level )`.
    Level(NodeRef),
    /// `textureSampleBias( t, t_sampler, uv, bias )` — `textureNode.bias(
    /// value )`. `FXAANode` samples its input at a bias of `-100`, pinning
    /// every tap to the top mip.
    Bias(NodeRef),
    /// `textureSampleGrad( t, t_sampler, uv, gradX, gradY )` —
    /// `textureNode.grad( gradX, gradY )` (`generateTextureGrad()`), each
    /// gradient built as a `vec2`.
    Grad(NodeRef, NodeRef),
    /// `textureGather( component, t, t_sampler, uv[, offset] )` —
    /// `textureNode.gather( component )`, optionally `.offset( ivec2 )`
    /// (`WGSLNodeBuilder.generateTextureGather()`): one channel of the four
    /// texels a bilinear tap would read, from mip level 0.
    Gather {
        /// Which channel to gather (0–3).
        component: NodeRef,
        /// An optional texel offset — `.offset( ivec2 )`.
        offset: Option<NodeRef>,
    },
    /// `textureGatherCompare( t, t_sampler, uv, depth[, offset] )` —
    /// `depthNode.gather().compare( depth )` on a depth texture with a
    /// comparison sampler (`generateTextureGatherCompare()`): the four
    /// texels' comparison results.
    GatherCompare {
        /// The depth value each of the four texels is compared against.
        compare: NodeRef,
        /// An optional texel offset — `.offset( ivec2 )`.
        offset: Option<NodeRef>,
    },
    /// The non-filterable path: `textureLoad` against `textureDimensions`,
    /// with no sampler binding at all. What Three emits for a depth texture.
    Load,
    /// `textureLoad( t, coord, layer, u32( 0u ) )` on a 2-D-array texture —
    /// `textureLoad( … ).depth( layer )`, with no clamping and no sampler.
    LoadLayer(NodeRef),
    /// `textureSample( t, t_sampler, uv, i32( layer ) )` on a 2-D-array
    /// texture — `texture( map, uv ).depth( layer )`. Three builds the depth
    /// node as `'int'`, so a float layer arrives truncated by `i32()`.
    SampleLayer(NodeRef),
    /// `textureSampleCompare( t, t_sampler, uv, depth )` — the depth-compare
    /// read `ShadowFilterNode`'s `depthCompare` lowers to.
    Compare(NodeRef),
    /// `textureLoad( t, coord, u32( 0u ) )` — an unclamped texel fetch at an
    /// integer coordinate, what `Batch.js` reads its data textures with. When
    /// the node's type is a scalar the snippet is swizzled (`.x`), which is
    /// how Three's `textureLoad( … ).x` on the `r32uint` indirect table lands
    /// in one `u32` property.
    LoadTexel,
    /// `storageTexture( t ).load( coord )` — `textureLoad( t, coord )` on a
    /// `texture_storage_*`, which takes no level argument
    /// (`WGSLNodeBuilder.generateStorageTextureLoad()`).
    StorageLoad,
}

/// A WGSL builtin input.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[non_exhaustive]
pub enum Builtin {
    /// `@builtin( vertex_index )`.
    VertexIndex,
    /// `@builtin( instance_index )`.
    InstanceIndex,
    /// `@builtin( position )` in the fragment stage.
    FragCoord,
    /// `@builtin( front_facing )` — `FrontFacingNode`.
    FrontFacing,
    /// `invocationLocalIndex` — `@builtin( local_invocation_index )`, which
    /// `WGSLNodeBuilder.getInvocationLocalIndex()` adds to the compute entry
    /// point's parameters only when a kernel reads it.
    InvocationLocalIndex,
    /// `workgroupId` / `localId` / `globalId` / `numWorkgroups` —
    /// `ComputeBuiltinNode`. The compute entry point always declares these four
    /// (`WGSLNodeBuilder.getAttributes( 'compute' )`), so reading one only names
    /// the parameter.
    WorkgroupId,
    /// `@builtin( local_invocation_id )` — see [`Builtin::WorkgroupId`].
    LocalId,
    /// `@builtin( global_invocation_id )` — see [`Builtin::WorkgroupId`].
    GlobalId,
    /// `@builtin( num_workgroups )` — see [`Builtin::WorkgroupId`].
    NumWorkgroups,
}

impl Builtin {
    pub(crate) fn name(self) -> &'static str {
        match self {
            Builtin::VertexIndex => "vertexIndex",
            Builtin::InstanceIndex => "instanceIndex",
            Builtin::FragCoord => "fragCoord",
            Builtin::FrontFacing => "isFront",
            Builtin::InvocationLocalIndex => "invocationLocalIndex",
            Builtin::WorkgroupId => "workgroupId",
            Builtin::LocalId => "localId",
            Builtin::GlobalId => "globalId",
            Builtin::NumWorkgroups => "numWorkgroups",
        }
    }

    pub(crate) fn ty(self) -> Type {
        match self {
            Builtin::VertexIndex | Builtin::InstanceIndex | Builtin::InvocationLocalIndex => {
                Type::U32
            }
            Builtin::WorkgroupId
            | Builtin::LocalId
            | Builtin::GlobalId
            | Builtin::NumWorkgroups => Type::UVec3,
            Builtin::FragCoord => Type::Vec4,
            Builtin::FrontFacing => Type::Bool,
        }
    }
}

/// `VarNode` — a `var<private>` that caches its value's first use.
#[derive(Debug)]
pub struct VarDef {
    /// `toVar( 'name' )`; `None` numbers it `nodeVarN`. A `String` because a
    /// sub-build layer prefixes the name with its own (`NORMAL_normalView`) —
    /// see `docs/nodes.md` §7.
    pub name: Option<String>,
    /// The node whose value is cached.
    pub value: NodeRef,
    /// The variable's WGSL type.
    pub ty: Type,
}

/// `VaryingNode` — a value computed in the vertex stage and interpolated.
#[derive(Debug)]
pub struct VaryingDef {
    /// `toVarying( 'name' )`; `None` numbers it `nodeVaryingN`.
    pub name: Option<&'static str>,
    /// The node whose value is carried across stages.
    pub value: NodeRef,
    /// The varying's WGSL type.
    pub ty: Type,
    /// u32 varyings need `@interpolate(flat, either)`.
    pub flat: bool,
}

/// `ShaderNode` / `Fn()`. With `layout` set, a real WGSL `fn` is emitted and
/// called; without it the body is inlined at every call site — which is what
/// Three does for `saturation`/`hue`, and why their expressions appear
/// expanded three times in the dumped shader.
pub struct FnDef {
    /// The `fn`'s name in WGSL when `layout` is set; `None` numbers it.
    pub name: Option<&'static str>,
    /// The parameters' names and types, in declaration order.
    pub params: Vec<(&'static str, Type)>,
    /// The return type.
    pub ret: Type,
    /// Whether a real WGSL `fn` is emitted, rather than inlining the body.
    pub layout: bool,
    /// Builds the call's result node from the argument nodes.
    #[allow(clippy::type_complexity)]
    pub body: Box<dyn Fn(&[NodeRef]) -> NodeRef>,
}

impl std::fmt::Debug for FnDef {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FnDef").field("name", &self.name).finish()
    }
}

/// A node type defined outside the crate: the port of `class MyNode extends
/// Node` with a `setup( builder )` override, the shape every addon in
/// `examples/jsm/tsl/` has.
///
/// **`setup` composes; it never emits.** It returns a graph of the existing
/// [`Node`] variants, and the builder builds that graph in its place. It
/// cannot write a WGSL statement of its own, declare a binding, or change how
/// a variant is generated. Three's addons never override `generate` either:
/// they override `setup` (and `updateBefore`, which is #162). A node that needs
/// a statement shape nothing can compose into belongs in the crate as a
/// variant, where the exhaustive `match` in the builder and the dump gates see
/// it (#155 decision 2).
///
/// The builder calls `setup` once per build, the first time it reaches the
/// node, and keeps the result as three keeps `nodeProperties.outputNode`: in
/// the builder's per-build data, keyed on the node's identity and scoped by
/// [`isolate`](crate::nodes::tsl::isolate). Two reaches of one
/// `Node::Custom` share one expansion; two `Node::Custom`s over equal structs
/// are two nodes. See `docs/nodes.md` §45.
///
/// ```ignore
/// struct Double(NodeRef);
///
/// impl CustomNode for Double {
///     fn type_name(&self) -> &'static str { "DoubleNode" }
///     fn node_type(&self) -> Type { self.0.ty() }
///     fn setup(&self, _: &NodeBuilder) -> NodeRef { self.0.clone().mul(2.0) }
/// }
///
/// let doubled = custom(Double(uv().x()));
/// ```
pub trait CustomNode {
    /// three's `static get type()`: the class name, `'RGBShiftNode'`. Only
    /// `Debug` output and panic messages read it.
    fn type_name(&self) -> &'static str;

    /// `Node.getNodeType( builder )`: the WGSL type of the value `setup`
    /// returns. The port needs it before the build, because the TSL methods
    /// that wrap this node (`.mul()`, `.x()`, …) type their result eagerly.
    fn node_type(&self) -> Type;

    /// `Node.isCacheable( builder )`: whether a node reached more than once
    /// is built into a var once and read from there. r187dev's `Node.build()`
    /// does that for every cacheable node with a value (`cacheResult`), not
    /// only for `TempNode`s, so the default is `true`, as three's is.
    /// `false` builds the output again at every reach.
    fn is_cacheable(&self) -> bool {
        true
    }

    /// `Node.setup( builder )`: the graph this node stands for.
    ///
    /// `builder` is shared, not mutable: the only thing to ask it is
    /// [`context`](crate::nodes::NodeBuilder::context), three's
    /// `builder.context`, as installed by an enclosing
    /// [`context`](crate::nodes::tsl::context) node. The TSL functions called
    /// here see the same context through the crate's own accessors.
    fn setup(&self, builder: &crate::nodes::NodeBuilder) -> NodeRef;

    /// `updateBeforeType`. Anything but `None` puts the node in the
    /// material's update-before list; see `docs/nodes.md` §57.
    fn update_before_type(&self) -> crate::nodes::NodeUpdateType {
        crate::nodes::NodeUpdateType::None
    }
    /// `updateType`.
    fn update_type(&self) -> crate::nodes::NodeUpdateType {
        crate::nodes::NodeUpdateType::None
    }
    /// `updateAfterType`.
    fn update_after_type(&self) -> crate::nodes::NodeUpdateType {
        crate::nodes::NodeUpdateType::None
    }
    /// `updateBefore( frame )`. `false` leaves the guard untouched, as
    /// three's `=== false` does. See [`NodeUpdate`](crate::nodes::NodeUpdate).
    fn update_before(&self, _renderer: &mut crate::renderer::Renderer) -> bool {
        true
    }
    /// `update( frame )`.
    fn update(&self, _renderer: &mut crate::renderer::Renderer) -> bool {
        true
    }
    /// `updateAfter( frame )`.
    fn update_after(&self, _renderer: &mut crate::renderer::Renderer) -> bool {
        true
    }
}

/// A [`CustomNode`] seen through [`NodeUpdate`](crate::nodes::NodeUpdate), so
/// the renderer drives it like any other updating node.
pub(crate) struct CustomUpdate(pub(crate) Rc<dyn CustomNode>);

impl crate::nodes::NodeUpdate for CustomUpdate {
    fn update_before_type(&self) -> crate::nodes::NodeUpdateType {
        self.0.update_before_type()
    }
    fn update_type(&self) -> crate::nodes::NodeUpdateType {
        self.0.update_type()
    }
    fn update_after_type(&self) -> crate::nodes::NodeUpdateType {
        self.0.update_after_type()
    }
    fn update_before(&self, renderer: &mut crate::renderer::Renderer) -> bool {
        self.0.update_before(renderer)
    }
    fn update(&self, renderer: &mut crate::renderer::Renderer) -> bool {
        self.0.update(renderer)
    }
    fn update_after(&self, renderer: &mut crate::renderer::Renderer) -> bool {
        self.0.update_after(renderer)
    }
}

impl std::fmt::Debug for dyn CustomNode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.type_name())
    }
}

/// `context( node, { … } )`'s value: the `builder.context` keys a
/// [`Node::Context`] installs for its subgraph, merged over the ones already
/// in force (`builder.addContext()`).
///
/// Three's values are arbitrary JS; the port's are nodes (`getViewZ: () =>
/// scenePassViewZ` is `.set( "getViewZ", scene_pass_view_z )`), held in
/// `BuildContext`'s string-keyed `extra` map, which is where #155 decision 6
/// put addon keys. The typed core keys (`setupNormal`, the material side, …)
/// are installed by the material's own setup and cannot be set from here.
#[derive(Clone, Debug, Default)]
pub struct ContextValue {
    pub(crate) entries: Vec<(&'static str, NodeRef)>,
    /// `{ uniformFlow: … }`, which is not a node: see
    /// [`uniform_flow`](Self::uniform_flow).
    pub(crate) uniform_flow: Option<bool>,
    /// `{ nodeName: … }`, which is not a node: see
    /// [`node_name`](Self::node_name).
    pub(crate) node_name: Option<&'static str>,
}

impl ContextValue {
    /// An empty set of context entries.
    pub fn new() -> Self {
        Self::default()
    }

    /// `{ …, key: value }`. A later `set` of the same key wins, as the later
    /// of two properties does in a JS object literal.
    pub fn set(mut self, key: &'static str, value: impl Into<NodeRef>) -> Self {
        self.entries.retain(|(k, _)| *k != key);
        self.entries.push((key, value.into()));
        self
    }

    /// `{ …, uniformFlow: on }` — what [`uniform_flow`] installs. Under it a
    /// `select()` with both branches is WGSL's `select( f, t, cond )` rather
    /// than an `if`/`else` over a var (`ConditionalNode.generate()`), so it
    /// stays in uniform control flow.
    ///
    /// [`uniform_flow`]: crate::nodes::tsl::uniform_flow
    pub fn uniform_flow(mut self, on: bool) -> Self {
        self.uniform_flow = Some(on);
        self
    }

    /// `{ …, nodeName: name }` — what [`set_name`] installs: the name the
    /// first unnamed `uniform()` built inside takes, which then clears it.
    ///
    /// [`set_name`]: crate::nodes::tsl::set_name
    pub fn node_name(mut self, name: &'static str) -> Self {
        self.node_name = Some(name);
        self
    }
}

/// What [`debug`](crate::nodes::tsl::debug)'s callback is handed: three's
/// `callback( builder, snippet )`, with the two things of the builder a
/// debugging callback reads.
#[derive(Clone, Copy, Debug)]
pub struct DebugInfo<'a> {
    /// `builder.shaderStage`: `"vertex"`, `"fragment"` or `"compute"`.
    pub stage: &'static str,
    /// `builder.flow.code`: the statements emitted so far in the current
    /// scope (the stage's `main`, or the `Fn()` being built), one per line,
    /// with one level of indentation removed.
    pub flow: &'a str,
    /// The debugged node's snippet, which is also the debug node's own.
    pub snippet: &'a str,
}

/// A [`debug`](crate::nodes::tsl::debug) callback.
#[derive(Clone)]
pub struct DebugCallback(pub(crate) Rc<dyn Fn(&DebugInfo<'_>)>);

impl DebugCallback {
    /// Wrap `callback`.
    pub fn new(callback: impl Fn(&DebugInfo<'_>) + 'static) -> Self {
        Self(Rc::new(callback))
    }
}

impl std::fmt::Debug for DebugCallback {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("DebugCallback")
    }
}

/// The node set. Closed on purpose: an exhaustive `match` in the builder is
/// what tells the next rung it has added something the generator cannot emit.
/// [`Node::Custom`] is the one opening, and it can only compose the others.
#[derive(Debug)]
#[non_exhaustive]
pub enum Node {
    /// A literal. `values` holds one entry per component.
    Const {
        /// The literal's WGSL type.
        ty: Type,
        /// One entry per component.
        values: Vec<f64>,
    },
    /// `array< f32, N >( … )` — `QuadMesh`'s `vertexNode`.
    ConstArray {
        /// The array's element type.
        element_ty: Type,
        /// The array's contents, one entry per component of each element.
        values: Vec<f64>,
    },
    /// The same literal array held in a `var<private> nodeVarN : array< T, N >`
    /// rather than written out at each use — three.js' `TempNode` promotion of
    /// an `array()` that the flow reads more than once, which `BloomNode`'s
    /// composite does five times (`dump/m11`).
    ArrayVar {
        /// The array's element type.
        element_ty: Type,
        /// The array's contents, one entry per component of each element.
        values: Vec<f64>,
    },
    /// `UniformNode`.
    Uniform(Rc<UniformNode>),
    /// `BufferNode` element access: `NodeBuffer_N.value[ index ]`.
    BufferElement {
        /// The buffer being indexed.
        buffer: Rc<BufferNode>,
        /// The element index.
        index: NodeRef,
    },
    /// A geometry attribute.
    Attribute {
        /// The attribute's name in WGSL.
        name: &'static str,
        /// The attribute's WGSL type.
        ty: Type,
    },
    /// `instancedBufferAttribute( buffer, type, stride, offset )`: a vertex
    /// attribute whose buffer steps once per instance. `offset` is in floats
    /// from the start of the instance; the stride is the buffer's `item_size`,
    /// which is all three.js' own call sites use.
    InstancedAttribute {
        /// The per-instance buffer.
        buffer: Rc<InstanceBuffer>,
        /// The offset into one instance's data, in floats.
        offset: usize,
        /// The attribute's WGSL type.
        ty: Type,
    },
    /// A WGSL builtin input.
    Builtin(Builtin),
    /// `ComputeNode` used as a value — `Fn( () => { …; return x } )().compute(
    /// count )` set as a material's `positionNode`. Outside the compute stage
    /// it generates `output` (`properties.outputComputeNode`); the kernel
    /// itself runs from `updateBefore()` (`NodeUpdateType.FRAME`), which the
    /// builder records in
    /// [`NodeProgram::update_before`](crate::nodes::NodeProgram) for the
    /// renderer to dispatch. See `docs/nodes.md` §44 and §57.
    Compute {
        /// The compute kernel's statements and dispatch size.
        flow: Rc<crate::nodes::ComputeFlow>,
        /// The value read outside the compute stage.
        output: NodeRef,
    },
    /// `VarNode` — a cached `var<private>`.
    Var(Rc<VarDef>),
    /// `VarNode` with `intent` set, as it ends up when something assigns to
    /// it — see [`to_var_intent`](crate::nodes::tsl::to_var_intent). A WGSL
    /// function-scope `var`, declared with its value where it is first
    /// built (`var nodeVar0 : vec3<f32> = …;`) instead of hoisted into the
    /// `// vars` block.
    VarIntent(Rc<VarDef>),
    /// `VarNode` with `readOnly` set — `node.toConst()`. A WGSL `let`, so it is
    /// declared where it is assigned and, unlike a `var<private>`, cannot be
    /// written again.
    Let(Rc<VarDef>),
    /// `VaryingNode`.
    Varying(Rc<VaryingDef>),
    /// A `var<private>` with a fixed name that the setup code assigns
    /// explicitly — `PropertyNode` (`DiffuseColor`, `Output`, …).
    Property {
        /// The property's name in WGSL.
        name: &'static str,
        /// The property's WGSL type.
        ty: Type,
    },
    /// A parameter of an emitted `fn` — a name that is already in scope.
    Param {
        /// The parameter's name in WGSL.
        name: &'static str,
        /// The parameter's WGSL type.
        ty: Type,
    },
    /// A statement: `target = value`.
    Assign {
        /// The assignment's left-hand side.
        target: NodeRef,
        /// The assignment's right-hand side.
        value: NodeRef,
    },
    /// `OperatorNode`.
    Op {
        /// The WGSL operator, e.g. `"+"`.
        op: &'static str,
        /// The left operand.
        a: NodeRef,
        /// The right operand.
        b: NodeRef,
        /// The result's WGSL type.
        ty: Type,
    },
    /// `MathNode` — a WGSL builtin call.
    Math {
        /// The WGSL builtin's name, e.g. `"sin"`.
        name: &'static str,
        /// The call's arguments.
        args: Vec<NodeRef>,
        /// The result's WGSL type.
        ty: Type,
    },
    /// `SplitNode`.
    Swizzle {
        /// The node being swizzled.
        node: NodeRef,
        /// The swizzle mask, e.g. `"xyz"`.
        components: &'static str,
        /// The result's WGSL type.
        ty: Type,
    },
    /// `ConvertNode` / a single-argument constructor: `vec4<f32>( x )`.
    Cast {
        /// The node being converted.
        node: NodeRef,
        /// The target WGSL type.
        ty: Type,
    },
    /// `OperatorNode` with one operand: `( - x )`.
    Neg {
        /// The negated node.
        node: NodeRef,
        /// The result's WGSL type.
        ty: Type,
    },
    /// `JoinNode` — `vec4<f32>( a, b, c, d )`.
    Join {
        /// The joined components.
        args: Vec<NodeRef>,
        /// The result's WGSL type.
        ty: Type,
    },
    /// `ArrayElementNode`: `m[ 3u ]`, `array< f32, 3 >( … )[ vertexIndex ]`.
    Element {
        /// The array or vector being indexed.
        node: NodeRef,
        /// The element index.
        index: NodeRef,
        /// The element's WGSL type.
        ty: Type,
    },
    /// `TextureNode` — a texture sample.
    Texture {
        /// The texture being sampled.
        texture: Rc<TextureSource>,
        /// The sample coordinate.
        uv: NodeRef,
        /// How the texture is read.
        mode: SampleMode,
        /// The result's WGSL type.
        ty: Type,
    },
    /// `TextureSizeNode` — `textureDimensions( t, level )`, a `vec2<u32>`.
    TextureSize {
        /// The texture being measured.
        texture: Rc<TextureSource>,
        /// The mip level.
        level: NodeRef,
    },
    /// `varyingProperty( type, name )` — a *named* varying assigned to by
    /// statement rather than built from a value. Unlike [`Node::Varying`] it
    /// is declared as soon as either stage mentions it, so a varying the
    /// fragment stage never reads still appears in `VaryingsStruct`, exactly
    /// as `vBatchIndirectId` does in Three's dump.
    VaryingProperty {
        /// The varying's name in WGSL.
        name: &'static str,
        /// The varying's WGSL type.
        ty: Type,
        /// Whether the varying needs `@interpolate(flat, either)`.
        flat: bool,
    },
    /// A call into a [`Node::Code`] / inlined `Fn()`.
    Call {
        /// The function being called.
        def: Rc<FnDef>,
        /// The call's arguments.
        args: Vec<NodeRef>,
    },
    /// `CodeNode` itself — a `wgslFn()` as a node rather than as a call, which
    /// is the shape another `wgslFn`'s `includes` list needs it in. Building
    /// one emits its declaration into `// codes` and nothing else; it has no
    /// value, so nothing but an `includes` list may hold one.
    Code(Rc<crate::nodes::code::CodeDef>),
    /// `FunctionCallNode` over a `wgslFn()` — a call into hand-written WGSL
    /// the node system copies through verbatim. See [`crate::nodes::code`].
    CodeCall {
        /// The hand-written WGSL function being called.
        def: Rc<crate::nodes::code::CodeDef>,
        /// The call's arguments.
        args: Vec<NodeRef>,
    },
    /// A sequence of statements followed by the value they produce — the shape
    /// an inlined `Fn()` body with `toVar()` statements has. Three has no node
    /// for it: its `ShaderNode` call simply flows its body's statements into the
    /// current stage and returns the last expression, which is what this does.
    Block {
        /// The statements built ahead of `result`.
        statements: Vec<NodeRef>,
        /// The block's value.
        result: NodeRef,
    },
    /// `Loop( count, ( { i } ) => { … } )` — `for ( var i : i32 = 0; i < n; i ++ )`.
    Loop {
        /// `Loop( { start, end }, … )`'s `start`. `None` is three.js'
        /// `Loop( count, … )` shorthand, whose start is the literal `0`; a
        /// node start is written out as it stands, so
        /// `webgpu_postprocessing_anamorphic`'s negated half-sample count
        /// reaches the loop header as `i32( ( - nodeVar1 ) )`.
        start: Option<NodeRef>,
        /// `Loop( { start, end }, … )`'s `end`, or `Loop( count, … )`'s
        /// count — the loop's exclusive upper bound.
        count: NodeRef,
        /// The loop index, as it appears inside `body` (`Node::Param`). Its
        /// type is `Loop( { type } )`: `i32`, or `f32` for `hashBlur`'s
        /// `type: 'float'`, which also changes the step to `i += 1.`.
        index: NodeRef,
        /// `Loop( { condition } )` — `"<"` unless the caller asked for
        /// another comparison (`boxBlur`'s `"<="`).
        condition: &'static str,
        /// `Loop( { update } )` — `i += update` in place of the default step.
        update: Option<NodeRef>,
        /// The loop body's statements.
        body: Vec<NodeRef>,
    },
    /// `Break()` — a bare `break;` out of the innermost `Loop`.
    Break,
    /// `textureStore( storageTexture, coord, value )` — a statement.
    ///
    /// `StorageTextureNode.generateStore()` writes the coordinate as
    /// `vec2<u32>( … )` (`vec3<u32>` for a 3D texture) around whatever the
    /// caller passed, so a `uvec2` coordinate is wrapped once more, exactly as
    /// three's dump shows.
    TextureStore {
        /// The storage texture being written.
        texture: Rc<TextureSource>,
        /// The texel coordinate.
        coord: NodeRef,
        /// The value stored.
        value: NodeRef,
    },
    /// `If( cond, () => { … } )` as a bare statement (`setupDiscard`),
    /// optionally with the `.Else( … )` / `.ElseIf( … )` arm `StackNode` adds.
    ///
    /// `ElseIf` is not a shape of its own: `StackNode.ElseIf()` puts a whole
    /// nested `If` inside the else block, so `If( a ).ElseIf( b )` is
    /// `else_body: vec![ if_then( b, … ) ]` and generates as a nested
    /// `if`/`else`, which is exactly what three emits.
    If {
        /// The branch condition.
        cond: NodeRef,
        /// The `if` block's statements.
        body: Vec<NodeRef>,
        /// Empty for a one-armed `If`, in which case no `else` is emitted and
        /// the generated text is byte-identical to what it was before the arm
        /// existed.
        else_body: Vec<NodeRef>,
    },
    /// `If( cond, () => { … } )` — a one-armed conditional over a result var
    /// that was initialised before it. `pre` holds the statements three.js
    /// emits ahead of the result var (its `toConst` lines), `result` is the var
    /// itself and `body` the statements inside the block, the last of which
    /// assigns `result`. The node's value is the result var.
    IfVar {
        /// The statements built ahead of `result`.
        pre: Vec<NodeRef>,
        /// The initialised result variable.
        result: NodeRef,
        /// The branch condition.
        cond: NodeRef,
        /// The block's statements, the last of which assigns `result`.
        body: Vec<NodeRef>,
    },
    /// `Discard()` — a bare `discard;`.
    Discard,
    /// `return value;` inside an emitted `fn` — what an `If( cond, () => {
    /// return x; } )` in a `Fn()` body compiles to (`neutralToneMapping`'s
    /// early out).
    Return {
        /// The returned value.
        value: NodeRef,
    },
    /// `x.not()` — `( ! x )`. A `bool`, or a `bvecN` for an `N`-vector
    /// operand (`OperatorNode.getNodeType()`'s `'!'` arm).
    Not {
        /// The negated node.
        node: NodeRef,
    },
    /// `x.bitNot()` — `( ~ x )`, typed `getIntegerType( typeA )`.
    BitNot {
        /// The negated node.
        node: NodeRef,
        /// The result's WGSL type.
        ty: Type,
    },
    /// `cond.select( a, b )` — lowered to an `if`/`else` writing a result var,
    /// exactly as Three does.
    Select {
        /// The branch condition.
        cond: NodeRef,
        /// The value when `cond` is true.
        a: NodeRef,
        /// The value when `cond` is false.
        b: NodeRef,
        /// The result's WGSL type.
        ty: Type,
    },
    /// `storageStruct.get( 'member' )` — `MemberNode` on a
    /// [`BufferSource::Struct`] storage buffer: `NodeBuffer_N.member`.
    StructMember {
        /// The struct buffer.
        buffer: Rc<BufferNode>,
        /// The member's index in the struct's layout.
        member: usize,
    },
    /// `AtomicFunctionNode` — `atomicStore( &pointer, value )` and the rest of
    /// the family. `pointer` is a struct member or buffer element declared
    /// `atomic< T >`. As a bare statement it is one `atomicX( … );` line; read
    /// as a value as well, its result is held in a `let` first.
    Atomic {
        /// The WGSL atomic function's name, e.g. `"atomicAdd"`.
        method: &'static str,
        /// The atomic value being operated on.
        pointer: NodeRef,
        /// The operand, when the method takes one.
        value: Option<NodeRef>,
    },
    /// `WorkgroupInfoNode` — the array itself. Read through
    /// [`Node::Element`], which is how `.element( i )` reaches it.
    Workgroup(Rc<WorkgroupArrayDef>),
    /// `BarrierNode` — `workgroupBarrier()` / `storageBarrier()` /
    /// `textureBarrier()`, `scope` being the prefix.
    Barrier {
        /// The barrier's WGSL prefix, e.g. `"workgroup"`.
        scope: &'static str,
    },
    /// A node type defined outside the crate; see [`CustomNode`]. Built by
    /// building what its `setup` returns.
    Custom(Rc<dyn CustomNode>),
    /// `ContextNode` — `node.context( { … } )`. Builds `node` with `value`'s
    /// keys merged into `builder.context`, and restores the previous context
    /// afterwards.
    Context {
        /// The node built under the merged context.
        node: NodeRef,
        /// The context entries merged in.
        value: Rc<ContextValue>,
    },
    /// `IsolateNode` — `isolate( node )`. Builds `node` in a `NodeCache` of
    /// its own, whose parent is the cache in force where the isolate is
    /// built: what `node`'s subgraph sets up, counts or declares for the first
    /// time stays inside it.
    Isolate {
        /// The node built in its own cache.
        node: NodeRef,
    },
    /// `ExpressionNode` — `expression( snippet, type )`: raw WGSL. A value
    /// type prints `snippet` where the node is read; `void` writes it to the
    /// flow as a statement. Not cacheable: each read prints it again.
    Expression {
        /// The WGSL, verbatim.
        snippet: Rc<str>,
        /// Its type, `Void` for a statement.
        ty: Type,
    },
    /// `DebugNode` — `debug( node, callback )`: `node`'s snippet, passed
    /// through after `callback` (or a log of the flow so far) has seen it.
    Debug {
        /// The node debugged.
        node: NodeRef,
        /// `null` logs the stage's flow code to stderr, as three's `log()`
        /// does to the console.
        callback: Option<DebugCallback>,
    },
    /// `structType( values )` — `StructNode`: a value of a [`struct_type`]
    /// (`StructTypeNode`) built from one value per member, in member order.
    /// Always held in a var of the struct's type, which is where three's
    /// `StructNode` lands too (`nodeVar31 = StructType0( … )` in the TRAA
    /// dump); its [`ty`](NodeRef::ty) is [`Type::Void`] because the port's
    /// `Type` has no struct case, and only [`Node::StructGet`] reads it.
    ///
    /// [`struct_type`]: crate::nodes::tsl::struct_type
    StructNew {
        /// The struct's layout.
        layout: Rc<StructLayout>,
        /// One value per member.
        values: Vec<NodeRef>,
    },
    /// `structNode.get( name )` — `MemberNode` over a [`Node::StructNew`]
    /// (or over a block or `Fn()` whose result is one): `{ var }.{ member }`.
    StructGet {
        /// The struct value read.
        value: NodeRef,
        /// Its layout.
        layout: Rc<StructLayout>,
        /// The member's index in the layout.
        member: usize,
    },
}

/// A handle on a node. Fluent TSL methods hang off this; see `tsl.rs`.
#[derive(Clone, Debug)]
pub struct NodeRef(Rc<Node>);

impl NodeRef {
    /// Wraps a node in a new handle.
    pub fn new(node: Node) -> Self {
        NodeRef(Rc::new(node))
    }

    /// The `Rc<Node>` this handle wraps.
    pub fn as_rc(&self) -> &Rc<Node> {
        &self.0
    }

    /// The `Node` this handle wraps.
    pub fn node(&self) -> &Node {
        &self.0
    }

    /// The identity the builder keys its per-node state on.
    pub fn key(&self) -> usize {
        Rc::as_ptr(&self.0) as *const u8 as usize
    }

    /// `Node.getNodeType( builder )`.
    pub fn ty(&self) -> Type {
        match &*self.0 {
            Node::Const { ty, .. } => *ty,
            Node::ConstArray { element_ty, .. } => *element_ty,
            Node::ArrayVar { element_ty, .. } => *element_ty,
            Node::Uniform(u) => u.ty,
            Node::BufferElement { buffer, .. } => buffer.element_ty,
            Node::Attribute { ty, .. } => *ty,
            Node::InstancedAttribute { ty, .. } => *ty,
            Node::Builtin(b) => b.ty(),
            Node::Var(v) => v.ty,
            Node::VarIntent(v) => v.ty,
            Node::Let(v) => v.ty,
            Node::Varying(v) => v.ty,
            Node::TextureSize { .. } => Type::UVec2,
            Node::VaryingProperty { ty, .. } => *ty,
            Node::Property { ty, .. } => *ty,
            Node::Param { ty, .. } => *ty,
            Node::Assign { target, .. } => target.ty(),
            Node::Op { ty, .. } => *ty,
            Node::Math { ty, .. } => *ty,
            Node::Swizzle { ty, .. } => *ty,
            Node::Cast { ty, .. } => *ty,
            Node::Neg { ty, .. } => *ty,
            Node::Join { ty, .. } => *ty,
            Node::Element { ty, .. } => *ty,
            Node::Texture { ty, .. } => *ty,
            Node::Call { def, .. } => def.ret,
            Node::Code(_) => Type::Void,
            Node::CodeCall { def, .. } => def.ret,
            Node::IfVar { result, .. } => result.ty(),
            Node::Select { ty, .. } => *ty,
            Node::Block { result, .. } => result.ty(),
            Node::Loop { .. }
            | Node::If { .. }
            | Node::Discard
            | Node::Break
            | Node::TextureStore { .. }
            | Node::Return { .. } => Type::Void,
            Node::Not { node } => Type::vector_of(Type::Bool, node.ty().components().max(1)),
            Node::BitNot { ty, .. } => *ty,
            Node::StructMember { buffer, member } => match &buffer.source {
                BufferSource::Struct { layout, .. } => layout.members[*member].ty,
                _ => unreachable!("three-rs: a struct member is only built on a struct buffer"),
            },
            // `AtomicFunctionNode.getNodeType()` is the pointer's type.
            Node::Atomic { pointer, .. } => pointer.ty(),
            Node::Workgroup(def) => def.element_ty,
            Node::Barrier { .. } => Type::Void,
            Node::Compute { output, .. } => output.ty(),
            Node::Custom(custom) => custom.node_type(),
            Node::Context { node, .. } | Node::Isolate { node } | Node::Debug { node, .. } => {
                node.ty()
            }
            Node::Expression { ty, .. } => *ty,
            Node::StructNew { .. } => Type::Void,
            Node::StructGet { layout, member, .. } => layout.members[*member].ty,
        }
    }
}

impl From<&NodeRef> for NodeRef {
    fn from(n: &NodeRef) -> Self {
        n.clone()
    }
}

/// `float( x )` from a Rust number — the `From` impls are what let TSL methods
/// take `impl Into<NodeRef>` and read like the JS.
macro_rules! const_from {
    ($t:ty, $ty:expr) => {
        impl From<$t> for NodeRef {
            fn from(v: $t) -> Self {
                NodeRef::new(Node::Const {
                    ty: $ty,
                    values: vec![v as f64],
                })
            }
        }
    };
}

const_from!(f64, Type::F32);
const_from!(f32, Type::F32);
const_from!(i32, Type::I32);
const_from!(u32, Type::U32);

impl From<bool> for NodeRef {
    fn from(v: bool) -> Self {
        NodeRef::new(Node::Const {
            ty: Type::Bool,
            values: vec![if v { 1.0 } else { 0.0 }],
        })
    }
}

impl From<Color> for NodeRef {
    fn from(c: Color) -> Self {
        NodeRef::new(Node::Const {
            ty: Type::Vec3,
            values: vec![c.r, c.g, c.b],
        })
    }
}

impl From<Vector2> for NodeRef {
    fn from(v: Vector2) -> Self {
        NodeRef::new(Node::Const {
            ty: Type::Vec2,
            values: vec![v.x, v.y],
        })
    }
}

impl From<Vector3> for NodeRef {
    fn from(v: Vector3) -> Self {
        NodeRef::new(Node::Const {
            ty: Type::Vec3,
            values: vec![v.x, v.y, v.z],
        })
    }
}

impl From<Matrix4> for NodeRef {
    fn from(m: Matrix4) -> Self {
        NodeRef::new(Node::Const {
            ty: Type::Mat4,
            values: m.elements.to_vec(),
        })
    }
}

/// A lazily-initialised accessor singleton. Three's `positionLocal` etc. are
/// module-level node objects, and their object identity is what makes the
/// builder emit one `var` shared by every reference; `thread_local!` + `Rc`
/// clone reproduces that.
pub(crate) struct Lazy<T: 'static> {
    cell: RefCell<Option<T>>,
}

impl<T: Clone + 'static> Default for Lazy<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: Clone + 'static> Lazy<T> {
    pub const fn new() -> Self {
        Lazy {
            cell: RefCell::new(None),
        }
    }

    pub fn get(&self, init: impl FnOnce() -> T) -> T {
        let mut slot = self.cell.borrow_mut();
        if slot.is_none() {
            *slot = Some(init());
        }
        slot.as_ref()
            .expect("three-rs: the lazy slot was filled in just above")
            .clone()
    }
}

// The program cache key hashes the binding descriptions, which bottom out in
// these three types. Their `Hash` impls are written by hand rather than derived
// so that they reach every field the *generated program* depends on and no
// field that is only a value: a texture's pixels, an instanced attribute's
// array. Three.js draws the same line in `Node.getCacheKey()`, where a texture
// contributes its uuid and never its image.

impl std::hash::Hash for UniformSource {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        std::mem::discriminant(self).hash(state);
        match self {
            UniformSource::LightColorIntensity(i)
            | UniformSource::LightCutoffDistance(i)
            | UniformSource::LightDecay(i)
            | UniformSource::LightViewPosition(i)
            | UniformSource::LightWorldPosition(i)
            | UniformSource::LightTargetPosition(i)
            | UniformSource::LightGroundColor(i)
            | UniformSource::LightConeCos(i)
            | UniformSource::LightPenumbraCos(i)
            | UniformSource::ShadowMatrix(i)
            | UniformSource::ShadowCameraNear(i)
            | UniformSource::ShadowCameraFar(i)
            | UniformSource::ShadowBias(i)
            | UniformSource::ShadowNormalBias(i)
            | UniformSource::ShadowRadius(i)
            | UniformSource::ShadowBlurSamples(i)
            | UniformSource::ShadowMapSize(i)
            | UniformSource::ShadowIntensity(i) => i.hash(state),
            // A baked `uniform( value )`: two materials can generate identical
            // WGSL and differ only here (a texture's uv matrix, say), so the
            // bits are part of the key.
            UniformSource::Value(values) => {
                values.len().hash(state);
                for value in values {
                    value.to_bits().hash(state);
                }
            }
            // Identity, never contents: the whole point is that the value
            // moves between draws while the program stays one program.
            UniformSource::Settable(cell) => cell.hash(state),
            UniformSource::Live(value) => value.hash(state),
            _ => {}
        }
    }
}

impl std::hash::Hash for BufferSource {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        std::mem::discriminant(self).hash(state);
        match self {
            BufferSource::Range { min, max } => {
                for value in min.iter().chain(max.iter()) {
                    value.to_bits().hash(state);
                }
            }
            // Identity, never contents — the array behind an instanced
            // attribute is megabytes and is resolved per draw anyway.
            BufferSource::Attribute(data) | BufferSource::UniformArray(data) => {
                (Rc::as_ptr(data) as *const u8 as usize).hash(state)
            }
            // The layout is spelled into the WGSL, which the key already
            // hashes; the initial contents are a value and stay out.
            BufferSource::Struct { layout, .. } => {
                layout.name.hash(state);
                for member in &layout.members {
                    member.name.hash(state);
                    member.ty.hash(state);
                    member.atomic.hash(state);
                }
            }
            // The array is a value; `.toReadOnly()` is spelt into the WGSL.
            BufferSource::StorageData { read_only, .. } => read_only.hash(state),
            BufferSource::SkeletonBoneMatrices(skeleton) => {
                (Rc::as_ptr(&skeleton.0) as *const u8 as usize).hash(state)
            }
            BufferSource::LightProbe(index) => index.hash(state),
            BufferSource::Live(value) => value.hash(state),
            BufferSource::InstanceMatrix
            | BufferSource::InstanceColor
            | BufferSource::MorphInfluences
            | BufferSource::BoneMatrices
            | BufferSource::PreviousBoneMatrices
            | BufferSource::CameraViewMatrices
            | BufferSource::CameraProjectionMatrices
            | BufferSource::Storage
            | BufferSource::AtomicStorage => {}
        }
    }
}

impl std::hash::Hash for TextureSource {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        std::mem::discriminant(self).hash(state);
        match self {
            // Identity only, as `Node.getCacheKey()` takes a texture's uuid.
            // The format, colour space, filters, wrapping and anisotropy are
            // *not* here: the compiled program's layout reads only the
            // binding's `kind` and `visibility` (`programs::layout_entry`),
            // and the view and sampler built from those fields are resolved
            // per draw. Neither the image data nor the `gpu` handle either —
            // the key must not move when the texture is uploaded.
            TextureSource::Texture2D(texture) => texture.id().hash(state),
            TextureSource::Depth(texture) | TextureSource::ShadowMap(texture) => {
                texture.id().hash(state)
            }
            TextureSource::Cube(texture) => texture.id().hash(state),
            TextureSource::DataArray(texture) => texture.id().hash(state),
            TextureSource::Data(texture) => texture.id().hash(state),
            TextureSource::CubeDepth(texture) => texture.id().hash(state),
            TextureSource::Texture3D(texture) => texture.id().hash(state),
            // The access is part of the declaration, so it is part of the key.
            TextureSource::Storage(texture, access) => {
                texture.id().hash(state);
                access.hash(state);
            }
            TextureSource::Storage3D(texture, access) => {
                texture.id().hash(state);
                access.hash(state);
            }
        }
    }
}

/// Derived but for `Attribute`, whose `Rc<Vec<f32>>` is the caller's whole
/// per-instance array — `BatchedText` hands it four floats per glyph.
impl std::fmt::Debug for BufferSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BufferSource::InstanceMatrix => f.write_str("InstanceMatrix"),
            BufferSource::InstanceColor => f.write_str("InstanceColor"),
            BufferSource::Storage => f.write_str("Storage"),
            BufferSource::AtomicStorage => f.write_str("AtomicStorage"),
            BufferSource::Struct { layout, init } => f
                .debug_struct("Struct")
                .field("layout", &layout.name)
                .field("init", &format_args!("{} words", init.len()))
                .finish(),
            BufferSource::Range { min, max } => f
                .debug_struct("Range")
                .field("min", min)
                .field("max", max)
                .finish(),
            BufferSource::MorphInfluences => f.write_str("MorphInfluences"),
            BufferSource::BoneMatrices => f.write_str("BoneMatrices"),
            BufferSource::PreviousBoneMatrices => f.write_str("PreviousBoneMatrices"),
            BufferSource::CameraViewMatrices => f.write_str("CameraViewMatrices"),
            BufferSource::CameraProjectionMatrices => f.write_str("CameraProjectionMatrices"),
            BufferSource::Attribute(data) => f
                .debug_tuple("Attribute")
                .field(&format_args!("{} floats", data.len()))
                .finish(),
            BufferSource::UniformArray(data) => f
                .debug_tuple("UniformArray")
                .field(&format_args!("{} floats", data.len()))
                .finish(),
            BufferSource::StorageData { init, read_only } => f
                .debug_struct("StorageData")
                .field("init", &format_args!("{} words", init.len()))
                .field("read_only", read_only)
                .finish(),
            BufferSource::SkeletonBoneMatrices(skeleton) => f
                .debug_tuple("SkeletonBoneMatrices")
                .field(&format_args!("{} bones", skeleton.0.borrow().bones.len()))
                .finish(),
            BufferSource::LightProbe(index) => f.debug_tuple("LightProbe").field(index).finish(),
            BufferSource::Live(value) => f.debug_tuple("Live").field(value).finish(),
        }
    }
}
