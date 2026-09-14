//! Ports of `three.js/src/cameras`.

mod orthographic_camera;
mod perspective_camera;

pub use orthographic_camera::OrthographicCamera;
pub use perspective_camera::{CameraView, PerspectiveCamera};

use crate::core::Layers;
use crate::math::{CoordinateSystem, Matrix4};

/// The slice of `Camera` that `Renderer.render()` and `_projectObject()`
/// actually read: `updateMatrixWorld()`, `projectionMatrix`,
/// `matrixWorldInverse`, `matrixWorld`, `layers` and `coordinateSystem`.
///
/// `Vector3::project()`/`unproject()` read the same four matrix accessors, and
/// `Raycaster.setFromCamera()` will read them too when it is ported; nothing
/// here is renderer-only.
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
    /// `camera.projectionMatrixInverse`, kept current by
    /// `update_projection_matrix()` on both cameras.
    fn projection_matrix_inverse(&self) -> Matrix4;
    /// `camera.matrixWorldInverse` — the view matrix.
    fn matrix_world_inverse(&self) -> Matrix4;
    /// `camera.coordinateSystem`.
    fn coordinate_system(&self) -> CoordinateSystem;
    /// `camera.matrixWorld`.
    fn matrix_world(&self) -> Matrix4;
    /// `camera.layers`.
    fn layers(&self) -> Layers;
}

impl RenderCamera for PerspectiveCamera {
    fn update_matrix_world(&mut self) {
        self.update_matrix_world();
    }
    fn projection_matrix(&self) -> Matrix4 {
        self.projection_matrix
    }
    fn projection_matrix_inverse(&self) -> Matrix4 {
        self.projection_matrix_inverse
    }
    fn matrix_world_inverse(&self) -> Matrix4 {
        self.matrix_world_inverse
    }
    fn coordinate_system(&self) -> CoordinateSystem {
        self.coordinate_system
    }
    fn matrix_world(&self) -> Matrix4 {
        self.node.borrow().matrix_world
    }
    fn layers(&self) -> Layers {
        self.node.borrow().layers
    }
}

impl RenderCamera for OrthographicCamera {
    fn update_matrix_world(&mut self) {
        self.update_matrix_world();
    }
    fn projection_matrix(&self) -> Matrix4 {
        self.projection_matrix
    }
    fn projection_matrix_inverse(&self) -> Matrix4 {
        self.projection_matrix_inverse
    }
    fn matrix_world_inverse(&self) -> Matrix4 {
        self.matrix_world_inverse
    }
    fn coordinate_system(&self) -> CoordinateSystem {
        self.coordinate_system
    }
    fn matrix_world(&self) -> Matrix4 {
        self.object.matrix_world
    }
    fn layers(&self) -> Layers {
        self.object.layers
    }
}
