//! Port of `three.js/src/nodes/accessors/Skinning.js` — `skinning()`, the
//! statements `NodeMaterial.setupPosition()` runs after morphing and before
//! `positionNode`.
//!
//! Only the uniform-buffer branch of `getBoneMatricesNode()` is here: the bone
//! texture is what Three falls back to when `bones * 16 * 4` passes the
//! uniform buffer limit, and no skeleton on the ladder comes near it (Michelle
//! is 65 bones = 4160 bytes against 65536). The fallback is noted in
//! `docs/nodes.md` §8 as a gap, not a divergence — it changes nothing for a
//! skeleton that fits.

use std::rc::Rc;

use crate::nodes::node::{
    BufferId, BufferNode, BufferSource, Node, Type, UniformGroup, UniformSource,
};
use crate::nodes::tsl::*;
use crate::nodes::NodeRef;

/// What `skinning()` reads off the `SkinnedMesh` at setup time: the bone count,
/// which sizes the `array< mat4x4<f32>, N >` and so belongs in the render
/// object's cache key.
#[derive(Clone, Copy, Debug, Default, Hash, PartialEq, Eq)]
pub struct SkinEntry {
    /// `skeleton.bones.length`.
    pub bones: usize,
}

/// `referenceBuffer( 'skeleton.boneMatrices', 'mat4', skeleton.bones.length )`.
///
/// One buffer node, `element()`ed eight times — four in the position and four
/// in the normal — so the generated shader has exactly one binding.
fn bone_matrices(entry: &SkinEntry) -> Rc<BufferNode> {
    Rc::new(BufferNode {
        id: BufferId::next(),
        source: BufferSource::BoneMatrices,
        element_ty: Type::Mat4,
        count: entry.bones.max(1),
    })
}

fn bone(buffer: &Rc<BufferNode>, index: NodeRef) -> NodeRef {
    NodeRef::new(Node::BufferElement {
        buffer: buffer.clone(),
        index,
    })
}

/// `skinning( skinnedMesh )` — the vertex statements, in Three's order.
pub fn skinning(entry: &SkinEntry) -> Vec<NodeRef> {
    let skin_index = attribute("skinIndex", Type::UVec4);
    let skin_weight = attribute("skinWeight", Type::Vec4);
    // `reference( 'bindMatrix', 'mat4' )`: a plain object-group uniform here,
    // because the port's renderer resolves uniforms by source rather than by
    // walking a property path off the object.
    let bind_matrix = uniform(
        UniformSource::BindMatrix,
        Type::Mat4,
        UniformGroup::Object,
        None,
    );
    let bind_matrix_inverse = uniform(
        UniformSource::BindMatrixInverse,
        Type::Mat4,
        UniformGroup::Object,
        None,
    );

    let buffer = bone_matrices(entry);
    let mats = [
        bone(&buffer, skin_index.x()),
        bone(&buffer, skin_index.y()),
        bone(&buffer, skin_index.z()),
        bone(&buffer, skin_index.w()),
    ];
    let weights = [
        skin_weight.x(),
        skin_weight.y(),
        skin_weight.z(),
        skin_weight.w(),
    ];

    let mut statements = Vec::new();

    // --- getSkinnedPosition()
    //
    // `bindMatrix.mul( positionLocal )` — a `mat4` times a `vec3` is the `vec4`
    // `NodeBuilder.format()` pads with 1.0. Held in a var because Three's
    // `TempNode` caches it: it is read by all four bone terms.
    let skin_vertex = to_var(
        None,
        bind_matrix.mul(vec4_join(vec![position_local(), float(1.0)])),
    );

    // `boneMatX.mul( skinWeight.x ).mul( skinVertex )`. The matrix comes first
    // in the source and second in the generated WGSL: `OperatorNode.generate()`
    // swaps a matrix-times-float round. See `builder.rs`' `Node::Op`.
    let skinned = (0..4)
        .map(|i| mats[i].mul(weights[i].clone()).mul(skin_vertex.clone()))
        .reduce(|a, b| a.add(b))
        .expect("three-rs: four bone terms");

    statements.push(position_local().assign(bind_matrix_inverse.mul(skinned).xyz()));

    // --- getSkinnedNormalAndTangent()
    //
    // `skinWeight.x.mul( boneMatX )` — the same product the other way round,
    // which is the form that generates without enclosing parentheses. Three
    // builds the sum a second time here rather than reusing the position's, and
    // then re-emits the whole `mat4` once per column of the `mat3`; both are
    // reproduced so the statement matches the dump.
    let skin_matrix = (0..4)
        .map(|i| weights[i].mul(mats[i].clone()))
        .reduce(|a, b| a.add(b))
        .expect("three-rs: four bone terms");
    let skin_matrix = bind_matrix_inverse.mul(skin_matrix).mul(bind_matrix);

    let skin_matrix3 = join(
        Type::Mat3,
        vec![
            skin_matrix.element(0).xyz(),
            skin_matrix.element(1).xyz(),
            skin_matrix.element(2).xyz(),
        ],
    );

    // `builder.hasGeometryAttribute( 'normal' )`: every skinned geometry on the
    // ladder has one, and a material that does not read `normalLocal` drops the
    // statement in the builder's dead-code pass anyway.
    statements.push(normal_local().assign(skin_matrix3.mul(normal_local())));

    statements
}
