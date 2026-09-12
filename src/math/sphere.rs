//! Port of `three.js/src/math/Sphere.js`.

use super::math_utils::js_max;
use super::{Box3, Matrix4, Plane, Vector3};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Sphere {
    pub center: Vector3,
    pub radius: f64,
}

impl Default for Sphere {
    /// `new Sphere()` — center `(0, 0, 0)`, radius `-1`.
    fn default() -> Self {
        Self {
            center: Vector3::new(0.0, 0.0, 0.0),
            radius: -1.0,
        }
    }
}

impl Sphere {
    pub const IS_SPHERE: bool = true;

    pub const fn new(center: Vector3, radius: f64) -> Self {
        Self { center, radius }
    }

    /// `Sphere.set()`.
    pub fn set(&mut self, center: &Vector3, radius: f64) -> &mut Self {
        self.center.copy(center);
        self.radius = radius;

        self
    }

    /// `Sphere.setFromPoints()`.
    pub fn set_from_points(
        &mut self,
        points: &[Vector3],
        optional_center: Option<&Vector3>,
    ) -> &mut Self {
        if let Some(optional_center) = optional_center {
            self.center.copy(optional_center);
        } else {
            let mut box_ = Box3::default();
            self.center = box_.set_from_points(points).get_center();
        }

        let center = self.center;

        let mut max_radius_sq = 0.0;

        for point in points {
            max_radius_sq = js_max(max_radius_sq, center.distance_to_squared(point));
        }

        self.radius = max_radius_sq.sqrt();

        self
    }

    /// `Sphere.copy()`.
    pub fn copy(&mut self, sphere: &Self) -> &mut Self {
        self.center.copy(&sphere.center);
        self.radius = sphere.radius;

        self
    }

    /// `Sphere.isEmpty()`.
    pub fn is_empty(&self) -> bool {
        self.radius < 0.0
    }

    /// `Sphere.makeEmpty()`.
    pub fn make_empty(&mut self) -> &mut Self {
        self.center.set(0.0, 0.0, 0.0);
        self.radius = -1.0;

        self
    }

    /// `Sphere.containsPoint()`.
    pub fn contains_point(&self, point: &Vector3) -> bool {
        point.distance_to_squared(&self.center) <= (self.radius * self.radius)
    }

    /// `Sphere.distanceToPoint()`.
    pub fn distance_to_point(&self, point: &Vector3) -> f64 {
        point.distance_to(&self.center) - self.radius
    }

    /// `Sphere.intersectsSphere()`.
    pub fn intersects_sphere(&self, sphere: &Self) -> bool {
        let radius_sum = self.radius + sphere.radius;

        sphere.center.distance_to_squared(&self.center) <= (radius_sum * radius_sum)
    }

    /// `Sphere.intersectsBox()`.
    pub fn intersects_box(&self, box_: &Box3) -> bool {
        box_.intersects_sphere(self)
    }

    /// `Sphere.intersectsPlane()`.
    pub fn intersects_plane(&self, plane: &Plane) -> bool {
        plane.distance_to_point(&self.center).abs() <= self.radius
    }

    /// `Sphere.clampPoint()`.
    pub fn clamp_point(&self, point: &Vector3) -> Vector3 {
        let delta_length_sq = self.center.distance_to_squared(point);

        let mut target = Vector3::default();
        target.copy(point);

        if delta_length_sq > (self.radius * self.radius) {
            target.sub(&self.center).normalize();
            target.multiply_scalar(self.radius).add(&self.center);
        }

        target
    }

    /// `Sphere.getBoundingBox()`.
    pub fn get_bounding_box(&self) -> Box3 {
        let mut target = Box3::default();

        if self.is_empty() {
            // Empty sphere produces empty bounding box
            target.make_empty();
            return target;
        }

        target.set(&self.center, &self.center);
        target.expand_by_scalar(self.radius);

        target
    }

    /// `Sphere.applyMatrix4()`.
    pub fn apply_matrix4(&mut self, matrix: &Matrix4) -> &mut Self {
        self.center.apply_matrix4(matrix);
        self.radius = self.radius * matrix.get_max_scale_on_axis();

        self
    }

    /// `Sphere.translate()`.
    pub fn translate(&mut self, offset: &Vector3) -> &mut Self {
        self.center.add(offset);

        self
    }

    /// `Sphere.expandByPoint()`.
    pub fn expand_by_point(&mut self, point: &Vector3) -> &mut Self {
        if self.is_empty() {
            self.center.copy(point);

            self.radius = 0.0;

            return self;
        }

        let mut v1 = Vector3::default();
        v1.sub_vectors(point, &self.center);

        let length_sq = v1.length_sq();

        if length_sq > (self.radius * self.radius) {
            // calculate the minimal sphere

            let length = length_sq.sqrt();

            let delta = (length - self.radius) * 0.5;

            self.center.add_scaled_vector(&v1, delta / length);

            self.radius += delta;
        }

        self
    }

    /// `Sphere.union()`.
    pub fn union(&mut self, sphere: &Self) -> &mut Self {
        if sphere.is_empty() {
            return self;
        }

        if self.is_empty() {
            self.copy(sphere);

            return self;
        }

        if self.center.equals(&sphere.center) {
            self.radius = js_max(self.radius, sphere.radius);
        } else {
            let mut v2 = Vector3::default();
            v2.sub_vectors(&sphere.center, &self.center)
                .set_length(sphere.radius);

            let mut v1 = Vector3::default();
            let point = *v1.copy(&sphere.center).add(&v2);
            self.expand_by_point(&point);

            let point = *v1.copy(&sphere.center).sub(&v2);
            self.expand_by_point(&point);
        }

        self
    }

    /// `Sphere.equals()`.
    pub fn equals(&self, sphere: &Self) -> bool {
        sphere.center.equals(&self.center) && (sphere.radius == self.radius)
    }
}
