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
    CubeDepthTexture, CubeTexture, DataArrayTexture, DataTexture, DepthTexture, Texture,
};

/// A WGSL value type. Three carries these as strings (`'vec3'`); the closed set
/// is the part of `NodeBuilder`'s type vocabulary the ladder has reached.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Type {
    Void,
    Bool,
    F32,
    I32,
    U32,
    Vec2,
    Vec3,
    Vec4,
    UVec2,
    IVec2,
    /// `vec3<u32>` — MaterialX's `mx_hash_vec3` packs its three byte hashes
    /// into one.
    UVec3,
    /// `vec4<u32>` — the `skinIndex` attribute, which three.js declares
    /// `attribute( 'skinIndex', 'uvec4' )` and uploads as a `Uint32Array`.
    UVec4,
    BVec3,
    /// `mat2` — `RotateNode`'s vec2 path emits `mat2x2<f32>( cos, sin, -sin, cos )`.
    Mat2,
    Mat3,
    Mat4,
}

impl Type {
    /// `NodeBuilder.getTypeLength()`.
    pub fn components(self) -> usize {
        match self {
            Type::Void => 0,
            Type::Bool | Type::F32 | Type::I32 | Type::U32 => 1,
            Type::Vec2 | Type::UVec2 | Type::IVec2 => 2,
            Type::Vec3 | Type::UVec3 | Type::BVec3 => 3,
            Type::Vec4 | Type::UVec4 => 4,
            Type::Mat2 => 4,
            Type::Mat3 => 9,
            Type::Mat4 => 16,
        }
    }

    /// `NodeBuilder.getComponentType()`.
    pub fn component_type(self) -> Type {
        match self {
            Type::UVec2 | Type::UVec3 | Type::UVec4 => Type::U32,
            Type::IVec2 => Type::I32,
            Type::BVec3 => Type::Bool,
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
            (Type::Bool, 3) => Type::BVec3,
            (Type::F32, 2) => Type::Vec2,
            (Type::F32, 3) => Type::Vec3,
            (Type::F32, 4) => Type::Vec4,
            (Type::U32, 4) => Type::UVec4,
            other => panic!("three-rs: the node builder only makes vectors of {other:?}"),
        }
    }

    pub fn is_matrix(self) -> bool {
        matches!(self, Type::Mat2 | Type::Mat3 | Type::Mat4)
    }
}

/// Which of the generated shader's two uniform blocks a uniform lives in.
///
/// Three has a `UniformGroupNode` per uniform; the two the ladder uses are
/// `renderGroup` (per render call — camera, time, viewport) and `objectGroup`
/// (per render object — model matrices, material values).
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum UniformGroup {
    Render,
    Object,
}

impl UniformGroup {
    pub fn struct_name(self) -> &'static str {
        match self {
            UniformGroup::Render => "render",
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
pub enum UniformSource {
    CameraProjectionMatrix,
    CameraViewMatrix,
    CameraWorldMatrix,
    ModelWorldMatrix,
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
    MaterialOpacity,
    MaterialReflectivity,
    /// `materialRotation` — `SpriteMaterial.rotation`.
    MaterialRotation,
    /// `MeshPhongMaterial.shininess` / `.specular` / `.emissive` /
    /// `.emissiveIntensity`.
    MaterialShininess,
    MaterialSpecular,
    MaterialEmissive,
    MaterialEmissiveIntensity,
    /// `materialEnvRotation` — the env map's rotation matrix.
    EnvRotationMatrix,
    BackgroundRotation,
    BackgroundBlurriness,
    BackgroundIntensity,
    Time,
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
    /// `LightsNode`'s per-light members, by index into the renderer's light
    /// list for the pass. The dumps put all four in the **render** group:
    /// `light.color * light.intensity` (linear), the cutoff distance, the decay
    /// exponent, and the light's position through the camera view matrix.
    LightColorIntensity(usize),
    LightCutoffDistance(usize),
    LightDecay(usize),
    LightViewPosition(usize),
    /// `Morph.js`' `base = uniform( 1 )`, updated per object to
    /// `1 - Σ morphTargetInfluences` (or 1 when the targets are relative).
    MorphBase,
    /// `lightPosition( light )` / `lightTargetPosition( light )` — the world
    /// positions `lightTargetDirection` differences.
    LightWorldPosition(usize),
    LightTargetPosition(usize),
    /// `HemisphereLightNode`: `light.groundColor * light.intensity` (linear).
    LightGroundColor(usize),
    /// `SpotLightNode`'s `coneCosNode` / `penumbraCosNode`.
    LightConeCos(usize),
    LightPenumbraCos(usize),
    /// `ShadowNode`'s per-shadow references: `lightShadowMatrix( light )`, the
    /// shadow camera's near and far planes (`PointShadowNode`) and
    /// `reference( …, shadow )` for the five scalars.
    ShadowMatrix(usize),
    ShadowCameraNear(usize),
    ShadowCameraFar(usize),
    ShadowBias(usize),
    ShadowNormalBias(usize),
    ShadowRadius(usize),
    ShadowMapSize(usize),
    ShadowIntensity(usize),
    /// `materialLineWidth` — `MaterialNode.LINE_WIDTH`, i.e.
    /// `material.linewidth`. Only a fat-line material reads it; a hairline
    /// `Line` ignores it, as WebGL and WebGPU both do.
    MaterialLineWidth,
    /// `materialMetalness` / `materialRoughness` / `materialBumpScale`.
    MaterialMetalness,
    MaterialRoughness,
    MaterialBumpScale,
    /// `MeshPhysicalMaterial`'s `ior` / `specularIntensity` / `specularColor`,
    /// and `MeshStandardMaterial.normalScale`.
    MaterialIor,
    MaterialSpecularIntensity,
    MaterialSpecularColor,
    MaterialNormalScale,
    /// `toneMappingExposure` — `renderer.toneMappingExposure`.
    ToneMappingExposure,
    /// `reference( 'bindMatrix', 'mat4' )` / `reference( 'bindMatrixInverse',
    /// 'mat4' )` — `SkinnedMesh`'s two bind matrices, in the object group.
    BindMatrix,
    BindMatrixInverse,
    /// A plain `uniform( value )` the example supplies.
    Value(Vec<f64>),
    /// `uniform( value )` whose `.value` is written between draws — three.js'
    /// `UniformNode` is always this; [`UniformSource::Value`] is the special
    /// case of one that never moves. `SSAAPassNode.sampleWeight` is the port's
    /// first: one node, one program, eight different values across the eight
    /// accumulation draws of a frame.
    Settable(SettableValue),
}

/// The cell behind [`UniformSource::Settable`]. Two cells with equal contents
/// are still two uniforms, so this compares and hashes by identity — a value
/// that changes must not be part of any cache key.
#[derive(Clone, Debug)]
pub struct SettableValue(Rc<RefCell<Vec<f64>>>);

impl SettableValue {
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
    pub fn update_type(&self) -> UpdateType {
        match self {
            UniformSource::ModelWorldMatrix
            | UniformSource::ModelNormalMatrix
            | UniformSource::MaterialColor
            | UniformSource::MaterialOpacity
            | UniformSource::MaterialReflectivity
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
            | UniformSource::MaterialNormalScale
            | UniformSource::EnvRotationMatrix
            | UniformSource::MorphBase
            | UniformSource::BindMatrix
            | UniformSource::BindMatrixInverse
            | UniformSource::Value(_)
            | UniformSource::Settable(_) => UpdateType::Object,
            _ => UpdateType::Render,
        }
    }
}

/// `UniformNode`.
#[derive(Debug)]
pub struct UniformNode {
    pub source: UniformSource,
    pub ty: Type,
    pub group: UniformGroup,
    /// Three names camera uniforms explicitly and numbers the rest
    /// `nodeUniformN`.
    pub name: Option<&'static str>,
}

/// Where an array-typed uniform buffer's contents come from — `BufferNode`.
#[derive(Clone, PartialEq)]
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
    Range { min: [f64; 4], max: [f64; 4] },
    /// `Morph.js`' `uniformArray( mesh.morphTargetInfluences, 'float' )` — one
    /// `vec4` per morph target with the influence in `.x`.
    MorphInfluences,
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
    pub fn next() -> Self {
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
    pub source: BufferSource,
    pub element_ty: Type,
    pub count: usize,
}

/// The texture a `TextureNode` / `CubeTextureNode` samples.
#[derive(Clone, Debug)]
pub enum TextureSource {
    Texture2D(Texture),
    Depth(DepthTexture),
    /// A `DepthTexture` with `compareFunction` set, bound as
    /// `texture_depth_2d` + `sampler_comparison` and read with
    /// `textureSampleCompare` — `ShadowNode`'s shadow map.
    ShadowMap(DepthTexture),
    Cube(CubeTexture),
    /// `DataArrayTexture` — the morph-target data texture.
    DataArray(DataArrayTexture),
    /// `DataTexture` — `BatchedMesh`' matrices / colours / indirect tables.
    Data(DataTexture),
    /// A point light's shadow map — `cubeTexture( CubeDepthTexture )`.
    CubeDepth(CubeDepthTexture),
}

/// How a `TextureNode` reads its texture — `WGSLNodeBuilder.generateTexture*`.
#[derive(Clone, Debug)]
pub enum SampleMode {
    /// `textureSample( t, t_sampler, uv )`.
    Sample,
    /// `textureSampleLevel( t, t_sampler, uv, level )`.
    Level(NodeRef),
    /// `textureSampleGrad( t, t_sampler, uv, vec2( 0 ), vec2( 0 ) )` —
    /// `textureNode.grad( vec2(), vec2() )`, which is how `PMREMUtils`'
    /// `bilinearCubeUV` turns anisotropic filtering off on the cubeUV atlas.
    /// The two gradients are always the zero constants three passes, so they
    /// are baked rather than carried as nodes.
    Grad,
    /// The non-filterable path: `textureLoad` against `textureDimensions`,
    /// with no sampler binding at all. What Three emits for a depth texture.
    Load,
    /// `textureLoad( t, coord, layer, u32( 0u ) )` on a 2-D-array texture —
    /// `textureLoad( … ).depth( layer )`, with no clamping and no sampler.
    LoadLayer(NodeRef),
    /// `textureSampleCompare( t, t_sampler, uv, depth )` — the depth-compare
    /// read `ShadowFilterNode`'s `depthCompare` lowers to.
    Compare(NodeRef),
    /// `textureLoad( t, coord, u32( 0u ) )` — an unclamped texel fetch at an
    /// integer coordinate, what `Batch.js` reads its data textures with. When
    /// the node's type is a scalar the snippet is swizzled (`.x`), which is
    /// how Three's `textureLoad( … ).x` on the `r32uint` indirect table lands
    /// in one `u32` property.
    LoadTexel,
}

/// A WGSL builtin input.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Builtin {
    VertexIndex,
    InstanceIndex,
    /// `@builtin( position )` in the fragment stage.
    FragCoord,
    /// `@builtin( front_facing )` — `FrontFacingNode`.
    FrontFacing,
}

impl Builtin {
    pub fn name(self) -> &'static str {
        match self {
            Builtin::VertexIndex => "vertexIndex",
            Builtin::InstanceIndex => "instanceIndex",
            Builtin::FragCoord => "fragCoord",
            Builtin::FrontFacing => "isFront",
        }
    }

    pub fn ty(self) -> Type {
        match self {
            Builtin::VertexIndex | Builtin::InstanceIndex => Type::U32,
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
    pub value: NodeRef,
    pub ty: Type,
}

/// `VaryingNode` — a value computed in the vertex stage and interpolated.
#[derive(Debug)]
pub struct VaryingDef {
    /// `toVarying( 'name' )`; `None` numbers it `nodeVaryingN`.
    pub name: Option<&'static str>,
    pub value: NodeRef,
    pub ty: Type,
    /// u32 varyings need `@interpolate(flat, either)`.
    pub flat: bool,
}

/// `ShaderNode` / `Fn()`. With `layout` set, a real WGSL `fn` is emitted and
/// called; without it the body is inlined at every call site — which is what
/// Three does for `saturation`/`hue`, and why their expressions appear
/// expanded three times in the dumped shader.
pub struct FnDef {
    pub name: Option<&'static str>,
    pub params: Vec<(&'static str, Type)>,
    pub ret: Type,
    pub layout: bool,
    #[allow(clippy::type_complexity)]
    pub body: Box<dyn Fn(&[NodeRef]) -> NodeRef>,
}

impl std::fmt::Debug for FnDef {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FnDef").field("name", &self.name).finish()
    }
}

/// The node set. Closed on purpose: an exhaustive `match` in the builder is
/// what tells the next rung it has added something the generator cannot emit.
#[derive(Debug)]
pub enum Node {
    /// A literal. `values` holds one entry per component.
    Const {
        ty: Type,
        values: Vec<f64>,
    },
    /// `array< f32, N >( … )` — `QuadMesh`'s `vertexNode`.
    ConstArray {
        element_ty: Type,
        values: Vec<f64>,
    },
    Uniform(Rc<UniformNode>),
    /// `BufferNode` element access: `NodeBuffer_N.value[ index ]`.
    BufferElement {
        buffer: Rc<BufferNode>,
        index: NodeRef,
    },
    /// A geometry attribute.
    Attribute {
        name: &'static str,
        ty: Type,
    },
    /// `instancedBufferAttribute( buffer, type, stride, offset )`: a vertex
    /// attribute whose buffer steps once per instance. `offset` is in floats
    /// from the start of the instance; the stride is the buffer's `item_size`,
    /// which is all three.js' own call sites use.
    InstancedAttribute {
        buffer: Rc<InstanceBuffer>,
        offset: usize,
        ty: Type,
    },
    Builtin(Builtin),
    Var(Rc<VarDef>),
    /// `VarNode` with `readOnly` set — `node.toConst()`. A WGSL `let`, so it is
    /// declared where it is assigned and, unlike a `var<private>`, cannot be
    /// written again.
    Let(Rc<VarDef>),
    Varying(Rc<VaryingDef>),
    /// A `var<private>` with a fixed name that the setup code assigns
    /// explicitly — `PropertyNode` (`DiffuseColor`, `Output`, …).
    Property {
        name: &'static str,
        ty: Type,
    },
    /// A parameter of an emitted `fn` — a name that is already in scope.
    Param {
        name: &'static str,
        ty: Type,
    },
    /// A statement: `target = value`.
    Assign {
        target: NodeRef,
        value: NodeRef,
    },
    /// `OperatorNode`.
    Op {
        op: &'static str,
        a: NodeRef,
        b: NodeRef,
        ty: Type,
    },
    /// `MathNode` — a WGSL builtin call.
    Math {
        name: &'static str,
        args: Vec<NodeRef>,
        ty: Type,
    },
    /// `SplitNode`.
    Swizzle {
        node: NodeRef,
        components: &'static str,
        ty: Type,
    },
    /// `ConvertNode` / a single-argument constructor: `vec4<f32>( x )`.
    Cast {
        node: NodeRef,
        ty: Type,
    },
    /// `OperatorNode` with one operand: `( - x )`.
    Neg {
        node: NodeRef,
        ty: Type,
    },
    /// `JoinNode` — `vec4<f32>( a, b, c, d )`.
    Join {
        args: Vec<NodeRef>,
        ty: Type,
    },
    /// `ArrayElementNode`: `m[ 3u ]`, `array< f32, 3 >( … )[ vertexIndex ]`.
    Element {
        node: NodeRef,
        index: NodeRef,
        ty: Type,
    },
    Texture {
        texture: Rc<TextureSource>,
        uv: NodeRef,
        mode: SampleMode,
        ty: Type,
    },
    /// `TextureSizeNode` — `textureDimensions( t, level )`, a `vec2<u32>`.
    TextureSize {
        texture: Rc<TextureSource>,
        level: NodeRef,
    },
    /// `varyingProperty( type, name )` — a *named* varying assigned to by
    /// statement rather than built from a value. Unlike [`Node::Varying`] it
    /// is declared as soon as either stage mentions it, so a varying the
    /// fragment stage never reads still appears in `VaryingsStruct`, exactly
    /// as `vBatchIndirectId` does in Three's dump.
    VaryingProperty {
        name: &'static str,
        ty: Type,
        flat: bool,
    },
    Call {
        def: Rc<FnDef>,
        args: Vec<NodeRef>,
    },
    /// `FunctionCallNode` over a `wgslFn()` — a call into hand-written WGSL
    /// the node system copies through verbatim. See [`crate::nodes::code`].
    CodeCall {
        def: Rc<crate::nodes::code::CodeDef>,
        args: Vec<NodeRef>,
    },
    /// A sequence of statements followed by the value they produce — the shape
    /// an inlined `Fn()` body with `toVar()` statements has. Three has no node
    /// for it: its `ShaderNode` call simply flows its body's statements into the
    /// current stage and returns the last expression, which is what this does.
    Block {
        statements: Vec<NodeRef>,
        result: NodeRef,
    },
    /// `Loop( count, ( { i } ) => { … } )` — `for ( var i : i32 = 0; i < n; i ++ )`.
    Loop {
        count: NodeRef,
        /// The loop index, as it appears inside `body` (`Node::Param`).
        index: NodeRef,
        body: Vec<NodeRef>,
    },
    /// `If( cond, () => { … } )` as a bare statement (`setupDiscard`),
    /// optionally with the `.Else( … )` / `.ElseIf( … )` arm `StackNode` adds.
    ///
    /// `ElseIf` is not a shape of its own: `StackNode.ElseIf()` puts a whole
    /// nested `If` inside the else block, so `If( a ).ElseIf( b )` is
    /// `else_body: vec![ if_then( b, … ) ]` and generates as a nested
    /// `if`/`else`, which is exactly what three emits.
    If {
        cond: NodeRef,
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
        pre: Vec<NodeRef>,
        result: NodeRef,
        cond: NodeRef,
        body: Vec<NodeRef>,
    },
    /// `Discard()` — a bare `discard;`.
    Discard,
    /// `return value;` inside an emitted `fn` — what an `If( cond, () => {
    /// return x; } )` in a `Fn()` body compiles to (`neutralToneMapping`'s
    /// early out).
    Return {
        value: NodeRef,
    },
    /// `x.not()` — `( ! x )`.
    Not {
        node: NodeRef,
    },
    /// `cond.select( a, b )` — lowered to an `if`/`else` writing a result var,
    /// exactly as Three does.
    Select {
        cond: NodeRef,
        a: NodeRef,
        b: NodeRef,
        ty: Type,
    },
}

/// A handle on a node. Fluent TSL methods hang off this; see `tsl.rs`.
#[derive(Clone, Debug)]
pub struct NodeRef(pub Rc<Node>);

impl NodeRef {
    pub fn new(node: Node) -> Self {
        NodeRef(Rc::new(node))
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
            Node::Uniform(u) => u.ty,
            Node::BufferElement { buffer, .. } => buffer.element_ty,
            Node::Attribute { ty, .. } => *ty,
            Node::InstancedAttribute { ty, .. } => *ty,
            Node::Builtin(b) => b.ty(),
            Node::Var(v) => v.ty,
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
            Node::CodeCall { def, .. } => def.ret,
            Node::IfVar { result, .. } => result.ty(),
            Node::Select { ty, .. } => *ty,
            Node::Block { result, .. } => result.ty(),
            Node::Loop { .. } | Node::If { .. } | Node::Discard | Node::Return { .. } => Type::Void,
            Node::Not { .. } => Type::Bool,
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
pub struct Lazy<T: 'static> {
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
            BufferSource::Attribute(data) => (Rc::as_ptr(data) as *const u8 as usize).hash(state),
            BufferSource::InstanceMatrix
            | BufferSource::InstanceColor
            | BufferSource::MorphInfluences
            | BufferSource::BoneMatrices
            | BufferSource::Storage => {}
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
            BufferSource::Range { min, max } => f
                .debug_struct("Range")
                .field("min", min)
                .field("max", max)
                .finish(),
            BufferSource::MorphInfluences => f.write_str("MorphInfluences"),
            BufferSource::BoneMatrices => f.write_str("BoneMatrices"),
            BufferSource::Attribute(data) => f
                .debug_tuple("Attribute")
                .field(&format_args!("{} floats", data.len()))
                .finish(),
        }
    }
}
