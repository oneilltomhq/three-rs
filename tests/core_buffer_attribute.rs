//! Port of `three.js/test/unit/src/core/BufferAttribute.tests.js`.
//!
//! Skipped: `Extending`/`Instancing` (the Rust constructor takes the array and
//! item size only, with no usage/normalized/typed-array variants), `setUsage`,
//! `isBufferAttribute`, `copy`, `clone` (the Rust attribute derives Clone),
//! `onUpload`, `toJSON`, and the whole typed-array family
//! (Int8/Uint8/…/Float16BufferAttribute) — the port stores `f32` only.

use three_rs::core::BufferAttribute;

#[test]
fn copy_at() {
    let attr = BufferAttribute::new(vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0], 3);
    let mut attr2 = BufferAttribute::new(vec![0.0; 9], 3);

    attr2.copy_at(1, &attr, 2);
    attr2.copy_at(0, &attr, 1);
    attr2.copy_at(2, &attr, 0);

    let i = &attr.array;
    let i2 = &attr2.array; // should be [4, 5, 6, 7, 8, 9, 1, 2, 3]

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

    assert_eq!(a.array.as_slice(), &f32a, "Check array has new values");
}

#[test]
fn set() {
    let mut a = BufferAttribute::new(vec![1.0, 2.0, 3.0, 4.0], 2);
    let expected = [9.0, 2.0, 8.0, 4.0];

    a.set(&[9.0], 0);
    a.set(&[8.0], 2);

    assert_eq!(
        a.array.as_slice(),
        &expected,
        "Check array has expected values"
    );
}

#[test]
fn set_get_xyzw() {
    let mut a = BufferAttribute::new(vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0], 4);
    let expected = [1.0, 2.0, -3.0, -4.0, -5.0, -6.0, 7.0, 8.0];

    let v = a.get_x(1) * -1.0;
    a.set_x(1, v);
    let v = a.get_y(1) * -1.0;
    a.set_y(1, v);
    let v = a.get_z(0) * -1.0;
    a.set_z(0, v);
    let v = a.get_w(0) * -1.0;
    a.set_w(0, v);

    assert_eq!(
        a.array.as_slice(),
        &expected,
        "Check all set* calls set the correct values"
    );
}

#[test]
fn set_xy() {
    let mut a = BufferAttribute::new(vec![1.0, 2.0, 3.0, 4.0], 2);
    let expected = [-1.0, -2.0, 3.0, 4.0];

    a.set_xy(0, -1.0, -2.0);

    assert_eq!(a.array.as_slice(), &expected, "Check for the correct values");
}

#[test]
fn set_xyz() {
    let mut a = BufferAttribute::new(vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0], 3);
    let expected = [1.0, 2.0, 3.0, -4.0, -5.0, -6.0];

    a.set_xyz(1, -4.0, -5.0, -6.0);

    assert_eq!(a.array.as_slice(), &expected, "Check for the correct values");
}

#[test]
fn set_xyzw() {
    let mut a = BufferAttribute::new(vec![1.0, 2.0, 3.0, 4.0], 4);
    let expected = [-1.0, -2.0, -3.0, -4.0];

    a.set_xyzw(0, -1.0, -2.0, -3.0, -4.0);

    assert_eq!(a.array.as_slice(), &expected, "Check for the correct values");
}

#[test]
fn count() {
    assert!(
        BufferAttribute::new(vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0], 3).count() == 2,
        "count is equal to the number of chunks"
    );
}
