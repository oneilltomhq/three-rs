//! Port of `three.js/src/nodes/tsl/` plus the accessor modules TSL re-exports
//! (`Position`, `Normal`, `Camera`, `Object3DNode`, `MaterialNode`, `UV`,
//! `TextureNode`, `Oscillators`, `ColorAdjustment`, …).
//!
//! JS property access becomes a method call (`.rgb()`, `.x()`), and the
//! accessor singletons become zero-argument functions, so ported example code
//! still reads like the original:
//!
//! ```ignore
//! hue(saturation(tex.rgb(), mouse.x().one_minus()), mouse.y())
//! ```

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use super::node::{
    BufferNode, BufferSource, Builtin, FnDef, InstanceBuffer, Lazy, Node, NodeRef, SampleMode,
    SettableValue, Type, UniformGroup, UniformNode, UniformSource, VarDef, VaryingDef,
};
use crate::materials::Side;
use crate::math::{Color, Matrix3};
use crate::textures::{
    CubeDepthTexture, CubeTexture, DataArrayTexture, DataTexture, DepthTexture, Texture,
};

pub use super::node::TextureSource;

// ---------------------------------------------------------------------------
// sub-builds and the build context (`docs/nodes.md` §7)
// ---------------------------------------------------------------------------

/// Cache key shared by every cache below that holds a node reading
/// `normalView`: `(sub_build_layer, normal_value_id, flat_shading,
/// material_side)`. The flat-shading flag belongs in it because
/// `normalViewGeometry` reads it and `negateOnBackSide()` is skipped when it
/// is set, so two materials that differ only there must not share a cached
/// node; the side is there for the same reason.
type NormalViewKey = (Option<&'static str>, Option<usize>, bool, Side);

thread_local! {
    /// `NodeBuilder.subBuildLayers`. One layer at a time is all the ladder
    /// needs; `NORMAL` is the only name so far.
    static SUB_BUILD: RefCell<Option<&'static str>> = const { RefCell::new(None) };
    /// `builder.context.setupNormal()` — `NodeMaterial.setupNormal()`'s result,
    /// i.e. the material's `normalNode`. `normal_view()` takes it as its value
    /// outside the `NORMAL` layer and `normalViewGeometry` inside it.
    static NORMAL_VALUE: RefCell<Option<NodeRef>> = const { RefCell::new(None) };
    /// `builder.isFlatShading()` — `material.flatShading && material.wireframe
    /// === false`. `normalViewGeometry` reads it, so like `NORMAL_VALUE` it is
    /// installed for the whole of one material's setup.
    static FLAT_SHADING: RefCell<bool> = const { RefCell::new(false) };
    /// `builder.material.side` — what `negateOnBackSide()` branches on, and so
    /// part of every cache key that reaches `normalView` or the tangent frame.
    static MATERIAL_SIDE: RefCell<Side> = const { RefCell::new(Side::Front) };
    /// `normalViewGeometry`'s node per flat-shading flag — the stand-in for
    /// three.js' per-build `nodeData`, which gives the two forms of the
    /// accessor's `Fn( … ).once()` separate cache entries.
    static NORMAL_VIEW_GEOMETRY: RefCell<HashMap<bool, NodeRef>> = RefCell::new(HashMap::new());
    /// `normalView`'s node per (layer, normal value) — the stand-in for
    /// three.js' per-build `nodeData` plus its `subBuildsCache`.
    static NORMAL_VIEW: RefCell<HashMap<NormalViewKey, NodeRef>> =
        RefCell::new(HashMap::new());
    /// `tangentView` / `bitangentView`, keyed the same way.
    static TANGENT_VIEW: RefCell<HashMap<NormalViewKey, (NodeRef, NodeRef)>> =
        RefCell::new(HashMap::new());
    /// `normalWorld`, keyed the same way: it reads `normalView`, so a plain
    /// singleton would bake in whichever material was built first and then
    /// re-assign `normalView` from the geometric normal in every later one.
    static NORMAL_WORLD: RefCell<HashMap<NormalViewKey, NodeRef>> =
        RefCell::new(HashMap::new());
    /// `builder.context.setupPositionView()` — `NodeMaterial.setup()` installs
    /// it before either stage is flowed, and `SpriteNodeMaterial` overrides it
    /// with the billboarded view position. `None` is the base class'
    /// `modelViewMatrix.mul( positionLocal ).xyz`.
    static POSITION_VIEW_VALUE: RefCell<Option<NodeRef>> = const { RefCell::new(None) };
    /// `positionView` / `modelViewProjection` per context value — three.js' own
    /// `Fn( … ).once()` cache is per build, so a second material in the same
    /// process must not inherit the first one's node.
    static POSITION_VIEW: RefCell<HashMap<Option<usize>, (NodeRef, NodeRef)>> =
        RefCell::new(HashMap::new());
}

/// `NodeBuilder.getSubBuildProperty( name )`: inside a layer a var's name is
/// prefixed with the layer's, which is where `NORMAL_normalView` comes from.
fn sub_build_name(name: &str) -> String {
    match SUB_BUILD.with(|s| *s.borrow()) {
        Some(layer) => format!("{layer}_{name}"),
        None => name.to_string(),
    }
}

/// `subBuild( node, name )` — build `f`'s nodes inside the named layer.
fn in_sub_build<R>(layer: &'static str, f: impl FnOnce() -> R) -> R {
    let previous = SUB_BUILD.with(|s| s.replace(Some(layer)));
    let out = f();
    SUB_BUILD.with(|s| *s.borrow_mut() = previous);
    out
}

/// Install the material's `normalNode` as `builder.context.setupNormal` for the
/// duration of `f` — `NodeMaterial.setup()` does exactly this before flowing
/// either stage. Returns what `f` returns.
pub fn with_material_normal<R>(
    normal: Option<NodeRef>,
    flat_shading: bool,
    side: Side,
    f: impl FnOnce() -> R,
) -> R {
    let previous = NORMAL_VALUE.with(|v| v.replace(normal));
    let previous_flat = FLAT_SHADING.with(|v| v.replace(flat_shading));
    let previous_side = MATERIAL_SIDE.with(|v| v.replace(side));
    let out = f();
    NORMAL_VALUE.with(|v| *v.borrow_mut() = previous);
    FLAT_SHADING.with(|v| *v.borrow_mut() = previous_flat);
    MATERIAL_SIDE.with(|v| *v.borrow_mut() = previous_side);
    out
}

/// `builder.material.side` alone, for the window in which
/// `NodeMaterial.setupNormal()` builds the material's normal node. three.js
/// calls `setupNormal()` lazily from inside the build, so the side is already
/// in scope there; the port builds the node up front and so has to open the
/// scope explicitly, or a `DoubleSide` material's TBN frame would be built
/// front-sided and then cached.
pub fn with_material_side<R>(side: Side, f: impl FnOnce() -> R) -> R {
    let previous = MATERIAL_SIDE.with(|v| v.replace(side));
    let out = f();
    MATERIAL_SIDE.with(|v| *v.borrow_mut() = previous);
    out
}

/// `negateOnBackSide( vector )` — `FrontFacingNode.js`. A back-sided material
/// inverts the vector outright; a double-sided one scales it by
/// `faceDirection`, so only the back-facing fragments flip. `normalView` and
/// the tangent frame both go through it, which is why a `DoubleSide` material's
/// dump multiplies three vectors by the same `( f32( isFront ) * 2 - 1 )`.
fn negate_on_back_side(vector: NodeRef) -> NodeRef {
    match MATERIAL_SIDE.with(|s| *s.borrow()) {
        Side::Front => vector,
        Side::Back => vector.mul(float(-1.0)),
        Side::Double => vector.mul(face_direction()),
    }
}

/// `builder.context.setupPositionView = () => this.setupPositionView( builder )`
/// (`NodeMaterial.js:472`), installed for the whole of the material's setup.
/// `SpriteNodeMaterial` is the only override the ladder needs, and it returns a
/// **`vec4`** rather than the base class' `vec3`, which is why `v_positionView`
/// is `vec4<f32>` in the galaxy dump.
pub fn with_material_position_view<R>(value: Option<NodeRef>, f: impl FnOnce() -> R) -> R {
    let previous = POSITION_VIEW_VALUE.with(|v| v.replace(value));
    let out = f();
    POSITION_VIEW_VALUE.with(|v| *v.borrow_mut() = previous);
    out
}

/// `positionView` and `modelViewProjection`, built together because both hang
/// off the same `builder.context` entry.
fn position_view_pair() -> (NodeRef, NodeRef) {
    let value = POSITION_VIEW_VALUE.with(|v| v.borrow().clone());
    let key = value.as_ref().map(|v| v.key());
    if let Some(pair) = POSITION_VIEW.with(|m| m.borrow().get(&key).cloned()) {
        return pair;
    }
    // `NodeMaterial.setupPositionView()`: `modelViewMatrix.mul( positionLocal ).xyz`.
    let view = value.unwrap_or_else(|| {
        model_view_matrix()
            .mul(vec4_join(vec![position_local(), float(1.0)]))
            .xyz()
    });
    let view = to_varying(Some("v_positionView"), view);
    // `NodeMaterial.setupModelViewProjection()`.
    let mvp = to_varying(
        Some("v_modelViewProjection"),
        camera_projection_matrix().mul(view.clone()),
    );
    let pair = (view, mvp);
    POSITION_VIEW.with(|m| m.borrow_mut().insert(key, pair.clone()));
    pair
}

// ---------------------------------------------------------------------------
// constructors
// ---------------------------------------------------------------------------

fn constant(ty: Type, values: Vec<f64>) -> NodeRef {
    NodeRef::new(Node::Const { ty, values })
}

/// `float( x )`.
pub fn float(v: impl Into<f64>) -> NodeRef {
    constant(Type::F32, vec![v.into()])
}

/// `int( x )`.
pub fn int(v: i64) -> NodeRef {
    constant(Type::I32, vec![v as f64])
}

/// `vec2( x, y )`.
pub fn vec2(x: impl Into<f64>, y: impl Into<f64>) -> NodeRef {
    constant(Type::Vec2, vec![x.into(), y.into()])
}

/// `vec3( x, y, z )`.
pub fn vec3(x: impl Into<f64>, y: impl Into<f64>, z: impl Into<f64>) -> NodeRef {
    constant(Type::Vec3, vec![x.into(), y.into(), z.into()])
}

/// `vec3( x )` — the splat form.
pub fn vec3s(x: impl Into<f64>) -> NodeRef {
    let x = x.into();
    constant(Type::Vec3, vec![x, x, x])
}

/// `vec4( x, y, z, w )`.
pub fn vec4(x: impl Into<f64>, y: impl Into<f64>, z: impl Into<f64>, w: impl Into<f64>) -> NodeRef {
    constant(Type::Vec4, vec![x.into(), y.into(), z.into(), w.into()])
}

/// `vec4( node, w )` — `JoinNode`.
pub fn join(ty: Type, args: Vec<NodeRef>) -> NodeRef {
    NodeRef::new(Node::Join { args, ty })
}

/// `vec2( a, b )` — `JoinNode` into a `vec2`.
pub fn vec2_join(args: Vec<NodeRef>) -> NodeRef {
    join(Type::Vec2, args)
}

/// `vec3( a, b )` where the components add up: `vec3<f32>( x, yz )`.
pub fn vec3_join(args: Vec<NodeRef>) -> NodeRef {
    join(Type::Vec3, args)
}

/// `vec4( vec3Node, w )`.
pub fn vec4_join(args: Vec<NodeRef>) -> NodeRef {
    join(Type::Vec4, args)
}

/// `array( […] ).element( i )`'s array half — `QuadMesh`'s `vertexNode`.
pub fn const_array(values: Vec<f64>) -> NodeRef {
    NodeRef::new(Node::ConstArray {
        element_ty: Type::F32,
        values,
    })
}

/// `array( [ … ] )` held in a `var<private> nodeVarN : array< f32, N >`.
///
/// Three emits the same literal as [`const_array`] does, but a `TempNode` the
/// flow reads more than once is promoted to a var, and `BloomNode`'s composite
/// reads its five bloom factors five times (`dump/m11`). `Node::ArrayVar` is
/// not a kind the builder promotes on its own, so — as with `radial_blur`'s
/// result — the caller asks for the var.
pub fn array_var(values: Vec<f64>) -> NodeRef {
    NodeRef::new(Node::ArrayVar {
        element_ty: Type::F32,
        values,
    })
}

/// `attribute( name, type )`.
pub fn attribute(name: &'static str, ty: Type) -> NodeRef {
    NodeRef::new(Node::Attribute { name, ty })
}

/// `uniform( value )` — a plain value uniform in the object group.
pub fn uniform_value(ty: Type, values: Vec<f64>) -> NodeRef {
    uniform(UniformSource::Value(values), ty, UniformGroup::Object, None)
}

/// `uniform( value )` whose value can be written between draws:
/// `SSAAPassNode`'s `this.sampleWeight.value = …`. The returned cell is the
/// handle; the node is the graph's view of it and never changes identity, so
/// the eight accumulation draws of a frame share one program.
pub fn uniform_settable(ty: Type, values: Vec<f64>) -> (NodeRef, SettableValue) {
    let cell = SettableValue::new(values);
    let node = uniform(
        UniformSource::Settable(cell.clone()),
        ty,
        UniformGroup::Object,
        None,
    );
    (node, cell)
}

pub fn uniform(
    source: UniformSource,
    ty: Type,
    group: UniformGroup,
    name: Option<&'static str>,
) -> NodeRef {
    NodeRef::new(Node::Uniform(Rc::new(UniformNode {
        source,
        ty,
        group,
        name,
    })))
}

/// `node.toVar( name )`. The name passes through `sub_build_name()`, so a var
/// declared while a sub-build layer is open takes the layer's prefix.
pub fn to_var(name: Option<&'static str>, value: NodeRef) -> NodeRef {
    let ty = value.ty();
    NodeRef::new(Node::Var(Rc::new(VarDef {
        name: name.map(sub_build_name),
        value,
        ty,
    })))
}

/// `node.toConst( name )` — a WGSL `let`. Same shape as [`to_var`], but the
/// value is written once where it is declared, so it takes its own
/// `nodeConstN` counter and no `var<private>` declaration.
pub fn to_const(name: Option<&'static str>, value: NodeRef) -> NodeRef {
    let ty = value.ty();
    NodeRef::new(Node::Let(Rc::new(VarDef {
        name: name.map(sub_build_name),
        value,
        ty,
    })))
}

/// `toVar( name )` for a var that keeps its name inside a sub-build layer.
/// Three prefixes only the nodes a layer is *tagged on* — the accessors that
/// declare the layer and their ancestors — so a var built inside one of those
/// accessors, like `tangentViewFrame`, stays unprefixed.
fn to_var_untagged(name: &'static str, value: NodeRef) -> NodeRef {
    let ty = value.ty();
    NodeRef::new(Node::Var(Rc::new(VarDef {
        name: Some(name.to_string()),
        value,
        ty,
    })))
}

/// `node.toVarying( name )`.
pub fn to_varying(name: Option<&'static str>, value: NodeRef) -> NodeRef {
    let ty = value.ty();
    NodeRef::new(Node::Varying(Rc::new(VaryingDef {
        name,
        value,
        ty,
        flat: matches!(ty, Type::U32 | Type::I32),
    })))
}

/// `property( type, name )`.
pub fn property(name: &'static str, ty: Type) -> NodeRef {
    NodeRef::new(Node::Property { name, ty })
}

// ---------------------------------------------------------------------------
// operators
// ---------------------------------------------------------------------------

const COMPARISONS: [&str; 6] = ["==", "!=", "<", "<=", ">", ">="];

/// `&&` / `||` — `OperatorNode` gives them a `bool` result without padding
/// either operand.
const LOGICAL: [&str; 2] = ["&&", "||"];

/// `OperatorNode.getNodeType()` plus the operand padding
/// `NodeBuilder.format()` performs: `mat4 * vec3` becomes
/// `mat4 * vec4( v, 1.0 )`.
fn binary(op: &'static str, a: NodeRef, b: NodeRef) -> NodeRef {
    let (ta, tb) = (a.ty(), b.ty());

    if LOGICAL.contains(&op) {
        return NodeRef::new(Node::Op {
            op,
            a,
            b,
            ty: Type::Bool,
        });
    }

    if COMPARISONS.contains(&op) {
        let n = ta.components().max(tb.components());
        let ty = if n == 1 {
            Type::Bool
        } else {
            Type::vector_of(Type::Bool, n)
        };
        return NodeRef::new(Node::Op { op, a, b, ty });
    }

    // matrix * vector: the result is a vector as wide as the matrix, and a
    // too-short vector is padded with 1.0 (`NodeBuilder.format`).
    if ta.is_matrix() && !tb.is_matrix() && tb.components() > 1 {
        let want = match ta {
            Type::Mat4 => 4,
            Type::Mat2 => 2,
            _ => 3,
        };
        let ty = Type::vector_of(Type::F32, want);
        let b = pad(b, want);
        return NodeRef::new(Node::Op { op, a, b, ty });
    }

    // vector * matrix: the row-vector form three.js uses for inverse-transpose
    // transforms (`vec4( n, 0.0 ) * cameraViewMatrix`).
    // A *scalar* times a matrix is not this: `OperatorNode.generate()` keeps
    // `typeA = 'float'` and scales the matrix (`skinWeight.x * boneMat`), so it
    // must not be padded into a row vector.
    if tb.is_matrix() && !ta.is_matrix() && ta.components() > 1 {
        let want = match tb {
            Type::Mat4 => 4,
            Type::Mat2 => 2,
            _ => 3,
        };
        let ty = Type::vector_of(Type::F32, want);
        let a = pad(a, want);
        return NodeRef::new(Node::Op { op, a, b, ty });
    }

    let ty = if ta.is_matrix() {
        ta
    } else if tb.is_matrix() || tb.components() > ta.components() {
        tb
    } else {
        ta
    };

    NodeRef::new(Node::Op { op, a, b, ty })
}

/// `NodeBuilder.format( snippet, 'vec3', 'vec4' )` — pad with 1.0.
fn pad(node: NodeRef, want: usize) -> NodeRef {
    let have = node.ty().components();
    if have >= want {
        return node;
    }
    let mut args = vec![node];
    for _ in have..want - 1 {
        args.push(float(0.0));
    }
    args.push(float(1.0));
    join(Type::vector_of(Type::F32, want), args)
}

fn math(name: &'static str, args: Vec<NodeRef>, ty: Type) -> NodeRef {
    NodeRef::new(Node::Math { name, args, ty })
}

/// `mix( a, b, t )`.
pub fn mix(a: impl Into<NodeRef>, b: impl Into<NodeRef>, t: impl Into<NodeRef>) -> NodeRef {
    let (a, b, t) = (a.into(), b.into(), t.into());
    let ty = if b.ty().components() > a.ty().components() {
        b.ty()
    } else {
        a.ty()
    };
    math("mix", vec![a, b, t], ty)
}

/// `dot( a, b )`.
pub fn dot(a: impl Into<NodeRef>, b: impl Into<NodeRef>) -> NodeRef {
    math("dot", vec![a.into(), b.into()], Type::F32)
}

/// `cross( a, b )`.
pub fn cross(a: impl Into<NodeRef>, b: impl Into<NodeRef>) -> NodeRef {
    let a = a.into();
    let ty = a.ty();
    math("cross", vec![a, b.into()], ty)
}

/// `reflect( i, n )`.
pub fn reflect(i: impl Into<NodeRef>, n: impl Into<NodeRef>) -> NodeRef {
    let i = i.into();
    let ty = i.ty();
    math("reflect", vec![i, n.into()], ty)
}

/// `floor( x )`.
pub fn floor(x: impl Into<NodeRef>) -> NodeRef {
    let x = x.into();
    let ty = x.ty();
    math("floor", vec![x], ty)
}

/// `fract( x )`.
pub fn fract(x: impl Into<NodeRef>) -> NodeRef {
    let x = x.into();
    let ty = x.ty();
    math("fract", vec![x], ty)
}

/// `sign( x )`.
pub fn sign(x: impl Into<NodeRef>) -> NodeRef {
    let x = x.into();
    let ty = x.ty();
    math("sign", vec![x], ty)
}

/// `exp2( x )`.
pub fn exp2(x: impl Into<NodeRef>) -> NodeRef {
    let x = x.into();
    let ty = x.ty();
    math("exp2", vec![x], ty)
}

/// `log2( x )` — `MathNode.LOG2`.
pub fn log2(x: impl Into<NodeRef>) -> NodeRef {
    let x = x.into();
    let ty = x.ty();
    math("log2", vec![x], ty)
}

/// `log( x )` — the natural logarithm, `MathNode.LOG`.
pub fn log(x: impl Into<NodeRef>) -> NodeRef {
    let x = x.into();
    let ty = x.ty();
    math("log", vec![x], ty)
}

/// `exp( x )` — `MathNode.EXP`.
pub fn exp(x: impl Into<NodeRef>) -> NodeRef {
    let x = x.into();
    let ty = x.ty();
    math("exp", vec![x], ty)
}

/// `min( a, b )` as a free function, to match [`max`].
pub fn min_of(a: impl Into<NodeRef>, b: impl Into<NodeRef>) -> NodeRef {
    let a = a.into();
    let ty = a.ty();
    math("min", vec![a, b.into()], ty)
}

/// `length( v )` — a scalar out of any vector.
pub fn length(v: impl Into<NodeRef>) -> NodeRef {
    math("length", vec![v.into()], Type::F32)
}

/// `TWO_PI` — `three.js/src/nodes/math/MathUtils.js`. A `float` const node, so
/// `TWO_PI.div( 3 )` is emitted as the division, never folded.
pub fn two_pi() -> NodeRef {
    float(std::f64::consts::TAU)
}

/// `rotate( position, rotation )` — `RotateNode`'s `vec2` branch
/// (`src/nodes/utils/RotateNode.js:106`):
///
/// ```ignore
/// mat2( cos, sin, sin.negate(), cos ).mul( position )
/// ```
///
/// `cos`/`sin` are one node each, used twice, so both become `nodeVarN` temps.
/// The `vec3`/`vec4` branch (three chained `mat4` rotations) is not ported.
pub fn rotate(position: impl Into<NodeRef>, rotation: impl Into<NodeRef>) -> NodeRef {
    let (position, rotation) = (position.into(), rotation.into());
    assert_eq!(
        position.ty(),
        Type::Vec2,
        "three-rs: rotate() only ports RotateNode's vec2 branch"
    );
    let cos_angle = rotation.cos();
    let sin_angle = rotation.sin();
    join(
        Type::Mat2,
        vec![
            cos_angle.clone(),
            sin_angle.clone(),
            sin_angle.negate(),
            cos_angle,
        ],
    )
    .mul(position)
}

/// `abs( x )`.
pub fn abs(x: impl Into<NodeRef>) -> NodeRef {
    let x = x.into();
    let ty = x.ty();
    math("abs", vec![x], ty)
}

/// `fwidth( x )`. `WGSLNodeBuilder`'s method table maps `dFdx`/`dFdy` but not
/// `fwidth`, so `MathNode.FWIDTH` reaches WGSL as the builtin of the same name.
pub fn fwidth(x: impl Into<NodeRef>) -> NodeRef {
    let x = x.into();
    let ty = x.ty();
    math("fwidth", vec![x], ty)
}

/// `x.mod( y )` on floats. WGSL has no `%` for floats the way three.js' node
/// system means it, so `MathNode` emits a helper; see `wgsl::MOD_FLOAT_SNIPPET`.
pub fn mod_float(x: impl Into<NodeRef>, y: impl Into<NodeRef>) -> NodeRef {
    math("tsl_mod_float", vec![x.into(), y.into()], Type::F32)
}

/// `smoothstep( low, high, x )`.
pub fn smoothstep(
    low: impl Into<NodeRef>,
    high: impl Into<NodeRef>,
    x: impl Into<NodeRef>,
) -> NodeRef {
    math(
        "smoothstep",
        vec![low.into(), high.into(), x.into()],
        Type::F32,
    )
}

/// `dFdx( x )` — WGSL `dpdx`.
pub fn dpdx(x: impl Into<NodeRef>) -> NodeRef {
    let x = x.into();
    let ty = x.ty();
    math("dpdx", vec![x], ty)
}

/// `dFdy( x )`. `WGSLNodeBuilder`'s method table maps `dFdy` to the string
/// `'- dpdy'`, so the emitted call carries the sign flip that takes WGSL's
/// framebuffer-down derivative back to GLSL's up convention.
pub fn dpdy(x: impl Into<NodeRef>) -> NodeRef {
    let x = x.into();
    let ty = x.ty();
    math("- dpdy", vec![x], ty)
}

/// Port of `three.js/src/nodes/procedural/Checker.js`. An `Fn()` with no
/// layout, so it inlines: `sign( mod( floor( uv.x * 2 ) + floor( uv.y * 2 ), 2 ) )`.
pub fn checker(coord: impl Into<NodeRef>) -> NodeRef {
    let uv = coord.into().mul(2.0);
    let cx = floor(uv.x());
    let cy = floor(uv.y());
    sign(mod_float(cx.add(cy), 2.0))
}

/// Port of `three.js/src/nodes/fog/Fog.js`' `rangeFogFactor( near, far )`:
/// `smoothstep( near, far, positionView.z.negate() )`.
pub fn range_fog_factor(near: impl Into<NodeRef>, far: impl Into<NodeRef>) -> NodeRef {
    smoothstep(near, far, position_view().z().negate())
}

/// Port of `three.js/src/nodes/fog/Fog.js`' `fog( color, factor )`. The node it
/// mixes into is the material's output, supplied at setup time, so the pair is
/// carried as a `FogNode` and unpacked by `NodeMaterial::setup_output()`:
/// `vec4( mix( output.rgb, fogColor, factor ), output.a )`.
pub fn fog(color: impl Into<NodeRef>, factor: impl Into<NodeRef>) -> FogNode {
    FogNode {
        color: color.into(),
        factor: factor.into(),
    }
}

/// `scene.fogNode = fog( color, factor )`.
#[derive(Clone)]
pub struct FogNode {
    pub color: NodeRef,
    pub factor: NodeRef,
}

/// By node identity — `fogNode.getCacheKey()`'s share of the render object's
/// dynamic cache key. A new `fog( … )` is a new program, as in three.js; a
/// uniform inside the same one is not.
impl std::hash::Hash for FogNode {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.color.key().hash(state);
        self.factor.key().hash(state);
    }
}

/// `LightsNode`'s four render-group uniforms for the point light at `index` of
/// the renderer's light list. Creating them here rather than inside
/// `phong.rs` keeps every `UniformSource` in one module.
pub fn light_color_intensity(index: usize) -> NodeRef {
    uniform(
        UniformSource::LightColorIntensity(index),
        Type::Vec3,
        UniformGroup::Render,
        None,
    )
}

pub fn light_cutoff_distance(index: usize) -> NodeRef {
    uniform(
        UniformSource::LightCutoffDistance(index),
        Type::F32,
        UniformGroup::Render,
        None,
    )
}

pub fn light_decay(index: usize) -> NodeRef {
    uniform(
        UniformSource::LightDecay(index),
        Type::F32,
        UniformGroup::Render,
        None,
    )
}

pub fn light_view_position(index: usize) -> NodeRef {
    uniform(
        UniformSource::LightViewPosition(index),
        Type::Vec3,
        UniformGroup::Render,
        None,
    )
}

/// `HemisphereLightNode`'s two extra render-group uniforms: the ground colour
/// (already multiplied by the light's intensity) and the light's **world**
/// position, which `lightPosition( light )` resolves to.
pub fn light_ground_color(index: usize) -> NodeRef {
    uniform(
        UniformSource::LightGroundColor(index),
        Type::Vec3,
        UniformGroup::Render,
        None,
    )
}

/// `lightPosition( light )` — `light.matrixWorld`'s translation.
pub fn light_world_position(index: usize) -> NodeRef {
    uniform(
        UniformSource::LightWorldPosition(index),
        Type::Vec3,
        UniformGroup::Render,
        None,
    )
}

/// `lightTargetPosition( light )` — `light.target.matrixWorld`'s translation.
pub fn light_target_position(index: usize) -> NodeRef {
    uniform(
        UniformSource::LightTargetPosition(index),
        Type::Vec3,
        UniformGroup::Render,
        None,
    )
}

/// `SpotLightNode.coneCosNode`.
pub fn light_cone_cos(index: usize) -> NodeRef {
    uniform(
        UniformSource::LightConeCos(index),
        Type::F32,
        UniformGroup::Render,
        None,
    )
}

/// `SpotLightNode.penumbraCosNode`.
pub fn light_penumbra_cos(index: usize) -> NodeRef {
    uniform(
        UniformSource::LightPenumbraCos(index),
        Type::F32,
        UniformGroup::Render,
        None,
    )
}

/// `lightShadowMatrix( light )`.
pub fn shadow_matrix(index: usize) -> NodeRef {
    uniform(
        UniformSource::ShadowMatrix(index),
        Type::Mat4,
        UniformGroup::Render,
        None,
    )
}

/// `PointShadowNode`'s shadow camera clipping planes —
/// `uniform( 'float' ).onRenderUpdate( () => shadow.camera.near / far )`.
pub fn shadow_camera_near(index: usize) -> NodeRef {
    uniform(
        UniformSource::ShadowCameraNear(index),
        Type::F32,
        UniformGroup::Render,
        None,
    )
}

pub fn shadow_camera_far(index: usize) -> NodeRef {
    uniform(
        UniformSource::ShadowCameraFar(index),
        Type::F32,
        UniformGroup::Render,
        None,
    )
}

/// `reference( 'bias', 'float', shadow )`.
pub fn shadow_bias(index: usize) -> NodeRef {
    uniform(
        UniformSource::ShadowBias(index),
        Type::F32,
        UniformGroup::Render,
        None,
    )
}

/// `reference( 'normalBias', 'float', shadow )`.
pub fn shadow_normal_bias(index: usize) -> NodeRef {
    uniform(
        UniformSource::ShadowNormalBias(index),
        Type::F32,
        UniformGroup::Render,
        None,
    )
}

/// `reference( 'radius', 'float', shadow )`.
pub fn shadow_radius(index: usize) -> NodeRef {
    uniform(
        UniformSource::ShadowRadius(index),
        Type::F32,
        UniformGroup::Render,
        None,
    )
}

/// `reference( 'mapSize', 'vec2', shadow )`.
pub fn shadow_map_size(index: usize) -> NodeRef {
    uniform(
        UniformSource::ShadowMapSize(index),
        Type::Vec2,
        UniformGroup::Render,
        None,
    )
}

/// `reference( 'intensity', 'float', shadow )`.
pub fn shadow_intensity(index: usize) -> NodeRef {
    uniform(
        UniformSource::ShadowIntensity(index),
        Type::F32,
        UniformGroup::Render,
        None,
    )
}

/// `materialMetalness` / `materialRoughness`.
pub fn material_metalness() -> NodeRef {
    uniform(
        UniformSource::MaterialMetalness,
        Type::F32,
        UniformGroup::Object,
        None,
    )
}

/// `materialIOR` / `materialSpecularIntensity` / `materialSpecularColor` —
/// `MeshPhysicalNodeMaterial.setupSpecular()`'s three uniforms.
pub fn material_ior() -> NodeRef {
    uniform(
        UniformSource::MaterialIor,
        Type::F32,
        UniformGroup::Object,
        None,
    )
}

pub fn material_specular_intensity() -> NodeRef {
    uniform(
        UniformSource::MaterialSpecularIntensity,
        Type::F32,
        UniformGroup::Object,
        None,
    )
}

pub fn material_specular_color() -> NodeRef {
    uniform(
        UniformSource::MaterialSpecularColor,
        Type::Vec3,
        UniformGroup::Object,
        None,
    )
}

/// `materialNormalScale` — a `vec2`.
pub fn material_normal_scale() -> NodeRef {
    uniform(
        UniformSource::MaterialNormalScale,
        Type::Vec2,
        UniformGroup::Object,
        None,
    )
}

pub fn material_roughness() -> NodeRef {
    uniform(
        UniformSource::MaterialRoughness,
        Type::F32,
        UniformGroup::Object,
        None,
    )
}

/// `materialBumpScale`.
pub fn material_bump_scale() -> NodeRef {
    uniform(
        UniformSource::MaterialBumpScale,
        Type::F32,
        UniformGroup::Object,
        None,
    )
}

/// `toneMappingExposure` — `RenderOutputNode`'s render-group `f32`.
pub fn tone_mapping_exposure() -> NodeRef {
    uniform(
        UniformSource::ToneMappingExposure,
        Type::F32,
        UniformGroup::Render,
        None,
    )
}

/// `inverseSqrt( x )`.
pub fn inverse_sqrt(x: impl Into<NodeRef>) -> NodeRef {
    math("inverseSqrt", vec![x.into()], Type::F32)
}

/// `AccessorsUtils.js`' `TBNViewMatrix` — `mat3( tangentView, bitangentView,
/// normalView ).toVar( 'TBNViewMatrix' )`. Three tags it with whichever
/// sub-build layers its descendants declare, which is why the centre teapot's
/// dump calls it `NORMAL_TBNViewMatrix`; here the layer is simply still open.
pub fn tbn_view_matrix() -> NodeRef {
    // Keyed like `tangentView` and `normalWorld`, and for the same reason: the
    // frame it joins depends on the material's side, so a singleton would bake
    // in whichever material was built first.
    thread_local! {
        static CELL: RefCell<HashMap<NormalViewKey, NodeRef>> = RefCell::new(HashMap::new());
    }
    let key = normal_key();
    if let Some(node) = CELL.with(|m| m.borrow().get(&key).cloned()) {
        return node;
    }
    let node = to_var(
        Some("TBNViewMatrix"),
        join(
            Type::Mat3,
            vec![tangent_view(), bitangent_view(), normal_view()],
        ),
    );
    CELL.with(|m| m.borrow_mut().insert(key, node.clone()));
    node
}

/// Port of `NormalMapNode` for `TangentSpaceNormalMap` with no scale:
/// `normalize( TBNViewMatrix * ( texel * 2 - 1 ).xyz )`.
///
/// The whole expression is built inside the `NORMAL` sub-build layer, the way
/// `NodeMaterial.setup()` wraps `setupNormal()` in `subBuild( …, 'NORMAL' )`:
/// that is what stops `normalView` inside the TBN frame from recursing into
/// this node, and what prefixes the four vars the layer names.
pub fn normal_map(node: impl Into<NodeRef>) -> NodeRef {
    let texel = node.into();
    in_sub_build("NORMAL", || {
        tbn_view_matrix()
            .mul(texel.mul(2.0).sub(1.0).xyz())
            .normalize()
    })
}

/// `normalMap( node, scale )` — the `scaleNode` branch of `NormalMapNode`:
/// `vec3( ( texel * 2 - 1 ).xy * scale, ( texel * 2 - 1 ).z )` through the TBN.
///
/// The unpacked texel is read twice, so it lands in a var of its own; the
/// no-scale [`normal_map`] reads it once and does not.
pub fn normal_map_scaled(node: impl Into<NodeRef>, scale: NodeRef) -> NodeRef {
    let texel = node.into();
    in_sub_build("NORMAL", || {
        let unpacked = to_var(None, texel.mul(2.0).sub(1.0));
        tbn_view_matrix()
            .mul(vec3_join(vec![
                unpacked.xy().mul(scale.clone()),
                unpacked.z(),
            ]))
            .normalize()
    })
}

/// `sqrt( x )`.
pub fn sqrt(x: impl Into<NodeRef>) -> NodeRef {
    let x = x.into();
    let ty = x.ty();
    math("sqrt", vec![x], ty)
}

/// `max( a, b )`.
pub fn max(a: impl Into<NodeRef>, b: impl Into<NodeRef>) -> NodeRef {
    let a = a.into();
    let ty = a.ty();
    math("max", vec![a, b.into()], ty)
}

// ---------------------------------------------------------------------------
// the fluent surface
// ---------------------------------------------------------------------------

/// The canonical swizzle for a whole value of this type — `SplitNode` collapses
/// `.rgb` on a `vec3` to nothing, which is why the dumped shaders read
/// `dot( nodeVar0.xyz, … )` and not `nodeVar0.xyz.xyz`.
fn whole(ty: Type) -> &'static str {
    match ty.components() {
        1 => "x",
        2 => "xy",
        3 => "xyz",
        _ => "xyzw",
    }
}

fn swizzle(node: NodeRef, components: &'static str) -> NodeRef {
    if components == whole(node.ty()) {
        return node;
    }
    let ty = Type::vector_of(node.ty().component_type(), components.len());
    NodeRef::new(Node::Swizzle {
        node,
        components,
        ty,
    })
}

impl NodeRef {
    // --- arithmetic ---
    pub fn add(&self, other: impl Into<NodeRef>) -> NodeRef {
        binary("+", self.clone(), other.into())
    }
    pub fn sub(&self, other: impl Into<NodeRef>) -> NodeRef {
        binary("-", self.clone(), other.into())
    }
    pub fn mul(&self, other: impl Into<NodeRef>) -> NodeRef {
        binary("*", self.clone(), other.into())
    }
    pub fn div(&self, other: impl Into<NodeRef>) -> NodeRef {
        binary("/", self.clone(), other.into())
    }
    /// `OperatorNode( '%' )` — integer remainder. (`MathNode`'s float `mod`
    /// is [`mod_float`], which lowers to a helper instead.)
    pub fn modulo(&self, other: impl Into<NodeRef>) -> NodeRef {
        binary("%", self.clone(), other.into())
    }
    pub fn equal(&self, other: impl Into<NodeRef>) -> NodeRef {
        binary("==", self.clone(), other.into())
    }
    pub fn not_equal(&self, other: impl Into<NodeRef>) -> NodeRef {
        binary("!=", self.clone(), other.into())
    }
    pub fn less_than_equal(&self, other: impl Into<NodeRef>) -> NodeRef {
        binary("<=", self.clone(), other.into())
    }
    pub fn greater_than(&self, other: impl Into<NodeRef>) -> NodeRef {
        binary(">", self.clone(), other.into())
    }
    pub fn greater_than_equal(&self, other: impl Into<NodeRef>) -> NodeRef {
        binary(">=", self.clone(), other.into())
    }
    pub fn less_than(&self, other: impl Into<NodeRef>) -> NodeRef {
        binary("<", self.clone(), other.into())
    }
    pub fn and(&self, other: impl Into<NodeRef>) -> NodeRef {
        binary("&&", self.clone(), other.into())
    }
    pub fn or(&self, other: impl Into<NodeRef>) -> NodeRef {
        binary("||", self.clone(), other.into())
    }
    /// `x.not()` — `( ! x )`.
    pub fn not(&self) -> NodeRef {
        NodeRef::new(Node::Not { node: self.clone() })
    }
    // --- bitwise (the MaterialX integer hashes) ---
    pub fn shift_left(&self, other: impl Into<NodeRef>) -> NodeRef {
        binary("<<", self.clone(), other.into())
    }
    pub fn shift_right(&self, other: impl Into<NodeRef>) -> NodeRef {
        binary(">>", self.clone(), other.into())
    }
    pub fn bit_and(&self, other: impl Into<NodeRef>) -> NodeRef {
        binary("&", self.clone(), other.into())
    }
    pub fn bit_or(&self, other: impl Into<NodeRef>) -> NodeRef {
        binary("|", self.clone(), other.into())
    }
    pub fn bit_xor(&self, other: impl Into<NodeRef>) -> NodeRef {
        binary("^", self.clone(), other.into())
    }

    /// `oneMinus()` — `1.0 - x`, emitted in that order.
    pub fn one_minus(&self) -> NodeRef {
        binary("-", float(1.0), self.clone())
    }

    pub fn negate(&self) -> NodeRef {
        NodeRef::new(Node::Neg {
            node: self.clone(),
            ty: self.ty(),
        })
    }

    // --- builtins ---
    pub fn normalize(&self) -> NodeRef {
        math("normalize", vec![self.clone()], self.ty())
    }
    pub fn cos(&self) -> NodeRef {
        math("cos", vec![self.clone()], self.ty())
    }
    pub fn sin(&self) -> NodeRef {
        math("sin", vec![self.clone()], self.ty())
    }
    /// `x.asin()` — `MathNode.ASIN`.
    pub fn asin(&self) -> NodeRef {
        math("asin", vec![self.clone()], self.ty())
    }
    /// `y.atan( x )` — `MathNode.ATAN` with two arguments, which WGSL spells
    /// `atan2( y, x )`. The receiver is the numerator, as in TSL.
    pub fn atan2(&self, x: impl Into<NodeRef>) -> NodeRef {
        math("atan2", vec![self.clone(), x.into()], self.ty())
    }
    pub fn floor(&self) -> NodeRef {
        math("floor", vec![self.clone()], self.ty())
    }
    pub fn fract(&self) -> NodeRef {
        math("fract", vec![self.clone()], self.ty())
    }
    pub fn sqrt(&self) -> NodeRef {
        math("sqrt", vec![self.clone()], self.ty())
    }
    pub fn abs(&self) -> NodeRef {
        math("abs", vec![self.clone()], self.ty())
    }
    /// `saturate()` — `clamp( x, 0, 1 )`, which three.js emits with vector
    /// bounds when `x` is a vector.
    pub fn saturate(&self) -> NodeRef {
        let ty = self.ty();
        let lo = if ty.components() > 1 {
            constant(ty, vec![0.0; ty.components()])
        } else {
            float(0.0)
        };
        let hi = if ty.components() > 1 {
            constant(ty, vec![1.0; ty.components()])
        } else {
            float(1.0)
        };
        math("clamp", vec![self.clone(), lo, hi], ty)
    }
    pub fn pow(&self, other: impl Into<NodeRef>) -> NodeRef {
        math("pow", vec![self.clone(), other.into()], self.ty())
    }
    /// `pow3( x )` — `MathNode`'s `POW3` is the multiplication chain
    /// `x * x * x`, not a `pow()` call.
    pub fn pow3(&self) -> NodeRef {
        self.mul(self.clone()).mul(self.clone())
    }

    /// `node.reciprocal()` — `1.0 / node`, which is how `OperatorNode` prints it.
    pub fn reciprocal(&self) -> NodeRef {
        float(1.0).div(self.clone())
    }

    pub fn exp2(&self) -> NodeRef {
        exp2(self.clone())
    }
    pub fn max(&self, other: impl Into<NodeRef>) -> NodeRef {
        math("max", vec![self.clone(), other.into()], self.ty())
    }
    pub fn min(&self, other: impl Into<NodeRef>) -> NodeRef {
        math("min", vec![self.clone(), other.into()], self.ty())
    }
    pub fn clamp(&self, lo: impl Into<NodeRef>, hi: impl Into<NodeRef>) -> NodeRef {
        math("clamp", vec![self.clone(), lo.into(), hi.into()], self.ty())
    }
    pub fn dot(&self, other: impl Into<NodeRef>) -> NodeRef {
        dot(self.clone(), other)
    }
    pub fn cross(&self, other: impl Into<NodeRef>) -> NodeRef {
        cross(self.clone(), other)
    }
    pub fn mix(&self, a: impl Into<NodeRef>, b: impl Into<NodeRef>) -> NodeRef {
        // `adjustment.mix( a, b )` in TSL means `mix( a, b, adjustment )`.
        mix(a, b, self.clone())
    }

    // --- swizzles ---
    /// `node[ components ]` — the general form of `.x` / `.xy` / `.rgb`, for
    /// when the component is chosen at build time.
    pub fn swizzle(&self, components: &'static str) -> NodeRef {
        swizzle(self.clone(), components)
    }

    pub fn x(&self) -> NodeRef {
        swizzle(self.clone(), "x")
    }
    pub fn y(&self) -> NodeRef {
        swizzle(self.clone(), "y")
    }
    pub fn z(&self) -> NodeRef {
        swizzle(self.clone(), "z")
    }
    pub fn w(&self) -> NodeRef {
        swizzle(self.clone(), "w")
    }
    pub fn xy(&self) -> NodeRef {
        swizzle(self.clone(), "xy")
    }
    pub fn yz(&self) -> NodeRef {
        swizzle(self.clone(), "yz")
    }
    pub fn zw(&self) -> NodeRef {
        swizzle(self.clone(), "zw")
    }
    pub fn zx(&self) -> NodeRef {
        swizzle(self.clone(), "zx")
    }
    pub fn xyz(&self) -> NodeRef {
        swizzle(self.clone(), "xyz")
    }
    pub fn xz(&self) -> NodeRef {
        swizzle(self.clone(), "xz")
    }
    pub fn zzz(&self) -> NodeRef {
        swizzle(self.clone(), "zzz")
    }
    pub fn rgb(&self) -> NodeRef {
        swizzle(self.clone(), "xyz")
    }
    pub fn a(&self) -> NodeRef {
        swizzle(self.clone(), "w")
    }

    /// `vec4( node.x, node.y, z, node.w )` — `SetNode` for `.setZ()`.
    pub fn set_z(&self, z: impl Into<NodeRef>) -> NodeRef {
        join(Type::Vec4, vec![self.x(), self.y(), z.into(), self.w()])
    }

    // --- conversion ---
    pub fn to(&self, ty: Type) -> NodeRef {
        if self.ty() == ty {
            return self.clone();
        }
        NodeRef::new(Node::Cast {
            node: self.clone(),
            ty,
        })
    }

    /// `m.element( i )` — a matrix column, or a vector component.
    pub fn element(&self, index: usize) -> NodeRef {
        let ty = match self.ty() {
            Type::Mat4 => Type::Vec4,
            Type::Mat3 => Type::Vec3,
            Type::Mat2 => Type::Vec2,
            other => other.component_type(),
        };
        NodeRef::new(Node::Element {
            node: self.clone(),
            index: constant(Type::U32, vec![index as f64]),
            ty,
        })
    }

    /// `array( … ).element( node )`.
    pub fn element_node(&self, index: NodeRef) -> NodeRef {
        let ty = match self.ty() {
            Type::Mat4 => Type::Vec4,
            Type::Mat3 => Type::Vec3,
            Type::Mat2 => Type::Vec2,
            // An `array< T, N >`'s element is a whole `T`; indexing anything
            // else is a *vector component*, so it is one scalar
            // (`ArrayElementNode.getNodeType()` over `getElementType()`).
            // Without the distinction a `vec2`'s `[ 0 ]` claims to be a
            // `vec2`, which `NodeBuilder.format()` then swizzles down —
            // `nodeVar1[ 0 ].x`, which three does not emit.
            other if matches!(&*self.0, Node::ConstArray { .. } | Node::ArrayVar { .. }) => other,
            other => other.component_type(),
        };
        NodeRef::new(Node::Element {
            node: self.clone(),
            index,
            ty,
        })
    }

    /// `cond.select( a, b )`.
    pub fn select(&self, a: impl Into<NodeRef>, b: impl Into<NodeRef>) -> NodeRef {
        let a = a.into();
        let ty = a.ty();
        NodeRef::new(Node::Select {
            cond: self.clone(),
            a,
            b: b.into(),
            ty,
        })
    }

    /// `.flipY()` — `FlipNode` over the y component: `vec2( v.x, 1.0 - v.y )`.
    ///
    /// `FlipNode.generate()` does not ask the usage counter for a temp, it
    /// takes one unconditionally (`builder.getVarFromNode( this )`) and assigns
    /// the *source* snippet into it, because it has to read two components of
    /// the source and will not evaluate it twice. So the var here is explicit
    /// rather than a `needs_var` promotion, and it is the flip's var, not the
    /// source's.
    ///
    /// **Divergence, cosmetic** (`docs/nodes.md` §8): three writes the flipped
    /// component as the bare string `1.0 - v.y`; the port builds it out of
    /// `sub`, so it comes out parenthesised as `( 1.0 - v.y )`. Same value,
    /// two characters more.
    pub fn flip_y(&self) -> NodeRef {
        let source = to_var(None, self.clone());
        match self.ty() {
            Type::Vec2 => vec2_join(vec![source.x(), float(1.0).sub(source.y())]),
            Type::Vec3 => vec3_join(vec![source.x(), float(1.0).sub(source.y()), source.z()]),
            Type::Vec4 => vec4_join(vec![
                source.x(),
                float(1.0).sub(source.y()),
                source.z(),
                source.w(),
            ]),
            ty => panic!("flipY() on a {ty:?}"),
        }
    }

    pub fn to_var(&self, name: &'static str) -> NodeRef {
        to_var(Some(name), self.clone())
    }

    pub fn to_varying(&self, name: &'static str) -> NodeRef {
        to_varying(Some(name), self.clone())
    }

    /// `target.addAssign( value )` — `target = ( target + value )`.
    pub fn add_assign(&self, value: impl Into<NodeRef>) -> NodeRef {
        self.assign(self.add(value))
    }

    /// `target.subAssign( value )` — `target = ( target - value )`.
    pub fn sub_assign(&self, value: impl Into<NodeRef>) -> NodeRef {
        self.assign(self.sub(value))
    }

    /// `target.mulAssign( value )` — `target = ( target * value )`.
    pub fn mul_assign(&self, value: impl Into<NodeRef>) -> NodeRef {
        self.assign(self.mul(value))
    }

    /// `target.divAssign( value )` — `target = ( target / value )`.
    pub fn div_assign(&self, value: impl Into<NodeRef>) -> NodeRef {
        self.assign(self.div(value))
    }

    /// `target.assign( value )` — a statement.
    pub fn assign(&self, value: impl Into<NodeRef>) -> NodeRef {
        NodeRef::new(Node::Assign {
            target: self.clone(),
            value: value.into(),
        })
    }
}

// ---------------------------------------------------------------------------
// accessors (`three.js/src/nodes/accessors/`)
// ---------------------------------------------------------------------------

macro_rules! accessor {
    ($(#[$m:meta])* $name:ident, $body:expr) => {
        $(#[$m])*
        pub fn $name() -> NodeRef {
            thread_local! {
                static CELL: Lazy<NodeRef> = const { Lazy::new() };
            }
            CELL.with(|c| c.get(|| $body))
        }
    };
}

accessor!(
    /// `positionGeometry` — the `position` attribute.
    position_geometry,
    attribute("position", Type::Vec3)
);
accessor!(
    /// `normalGeometry` — the `normal` attribute.
    normal_geometry,
    attribute("normal", Type::Vec3)
);
accessor!(
    /// `uv()` — the `uv` attribute, interpolated.
    uv,
    to_varying(None, attribute("uv", Type::Vec2))
);
accessor!(
    /// `vertexColor()` — `VertexColorNode`, the `color` attribute interpolated
    /// and widened to a `vec4`.
    ///
    /// three.js declares the node as `vec4` and lets `NodeBuilder.format()`
    /// widen a three-component `color` attribute with an alpha of 1, which is
    /// the only shape the port's geometries carry; a four-component `color`
    /// (three's `vertexAlphas`) is not modelled. The widening happens *before*
    /// the varying, so the interpolated value is a `vec4` and the fragment
    /// stage reads it whole — `webgpu_materials`' grid helper is the first
    /// example to put this on screen and three's m13/m14 pin the shape.
    ///
    /// **Divergence, deliberate** (`docs/nodes.md` §10): three's
    /// `VertexColorNode.generate()` falls back to a white constant when the
    /// geometry has no `color` attribute. The port has no geometry in hand at
    /// setup time — `SetupContext` is the whole of what `setup()` reads off the
    /// object — so `material.vertex_colors` alone decides, and the attribute
    /// has to be there.
    vertex_color,
    to_varying(
        None,
        vec4_join(vec![attribute("color", Type::Vec3), float(1.0)])
    )
);
accessor!(
    /// `vertexIndex`.
    vertex_index,
    NodeRef::new(Node::Builtin(Builtin::VertexIndex))
);
accessor!(
    /// `instanceIndex`.
    instance_index,
    NodeRef::new(Node::Builtin(Builtin::InstanceIndex))
);
accessor!(
    /// `frontFacing` — `@builtin( front_facing )`, fragment stage only.
    front_facing,
    NodeRef::new(Node::Builtin(Builtin::FrontFacing))
);
accessor!(
    /// `faceDirection` — `float( frontFacing ).mul( 2 ).sub( 1 )`: `1` on a
    /// front face, `-1` on a back face.
    face_direction,
    front_facing().to(Type::F32).mul(2.0).sub(1.0)
);
accessor!(
    /// `screenCoordinate`'s raw source: `@builtin( position )`.
    frag_coord,
    NodeRef::new(Node::Builtin(Builtin::FragCoord))
);

accessor!(
    /// `cameraProjectionMatrix`.
    camera_projection_matrix,
    uniform(
        UniformSource::CameraProjectionMatrix,
        Type::Mat4,
        UniformGroup::Render,
        Some("cameraProjectionMatrix")
    )
);
accessor!(
    /// `cameraViewMatrix`.
    camera_view_matrix,
    uniform(
        UniformSource::CameraViewMatrix,
        Type::Mat4,
        UniformGroup::Render,
        Some("cameraViewMatrix")
    )
);
accessor!(
    /// `cameraWorldMatrix`.
    camera_world_matrix,
    uniform(
        UniformSource::CameraWorldMatrix,
        Type::Mat4,
        UniformGroup::Render,
        Some("cameraWorldMatrix")
    )
);
accessor!(
    /// `modelWorldMatrix`.
    model_world_matrix,
    uniform(
        UniformSource::ModelWorldMatrix,
        Type::Mat4,
        UniformGroup::Object,
        None
    )
);
accessor!(
    /// `modelNormalMatrix`.
    model_normal_matrix,
    uniform(
        UniformSource::ModelNormalMatrix,
        Type::Mat3,
        UniformGroup::Object,
        None
    )
);
accessor!(
    /// `materialColor`.
    material_color,
    uniform(
        UniformSource::MaterialColor,
        Type::Vec3,
        UniformGroup::Object,
        None
    )
);
accessor!(
    /// `materialOpacity`.
    material_opacity,
    uniform(
        UniformSource::MaterialOpacity,
        Type::F32,
        UniformGroup::Object,
        None
    )
);
accessor!(
    /// `materialRotation` — `SpriteMaterial.rotation`, the angle
    /// `SpriteNodeMaterial.setupPositionView()` rotates the quad by when the
    /// material sets no `rotationNode`.
    material_rotation,
    uniform(
        UniformSource::MaterialRotation,
        Type::F32,
        UniformGroup::Object,
        None
    )
);
accessor!(
    /// `materialShininess`.
    material_shininess,
    uniform(
        UniformSource::MaterialShininess,
        Type::F32,
        UniformGroup::Object,
        None
    )
);
accessor!(
    /// `materialSpecular`.
    material_specular,
    uniform(
        UniformSource::MaterialSpecular,
        Type::Vec3,
        UniformGroup::Object,
        None
    )
);
accessor!(
    /// `materialEmissive`.
    material_emissive,
    uniform(
        UniformSource::MaterialEmissive,
        Type::Vec3,
        UniformGroup::Object,
        None
    )
);
accessor!(
    /// `materialEmissiveIntensity`.
    material_emissive_intensity,
    uniform(
        UniformSource::MaterialEmissiveIntensity,
        Type::F32,
        UniformGroup::Object,
        None
    )
);
accessor!(
    /// `materialEnvIntensity` — `MeshStandardMaterial.envMapIntensity`.
    material_env_intensity,
    uniform(
        UniformSource::MaterialEnvIntensity,
        Type::F32,
        UniformGroup::Object,
        None,
    )
);
accessor!(
    /// `materialReflectivity`.
    material_reflectivity,
    uniform(
        UniformSource::MaterialReflectivity,
        Type::F32,
        UniformGroup::Object,
        None
    )
);
accessor!(
    /// `materialEnvRotation` — the env map's rotation matrix.
    material_env_rotation,
    uniform(
        UniformSource::EnvRotationMatrix,
        Type::Mat4,
        UniformGroup::Object,
        None
    )
);
accessor!(
    /// `backgroundRotation`.
    background_rotation,
    uniform(
        UniformSource::BackgroundRotation,
        Type::Mat4,
        UniformGroup::Render,
        None
    )
);
accessor!(
    /// `backgroundBlurriness`, reused by `Background` as the texture LOD.
    background_blurriness,
    uniform(
        UniformSource::BackgroundBlurriness,
        Type::F32,
        UniformGroup::Render,
        None
    )
);
accessor!(
    /// `backgroundIntensity`.
    background_intensity,
    uniform(
        UniformSource::BackgroundIntensity,
        Type::F32,
        UniformGroup::Render,
        None
    )
);
accessor!(
    /// `time` — `TimerNode`, seconds since the renderer started.
    time,
    uniform(UniformSource::Time, Type::F32, UniformGroup::Render, None)
);
accessor!(
    /// `viewportSize` — `ScreenNode.SIZE`, the bound target's dimensions.
    viewport_size,
    uniform(
        UniformSource::ViewportSize,
        Type::Vec2,
        UniformGroup::Render,
        None
    )
);
accessor!(
    /// `viewport` — `ScreenNode.VIEWPORT`, `( x, y, width, height )` in
    /// physical pixels. `.zw` is the pair a screen-space line width divides by.
    viewport,
    uniform(UniformSource::Viewport, Type::Vec4, UniformGroup::Render, None)
);
accessor!(
    /// `screenDPR` — `renderer.getPixelRatio()`.
    screen_dpr,
    uniform(UniformSource::ScreenDpr, Type::F32, UniformGroup::Render, None)
);
accessor!(
    /// `cameraProjectionMatrixInverse`. Named, like the other camera matrices:
    /// `uniform( camera.projectionMatrixInverse ).setName(
    /// 'cameraProjectionMatrixInverse' )`.
    camera_projection_matrix_inverse,
    uniform(
        UniformSource::CameraProjectionMatrixInverse,
        Type::Mat4,
        UniformGroup::Render,
        Some("cameraProjectionMatrixInverse")
    )
);
accessor!(
    /// `modelWorldMatrixInverse` — `object.matrixWorld` inverted, per object.
    /// Unnamed, so it takes a `nodeUniformN` slot.
    model_world_matrix_inverse,
    uniform(
        UniformSource::ModelWorldMatrixInverse,
        Type::Mat4,
        UniformGroup::Object,
        None
    )
);
accessor!(
    /// `materialLineWidth` — `material.linewidth`.
    material_line_width,
    uniform(
        UniformSource::MaterialLineWidth,
        Type::F32,
        UniformGroup::Object,
        None
    )
);

accessor!(
    /// `screenUV` — `ScreenNode`'s `UV` scope: `screenCoordinate.div(
    /// screenSize )`, the fragment's position normalised into `[0,1]`.
    ///
    /// No Y flip, so y points *down*: `ScreenNode` flips only under WebGL
    /// (`builder.renderer.backend.isWebGLBackend`), and `webgpu_skinning`'s
    /// dumped background shader reads `( fragCoord.xy / render.nodeUniform0 ).y`
    /// straight.
    screen_uv,
    frag_coord().xy().div(viewport_size())
);

accessor!(
    /// `positionLocal` — `positionGeometry.toVarying( 'positionLocal' )`, so
    /// that instancing, morphing and skinning can reassign it and so that a
    /// fragment-stage read (the torus knot's `maskNode`) carries it across as a
    /// varying. A varying nothing in the fragment stage asks for stays a plain
    /// `var<private>` in the vertex shader, which is every other material.
    position_local,
    to_varying(Some("positionLocal"), position_geometry())
);
accessor!(
    /// `normalLocal`.
    normal_local,
    to_var(Some("normalLocal"), normal_geometry())
);
accessor!(
    /// `modelViewMatrix` — `cameraViewMatrix * modelWorldMatrix`.
    model_view_matrix,
    to_var(
        Some("modelViewMatrix"),
        camera_view_matrix().mul(model_world_matrix())
    )
);
/// `positionView` — `Position.js`' `Fn( builder =>
/// builder.context.setupPositionView() ).once( [ 'POSITION', 'VERTEX' ] )`.
pub fn position_view() -> NodeRef {
    position_view_pair().0
}
accessor!(
    /// `positionWorld` — `modelWorldMatrix * vec4( positionLocal, 1 )`, carried
    /// to the fragment stage as `v_positionWorld`.
    position_world,
    to_varying(
        Some("v_positionWorld"),
        model_world_matrix()
            .mul(vec4_join(vec![position_local(), float(1.0)]))
            .xyz()
    )
);
accessor!(
    /// `positionViewDirection`.
    position_view_direction,
    to_var(
        Some("positionViewDirection"),
        to_varying(Some("v_positionViewDirection"), position_view().negate()).normalize()
    )
);
/// `normalFlat` — `positionView.dFdx().cross( positionView.dFdy() ).normalize()
/// .toVar( 'normalFlat' )`. `dpdy()` carries WGSL's sign flip, so this prints as
/// `normalize( cross( dpdx( v_positionView ), - dpdy( v_positionView ) ) )`.
pub fn normal_flat() -> NodeRef {
    thread_local! { static CELL: Lazy<NodeRef> = const { Lazy::new() }; }
    CELL.with(|c| {
        c.get(|| {
            to_var(
                Some("normalFlat"),
                cross(dpdx(position_view()), dpdy(position_view())).normalize(),
            )
        })
    })
}

/// `normalViewGeometry` — `Fn( builder => builder.isFlatShading() ? normalFlat :
/// transformNormalToView( normalLocal ).toVarying( 'v_normalViewGeometry'
/// ).normalize() ).once()().toVar( 'normalViewGeometry' )`.
pub fn normal_view_geometry() -> NodeRef {
    let flat = FLAT_SHADING.with(|f| *f.borrow());
    if let Some(node) = NORMAL_VIEW_GEOMETRY.with(|m| m.borrow().get(&flat).cloned()) {
        return node;
    }
    let value = if flat {
        normal_flat()
    } else {
        to_varying(
            Some("v_normalViewGeometry"),
            camera_view_matrix()
                .mul(vec4_join(vec![
                    model_normal_matrix().mul(normal_local()),
                    float(0.0),
                ]))
                .xyz()
                .normalize(),
        )
        .normalize()
    };
    let node = to_var(Some("normalViewGeometry"), value);
    NORMAL_VIEW_GEOMETRY.with(|m| m.borrow_mut().insert(flat, node.clone()));
    node
}
/// `normalView` — `Normal.js`' `Fn( … ).once( [ 'NORMAL', 'VERTEX' ] )`.
///
/// Inside the `NORMAL` sub-build layer it is the geometric normal, under the
/// layer-prefixed name `NORMAL_normalView`; outside it, it is the material's
/// `normalNode` (reached through the build context) or, with no normal node,
/// the geometric normal again. Keyed by (layer, normal value) so that two
/// materials in the same process get their own node, which is what three.js'
/// per-build `nodeData` gives it for free.
/// The cache key every node that reads `normalView` shares: the open sub-build
/// layer plus the material's own normal node.
fn normal_key() -> NormalViewKey {
    let layer = SUB_BUILD.with(|s| *s.borrow());
    let value = if layer.is_some() {
        None
    } else {
        NORMAL_VALUE.with(|v| v.borrow().clone())
    };
    (
        layer,
        value.as_ref().map(|v| v.key()),
        FLAT_SHADING.with(|f| *f.borrow()),
        MATERIAL_SIDE.with(|s| *s.borrow()),
    )
}

/// The material's normal node for the current build, or `None` inside a
/// sub-build layer, which runs on the geometric normal.
fn normal_value() -> Option<NodeRef> {
    let layer = SUB_BUILD.with(|s| *s.borrow());
    if layer.is_some() {
        None
    } else {
        NORMAL_VALUE.with(|v| v.borrow().clone())
    }
}

pub fn normal_view() -> NodeRef {
    let key = normal_key();
    let flat = FLAT_SHADING.with(|f| *f.borrow());
    if let Some(node) = NORMAL_VIEW.with(|m| m.borrow().get(&key).cloned()) {
        return node;
    }
    // Inside the `NORMAL` layer the value is the geometric normal, and
    // `negateOnBackSide()` applies unless the material is flat shaded.
    let node = to_var(
        Some("normalView"),
        match normal_value() {
            Some(value) => value,
            None if flat => normal_view_geometry(),
            None => negate_on_back_side(normal_view_geometry()),
        },
    );
    NORMAL_VIEW.with(|m| m.borrow_mut().insert(key, node.clone()));
    node
}

/// `tangentView` / `bitangentView` for a geometry with no `tangent` attribute:
/// the derivative frame of `TangentUtils.js`, both `.once( [ 'NORMAL',
/// 'VERTEX' ] )` so they take the layer prefix. They are returned as a pair
/// because `tangentViewFrame` and `bitangentViewFrame` share the `scale` temp.
fn tangent_frame() -> (NodeRef, NodeRef) {
    let key = normal_key();
    if let Some(pair) = TANGENT_VIEW.with(|m| m.borrow().get(&key).cloned()) {
        return pair;
    }
    // `q1perp = dFdy( positionView ).cross( N )`, `q0perp = N.cross( dFdx(
    // positionView ) )`, with `dFdy` carrying the `- dpdy` sign flip.
    let n = normal_view();
    let q1perp = cross(dpdy(position_view()), n.clone());
    let q0perp = cross(n, dpdx(position_view()));
    let st0 = dpdx(uv());
    let st1 = dpdy(uv());

    let t = q1perp
        .clone()
        .mul(st0.clone().x())
        .add(q0perp.clone().mul(st1.clone().x()));
    let b = q1perp.mul(st0.y()).add(q0perp.mul(st1.y()));
    let det = max(t.clone().dot(t.clone()), b.clone().dot(b.clone()));
    // `det.equal( 0 ).select( 0, det.inverseSqrt() )`, which three.js lowers to
    // an if/else over a shared temp — `Node::Select`'s exact shape.
    let scale = det
        .clone()
        .equal(float(0.0))
        .select(float(0.0), inverse_sqrt(det));

    // `tangentView` / `bitangentView` go through `negateOnBackSide()` too,
    // unless the material is flat shaded.
    let flat = FLAT_SHADING.with(|f| *f.borrow());
    let frame = |name, value| {
        if flat {
            value
        } else {
            let _ = name;
            negate_on_back_side(value)
        }
    };
    let pair = (
        to_var(
            Some("tangentView"),
            frame(
                "tangentView",
                to_var_untagged("tangentViewFrame", t.mul(scale.clone())),
            ),
        ),
        to_var(
            Some("bitangentView"),
            frame(
                "bitangentView",
                to_var_untagged("bitangentViewFrame", b.mul(scale)),
            ),
        ),
    );
    TANGENT_VIEW.with(|m| m.borrow_mut().insert(key, pair.clone()));
    pair
}

/// `tangentView`.
pub fn tangent_view() -> NodeRef {
    tangent_frame().0
}

/// `bitangentView`.
pub fn bitangent_view() -> NodeRef {
    tangent_frame().1
}
/// `normalWorld` — `normalView` rotated out of view space.
pub fn normal_world() -> NodeRef {
    let key = normal_key();
    if let Some(node) = NORMAL_WORLD.with(|m| m.borrow().get(&key).cloned()) {
        return node;
    }
    let node = to_var(
        Some("normalWorld"),
        vec4_join(vec![normal_view(), float(0.0)])
            .mul(camera_view_matrix())
            .xyz()
            .normalize(),
    );
    NORMAL_WORLD.with(|m| m.borrow_mut().insert(key, node.clone()));
    node
}
accessor!(
    /// `normalWorldGeometry`.
    normal_world_geometry,
    to_var(
        Some("normalWorldGeometry"),
        to_varying(
            Some("v_normalWorldGeometry"),
            vec4_join(vec![normal_view_geometry(), float(0.0)])
                .mul(camera_view_matrix())
                .xyz()
                .normalize()
        )
        .normalize()
    )
);
/// `modelViewProjection` — `builder.context.setupModelViewProjection()`.
pub fn model_view_projection() -> NodeRef {
    position_view_pair().1
}
accessor!(
    /// `reflectVector` — `ReflectVectorNode`.
    reflect_vector,
    to_var(
        Some("reflectVector"),
        camera_world_matrix()
            .mul(vec4_join(vec![
                reflect(position_view_direction().negate(), normal_view()),
                float(0.0)
            ]))
            .xyz()
            .normalize()
    )
);

// Properties the material setup assigns to explicitly.
pub fn diffuse_color() -> NodeRef {
    thread_local! { static CELL: Lazy<NodeRef> = const { Lazy::new() }; }
    CELL.with(|c| c.get(|| property("DiffuseColor", Type::Vec4)))
}

macro_rules! prop {
    ($(#[$meta:meta])* $name:ident, $wgsl:literal, $ty:expr) => {
        $(#[$meta])*
        pub fn $name() -> NodeRef {
            thread_local! { static CELL: Lazy<NodeRef> = const { Lazy::new() }; }
            CELL.with(|c| c.get(|| property($wgsl, $ty)))
        }
    };
}

prop!(output_property, "Output", Type::Vec4);
prop!(total_diffuse, "totalDiffuse", Type::Vec3);
prop!(total_specular, "totalSpecular", Type::Vec3);
prop!(outgoing_light, "outgoingLight", Type::Vec3);
prop!(shininess, "Shininess", Type::F32);
prop!(specular_color, "SpecularColor", Type::Vec3);
prop!(emissive_color, "EmissiveColor", Type::Vec3);

/// `LightingContextNode.getContext()`'s accumulators. These are **vars**, not
/// properties: three.js builds them as `vec3().toVar( 'directDiffuse' )` /
/// `float( 1 ).toVar( 'ambientOcclusion' )`, so each one's initialiser is
/// emitted where the flow first reads it rather than up front — which is why the
/// dumps interleave `directSpecular = vec3<f32>( 0.0, 0.0, 0.0 )` with the
/// light's own statements.
macro_rules! lighting_var {
    ($name:ident, $wgsl:literal, $init:expr) => {
        pub fn $name() -> NodeRef {
            thread_local! { static CELL: Lazy<NodeRef> = const { Lazy::new() }; }
            CELL.with(|c| c.get(|| to_var_untagged($wgsl, $init)))
        }
    };
}

lighting_var!(direct_diffuse, "directDiffuse", vec3(0.0, 0.0, 0.0));
lighting_var!(direct_specular, "directSpecular", vec3(0.0, 0.0, 0.0));
lighting_var!(indirect_diffuse, "indirectDiffuse", vec3(0.0, 0.0, 0.0));
lighting_var!(indirect_specular, "indirectSpecular", vec3(0.0, 0.0, 0.0));
lighting_var!(irradiance, "irradiance", vec3(0.0, 0.0, 0.0));
lighting_var!(ambient_occlusion, "ambientOcclusion", float(1.0));
// `radiance` / `iblIrradiance` are `vec3().toVar()` on the lighting context
// too; nothing adds to them without an environment node.
lighting_var!(radiance, "radiance", vec3(0.0, 0.0, 0.0));
lighting_var!(ibl_irradiance, "iblIrradiance", vec3(0.0, 0.0, 0.0));

// `PhysicalLightingModel`'s properties.
prop!(metalness, "Metalness", Type::F32);
prop!(single_scattering, "singleScattering", Type::Vec3);
prop!(multi_scattering, "multiScattering", Type::Vec3);
prop!(roughness, "Roughness", Type::F32);
prop!(specular_color_blended, "SpecularColorBlended", Type::Vec3);
prop!(specular_f90, "SpecularF90", Type::F32);
// `MeshPhysicalNodeMaterial.setupSpecular()`'s `ior` property.
prop!(ior, "IOR", Type::F32);
prop!(diffuse_contribution, "DiffuseContribution", Type::Vec3);
prop!(
    single_scattering_dielectric,
    "singleScatteringDielectric",
    Type::Vec3
);
prop!(
    multi_scattering_dielectric,
    "multiScatteringDielectric",
    Type::Vec3
);
prop!(
    single_scattering_metallic,
    "singleScatteringMetallic",
    Type::Vec3
);
prop!(
    multi_scattering_metallic,
    "multiScatteringMetallic",
    Type::Vec3
);

// ---------------------------------------------------------------------------
// textures
// ---------------------------------------------------------------------------

fn texture_node(source: TextureSource, uv: NodeRef, mode: SampleMode, ty: Type) -> NodeRef {
    NodeRef::new(Node::Texture {
        texture: Rc::new(source),
        uv,
        mode,
        ty,
    })
}

/// The `texture.matrix * vec3( uv, 1.0 )` transform `TextureNode.setupUV()`
/// applies before sampling. `key` identifies the texture the matrix belongs to
/// (`(kind tag, texture id)`), so that two samples of the same map share one
/// uniform member the way three.js' per-texture `uniform( texture.matrix )`
/// does — `BumpMapNode` samples its map three times.
fn transformed_uv(uv: NodeRef, key: (u8, usize), matrix: Matrix3) -> NodeRef {
    thread_local! {
        static CACHE: RefCell<HashMap<(u8, usize), NodeRef>> = RefCell::new(HashMap::new());
    }
    let matrix_uniform = CACHE.with(|c| {
        c.borrow_mut()
            .entry(key)
            .or_insert_with(|| {
                uniform(
                    UniformSource::Value(
                        matrix
                            .to_padded_f32_array()
                            .iter()
                            .map(|&v| v as f64)
                            .collect(),
                    ),
                    Type::Mat3,
                    UniformGroup::Object,
                    None,
                )
            })
            .clone()
    });
    matrix_uniform.mul(vec3_join(vec![uv, float(1.0)])).xy()
}

/// `texture( map )`.
pub fn texture(map: &Texture) -> NodeRef {
    texture_node(
        TextureSource::Texture2D(map.clone()),
        transformed_uv(uv(), (0, map.id()), map.matrix()),
        SampleMode::Sample,
        Type::Vec4,
    )
}

/// `triplanarTexture( textureX, textureY, textureZ, scale )` —
/// `TriplanarTextures.js`. Three axis-aligned taps of the map, blended by the
/// normal: `bf = normalize( abs( normalLocal ) )`, renormalised so its
/// components sum to one, then `texture( x, position.yz * scale ) * bf.x + …`.
///
/// The `Fn()` has no layout, so three inlines it; this is a plain Rust
/// function for the same reason. The taps go through [`texture_uv`], not
/// [`texture`]: three calls `texture( value, tx )` with an explicit uv, and
/// `TextureNode.setupUV()` only applies the map's uv matrix to the *default*
/// uv, so no texture-matrix uniform appears.
///
/// **Divergence, API shape** (`docs/nodes.md` §8): three takes texture *nodes*
/// and reads `.value` back off them to rebuild a tap per axis. A `NodeRef` is
/// an opaque `Rc<Node>` here with no way back to the `Texture`, so the port
/// takes the maps themselves. `None` for y or z means "sample x", exactly as
/// three's `null` does.
pub fn triplanar_texture(
    map_x: &Texture,
    map_y: Option<&Texture>,
    map_z: Option<&Texture>,
    scale: NodeRef,
) -> NodeRef {
    let map_y = map_y.unwrap_or(map_x);
    let map_z = map_z.unwrap_or(map_x);

    let bf = normal_local().abs().normalize();
    let bf = bf.clone().div(bf.dot(vec3(1.0, 1.0, 1.0)));

    let tx = position_local().yz().mul(scale.clone());
    let ty = position_local().zx().mul(scale.clone());
    let tz = position_local().xy().mul(scale);

    let cx = texture_uv(map_x, tx).mul(bf.clone().x());
    let cy = texture_uv(map_y, ty).mul(bf.clone().y());
    let cz = texture_uv(map_z, tz).mul(bf.z());

    cx.add(cy).add(cz)
}

/// Port of `BumpMapNode` — `bumpMap( texture( bumpMap ).r, materialBumpScale )`.
///
/// `dHdxy_fwd` takes three taps of the height map (at `uv`, `uv + dFdx( uv )`
/// and `uv + dFdy( uv )`, each through the map's own uv matrix) and
/// `perturbNormalArb` rebuilds the normal from them in the screen-space frame
/// of `positionView`, flipping with `faceDirection` on a back face.
///
/// Built inside the `NORMAL` sub-build layer, the way `NodeMaterial.setup()`
/// wraps `setupNormal()`: that is what makes the normal it reads the geometric
/// one (`NORMAL_normalView`) instead of recursing into this node.
pub fn bump_map(map: &Texture, scale: NodeRef) -> NodeRef {
    in_sub_build("NORMAL", || {
        let tap = |coord: NodeRef| {
            texture_uv(map, transformed_uv(coord, (0, map.id()), map.matrix())).x()
        };
        let hll = tap(uv());
        let dhdxy = join(
            Type::Vec2,
            vec![
                tap(uv().add(dpdx(uv()))).sub(hll.clone()),
                tap(uv().add(dpdy(uv()))).sub(hll),
            ],
        )
        .mul(scale);

        let surf_norm = normal_view();
        let v_sigma_x = dpdx(position_view()).normalize();
        let v_sigma_y = dpdy(position_view()).normalize();
        let r1 = cross(v_sigma_y, surf_norm.clone());
        let r2 = cross(surf_norm.clone(), v_sigma_x.clone());
        let f_det = v_sigma_x.dot(r1.clone()).mul(face_direction());
        let v_grad = sign(f_det.clone()).mul(dhdxy.clone().x().mul(r1).add(dhdxy.y().mul(r2)));
        abs(f_det).mul(surf_norm).sub(v_grad).normalize()
    })
}

/// `texture( map ).sample( uv ).grad( vec2(), vec2() )` — a tap with the
/// gradients pinned to zero, which is how `PMREMUtils.bilinearCubeUV` turns
/// anisotropic filtering off on the cubeUV atlas.
pub fn texture_grad(map: &Texture, coord: NodeRef) -> NodeRef {
    texture_node(
        TextureSource::Texture2D(map.clone()),
        coord,
        SampleMode::Grad,
        Type::Vec4,
    )
}

/// `texture( map, uv, level )` — a 2-D tap at an explicit mip level.
///
/// `PMREMGenerator._getEquirectMaterial` samples the equirect source at level
/// `0` rather than letting the implicit derivative pick one: the six quads of
/// the lod plane are a wildly non-uniform parameterisation of the sphere, so
/// the derivative would pick a different level per face. With
/// `generateMipmaps = false` on the decoded HDR the two agree here, but the
/// WGSL has to say `textureSampleLevel( …, 0.0 )` to match three's.
pub fn texture_level(map: &Texture, coord: NodeRef, level: NodeRef) -> NodeRef {
    texture_node(
        TextureSource::Texture2D(map.clone()),
        coord,
        SampleMode::Level(level),
        Type::Vec4,
    )
}

/// `equirectUV( direction )` — `nodes/utils/EquirectUV.js`.
///
/// The longitude/latitude of a direction, in `[ 0, 1 ]²`. Three writes it as a
/// two-line `Fn`; a single-expression `Fn` inlines at its use site, which is
/// why three's own dump carries the whole thing on the `textureSampleLevel`
/// line rather than as a WGSL function.
pub fn equirect_uv(direction: NodeRef) -> NodeRef {
    let u = direction
        .z()
        .atan2(direction.x())
        .mul(float(1.0 / (std::f64::consts::PI * 2.0)))
        .add(float(0.5));
    let v = direction
        .y()
        .clamp(float(-1.0), float(1.0))
        .asin()
        .mul(float(1.0 / std::f64::consts::PI))
        .add(float(0.5));
    vec2_join(vec![u, v])
}

/// `texture( map ).sample( uv )` — the same tap as [`texture_uv`], but through
/// the map's `mat3x3` uv matrix.
///
/// `TextureNode.sample()` *clones* the texture node and leaves `updateMatrix`
/// on, so every sample of one map carries its **own** `uniform( texture.matrix
/// )` — `BloomNode`'s separable blur samples one texture three times and
/// three's dump has three `mat3x3` members for it (`dump/m05`). [`texture`]
/// shares one matrix per map instead, which is what three does when the same
/// *node* is read twice.
pub fn texture_sample(map: &Texture, coord: NodeRef) -> NodeRef {
    let matrix = uniform(
        UniformSource::Value(
            map.matrix()
                .to_padded_f32_array()
                .iter()
                .map(|&v| v as f64)
                .collect(),
        ),
        Type::Mat3,
        UniformGroup::Object,
        None,
    );
    texture_uv(map, matrix.mul(vec3_join(vec![coord, float(1.0)])).xy())
}

/// `texture( map, uv )` without the default UV.
pub fn texture_uv(map: &Texture, coord: NodeRef) -> NodeRef {
    texture_node(
        TextureSource::Texture2D(map.clone()),
        coord,
        SampleMode::Sample,
        Type::Vec4,
    )
}

/// `texture( depthTexture )` — a depth texture is not filterable, so three.js
/// emits a `textureLoad` with no sampler binding at all.
pub fn depth_texture(map: &DepthTexture) -> NodeRef {
    texture_node(
        TextureSource::Depth(map.clone()),
        transformed_uv(uv(), (1, map.id()), Matrix3::identity()),
        SampleMode::Load,
        Type::F32,
    )
}

/// `cubeTexture( map ).sample( dir )`. Cube UVs get their x negated, which is
/// `WGSLNodeBuilder.generateTextureSample`'s `vec3( - uv.x, uv.yz )`.
pub fn cube_texture(map: &CubeTexture, dir: NodeRef) -> NodeRef {
    let dir = vec3_join(vec![dir.x().negate(), dir.yz()]);
    texture_node(
        TextureSource::Cube(map.clone()),
        dir,
        SampleMode::Sample,
        Type::Vec4,
    )
}

/// `cubeTexture( shadowMap, dir ).compare( dp )` for a `CubeDepthTexture`.
///
/// `CubeTextureNode.setupUV()` takes the depth-texture branch: no environment
/// rotation, and the WebGPU Y flip — `vec3( uv.x, uv.y.negate(), uv.z )`.
pub fn cube_depth_texture_compare(map: &CubeDepthTexture, dir: NodeRef, dp: NodeRef) -> NodeRef {
    let dir = vec3_join(vec![dir.x(), dir.y().negate(), dir.z()]);
    texture_node(
        TextureSource::CubeDepth(map.clone()),
        dir,
        SampleMode::Compare(dp),
        Type::F32,
    )
}

/// `cubeTexture( map ).sample( dir ).level( lod )`.
pub fn cube_texture_level(map: &CubeTexture, dir: NodeRef, level: NodeRef) -> NodeRef {
    let dir = vec3_join(vec![dir.x().negate(), dir.yz()]);
    texture_node(
        TextureSource::Cube(map.clone()),
        dir,
        SampleMode::Level(level),
        Type::Vec4,
    )
}

// ---------------------------------------------------------------------------
// buffers
// ---------------------------------------------------------------------------

/// `instancedArray( count, type )` — `StorageBufferNode` over a
/// `StorageInstancedBufferAttribute` with **no CPU array behind it**
/// (`src/nodes/accessors/StorageBufferNode.js:270-290`): the storage is created
/// on the GPU, zero-filled once, and only ever written by a compute pass.
///
/// Held by the caller, because its identity is the buffer: two
/// `instancedArray( n, ty )` calls are two buffers, exactly as two
/// `StorageBufferNode`s are in three.js, and the renderer keys the GPU buffer
/// on it.
#[derive(Clone)]
pub struct StorageArray(Rc<BufferNode>);

/// `instancedArray( count, type )`.
pub fn instanced_array(count: usize, element_ty: Type) -> StorageArray {
    StorageArray(Rc::new(BufferNode {
        id: crate::nodes::node::BufferId::next(),
        source: BufferSource::Storage,
        element_ty,
        count,
    }))
}

impl StorageArray {
    /// `.element( index )` — `NodeBuffer_N.value[ index ]`.
    pub fn element(&self, index: impl Into<NodeRef>) -> NodeRef {
        NodeRef::new(Node::BufferElement {
            buffer: self.0.clone(),
            index: index.into(),
        })
    }

    /// The buffer's identity, which is how the renderer finds its GPU buffer.
    pub fn id(&self) -> crate::nodes::node::BufferId {
        self.0.id
    }

    /// The element count `instancedArray` was given.
    pub fn count(&self) -> usize {
        self.0.count
    }

    pub fn element_ty(&self) -> Type {
        self.0.element_ty
    }
}

/// `uniformArray( values )` — a constant array in a uniform block, one
/// `vec4<f32>` per element (`UniformArrayNode.getPaddedType()`).
///
/// Held by the caller like a [`StorageArray`], because its identity is the
/// buffer: all five `element()` calls of `BloomNode`'s composite have to reach
/// the *same* `NodeBuffer_N` binding.
#[derive(Clone)]
pub struct UniformArray(Rc<BufferNode>);

/// `uniformArray( [ Vector3, … ] )`.
///
/// Three takes the element type from `value[ 0 ]`; the port takes the three
/// components explicitly, since a `Vec<Vector3>` is the only shape the ladder
/// asks for. Each element is padded to a `vec4` and read back as its `.xyz`,
/// exactly as `UniformArrayElementNode.generate()` formats it.
pub fn uniform_array_vec3(values: &[[f64; 3]]) -> UniformArray {
    let mut padded = Vec::with_capacity(values.len() * 4);
    for v in values {
        padded.extend([v[0] as f32, v[1] as f32, v[2] as f32, 0.0]);
    }
    UniformArray(Rc::new(BufferNode {
        id: crate::nodes::node::BufferId::next(),
        source: BufferSource::UniformArray(Rc::new(padded)),
        element_ty: Type::Vec4,
        count: values.len(),
    }))
}

impl UniformArray {
    /// `.element( i )` — `NodeBuffer_N.value[ i ].xyz`.
    pub fn element(&self, index: usize) -> NodeRef {
        NodeRef::new(Node::BufferElement {
            buffer: self.0.clone(),
            index: constant(Type::U32, vec![index as f64]),
        })
        .xyz()
    }
}

fn buffer_element(source: BufferSource, element_ty: Type, count: usize, index: NodeRef) -> NodeRef {
    NodeRef::new(Node::BufferElement {
        buffer: Rc::new(BufferNode {
            id: crate::nodes::node::BufferId::next(),
            source,
            element_ty,
            count,
        }),
        index,
    })
}

/// `instancedBufferAttribute( buffer, type, stride, offset )`.
fn instanced_attribute(buffer: &Rc<InstanceBuffer>, offset: usize, ty: Type) -> NodeRef {
    NodeRef::new(Node::InstancedAttribute {
        buffer: buffer.clone(),
        offset,
        ty,
    })
}

/// `createInstanceMatrixNode( builder, instanceMatrix )`
/// (`src/nodes/accessors/Instance.js:27`).
///
/// Under the uniform buffer limit the matrices are a `buffer( array, 'mat4',
/// count ).element( instanceIndex )`; over it they are an
/// `InstancedInterleavedBuffer( array, 16, 1 )` read as four
/// `instancedBufferAttribute( interleaved, 'vec4', 16, offset )` views joined
/// back into a `mat4`. The second branch is what lets an `InstancedMesh` go
/// past `maxUniformBufferBindingSize / 64` ≈ 1024 instances.
pub fn instance_matrix(count: usize) -> NodeRef {
    let matrix_count = count.max(1);
    let uniform_buffer_size = matrix_count * 16 * 4;

    if uniform_buffer_size <= crate::nodes::builder::uniform_buffer_limit() {
        return buffer_element(
            BufferSource::InstanceMatrix,
            Type::Mat4,
            count,
            instance_index(),
        );
    }

    let interleaved = Rc::new(InstanceBuffer {
        id: crate::nodes::node::BufferId::next(),
        source: BufferSource::InstanceMatrix,
        count: matrix_count,
        item_size: 16,
    });
    join(
        Type::Mat4,
        (0..4)
            .map(|i| instanced_attribute(&interleaved, i * 4, Type::Vec4))
            .collect(),
    )
}

/// `instanceColor` — `varyingProperty( 'vec3', 'vInstanceColor' )` fed by
/// `instancedBufferAttribute( new InstancedBufferAttribute( colors.array, 3 ),
/// 'vec3', 3, 0 )` (`src/nodes/accessors/Instance.js`).
///
/// Unlike the matrices there is no uniform-buffer branch: three.js always
/// re-wraps the colours as an instanced vertex attribute.
pub fn instance_color(count: usize) -> NodeRef {
    let buffer = Rc::new(InstanceBuffer {
        id: crate::nodes::node::BufferId::next(),
        source: BufferSource::InstanceColor,
        count: count.max(1),
        item_size: 3,
    });
    to_varying(
        Some("vInstanceColor"),
        instanced_attribute(&buffer, 0, Type::Vec3),
    )
}

/// One end of a `range( min, max )`, i.e. the value `RangeNode` reads the
/// `Vector4` and the node type out of.
///
/// `RangeNode.setup()` (`src/nodes/geometry/RangeNode.js:122`) widens both ends
/// to a `Vector4` by three different rules, and they do not agree on `w`:
///
/// * a scalar splats into all four components,
/// * a `Color` becomes `( r, g, b, 1 )`,
/// * any other vector becomes `( x, y, z || 0, w || 0 )`.
///
/// So `range( vec3( -1 ), vec3( 1 ) )` has `w = 0` at *both* ends — the fourth
/// random draw per instance is still consumed, but it lands on a constant 0.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum RangeValue {
    /// `range( 0, 1 )` — `min.setScalar( value )`.
    Scalar(f64),
    /// `range( new Color( … ), … )` — `min.set( r, g, b, 1 )`.
    Color(Color),
    /// `range( vec3( -1 ), vec3( 1 ) )` — `min.set( x, y, z, 0 )`.
    Vec3([f64; 3]),
}

impl From<f64> for RangeValue {
    fn from(v: f64) -> Self {
        RangeValue::Scalar(v)
    }
}

impl From<Color> for RangeValue {
    fn from(v: Color) -> Self {
        RangeValue::Color(v)
    }
}

impl From<crate::math::Vector3> for RangeValue {
    fn from(v: crate::math::Vector3) -> Self {
        RangeValue::Vec3([v.x, v.y, v.z])
    }
}

impl RangeValue {
    /// The `Vector4` `RangeNode.setup()` lerps between.
    fn vector4(self) -> [f64; 4] {
        match self {
            RangeValue::Scalar(v) => [v, v, v, v],
            RangeValue::Color(c) => [c.r, c.g, c.b, 1.0],
            RangeValue::Vec3([x, y, z]) => [x, y, z, 0.0],
        }
    }

    /// `RangeNode.getNodeType()` — the value's own type, which is what the
    /// `vec4` the buffer holds is `convert()`ed down to.
    fn ty(self) -> Type {
        match self {
            RangeValue::Scalar(_) => Type::F32,
            RangeValue::Color(_) | RangeValue::Vec3(_) => Type::Vec3,
        }
    }
}

/// `range( min, max )` — `RangeNode` on an `InstancedMesh` resolves to one
/// `vec4` per instance, `lerp( min[c], max[c], Math.random() )` per component.
/// The returned node is the raw `vec4`; [`instanced_range`] narrows it.
pub fn range(min: [f64; 4], max: [f64; 4], count: usize, index: NodeRef) -> NodeRef {
    buffer_element(BufferSource::Range { min, max }, Type::Vec4, count, index)
}

/// `RangeNode.setup()` on an object with `count > 1`
/// (`src/nodes/geometry/RangeNode.js:122`): the same uniform-or-attribute
/// branch as [`instance_matrix`], on `count * 4 * 4` bytes, then
/// `.convert( nodeType )` — which for a `vec4` source is a swizzle, so a
/// `range( 0, 1 )` reads `…​.x` and a `range( vec3( -1 ), vec3( 1 ) )` reads
/// `….xyz`.
///
/// Each call builds its own buffer node, so two `range( 0, 1 )` calls are two
/// buffers with two different random fills — Three's behaviour, and the thing a
/// value-keyed cache would silently collapse.
pub fn instanced_range(
    min: impl Into<RangeValue>,
    max: impl Into<RangeValue>,
    count: usize,
) -> NodeRef {
    let (min, max) = (min.into(), max.into());
    let ty = min.ty();
    let uniform_buffer_size = count * 4 * 4;

    let vec4 = if uniform_buffer_size <= crate::nodes::builder::uniform_buffer_limit() {
        // `buffer( array, 'vec4', count ).element( instanceIndex )`. The index
        // goes through a varying because the port reads the buffer in the
        // fragment stage; see `docs/nodes.md` §9.
        range(
            min.vector4(),
            max.vector4(),
            count,
            to_varying(None, instance_index()),
        )
    } else {
        let buffer = Rc::new(InstanceBuffer {
            id: crate::nodes::node::BufferId::next(),
            source: BufferSource::Range {
                min: min.vector4(),
                max: max.vector4(),
            },
            count,
            item_size: 4,
        });
        instanced_attribute(&buffer, 0, Type::Vec4)
    };

    // `ConvertNode` — `NodeBuilder.format( snippet, 'vec4', nodeType )`.
    match ty {
        Type::F32 => vec4.x(),
        Type::Vec2 => vec4.xy(),
        Type::Vec3 => vec4.xyz(),
        _ => vec4,
    }
}

/// `Loop( count, ( { i } ) => { … } )` — the statement and the index node the
/// body reads, which is a plain `i` already in scope.
pub fn loop_index() -> NodeRef {
    NodeRef::new(Node::Param {
        name: "i",
        ty: Type::I32,
    })
}

pub fn loop_statement(count: usize, index: NodeRef, body: Vec<NodeRef>) -> NodeRef {
    let count = int(count as i64);
    NodeRef::new(Node::Loop { index, count, body })
}

/// `If( cond, () => { … } )` with no `Else` — a statement.
pub fn if_statement(cond: NodeRef, body: Vec<NodeRef>) -> NodeRef {
    NodeRef::new(Node::If {
        cond,
        body,
        else_body: Vec::new(),
    })
}

/// `ivec2( x, y )`.
pub fn ivec2(x: impl Into<NodeRef>, y: impl Into<NodeRef>) -> NodeRef {
    join(Type::IVec2, vec![x.into(), y.into()])
}

/// `textureSize( textureLoad( map ), level )` — `textureDimensions`, a
/// `vec2<u32>`.
pub fn texture_size(source: TextureSource, level: NodeRef) -> NodeRef {
    NodeRef::new(Node::TextureSize {
        texture: Rc::new(source),
        level,
    })
}

/// `textureLoad( dataTexture, ivec2( x, y ) )` — the unclamped, sampler-less
/// texel fetch `Batch.js` reads its three data textures with. `ty` is the
/// node's type: `vec4` for the `rgba32float` matrices and colours, and the
/// scalar `uint` for the `r32uint` indirect table, whose `.x` Three folds into
/// the same cached property.
pub fn texture_load_texel(map: &DataTexture, coord: NodeRef, ty: Type) -> NodeRef {
    texture_node(
        TextureSource::Data(map.clone()),
        coord,
        SampleMode::LoadTexel,
        ty,
    )
}

/// `varyingProperty( type, name )` — a named varying written by assignment.
pub fn varying_property(name: &'static str, ty: Type, flat: bool) -> NodeRef {
    NodeRef::new(Node::VaryingProperty { name, ty, flat })
}

/// `textureLoad( dataArrayTexture, ivec2( x, y ) ).depth( layer )` — the
/// sampler-less array read `Morph.js`' `getMorph()` performs.
pub fn texture_load_array(map: &DataArrayTexture, coord: NodeRef, layer: NodeRef) -> NodeRef {
    texture_node(
        TextureSource::DataArray(map.clone()),
        coord,
        SampleMode::LoadLayer(layer),
        Type::Vec4,
    )
}

/// `uniformArray( mesh.morphTargetInfluences, 'float' ).element( i )` — one
/// `vec4` per morph target, the influence in `.x`.
pub fn morph_influences(count: usize, index: NodeRef) -> NodeRef {
    buffer_element(BufferSource::MorphInfluences, Type::Vec4, count, index)
}

/// `Morph.js`' `base = uniform( 1 )`, in the object group.
pub fn morph_base() -> NodeRef {
    uniform(
        UniformSource::MorphBase,
        Type::F32,
        UniformGroup::Object,
        None,
    )
}

/// `geometry.setAttribute( name, new InstancedBufferAttribute( array, items ) )`
/// read back as `attribute( name, type )`.
///
/// three.js resolves the name through the geometry; here the data travels with
/// the node, because the node graph is built from the material and the renderer
/// has no attribute-name table. `item_size` is the stride in floats (the items
/// of one instance) and `offset` the component offset inside it, so several
/// views of one interleaved buffer share a single `stepMode: 'instance'` vertex
/// buffer — the same grouping `instance_matrix` uses above.
pub fn instanced_data_attribute(
    data: &Rc<Vec<f32>>,
    item_size: usize,
    offset: usize,
    ty: Type,
) -> NodeRef {
    instanced_buffer_attribute(&instanced_data_buffer(data, item_size), offset, ty)
}

/// `new InstancedInterleavedBuffer( array, stride, 1 )` — the buffer itself,
/// so that several `InterleavedBufferAttribute` views of it can be taken with
/// [`instanced_buffer_attribute`] and land in **one** `stepMode: 'instance'`
/// vertex buffer.
///
/// [`NodeProgram::vertex_buffers`](crate::nodes::NodeProgram::vertex_buffers)
/// groups on the buffer's `Rc` identity, so two `instanced_data_attribute`
/// calls over the same array are two buffers; a fat line's `instanceStart` and
/// `instanceEnd` have to be two views of one.
pub fn instanced_data_buffer(data: &Rc<Vec<f32>>, item_size: usize) -> Rc<InstanceBuffer> {
    let count = data.len().checked_div(item_size).unwrap_or(0);
    Rc::new(InstanceBuffer {
        id: crate::nodes::node::BufferId::next(),
        source: BufferSource::Attribute(data.clone()),
        count,
        item_size,
    })
}

/// `instancedBufferAttribute( buffer, type, stride, offset )` — one view of a
/// buffer from [`instanced_data_buffer`]. `offset` is in components, not bytes.
pub fn instanced_buffer_attribute(buffer: &Rc<InstanceBuffer>, offset: usize, ty: Type) -> NodeRef {
    assert!(
        offset + ty.components() <= buffer.item_size,
        "three-rs: instanced attribute reads past the instance stride"
    );
    instanced_attribute(buffer, offset, ty)
}

// ---------------------------------------------------------------------------
// Fn()
// ---------------------------------------------------------------------------

/// `Fn( body )` with no layout: the body is inlined at every call site.
pub fn inline_fn(
    params: usize,
    ret: Type,
    body: impl Fn(&[NodeRef]) -> NodeRef + 'static,
) -> Rc<FnDef> {
    Rc::new(FnDef {
        name: None,
        params: vec![("", Type::Void); params],
        ret,
        layout: false,
        body: Box::new(body),
    })
}

/// `wgslFn( source )` / `wgslFn( source, includes )` — see
/// [`crate::nodes::code`]. Re-exported here because every other TSL entry
/// point lives in this module.
pub use crate::nodes::code::wgsl_fn;

/// Calling a `wgslFn` — `FunctionCallNode` with three's named-parameter form:
/// `getWGSLTextureSample( { tex, tex_sampler, uv } )`. The names are the ones
/// the WGSL declaration used, and the order they are given in does not matter.
///
/// # Panics
///
/// If a declared parameter has no argument, or an argument names a parameter
/// the declaration does not have. Three leaves the first as `undefined` and
/// generates a shader that will not compile; the port refuses at setup.
pub fn call_wgsl(def: &Rc<crate::nodes::code::CodeDef>, args: Vec<(&str, NodeRef)>) -> NodeRef {
    let ordered = def
        .params
        .iter()
        .map(|(name, _)| {
            args.iter()
                .find(|(given, _)| given == name)
                .map(|(_, node)| node.clone())
                .unwrap_or_else(|| {
                    panic!(
                        "three-rs: wgslFn `{}` has no argument for `{name}`",
                        def.name
                    )
                })
        })
        .collect();
    for (given, _) in &args {
        assert!(
            def.params.iter().any(|(name, _)| name == given),
            "three-rs: wgslFn `{}` has no parameter `{given}`",
            def.name
        );
    }
    NodeRef::new(Node::CodeCall {
        def: def.clone(),
        args: ordered,
    })
}

/// `Fn( body, layout )`: a real WGSL `fn` is emitted and called.
pub fn shader_fn(
    name: Option<&'static str>,
    params: Vec<(&'static str, Type)>,
    ret: Type,
    body: impl Fn(&[NodeRef]) -> NodeRef + 'static,
) -> Rc<FnDef> {
    Rc::new(FnDef {
        name,
        params,
        ret,
        layout: true,
        body: Box::new(body),
    })
}

/// `If( cond, () => { … } )` over a result var that was initialised first.
///
/// `pre` are the statements three.js emits ahead of the result var, `body` the
/// statements inside the block (the last of which assigns the result). The
/// node's value is the result var, so a second reference reuses it rather than
/// re-emitting the block.
pub fn if_node(pre: Vec<NodeRef>, result: NodeRef, cond: NodeRef, body: Vec<NodeRef>) -> NodeRef {
    NodeRef::new(Node::IfVar {
        pre,
        result,
        cond,
        body,
    })
}

pub fn call(def: &Rc<FnDef>, args: Vec<NodeRef>) -> NodeRef {
    NodeRef::new(Node::Call {
        def: def.clone(),
        args,
    })
}

// ---------------------------------------------------------------------------
// display / math node functions the ladder uses
// ---------------------------------------------------------------------------

/// `luminance( color )`, with the working (linear-sRGB) coefficients.
pub fn luminance(color: NodeRef) -> NodeRef {
    dot(color, vec3(0.2126, 0.7152, 0.0722))
}

/// `saturation( color, adjustment )` — ported verbatim from
/// `ColorAdjustment.js`: `adjustment.mix( luminance( color.rgb ), color.rgb ).max( 0.0 )`.
pub fn saturation(color: NodeRef, adjustment: NodeRef) -> NodeRef {
    thread_local! { static CELL: Lazy<Rc<FnDef>> = const { Lazy::new() }; }
    let def = CELL.with(|c| {
        c.get(|| {
            inline_fn(2, Type::Vec3, |args| {
                let (color, adjustment) = (args[0].clone(), args[1].clone());
                adjustment
                    .mix(luminance(color.rgb()), color.rgb())
                    .max(float(0.0))
            })
        })
    });
    call(&def, vec![color, adjustment])
}

/// `hue( color, adjustment )` — ported verbatim from `ColorAdjustment.js`.
pub fn hue(color: NodeRef, adjustment: NodeRef) -> NodeRef {
    thread_local! { static CELL: Lazy<Rc<FnDef>> = const { Lazy::new() }; }
    let def = CELL.with(|c| {
        c.get(|| {
            inline_fn(2, Type::Vec3, |args| {
                let (color, adjustment) = (args[0].clone(), args[1].clone());
                let k = vec3(0.57735, 0.57735, 0.57735);
                let cos_angle = adjustment.cos();
                color
                    .rgb()
                    .mul(cos_angle.clone())
                    .add(
                        k.cross(color.rgb())
                            .mul(adjustment.sin())
                            .add(k.mul(dot(k.clone(), color.rgb()).mul(cos_angle.one_minus()))),
                    )
                    .max(float(0.0))
            })
        })
    });
    call(&def, vec![color, adjustment])
}

/// `oscSine( t )` — `Oscillators.js`: `t.add( 0.75 ).mul( PI2 ).sin().mul( 0.5 ).add( 0.5 )`.
pub fn osc_sine(t: NodeRef) -> NodeRef {
    t.add(float(0.75))
        .mul(float(std::f64::consts::TAU))
        .sin()
        .mul(float(0.5))
        .add(float(0.5))
}

/// `sRGBTransferOETF` — `ColorSpaceFunctions.js`, emitted as a real `fn`.
pub fn srgb_transfer_oetf(color: NodeRef) -> NodeRef {
    thread_local! { static CELL: Lazy<Rc<FnDef>> = const { Lazy::new() }; }
    let def = CELL.with(|c| {
        c.get(|| {
            shader_fn(
                Some("sRGBTransferOETF"),
                vec![("color", Type::Vec3)],
                Type::Vec3,
                |args| {
                    let color = args[0].clone();
                    mix(
                        color
                            .pow(float(0.41666))
                            .mul(float(1.055))
                            .sub(float(0.055)),
                        color.mul(float(12.92)),
                        color.less_than_equal(float(0.0031308)).to(Type::Vec3),
                    )
                },
            )
        })
    });
    call(&def, vec![color])
}

/// `linearToneMapping` — `ToneMappingFunctions.js`. The cheapest of the three:
/// exposure and a clamp, and nothing else.
pub fn linear_tone_mapping(color: NodeRef, exposure: NodeRef) -> NodeRef {
    thread_local! { static CELL: Lazy<Rc<FnDef>> = const { Lazy::new() }; }
    let def = CELL.with(|c| {
        c.get(|| {
            shader_fn(
                Some("linearToneMapping"),
                vec![("color", Type::Vec3), ("exposure", Type::F32)],
                Type::Vec3,
                |args| {
                    args[0]
                        .clone()
                        .mul(args[1].clone())
                        .clamp(float(0.0), float(1.0))
                },
            )
        })
    });
    call(&def, vec![color, exposure])
}

/// `sRGBTransferEOTF` — `ColorSpaceFunctions.js`, emitted as a real `fn`. The
/// inverse of [`srgb_transfer_oetf`]: sRGB in, working (linear-sRGB) out.
pub fn srgb_transfer_eotf(color: NodeRef) -> NodeRef {
    thread_local! { static CELL: Lazy<Rc<FnDef>> = const { Lazy::new() }; }
    let def = CELL.with(|c| {
        c.get(|| {
            shader_fn(
                Some("sRGBTransferEOTF"),
                vec![("color", Type::Vec3)],
                Type::Vec3,
                |args| {
                    let color = args[0].clone();
                    mix(
                        color
                            .mul(float(0.9478672986))
                            .add(float(0.0521327014))
                            .pow(float(2.4)),
                        color.mul(float(0.0773993808)),
                        color.less_than_equal(float(0.04045)).to(Type::Vec3),
                    )
                },
            )
        })
    });
    call(&def, vec![color])
}

/// `colorSpaceToWorking( node, SRGBColorSpace )` — `ColorSpaceNode.setup()`'s
/// `SRGBTransfer` branch: `vec4( sRGBTransferEOTF( node.rgb ), node.a )`.
pub fn srgb_to_working(color: NodeRef) -> NodeRef {
    vec4_join(vec![srgb_transfer_eotf(color.rgb()), color.a()])
}

/// `packNormalToRGB( node )` — `Packing.js`: `node * 0.5 + 0.5`, the
/// convention that puts a unit direction in a colour.
pub fn pack_normal_to_rgb(node: NodeRef) -> NodeRef {
    node.mul(float(0.5)).add(float(0.5))
}

/// `reinhardToneMapping` — `ToneMappingFunctions.js`, emitted as a real `fn`.
pub fn reinhard_tone_mapping(color: NodeRef, exposure: NodeRef) -> NodeRef {
    thread_local! { static CELL: Lazy<Rc<FnDef>> = const { Lazy::new() }; }
    let def = CELL.with(|c| {
        c.get(|| {
            shader_fn(
                Some("reinhardToneMapping"),
                vec![("color", Type::Vec3), ("exposure", Type::F32)],
                Type::Vec3,
                |args| {
                    // `color = color.mul( exposure )`, a var because it is
                    // read twice.
                    let color = args[0].clone().mul(args[1].clone());
                    color
                        .clone()
                        .div(color.add(float(1.0)))
                        .clamp(float(0.0), float(1.0))
                },
            )
        })
    });
    call(&def, vec![color, exposure])
}

/// `premultiplyAlpha` — `PremultiplyAlphaFunctions.js`.
pub fn premultiply_alpha(color: NodeRef) -> NodeRef {
    thread_local! { static CELL: Lazy<Rc<FnDef>> = const { Lazy::new() }; }
    let def = CELL.with(|c| {
        c.get(|| {
            shader_fn(None, vec![("color", Type::Vec4)], Type::Vec4, |args| {
                let color = args[0].clone();
                vec4_join(vec![color.rgb().mul(color.a()), color.a()])
            })
        })
    });
    call(&def, vec![color])
}

/// `unpremultiplyAlpha` — `PremultiplyAlphaFunctions.js`.
pub fn unpremultiply_alpha(color: NodeRef) -> NodeRef {
    thread_local! { static CELL: Lazy<Rc<FnDef>> = const { Lazy::new() }; }
    let def = CELL.with(|c| {
        c.get(|| {
            shader_fn(None, vec![("color", Type::Vec4)], Type::Vec4, |args| {
                let color = args[0].clone();
                color.a().equal(float(0.0)).select(
                    vec4(0.0, 0.0, 0.0, 0.0),
                    vec4_join(vec![color.rgb().div(color.a().to(Type::Vec3)), color.a()]),
                )
            })
        })
    });
    call(&def, vec![color])
}

// ---------------------------------------------------------------------------
// statements (`Fn()` bodies, `Loop()`, `If()`, `Discard()`)
// ---------------------------------------------------------------------------

/// A sequence of statements followed by the value they produce — what an
/// inlined `Fn()` whose body uses `toVar()` / `assign()` amounts to.
pub fn block(statements: Vec<NodeRef>, result: NodeRef) -> NodeRef {
    NodeRef::new(Node::Block { statements, result })
}

/// `Loop( count, ( { i } ) => { … } )`. `body` is called with the loop index.
pub fn loop_n(
    name: &'static str,
    count: NodeRef,
    body: impl FnOnce(&NodeRef) -> Vec<NodeRef>,
) -> NodeRef {
    let index = NodeRef::new(Node::Param {
        name,
        ty: Type::I32,
    });
    let body = body(&index);
    NodeRef::new(Node::Loop { count, index, body })
}

/// `If( cond, () => { … } )`.
pub fn if_then(cond: NodeRef, body: Vec<NodeRef>) -> NodeRef {
    NodeRef::new(Node::If {
        cond,
        body,
        else_body: Vec::new(),
    })
}

/// `If( cond, () => { … } ).Else( () => { … } )`.
pub fn if_else(cond: NodeRef, body: Vec<NodeRef>, else_body: Vec<NodeRef>) -> NodeRef {
    NodeRef::new(Node::If {
        cond,
        body,
        else_body,
    })
}

/// `If( cond, … ).ElseIf( other, … )`.
///
/// `StackNode.ElseIf()` is `this.Else( () => If( other, … ) )`, so the second
/// condition is a whole nested `If` inside the else block and the generated
/// WGSL is a nested `if`/`else` rather than an `else if`. Reproducing that
/// shape is the point — three's own dumps have the nesting.
pub fn if_else_if(
    cond: NodeRef,
    body: Vec<NodeRef>,
    other: NodeRef,
    other_body: Vec<NodeRef>,
) -> NodeRef {
    if_else(cond, body, vec![if_then(other, other_body)])
}

/// `return value;` inside a `Fn()` body — the statement an early-out `If(
/// cond, () => { return x; } )` compiles to.
pub fn return_statement(value: NodeRef) -> NodeRef {
    NodeRef::new(Node::Return { value })
}

/// `Discard()`.
pub fn discard() -> NodeRef {
    NodeRef::new(Node::Discard)
}

/// `Discard( condition )` — `NodeMaterial.setupDiscard()`'s
/// `If( cond.not(), () => Discard() )`.
pub fn discard_if(cond: NodeRef) -> NodeRef {
    if_then(cond.not(), vec![discard()])
}

/// WGSL's `select( falseValue, trueValue, condition )` builtin, which is what
/// MaterialX's `mx_select` / `mx_negate_if` emit.
pub fn wgsl_select(f: NodeRef, t: NodeRef, cond: NodeRef) -> NodeRef {
    let ty = t.ty();
    math("select", vec![f, t, cond], ty)
}

/// `step( edge, x )`.
pub fn step(edge: impl Into<NodeRef>, x: impl Into<NodeRef>) -> NodeRef {
    let x = x.into();
    let ty = x.ty();
    math("step", vec![edge.into(), x], ty)
}

/// `uint( x )`.
pub fn uint(v: u32) -> NodeRef {
    constant(Type::U32, vec![v as f64])
}

// ---------------------------------------------------------------------------
// shadows (`ShadowBaseNode`, `ShadowNode`, `ShadowFilterNode`)
// ---------------------------------------------------------------------------

prop!(
    /// `shadowPositionWorld` — `property( 'vec3', 'shadowPositionWorld' )`,
    /// assigned by `ShadowBaseNode.setupShadowPosition()` once per shadow.
    shadow_position_world,
    "shadowPositionWorld",
    Type::Vec3
);
/// `cameraViewMatrix.transformDirection( dir )` —
/// `MathNode.TRANSFORM_DIRECTION`: `normalize( ( m * vec4( dir, 0 ) ).xyz )`.
/// Spelt out because the port's `mul` pads a `mat4 * vec3` with `1.0`.
pub fn transform_direction(matrix: NodeRef, dir: NodeRef) -> NodeRef {
    matrix
        .mul(vec4_join(vec![dir, float(0.0)]))
        .xyz()
        .normalize()
}

/// `lightTargetDirection( light )` — `Lights.js`.
pub fn light_target_direction(index: usize) -> NodeRef {
    transform_direction(
        camera_view_matrix(),
        light_world_position(index).sub(light_target_position(index)),
    )
}

/// `texture( depthTexture, uv ).compare( z )` — `ShadowFilterNode`'s
/// `depthCompare`, which lowers to `textureSampleCompare`.
pub fn shadow_map_compare(map: &DepthTexture, coord: NodeRef, z: NodeRef) -> NodeRef {
    texture_node(
        TextureSource::ShadowMap(map.clone()),
        coord,
        SampleMode::Compare(z),
        Type::F32,
    )
}

/// `interleavedGradientNoise( position )` — `PostProcessingUtils.js`.
pub fn interleaved_gradient_noise(position: NodeRef) -> NodeRef {
    thread_local! { static CELL: Lazy<Rc<FnDef>> = const { Lazy::new() }; }
    let def = CELL.with(|c| {
        c.get(|| {
            shader_fn(
                Some("interleavedGradientNoise"),
                vec![("position", Type::Vec2)],
                Type::F32,
                |args| {
                    float(52.9829189)
                        .mul(dot(args[0].clone(), vec2(0.06711056, 0.00583715)).fract())
                        .fract()
                },
            )
        })
    });
    call(&def, vec![position])
}

/// `vogelDiskSample( sampleIndex, samplesCount, phi )` —
/// `PostProcessingUtils.js`.
pub fn vogel_disk_sample(sample_index: NodeRef, samples_count: NodeRef, phi: NodeRef) -> NodeRef {
    thread_local! { static CELL: Lazy<Rc<FnDef>> = const { Lazy::new() }; }
    let def = CELL.with(|c| {
        c.get(|| {
            shader_fn(
                Some("vogelDiskSample"),
                vec![
                    ("sampleIndex", Type::I32),
                    ("samplesCount", Type::I32),
                    ("phi", Type::F32),
                ],
                Type::Vec2,
                |args| {
                    let (index, count, phi) = (args[0].clone(), args[1].clone(), args[2].clone());
                    // `goldenAngle = 2π * ( 2 - φ )`.
                    let theta = index
                        .clone()
                        .to(Type::F32)
                        .mul(float(2.399963229728653))
                        .add(phi);
                    let r = index
                        .to(Type::F32)
                        .add(float(0.5))
                        .div(count.to(Type::F32))
                        .sqrt();
                    vec2_join(vec![theta.cos(), theta.sin()]).mul(r)
                },
            )
        })
    });
    call(&def, vec![sample_index, samples_count, phi])
}

/// `acesFilmicToneMapping( color, exposure )` —
/// `ToneMappingFunctions.js`, emitted as a real `fn`.
pub fn aces_filmic_tone_mapping(color: NodeRef, exposure: NodeRef) -> NodeRef {
    thread_local! { static CELL: Lazy<Rc<FnDef>> = const { Lazy::new() }; }
    let def = CELL.with(|c| {
        c.get(|| {
            shader_fn(
                Some("acesFilmicToneMapping"),
                vec![("color", Type::Vec3), ("exposure", Type::F32)],
                Type::Vec3,
                |args| {
                    let (color, exposure) = (args[0].clone(), args[1].clone());
                    // sRGB → ACEScg, with three.js' `/ 0.6` pre-exposure.
                    let input = constant(
                        Type::Mat3,
                        vec![
                            0.59719, 0.076, 0.0284, //
                            0.35458, 0.90834, 0.13383, //
                            0.04823, 0.01566, 0.83777,
                        ],
                    );
                    let output = constant(
                        Type::Mat3,
                        vec![
                            1.60475, -0.10208, -0.00327, //
                            -0.53108, 1.10813, -0.07276, //
                            -0.07367, -0.00605, 1.07602,
                        ],
                    );
                    let c = input.mul(color.mul(exposure).div(float(0.6)));
                    // `RRTAndODTFit( v )`.
                    let a = c
                        .clone()
                        .mul(c.clone().add(float(0.0245786)))
                        .sub(float(0.000090537));
                    let b = c
                        .clone()
                        .mul(c.add(float(0.432951)).mul(float(0.983729)))
                        .add(float(0.238081));
                    output.mul(a.div(b)).saturate()
                },
            )
        })
    });
    call(&def, vec![color, exposure])
}

/// `neutralToneMapping( color, exposure )` — `ToneMappingFunctions.js`'
/// Khronos PBR Neutral, emitted as a real `fn`.
///
/// `StartCompression` is `0.8 - 0.04`; three.js prints the double as `0.76`,
/// and `0.76f32` is the same float, so the literal is written out.
pub fn neutral_tone_mapping(color: NodeRef, exposure: NodeRef) -> NodeRef {
    thread_local! { static CELL: Lazy<Rc<FnDef>> = const { Lazy::new() }; }
    let def = CELL.with(|c| {
        c.get(|| {
            shader_fn(
                Some("neutralToneMapping"),
                vec![("color", Type::Vec3), ("exposure", Type::F32)],
                Type::Vec3,
                |args| {
                    let start_compression = || float(0.76);
                    let desaturation = float(0.15);
                    // `color = color.mul( exposure )` — reassigned twice
                    // below, so it is a var, not a temp.
                    let color = to_var(None, args[0].clone().mul(args[1].clone()));
                    let x = color.x().min(color.y().min(color.z()));
                    let offset = x
                        .less_than(float(0.08))
                        .select(x.sub(float(6.25).mul(x.mul(x.clone()))), float(0.04));
                    let peak = color.x().max(color.y().max(color.z()));
                    let d = float(1.0).sub(start_compression());
                    let new_peak =
                        float(1.0).sub(d.mul(d.clone()).div(peak.add(d.sub(start_compression()))));
                    let g = float(1.0).sub(
                        float(1.0)
                            .div(desaturation.mul(peak.sub(new_peak.clone())).add(float(1.0))),
                    );
                    block(
                        vec![
                            color.assign(color.sub(offset)),
                            // `If( peak.lessThan( StartCompression ), () => {
                            // return color; } )`.
                            if_statement(
                                peak.less_than(start_compression()),
                                vec![return_statement(color.clone())],
                            ),
                            color.assign(color.mul(new_peak.div(peak))),
                        ],
                        mix(color, new_peak.to(Type::Vec3), g),
                    )
                },
            )
        })
    });
    call(&def, vec![color, exposure])
}
