//! Port of `three.js/src/cameras/PerspectiveCamera.js` + `Camera.js`
//! (rung 1 subset).

use crate::core::Object3D;
use crate::math::{CoordinateSystem, Matrix4, Vector3, DEG2RAD};

pub struct PerspectiveCamera {
    pub object: Object3D,
    pub fov: f64,
    pub aspect: f64,
    pub near: f64,
    pub far: f64,
    pub zoom: f64,
    pub film_gauge: f64,
    pub film_offset: f64,
    pub coordinate_system: CoordinateSystem,
    pub projection_matrix: Matrix4,
    pub projection_matrix_inverse: Matrix4,
    pub matrix_world_inverse: Matrix4,
}

impl PerspectiveCamera {
    pub fn new(fov: f64, aspect: f64, near: f64, far: f64) -> Self {
        let mut camera = Self {
            object: Object3D::default(),
            fov,
            aspect,
            near,
            far,
            zoom: 1.0,
            film_gauge: 35.0,
            film_offset: 0.0,
            // `WebGPURenderer` sets `camera.coordinateSystem` to `WebGPUCoordinateSystem`.
            coordinate_system: CoordinateSystem::WebGPU,
            projection_matrix: Matrix4::identity(),
            projection_matrix_inverse: Matrix4::identity(),
            matrix_world_inverse: Matrix4::identity(),
        };
        camera.update_projection_matrix();
        camera
    }

    /// `PerspectiveCamera.updateProjectionMatrix()`.
    pub fn update_projection_matrix(&mut self) {
        let near = self.near;
        let top = near * (DEG2RAD * 0.5 * self.fov).tan() / self.zoom;
        let height = 2.0 * top;
        let width = self.aspect * height;
        let left = -0.5 * width;

        self.projection_matrix.make_perspective(
            left,
            left + width,
            top,
            top - height,
            near,
            self.far,
            self.coordinate_system,
        );

        self.projection_matrix_inverse = self.projection_matrix;
        self.projection_matrix_inverse.invert();
    }

    /// `Object3D.lookAt()` for a camera: the matrix looks *from* the camera
    /// position *at* the target.
    pub fn look_at(&mut self, target: &Vector3) {
        let mut m = Matrix4::identity();
        m.look_at(&self.object.position, target, &self.object.up);
        self.object.quaternion.set_from_rotation_matrix(&m);
    }

    /// `Camera.updateMatrixWorld()`.
    pub fn update_matrix_world(&mut self) {
        self.object.update_matrix_world(None);
        self.matrix_world_inverse = self.object.matrix_world;
        self.matrix_world_inverse.invert();
    }
}
