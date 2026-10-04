//! Port of `three.js/test/unit/src/core/InterleavedBufferAttribute.tests.js`,
//! plus the clone semantics of an interleaved view (de-interleaving on its
//! own, kept when the whole geometry is cloned).
//!
//! Skipped: `Instancing` with no arguments, `isInterleavedBufferAttribute`.

use std::rc::Rc;
use three_rs::core::{
    ArrayKind, AttributeLayout, BufferAttribute, BufferGeometry, InterleavedBuffer, TypedArray,
};

#[test]
fn count() {
    let buffer = Rc::new(InterleavedBuffer::new(
        vec![1.0f32, 2.0, 3.0, 7.0, 8.0, 9.0],
        3,
    ));
    let instance = BufferAttribute::interleaved(buffer, 2, 0, false);
    assert!(
        instance.count() == 2,
        "count is calculated via array length / stride"
    );
}

#[test]
fn set_x() {
    let buffer = Rc::new(InterleavedBuffer::new(
        vec![1.0f32, 2.0, 3.0, 7.0, 8.0, 9.0],
        3,
    ));
    let mut instance = BufferAttribute::interleaved(buffer.clone(), 2, 0, false);
    instance.set_x(0, 123.0);
    instance.set_x(1, 321.0);
    {
        let array = buffer.array();
        assert!(
            array[0] == 123.0 && array[3] == 321.0,
            "x was calculated correct based on index and default offset"
        );
    }

    let buffer = Rc::new(InterleavedBuffer::new(
        vec![1.0f32, 2.0, 3.0, 7.0, 8.0, 9.0],
        3,
    ));
    let mut instance = BufferAttribute::interleaved(buffer.clone(), 2, 1, false);
    instance.set_x(0, 123.0);
    instance.set_x(1, 321.0);
    // the offset was defined as 1, so go one step further in the array
    let array = buffer.array();
    assert!(
        array[1] == 123.0 && array[4] == 321.0,
        "x was calculated correct based on index and default offset"
    );
}

/// A normalized `Uint8` view reads and writes through the shared buffer.
#[test]
fn normalized_view() {
    let buffer = Rc::new(InterleavedBuffer::new(vec![0u8, 255, 10, 20, 0, 0], 3));
    let mut colour = BufferAttribute::interleaved(buffer.clone(), 2, 0, true);
    assert_eq!(colour.kind(), ArrayKind::U8);
    assert_eq!(colour.get_y(0), 1.0);
    colour.set_xy(1, 1.0, 0.5);
    assert_eq!(
        *buffer.data(),
        TypedArray::U8(vec![0, 255, 10, 255, 128, 0])
    );
    assert!(colour.is_interleaved());
    assert_eq!(colour.offset(), 0);
}

/// `InterleavedBufferAttribute.clone()` with no `data` de-interleaves: a plain
/// attribute of the same kind holding just this view's items.
#[test]
fn clone_de_interleaves() {
    let buffer = Rc::new(InterleavedBuffer::new(vec![1i16, 2, 3, 4, 5, 6], 3));
    let view = BufferAttribute::interleaved(buffer, 2, 1, false);
    let cloned = view.clone();
    assert!(!cloned.is_interleaved());
    assert_eq!(*cloned.data(), TypedArray::I16(vec![2, 3, 5, 6]));
    assert_eq!(cloned.count(), 2);
}

/// `BufferGeometry.clone()` goes through `toJSON`/`copy` with a shared
/// `data` cache, so two views of one buffer still share one (new) buffer.
#[test]
fn geometry_clone_keeps_views_sharing_a_buffer() {
    let buffer = Rc::new(InterleavedBuffer::new(
        vec![0.0f32, 1.0, 2.0, 3.0, 4.0, 10.0, 11.0, 12.0, 13.0, 14.0],
        5,
    ));
    let mut geometry = BufferGeometry::new();
    geometry.set_attribute(
        "position",
        BufferAttribute::interleaved(buffer.clone(), 3, 0, false),
    );
    geometry.set_attribute(
        "uv",
        BufferAttribute::interleaved(buffer.clone(), 2, 3, false),
    );

    let cloned = geometry.clone();
    let position = cloned.get_attribute("position").unwrap();
    let uv = cloned.get_attribute("uv").unwrap();
    let a = position.data_buffer().expect("position stays interleaved");
    let b = uv.data_buffer().expect("uv stays interleaved");
    assert!(Rc::ptr_eq(a, b), "the views share one buffer");
    assert!(!Rc::ptr_eq(a, &buffer), "and it is a copy");
    assert_eq!(*a.array(), *buffer.array());
    assert_eq!(uv.offset(), 3);

    let descs = cloned.attribute_descs();
    let groups: Vec<_> = descs
        .iter()
        .map(|d| match d.layout {
            AttributeLayout::Interleaved { buffer, .. } => buffer,
            other => panic!("{} is not interleaved: {other:?}", d.name),
        })
        .collect();
    assert_eq!(groups[0], groups[1], "one vertex buffer group");
}

/// `computeVertexNormals()` writes an existing normal through `setXYZ()`, so
/// an interleaved normal stays a view of its buffer (and the buffer is the
/// one marked for re-upload) rather than being replaced by a plain copy.
#[test]
fn compute_vertex_normals_writes_an_interleaved_normal_in_place() {
    // position xyz, normal xyz per vertex: one triangle in the z = 0 plane.
    let buffer = Rc::new(InterleavedBuffer::new(
        vec![
            0.0, 0.0, 0.0, 9.0, 9.0, 9.0, //
            1.0, 0.0, 0.0, 9.0, 9.0, 9.0, //
            0.0, 1.0, 0.0, 9.0, 9.0, 9.0,
        ],
        6,
    ));
    let mut geometry = BufferGeometry::new();
    geometry.set_attribute(
        "position",
        BufferAttribute::interleaved(buffer.clone(), 3, 0, false),
    );
    geometry.set_attribute(
        "normal",
        BufferAttribute::interleaved(buffer.clone(), 3, 3, false),
    );
    let version = buffer.version();

    geometry.compute_vertex_normals();

    let normal = geometry.get_attribute("normal").unwrap();
    assert!(normal.is_interleaved(), "still a view");
    assert!(Rc::ptr_eq(normal.data_buffer().unwrap(), &buffer));
    for i in 0..3 {
        assert_eq!(
            (normal.get_x(i), normal.get_y(i), normal.get_z(i)),
            (0.0, 0.0, 1.0)
        );
    }
    assert_eq!(buffer.array()[0..3], [0.0, 0.0, 0.0], "positions untouched");
    assert!(buffer.version() > version, "the shared buffer re-uploads");
}
