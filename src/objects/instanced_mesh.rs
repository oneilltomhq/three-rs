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
#[derive(Clone)]
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
    #[allow(clippy::new_ret_no_self)] // `new` mirrors three.js's constructor and returns a scene-graph `Node`, not `Self`; public API, not changing.
    pub fn new(
        geometry: Rc<BufferGeometry>,
        material: MeshBasicNodeMaterial,
        count: usize,
    ) -> Node {
        let mut object = Object3D {
            object_type: "InstancedMesh",
            ..Default::default()
        };
        let mut instanced = Self {
            mesh: Mesh {
                geometry,
                material: Some(material),
                morph_target_influences: Vec::new(),
            },
            count,
            instance_matrix: InstancedBufferAttribute::new(vec![0.0; count * 16], 16),
            instance_color: None,
        };
        // `for ( let i = 0; i < count; i ++ ) this.setMatrixAt( i, _identity );`
        // — the constructor's last step. A mesh whose instances are never placed
        // still draws at the origin, not collapsed to a zero matrix.
        let identity = Matrix4::identity();
        for i in 0..count {
            instanced.set_matrix_at(i, &identity);
        }
        object.payload = Payload::InstancedMesh(instanced);
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

/// The array is `count * item_size` floats — sixteen per instance for the
/// instance matrix. Debug prints its length, as the textures do.
impl std::fmt::Debug for InstancedBufferAttribute {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("InstancedBufferAttribute")
            .field("array", &format_args!("{} floats", self.array.len()))
            .field("item_size", &self.item_size)
            .finish()
    }
}
