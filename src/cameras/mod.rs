//! Ports of `three.js/src/cameras`.

mod orthographic_camera;
mod perspective_camera;

pub use orthographic_camera::OrthographicCamera;
pub use perspective_camera::{CameraView, PerspectiveCamera};

use crate::core::Layers;
use crate::math::{Box3, CoordinateSystem, Matrix4, Vector3};

/// The slice of `Camera` that `Renderer.render()` and `_projectObject()`
/// actually read: `updateMatrixWorld()`, `projectionMatrix`,
/// `matrixWorldInverse`, `matrixWorld`, `layers` and `coordinateSystem`.
///
/// `Vector3::project()`/`unproject()` read the same four matrix accessors, and
/// `Raycaster.setFromCamera()` reads them and the type flags; nothing here is
/// renderer-only.
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
    /// `camera.far` — the sort key scale `BatchedMesh`' custom sort uses.
    fn far(&self) -> f64;
    /// `camera.near` — `LineSegments2`' screen-space raycast clips to it.
    fn near(&self) -> f64;
    /// `camera.isPerspectiveCamera`.
    fn is_perspective_camera(&self) -> bool {
        false
    }
    /// `camera.isOrthographicCamera`.
    fn is_orthographic_camera(&self) -> bool {
        false
    }
}

impl RenderCamera for PerspectiveCamera {
    fn far(&self) -> f64 {
        self.far
    }
    fn near(&self) -> f64 {
        self.near
    }
    fn is_perspective_camera(&self) -> bool {
        true
    }
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
    fn far(&self) -> f64 {
        self.far
    }
    fn near(&self) -> f64 {
        self.near
    }
    fn is_orthographic_camera(&self) -> bool {
        true
    }
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

/// The camera's world basis, for [`PerspectiveCamera::fit`] and
/// [`OrthographicCamera::fit`]: the x and y columns of `matrix_world`, and
/// *minus* its z column, which is the direction a camera looks along. All three
/// are normalised, so a scaled camera node still gives a unit basis.
fn view_basis(matrix_world: &Matrix4) -> (Vector3, Vector3, Vector3) {
    let mut right = Vector3::ZERO;
    let mut up = Vector3::ZERO;
    let mut forward = Vector3::ZERO;
    matrix_world.extract_basis(&mut right, &mut up, &mut forward);

    right.normalize();
    up.normalize();
    forward.normalize().negate();

    (right, up, forward)
}

/// The box's eight corners as offsets from its centre, resolved in a camera
/// basis: `x` is the offset along `right`, `y` along `up`, `z` along `forward`
/// — that is, how much deeper than the centre the corner lies.
fn corner_offsets(box_: &Box3, right: &Vector3, up: &Vector3, forward: &Vector3) -> [Vector3; 8] {
    let center = box_.get_center();
    let mut offsets = [Vector3::ZERO; 8];

    for (i, offset) in offsets.iter_mut().enumerate() {
        // The same binary pattern `Box3::apply_matrix4()` uses.
        let corner = Vector3::new(
            if i & 4 == 0 { box_.min.x } else { box_.max.x },
            if i & 2 == 0 { box_.min.y } else { box_.max.y },
            if i & 1 == 0 { box_.min.z } else { box_.max.z },
        );

        let mut d = Vector3::ZERO;
        d.sub_vectors(&corner, &center);

        offset.set(d.dot(right), d.dot(up), d.dot(forward));
    }

    offsets
}
