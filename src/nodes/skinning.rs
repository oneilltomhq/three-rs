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
    /// `builder.needsPreviousData()`: the renderer's MRT has a `velocity`
    /// output, so `positionPrevious` is skinned with last frame's bones as
    /// well. Part of the key, because it adds a binding.
    pub previous: bool,
}

/// `referenceBuffer( 'skeleton.boneMatrices', 'mat4', skeleton.bones.length )`.
///
/// One buffer node, `element()`ed eight times — four in the position and four
/// in the normal — so the generated shader has exactly one binding.
fn bone_matrices(entry: &SkinEntry, source: BufferSource) -> Rc<BufferNode> {
    Rc::new(BufferNode {
        id: BufferId::next(),
        source,
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

    let mut statements = Vec::new();

    // --- getPreviousSkinnedPosition()
    //
    // `positionPrevious.assign( getSkinnedPosition( previousBoneMatricesNode,
    // positionPrevious ) )`, ahead of the current skinning as in
    // `SkinningNode.setup()`. Its buffer is created first, so its binding
    // comes first too.
    if entry.previous {
        let previous = bone_matrices(entry, BufferSource::PreviousBoneMatrices);
        let mats = [
            bone(&previous, skin_index.x()),
            bone(&previous, skin_index.y()),
            bone(&previous, skin_index.z()),
            bone(&previous, skin_index.w()),
        ];
        let weights = [
            skin_weight.x(),
            skin_weight.y(),
            skin_weight.z(),
            skin_weight.w(),
        ];
        let skin_vertex = to_var(
            None,
            bind_matrix.mul(vec4_join(vec![position_previous(), float(1.0)])),
        );
        let skinned = (0..4)
            .map(|i| mats[i].mul(weights[i].clone()).mul(skin_vertex.clone()))
            .reduce(|a, b| a.add(b))
            .expect("three-rs: four bone terms");
        statements.push(position_previous().assign(bind_matrix_inverse.mul(skinned).xyz()));
    }

    let buffer = bone_matrices(entry, BufferSource::BoneMatrices);
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

/// `computeSkinning( skinnedMesh, toPosition = null )` — `SkinningNode` in
/// compute mode: `getSkinnedPosition()` over storage copies of the geometry's
/// `position`, `skinIndex` and `skinWeight` rather than vertex attributes,
/// indexed by `instanceIndex`. The value is the skinned position in the
/// mesh's local space (a `vec3`), for a kernel to write where it likes.
///
/// Three builds each storage buffer as `new InstancedBufferAttribute( array,
/// itemSize )` over the geometry attribute's own array, so every
/// `computeSkinning()` call gets its own GPU copies — `webgpu_skinning_points`
/// calls it twice per mesh (the per-frame kernel and its `onInit`) and the
/// dump has two sets of bindings. The same here.
///
/// `bindMatrix` / `bindMatrixInverse` are read off the mesh whenever the
/// kernel's object buffer is written, and the bone matrices come from the
/// mesh's skeleton, updated once per frame by the renderer
/// (`SkinningNode.update()`'s `skeleton.update()`), since the mesh itself is
/// usually hidden and never reaches the render list.
///
/// # Panics
///
/// If `mesh` is not a skinned mesh with a skeleton and skin attributes.
pub fn compute_skinning(mesh: &crate::core::ObjectRef) -> NodeRef {
    use crate::nodes::node::{LiveValue, SkeletonRef};
    use crate::objects::Payload;

    let (geometry, skeleton) = match &mesh.borrow().payload {
        Payload::SkinnedMesh(skinned) => (
            skinned.mesh.geometry.clone(),
            skinned
                .skeleton
                .clone()
                .expect("three-rs: computeSkinning() on a SkinnedMesh with no skeleton"),
        ),
        _ => panic!("three-rs: computeSkinning() needs a SkinnedMesh"),
    };
    let attribute = |name: &str| {
        geometry
            .get_attribute(name)
            .unwrap_or_else(|| panic!("three-rs: computeSkinning() needs `{name}`"))
    };
    // `storage( attribute, 'vec3' / 'vec4' )` reads floats; the port hands
    // the values over as floats whatever the array (a normalized `Uint16Array`
    // skinWeight from `BufferGeometryLoader`, an interleaved position), which
    // is what three's typed `getX()` would read.
    let position = storage_f32(&attribute("position").to_f32_items(), Type::Vec3).to_read_only();
    // `skinIndex` is a `Uint16Array` in three (a `Uint32Array` from the
    // port's glTF loader, a `Float32Array` from hand-built geometry);
    // `storage( …, 'uvec4' )` over it is a `u32` array on the GPU.
    let indices: Vec<u32> = {
        let data = attribute("skinIndex").data();
        (0..data.len()).map(|i| data.get(i) as u32).collect()
    };
    let skin_index = storage_data(&indices, Type::UVec4).to_read_only();
    let skin_weight =
        storage_f32(&attribute("skinWeight").to_f32_items(), Type::Vec4).to_read_only();

    let skinned = |read: fn(&crate::objects::SkinnedMesh) -> Vec<f64>| {
        let mesh = mesh.downgrade();
        uniform(
            UniformSource::Live(LiveValue::new(move || {
                let mesh = mesh
                    .upgrade()
                    .expect("three-rs: computeSkinning() outlived its mesh");
                let node = mesh.borrow();
                match &node.payload {
                    Payload::SkinnedMesh(skinned) => read(skinned),
                    _ => unreachable!("three-rs: checked above"),
                }
            })),
            Type::Mat4,
            UniformGroup::Object,
            None,
        )
    };
    let bind_matrix = skinned(|m| m.bind_matrix.elements.to_vec());
    let bind_matrix_inverse = skinned(|m| m.bind_matrix_inverse.elements.to_vec());

    let bones = skeleton.borrow().bones.len();
    let buffer = Rc::new(BufferNode {
        id: BufferId::next(),
        source: BufferSource::SkeletonBoneMatrices(SkeletonRef(skeleton)),
        element_ty: Type::Mat4,
        count: bones.max(1),
    });

    let index = to_var(None, skin_index.element(instance_index()));
    let weight = to_var(None, skin_weight.element(instance_index()));
    let position = to_var(None, position.element(instance_index()));

    let mats = [
        bone(&buffer, index.x()),
        bone(&buffer, index.y()),
        bone(&buffer, index.z()),
        bone(&buffer, index.w()),
    ];
    let weights = [weight.x(), weight.y(), weight.z(), weight.w()];

    // `getSkinnedPosition()`: in compute mode three's `skinVertex` is a plain
    // (cached) product, a `let` in the dump.
    let skin_vertex = bind_matrix.mul(vec4_join(vec![position, float(1.0)]));
    let skinned = (0..4)
        .map(|i| weights[i].mul(mats[i].clone()).mul(skin_vertex.clone()))
        .reduce(|a, b| a.add(b))
        .expect("three-rs: four bone terms");
    bind_matrix_inverse.mul(skinned).xyz()
}
