//! Port of `three.js/src/renderers/common/QuadMesh.js` (rung 1 subset).
//!
//! three.js draws a single oversized triangle with clip-space positions
//! `(-1, 3)`, `(-1, -1)`, `(3, -1)` supplied by `QuadMesh.render()`'s
//! `vertexNode`, and uvs `(0, -1)`, `(0, 1)`, `(2, 1)` from the geometry. Those
//! constants live in the quad WGSL (`src/renderer/shaders/quad.wgsl`).

use crate::materials::MeshBasicNodeMaterial;

pub struct QuadMesh {
    pub material: MeshBasicNodeMaterial,
}

impl QuadMesh {
    pub fn new(material: MeshBasicNodeMaterial) -> Self {
        Self { material }
    }
}
