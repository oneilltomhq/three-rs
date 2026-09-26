//! Port of `three.js/src/cameras/ArrayCamera.js`.

use std::ops::{Deref, DerefMut};

use super::{PerspectiveCamera, RenderCamera};
use crate::core::Layers;
use crate::math::{CoordinateSystem, Matrix4};

/// `ArrayCamera` — a `PerspectiveCamera` holding sub-cameras that one
/// `render()` call draws the scene through, each into its own
/// [`viewport`](PerspectiveCamera::viewport).
///
/// three.js' `ArrayCamera extends PerspectiveCamera`; the port derefs to the
/// base camera, which is what the render list's sort and the frustum array's
/// fallback read. The sub-cameras are not its children: their matrices are
/// the application's to update, as `webgpu_camera_array` does with
/// `subcamera.updateMatrixWorld()`.
///
/// `isMultiViewCamera` (WebXR's `OVR_multiview2`) is not ported.
#[derive(Clone)]
pub struct ArrayCamera {
    pub camera: PerspectiveCamera,
    /// `camera.cameras`.
    pub cameras: Vec<PerspectiveCamera>,
}

impl ArrayCamera {
    /// `new ArrayCamera( array )` — the base camera is `new
    /// PerspectiveCamera()`'s defaults.
    pub fn new(cameras: Vec<PerspectiveCamera>) -> Self {
        Self {
            camera: PerspectiveCamera::new(50.0, 1.0, 0.1, 2000.0),
            cameras,
        }
    }
}

impl Deref for ArrayCamera {
    type Target = PerspectiveCamera;
    fn deref(&self) -> &PerspectiveCamera {
        &self.camera
    }
}

impl DerefMut for ArrayCamera {
    fn deref_mut(&mut self) -> &mut PerspectiveCamera {
        &mut self.camera
    }
}

impl RenderCamera for ArrayCamera {
    fn far(&self) -> f64 {
        self.camera.far
    }
    fn near(&self) -> f64 {
        self.camera.near
    }
    fn is_perspective_camera(&self) -> bool {
        true
    }
    fn update_matrix_world(&mut self) {
        self.camera.update_matrix_world();
    }
    /// `ArrayCamera` inherits `PerspectiveCamera.setViewOffset`, so the
    /// offset lands on the outer camera; three's sub-cameras keep their own
    /// projections, and so do these.
    fn set_view_offset(
        &mut self,
        full_width: f64,
        full_height: f64,
        x: f64,
        y: f64,
        width: f64,
        height: f64,
    ) {
        self.camera
            .set_view_offset(full_width, full_height, x, y, width, height);
    }
    fn clear_view_offset(&mut self) {
        self.camera.clear_view_offset();
    }
    fn projection_matrix(&self) -> Matrix4 {
        self.camera.projection_matrix
    }
    fn projection_matrix_inverse(&self) -> Matrix4 {
        self.camera.projection_matrix_inverse
    }
    fn matrix_world_inverse(&self) -> Matrix4 {
        self.camera.matrix_world_inverse
    }
    fn coordinate_system(&self) -> CoordinateSystem {
        self.camera.coordinate_system
    }
    fn matrix_world(&self) -> Matrix4 {
        self.camera.node.borrow().matrix_world
    }
    fn layers(&self) -> Layers {
        self.camera.node.borrow().layers
    }
    fn id(&self) -> u32 {
        self.camera.node.borrow().id
    }
    fn sub_cameras(&self) -> &[PerspectiveCamera] {
        &self.cameras
    }
}
