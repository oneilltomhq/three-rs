//! Vertex inputs in the geometry attribute's own type (issue #294).
//!
//! `NodeBuilder.getTypeFromAttribute()` declares an attribute by its array:
//! a normalized `Uint8Array` colour is a `vec4<f32>` input read through
//! `unorm8x4`, and a non-normalized `Uint32Array` is a `vec4<u32>` over
//! `uint32x4`, converted where the node wants floats. Two more rows of the
//! table ride along: a normalized `Int8Array` of item size 3 is padded to a
//! 4-byte stride (`snorm8x4`), and a non-normalized `Uint16Array` is widened
//! to `Uint32Array` on upload (`uint32x2`, stride 8). This reads the built
//! program, so no GPU is needed; `tests/renderer_typed_attributes.rs` draws
//! the colour.

use three_rs::core::{BufferAttribute, BufferGeometry};
use three_rs::materials::{setup, MeshBasicNodeMaterial, SetupContext};
use three_rs::nodes::builder::VertexBufferSource;
use three_rs::nodes::tsl::attribute;
use three_rs::nodes::{NodeBuilder, NodeProgram, Type};

fn program() -> NodeProgram {
    let mut geometry = BufferGeometry::new();
    geometry.set_attribute(
        "position",
        BufferAttribute::new(vec![0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0], 3),
    );
    geometry.set_attribute(
        "foo",
        BufferAttribute::uint8(
            vec![255, 0, 0, 255, 0, 255, 0, 255, 0, 0, 255, 255],
            4,
            true,
        ),
    );
    geometry.set_attribute(
        "bar",
        BufferAttribute::uint32(vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12], 4, false),
    );

    geometry.set_attribute(
        "baz",
        BufferAttribute::int8(vec![127, 0, -127, 0, 127, 0, -127, -127, 0], 3, true),
    );
    geometry.set_attribute(
        "qux",
        BufferAttribute::uint16(vec![1, 2, 3, 4, 5, 6], 2, false),
    );

    let mut material = MeshBasicNodeMaterial::new();
    material.color_node = Some(
        attribute("foo", Type::Vec4)
            .xyz()
            .add(attribute("bar", Type::Vec4).xyz())
            .add(attribute("baz", Type::Vec3))
            .add(attribute("qux", Type::Vec2).x()),
    );

    let flow = setup(
        &material,
        &SetupContext {
            geometry_attributes: geometry.attribute_descs(),
            ..SetupContext::default()
        },
        None,
    );
    NodeBuilder::new().build(&flow)
}

#[test]
fn vertex_inputs_are_declared_in_the_attributes_own_types() {
    let program = program();
    let wgsl = &program.vertex_wgsl;
    for input in [
        "foo : vec4<f32>",
        "bar : vec4<u32>",
        "baz : vec3<f32>",
        "qux : vec2<u32>",
        "position : vec3<f32>",
    ] {
        assert!(wgsl.contains(input), "missing `{input}` in:\n{wgsl}");
    }
    // `builder.format()` converts the integer inputs to the node's floats;
    // the normalized ones are floats already.
    assert!(wgsl.contains("vec4<f32>( bar )"), "{wgsl}");
    assert!(wgsl.contains("vec2<f32>( qux )"), "{wgsl}");
    assert!(!wgsl.contains("vec4<f32>( foo )"), "{wgsl}");
}

#[test]
fn vertex_buffers_carry_the_typed_formats_and_strides() {
    let program = program();
    let mut buffers: Vec<(String, u64, Type, u64, wgpu::VertexFormat)> = program
        .vertex_buffers()
        .into_iter()
        .map(|desc| {
            assert!(!desc.instanced);
            assert_eq!(desc.attributes.len(), 1, "one attribute per own buffer");
            let (_, ty, offset, format) = desc.attributes[0];
            let name = match &desc.source {
                VertexBufferSource::Geometry(slot) => slot.name.to_string(),
                other => panic!("not a geometry buffer: {other:?}"),
            };
            (name, desc.array_stride, ty, offset, format)
        })
        .collect();
    buffers.sort_by(|a, b| a.0.cmp(&b.0));
    assert_eq!(
        buffers,
        vec![
            (
                "bar".into(),
                16,
                Type::UVec4,
                0,
                wgpu::VertexFormat::Uint32x4
            ),
            ("baz".into(), 4, Type::Vec3, 0, wgpu::VertexFormat::Snorm8x4),
            ("foo".into(), 4, Type::Vec4, 0, wgpu::VertexFormat::Unorm8x4),
            (
                "position".into(),
                12,
                Type::Vec3,
                0,
                wgpu::VertexFormat::Float32x3
            ),
            (
                "qux".into(),
                8,
                Type::UVec2,
                0,
                wgpu::VertexFormat::Uint32x2
            ),
        ]
    );
}

/// `computeSkinning()` over a normalized `Uint16Array` skinWeight — what
/// `BufferGeometryLoader` builds from a `Uint16Array` JSON attribute — reads
/// it through the typed accessors instead of panicking on `array()`.
#[test]
fn compute_skinning_reads_a_normalized_uint16_skin_weight() {
    use std::cell::RefCell;
    use std::rc::Rc;
    use three_rs::nodes::skinning::compute_skinning;
    use three_rs::objects::{Bone, Skeleton, SkinnedMesh};

    let mut geometry = BufferGeometry::new();
    geometry.set_attribute(
        "position",
        BufferAttribute::new(vec![0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0], 3),
    );
    geometry.set_attribute("skinIndex", BufferAttribute::uint16(vec![0; 12], 4, false));
    geometry.set_attribute(
        "skinWeight",
        BufferAttribute::uint16([65535, 0, 0, 0].repeat(3), 4, true),
    );
    let mesh = SkinnedMesh::new(Rc::new(geometry), None);
    let bone = Bone::new();
    mesh.add(&bone);
    let skeleton = Rc::new(RefCell::new(Skeleton::new(vec![bone], None)));
    SkinnedMesh::bind(&mesh, skeleton, None);

    let _ = compute_skinning(&mesh);
}
