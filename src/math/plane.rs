//! Port of `three.js/src/math/Plane.js`.

use super::{Box3, Line3, Matrix3, Matrix4, Sphere, Vector3};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Plane {
    pub normal: Vector3,
    pub constant: f64,
}

impl Default for Plane {
    /// `new Plane()` — normal `(1, 0, 0)`, constant `0`.
    fn default() -> Self {
        Self {
            normal: Vector3::new(1.0, 0.0, 0.0),
            constant: 0.0,
        }
    }
}

impl Plane {
    pub const IS_PLANE: bool = true;

    pub const fn new(normal: Vector3, constant: f64) -> Self {
        Self { normal, constant }
    }

    /// `Plane.set()`.
    pub fn set(&mut self, normal: &Vector3, constant: f64) -> &mut Self {
        self.normal.copy(normal);
        self.constant = constant;
        self
    }

    /// `Plane.setComponents()`.
    pub fn set_components(&mut self, x: f64, y: f64, z: f64, w: f64) -> &mut Self {
        self.normal.set(x, y, z);
        self.constant = w;
        self
    }

    /// `Plane.setFromNormalAndCoplanarPoint()`.
    pub fn set_from_normal_and_coplanar_point(
        &mut self,
        normal: &Vector3,
        point: &Vector3,
    ) -> &mut Self {
        self.normal.copy(normal);
        self.constant = -point.dot(&self.normal);
        self
    }

    /// `Plane.setFromCoplanarPoints()`.
    pub fn set_from_coplanar_points(&mut self, a: &Vector3, b: &Vector3, c: &Vector3) -> &mut Self {
        let mut vector1 = Vector3::default();
        let mut vector2 = Vector3::default();
        let normal = *vector1
            .sub_vectors(c, b)
            .cross(vector2.sub_vectors(a, b))
            .normalize();

        // Q: should an error be thrown if normal is zero (e.g. degenerate plane)?

        self.set_from_normal_and_coplanar_point(&normal, a);

        self
    }

    /// `Plane.copy()`.
    pub fn copy(&mut self, plane: &Self) -> &mut Self {
        self.normal.copy(&plane.normal);
        self.constant = plane.constant;
        self
    }

    /// `Plane.normalize()`.
    pub fn normalize(&mut self) -> &mut Self {
        // Note: will lead to a divide by zero if the plane is invalid.

        let inverse_normal_length = 1.0 / self.normal.length();
        self.normal.multiply_scalar(inverse_normal_length);
        self.constant *= inverse_normal_length;

        self
    }

    /// `Plane.negate()`.
    pub fn negate(&mut self) -> &mut Self {
        self.constant *= -1.0;
        self.normal.negate();
        self
    }

    /// `Plane.distanceToPoint()`.
    pub fn distance_to_point(&self, point: &Vector3) -> f64 {
        self.normal.dot(point) + self.constant
    }

    /// `Plane.distanceToSphere()`.
    pub fn distance_to_sphere(&self, sphere: &Sphere) -> f64 {
        self.distance_to_point(&sphere.center) - sphere.radius
    }

    /// `Plane.projectPoint()`.
    pub fn project_point(&self, point: &Vector3) -> Vector3 {
        let mut target = Vector3::default();
        *target
            .copy(point)
            .add_scaled_vector(&self.normal, -self.distance_to_point(point))
    }

    /// `Plane.intersectLine()`; `clamp_to_line` defaults to `true`.
    pub fn intersect_line(&self, line: &Line3, clamp_to_line: Option<bool>) -> Option<Vector3> {
        let clamp_to_line = clamp_to_line.unwrap_or(true);

        let direction = line.delta();

        let denominator = self.normal.dot(&direction);

        if denominator == 0.0 {
            // line is coplanar, return origin
            if self.distance_to_point(&line.start) == 0.0 {
                return Some(line.start);
            }

            // Unsure if this is the correct method to handle this case.
            return None;
        }

        let t = -(line.start.dot(&self.normal) + self.constant) / denominator;

        if clamp_to_line && (t < 0.0 || t > 1.0) {
            return None;
        }

        let mut target = Vector3::default();
        Some(*target.copy(&line.start).add_scaled_vector(&direction, t))
    }

    /// `Plane.intersectsLine()`.
    pub fn intersects_line(&self, line: &Line3) -> bool {
        // Note: this tests if a line intersects the plane, not whether it (or its end-points) are coplanar with it.

        let start_sign = self.distance_to_point(&line.start);
        let end_sign = self.distance_to_point(&line.end);

        (start_sign < 0.0 && end_sign > 0.0) || (end_sign < 0.0 && start_sign > 0.0)
    }

    /// `Plane.intersectsBox()`.
    pub fn intersects_box(&self, box3: &Box3) -> bool {
        box3.intersects_plane(self)
    }

    /// `Plane.intersectsSphere()`.
    pub fn intersects_sphere(&self, sphere: &Sphere) -> bool {
        sphere.intersects_plane(self)
    }

    /// `Plane.coplanarPoint()`.
    pub fn coplanar_point(&self) -> Vector3 {
        let mut target = Vector3::default();
        *target.copy(&self.normal).multiply_scalar(-self.constant)
    }

    /// `Plane.applyMatrix4()`.
    pub fn apply_matrix4(
        &mut self,
        matrix: &Matrix4,
        optional_normal_matrix: Option<&Matrix3>,
    ) -> &mut Self {
        let mut normal_matrix_storage = Matrix3::default();
        let normal_matrix = match optional_normal_matrix {
            Some(m) => *m,
            None => *normal_matrix_storage.get_normal_matrix(matrix),
        };

        let mut vector1 = self.coplanar_point();
        let reference_point = *vector1.apply_matrix4(matrix);

        let normal = *self.normal.apply_matrix3(&normal_matrix).normalize();

        self.constant = -reference_point.dot(&normal);

        self
    }

    /// `Plane.translate()`.
    pub fn translate(&mut self, offset: &Vector3) -> &mut Self {
        self.constant -= offset.dot(&self.normal);
        self
    }

    /// `Plane.equals()`.
    pub fn equals(&self, plane: &Self) -> bool {
        plane.normal.equals(&self.normal) && (plane.constant == self.constant)
    }
}
