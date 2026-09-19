//! The vertex layout `Line2NodeMaterial` asks the renderer for.
//!
//! three.js builds a fat line out of one `InstancedInterleavedBuffer( array, 6,
//! 1 )` per pair of instanced attributes: `instanceStart` / `instanceEnd` are
//! two `InterleavedBufferAttribute` views into the *same* buffer, at float
//! offsets 0 and 3, and so are `instanceColorStart` / `instanceColorEnd`. That
//! means two instanced vertex buffers of stride 24, not four of stride 12 —
//! a difference the picture would not show (the shader reads the same numbers
//! either way) but the buffer count and the offsets would, so it is asserted
//! here rather than left to the image.
//!
//! No GPU: this reads the built program.

use three_rs::addons::lines::LineGeometry;
use three_rs::materials::{setup, MeshBasicNodeMaterial, SetupContext};
use three_rs::math::Color;
use three_rs::nodes::builder::VertexBufferDesc;
use three_rs::nodes::{NodeBuilder, Type};

fn layout(vertex_colors: bool) -> Vec<VertexBufferDesc> {
    let mut geometry = LineGeometry::new();
    geometry.set_positions(&[0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 1.0, 1.0, 0.0]);
    geometry.set_colors(&[1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0]);

    let mut material = MeshBasicNodeMaterial::line2(Color::from_hex(0xffffff));
    material.linewidth = 5.0;
    material.vertex_colors = vertex_colors;

    let flow = setup(
        &material,
        &SetupContext {
            line_segments: Some(geometry.as_segments().attributes()),
            ..SetupContext::default()
        },
        None,
    );
    NodeBuilder::new().build(&flow).vertex_buffers()
}

/// `( stride, stepMode == 'instance', ( type, offset ) per attribute )`.
type BufferShape = (u64, bool, Vec<(Type, u64)>);

fn shape(buffers: &[VertexBufferDesc]) -> Vec<BufferShape> {
    buffers
        .iter()
        .map(|desc| {
            (
                desc.array_stride,
                desc.instanced,
                desc.attributes
                    .iter()
                    .map(|(_, ty, offset)| (*ty, *offset))
                    .collect(),
            )
        })
        .collect()
}

#[test]
fn the_fat_line_takes_four_buffers_two_of_them_interleaved_pairs() {
    assert_eq!(
        shape(&layout(true)),
        vec![
            // The quad's own `position`.
            (12, false, vec![(Type::Vec3, 0)]),
            // `instanceStart` / `instanceEnd` — one buffer, two views.
            (24, true, vec![(Type::Vec3, 0), (Type::Vec3, 12)]),
            // The quad's `uv`, read in the fragment stage through a varying.
            (8, false, vec![(Type::Vec2, 0)]),
            // `instanceColorStart` / `instanceColorEnd`.
            (24, true, vec![(Type::Vec3, 0), (Type::Vec3, 12)]),
        ]
    );
}

/// Shader locations are handed out in flow order and must not collide across
/// buffers — two views of one buffer get two of them.
#[test]
fn every_attribute_has_its_own_shader_location() {
    let buffers = layout(true);
    let mut locations: Vec<u32> = buffers
        .iter()
        .flat_map(|desc| desc.attributes.iter().map(|(location, _, _)| *location))
        .collect();
    let before = locations.len();
    locations.sort_unstable();
    locations.dedup();
    assert_eq!(locations.len(), before, "duplicate shader location");
    assert_eq!(locations, (0..before as u32).collect::<Vec<_>>());
}

/// With `vertexColors` off nothing reads the colour pair, so its buffer is not
/// bound at all — three drops it the same way, by never building the node.
#[test]
fn without_vertex_colors_the_colour_pair_is_not_bound() {
    assert_eq!(
        shape(&layout(false)),
        vec![
            (12, false, vec![(Type::Vec3, 0)]),
            (24, true, vec![(Type::Vec3, 0), (Type::Vec3, 12)]),
            (8, false, vec![(Type::Vec2, 0)]),
        ]
    );
}
