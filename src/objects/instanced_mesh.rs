//! Port of `three.js/src/objects/InstancedMesh.js` +
//! `src/core/InstancedBufferAttribute.js` (rung 2 subset).
//!
//! `InstancedMesh` extends `Mesh` in three.js; here it holds the `Mesh` payload
//! it extends, and the node's [`Payload`] variant keeps the two apart for the
//! renderer.

use std::rc::Rc;

use crate::core::{BufferGeometry, Node, Object3D};
use crate::materials::MeshBasicNodeMaterial;
use crate::math::Matrix4;
use crate::objects::{Mesh, Payload};

/// `new InstancedBufferAttribute( new Float32Array( count * 16 ), 16 )`.
#[derive(Clone, Debug)]
pub struct InstancedBufferAttribute {
    pub array: Vec<f32>,
    pub item_size: usize,
}

impl InstancedBufferAttribute {
    pub fn new(array: Vec<f32>, item_size: usize) -> Self {
        Self { array, item_size }
    }

    pub fn count(&self) -> usize {
        self.array.len() / self.item_size
    }
}

#[derive(Clone)]
pub struct InstancedMesh {
    pub mesh: Mesh,
    /// `InstancedMesh.count` — the number of instances the renderer draws.
    pub count: usize,
    pub instance_matrix: InstancedBufferAttribute,
    /// `InstancedMesh.instanceColor`, `null` until `setColorAt()` is called.
    pub instance_color: Option<InstancedBufferAttribute>,
}

impl InstancedMesh {
    /// `new InstancedMesh( geometry, material, count )`, as a scene-graph [`Node`].
    pub fn new(
        geometry: Rc<BufferGeometry>,
        material: MeshBasicNodeMaterial,
        count: usize,
    ) -> Node {
        let mut object = Object3D::default();
        object.object_type = "InstancedMesh";
        object.payload = Payload::InstancedMesh(Self {
            mesh: Mesh {
                geometry,
                material: Some(material),
                morph_target_influences: Vec::new(),
            },
            count,
            instance_matrix: InstancedBufferAttribute::new(vec![0.0; count * 16], 16),
            instance_color: None,
        });
        object.into_node()
    }

    /// `InstancedMesh.setMatrixAt( index, matrix )` —
    /// `matrix.toArray( this.instanceMatrix.array, index * 16 )`.
    pub fn set_matrix_at(&mut self, index: usize, matrix: &Matrix4) {
        let offset = index * 16;
        let elements = matrix.to_f32_array();
        self.instance_matrix.array[offset..offset + 16].copy_from_slice(&elements);
    }
}
