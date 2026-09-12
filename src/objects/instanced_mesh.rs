//! Port of `three.js/src/objects/InstancedMesh.js` +
//! `src/core/InstancedBufferAttribute.js` (rung 2 subset).
//!
//! `InstancedMesh` extends `Mesh` in three.js; here it wraps one, and
//! `Scene`'s child enum keeps the two apart for the renderer.

use std::rc::Rc;

use crate::core::BufferGeometry;
use crate::materials::MeshBasicNodeMaterial;
use crate::math::Matrix4;
use crate::objects::Mesh;

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

pub struct InstancedMesh {
    pub mesh: Mesh,
    /// `InstancedMesh.count` — the number of instances the renderer draws.
    pub count: usize,
    pub instance_matrix: InstancedBufferAttribute,
    /// `InstancedMesh.instanceColor`, `null` until `setColorAt()` is called.
    pub instance_color: Option<InstancedBufferAttribute>,
}

impl InstancedMesh {
    /// `new InstancedMesh( geometry, material, count )`.
    pub fn new(geometry: Rc<BufferGeometry>, material: MeshBasicNodeMaterial, count: usize) -> Self {
        let mut mesh = Mesh::new(geometry);
        mesh.material = Some(material);

        Self {
            mesh,
            count,
            instance_matrix: InstancedBufferAttribute::new(vec![0.0; count * 16], 16),
            instance_color: None,
        }
    }

    /// `InstancedMesh.setMatrixAt( index, matrix )` —
    /// `matrix.toArray( this.instanceMatrix.array, index * 16 )`.
    pub fn set_matrix_at(&mut self, index: usize, matrix: &Matrix4) {
        let offset = index * 16;
        let elements = matrix.to_f32_array();
        self.instance_matrix.array[offset..offset + 16].copy_from_slice(&elements);
    }
}
