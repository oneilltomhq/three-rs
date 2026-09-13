//! Ports of `three.js/src/cameras`.

mod orthographic_camera;
mod perspective_camera;

pub use orthographic_camera::OrthographicCamera;
pub use perspective_camera::{CameraView, PerspectiveCamera};

use crate::core::Object3D;
use crate::math::{CoordinateSystem, Matrix4};

/// The slice of `Camera` that `Renderer.render()` and `_projectObject()`
/// actually read: `updateMatrixWorld()`, `projectionMatrix`,
/// `matrixWorldInverse`, `matrixWorld`, `layers` and `coordinateSystem`.
///
/// three.js has a real `Camera extends Object3D` base class that both cameras
/// extend; the port grew `PerspectiveCamera` first and kept them as separate
/// structs, so this trait is the base class's render-facing surface rather than
/// a new idea. It exists because lib3's SDF text page — and d33's, and three's
/// own `QuadMesh` path — use an `OrthographicCamera`, and `render()` took a
/// `&mut PerspectiveCamera` by name.
pub trait RenderCamera {
    /// `camera.updateMatrixWorld()`, which also refreshes `matrixWorldInverse`.
    fn update_matrix_world(&mut self);
    /// `camera.projectionMatrix`.
    fn projection_matrix(&self) -> Matrix4;
    /// `camera.matrixWorldInverse` — the view matrix.
    fn matrix_world_inverse(&self) -> Matrix4;
    /// `camera.coordinateSystem`.
    fn coordinate_system(&self) -> CoordinateSystem;
    /// The camera as an `Object3D`, for `matrixWorld` and `layers`.
    fn object(&self) -> &Object3D;
}

impl RenderCamera for PerspectiveCamera {
    fn update_matrix_world(&mut self) {
        self.update_matrix_world();
    }
    fn projection_matrix(&self) -> Matrix4 {
        self.projection_matrix
    }
    fn matrix_world_inverse(&self) -> Matrix4 {
        self.matrix_world_inverse
    }
    fn coordinate_system(&self) -> CoordinateSystem {
        self.coordinate_system
    }
    fn object(&self) -> &Object3D {
        &self.object
    }
}

impl RenderCamera for OrthographicCamera {
    fn update_matrix_world(&mut self) {
        self.update_matrix_world();
    }
    fn projection_matrix(&self) -> Matrix4 {
        self.projection_matrix
    }
    fn matrix_world_inverse(&self) -> Matrix4 {
        self.matrix_world_inverse
    }
    fn coordinate_system(&self) -> CoordinateSystem {
        self.coordinate_system
    }
    fn object(&self) -> &Object3D {
        &self.object
    }
}
