//! Port of `three.js/src/renderers/common/QuadMesh.js` (rung 1 subset).
//!
//! three.js draws a single oversized triangle with clip-space positions
//! `(-1, 3)`, `(-1, -1)`, `(3, -1)` supplied by `QuadMesh.render()`'s
//! `vertexNode`, and uvs `(0, -1)`, `(0, 1)`, `(2, 1)` from the geometry. Those
//! constants live in the quad WGSL (`src/renderer/shaders/quad.wgsl`).

use crate::materials::MeshBasicNodeMaterial;

/// `QuadMesh` — draws one oversized triangle covering the full screen, for a
/// post-processing pass.
pub struct QuadMesh {
    /// The material the full-screen triangle is drawn with.
    pub material: MeshBasicNodeMaterial,
}

impl QuadMesh {
    /// `new QuadMesh( material )`.
    pub fn new(material: MeshBasicNodeMaterial) -> Self {
        Self { material }
    }
}
