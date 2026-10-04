//! `ArrayKind::vertex_format` row by row: every element kind × item size 1..4
//! × `normalized` × interleaved.
//!
//! The expected column is derived from three r187 by hand, not from the port:
//! `WebGPUAttributeUtils.createAttribute()` first widens a non-normalized,
//! non-interleaved `Int8`/`Int16` array to `Int32Array` and `Uint8`/`Uint16`
//! to `Uint32Array`; `_getVertexFormat()` then maps item size 1 through
//! `typeArraysToVertexFormatPrefixForItemSize1` (Int32, Uint32, Float32 only)
//! and larger item sizes to `${prefix}x${paddedItemSize}`, with the prefix
//! from `typedArraysToVertexFormatPrefix` indexed by `normalized` and the item
//! size padded to a 4-byte stride. A missing entry is three's "Vertex format
//! not supported yet" — a panic here. `Uint8ClampedArray` is in neither map.
//!
//! `F16` is `Float16BufferAttribute`, whose prefix list is `[ 'float16' ]`.
//! three also widens its `Uint16Array` to `Uint32Array` before the lookup
//! (so a three `float16x3` request comes out as an invalid `float16x3`); the
//! port keeps the halves, which is the format three means.

use three_rs::core::ArrayKind;
use wgpu::VertexFormat;

macro_rules! rows {
    ($($name:ident: $kind:ident, $n:expr, $norm:expr, $inter:expr => $expect:tt;)*) => {
        $( rows!(@row $name, $kind, $n, $norm, $inter, $expect); )*
    };
    (@row $name:ident, $kind:ident, $n:expr, $norm:expr, $inter:expr, panic) => {
        #[test]
        #[should_panic(expected = "Vertex format not supported yet")]
        fn $name() {
            ArrayKind::$kind.vertex_format($n, $norm, $inter);
        }
    };
    (@row $name:ident, $kind:ident, $n:expr, $norm:expr, $inter:expr, ($format:ident, $padded:expr)) => {
        #[test]
        fn $name() {
            assert_eq!(
                ArrayKind::$kind.vertex_format($n, $norm, $inter),
                (VertexFormat::$format, $padded)
            );
        }
    };
}

rows! {
    f32_1_raw_own: F32, 1, false, false => (Float32, 1);
    f32_1_raw_inter: F32, 1, false, true => (Float32, 1);
    f32_1_norm_own: F32, 1, true, false => (Float32, 1);
    f32_1_norm_inter: F32, 1, true, true => (Float32, 1);
    f32_2_raw_own: F32, 2, false, false => (Float32x2, 2);
    f32_2_raw_inter: F32, 2, false, true => (Float32x2, 2);
    f32_2_norm_own: F32, 2, true, false => panic;
    f32_2_norm_inter: F32, 2, true, true => panic;
    f32_3_raw_own: F32, 3, false, false => (Float32x3, 3);
    f32_3_raw_inter: F32, 3, false, true => (Float32x3, 3);
    f32_3_norm_own: F32, 3, true, false => panic;
    f32_3_norm_inter: F32, 3, true, true => panic;
    f32_4_raw_own: F32, 4, false, false => (Float32x4, 4);
    f32_4_raw_inter: F32, 4, false, true => (Float32x4, 4);
    f32_4_norm_own: F32, 4, true, false => panic;
    f32_4_norm_inter: F32, 4, true, true => panic;
    f16_1_raw_own: F16, 1, false, false => panic;
    f16_1_raw_inter: F16, 1, false, true => panic;
    f16_1_norm_own: F16, 1, true, false => panic;
    f16_1_norm_inter: F16, 1, true, true => panic;
    f16_2_raw_own: F16, 2, false, false => (Float16x2, 2);
    f16_2_raw_inter: F16, 2, false, true => (Float16x2, 2);
    f16_2_norm_own: F16, 2, true, false => panic;
    f16_2_norm_inter: F16, 2, true, true => panic;
    f16_3_raw_own: F16, 3, false, false => (Float16x4, 4);
    f16_3_raw_inter: F16, 3, false, true => (Float16x4, 4);
    f16_3_norm_own: F16, 3, true, false => panic;
    f16_3_norm_inter: F16, 3, true, true => panic;
    f16_4_raw_own: F16, 4, false, false => (Float16x4, 4);
    f16_4_raw_inter: F16, 4, false, true => (Float16x4, 4);
    f16_4_norm_own: F16, 4, true, false => panic;
    f16_4_norm_inter: F16, 4, true, true => panic;
    i8_1_raw_own: I8, 1, false, false => (Sint32, 1);
    i8_1_raw_inter: I8, 1, false, true => panic;
    i8_1_norm_own: I8, 1, true, false => panic;
    i8_1_norm_inter: I8, 1, true, true => panic;
    i8_2_raw_own: I8, 2, false, false => (Sint32x2, 2);
    i8_2_raw_inter: I8, 2, false, true => (Sint8x4, 4);
    i8_2_norm_own: I8, 2, true, false => (Snorm8x4, 4);
    i8_2_norm_inter: I8, 2, true, true => (Snorm8x4, 4);
    i8_3_raw_own: I8, 3, false, false => (Sint32x3, 3);
    i8_3_raw_inter: I8, 3, false, true => (Sint8x4, 4);
    i8_3_norm_own: I8, 3, true, false => (Snorm8x4, 4);
    i8_3_norm_inter: I8, 3, true, true => (Snorm8x4, 4);
    i8_4_raw_own: I8, 4, false, false => (Sint32x4, 4);
    i8_4_raw_inter: I8, 4, false, true => (Sint8x4, 4);
    i8_4_norm_own: I8, 4, true, false => (Snorm8x4, 4);
    i8_4_norm_inter: I8, 4, true, true => (Snorm8x4, 4);
    u8_1_raw_own: U8, 1, false, false => (Uint32, 1);
    u8_1_raw_inter: U8, 1, false, true => panic;
    u8_1_norm_own: U8, 1, true, false => panic;
    u8_1_norm_inter: U8, 1, true, true => panic;
    u8_2_raw_own: U8, 2, false, false => (Uint32x2, 2);
    u8_2_raw_inter: U8, 2, false, true => (Uint8x4, 4);
    u8_2_norm_own: U8, 2, true, false => (Unorm8x4, 4);
    u8_2_norm_inter: U8, 2, true, true => (Unorm8x4, 4);
    u8_3_raw_own: U8, 3, false, false => (Uint32x3, 3);
    u8_3_raw_inter: U8, 3, false, true => (Uint8x4, 4);
    u8_3_norm_own: U8, 3, true, false => (Unorm8x4, 4);
    u8_3_norm_inter: U8, 3, true, true => (Unorm8x4, 4);
    u8_4_raw_own: U8, 4, false, false => (Uint32x4, 4);
    u8_4_raw_inter: U8, 4, false, true => (Uint8x4, 4);
    u8_4_norm_own: U8, 4, true, false => (Unorm8x4, 4);
    u8_4_norm_inter: U8, 4, true, true => (Unorm8x4, 4);
    u8clamped_1_raw_own: U8Clamped, 1, false, false => panic;
    u8clamped_1_raw_inter: U8Clamped, 1, false, true => panic;
    u8clamped_1_norm_own: U8Clamped, 1, true, false => panic;
    u8clamped_1_norm_inter: U8Clamped, 1, true, true => panic;
    u8clamped_2_raw_own: U8Clamped, 2, false, false => panic;
    u8clamped_2_raw_inter: U8Clamped, 2, false, true => panic;
    u8clamped_2_norm_own: U8Clamped, 2, true, false => panic;
    u8clamped_2_norm_inter: U8Clamped, 2, true, true => panic;
    u8clamped_3_raw_own: U8Clamped, 3, false, false => panic;
    u8clamped_3_raw_inter: U8Clamped, 3, false, true => panic;
    u8clamped_3_norm_own: U8Clamped, 3, true, false => panic;
    u8clamped_3_norm_inter: U8Clamped, 3, true, true => panic;
    u8clamped_4_raw_own: U8Clamped, 4, false, false => panic;
    u8clamped_4_raw_inter: U8Clamped, 4, false, true => panic;
    u8clamped_4_norm_own: U8Clamped, 4, true, false => panic;
    u8clamped_4_norm_inter: U8Clamped, 4, true, true => panic;
    i16_1_raw_own: I16, 1, false, false => (Sint32, 1);
    i16_1_raw_inter: I16, 1, false, true => panic;
    i16_1_norm_own: I16, 1, true, false => panic;
    i16_1_norm_inter: I16, 1, true, true => panic;
    i16_2_raw_own: I16, 2, false, false => (Sint32x2, 2);
    i16_2_raw_inter: I16, 2, false, true => (Sint16x2, 2);
    i16_2_norm_own: I16, 2, true, false => (Snorm16x2, 2);
    i16_2_norm_inter: I16, 2, true, true => (Snorm16x2, 2);
    i16_3_raw_own: I16, 3, false, false => (Sint32x3, 3);
    i16_3_raw_inter: I16, 3, false, true => (Sint16x4, 4);
    i16_3_norm_own: I16, 3, true, false => (Snorm16x4, 4);
    i16_3_norm_inter: I16, 3, true, true => (Snorm16x4, 4);
    i16_4_raw_own: I16, 4, false, false => (Sint32x4, 4);
    i16_4_raw_inter: I16, 4, false, true => (Sint16x4, 4);
    i16_4_norm_own: I16, 4, true, false => (Snorm16x4, 4);
    i16_4_norm_inter: I16, 4, true, true => (Snorm16x4, 4);
    u16_1_raw_own: U16, 1, false, false => (Uint32, 1);
    u16_1_raw_inter: U16, 1, false, true => panic;
    u16_1_norm_own: U16, 1, true, false => panic;
    u16_1_norm_inter: U16, 1, true, true => panic;
    u16_2_raw_own: U16, 2, false, false => (Uint32x2, 2);
    u16_2_raw_inter: U16, 2, false, true => (Uint16x2, 2);
    u16_2_norm_own: U16, 2, true, false => (Unorm16x2, 2);
    u16_2_norm_inter: U16, 2, true, true => (Unorm16x2, 2);
    u16_3_raw_own: U16, 3, false, false => (Uint32x3, 3);
    u16_3_raw_inter: U16, 3, false, true => (Uint16x4, 4);
    u16_3_norm_own: U16, 3, true, false => (Unorm16x4, 4);
    u16_3_norm_inter: U16, 3, true, true => (Unorm16x4, 4);
    u16_4_raw_own: U16, 4, false, false => (Uint32x4, 4);
    u16_4_raw_inter: U16, 4, false, true => (Uint16x4, 4);
    u16_4_norm_own: U16, 4, true, false => (Unorm16x4, 4);
    u16_4_norm_inter: U16, 4, true, true => (Unorm16x4, 4);
    i32_1_raw_own: I32, 1, false, false => (Sint32, 1);
    i32_1_raw_inter: I32, 1, false, true => (Sint32, 1);
    i32_1_norm_own: I32, 1, true, false => (Sint32, 1);
    i32_1_norm_inter: I32, 1, true, true => (Sint32, 1);
    i32_2_raw_own: I32, 2, false, false => (Sint32x2, 2);
    i32_2_raw_inter: I32, 2, false, true => (Sint32x2, 2);
    i32_2_norm_own: I32, 2, true, false => panic;
    i32_2_norm_inter: I32, 2, true, true => panic;
    i32_3_raw_own: I32, 3, false, false => (Sint32x3, 3);
    i32_3_raw_inter: I32, 3, false, true => (Sint32x3, 3);
    i32_3_norm_own: I32, 3, true, false => panic;
    i32_3_norm_inter: I32, 3, true, true => panic;
    i32_4_raw_own: I32, 4, false, false => (Sint32x4, 4);
    i32_4_raw_inter: I32, 4, false, true => (Sint32x4, 4);
    i32_4_norm_own: I32, 4, true, false => panic;
    i32_4_norm_inter: I32, 4, true, true => panic;
    u32_1_raw_own: U32, 1, false, false => (Uint32, 1);
    u32_1_raw_inter: U32, 1, false, true => (Uint32, 1);
    u32_1_norm_own: U32, 1, true, false => (Uint32, 1);
    u32_1_norm_inter: U32, 1, true, true => (Uint32, 1);
    u32_2_raw_own: U32, 2, false, false => (Uint32x2, 2);
    u32_2_raw_inter: U32, 2, false, true => (Uint32x2, 2);
    u32_2_norm_own: U32, 2, true, false => panic;
    u32_2_norm_inter: U32, 2, true, true => panic;
    u32_3_raw_own: U32, 3, false, false => (Uint32x3, 3);
    u32_3_raw_inter: U32, 3, false, true => (Uint32x3, 3);
    u32_3_norm_own: U32, 3, true, false => panic;
    u32_3_norm_inter: U32, 3, true, true => panic;
    u32_4_raw_own: U32, 4, false, false => (Uint32x4, 4);
    u32_4_raw_inter: U32, 4, false, true => (Uint32x4, 4);
    u32_4_norm_own: U32, 4, true, false => panic;
    u32_4_norm_inter: U32, 4, true, true => panic;
}
