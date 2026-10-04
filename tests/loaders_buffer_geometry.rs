//! `BufferGeometryLoader.parse()` over the JSON `BufferGeometry.toJSON()`
//! writes for interleaved and typed attributes. No GPU.
//!
//! The fixture is in the shape three serialises: an
//! `InterleavedBufferAttribute` is `{ isInterleavedBufferAttribute, itemSize,
//! data: <interleaved buffer uuid>, offset, normalized }`, the buffer is
//! `data.interleavedBuffers[ uuid ] = { uuid, buffer, type, stride }`, and its
//! bytes are `data.arrayBuffers[ buffer ]` as `Uint32Array` words. The
//! float words are the IEEE bits of `[ 0, 1, 2, 0.5, 0.25, 3, 4, 5, 0.75, 1 ]`;
//! the byte buffer's words are `[ 255, 0, 128, 255 ]` and `[ 0, 255, 0, 64 ]`
//! little-endian.

use std::rc::Rc;

use three_rs::core::{ArrayKind, AttributeLayout, TypedArray};
use three_rs::loaders::BufferGeometryLoader;

const FIXTURE: &str = r#"{
  "metadata": { "version": 4.7, "type": "BufferGeometry", "generator": "BufferGeometry.toJSON" },
  "uuid": "geometry",
  "type": "BufferGeometry",
  "data": {
    "attributes": {
      "position": { "isInterleavedBufferAttribute": true, "itemSize": 3, "data": "ib-float", "offset": 0, "normalized": false },
      "uv": { "isInterleavedBufferAttribute": true, "itemSize": 2, "data": "ib-float", "offset": 3, "normalized": false },
      "color": { "isInterleavedBufferAttribute": true, "itemSize": 4, "data": "ib-bytes", "offset": 0, "normalized": true },
      "skinIndex": { "itemSize": 4, "type": "Uint16Array", "array": [1, 2, 3, 70000, 0, 0, 0, 0], "normalized": false },
      "signed": { "itemSize": 1, "type": "Int8Array", "array": [-1, 127, 128], "normalized": true },
      "clamped": { "itemSize": 4, "type": "Uint8ClampedArray", "array": [300, -5, 1.5, 2.5], "normalized": true },
      "offset": { "itemSize": 3, "type": "Float32Array", "array": [1, 2, 3], "normalized": false, "isInstancedBufferAttribute": true, "meshPerAttribute": 1 }
    },
    "morphAttributes": {
      "position": [ { "itemSize": 3, "type": "Float32Array", "array": [0.5, 0, 0, 0, 0.5, 0], "normalized": false } ]
    },
    "morphTargetsRelative": true,
    "interleavedBuffers": {
      "ib-float": { "uuid": "ib-float", "buffer": "ab-float", "type": "Float32Array", "stride": 5 },
      "ib-bytes": { "uuid": "ib-bytes", "buffer": "ab-bytes", "type": "Uint8Array", "stride": 4 }
    },
    "arrayBuffers": {
      "ab-float": [0, 1065353216, 1073741824, 1056964608, 1048576000, 1077936128, 1082130432, 1084227584, 1061158912, 1065353216],
      "ab-bytes": [4286578943, 1073807104]
    },
    "index": { "type": "Uint16Array", "array": [0, 1, 0] }
  }
}"#;

fn parse() -> three_rs::core::BufferGeometry {
    let json: serde_json::Value = serde_json::from_str(FIXTURE).unwrap();
    BufferGeometryLoader::new().parse(&json).unwrap()
}

#[test]
fn interleaved_views_share_one_buffer() {
    let geometry = parse();
    let position = geometry.get_attribute("position").unwrap();
    let uv = geometry.get_attribute("uv").unwrap();
    assert!(position.is_interleaved() && uv.is_interleaved());
    assert!(Rc::ptr_eq(
        position.data_buffer().unwrap(),
        uv.data_buffer().unwrap()
    ));
    let buffer = position.data_buffer().unwrap();
    assert_eq!(buffer.stride(), 5);
    assert_eq!(buffer.count(), 2);
    assert_eq!((position.offset(), uv.offset()), (0, 3));
    assert_eq!(position.count(), 2);
    assert_eq!(
        (position.get_x(1), position.get_y(1), position.get_z(1)),
        (3.0, 4.0, 5.0)
    );
    assert_eq!((uv.get_x(0), uv.get_y(0)), (0.5, 0.25));
    assert_eq!((uv.get_x(1), uv.get_y(1)), (0.75, 1.0));
}

#[test]
fn an_interleaved_byte_buffer_is_a_typed_view_of_the_words() {
    let geometry = parse();
    let color = geometry.get_attribute("color").unwrap();
    assert_eq!(color.kind(), ArrayKind::U8);
    assert!(color.normalized);
    assert_eq!(
        *color.data(),
        TypedArray::U8(vec![255, 0, 128, 255, 0, 255, 0, 64])
    );
    assert_eq!(color.get_x(0), 1.0);
    assert_eq!(color.get_z(0), 128.0 / 255.0);
    assert_eq!(color.get_w(1), 64.0 / 255.0);
}

#[test]
fn typed_arrays_store_as_their_javascript_constructors_do() {
    let geometry = parse();

    let skin_index = geometry.get_attribute("skinIndex").unwrap();
    assert!(!skin_index.normalized);
    // `new Uint16Array( [ 70000 ] )` wraps modulo 2^16.
    assert_eq!(
        *skin_index.data(),
        TypedArray::U16(vec![1, 2, 3, (70000 - 65536) as u16, 0, 0, 0, 0])
    );

    let signed = geometry.get_attribute("signed").unwrap();
    assert_eq!(*signed.data(), TypedArray::I8(vec![-1, 127, -128]));
    // `denormalize` on an Int8Array: max( x / 127, -1 ).
    assert_eq!(signed.get_x(0), -1.0 / 127.0);
    assert_eq!(signed.get_x(2), -1.0);

    let clamped = geometry.get_attribute("clamped").unwrap();
    assert_eq!(*clamped.data(), TypedArray::U8Clamped(vec![255, 0, 2, 2]));

    let offset = geometry.get_attribute("offset").unwrap();
    assert!(offset.is_instanced());
    assert_eq!(*offset.data(), TypedArray::F32(vec![1.0, 2.0, 3.0]));
}

#[test]
fn morph_attributes_and_the_index_load() {
    let geometry = parse();
    let (name, morphs) = geometry.morph_attributes().next().unwrap();
    assert_eq!(name, "position");
    assert_eq!(morphs.len(), 1);
    assert_eq!(morphs[0].get_y(1), 0.5);
    assert!(geometry.morph_targets_relative);
    assert!(geometry.index.is_some());
}

#[test]
fn the_vertex_layout_reads_the_interleaved_buffer_once() {
    let geometry = parse();
    let descs = geometry.attribute_descs();
    let find = |name: &str| descs.iter().find(|d| d.name == name).unwrap();
    let (position, uv, color) = (find("position"), find("uv"), find("color"));
    let group = |layout: &AttributeLayout| match layout {
        AttributeLayout::Interleaved { buffer, .. } => *buffer,
        other => panic!("not interleaved: {other:?}"),
    };
    assert_eq!(group(&position.layout), group(&uv.layout));
    assert_ne!(group(&position.layout), group(&color.layout));
    assert_eq!((position.array_stride(), uv.array_stride()), (20, 20));
    assert_eq!((position.offset(), uv.offset()), (0, 12));
    assert_eq!(color.vertex_format(), wgpu::VertexFormat::Unorm8x4);
    assert_eq!(color.array_stride(), 4);
}

#[test]
fn a_float64_array_is_refused() {
    let json = serde_json::json!({
        "data": { "attributes": { "position": {
            "itemSize": 3, "type": "Float64Array", "array": [0, 0, 0], "normalized": false
        } } }
    });
    assert!(BufferGeometryLoader::new().parse(&json).is_err());
}
