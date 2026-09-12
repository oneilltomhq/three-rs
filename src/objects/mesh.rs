//! Port of `three.js/src/objects/Mesh.js` (rung 1 subset).

use std::rc::Rc;

use crate::core::{BufferGeometry, Object3D};
use crate::materials::MeshBasicNodeMaterial;

pub struct Mesh {
    pub object: Object3D,
    pub geometry: Rc<BufferGeometry>,
    pub material: Option<MeshBasicNodeMaterial>,
}

impl Mesh {
    /// `new Mesh( geometry )` — the example relies on `scene.overrideMaterial`,
    /// so the material is optional here just as it defaults in three.js.
    pub fn new(geometry: Rc<BufferGeometry>) -> Self {
        Self {
            object: Object3D::default(),
            geometry,
            material: None,
        }
    }
}
