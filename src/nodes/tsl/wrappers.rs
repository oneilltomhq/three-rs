//! The thin-wrapper tier of TSL (issue #141): the constructors, constants and
//! named math functions that three.js exports as one-liners over `ConstNode`,
//! `ConvertNode`, `JoinNode`, `MathNode`, `OperatorNode` and a few inlined
//! `Fn()`s. Each item's doc comment names the JS it ports; the WGSL each one
//! emits is pinned against three's own dump in `tests/nodes_tsl_batch.rs`.
//!
//! Where three's output and the port's differ only in how a shared
//! intermediate is named (`let nodeConstN` in three, `var<private> nodeVarN`
//! here), the divergence is the general one `docs/nodes.md` §8 records, not
//! something specific to these wrappers.

use std::rc::Rc;

use super::{
    binary, block, call, constant, float, join, math, rotate, shader_fn, to_const, to_var,
};
use super::{
    camera_position, camera_projection_matrix, camera_view_matrix, model_world_matrix,
    position_geometry, position_world, uniform,
};
use crate::math::{Color, Matrix2, Matrix3, Matrix4};
use crate::nodes::node::{FnDef, Lazy, Node, NodeRef, Type, UniformGroup, UniformSource};

// ---------------------------------------------------------------------------
// type helpers
// ---------------------------------------------------------------------------

/// `MathNode.getInputType()`: the widest operand, matrices counting as
/// width 0, with `a` winning ties.
fn input_type(args: &[&NodeRef]) -> Type {
    let len = |n: Option<&&NodeRef>| match n {
        Some(n) if !n.ty().is_matrix() => n.ty().components(),
        _ => 0,
    };
    let (a, b, c) = (args.first(), args.get(1), args.get(2));
    let (la, lb, lc) = (len(a), len(b), len(c));
    if la > lb && la > lc {
        a.unwrap().ty()
    } else if lb > lc {
        b.unwrap().ty()
    } else if lc > la {
        c.unwrap().ty()
    } else {
        a.unwrap().ty()
    }
}

/// `NodeBuilder.getIntegerType()`: an integer type keeps itself, anything
/// else becomes the `i32` vector of its width.
fn integer_type(ty: Type) -> Type {
    match ty.component_type() {
        Type::I32 | Type::U32 => ty,
        _ => Type::vector_of(Type::I32, ty.components()),
    }
}

/// A one-operand `MathNode`, typed by its operand.
fn unary(name: &'static str, x: NodeRef) -> NodeRef {
    let ty = x.ty();
    math(name, vec![x], ty)
}

/// `ConvertType( type, ...params )` from `TSLCore.js`: what `uvec3( … )`,
/// `bvec2( … )` and friends are.
///
/// - Every parameter a number: one `ConstNode` of the target type (a single
///   scalar splats), which `generateConst()` writes component by component —
///   `vec3<u32>( 1u, 2u, 3u )`.
/// - One node: a `ConvertNode` (`u32( x )`), or the node itself when it
///   already has the type.
/// - Several nodes: a `JoinNode`.
///
/// Rust has no JS-number/node distinction at the call site, so a scalar
/// `Node::Const` stands in for a JS number. Three would join a `float( 1 )`
/// *node* rather than fold it; nothing ported passes one, and folding is the
/// literal three emits for the number form every example uses.
fn convert_type(ty: Type, params: Vec<NodeRef>) -> NodeRef {
    let n = ty.components();
    if params.is_empty() {
        return constant(ty, vec![0.0; n]);
    }
    let all_const = params.iter().all(|p| matches!(&*p.0, Node::Const { .. }));
    if all_const {
        let mut values: Vec<f64> = params
            .iter()
            .flat_map(|p| match &*p.0 {
                Node::Const { values, .. } => values.clone(),
                _ => unreachable!(),
            })
            .collect();
        if values.len() == 1 {
            values = vec![values[0]; n];
        }
        values.resize(n, 0.0);
        return constant(ty, values);
    }
    if params.len() == 1 {
        return params[0].to(ty);
    }
    join(ty, params)
}

// ---------------------------------------------------------------------------
// constructors (`TSLCore.js`)
// ---------------------------------------------------------------------------

/// `color( 0xff8000 )` — `TSLCore.js`' `color = new ConvertType( 'color' )`
/// over a `Color`. The hex is sRGB and the const is the working (linear)
/// value, exactly as `new Color( hex )` stores it.
pub fn color(hex: u32) -> NodeRef {
    Color::from_hex(hex).into()
}

/// `color( r, g, b )` — the three-number form, which `Color.setRGB()` takes
/// as working-space components unchanged.
pub fn color_rgb(r: f64, g: f64, b: f64) -> NodeRef {
    constant(Type::Vec3, vec![r, g, b])
}

/// `bool( x )` — `TSLCore.js`' `bool = new ConvertType( 'bool' )`.
pub fn bool(x: impl Into<NodeRef>) -> NodeRef {
    convert_type(Type::Bool, vec![x.into()])
}

macro_rules! vector_ctor {
    ($(#[$m:meta])* $name:ident, $ty:expr, $($arg:ident),+) => {
        $(#[$m])*
        pub fn $name($($arg: impl Into<NodeRef>),+) -> NodeRef {
            convert_type($ty, vec![$($arg.into()),+])
        }
    };
}

vector_ctor!(
    /// `uvec2( x, y )` — `TSLCore.js`' `ConvertType( 'uvec2' )`.
    uvec2, Type::UVec2, x, y
);
vector_ctor!(
    /// `uvec3( x, y, z )` — `TSLCore.js`' `ConvertType( 'uvec3' )`.
    uvec3, Type::UVec3, x, y, z
);
vector_ctor!(
    /// `uvec4( x, y, z, w )` — `TSLCore.js`' `ConvertType( 'uvec4' )`.
    uvec4, Type::UVec4, x, y, z, w
);
vector_ctor!(
    /// `ivec3( x, y, z )` — `TSLCore.js`' `ConvertType( 'ivec3' )`.
    ivec3, Type::IVec3, x, y, z
);
vector_ctor!(
    /// `ivec4( x, y, z, w )` — `TSLCore.js`' `ConvertType( 'ivec4' )`.
    ivec4, Type::IVec4, x, y, z, w
);
vector_ctor!(
    /// `bvec2( x, y )` — `TSLCore.js`' `ConvertType( 'bvec2' )`.
    bvec2, Type::BVec2, x, y
);
vector_ctor!(
    /// `bvec3( x, y, z )` — `TSLCore.js`' `ConvertType( 'bvec3' )`.
    bvec3, Type::BVec3, x, y, z
);
vector_ctor!(
    /// `bvec4( x, y, z, w )` — `TSLCore.js`' `ConvertType( 'bvec4' )`.
    bvec4, Type::BVec4, x, y, z, w
);

/// `mat2( matrix2 )` / `mat2( a, b, c, d )` with numbers — `TSLCore.js`'
/// `ConvertType( 'mat2' )`, whose number form goes through `Matrix2.set()`
/// (row-major arguments). The const is written column-major, as
/// `generateConst()` reads `matrix.elements`: `mat2( 1, 2, 3, 4 )` is
/// `mat2x2<f32>( 1.0, 3.0, 2.0, 4.0 )`.
pub fn mat2(m: Matrix2) -> NodeRef {
    constant(Type::Mat2, m.elements.to_vec())
}

/// `mat3( matrix3 )` — `TSLCore.js`' `ConvertType( 'mat3' )` over a value.
pub fn mat3(m: Matrix3) -> NodeRef {
    constant(Type::Mat3, m.elements.to_vec())
}

/// `mat4( matrix4 )` — `TSLCore.js`' `ConvertType( 'mat4' )` over a value.
pub fn mat4(m: Matrix4) -> NodeRef {
    constant(Type::Mat4, m.elements.to_vec())
}

/// `mat2( a, b, c, d )` with nodes — a `JoinNode` in column order.
pub fn mat2_join(args: Vec<NodeRef>) -> NodeRef {
    convert_type(Type::Mat2, args)
}

/// `mat3( … )` with nodes — a `JoinNode` (or a `ConvertNode` for one node:
/// `mat3( modelWorldMatrix )` is the `mat4` → `mat3` narrowing).
pub fn mat3_join(args: Vec<NodeRef>) -> NodeRef {
    convert_type(Type::Mat3, args)
}

/// `mat4( … )` with nodes.
pub fn mat4_join(args: Vec<NodeRef>) -> NodeRef {
    convert_type(Type::Mat4, args)
}

// ---------------------------------------------------------------------------
// constants (`math/MathNode.js`)
// ---------------------------------------------------------------------------

/// `PI` — `MathNode.js`: `float( Math.PI )`.
pub fn pi() -> NodeRef {
    float(std::f64::consts::PI)
}

/// `PI2` — `MathNode.js`: `float( Math.PI * 2 )`, the deprecated spelling of
/// `TWO_PI` ([`super::two_pi`]).
pub fn pi2() -> NodeRef {
    float(std::f64::consts::TAU)
}

/// `HALF_PI` — `MathNode.js`: `float( Math.PI * 0.5 )`.
pub fn half_pi() -> NodeRef {
    float(std::f64::consts::FRAC_PI_2)
}

/// `EPSILON` — `MathNode.js`: `float( 1e-6 )`.
pub fn epsilon() -> NodeRef {
    float(1e-6)
}

/// `INFINITY` — `MathNode.js`: `float( 1e6 )`. Not IEEE infinity: three's
/// constant is a large finite number, and so is this.
pub fn infinity() -> NodeRef {
    float(1e6)
}

// ---------------------------------------------------------------------------
// one-operand math (`math/MathNode.js`)
// ---------------------------------------------------------------------------

macro_rules! unary_math {
    ($(#[$m:meta])* $name:ident, $wgsl:literal) => {
        $(#[$m])*
        pub fn $name(x: impl Into<NodeRef>) -> NodeRef {
            unary($wgsl, x.into())
        }
    };
}

unary_math!(
    /// `atan( x )` — `MathNode.ATAN` with one argument. The two-argument form
    /// is [`NodeRef::atan2`].
    atan, "atan"
);
unary_math!(
    /// `acos( x )` — `MathNode.ACOS`.
    acos, "acos"
);
unary_math!(
    /// `asin( x )` — `MathNode.ASIN`.
    asin, "asin"
);
unary_math!(
    /// `tan( x )` — `MathNode.TAN`.
    tan, "tan"
);
unary_math!(
    /// `sinh( x )` — `MathNode.SINH`.
    sinh, "sinh"
);
unary_math!(
    /// `cosh( x )` — `MathNode.COSH`.
    cosh, "cosh"
);
unary_math!(
    /// `tanh( x )` — `MathNode.TANH`.
    tanh, "tanh"
);
unary_math!(
    /// `asinh( x )` — `MathNode.ASINH`.
    asinh, "asinh"
);
unary_math!(
    /// `acosh( x )` — `MathNode.ACOSH`.
    acosh, "acosh"
);
unary_math!(
    /// `atanh( x )` — `MathNode.ATANH`.
    atanh, "atanh"
);
unary_math!(
    /// `round( x )` — `MathNode.ROUND`.
    round, "round"
);
unary_math!(
    /// `trunc( x )` — `MathNode.TRUNC`.
    trunc, "trunc"
);
unary_math!(
    /// `degrees( x )` — `MathNode.DEGREES`.
    degrees, "degrees"
);
unary_math!(
    /// `radians( x )` — `MathNode.RADIANS`.
    radians, "radians"
);
unary_math!(
    /// `transpose( m )` — `MathNode.TRANSPOSE`.
    transpose, "transpose"
);
unary_math!(
    /// `countOneBits( x )` — `BitcountNode.COUNT_ONE_BITS`, WGSL's builtin
    /// of the same name, typed as its operand.
    count_one_bits, "countOneBits"
);
unary_math!(
    /// `countTrailingZeros( x )` — `BitcountNode.COUNT_TRAILING_ZEROS`.
    count_trailing_zeros, "countTrailingZeros"
);
unary_math!(
    /// `countLeadingZeros( x )` — `BitcountNode.COUNT_LEADING_ZEROS`.
    count_leading_zeros, "countLeadingZeros"
);

/// `determinant( m )` — `MathNode.DETERMINANT`, a `float` out of a matrix.
pub fn determinant(m: impl Into<NodeRef>) -> NodeRef {
    math("determinant", vec![m.into()], Type::F32)
}

/// `inverse( m )` — `MathNode.INVERSE`. WGSL has no `inverse`, so
/// `WGSLNodeBuilder`'s polyfill table lowers it to `tsl_inverse_mat2/3/4`,
/// whose code the builder adds on first use (`wgsl::polyfill`).
pub fn inverse(m: impl Into<NodeRef>) -> NodeRef {
    let m = m.into();
    let name = match m.ty() {
        Type::Mat2 => "tsl_inverse_mat2",
        Type::Mat3 => "tsl_inverse_mat3",
        Type::Mat4 => "tsl_inverse_mat4",
        other => panic!("three-rs: inverse() of a {other:?}"),
    };
    unary(name, m)
}

// ---------------------------------------------------------------------------
// powers and shaping (`math/MathNode.js`, `math/MathUtils.js`)
// ---------------------------------------------------------------------------

/// `pow( a, b )` typed by `MathNode.getInputType()`.
fn pow_of(a: NodeRef, b: NodeRef) -> NodeRef {
    let ty = input_type(&[&a, &b]);
    math("pow", vec![a, b], ty)
}

/// `pow2( x )` — `MathNode.js`: `mul( x, x )`.
pub fn pow2(x: impl Into<NodeRef>) -> NodeRef {
    let x = x.into();
    x.mul(x.clone())
}

/// `pow3( x )` — `MathNode.js`: `mul( x, x, x )`.
pub fn pow3(x: impl Into<NodeRef>) -> NodeRef {
    x.into().pow3()
}

/// `pow4( x )` — `MathNode.js`: `mul( x, x, x, x )`.
pub fn pow4(x: impl Into<NodeRef>) -> NodeRef {
    let x = x.into();
    x.mul(x.clone()).mul(x.clone()).mul(x)
}

/// `difference( a, b )` — `MathNode.DIFFERENCE`, `abs( a - b )`.
pub fn difference(a: impl Into<NodeRef>, b: impl Into<NodeRef>) -> NodeRef {
    let d = a.into().sub(b);
    unary("abs", d)
}

/// `cbrt( a )` — `MathNode.js`: `mul( sign( a ), pow( abs( a ), 1.0 / 3.0 ) )`.
pub fn cbrt(a: impl Into<NodeRef>) -> NodeRef {
    let a = a.into();
    unary("sign", a.clone()).mul(pow_of(unary("abs", a), float(1.0 / 3.0)))
}

/// `lengthSq( a )` — `MathNode.js`: `dot( a, a )`.
pub fn length_sq(a: impl Into<NodeRef>) -> NodeRef {
    let a = a.into();
    super::dot(a.clone(), a)
}

/// `faceForward( N, I, Nref )` — `MathNode.FACEFORWARD`, WGSL's builtin.
pub fn face_forward(
    n: impl Into<NodeRef>,
    i: impl Into<NodeRef>,
    nref: impl Into<NodeRef>,
) -> NodeRef {
    let (n, i, nref) = (n.into(), i.into(), nref.into());
    let ty = input_type(&[&n, &i, &nref]);
    math("faceForward", vec![n, i, nref], ty)
}

/// `parabola( x, k )` — `MathUtils.js`:
/// `pow( mul( 4.0, x.mul( sub( 1.0, x ) ) ), k )`.
pub fn parabola(x: impl Into<NodeRef>, k: impl Into<NodeRef>) -> NodeRef {
    let x = x.into();
    pow_of(float(4.0).mul(x.mul(float(1.0).sub(x.clone()))), k.into())
}

/// `gain( x, k )` — `MathUtils.js`: `select( x.lessThan( 0.5 ), pow( mul(
/// 2.0, x ), k ).mul( 0.5 ), sub( 1.0, pow( mul( 2.0, sub( 1.0, x ) ), k
/// ).mul( 0.5 ) ) )`, which generates as an `if`/`else` over a result var.
pub fn gain(x: impl Into<NodeRef>, k: impl Into<NodeRef>) -> NodeRef {
    let (x, k) = (x.into(), k.into());
    let low = pow_of(float(2.0).mul(x.clone()), k.clone()).mul(0.5);
    let high = float(1.0).sub(pow_of(float(2.0).mul(float(1.0).sub(x.clone())), k).mul(0.5));
    x.less_than(0.5).select(low, high)
}

/// `pcurve( x, a, b )` — `MathUtils.js`: `pow( div( pow( x, a ), add( pow(
/// x, a ), pow( sub( 1.0, x ), b ) ) ), div( 1.0, a ) )`. The two `pow( x, a
/// )` are two nodes in three, so they are two expressions here too.
pub fn pcurve(x: impl Into<NodeRef>, a: impl Into<NodeRef>, b: impl Into<NodeRef>) -> NodeRef {
    let (x, a, b) = (x.into(), a.into(), b.into());
    let num = pow_of(x.clone(), a.clone());
    let den = pow_of(x.clone(), a.clone()).add(pow_of(float(1.0).sub(x), b));
    pow_of(num.div(den), float(1.0).div(a))
}

/// `sinc( x, k )` — `MathUtils.js`: `arg = abs( PI.mul( k.mul( x ).sub( 1.0
/// ) ) ).max( 1e-6 ).toConst()`, then `sin( arg ).div( arg )`. Three calls
/// `k.mul()`, so `k` must be a node there; here anything `Into<NodeRef>` is.
pub fn sinc(x: impl Into<NodeRef>, k: impl Into<NodeRef>) -> NodeRef {
    let (x, k) = (x.into(), k.into());
    let arg = unary("abs", pi().mul(k.mul(x).sub(1.0))).max(1e-6);
    let arg = to_const(None, arg);
    arg.sin().div(arg)
}

// ---------------------------------------------------------------------------
// bits and integers (`math/OperatorNode.js`, `math/BitcastNode.js`)
// ---------------------------------------------------------------------------

/// `xor( a, b )` — `OperatorNode( '^^' )`, the logical exclusive or. WGSL has
/// no `^^`, so it lowers to `WGSLNodeBuilder`'s `tsl_xor` polyfill.
pub fn xor(a: impl Into<NodeRef>, b: impl Into<NodeRef>) -> NodeRef {
    math("tsl_xor", vec![a.into(), b.into()], Type::Bool)
}

/// `bitNot( a )` — `OperatorNode( '~' )`, typed `getIntegerType( typeA )`.
pub fn bit_not(a: impl Into<NodeRef>) -> NodeRef {
    let node = a.into();
    let ty = integer_type(node.ty());
    NodeRef::new(Node::BitNot { node, ty })
}

/// A `BitcastNode` over `x` into the vector of `component` of `x`'s width:
/// `bitcast<T>( x )`.
fn bitcast(x: NodeRef, component: Type) -> NodeRef {
    let ty = Type::vector_of(component, x.ty().components());
    let name = match ty {
        Type::F32 => "bitcast<f32>",
        Type::I32 => "bitcast<i32>",
        Type::U32 => "bitcast<u32>",
        Type::Vec2 => "bitcast<vec2<f32>>",
        Type::Vec3 => "bitcast<vec3<f32>>",
        Type::Vec4 => "bitcast<vec4<f32>>",
        Type::IVec2 => "bitcast<vec2<i32>>",
        Type::IVec3 => "bitcast<vec3<i32>>",
        Type::IVec4 => "bitcast<vec4<i32>>",
        Type::UVec2 => "bitcast<vec2<u32>>",
        Type::UVec3 => "bitcast<vec3<u32>>",
        Type::UVec4 => "bitcast<vec4<u32>>",
        other => panic!("three-rs: bitcast to {other:?}"),
    };
    math(name, vec![x], ty)
}

/// `floatBitsToInt( x )` — `BitcastNode.js`: `new BitcastNode( x, 'int' )`.
pub fn float_bits_to_int(x: impl Into<NodeRef>) -> NodeRef {
    bitcast(x.into(), Type::I32)
}

/// `floatBitsToUint( x )` — `BitcastNode.js`: `new BitcastNode( x, 'uint' )`.
pub fn float_bits_to_uint(x: impl Into<NodeRef>) -> NodeRef {
    bitcast(x.into(), Type::U32)
}

/// `intBitsToFloat( x )` — `BitcastNode.js`: `new BitcastNode( x, 'float' )`.
pub fn int_bits_to_float(x: impl Into<NodeRef>) -> NodeRef {
    bitcast(x.into(), Type::F32)
}

/// `uintBitsToFloat( x )` — `BitcastNode.js`: `new BitcastNode( x, 'float' )`.
pub fn uint_bits_to_float(x: impl Into<NodeRef>) -> NodeRef {
    bitcast(x.into(), Type::F32)
}

/// `increment( a )` — `OperatorNode.js`: `temp = int( a ).toConst();
/// a.addAssign( 1 ); return temp;`. `a` is a var; the value is `a` as it was.
pub fn increment(a: &NodeRef) -> NodeRef {
    let temp = to_const(None, a.to(Type::I32));
    block(vec![temp.clone(), a.add_assign(1)], temp)
}

/// `decrement( a )` — `OperatorNode.js`: the post-decrement twin of
/// [`increment`].
pub fn decrement(a: &NodeRef) -> NodeRef {
    let temp = to_const(None, a.to(Type::I32));
    block(vec![temp.clone(), a.sub_assign(1)], temp)
}

/// `incrementBefore( a )` — `OperatorNode.js`: `a.addAssign( 1 ); return a;`.
pub fn increment_before(a: &NodeRef) -> NodeRef {
    block(vec![a.add_assign(1)], a.clone())
}

/// `decrementBefore( a )` — `OperatorNode.js`: `a.subAssign( 1 ); return a;`.
pub fn decrement_before(a: &NodeRef) -> NodeRef {
    block(vec![a.sub_assign(1)], a.clone())
}

/// `mod( a, b )` — `MathNode.MOD`. On integers it is the `%` operator; on
/// floats WGSL's `%` truncates where GLSL's `mod` floors, so
/// `WGSLNodeBuilder` lowers it to `tsl_mod_float` / `tsl_mod_vec2/3/4`, with
/// `b` widened to the input type (`uv.mod( 0.5 )` is `tsl_mod_vec2( uv,
/// vec2<f32>( 0.5 ) )`). `mod` is a Rust keyword, hence the trailing `_`.
pub fn mod_(a: impl Into<NodeRef>, b: impl Into<NodeRef>) -> NodeRef {
    let (a, b) = (a.into(), b.into());
    let ty = input_type(&[&a, &b]);
    if matches!(ty.component_type(), Type::I32 | Type::U32) {
        return binary("%", a, b);
    }
    let name = match ty {
        Type::F32 => "tsl_mod_float",
        Type::Vec2 => "tsl_mod_vec2",
        Type::Vec3 => "tsl_mod_vec3",
        Type::Vec4 => "tsl_mod_vec4",
        other => panic!("three-rs: mod() of a {other:?}"),
    };
    math(name, vec![a, b], ty)
}

// ---------------------------------------------------------------------------
// noise and hashing (`math/Hash.js`, `math/MathNode.js`, `math/TriNoise3D.js`)
// ---------------------------------------------------------------------------

/// `hash( seed )` — `Hash.js`, the PCG hash from shadertoy XlGcRh, inlined:
///
/// ```ignore
/// state = seed.toUint().mul( 747796405 ).add( 2891336453 )
/// word = state.shiftRight( state.shiftRight( 28 ).add( 4 ) ).bitXor( state ).mul( 277803737 )
/// return word.shiftRight( 22 ).bitXor( word ).toFloat().mul( 1 / 2 ** 32 )
/// ```
pub fn hash(seed: impl Into<NodeRef>) -> NodeRef {
    let state = seed.into().to_uint().mul(747796405u32).add(2891336453u32);
    let word = state
        .shift_right(state.shift_right(28u32).add(4u32))
        .bit_xor(state.clone())
        .mul(277803737u32);
    word.shift_right(22u32)
        .bit_xor(word)
        .to_float()
        .mul(1.0 / 4294967296.0)
}

/// `rand( uv )` — `MathNode.js`: `fract( sin( mod( dot( uv.xy, vec2(
/// 12.9898, 78.233 ) ), PI ) ).mul( 43758.5453 ) )`, inlined.
pub fn rand(uv: impl Into<NodeRef>) -> NodeRef {
    let uv = uv.into();
    // `uv.xy` of a `vec2` is `SplitNode` over the whole vector, which three
    // generates as the vector itself.
    let uv = if uv.ty() == Type::Vec2 { uv } else { uv.xy() };
    let dt = super::dot(uv, constant(Type::Vec2, vec![12.9898, 78.233]));
    let sn = mod_(dt, pi());
    unary("fract", sn.sin().mul(43758.5453))
}

/// `tri( x )` — `TriNoise3D.js`, a layout `Fn` (`fn tri`).
fn tri_def() -> Rc<FnDef> {
    thread_local! { static CELL: Lazy<Rc<FnDef>> = const { Lazy::new() }; }
    CELL.with(|c| {
        c.get(|| {
            shader_fn(Some("tri"), vec![("x", Type::F32)], Type::F32, |args| {
                args[0].fract().sub(0.5).abs()
            })
        })
    })
}

fn tri(x: NodeRef) -> NodeRef {
    call(&tri_def(), vec![x])
}

/// `tri3( p )` — `TriNoise3D.js`, a layout `Fn` (`fn tri3`).
fn tri3(p: NodeRef) -> NodeRef {
    thread_local! { static CELL: Lazy<Rc<FnDef>> = const { Lazy::new() }; }
    let def = CELL.with(|c| {
        c.get(|| {
            shader_fn(Some("tri3"), vec![("p", Type::Vec3)], Type::Vec3, |args| {
                let p = &args[0];
                join(
                    Type::Vec3,
                    vec![
                        tri(p.z().add(tri(p.y().mul(1.0)))),
                        tri(p.z().add(tri(p.x().mul(1.0)))),
                        tri(p.y().add(tri(p.x().mul(1.0)))),
                    ],
                )
            })
        })
    });
    call(&def, vec![p])
}

/// `triNoise3D( position, speed, time )` — `TriNoise3D.js`
/// (cabbibo's glsl-tri-noise-3d), a layout `Fn` emitted as `fn triNoise3D`
/// with its float `Loop( { start: 0, end: 3, type: 'float', condition: '<='
/// } )`.
pub fn tri_noise_3d(
    position: impl Into<NodeRef>,
    speed: impl Into<NodeRef>,
    time: impl Into<NodeRef>,
) -> NodeRef {
    thread_local! { static CELL: Lazy<Rc<FnDef>> = const { Lazy::new() }; }
    let def = CELL.with(|c| {
        c.get(|| {
            shader_fn(
                Some("triNoise3D"),
                vec![
                    ("position", Type::Vec3),
                    ("speed", Type::F32),
                    ("time", Type::F32),
                ],
                Type::F32,
                |args| {
                    let (position, speed, time) = (&args[0], &args[1], &args[2]);
                    let p = to_var(None, position.clone());
                    let z = to_var(None, float(1.4));
                    let rz = to_var(None, float(0.0));
                    let bp = to_var(None, p.clone());
                    let index = NodeRef::new(Node::Param {
                        name: "i",
                        ty: Type::F32,
                    });
                    let dg = to_var(None, tri3(bp.mul(2.0)));
                    let t = to_var(None, tri(p.z().add(tri(p.x().add(tri(p.y()))))));
                    let body = vec![
                        dg.clone(),
                        p.add_assign(dg.add(time.mul(float(0.1).mul(speed.clone())))),
                        bp.mul_assign(1.8),
                        z.mul_assign(1.5),
                        p.mul_assign(1.2),
                        t.clone(),
                        rz.add_assign(t.div(z.clone())),
                        bp.add_assign(0.14),
                    ];
                    let looped = NodeRef::new(Node::Loop {
                        start: Some(float(0.0)),
                        count: float(3.0),
                        index,
                        condition: "<=",
                        body,
                    });
                    block(vec![p, z, rz.clone(), bp, looped], rz)
                },
            )
        })
    });
    call(
        &def,
        vec![position.into().to(Type::Vec3), speed.into(), time.into()],
    )
}

// ---------------------------------------------------------------------------
// oscillators and uv utils (`utils/Oscillators.js`, `utils/UVUtils.js`)
// ---------------------------------------------------------------------------

/// `oscTriangle( t )` — `Oscillators.js`:
/// `t.add( 0.5 ).fract().mul( 2 ).sub( 1 ).abs()`. Three defaults `t` to
/// `time`; pass [`super::time`] for that.
pub fn osc_triangle(t: impl Into<NodeRef>) -> NodeRef {
    t.into().add(0.5).fract().mul(2.0).sub(1.0).abs()
}

/// `oscSquare( t )` — `Oscillators.js`: `t.fract().round()`.
pub fn osc_square(t: impl Into<NodeRef>) -> NodeRef {
    unary("round", t.into().fract())
}

/// `oscSawtooth( t )` — `Oscillators.js`: `t.fract()`.
pub fn osc_sawtooth(t: impl Into<NodeRef>) -> NodeRef {
    t.into().fract()
}

/// `rotateUV( uv, rotation, center = vec2( 0.5 ) )` — `UVUtils.js`:
/// `rotate( uv.sub( center ), rotation ).add( center )`.
pub fn rotate_uv(uv: impl Into<NodeRef>, rotation: impl Into<NodeRef>) -> NodeRef {
    rotate_uv_about(uv, rotation, constant(Type::Vec2, vec![0.5, 0.5]))
}

/// [`rotate_uv`] with an explicit `center`.
pub fn rotate_uv_about(
    uv: impl Into<NodeRef>,
    rotation: impl Into<NodeRef>,
    center: impl Into<NodeRef>,
) -> NodeRef {
    let center = center.into();
    rotate(uv.into().sub(center.clone()), rotation).add(center)
}

/// `spherizeUV( uv, strength, center = vec2( 0.5 ) )` — `UVUtils.js`:
/// `delta = uv - center; delta4 = dot( delta, delta )²; uv + delta *
/// delta4 * strength`.
pub fn spherize_uv(uv: impl Into<NodeRef>, strength: impl Into<NodeRef>) -> NodeRef {
    spherize_uv_about(uv, strength, constant(Type::Vec2, vec![0.5, 0.5]))
}

/// [`spherize_uv`] with an explicit `center`.
pub fn spherize_uv_about(
    uv: impl Into<NodeRef>,
    strength: impl Into<NodeRef>,
    center: impl Into<NodeRef>,
) -> NodeRef {
    let uv = uv.into();
    let delta = uv.sub(center);
    let delta2 = delta.dot(delta.clone());
    let delta4 = delta2.mul(delta2.clone());
    let offset = delta4.mul(strength);
    uv.add(delta.mul(offset))
}

/// `remap( node, inLow, inHigh, outLow = 0, outHigh = 1 )` — `Remap.js`:
/// `node.sub( inLow ).div( inHigh.sub( inLow ) ).mul( outHigh.sub( outLow )
/// ).add( outLow )`. Rust has no default arguments; pass `0.0, 1.0` for
/// three's defaults, which is also what three emits for them.
pub fn remap(
    node: impl Into<NodeRef>,
    in_low: impl Into<NodeRef>,
    in_high: impl Into<NodeRef>,
    out_low: impl Into<NodeRef>,
    out_high: impl Into<NodeRef>,
) -> NodeRef {
    remap_impl(
        node.into(),
        in_low.into(),
        in_high.into(),
        out_low.into(),
        out_high.into(),
        false,
    )
}

/// `remapClamp( … )` — `Remap.js`: [`remap`] with `t.clamp()` before the
/// output range is applied.
pub fn remap_clamp(
    node: impl Into<NodeRef>,
    in_low: impl Into<NodeRef>,
    in_high: impl Into<NodeRef>,
    out_low: impl Into<NodeRef>,
    out_high: impl Into<NodeRef>,
) -> NodeRef {
    remap_impl(
        node.into(),
        in_low.into(),
        in_high.into(),
        out_low.into(),
        out_high.into(),
        true,
    )
}

fn remap_impl(
    node: NodeRef,
    in_low: NodeRef,
    in_high: NodeRef,
    out_low: NodeRef,
    out_high: NodeRef,
    clamp: bool,
) -> NodeRef {
    let mut t = node.sub(in_low.clone()).div(in_high.sub(in_low));
    if clamp {
        t = t.clamp(0.0, 1.0);
    }
    t.mul(out_high.sub(out_low.clone())).add(out_low)
}

// ---------------------------------------------------------------------------
// timers (`utils/Timer.js`)
// ---------------------------------------------------------------------------

/// `deltaTime` — `Timer.js`' `uniform( 0 ).setGroup( renderGroup ).onRenderUpdate(
/// ( frame ) => frame.deltaTime )`: seconds since the previous frame.
pub fn delta_time() -> NodeRef {
    thread_local! { static CELL: Lazy<NodeRef> = const { Lazy::new() }; }
    CELL.with(|c| {
        c.get(|| {
            uniform(
                UniformSource::DeltaTime,
                Type::F32,
                UniformGroup::Render,
                None,
            )
        })
    })
}

/// `frameId` — `Timer.js`' `uniform( 0, 'uint' ).setGroup( renderGroup
/// ).onRenderUpdate( ( frame ) => frame.frameId )`.
pub fn frame_id() -> NodeRef {
    thread_local! { static CELL: Lazy<NodeRef> = const { Lazy::new() }; }
    CELL.with(|c| {
        c.get(|| {
            uniform(
                UniformSource::FrameId,
                Type::U32,
                UniformGroup::Render,
                None,
            )
        })
    })
}

// ---------------------------------------------------------------------------
// billboarding (`utils/SpriteUtils.js`)
// ---------------------------------------------------------------------------

/// The options object `billboarding( { … } )` takes. `Default` is three's
/// defaults: `position: null, horizontal: true, vertical: false,
/// horizontalRotation: false`.
#[derive(Clone, Debug)]
pub struct Billboarding {
    pub position: Option<NodeRef>,
    pub horizontal: bool,
    pub vertical: bool,
    pub horizontal_rotation: bool,
}

impl Default for Billboarding {
    fn default() -> Self {
        Self {
            position: None,
            horizontal: true,
            vertical: false,
            horizontal_rotation: false,
        }
    }
}

/// `billboarding( { position, horizontal, vertical, horizontalRotation } )` —
/// `SpriteUtils.js`: a clip-space position for a `vertexNode` that turns a
/// mesh's local axes toward the camera.
///
/// **Divergence, cosmetic** (`docs/nodes.md` §8): three assigns into the
/// `modelViewMatrix` operator node, which its builder turns into an inline
/// `var nodeVarN : mat4x4<f32> = …`; the port asks for the var explicitly,
/// so it is a `var<private>` assigned in the flow. Same value.
pub fn billboarding(options: Billboarding) -> NodeRef {
    let mw = model_world_matrix();
    let center = match options.position {
        Some(p) => p,
        None => position_world().sub(
            mw.mul(join(Type::Vec4, vec![position_geometry(), float(0.0)]))
                .xyz(),
        ),
    };

    let mut statements = Vec::new();
    let world = to_var(None, mw.clone());
    let column = |m: &NodeRef, c: usize, v: &NodeRef, out: &mut Vec<NodeRef>| {
        out.push(m.element(c).element(0).assign(v.x()));
        out.push(m.element(c).element(1).assign(v.y()));
        out.push(m.element(c).element(2).assign(v.z()));
    };
    column(&world, 3, &center, &mut statements);

    let model_view = to_var(None, camera_view_matrix().mul(world.clone()));
    let scale_x = mw.element(0).length();
    let scale_y = mw.element(1).length();
    let scale_z = mw.element(2).length();

    let (right, up, forward);
    if options.horizontal_rotation {
        let look = camera_position().sub(world.element(3).xyz());
        let look_xz = join(Type::Vec3, vec![look.x(), float(0.0), look.z()]).normalize();
        let right_w = join(
            Type::Vec3,
            vec![look_xz.z(), float(0.0), look_xz.x().negate()],
        );
        right = Some(
            camera_view_matrix()
                .mul(join(Type::Vec4, vec![right_w, float(0.0)]))
                .xyz()
                .mul(scale_x),
        );
        up = Some(camera_view_matrix().element(1).xyz().mul(scale_y));
        forward = camera_view_matrix()
            .mul(join(Type::Vec4, vec![look_xz, float(0.0)]))
            .xyz()
            .mul(scale_z);
    } else {
        right = options
            .horizontal
            .then(|| join(Type::Vec3, vec![scale_x, float(0.0), float(0.0)]));
        up = options
            .vertical
            .then(|| join(Type::Vec3, vec![float(0.0), scale_y, float(0.0)]));
        forward = constant(Type::Vec3, vec![0.0, 0.0, 1.0]);
    }
    if let Some(right) = right {
        column(&model_view, 0, &right, &mut statements);
    }
    if let Some(up) = up {
        column(&model_view, 1, &up, &mut statements);
    }
    column(&model_view, 2, &forward, &mut statements);

    let result = camera_projection_matrix()
        .mul(model_view)
        .mul(position_geometry());
    block(statements, result)
}

// ---------------------------------------------------------------------------
// methods
// ---------------------------------------------------------------------------

impl NodeRef {
    /// `x.toFloat()` — `TSLCore.js`' `addMethodChaining( 'toFloat', float )`.
    pub fn to_float(&self) -> NodeRef {
        self.to(Type::vector_of(Type::F32, 1))
    }
    /// `x.toInt()`.
    pub fn to_int(&self) -> NodeRef {
        self.to(Type::I32)
    }
    /// `x.toUint()`.
    pub fn to_uint(&self) -> NodeRef {
        self.to(Type::U32)
    }
    /// `x.toBool()`.
    pub fn to_bool(&self) -> NodeRef {
        self.to(Type::Bool)
    }
    /// `x.toVec2()`.
    pub fn to_vec2(&self) -> NodeRef {
        self.to(Type::Vec2)
    }
    /// `x.toVec3()`.
    pub fn to_vec3(&self) -> NodeRef {
        self.to(Type::Vec3)
    }
    /// `x.toVec4()`.
    pub fn to_vec4(&self) -> NodeRef {
        self.to(Type::Vec4)
    }
    /// `x.toColor()` — a `vec3`.
    pub fn to_color(&self) -> NodeRef {
        self.to(Type::Vec3)
    }
    /// `x.toIVec2()`.
    pub fn to_ivec2(&self) -> NodeRef {
        self.to(Type::IVec2)
    }
    /// `x.toIVec3()`.
    pub fn to_ivec3(&self) -> NodeRef {
        self.to(Type::IVec3)
    }
    /// `x.toIVec4()`.
    pub fn to_ivec4(&self) -> NodeRef {
        self.to(Type::IVec4)
    }
    /// `x.toUVec2()`.
    pub fn to_uvec2(&self) -> NodeRef {
        self.to(Type::UVec2)
    }
    /// `x.toUVec3()`.
    pub fn to_uvec3(&self) -> NodeRef {
        self.to(Type::UVec3)
    }
    /// `x.toUVec4()`.
    pub fn to_uvec4(&self) -> NodeRef {
        self.to(Type::UVec4)
    }
    /// `x.toBVec2()`.
    pub fn to_bvec2(&self) -> NodeRef {
        self.to(Type::BVec2)
    }
    /// `x.toBVec3()`.
    pub fn to_bvec3(&self) -> NodeRef {
        self.to(Type::BVec3)
    }
    /// `x.toBVec4()`.
    pub fn to_bvec4(&self) -> NodeRef {
        self.to(Type::BVec4)
    }
    /// `x.toMat2()`.
    pub fn to_mat2(&self) -> NodeRef {
        self.to(Type::Mat2)
    }
    /// `x.toMat3()` — from a `mat4`, `NodeBuilder.format()`'s column
    /// narrowing.
    pub fn to_mat3(&self) -> NodeRef {
        self.to(Type::Mat3)
    }
    /// `x.toMat4()`.
    pub fn to_mat4(&self) -> NodeRef {
        self.to(Type::Mat4)
    }

    /// `x.atan()` — one-argument `MathNode.ATAN`.
    pub fn atan(&self) -> NodeRef {
        atan(self)
    }
    /// `x.acos()`.
    pub fn acos(&self) -> NodeRef {
        acos(self)
    }
    /// `x.tan()`.
    pub fn tan(&self) -> NodeRef {
        tan(self)
    }
    /// `x.sinh()`.
    pub fn sinh(&self) -> NodeRef {
        sinh(self)
    }
    /// `x.cosh()`.
    pub fn cosh(&self) -> NodeRef {
        cosh(self)
    }
    /// `x.tanh()`.
    pub fn tanh(&self) -> NodeRef {
        tanh(self)
    }
    /// `x.asinh()`.
    pub fn asinh(&self) -> NodeRef {
        asinh(self)
    }
    /// `x.acosh()`.
    pub fn acosh(&self) -> NodeRef {
        acosh(self)
    }
    /// `x.atanh()`.
    pub fn atanh(&self) -> NodeRef {
        atanh(self)
    }
    /// `x.round()`.
    pub fn round(&self) -> NodeRef {
        round(self)
    }
    /// `x.trunc()`.
    pub fn trunc(&self) -> NodeRef {
        trunc(self)
    }
    /// `x.degrees()`.
    pub fn degrees(&self) -> NodeRef {
        degrees(self)
    }
    /// `x.radians()`.
    pub fn radians(&self) -> NodeRef {
        radians(self)
    }
    /// `m.determinant()`.
    pub fn determinant(&self) -> NodeRef {
        determinant(self)
    }
    /// `m.inverse()`.
    pub fn inverse(&self) -> NodeRef {
        inverse(self)
    }
    /// `m.transpose()`.
    pub fn transpose(&self) -> NodeRef {
        transpose(self)
    }
    /// `x.pow2()`.
    pub fn pow2(&self) -> NodeRef {
        pow2(self)
    }
    /// `x.pow4()`.
    pub fn pow4(&self) -> NodeRef {
        pow4(self)
    }
    /// `a.difference( b )`.
    pub fn difference(&self, b: impl Into<NodeRef>) -> NodeRef {
        difference(self, b)
    }
    /// `x.cbrt()`.
    pub fn cbrt(&self) -> NodeRef {
        cbrt(self)
    }
    /// `a.lengthSq()`.
    pub fn length_sq(&self) -> NodeRef {
        length_sq(self)
    }
    /// `v.length()` — `MathNode.LENGTH`.
    pub fn length(&self) -> NodeRef {
        super::length(self)
    }
    /// `n.faceForward( i, nref )`.
    pub fn face_forward(&self, i: impl Into<NodeRef>, nref: impl Into<NodeRef>) -> NodeRef {
        face_forward(self, i, nref)
    }
    /// `a.xor( b )`.
    pub fn xor(&self, b: impl Into<NodeRef>) -> NodeRef {
        xor(self, b)
    }
    /// `a.bitNot()`.
    pub fn bit_not(&self) -> NodeRef {
        bit_not(self)
    }
    /// `x.countOneBits()`.
    pub fn count_one_bits(&self) -> NodeRef {
        count_one_bits(self)
    }
    /// `x.countTrailingZeros()`.
    pub fn count_trailing_zeros(&self) -> NodeRef {
        count_trailing_zeros(self)
    }
    /// `x.countLeadingZeros()`.
    pub fn count_leading_zeros(&self) -> NodeRef {
        count_leading_zeros(self)
    }
    /// `a.mod( b )` — see [`mod_`].
    pub fn mod_(&self, b: impl Into<NodeRef>) -> NodeRef {
        mod_(self, b)
    }
    /// `x.hash()`.
    pub fn hash(&self) -> NodeRef {
        hash(self)
    }
    /// `x.remap( inLow, inHigh, outLow, outHigh )` — `Remap.js`'
    /// `addMethodChaining( 'remap', remap )`.
    pub fn remap(
        &self,
        in_low: impl Into<NodeRef>,
        in_high: impl Into<NodeRef>,
        out_low: impl Into<NodeRef>,
        out_high: impl Into<NodeRef>,
    ) -> NodeRef {
        remap(self, in_low, in_high, out_low, out_high)
    }
    /// `x.remapClamp( inLow, inHigh, outLow, outHigh )`.
    pub fn remap_clamp(
        &self,
        in_low: impl Into<NodeRef>,
        in_high: impl Into<NodeRef>,
        out_low: impl Into<NodeRef>,
        out_high: impl Into<NodeRef>,
    ) -> NodeRef {
        remap_clamp(self, in_low, in_high, out_low, out_high)
    }
    /// `x.smoothstep( low, high )` — `MathNode.js`' `smoothstepElement`:
    /// `smoothstep( low, high, x )`.
    pub fn smoothstep(&self, low: impl Into<NodeRef>, high: impl Into<NodeRef>) -> NodeRef {
        super::smoothstep(low, high, self)
    }
    /// `x.step( edge )` — `MathNode.js`' `stepElement`: `step( edge, x )`.
    pub fn step(&self, edge: impl Into<NodeRef>) -> NodeRef {
        super::step(edge, self)
    }
}

/// `MathNode.getInputType()` for callers elsewhere in `tsl`.
pub(super) fn math_input_type(args: &[&NodeRef]) -> Type {
    input_type(args)
}
