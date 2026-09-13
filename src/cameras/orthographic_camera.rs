//! Port of `three.js/src/cameras/OrthographicCamera.js` — the renderer's
//! internal `QuadMesh` camera, `new OrthographicCamera( -1, 1, 1, -1, 0, 1 )`.

use crate::core::Object3D;
use crate::math::{CoordinateSystem, Matrix4};

#[derive(Clone)]
pub struct OrthographicCamera {
    pub object: Object3D,
    pub left: f64,
    pub right: f64,
    pub top: f64,
    pub bottom: f64,
    pub near: f64,
    pub far: f64,
    pub zoom: f64,
    pub coordinate_system: CoordinateSystem,
    pub projection_matrix: Matrix4,
    pub matrix_world_inverse: Matrix4,
}

impl OrthographicCamera {
    pub fn new(left: f64, right: f64, top: f64, bottom: f64, near: f64, far: f64) -> Self {
        let mut camera = Self {
            object: Object3D::default(),
            left,
            right,
            top,
            bottom,
            near,
            far,
            zoom: 1.0,
            coordinate_system: CoordinateSystem::WebGPU,
            projection_matrix: Matrix4::identity(),
            matrix_world_inverse: Matrix4::identity(),
        };
        camera.update_projection_matrix();
        camera.update_matrix_world();
        camera
    }

    /// `OrthographicCamera.updateProjectionMatrix()` with no view offset.
    pub fn update_projection_matrix(&mut self) {
        let dx = (self.right - self.left) / (2.0 * self.zoom);
        let dy = (self.top - self.bottom) / (2.0 * self.zoom);
        let cx = (self.right + self.left) / 2.0;
        let cy = (self.top + self.bottom) / 2.0;

        self.projection_matrix.make_orthographic(
            cx - dx,
            cx + dx,
            cy + dy,
            cy - dy,
            self.near,
            self.far,
            self.coordinate_system,
        );
    }

    pub fn update_matrix_world(&mut self) {
        self.object.update_matrix_world(None);
        self.matrix_world_inverse = self.object.matrix_world;
        self.matrix_world_inverse.invert();
    }
}
