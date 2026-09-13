//! The uniform-buffer / instanced-attribute branch of `RangeNode.setup()` and
//! `createInstanceMatrixNode()`.
//!
//! Three's rule is `uniformBufferSize <= builder.getUniformBufferLimit()`, the
//! limit being `device.limits.maxUniformBufferBindingSize` — 65536 on the
//! grader's adapter and with wgpu's default limits. So an instance matrix
//! switches to instanced vertex attributes above 65536 / 64 = 1024 instances,
//! and a `range()` above 65536 / 16 = 4096. Everything here reads the built
//! program, so no GPU is needed; `tests/renderer_instanced.rs` draws through
//! the attribute path.

use three_rs::materials::{instanced_range, setup, MeshBasicNodeMaterial, SetupContext};
use three_rs::math::Color;
use three_rs::nodes::builder::{AttributeSource, VertexBufferSource};
use three_rs::nodes::{BindingDesc, NodeBuilder, NodeProgram, Type};

const LIMIT: usize = 65536;

fn build(material: &MeshBasicNodeMaterial, count: usize) -> NodeProgram {
    let flow = setup(
        material,
        &SetupContext {
            instance_count: Some(count),
            instanced: true,
            light_count: 0,
        },
        None,
    );
    NodeBuilder::new().build(&flow)
}

fn program(count: usize, range_count: Option<usize>) -> NodeProgram {
    let mut material = MeshBasicNodeMaterial::new();
    if let Some(range_count) = range_count {
        material.color_node = Some(
            instanced_range(
                Color::new(0.0, 1.0, 0.0),
                Color::new(0.0, 0.0, 1.0),
                range_count,
            )
            .xyz(),
        );
    }
    build(&material, count)
}

/// The per-instance vertex buffers of a program: `( stride, attribute offsets )`
/// in bytes.
fn instanced_buffers(program: &NodeProgram) -> Vec<(u64, Vec<u64>)> {
    program
        .vertex_buffers()
        .iter()
        .filter(|desc| desc.instanced)
        .map(|desc| {
            assert!(matches!(desc.source, VertexBufferSource::Instance(_)));
            (
                desc.array_stride,
                desc.attributes
                    .iter()
                    .map(|(_, _, offset)| *offset)
                    .collect(),
            )
        })
        .collect()
}

fn buffer_names(program: &NodeProgram) -> Vec<String> {
    program
        .groups
        .iter()
        .flatten()
        .filter_map(|desc| match desc {
            BindingDesc::Buffer { name, .. } => Some(name.clone()),
            _ => None,
        })
        .collect()
}

#[test]
fn an_instance_matrix_at_the_limit_is_a_uniform_buffer() {
    // 1024 * 16 * 4 = 65536, the limit exactly, so still a uniform buffer.
    assert_eq!(1024 * 16 * 4, LIMIT);
    let program = program(1024, None);
    assert!(instanced_buffers(&program).is_empty());
    assert!(!program.vertex_wgsl.contains("nodeAttribute"));
    assert_eq!(buffer_names(&program), vec!["NodeBuffer_0".to_string()]);
}

#[test]
fn an_instance_matrix_over_the_limit_is_four_interleaved_vec4_attributes() {
    // 1025 * 16 * 4 = 65600 > 65536. `InstancedInterleavedBuffer( array, 16, 1 )`
    // read as four `vec4` views at float offsets 0/4/8/12 — stride 64 bytes,
    // attribute offsets 0/16/32/48.
    let program = program(1025, None);
    assert_eq!(instanced_buffers(&program), vec![(64, vec![0, 16, 32, 48])]);

    // The four views are joined back into the `mat4` the instance transform
    // needs, and no uniform buffer is declared for it.
    assert!(program.vertex_wgsl.contains(
        "mat4x4<f32>( nodeAttribute0, nodeAttribute1, nodeAttribute2, nodeAttribute3 )"
    ));
    assert!(buffer_names(&program).is_empty());

    // Four `vec4` instanced attributes, then the geometry's own attributes,
    // each in a per-vertex buffer of its own.
    let slots: Vec<(&str, Type)> = program
        .attributes
        .iter()
        .map(|slot| (slot.name.as_str(), slot.ty))
        .collect();
    assert_eq!(
        slots,
        vec![
            ("nodeAttribute0", Type::Vec4),
            ("nodeAttribute1", Type::Vec4),
            ("nodeAttribute2", Type::Vec4),
            ("nodeAttribute3", Type::Vec4),
            ("position", Type::Vec3),
            ("normal", Type::Vec3),
        ]
    );
    let instanced: Vec<bool> = program
        .vertex_buffers()
        .iter()
        .map(|desc| desc.instanced)
        .collect();
    assert_eq!(instanced, vec![true, false, false]);
}

#[test]
fn a_range_at_the_limit_is_a_uniform_buffer() {
    // 4096 * 4 * 4 = 65536, the limit exactly.
    let program = program(16, Some(4096));
    assert!(instanced_buffers(&program).is_empty());
    // Read in the fragment stage, indexed through the instance-index varying.
    assert!(program.fragment_wgsl.contains("NodeBuffer_"));
}

#[test]
fn a_range_over_the_limit_is_an_instanced_attribute_reaching_the_fragment_stage() {
    // 4097 * 4 * 4 = 65552 > 65536. One `vec4` per instance in its own buffer,
    // stride 16, offset 0.
    let program = program(16, Some(4097));
    assert_eq!(instanced_buffers(&program), vec![(16, vec![0])]);

    // `AttributeNode.generate()` in a fragment-stage flow: the attribute turns
    // into a varying the vertex stage writes, which is how a whole instanced
    // `vec4` reaches the fragment flow without an instance-index round trip.
    assert!(program.vertex_wgsl.contains("nodeAttribute0"));
    assert!(program.fragment_wgsl.contains("nodeVarying"));
    // The only uniform buffer left is the 16-instance matrix, which is under
    // its own limit; the range is gone from the fragment stage's bindings.
    assert_eq!(buffer_names(&program), vec!["NodeBuffer_0".to_string()]);
    assert!(!program.fragment_wgsl.contains("NodeBuffer_"));
}

#[test]
fn two_ranges_with_the_same_bounds_are_two_buffers() {
    // `RangeNode` fills its array from `Math.random` once per node, so two
    // `range( 0, 1 )` calls are two buffers with two different fills. Dedup is
    // by node identity; a value-keyed cache would collapse them and hand every
    // instance the first node's values.
    let min = Color::new(0.0, 0.0, 0.0);
    let max = Color::new(1.0, 1.0, 1.0);

    let count = 64;
    let mut material = MeshBasicNodeMaterial::new();
    material.color_node = Some(
        instanced_range(min, max, count)
            .xyz()
            .add(instanced_range(min, max, count).xyz()),
    );
    let program = build(&material, count);
    // `NodeBuffer_0` is the instance matrix (64 instances, under its limit);
    // the two ranges are the two after it.
    assert_eq!(
        buffer_names(&program),
        vec![
            "NodeBuffer_0".to_string(),
            "NodeBuffer_1".to_string(),
            "NodeBuffer_2".to_string()
        ]
    );

    // And over the limit: the instance matrix plus one buffer per range.
    let many = 8192;
    let mut material = MeshBasicNodeMaterial::new();
    material.color_node = Some(
        instanced_range(min, max, many)
            .xyz()
            .add(instanced_range(min, max, many).xyz()),
    );
    let program = build(&material, many);
    assert_eq!(
        instanced_buffers(&program),
        vec![(64, vec![0, 16, 32, 48]), (16, vec![0]), (16, vec![0])]
    );

    let ids: Vec<usize> = program
        .attributes
        .iter()
        .filter_map(|slot| match &slot.source {
            AttributeSource::Instance { buffer, .. } => Some(std::rc::Rc::as_ptr(buffer) as usize),
            AttributeSource::Geometry(_) => None,
        })
        .collect();
    // The four views of the interleaved matrix share one buffer; the two ranges
    // get one each.
    assert_eq!(ids[0], ids[3]);
    assert_ne!(ids[4], ids[5]);
}
