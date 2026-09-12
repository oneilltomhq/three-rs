//! `QuadGeometry`, the private geometry in
//! `three.js/src/renderers/common/QuadMesh.js`: one oversized triangle that
//! covers the viewport.

use crate::core::{BufferAttribute, BufferGeometry};

/// `new QuadGeometry( flipY = false )`.
pub fn quad_geometry() -> BufferGeometry {
    let mut geometry = BufferGeometry::new();
    geometry.position = Some(BufferAttribute::new(
        vec![-1.0, 3.0, 0.0, -1.0, -1.0, 0.0, 3.0, -1.0, 0.0],
        3,
    ));
    geometry.uv = Some(BufferAttribute::new(vec![0.0, -1.0, 0.0, 1.0, 2.0, 1.0], 2));
    geometry
}
