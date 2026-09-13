//! Port of `three.js/src/nodes/core/` — the node graph itself.
//!
//! Three's nodes are mutable JS objects that grow builder state as they are
//! visited. Here a node is an immutable `Rc<Node>`; `Rc::as_ptr` gives it a
//! stable identity, and every piece of per-build state the builder would have
//! hung off the node (usage count, property name, varying slot, uniform slot)
//! lives in the builder keyed by that identity. See `docs/nodes.md` §1.

use std::cell::RefCell;
use std::rc::Rc;

use crate::math::{Color, Matrix4, Vector2, Vector3};
use crate::textures::{CubeTexture, DataArrayTexture, DepthTexture, Texture};

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
    BVec3,
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
            Type::Vec3 | Type::BVec3 => 3,
            Type::Vec4 => 4,
            Type::Mat3 => 9,
            Type::Mat4 => 16,
        }
    }

    /// `NodeBuilder.getComponentType()`.
    pub fn component_type(self) -> Type {
        match self {
            Type::UVec2 => Type::U32,
            Type::IVec2 => Type::I32,
            Type::BVec3 => Type::Bool,
            Type::Vec2 | Type::Vec3 | Type::Vec4 | Type::Mat3 | Type::Mat4 => Type::F32,
            other => other,
        }
    }

    /// The vector type with `self`'s component type and `n` components.
    pub fn vector_of(component: Type, n: usize) -> Type {
        match (component, n) {
            (_, 1) => component,
            (Type::U32, 2) => Type::UVec2,
            (Type::I32, 2) => Type::IVec2,
            (Type::Bool, 3) => Type::BVec3,
            (Type::F32, 2) => Type::Vec2,
            (Type::F32, 3) => Type::Vec3,
            (Type::F32, 4) => Type::Vec4,
            other => panic!("no vector type for {:?}", other),
        }
    }

    pub fn is_matrix(self) -> bool {
        matches!(self, Type::Mat3 | Type::Mat4)
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
    /// `materialColor` — `MeshBasicMaterial.color` in the working space.
    MaterialColor,
    MaterialOpacity,
    MaterialReflectivity,
    /// `MeshPhongMaterial.shininess` / `.specular` / `.emissive` /
    /// `.emissiveIntensity`.
    MaterialShininess,
    MaterialSpecular,
    MaterialEmissive,
    MaterialEmissiveIntensity,
    /// A `TextureNode`'s `texture.matrix` (offset/repeat/rotation/center).
    TextureMatrix,
    /// `materialEnvRotation` — the env map's rotation matrix.
    EnvRotationMatrix,
    BackgroundRotation,
    BackgroundBlurriness,
    BackgroundIntensity,
    Time,
    /// `viewportSize` — the render target's pixel dimensions.
    ViewportSize,
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
    /// A plain `uniform( value )` the example supplies.
    Value(Vec<f64>),
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
            | UniformSource::TextureMatrix
            | UniformSource::EnvRotationMatrix
            | UniformSource::MorphBase
            | UniformSource::Value(_) => UpdateType::Object,
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
#[derive(Clone, Debug, PartialEq)]
pub enum BufferSource {
    /// `InstancedMesh.instanceMatrix`.
    InstanceMatrix,
    /// `RangeNode` resolved per instance:
    /// `lerp( min[c], max[c], Math.random() )`.
    Range { min: Color, max: Color },
    /// `Morph.js`' `uniformArray( mesh.morphTargetInfluences, 'float' )` — one
    /// `vec4` per morph target with the influence in `.x`.
    MorphInfluences,
}

/// `BufferNode` — `buffer( array, type, count )`.
#[derive(Debug)]
pub struct BufferNode {
    pub source: BufferSource,
    pub element_ty: Type,
    pub count: usize,
}

/// The texture a `TextureNode` / `CubeTextureNode` samples.
#[derive(Clone, Debug)]
pub enum TextureSource {
    Texture2D(Texture),
    Depth(DepthTexture),
    Cube(CubeTexture),
    /// `DataArrayTexture` — the morph-target data texture.
    DataArray(DataArrayTexture),
}

/// How a `TextureNode` reads its texture — `WGSLNodeBuilder.generateTexture*`.
#[derive(Clone, Debug)]
pub enum SampleMode {
    /// `textureSample( t, t_sampler, uv )`.
    Sample,
    /// `textureSampleLevel( t, t_sampler, uv, level )`.
    Level(NodeRef),
    /// The non-filterable path: `textureLoad` against `textureDimensions`,
    /// with no sampler binding at all. What Three emits for a depth texture.
    Load,
    /// `textureLoad( t, coord, layer, u32( 0u ) )` on a 2-D-array texture —
    /// `textureLoad( … ).depth( layer )`, with no clamping and no sampler.
    LoadLayer(NodeRef),
}

/// A WGSL builtin input.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Builtin {
    VertexIndex,
    InstanceIndex,
    /// `@builtin( position )` in the fragment stage.
    FragCoord,
}

impl Builtin {
    pub fn name(self) -> &'static str {
        match self {
            Builtin::VertexIndex => "vertexIndex",
            Builtin::InstanceIndex => "instanceIndex",
            Builtin::FragCoord => "fragCoord",
        }
    }

    pub fn ty(self) -> Type {
        match self {
            Builtin::VertexIndex | Builtin::InstanceIndex => Type::U32,
            Builtin::FragCoord => Type::Vec4,
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
    Const { ty: Type, values: Vec<f64> },
    /// `array< f32, N >( … )` — `QuadMesh`'s `vertexNode`.
    ConstArray { element_ty: Type, values: Vec<f64> },
    Uniform(Rc<UniformNode>),
    /// `BufferNode` element access: `NodeBuffer_N.value[ index ]`.
    BufferElement { buffer: Rc<BufferNode>, index: NodeRef },
    /// A geometry attribute.
    Attribute { name: &'static str, ty: Type },
    Builtin(Builtin),
    Var(Rc<VarDef>),
    Varying(Rc<VaryingDef>),
    /// A `var<private>` with a fixed name that the setup code assigns
    /// explicitly — `PropertyNode` (`DiffuseColor`, `Output`, …).
    Property { name: &'static str, ty: Type },
    /// A parameter of an emitted `fn` — a name that is already in scope.
    Param { name: &'static str, ty: Type },
    /// A statement: `target = value`.
    Assign { target: NodeRef, value: NodeRef },
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
    Cast { node: NodeRef, ty: Type },
    /// `OperatorNode` with one operand: `( - x )`.
    Neg { node: NodeRef, ty: Type },
    /// `JoinNode` — `vec4<f32>( a, b, c, d )`.
    Join { args: Vec<NodeRef>, ty: Type },
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
    Call { def: Rc<FnDef>, args: Vec<NodeRef> },
    /// `LoopNode` over a count: `for ( var i : i32 = 0; i < count; i ++ ) { … }`.
    /// A statement; `index` is the `Param` the body reads.
    Loop {
        index: NodeRef,
        count: usize,
        body: Vec<NodeRef>,
    },
    /// `If( cond, () => { … } )` as a statement — `ConditionalNode` with no
    /// result value, which is what the morph guard is.
    If { cond: NodeRef, body: Vec<NodeRef> },
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
            Node::Builtin(b) => b.ty(),
            Node::Var(v) => v.ty,
            Node::Varying(v) => v.ty,
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
            Node::Select { ty, .. } => *ty,
            // Statements have no value.
            Node::Loop { .. } | Node::If { .. } => Type::Void,
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
        slot.as_ref().unwrap().clone()
    }
}
