//! Port of `three.js/src/renderers/webgpu/nodes/WGSLNodeBuilder.js` — the
//! backend half of the builder: type spellings, constant formatting, the
//! sampling snippets and the fixed shader skeleton.
//!
//! A module boundary rather than a trait: there is exactly one backend, and
//! pretending otherwise would buy nothing but indirection.

use super::node::Type;

/// `WGSLNodeBuilder.getType()`.
pub fn type_name(ty: Type) -> &'static str {
    match ty {
        Type::Void => "void",
        Type::Bool => "bool",
        Type::F32 => "f32",
        Type::I32 => "i32",
        Type::U32 => "u32",
        Type::Vec2 => "vec2<f32>",
        Type::Vec3 => "vec3<f32>",
        Type::Vec4 => "vec4<f32>",
        Type::UVec2 => "vec2<u32>",
        Type::IVec2 => "vec2<i32>",
        Type::UVec3 => "vec3<u32>",
        Type::UVec4 => "vec4<u32>",
        Type::IVec3 => "vec3<i32>",
        Type::IVec4 => "vec4<i32>",
        Type::BVec2 => "vec2<bool>",
        Type::BVec3 => "vec3<bool>",
        Type::BVec4 => "vec4<bool>",
        Type::Mat2 => "mat2x2<f32>",
        Type::Mat3 => "mat3x3<f32>",
        Type::Mat4 => "mat4x4<f32>",
    }
}

/// A number as three.js writes it — `TSLBase.js`' `toFloat( value )`:
/// `Number.isInteger( value ) ? value + '.0' : String( value )`.
pub fn number(v: f64) -> String {
    if v == v.trunc() && v.abs() < 1e21 {
        format!("{:.1}", v)
    } else {
        js_to_string(v)
    }
}

/// JS `Number.prototype.toString()` for a non-integral finite double, i.e.
/// ECMA-262 §6.1.6.1.20 steps 5-12 over the shortest round-tripping decimal.
///
/// Rust's own `{}` is the same shortest representation but always in positional
/// notation, so `1e-8` prints as `0.00000001` where JS — and therefore three.js'
/// dumped WGSL — writes `1e-8`. `webgpu_compute_points` is the first rung with
/// a literal small enough to tell them apart (`1e-8`, `1e-7`); everything the
/// earlier rungs emit is in the positional range and is unchanged.
fn js_to_string(v: f64) -> String {
    // `{:e}` is the shortest round-tripping mantissa with one digit before the
    // point, plus the base-10 exponent — exactly the `s` and `n - 1` of the
    // spec.
    let sci = format!("{:e}", v);
    let (mantissa, exponent) = sci
        .split_once('e')
        .expect("three-rs: `{:e}` always writes an exponent");
    let exponent: i32 = exponent
        .parse()
        .expect("three-rs: `{:e}`'s exponent is an integer");

    let negative = mantissa.starts_with('-');
    let digits: String = mantissa.chars().filter(|c| c.is_ascii_digit()).collect();
    let k = digits.len() as i32;
    // `s * 10^( n - k ) = |v|`, with `s` the digit string.
    let n = exponent + 1;
    let sign = if negative { "-" } else { "" };

    // Steps 6-8: positional notation for `-6 < n <= 21`.
    if n > -6 && n <= 21 {
        if k <= n {
            return format!("{sign}{digits}{}", "0".repeat((n - k) as usize));
        }
        if n > 0 {
            let at = n as usize;
            return format!("{sign}{}.{}", &digits[..at], &digits[at..]);
        }
        return format!("{sign}0.{}{digits}", "0".repeat((-n) as usize));
    }

    // Steps 9-12: exponential notation, `e+`/`e-` and the exponent `n - 1`.
    let exponent_sign = if n > 0 { "+" } else { "-" };
    let exponent = (n - 1).abs();
    if k == 1 {
        format!("{sign}{digits}e{exponent_sign}{exponent}")
    } else {
        format!(
            "{sign}{}.{}e{exponent_sign}{exponent}",
            &digits[..1],
            &digits[1..]
        )
    }
}

/// `NodeBuilder.format( snippet, fromType, toType )` — the *widening* half.
///
/// three.js' ladder (`src/nodes/core/NodeBuilder.js:2726-2790`) also narrows
/// (`v.xyz`), changes component type in place (`f32( i )`) and cuts a `mat4`
/// down to a `mat3`. Those arms are deliberately not here: this port reaches
/// `format()` from places three.js does not — `Node::Op` widens both operands
/// to the result type, so a `bool` comparison would come out as
/// `f32( a <= b )` — and every narrowing the port needs is written explicitly
/// by the caller (`.xyz()`, `to_vec3()`, `Type::Mat3` joins). Implementing them
/// here changes the WGSL of six already-green rungs for no gain; see
/// `docs/nodes.md` §8.
///
/// What is here is the widening three.js does and the port used to splat: a
/// `vec2` promoted to a `vec3` is `vec3<f32>( v, 0.0 )`, a `vec3` promoted to a
/// `vec4` is `vec4<f32>( v, 1.0 )`, and a scalar splats.
pub fn convert(snippet: &str, from: Type, to: Type) -> String {
    if from == to || to == Type::Void || from.is_matrix() || to.is_matrix() {
        return snippet.to_string();
    }

    let (from_len, to_len) = (from.components(), to.components());
    if to_len == 0 {
        return snippet.to_string();
    }

    // `NodeBuilder.format()`'s narrowing arm: a value wider than the slot it
    // is being written into is *swizzled* down, not left alone. That is where
    // `( materialEnvRotation * vec4( dir, 1.0 ) ).xyz` gets its `.xyz` from —
    // the product is a `vec4` and `getFace( direction : vec3<f32> )` wants
    // three components — and where `color.addAssign( bilinearCubeUV( … ) )`
    // gets its, a `vec3` accumulator taking a `vec4` sample.
    if to_len < from_len {
        let swizzled = if to_len == 1 {
            format!("{snippet}.x")
        } else {
            format!("{snippet}.{}", &"xyz"[..to_len])
        };
        let narrowed = Type::vector_of(from.component_type(), to_len);
        return convert(&swizzled, narrowed, to);
    }

    if to_len == from_len {
        // Same width, different component type: `NodeBuilder.format()`'s
        // `${ getType( toType ) }( ${ snippet } )`. A same-type pair returned
        // at the top, so this is only ever a real conversion.
        if from.component_type() != to.component_type() {
            return format!("{}( {snippet} )", type_name(to));
        }
        return snippet.to_string();
    }

    // The padding component is `generateConst( componentType, … )` of the
    // *target* type, and the recursion converts the source to the target's
    // component type first: `uvec2 → vec3` is `vec3<f32>( vec2<f32>( v ), 0.0 )`.
    let component = to.component_type();
    if to_len == 4 && from_len > 1 {
        let widened = Type::vector_of(component, 3);
        let inner = convert(snippet, from, widened);
        let one = constant(component, &[1.0]);
        return format!("{}( {inner}, {one} )", type_name(to));
    }

    if from_len == 2 {
        // `to_len == 3`.
        let inner = convert(snippet, from, Type::vector_of(component, 2));
        let zero = constant(component, &[0.0]);
        return format!("{}( {inner}, {zero} )", type_name(to));
    }

    // A scalar splats — through its target component type when that differs:
    // `vec3( 1u )` is `vec3<f32>( f32( 1u ) )`.
    if from != component {
        return format!("{}( {}( {snippet} ) )", type_name(to), type_name(component));
    }
    format!("{}( {snippet} )", type_name(to))
}

/// A literal of `ty` from its components.
pub fn constant(ty: Type, values: &[f64]) -> String {
    match ty {
        Type::F32 => number(values[0]),
        Type::I32 => format!("{}", js_round(values[0]) as i64),
        // `value >= 0 ? `${ Math.round( value ) }u` : '0u'`.
        Type::U32 => format!("{}u", js_round(values[0].max(0.0)) as u64),
        Type::Bool => (values[0] != 0.0).to_string(),
        _ => {
            // `NodeBuilder.generateConst()` recurses per component with the
            // component type: `vec3<u32>( 1u, 2u, 3u )`, `vec2<bool>( true,
            // false )`. A matrix's elements are floats.
            let component = ty.component_type();
            let parts: Vec<String> = values
                .iter()
                .map(|v| match component {
                    Type::F32 => number(*v),
                    other => constant(other, &[*v]),
                })
                .collect();
            format!("{}( {} )", type_name(ty), parts.join(", "))
        }
    }
}

/// JS `Math.round()`: halves round towards +∞ (`Math.round( -2.5 )` is -2).
fn js_round(v: f64) -> f64 {
    (v + 0.5).floor()
}

/// The WGSL sample-type of a texture binding, as the bind-group layout needs it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TextureKind {
    Float2D,
    /// `texture_2d_array<f32>` — the morph data texture, read with
    /// `textureLoad` only, so it needs no sampler.
    Float2DArray,
    /// `texture_2d_array<f32>` with a filtering sampler — a
    /// `CompressedArrayTexture` (or any [`Texture`](crate::textures::Texture)
    /// with array layers) read with `textureSample( …, layer )`.
    Sampled2DArray,
    /// `texture_2d<f32>` for a `DataTexture`: `rgba32float`, `NearestFilter`,
    /// read with `textureLoad` only — no sampler, non-filterable.
    FloatData2D,
    /// `texture_2d<u32>` — a `RedIntegerFormat` / `UnsignedIntType`
    /// `DataTexture`, `BatchedMesh._indirectTexture`.
    Uint2D,
    Depth2D,
    /// `texture_depth_multisampled_2d` — the depth attachment of an MSAA
    /// render target, which WebGPU never resolves. `textureLoad` takes the
    /// sample index, and three passes `0`, so the composite reads the first
    /// sample of each fragment. `webgpu_custom_fog_background` is the page
    /// this exists for (`docs/nodes.md` §24).
    DepthMultisampled2D,
    /// A depth texture bound for `textureSampleCompare`: the same
    /// `texture_depth_2d`, but with a `sampler_comparison` beside it.
    DepthCompare2D,
    Cube,
    DepthCube,
}

impl TextureKind {
    /// The declared WGSL type of the texture variable.
    pub fn wgsl(self) -> &'static str {
        match self {
            TextureKind::Float2D => "texture_2d<f32>",
            TextureKind::Float2DArray | TextureKind::Sampled2DArray => "texture_2d_array<f32>",
            TextureKind::FloatData2D => "texture_2d<f32>",
            TextureKind::Uint2D => "texture_2d<u32>",
            TextureKind::Depth2D | TextureKind::DepthCompare2D => "texture_depth_2d",
            TextureKind::DepthMultisampled2D => "texture_depth_multisampled_2d",
            TextureKind::Cube => "texture_cube<f32>",
            TextureKind::DepthCube => "texture_depth_cube",
        }
    }

    /// The declared WGSL type of the sampler beside the texture.
    pub fn sampler_wgsl(self) -> &'static str {
        match self {
            TextureKind::DepthCompare2D | TextureKind::DepthCube => "sampler_comparison",
            _ => "sampler",
        }
    }

    /// A depth texture is not filterable, so three.js emits no sampler for it
    /// and reads it with `textureLoad`.
    pub fn has_sampler(self) -> bool {
        !matches!(
            self,
            TextureKind::Depth2D
                | TextureKind::DepthMultisampled2D
                | TextureKind::Float2DArray
                | TextureKind::FloatData2D
                | TextureKind::Uint2D
        )
    }

    /// A shadow map is read through a comparison sampler.
    pub fn is_comparison(self) -> bool {
        matches!(self, TextureKind::DepthCompare2D | TextureKind::DepthCube)
    }
}

/// `WGSLNodeBuilder.generateWrapFunction()` for the default
/// `ClampToEdgeWrapping` on both axes, plus the helper it calls. Three emits
/// these as code snippets too — they are not built out of nodes.
pub const CLAMP_WRAP_SNIPPET: &str = "fn tsl_clampWrapping_float( coord: f32 ) -> f32 { return clamp( coord, 0.0, 1.0 ); }\nfn tsl_coord_clampS_clampT_2d( coord : vec2f ) -> vec2f {\n\n\treturn vec2f(\n\t\ttsl_clampWrapping_float( coord.x ),\n\t\ttsl_clampWrapping_float( coord.y )\n\t);\n\n}\n";

/// `WGSLNodeBuilder`'s `wgslPolyfill` entry for a `tsl_*` method name, if
/// it has one: the helper a `MathNode` / `OperatorNode` lowering calls.
pub fn polyfill(name: &str) -> Option<&'static str> {
    Some(match name {
        "tsl_mod_float" => MOD_FLOAT_SNIPPET,
        "tsl_mod_vec2" => MOD_VEC2_SNIPPET,
        "tsl_mod_vec3" => MOD_VEC3_SNIPPET,
        "tsl_mod_vec4" => MOD_VEC4_SNIPPET,
        "tsl_inverse_mat2" => INVERSE_MAT2_SNIPPET,
        "tsl_inverse_mat3" => INVERSE_MAT3_SNIPPET,
        "tsl_inverse_mat4" => INVERSE_MAT4_SNIPPET,
        "tsl_xor" => XOR_SNIPPET,
        _ => return None,
    })
}

/// `wgslPolyfill.mod_vec2` / `mod_vec3` / `mod_vec4` — `OperatorNode`'s `%`
/// on a float vector (`getOperatorMethod()` → `mod_<type>`).
pub const MOD_VEC2_SNIPPET: &str =
    "fn tsl_mod_vec2( x : vec2f, y : vec2f ) -> vec2f { return x - y * floor( x / y ); }\n";
pub const MOD_VEC3_SNIPPET: &str =
    "fn tsl_mod_vec3( x : vec3f, y : vec3f ) -> vec3f { return x - y * floor( x / y ); }\n";
pub const MOD_VEC4_SNIPPET: &str =
    "fn tsl_mod_vec4( x : vec4f, y : vec4f ) -> vec4f { return x - y * floor( x / y ); }\n";

/// `wgslPolyfill.tsl_xor` — `OperatorNode`'s `'^^'`.
pub const XOR_SNIPPET: &str =
    "fn tsl_xor( a : bool, b : bool ) -> bool { return ( a || b ) && !( a && b ); }\n";

/// `wgslPolyfill.inverse_mat2`.
pub const INVERSE_MAT2_SNIPPET: &str = "fn tsl_inverse_mat2( m : mat2x2<f32> ) -> mat2x2<f32> {\n\n\tlet det = m[ 0 ][ 0 ] * m[ 1 ][ 1 ] - m[ 0 ][ 1 ] * m[ 1 ][ 0 ];\n\n\treturn mat2x2<f32>(\n\t\tm[ 1 ][ 1 ], - m[ 0 ][ 1 ],\n\t\t- m[ 1 ][ 0 ], m[ 0 ][ 0 ]\n\t) * ( 1.0 / det );\n\n}\n";

/// `wgslPolyfill.inverse_mat4`.
pub const INVERSE_MAT4_SNIPPET: &str = "fn tsl_inverse_mat4( m : mat4x4<f32> ) -> mat4x4<f32> {\n\n\tlet a00 = m[ 0 ][ 0 ]; let a01 = m[ 0 ][ 1 ]; let a02 = m[ 0 ][ 2 ]; let a03 = m[ 0 ][ 3 ];\n\tlet a10 = m[ 1 ][ 0 ]; let a11 = m[ 1 ][ 1 ]; let a12 = m[ 1 ][ 2 ]; let a13 = m[ 1 ][ 3 ];\n\tlet a20 = m[ 2 ][ 0 ]; let a21 = m[ 2 ][ 1 ]; let a22 = m[ 2 ][ 2 ]; let a23 = m[ 2 ][ 3 ];\n\tlet a30 = m[ 3 ][ 0 ]; let a31 = m[ 3 ][ 1 ]; let a32 = m[ 3 ][ 2 ]; let a33 = m[ 3 ][ 3 ];\n\n\tlet b00 = a00 * a11 - a01 * a10;\n\tlet b01 = a00 * a12 - a02 * a10;\n\tlet b02 = a00 * a13 - a03 * a10;\n\tlet b03 = a01 * a12 - a02 * a11;\n\tlet b04 = a01 * a13 - a03 * a11;\n\tlet b05 = a02 * a13 - a03 * a12;\n\tlet b06 = a20 * a31 - a21 * a30;\n\tlet b07 = a20 * a32 - a22 * a30;\n\tlet b08 = a20 * a33 - a23 * a30;\n\tlet b09 = a21 * a32 - a22 * a31;\n\tlet b10 = a21 * a33 - a23 * a31;\n\tlet b11 = a22 * a33 - a23 * a32;\n\n\tlet det = b00 * b11 - b01 * b10 + b02 * b09 + b03 * b08 - b04 * b07 + b05 * b06;\n\n\treturn mat4x4<f32>(\n\t\ta11 * b11 - a12 * b10 + a13 * b09,\n\t\ta02 * b10 - a01 * b11 - a03 * b09,\n\t\ta31 * b05 - a32 * b04 + a33 * b03,\n\t\ta22 * b04 - a21 * b05 - a23 * b03,\n\t\ta12 * b08 - a10 * b11 - a13 * b07,\n\t\ta00 * b11 - a02 * b08 + a03 * b07,\n\t\ta32 * b02 - a30 * b05 - a33 * b01,\n\t\ta20 * b05 - a22 * b02 + a23 * b01,\n\t\ta10 * b10 - a11 * b08 + a13 * b06,\n\t\ta01 * b08 - a00 * b10 - a03 * b06,\n\t\ta30 * b04 - a31 * b02 + a33 * b00,\n\t\ta21 * b02 - a20 * b04 - a23 * b00,\n\t\ta11 * b07 - a10 * b09 - a12 * b06,\n\t\ta00 * b09 - a01 * b07 + a02 * b06,\n\t\ta31 * b01 - a30 * b03 - a32 * b00,\n\t\ta20 * b03 - a21 * b01 + a22 * b00\n\t) * ( 1.0 / det );\n\n}\n";

/// `WGSLNodeBuilder`'s `inverse_mat3` polyfill.
/// `MathNode`'s float `mod` lowers to a helper in WGSL, exactly as the dumps of
/// `webgpu_lights_phong` show (`checker()` is the only rung-5 user).
pub const MOD_FLOAT_SNIPPET: &str =
    "fn tsl_mod_float( x : f32, y : f32 ) -> f32 { return x - y * floor( x / y ); }\n";

pub const INVERSE_MAT3_SNIPPET: &str = "fn tsl_inverse_mat3( m : mat3x3<f32> ) -> mat3x3<f32> {\n\n\tlet a00 = m[ 0 ][ 0 ]; let a01 = m[ 0 ][ 1 ]; let a02 = m[ 0 ][ 2 ];\n\tlet a10 = m[ 1 ][ 0 ]; let a11 = m[ 1 ][ 1 ]; let a12 = m[ 1 ][ 2 ];\n\tlet a20 = m[ 2 ][ 0 ]; let a21 = m[ 2 ][ 1 ]; let a22 = m[ 2 ][ 2 ];\n\n\tlet b01 = a22 * a11 - a12 * a21;\n\tlet b11 = - a22 * a10 + a12 * a20;\n\tlet b21 = a21 * a10 - a11 * a20;\n\n\tlet det = a00 * b01 + a01 * b11 + a02 * b21;\n\n\treturn mat3x3<f32>(\n\t\tb01, ( - a22 * a01 + a02 * a21 ), ( a12 * a01 - a02 * a11 ),\n\t\tb11, ( a22 * a00 - a02 * a20 ), ( - a12 * a00 + a02 * a10 ),\n\t\tb21, ( - a21 * a00 + a01 * a20 ), ( a11 * a00 - a01 * a10 )\n\t) * ( 1.0 / det );\n\n}\n";

/// `WGSLNodeBuilder.generateTextureLoad()` for the non-filterable path: the
/// texel coordinate is the clamped UV scaled by `textureDimensions`.
pub fn texture_load(texture: &str, uv: &str, dims: &str) -> String {
    format!(
        "textureLoad( {texture}, vec2<u32>( clamp( floor( tsl_coord_clampS_clampT_2d( {uv} ) * \
         vec2<f32>( {dims} ) ), vec2<f32>( 0 ), vec2<f32>( {dims} - vec2<u32>( 1, 1 ) ) ) ), \
         u32( 0 ) )"
    )
}

/// `WGSLNodeBuilder.generateTextureLoad()` with a `depthSnippet`: the array
/// layer is a separate argument and the level defaults to the string `'0u'`,
/// which the template then wraps in `u32( … )`.
pub fn texture_load_layer(texture: &str, coord: &str, layer: &str) -> String {
    format!("textureLoad( {texture}, {coord}, {layer}, u32( 0u ) )")
}

/// `WGSLNodeBuilder.generateTextureDimension()`. A multisampled texture has
/// exactly one level and WGSL gives `textureDimensions` no level overload for
/// one, so three (and this) drop the argument there.
pub fn texture_dimensions(texture: &str, kind: TextureKind) -> String {
    if kind == TextureKind::DepthMultisampled2D {
        format!("textureDimensions( {texture} )")
    } else {
        format!("textureDimensions( {texture}, u32( 0 ) )")
    }
}

/// `WGSLNodeBuilder.generateTextureLoad()` with an explicit texel coordinate
/// and no array layer — `textureLoad( t, ivec2( x, y ) )` in `Batch.js`. The
/// level argument goes through the same `u32( … )` template as the layered
/// form, so it reads `u32( 0u )`.
pub fn texture_load_texel(texture: &str, coord: &str) -> String {
    format!("textureLoad( {texture}, {coord}, u32( 0u ) )")
}

/// `TextureSizeNode` — `textureDimensions( t, levelNode )` with the level a
/// plain int literal, not the `u32( … )`-wrapped default `texture_load` uses.
pub fn texture_size(texture: &str, level: &str) -> String {
    format!("textureDimensions( {texture}, {level} )")
}

/// WGSL uniform-buffer layout rules: alignment of a member of this type.
pub fn align_of(ty: Type) -> u32 {
    match ty {
        Type::F32 | Type::I32 | Type::U32 | Type::Bool => 4,
        Type::Vec2 | Type::UVec2 | Type::IVec2 | Type::BVec2 | Type::Mat2 => 8,
        _ => 16,
    }
}

/// Size of a member of this type inside a uniform struct. A `mat3x3<f32>` is
/// three 16-byte columns.
pub fn size_of(ty: Type) -> u32 {
    match ty {
        Type::F32 | Type::I32 | Type::U32 | Type::Bool => 4,
        Type::Vec2 | Type::UVec2 | Type::IVec2 | Type::BVec2 => 8,
        Type::Vec3 | Type::UVec3 | Type::IVec3 | Type::BVec3 => 12,
        Type::Vec4 | Type::UVec4 | Type::IVec4 | Type::BVec4 => 16,
        // Two 8-byte columns; `align_of` is 8 for a `mat2x2<f32>`, like `vec2`.
        Type::Mat2 => 16,
        Type::Mat3 => 48,
        Type::Mat4 => 64,
        Type::Void => 0,
    }
}
