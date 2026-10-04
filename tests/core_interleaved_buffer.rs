//! Port of `three.js/test/unit/src/core/InterleavedBuffer.tests.js` and
//! `InstancedInterleavedBuffer.tests.js`.
//!
//! Skipped: `Instancing` with no arguments, `isInterleavedBuffer`,
//! `isInstancedInterleavedBuffer`, `setUsage` and the `usage` assertion in
//! `copy` (no usage hint), `onUpload`.

use three_rs::core::{InterleavedBuffer, TypedArray};

fn check_instance_against_copy(instance: &InterleavedBuffer, copied: &InterleavedBuffer) {
    let a = instance.array();
    let b = copied.array();
    for i in 0..a.len() {
        assert!(b[i] == a[i], "array was copied");
    }
    assert!(copied.stride() == instance.stride(), "stride was copied");
}

#[test]
fn needs_update() {
    let a = InterleavedBuffer::new(vec![1.0f32, 2.0, 3.0, 4.0], 2);
    a.set_needs_update();
    assert_eq!(a.version(), 1, "Check version increased");
}

#[test]
fn copy() {
    // three copies the instance onto itself; borrowing rules want two.
    let instance = InterleavedBuffer::new(vec![1.0f32, 2.0, 3.0, 7.0, 8.0, 9.0], 3);
    let mut copied = InterleavedBuffer::new(vec![0.0f32; 4], 2);
    copied.copy(&instance);
    check_instance_against_copy(&instance, &copied);
}

#[test]
fn copy_at() {
    let a = InterleavedBuffer::new(vec![1.0f32, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0], 3);
    let b = InterleavedBuffer::new(vec![0.0f32; 9], 3);
    let expected = [4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 1.0, 2.0, 3.0];

    b.copy_at(1, &a, 2);
    b.copy_at(0, &a, 1);
    b.copy_at(2, &a, 0);

    assert_eq!(
        b.array().as_slice(),
        &expected,
        "Check the right values were replaced"
    );
}

#[test]
fn set() {
    let instance = InterleavedBuffer::new(vec![1.0f32, 2.0, 3.0, 7.0, 8.0, 9.0], 3);
    instance.set(&[0.0, -1.0], 0);
    let array = instance.array();
    assert!(
        array[0] == 0.0 && array[1] == -1.0,
        "replace at first by default"
    );
}

#[test]
fn count() {
    let instance = InterleavedBuffer::new(vec![1.0f32, 2.0, 3.0, 7.0, 8.0, 9.0], 3);
    assert_eq!(
        instance.count(),
        2,
        "count is calculated via array length / stride"
    );
}

/// `InterleavedBuffer.clone()` copies the array under a new id at version 0.
#[test]
fn clone() {
    let instance = InterleavedBuffer::new(vec![1u8, 2, 3, 4], 2);
    instance.set_needs_update();
    let cloned = instance.clone();
    assert_eq!(*cloned.data(), TypedArray::U8(vec![1, 2, 3, 4]));
    assert_eq!(cloned.stride(), 2);
    assert_eq!(cloned.version(), 0);
    assert_ne!(cloned.id(), instance.id());
}

/// A typed array other than `Float32Array` goes through `copyAt` and `set`
/// with the store's conversion.
#[test]
fn typed_set_and_copy_at() {
    let a = InterleavedBuffer::new(vec![1i16, -2, 3, -4], 2);
    a.set(&[40000.0], 0);
    assert_eq!(*a.data(), TypedArray::I16(vec![-25536, -2, 3, -4]));

    let b = InterleavedBuffer::new(vec![0u8; 4], 2);
    b.copy_at(1, &a, 0);
    assert_eq!(*b.data(), TypedArray::U8(vec![0, 0, 64, 254]));
}

// InstancedInterleavedBuffer

#[test]
fn instanced_instancing() {
    let instance = InterleavedBuffer::new_instanced(vec![1.0f32, 2.0, 3.0, 7.0, 8.0, 9.0], 3, 1);
    assert!(instance.mesh_per_attribute() == 1, "ok");
    assert!(instance.is_instanced());
}

#[test]
fn instanced_copy() {
    let instance = InterleavedBuffer::new_instanced(vec![1.0f32, 2.0, 3.0, 7.0, 8.0, 9.0], 3, 2);
    let mut copied = InterleavedBuffer::new_instanced(vec![0.0f32; 3], 3, 1);
    copied.copy(&instance);
    assert!(
        copied.mesh_per_attribute() == 2,
        "additional attribute was copied"
    );
    check_instance_against_copy(&instance, &copied);
}
