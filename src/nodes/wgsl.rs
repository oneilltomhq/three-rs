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
        Type::BVec3 => "vec3<bool>",
        Type::Mat2 => "mat2x2<f32>",
        Type::Mat3 => "mat3x3<f32>",
        Type::Mat4 => "mat4x4<f32>",
    }
}

/// A number as three.js writes it: JS `toString()`, with `.0` appended when the
/// result would otherwise read as an integer.
pub fn number(v: f64) -> String {
    if v == v.trunc() && v.abs() < 1e21 {
        format!("{:.1}", v)
    } else {
        let s = format!("{}", v);
        if s.contains('.') || s.contains('e') {
            s
        } else {
            format!("{s}.0")
        }
    }
}

/// A literal of `ty` from its components.
pub fn constant(ty: Type, values: &[f64]) -> String {
    match ty {
        Type::F32 => number(values[0]),
        Type::I32 => format!("{}", values[0] as i64),
        Type::U32 => format!("{}u", values[0] as u64),
        Type::Bool => (values[0] != 0.0).to_string(),
        _ => {
            let parts: Vec<String> = values.iter().map(|v| number(*v)).collect();
            format!("{}( {} )", type_name(ty), parts.join(", "))
        }
    }
}

/// The WGSL sample-type of a texture binding, as the bind-group layout needs it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TextureKind {
    Float2D,
    Depth2D,
    Cube,
}

impl TextureKind {
    /// The declared WGSL type of the texture variable.
    pub fn wgsl(self) -> &'static str {
        match self {
            TextureKind::Float2D => "texture_2d<f32>",
            TextureKind::Depth2D => "texture_depth_2d",
            TextureKind::Cube => "texture_cube<f32>",
        }
    }

    /// A depth texture is not filterable, so three.js emits no sampler for it
    /// and reads it with `textureLoad`.
    pub fn has_sampler(self) -> bool {
        self != TextureKind::Depth2D
    }
}

/// `WGSLNodeBuilder.generateWrapFunction()` for the default
/// `ClampToEdgeWrapping` on both axes, plus the helper it calls. Three emits
/// these as code snippets too — they are not built out of nodes.
pub const CLAMP_WRAP_SNIPPET: &str = "fn tsl_clampWrapping_float( coord: f32 ) -> f32 { return clamp( coord, 0.0, 1.0 ); }\nfn tsl_coord_clampS_clampT_2d( coord : vec2f ) -> vec2f {\n\n\treturn vec2f(\n\t\ttsl_clampWrapping_float( coord.x ),\n\t\ttsl_clampWrapping_float( coord.y )\n\t);\n\n}\n";

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

pub fn texture_dimensions(texture: &str) -> String {
    format!("textureDimensions( {texture}, u32( 0 ) )")
}

/// WGSL uniform-buffer layout rules: alignment of a member of this type.
pub fn align_of(ty: Type) -> u32 {
    match ty {
        Type::F32 | Type::I32 | Type::U32 | Type::Bool => 4,
        Type::Vec2 | Type::UVec2 | Type::Mat2 => 8,
        _ => 16,
    }
}

/// Size of a member of this type inside a uniform struct. A `mat3x3<f32>` is
/// three 16-byte columns.
pub fn size_of(ty: Type) -> u32 {
    match ty {
        Type::F32 | Type::I32 | Type::U32 | Type::Bool => 4,
        Type::Vec2 | Type::UVec2 => 8,
        Type::Vec3 | Type::BVec3 => 12,
        Type::Vec4 => 16,
        // Two 8-byte columns; `align_of` is 8 for a `mat2x2<f32>`, like `vec2`.
        Type::Mat2 => 16,
        Type::Mat3 => 48,
        Type::Mat4 => 64,
        Type::Void => 0,
    }
}
