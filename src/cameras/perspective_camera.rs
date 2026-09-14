//! Port of `three.js/src/cameras/PerspectiveCamera.js` + `Camera.js`
//! (rung 1 subset).

use crate::core::{Node, Object3D};
use crate::math::math_utils::{js_max, js_min};
use crate::math::{CoordinateSystem, Matrix4, Vector2, Vector3, DEG2RAD, RAD2DEG};

/// `PerspectiveCamera.view` — the frustum window specification set by
/// [`PerspectiveCamera::set_view_offset`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CameraView {
    pub enabled: bool,
    pub full_width: f64,
    pub full_height: f64,
    pub offset_x: f64,
    pub offset_y: f64,
    pub width: f64,
    pub height: f64,
}

#[derive(Clone)]
pub struct PerspectiveCamera {
    /// The camera's own scene-graph node. `Camera` is an `Object3D` in three.js
    /// and examples nest it (`scene.add( camera )`, `camera.add( light )`), so it
    /// owns a [`Node`] rather than a bare `Object3D`.
    pub node: Node,
    pub fov: f64,
    pub aspect: f64,
    pub near: f64,
    pub far: f64,
    pub zoom: f64,
    /// Object distance used for stereoscopy and depth-of-field effects. Does not
    /// influence the projection matrix.
    pub focus: f64,
    pub view: Option<CameraView>,
    pub film_gauge: f64,
    pub film_offset: f64,
    pub coordinate_system: CoordinateSystem,
    pub projection_matrix: Matrix4,
    pub projection_matrix_inverse: Matrix4,
    pub matrix_world_inverse: Matrix4,
}

impl PerspectiveCamera {
    pub fn new(fov: f64, aspect: f64, near: f64, far: f64) -> Self {
        // `Camera.isCamera` — `Object3D.lookAt()` branches on it.
        let object = Object3D {
            object_type: "PerspectiveCamera",
            is_camera: true,
            ..Default::default()
        };

        let mut camera = Self {
            node: object.into_node(),
            fov,
            aspect,
            near,
            far,
            zoom: 1.0,
            focus: 10.0,
            view: None,
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

    /// `PerspectiveCamera.setFocalLength()`.
    pub fn set_focal_length(&mut self, focal_length: f64) {
        // see http://www.bobatkins.com/photography/technical/field_of_view.html
        let v_extent_slope = 0.5 * self.get_film_height() / focal_length;

        self.fov = RAD2DEG * 2.0 * v_extent_slope.atan();
        self.update_projection_matrix();
    }

    /// `PerspectiveCamera.getFocalLength()`.
    pub fn get_focal_length(&self) -> f64 {
        let v_extent_slope = (DEG2RAD * 0.5 * self.fov).tan();

        0.5 * self.get_film_height() / v_extent_slope
    }

    /// `PerspectiveCamera.getEffectiveFOV()`.
    pub fn get_effective_fov(&self) -> f64 {
        RAD2DEG * 2.0 * ((DEG2RAD * 0.5 * self.fov).tan() / self.zoom).atan()
    }

    /// `PerspectiveCamera.getFilmWidth()`.
    pub fn get_film_width(&self) -> f64 {
        // film not completely covered in portrait format (aspect < 1)
        self.film_gauge * js_min(self.aspect, 1.0)
    }

    /// `PerspectiveCamera.getFilmHeight()`.
    pub fn get_film_height(&self) -> f64 {
        // film not completely covered in landscape format (aspect > 1)
        self.film_gauge / js_max(self.aspect, 1.0)
    }

    /// `PerspectiveCamera.getViewBounds()`.
    pub fn get_view_bounds(
        &self,
        distance: f64,
        min_target: &mut Vector2,
        max_target: &mut Vector2,
    ) {
        let mut v3 = Vector3::new(-1.0, -1.0, 0.5);
        v3.apply_matrix4(&self.projection_matrix_inverse);
        min_target.set(v3.x, v3.y).multiply_scalar(-distance / v3.z);

        let mut v3 = Vector3::new(1.0, 1.0, 0.5);
        v3.apply_matrix4(&self.projection_matrix_inverse);
        max_target.set(v3.x, v3.y).multiply_scalar(-distance / v3.z);
    }

    /// `PerspectiveCamera.getViewSize()`.
    pub fn get_view_size(&self, distance: f64) -> Vector2 {
        let mut min_target = Vector2::default();
        let mut max_target = Vector2::default();
        self.get_view_bounds(distance, &mut min_target, &mut max_target);

        let mut target = Vector2::default();
        target.sub_vectors(&max_target, &min_target);
        target
    }

    /// `PerspectiveCamera.setViewOffset()`.
    pub fn set_view_offset(
        &mut self,
        full_width: f64,
        full_height: f64,
        x: f64,
        y: f64,
        width: f64,
        height: f64,
    ) {
        self.aspect = full_width / full_height;

        self.view = Some(CameraView {
            enabled: true,
            full_width,
            full_height,
            offset_x: x,
            offset_y: y,
            width,
            height,
        });

        self.update_projection_matrix();
    }

    /// `PerspectiveCamera.clearViewOffset()`.
    pub fn clear_view_offset(&mut self) {
        if let Some(view) = self.view.as_mut() {
            view.enabled = false;
        }

        self.update_projection_matrix();
    }

    /// `PerspectiveCamera.updateProjectionMatrix()`.
    pub fn update_projection_matrix(&mut self) {
        let near = self.near;
        let mut top = near * (DEG2RAD * 0.5 * self.fov).tan() / self.zoom;
        let mut height = 2.0 * top;
        let mut width = self.aspect * height;
        let mut left = -0.5 * width;

        if let Some(view) = self.view {
            if view.enabled {
                let full_width = view.full_width;
                let full_height = view.full_height;

                left += view.offset_x * width / full_width;
                top -= view.offset_y * height / full_height;
                width *= view.width / full_width;
                height *= view.height / full_height;
            }
        }

        let skew = self.film_offset;
        if skew != 0.0 {
            left += near * skew / self.get_film_width();
        }

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

    /// Move the camera along the direction it already looks in, until `box_`
    /// fills the frame with `margin` of it left as slack (`0.0` is tight, `0.1`
    /// leaves 10 % of the half-frame free on the tightest side). Not a three.js
    /// method — see issue #51.
    ///
    /// The view *direction* is the camera's own, taken from `matrix_world`; only
    /// the position moves, along that direction, and `near`/`far` are set to the
    /// box's extent along it. `fov`, `aspect` and `zoom` are read, not written,
    /// so the caller's framing choices survive the fit.
    ///
    /// The distance is the one that actually frames the box: a bounding sphere
    /// wastes the corners, and so does taking the eight corners' *lateral*
    /// extent, because a corner nearer the camera needs more distance than a far
    /// one at the same offset. For each corner this asks what distance `d` puts
    /// it exactly on a frustum plane — `|x| / tan_h - depth` horizontally,
    /// `|y| / tan_v - depth` vertically, with `depth` the corner's offset along
    /// the view direction from the box centre — and takes the largest of the
    /// sixteen answers.
    ///
    /// A view offset ([`set_view_offset`](Self::set_view_offset)) is ignored;
    /// the fit is to the full frustum.
    pub fn fit(&mut self, box_: &crate::math::Box3, margin: f64) {
        if box_.is_empty() {
            return;
        }

        self.update_matrix_world();

        let matrix_world = self.node.borrow().matrix_world;
        let (right, up, forward) = super::view_basis(&matrix_world);
        let offsets = super::corner_offsets(box_, &right, &up, &forward);

        // The margin shrinks the half-angles, so the tightest corner lands at
        // `1 - margin` in normalised device coordinates instead of at 1.
        let slack = (1.0 - margin).clamp(f64::EPSILON, 1.0);
        let tan_v = (DEG2RAD * 0.5 * self.fov).tan() / self.zoom * slack;
        let tan_h = tan_v * self.aspect;

        let mut distance = 0.0_f64;
        let mut near_depth = f64::INFINITY;
        let mut far_depth = f64::NEG_INFINITY;

        for offset in &offsets {
            distance = js_max(distance, offset.x.abs() / tan_h - offset.z);
            distance = js_max(distance, offset.y.abs() / tan_v - offset.z);

            near_depth = js_min(near_depth, offset.z);
            far_depth = js_max(far_depth, offset.z);
        }

        // A box much deeper than it is wide can be framed from inside itself;
        // hold it at the camera's current near plane instead.
        let near_floor = js_max(self.near, f64::EPSILON);
        distance = js_max(distance, near_floor - near_depth);

        self.near = distance + near_depth;
        self.far = distance + far_depth;
        if self.far <= self.near {
            // A box with no depth along the view direction.
            self.far = self.near * 2.0;
        }
        self.update_projection_matrix();

        // `position` is local; the direction and the centre are in world space.
        let center = box_.get_center();
        let mut position = Vector3::ZERO;
        position
            .copy(&forward)
            .multiply_scalar(-distance)
            .add(&center);

        if let Some(parent) = self.node.parent() {
            parent.update_world_matrix(true, false);
            let mut inverse = parent.borrow().matrix_world;
            inverse.invert();
            position.apply_matrix4(&inverse);
        }

        self.node.borrow_mut().position = position;
        self.update_matrix_world();
    }

    /// `Object3D.lookAt()` for a camera: the matrix looks *from* the camera
    /// position *at* the target.
    pub fn look_at(&mut self, target: &Vector3) {
        let mut object = self.node.borrow_mut();
        let mut m = Matrix4::identity();
        m.look_at(&object.position, target, &object.up);
        object.quaternion.set_from_rotation_matrix(&m);
    }

    /// `Camera.updateMatrixWorld()`.
    pub fn update_matrix_world(&mut self) {
        self.node.update_matrix_world(false);
        self.matrix_world_inverse = self.node.borrow().matrix_world;
        self.matrix_world_inverse.invert();
    }
}
