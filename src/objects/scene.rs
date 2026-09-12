//! Port of `three.js/src/scenes/Scene.js` (rung 1 subset).

use crate::materials::MeshBasicNodeMaterial;
use crate::math::{Color, Matrix4};
use crate::objects::Mesh;

#[derive(Default)]
pub struct Scene {
    pub children: Vec<Mesh>,
    pub background: Option<Color>,
    pub override_material: Option<MeshBasicNodeMaterial>,
    pub matrix_world: Matrix4,
}

impl Scene {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add(&mut self, mesh: Mesh) {
        self.children.push(mesh);
    }

    /// `Object3D.updateMatrixWorld()` on the scene root: the scene's own world
    /// matrix stays the identity and each child is composed then multiplied by it.
    pub fn update_matrix_world(&mut self) {
        let parent = self.matrix_world;
        for child in &mut self.children {
            child.object.update_matrix_world(Some(&parent));
        }
    }
}
