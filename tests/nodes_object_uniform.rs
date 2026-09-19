//! `webgpu_instance_uniform`'s per-object uniform, without a GPU.
//!
//! Three's `InstanceUniformNode` has `updateType = NodeUpdateType.OBJECT`: one
//! uniform node, one program, one pipeline, and a value re-read from
//! `frame.object` before every draw. The port spells that
//! [`three_rs::nodes::tsl::uniform_object`]; `docs/nodes.md` §18.
//!
//! The pixel ladder can only see this as "the teapots are the wrong colours",
//! and a diff that big is the least informative failure there is. These
//! assertions are the informative half: the uniform lands in the **object**
//! group, and two objects passed to the same members produce two different
//! sets of bytes.

use three_rs::materials::{setup, MeshBasicNodeMaterial, SetupContext};
use three_rs::math::Color;
use three_rs::nodes::tsl::uniform_object;
use three_rs::nodes::{BindingDesc, NodeBuilder, Type, UniformGroup, UpdateType};
use three_rs::renderer::UniformContext;
use three_rs::{Mesh, Object3D};

/// The object group's members and byte size for a material whose `colorNode`
/// is a per-object uniform.
fn object_group(material: &MeshBasicNodeMaterial) -> (Vec<three_rs::nodes::UniformMember>, u32) {
    let flow = setup(material, &SetupContext::default(), None);
    let program = NodeBuilder::new().build(&flow);

    for group in &program.groups {
        for binding in group {
            if let BindingDesc::Uniforms {
                group: UniformGroup::Object,
                members,
                size,
                update,
                ..
            } = binding
            {
                assert_eq!(
                    *update,
                    UpdateType::Object,
                    "an object-update uniform must not land in a render-group buffer"
                );
                return (members.clone(), *size);
            }
        }
    }
    panic!("three-rs: the material has no object group");
}

#[test]
fn a_per_object_uniform_resolves_once_per_object() {
    // The page's colours: `mesh.color`, which the port keeps beside the object
    // rather than on it.
    let red = Color::new(1.0, 0.0, 0.0);
    let green = Color::new(0.0, 1.0, 0.0);

    let first = Mesh::new(
        std::rc::Rc::new(three_rs::box_geometry(1.0, 1.0, 1.0, 1, 1, 1)),
        None,
    );
    let second = Mesh::new(
        std::rc::Rc::new(three_rs::box_geometry(1.0, 1.0, 1.0, 1, 1, 1)),
        None,
    );
    let (first_id, second_id) = (first.borrow().id, second.borrow().id);
    assert_ne!(first_id, second_id);

    let mut material = MeshBasicNodeMaterial::new();
    material.color_node = Some(uniform_object(Type::Vec3, move |object| {
        let color = if object.id == first_id { red } else { green };
        vec![color.r, color.g, color.b]
    }));

    let (members, size) = object_group(&material);

    let bytes_for = |object: &Object3D| {
        UniformContext {
            object: Some(object),
            ..UniformContext::default()
        }
        .bytes(&members, size)
    };

    let first_bytes = bytes_for(&first.borrow());
    let second_bytes = bytes_for(&second.borrow());

    // The first three floats of the object struct are the uniform: red, then
    // green, out of one set of members and one program.
    assert_eq!(
        &first_bytes[0..12],
        bytemuck::cast_slice::<f32, u8>(&[1.0f32, 0.0, 0.0])
    );
    assert_eq!(
        &second_bytes[0..12],
        bytemuck::cast_slice::<f32, u8>(&[0.0f32, 1.0, 0.0])
    );
    assert_ne!(first_bytes, second_bytes);
}

#[test]
fn two_per_object_uniforms_are_two_uniforms() {
    // `SettableValue`'s rule, for the same reason: a value that changes must
    // not be part of any cache key, so two callbacks that happen to be spelled
    // alike are still two uniforms and two members.
    let mut material = MeshBasicNodeMaterial::new();
    material.color_node = Some(
        uniform_object(Type::Vec3, |_| vec![1.0, 0.0, 0.0])
            .add(uniform_object(Type::Vec3, |_| vec![1.0, 0.0, 0.0])),
    );

    let (members, _) = object_group(&material);
    let updates = members
        .iter()
        .filter(|member| matches!(member.ty, Type::Vec3))
        .count();
    assert_eq!(
        updates, 2,
        "two callbacks must not collapse into one member"
    );
}
