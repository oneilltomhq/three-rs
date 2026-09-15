//! Port of `three.js/src/cameras/OrthographicCamera.js` — the renderer's
//! internal `QuadMesh` camera, `new OrthographicCamera( -1, 1, 1, -1, 0, 1 )`.

use crate::core::Object3D;
use crate::math::{Box3, CoordinateSystem, Matrix4, Vector3};

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
    pub projection_matrix_inverse: Matrix4,
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
            projection_matrix_inverse: Matrix4::identity(),
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

        self.projection_matrix_inverse = self.projection_matrix;
        self.projection_matrix_inverse.invert();
    }

    /// Frame `box_` from the direction the camera already looks in, with
    /// `margin` of the frame left as slack (`0.0` is tight, `0.1` leaves 10 % of
    /// the half-frame free on the tightest side). Not a three.js method — see
    /// issue #51.
    ///
    /// An orthographic frame has no distance term, so this is the box's extent
    /// in the camera's own basis: `left`/`right` and `top`/`bottom` come from the
    /// lateral extents, and `near`/`far` from the extent along the view
    /// direction. The current frame's aspect ratio is kept by widening whichever
    /// pair is the less demanding of the two — the box is framed, never
    /// stretched — and the current `zoom` is folded into the planes, so the
    /// projection matrix comes out the same whatever the zoom was.
    ///
    /// The camera is also moved to stand one box diagonal clear of the near face,
    /// so `near` is positive and the whole box is between the planes.
    pub fn fit(&mut self, box_: &Box3, margin: f64) {
        if box_.is_empty() {
            return;
        }

        self.update_matrix_world();

        let (right, up, forward) = super::view_basis(&self.object.matrix_world);
        let offsets = super::corner_offsets(box_, &right, &up, &forward);

        let mut half_width = 0.0_f64;
        let mut half_height = 0.0_f64;
        let mut near_depth = f64::INFINITY;
        let mut far_depth = f64::NEG_INFINITY;

        for offset in &offsets {
            half_width = half_width.max(offset.x.abs());
            half_height = half_height.max(offset.y.abs());
            near_depth = near_depth.min(offset.z);
            far_depth = far_depth.max(offset.z);
        }

        // The margin grows the frame, so the tightest corner lands at
        // `1 - margin` in normalised device coordinates instead of at 1.
        let slack = (1.0 - margin).clamp(f64::EPSILON, 1.0);
        half_width /= slack;
        half_height /= slack;

        // Keep the frame's shape: widen the pair that has room to spare.
        let aspect = if self.top != self.bottom {
            (self.right - self.left) / (self.top - self.bottom)
        } else {
            1.0
        };
        if half_width < half_height * aspect {
            half_width = half_height * aspect;
        } else if aspect != 0.0 {
            half_height = half_width / aspect;
        }

        self.left = -half_width * self.zoom;
        self.right = half_width * self.zoom;
        self.top = half_height * self.zoom;
        self.bottom = -half_height * self.zoom;

        // One box diagonal of clearance in front, so `near` is positive whatever
        // the view direction is.
        let clearance = {
            let diagonal = box_.get_size().length();
            if diagonal > 0.0 {
                diagonal
            } else {
                1.0
            }
        };
        let distance = clearance - near_depth;
        self.near = clearance;
        self.far = distance + far_depth;
        if self.far <= self.near {
            // A box with no depth along the view direction.
            self.far = self.near * 2.0;
        }
        self.update_projection_matrix();

        let center = box_.get_center();
        let mut position = Vector3::ZERO;
        position
            .copy(&forward)
            .multiply_scalar(-distance)
            .add(&center);
        self.object.position = position;
        self.update_matrix_world();
    }

    /// `Object3D.lookAt()` for a camera: the matrix looks *from* the camera
    /// position *at* the target (`PerspectiveCamera::look_at` is the same on
    /// its node).
    pub fn look_at(&mut self, target: &Vector3) {
        let mut m = Matrix4::identity();
        m.look_at(&self.object.position, target, &self.object.up);
        self.object.quaternion.set_from_rotation_matrix(&m);
        self.object.sync_rotation_from_quaternion();
    }

    pub fn update_matrix_world(&mut self) {
        self.object.update_matrix_world(None);
        self.matrix_world_inverse = self.object.matrix_world;
        self.matrix_world_inverse.invert();
    }
}
