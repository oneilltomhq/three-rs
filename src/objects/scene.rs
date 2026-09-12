//! Port of `three.js/src/scenes/Scene.js` (rung 2 subset).

use crate::core::Object3D;
use crate::materials::MeshBasicNodeMaterial;
use crate::math::{Color, Matrix4};
use crate::objects::{InstancedBufferAttribute, InstancedMesh, Mesh};

/// One entry of `Object3D.children`. `InstancedMesh` extends `Mesh` in
/// three.js; the renderer walks children through the accessors below and only
/// looks at `instance_matrix()` to decide whether to draw instanced.
pub enum Child {
    Mesh(Mesh),
    InstancedMesh(InstancedMesh),
}

impl From<Mesh> for Child {
    fn from(mesh: Mesh) -> Self {
        Child::Mesh(mesh)
    }
}

impl From<InstancedMesh> for Child {
    fn from(mesh: InstancedMesh) -> Self {
        Child::InstancedMesh(mesh)
    }
}

impl Child {
    pub fn mesh(&self) -> &Mesh {
        match self {
            Child::Mesh(mesh) => mesh,
            Child::InstancedMesh(instanced) => &instanced.mesh,
        }
    }

    pub fn mesh_mut(&mut self) -> &mut Mesh {
        match self {
            Child::Mesh(mesh) => mesh,
            Child::InstancedMesh(instanced) => &mut instanced.mesh,
        }
    }

    pub fn object(&self) -> &Object3D {
        &self.mesh().object
    }

    pub fn object_mut(&mut self) -> &mut Object3D {
        &mut self.mesh_mut().object
    }

    /// The number of instances to draw: `InstancedMesh.count`, else 1.
    pub fn count(&self) -> u32 {
        match self {
            Child::Mesh(_) => 1,
            Child::InstancedMesh(instanced) => instanced.count as u32,
        }
    }

    pub fn instance_matrix(&self) -> Option<&InstancedBufferAttribute> {
        match self {
            Child::Mesh(_) => None,
            Child::InstancedMesh(instanced) => Some(&instanced.instance_matrix),
        }
    }
}

#[derive(Default)]
pub struct Scene {
    pub children: Vec<Child>,
    pub background: Option<Color>,
    pub override_material: Option<MeshBasicNodeMaterial>,
    pub matrix_world: Matrix4,
}

impl Scene {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add(&mut self, child: impl Into<Child>) {
        self.children.push(child.into());
    }

    /// `Object3D.updateMatrixWorld()` on the scene root: the scene's own world
    /// matrix stays the identity and each child is composed then multiplied by it.
    pub fn update_matrix_world(&mut self) {
        let parent = self.matrix_world;
        for child in &mut self.children {
            child.object_mut().update_matrix_world(Some(&parent));
        }
    }
}
