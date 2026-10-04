//! Port of `three.js/test/unit/src/core/BufferAttribute.tests.js`, with the
//! typed-array family (`Int8BufferAttribute` … `Float16BufferAttribute`) and
//! the normalized accessors, whose expected values follow `MathUtils.normalize`
//! / `denormalize` and the ECMAScript typed-array store conversions.
//!
//! Skipped: `Extending`/`Instancing` (no class hierarchy), `setUsage` and the
//! `usage` assertions (no usage hint), `isBufferAttribute`, `onUpload`,
//! `toJSON`.

use three_rs::core::{ArrayKind, BufferAttribute, TypedArray};
use three_rs::extras::{from_half_float, to_half_float};

#[test]
fn copy_at() {
    let attr = BufferAttribute::new(vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0], 3);
    let mut attr2 = BufferAttribute::new(vec![0.0; 9], 3);

    attr2.copy_at(1, &attr, 2);
    attr2.copy_at(0, &attr, 1);
    attr2.copy_at(2, &attr, 0);

    let i = attr.array();
    let i2 = attr2.array(); // should be [4, 5, 6, 7, 8, 9, 1, 2, 3]

    assert!(
        i2[0] == i[3] && i2[1] == i[4] && i2[2] == i[5],
        "chunk copied to correct place"
    );
    assert!(
        i2[3] == i[6] && i2[4] == i[7] && i2[5] == i[8],
        "chunk copied to correct place"
    );
    assert!(
        i2[6] == i[0] && i2[7] == i[1] && i2[8] == i[2],
        "chunk copied to correct place"
    );
}

#[test]
fn copy_array() {
    let f32a = [5.0, 6.0, 7.0, 8.0];
    let mut a = BufferAttribute::new(vec![1.0, 2.0, 3.0, 4.0], 2);

    a.copy_array(&f32a);

    assert_eq!(a.array().as_slice(), &f32a, "Check array has new values");
}

#[test]
fn set() {
    let mut a = BufferAttribute::new(vec![1.0, 2.0, 3.0, 4.0], 2);
    let expected = [9.0, 2.0, 8.0, 4.0];

    a.set(&[9.0], 0);
    a.set(&[8.0], 2);

    assert_eq!(
        a.array().as_slice(),
        &expected,
        "Check array has expected values"
    );
}

#[test]
fn set_get_xyzw() {
    let mut a = BufferAttribute::new(vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0], 4);
    let expected = [1.0, 2.0, -3.0, -4.0, -5.0, -6.0, 7.0, 8.0];

    let v = -a.get_x(1);
    a.set_x(1, v);
    let v = -a.get_y(1);
    a.set_y(1, v);
    let v = -a.get_z(0);
    a.set_z(0, v);
    let v = -a.get_w(0);
    a.set_w(0, v);

    assert_eq!(
        a.array().as_slice(),
        &expected,
        "Check all set* calls set the correct values"
    );
}

#[test]
fn set_xy() {
    let mut a = BufferAttribute::new(vec![1.0, 2.0, 3.0, 4.0], 2);
    let expected = [-1.0, -2.0, 3.0, 4.0];

    a.set_xy(0, -1.0, -2.0);

    assert_eq!(
        a.array().as_slice(),
        &expected,
        "Check for the correct values"
    );
}

#[test]
fn set_xyz() {
    let mut a = BufferAttribute::new(vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0], 3);
    let expected = [1.0, 2.0, 3.0, -4.0, -5.0, -6.0];

    a.set_xyz(1, -4.0, -5.0, -6.0);

    assert_eq!(
        a.array().as_slice(),
        &expected,
        "Check for the correct values"
    );
}

#[test]
fn set_xyzw() {
    let mut a = BufferAttribute::new(vec![1.0, 2.0, 3.0, 4.0], 4);
    let expected = [-1.0, -2.0, -3.0, -4.0];

    a.set_xyzw(0, -1.0, -2.0, -3.0, -4.0);

    assert_eq!(
        a.array().as_slice(),
        &expected,
        "Check for the correct values"
    );
}

#[test]
fn count() {
    assert!(
        BufferAttribute::new(vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0], 3).count() == 2,
        "count is equal to the number of chunks"
    );
}

/// `attribute.version` / `attribute.needsUpdate = true` (issue #47). three.js
/// starts at 0 and `needsUpdate`'s setter is `this.version ++`, so a write
/// without it leaves the version alone — which is what lets the renderer tell
/// a changed attribute from an unchanged one.
#[test]
fn needs_update_bumps_the_version() {
    let attribute = BufferAttribute::new(vec![1.0, 2.0, 3.0], 3);
    assert_eq!(attribute.version(), 0, "a new attribute is version 0");

    // interior mutability: no `&mut`, so this works through an `Rc`
    attribute.array_mut()[0] = 9.0;
    assert_eq!(attribute.array()[0], 9.0);
    assert_eq!(
        attribute.version(),
        0,
        "writing the array does not bump the version on its own"
    );

    attribute.set_needs_update();
    assert_eq!(attribute.version(), 1);
    attribute.set_needs_update();
    assert_eq!(attribute.version(), 2);
}

#[test]
fn copy() {
    let attr = BufferAttribute::new(vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0], 3);
    attr.set_needs_update();

    let mut attr_copy = BufferAttribute::new(Vec::new(), 1);
    attr_copy.copy(&attr);

    assert!(attr.count() == attr_copy.count(), "count is equal");
    assert!(attr.item_size == attr_copy.item_size, "itemSize is equal");
    assert!(
        attr.array().len() == attr_copy.array().len(),
        "array length is equal"
    );
    assert!(
        attr.version() == 1 && attr_copy.version() == 0,
        "version is not copied which is good"
    );
}

#[test]
fn copy_takes_kind_and_normalized() {
    let attr = BufferAttribute::uint8(vec![0, 128, 255], 3, true);
    let mut attr_copy = BufferAttribute::new(Vec::new(), 1);
    attr_copy.copy(&attr);

    assert_eq!(attr_copy.kind(), ArrayKind::U8);
    assert!(attr_copy.normalized);
    assert_eq!(*attr_copy.data(), TypedArray::U8(vec![0, 128, 255]));

    attr_copy.set_x(0, 1.0);
    assert_eq!(attr.get_x(0), 0.0, "the array is a copy, not shared");
}

#[test]
fn clone() {
    let attr = BufferAttribute::new(vec![1.0, 2.0, 3.0, 4.0, 0.12, -12.0], 2);
    let attr_copy = attr.clone();

    assert!(
        attr.array().len() == attr_copy.array().len(),
        "attribute was cloned"
    );
    for i in 0..attr.array().len() {
        assert!(
            attr.array()[i] == attr_copy.array()[i],
            "array item is equal"
        );
    }
    assert_ne!(attr.id, attr_copy.id, "a clone is a new attribute");
}

/// `Int8BufferAttribute` … `Uint32BufferAttribute`, `Float16BufferAttribute`:
/// each constructor keeps its array's element type.
#[test]
fn typed_constructors() {
    let cases = [
        (BufferAttribute::int8(vec![-1, 2], 1, false), ArrayKind::I8),
        (BufferAttribute::uint8(vec![1, 2], 1, false), ArrayKind::U8),
        (
            BufferAttribute::uint8_clamped(vec![1, 2], 1, false),
            ArrayKind::U8Clamped,
        ),
        (
            BufferAttribute::int16(vec![-1, 2], 1, false),
            ArrayKind::I16,
        ),
        (
            BufferAttribute::uint16(vec![1, 2], 1, false),
            ArrayKind::U16,
        ),
        (
            BufferAttribute::int32(vec![-1, 2], 1, false),
            ArrayKind::I32,
        ),
        (
            BufferAttribute::uint32(vec![1, 2], 1, false),
            ArrayKind::U32,
        ),
        (
            BufferAttribute::float16(vec![to_half_float(1.0), to_half_float(2.0)], 1, false),
            ArrayKind::F16,
        ),
        (BufferAttribute::new(vec![1.0, 2.0], 1), ArrayKind::F32),
    ];
    for (attr, kind) in cases {
        assert_eq!(attr.kind(), kind);
        assert_eq!(attr.data().kind(), kind);
        assert_eq!(attr.count(), 2, "{kind:?}: count is in elements");
        assert_eq!(attr.get_y(0), 2.0, "{kind:?}: getX reads the element");
    }
}

/// `array()` is the `Float32Array` view; asking it of another kind is a bug.
#[test]
#[should_panic]
fn array_of_a_non_f32_attribute_panics() {
    let attr = BufferAttribute::uint8(vec![1, 2, 3], 3, false);
    let _ = attr.array();
}

fn from_half_float_array(attr: &BufferAttribute) -> Vec<f32> {
    match &*attr.data() {
        TypedArray::F16(bits) => bits.iter().map(|&h| from_half_float(h)).collect(),
        other => panic!("not a Float16BufferAttribute: {other:?}"),
    }
}

fn to_half_float_array(values: &[f64]) -> Vec<u16> {
    values.iter().map(|&v| to_half_float(v)).collect()
}

#[test]
fn float16_set_get_xyzw() {
    let mut a = BufferAttribute::float16(
        to_half_float_array(&[1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0]),
        4,
        false,
    );
    let expected = [1.0, 2.0, -3.0, -4.0, -5.0, -6.0, 7.0, 8.0];

    let v = -a.get_x(1);
    a.set_x(1, v);
    let v = -a.get_y(1);
    a.set_y(1, v);
    let v = -a.get_z(0);
    a.set_z(0, v);
    let v = -a.get_w(0);
    a.set_w(0, v);

    assert_eq!(
        from_half_float_array(&a),
        expected,
        "Check all set* calls set the correct values"
    );
}

#[test]
fn float16_set_xy() {
    let mut a = BufferAttribute::float16(to_half_float_array(&[1.0, 2.0, 3.0, 4.0]), 2, false);
    a.set_xy(0, -1.0, -2.0);
    assert_eq!(from_half_float_array(&a), [-1.0, -2.0, 3.0, 4.0]);
}

#[test]
fn float16_set_xyz() {
    let mut a = BufferAttribute::float16(
        to_half_float_array(&[1.0, 2.0, 3.0, 4.0, 5.0, 6.0]),
        3,
        false,
    );
    a.set_xyz(1, -4.0, -5.0, -6.0);
    assert_eq!(from_half_float_array(&a), [1.0, 2.0, 3.0, -4.0, -5.0, -6.0]);
}

#[test]
fn float16_set_xyzw() {
    let mut a = BufferAttribute::float16(to_half_float_array(&[1.0, 2.0, 3.0, 4.0]), 4, false);
    a.set_xyzw(0, -1.0, -2.0, -3.0, -4.0);
    assert_eq!(from_half_float_array(&a), [-1.0, -2.0, -3.0, -4.0]);
}

/// `BufferAttribute.set()` on a `Float16BufferAttribute` writes raw bits, as
/// three's `this.array.set( value, offset )` does.
#[test]
fn float16_set_writes_bits() {
    let mut a = BufferAttribute::float16(vec![0, 0], 1, false);
    a.set(&[f32::from(to_half_float(0.5))], 1);
    assert_eq!(*a.data(), TypedArray::F16(vec![0, to_half_float(0.5)]));
    assert_eq!(a.get_x(1), 0.5);
}

/// `normalized` reads through `MathUtils.denormalize` and writes through
/// `MathUtils.normalize` (`Math.round( value * max )`), then the store's
/// conversion: wrap modulo 2^bits, or clamp for `Uint8ClampedArray`.
#[test]
fn normalized_get_set() {
    // Uint8: 255 → 1, 1.5 → round(382.5) = 383 wraps to 127
    let mut a = BufferAttribute::uint8(vec![255, 0], 1, true);
    assert_eq!(a.get_x(0), 1.0);
    a.set_x(1, 1.5);
    assert_eq!(*a.data(), TypedArray::U8(vec![255, 127]));
    a.set_x(1, 0.5);
    assert_eq!(
        *a.data(),
        TypedArray::U8(vec![255, 128]),
        "round(127.5) = 128"
    );

    // Uint8Clamped: the same 1.5 clamps to 255, -0.1 → round(-25.5) = -25 → 0
    let mut a = BufferAttribute::uint8_clamped(vec![0, 0], 1, true);
    a.set_x(0, 1.5);
    a.set_x(1, -0.1);
    assert_eq!(*a.data(), TypedArray::U8Clamped(vec![255, 0]));
    assert_eq!(a.get_x(0), 1.0);

    // Int8: denormalize is max( x / 127, -1 ), so -128 reads as -1
    let mut a = BufferAttribute::int8(vec![-128, 127, 0], 1, true);
    assert_eq!(a.get_x(0), -1.0);
    assert_eq!(a.get_x(1), 1.0);
    a.set_x(2, -0.5);
    assert_eq!(
        *a.data(),
        TypedArray::I8(vec![-128, 127, -63]),
        "round(-63.5) = -63"
    );

    // Int16 / Uint16 / Int32 / Uint32
    let mut a = BufferAttribute::int16(vec![0], 1, true);
    a.set_x(0, 0.5);
    assert_eq!(*a.data(), TypedArray::I16(vec![16384]), "round(16383.5)");
    assert_eq!(a.get_x(0), 16384.0 / 32767.0);

    let mut a = BufferAttribute::uint16(vec![0], 1, true);
    a.set_x(0, 1.0);
    assert_eq!(*a.data(), TypedArray::U16(vec![65535]));

    let mut a = BufferAttribute::int32(vec![0], 1, true);
    a.set_x(0, -1.0);
    assert_eq!(*a.data(), TypedArray::I32(vec![-2147483647]));
    assert_eq!(a.get_x(0), -1.0);

    let a = BufferAttribute::uint32(vec![4294967295], 1, true);
    assert_eq!(a.get_x(0), 1.0);

    // Float32 normalized is the identity
    let mut a = BufferAttribute::from_typed(TypedArray::F32(vec![0.0]), 1, true);
    a.set_x(0, 0.25);
    assert_eq!(a.get_x(0), 0.25);
}

/// Non-normalized writes truncate toward zero and wrap: `new Uint8Array( [ -1.5 ] )`
/// is `[ 255 ]`; non-finite stores 0.
#[test]
fn non_normalized_stores_truncate_and_wrap() {
    let mut a = BufferAttribute::uint8(vec![0, 0, 0], 1, false);
    a.set_x(0, -1.5);
    a.set_x(1, 256.9);
    a.set_x(2, f64::NAN);
    assert_eq!(*a.data(), TypedArray::U8(vec![255, 0, 0]));

    let mut a = BufferAttribute::uint8_clamped(vec![0, 0, 0], 1, false);
    a.set_x(0, 2.5);
    a.set_x(1, 3.5);
    a.set_x(2, 1000.0);
    assert_eq!(
        *a.data(),
        TypedArray::U8Clamped(vec![2, 4, 255]),
        "Uint8Clamped rounds half to even and clamps"
    );

    let mut a = BufferAttribute::int16(vec![0], 1, false);
    a.set_x(0, 32768.0);
    assert_eq!(*a.data(), TypedArray::I16(vec![-32768]));
}

/// `copyAt` across kinds copies the raw element through the target's store
/// conversion, with no normalization.
#[test]
fn copy_at_across_kinds() {
    let source = BufferAttribute::new(vec![300.5, -1.0], 2);
    let mut target = BufferAttribute::uint8(vec![0, 0], 2, true);
    target.copy_at(0, &source, 0);
    assert_eq!(*target.data(), TypedArray::U8(vec![44, 255]));
}

/// `set( value, offset )` and `copyArray` store raw elements.
#[test]
fn set_and_copy_array_on_typed_arrays() {
    let mut a = BufferAttribute::int8(vec![1, 2, 3, 4], 2, false);
    a.set(&[200.0, -1.0], 1);
    assert_eq!(*a.data(), TypedArray::I8(vec![1, -56, -1, 4]));

    a.copy_array(&[5.0, 6.0, 7.0, 8.0]);
    assert_eq!(*a.data(), TypedArray::I8(vec![5, 6, 7, 8]));
}
