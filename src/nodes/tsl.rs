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
    Builtin, BufferNode, BufferSource, FnDef, Lazy, Node, NodeRef, SampleMode, Type, UniformGroup,
    UniformNode, UniformSource, VarDef, VaryingDef,
};
use crate::math::Color;
use crate::textures::{CubeTexture, DataArrayTexture, DepthTexture, Texture};

pub use super::node::TextureSource;


// ---------------------------------------------------------------------------
// sub-builds and the build context (`docs/nodes.md` §7)
// ---------------------------------------------------------------------------

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
    /// `normalViewGeometry`'s node per flat-shading flag — the stand-in for
    /// three.js' per-build `nodeData`, which gives the two forms of the
    /// accessor's `Fn( … ).once()` separate cache entries.
    static NORMAL_VIEW_GEOMETRY: RefCell<HashMap<bool, NodeRef>> = RefCell::new(HashMap::new());
    /// `normalView`'s node per (layer, normal value) — the stand-in for
    /// three.js' per-build `nodeData` plus its `subBuildsCache`.
    static NORMAL_VIEW: RefCell<HashMap<(Option<&'static str>, Option<usize>, bool), NodeRef>> =
        RefCell::new(HashMap::new());
    /// `tangentView` / `bitangentView`, keyed by layer the same way.
    static TANGENT_VIEW: RefCell<HashMap<Option<&'static str>, (NodeRef, NodeRef)>> =
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
pub fn with_material_normal<R>(normal: Option<NodeRef>, flat_shading: bool, f: impl FnOnce() -> R) -> R {
    let previous = NORMAL_VALUE.with(|v| v.replace(normal));
    let previous_flat = FLAT_SHADING.with(|v| v.replace(flat_shading));
    let out = f();
    NORMAL_VALUE.with(|v| *v.borrow_mut() = previous);
    FLAT_SHADING.with(|v| *v.borrow_mut() = previous_flat);
    out
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
pub fn vec4(
    x: impl Into<f64>,
    y: impl Into<f64>,
    z: impl Into<f64>,
    w: impl Into<f64>,
) -> NodeRef {
    constant(Type::Vec4, vec![x.into(), y.into(), z.into(), w.into()])
}

/// `vec4( node, w )` — `JoinNode`.
pub fn join(ty: Type, args: Vec<NodeRef>) -> NodeRef {
    NodeRef::new(Node::Join { args, ty })
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

/// `attribute( name, type )`.
pub fn attribute(name: &'static str, ty: Type) -> NodeRef {
    NodeRef::new(Node::Attribute { name, ty })
}

/// `uniform( value )` — a plain value uniform in the object group.
pub fn uniform_value(ty: Type, values: Vec<f64>) -> NodeRef {
    uniform(UniformSource::Value(values), ty, UniformGroup::Object, None)
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

/// `OperatorNode.getNodeType()` plus the operand padding
/// `NodeBuilder.format()` performs: `mat4 * vec3` becomes
/// `mat4 * vec4( v, 1.0 )`.
fn binary(op: &'static str, a: NodeRef, b: NodeRef) -> NodeRef {
    let (ta, tb) = (a.ty(), b.ty());

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
        let want = if ta == Type::Mat4 { 4 } else { 3 };
        let ty = Type::vector_of(Type::F32, want);
        let b = pad(b, want);
        return NodeRef::new(Node::Op { op, a, b, ty });
    }

    // vector * matrix: the row-vector form three.js uses for inverse-transpose
    // transforms (`vec4( n, 0.0 ) * cameraViewMatrix`).
    if tb.is_matrix() && !ta.is_matrix() {
        let want = if tb == Type::Mat4 { 4 } else { 3 };
        let ty = Type::vector_of(Type::F32, want);
        let a = pad(a, want);
        return NodeRef::new(Node::Op { op, a, b, ty });
    }

    let ty = if ta.is_matrix() {
        ta
    } else if tb.is_matrix() {
        tb
    } else if tb.components() > ta.components() {
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

/// `length( v )` — a scalar out of any vector.
pub fn length(v: impl Into<NodeRef>) -> NodeRef {
    math("length", vec![v.into()], Type::F32)
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
    math("smoothstep", vec![low.into(), high.into(), x.into()], Type::F32)
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

/// `inverseSqrt( x )`.
pub fn inverse_sqrt(x: impl Into<NodeRef>) -> NodeRef {
    math("inverseSqrt", vec![x.into()], Type::F32)
}

/// `AccessorsUtils.js`' `TBNViewMatrix` — `mat3( tangentView, bitangentView,
/// normalView ).toVar( 'TBNViewMatrix' )`. Three tags it with whichever
/// sub-build layers its descendants declare, which is why the centre teapot's
/// dump calls it `NORMAL_TBNViewMatrix`; here the layer is simply still open.
pub fn tbn_view_matrix() -> NodeRef {
    thread_local! {
        static CELL: Lazy<NodeRef> = Lazy::new();
    }
    CELL.with(|c| {
        c.get(|| {
            to_var(
                Some("TBNViewMatrix"),
                join(
                    Type::Mat3,
                    vec![tangent_view(), bitangent_view(), normal_view()],
                ),
            )
        })
    })
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
    pub fn floor(&self) -> NodeRef {
        math("floor", vec![self.clone()], self.ty())
    }
    pub fn pow(&self, other: impl Into<NodeRef>) -> NodeRef {
        math("pow", vec![self.clone(), other.into()], self.ty())
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
    pub fn xyz(&self) -> NodeRef {
        swizzle(self.clone(), "xyz")
    }
    pub fn rgb(&self) -> NodeRef {
        swizzle(self.clone(), "xyz")
    }
    pub fn a(&self) -> NodeRef {
        swizzle(self.clone(), "w")
    }

    /// `vec4( node.x, node.y, z, node.w )` — `SetNode` for `.setZ()`.
    pub fn set_z(&self, z: impl Into<NodeRef>) -> NodeRef {
        join(
            Type::Vec4,
            vec![self.x(), self.y(), z.into(), self.w()],
        )
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
            other => other,
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

    /// `target.mulAssign( value )` — `target = ( target * value )`.
    pub fn mul_assign(&self, value: impl Into<NodeRef>) -> NodeRef {
        self.assign(self.mul(value))
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
                static CELL: Lazy<NodeRef> = Lazy::new();
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
    /// `viewportSize`.
    viewport_size,
    uniform(
        UniformSource::ViewportSize,
        Type::Vec2,
        UniformGroup::Render,
        None
    )
);

accessor!(
    /// `positionLocal` — the geometry position, as a var so that instancing,
    /// morphing and skinning can reassign it.
    position_local,
    to_var(Some("positionLocal"), position_geometry())
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
accessor!(
    /// `positionView` — `NodeMaterial.setupPositionView()`.
    position_view,
    to_varying(
        Some("v_positionView"),
        model_view_matrix()
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
    thread_local! { static CELL: Lazy<NodeRef> = Lazy::new(); }
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
pub fn normal_view() -> NodeRef {
    let layer = SUB_BUILD.with(|s| *s.borrow());
    let value = if layer.is_some() {
        None
    } else {
        NORMAL_VALUE.with(|v| v.borrow().clone())
    };
    let key = (
        layer,
        value.as_ref().map(|v| v.key()),
        FLAT_SHADING.with(|f| *f.borrow()),
    );
    if let Some(node) = NORMAL_VIEW.with(|m| m.borrow().get(&key).cloned()) {
        return node;
    }
    let node = to_var(
        Some("normalView"),
        value.unwrap_or_else(normal_view_geometry),
    );
    NORMAL_VIEW.with(|m| m.borrow_mut().insert(key, node.clone()));
    node
}

/// `tangentView` / `bitangentView` for a geometry with no `tangent` attribute:
/// the derivative frame of `TangentUtils.js`, both `.once( [ 'NORMAL',
/// 'VERTEX' ] )` so they take the layer prefix. They are returned as a pair
/// because `tangentViewFrame` and `bitangentViewFrame` share the `scale` temp.
fn tangent_frame() -> (NodeRef, NodeRef) {
    let layer = SUB_BUILD.with(|s| *s.borrow());
    if let Some(pair) = TANGENT_VIEW.with(|m| m.borrow().get(&layer).cloned()) {
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

    let pair = (
        to_var(
            Some("tangentView"),
            to_var_untagged("tangentViewFrame", t.mul(scale.clone())),
        ),
        to_var(
            Some("bitangentView"),
            to_var_untagged("bitangentViewFrame", b.mul(scale)),
        ),
    );
    TANGENT_VIEW.with(|m| m.borrow_mut().insert(layer, pair.clone()));
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
accessor!(
    /// `normalWorld` — `normalView` rotated out of view space.
    normal_world,
    to_var(
        Some("normalWorld"),
        vec4_join(vec![normal_view(), float(0.0)])
            .mul(camera_view_matrix())
            .xyz()
            .normalize()
    )
);
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
accessor!(
    /// `modelViewProjection`.
    model_view_projection,
    to_varying(
        Some("v_modelViewProjection"),
        camera_projection_matrix().mul(position_view())
    )
);
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
    thread_local! { static CELL: Lazy<NodeRef> = Lazy::new(); }
    CELL.with(|c| c.get(|| property("DiffuseColor", Type::Vec4)))
}

macro_rules! prop {
    ($name:ident, $wgsl:literal, $ty:expr) => {
        pub fn $name() -> NodeRef {
            thread_local! { static CELL: Lazy<NodeRef> = Lazy::new(); }
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
            thread_local! { static CELL: Lazy<NodeRef> = Lazy::new(); }
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
/// applies before sampling.
fn transformed_uv(uv: NodeRef) -> NodeRef {
    uniform(
        UniformSource::TextureMatrix,
        Type::Mat3,
        UniformGroup::Object,
        None,
    )
    .mul(vec3_join(vec![uv, float(1.0)]))
    .xy()
}

/// `texture( map )`.
pub fn texture(map: &Texture) -> NodeRef {
    texture_node(
        TextureSource::Texture2D(map.clone()),
        transformed_uv(uv()),
        SampleMode::Sample,
        Type::Vec4,
    )
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
        transformed_uv(uv()),
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

fn buffer_element(source: BufferSource, element_ty: Type, count: usize, index: NodeRef) -> NodeRef {
    NodeRef::new(Node::BufferElement {
        buffer: Rc::new(BufferNode {
            source,
            element_ty,
            count,
        }),
        index,
    })
}

/// `InstanceNode`'s `instanceMatrix` buffer, indexed by `instanceIndex`.
pub fn instance_matrix(count: usize) -> NodeRef {
    buffer_element(
        BufferSource::InstanceMatrix,
        Type::Mat4,
        count,
        instance_index(),
    )
}

/// `range( min, max )` — `RangeNode` on an `InstancedMesh` resolves to one
/// `vec4` per instance, `lerp( min[c], max[c], Math.random() )` per component.
pub fn range(min: Color, max: Color, count: usize, index: NodeRef) -> NodeRef {
    buffer_element(BufferSource::Range { min, max }, Type::Vec4, count, index)
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
    NodeRef::new(Node::Loop { index, count, body })
}

/// `If( cond, () => { … } )` with no `Else` — a statement.
pub fn if_statement(cond: NodeRef, body: Vec<NodeRef>) -> NodeRef {
    NodeRef::new(Node::If { cond, body })
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
    uniform(UniformSource::MorphBase, Type::F32, UniformGroup::Object, None)
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
    thread_local! { static CELL: Lazy<Rc<FnDef>> = Lazy::new(); }
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
    thread_local! { static CELL: Lazy<Rc<FnDef>> = Lazy::new(); }
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
                        k.cross(color.rgb()).mul(adjustment.sin()).add(
                            k.mul(dot(k.clone(), color.rgb()).mul(cos_angle.one_minus())),
                        ),
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
    thread_local! { static CELL: Lazy<Rc<FnDef>> = Lazy::new(); }
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

/// `premultiplyAlpha` — `PremultiplyAlphaFunctions.js`.
pub fn premultiply_alpha(color: NodeRef) -> NodeRef {
    thread_local! { static CELL: Lazy<Rc<FnDef>> = Lazy::new(); }
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
    thread_local! { static CELL: Lazy<Rc<FnDef>> = Lazy::new(); }
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
