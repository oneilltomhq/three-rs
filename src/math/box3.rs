//! Port of `three.js/src/math/Box3.js`.

use crate::core::BufferAttribute;

use super::math_utils::{js_max, js_min};
use super::{Matrix4, Plane, Sphere, Triangle, Vector3};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Box3 {
    pub min: Vector3,
    pub max: Vector3,
}

impl Default for Box3 {
    /// `new Box3()` — min `(+Infinity, +Infinity, +Infinity)`,
    /// max `(-Infinity, -Infinity, -Infinity)`.
    fn default() -> Self {
        Self {
            min: Vector3::new(f64::INFINITY, f64::INFINITY, f64::INFINITY),
            max: Vector3::new(f64::NEG_INFINITY, f64::NEG_INFINITY, f64::NEG_INFINITY),
        }
    }
}

impl Box3 {
    pub const IS_BOX3: bool = true;

    pub const fn new(min: Vector3, max: Vector3) -> Self {
        Self { min, max }
    }

    /// `Box3.set()`.
    pub fn set(&mut self, min: &Vector3, max: &Vector3) -> &mut Self {
        self.min.copy(min);
        self.max.copy(max);

        self
    }

    /// `Box3.setFromArray()`.
    pub fn set_from_array(&mut self, array: &[f64]) -> &mut Self {
        self.make_empty();

        let mut vector = Vector3::default();

        let il = array.len();
        let mut i = 0;
        while i < il {
            let point = *vector.from_array(array, i);
            self.expand_by_point(&point);
            i += 3;
        }

        self
    }

    /// `Box3.setFromBufferAttribute()`.
    pub fn set_from_buffer_attribute(&mut self, attribute: &BufferAttribute) -> &mut Self {
        self.make_empty();

        for i in 0..attribute.count() {
            let point = attribute.get_vector3(i);
            self.expand_by_point(&point);
        }

        self
    }

    /// `Box3.setFromPoints()`.
    pub fn set_from_points(&mut self, points: &[Vector3]) -> &mut Self {
        self.make_empty();

        for point in points {
            self.expand_by_point(point);
        }

        self
    }

    /// `Box3.setFromCenterAndSize()`.
    pub fn set_from_center_and_size(&mut self, center: &Vector3, size: &Vector3) -> &mut Self {
        let mut vector = Vector3::default();
        let half_size = *vector.copy(size).multiply_scalar(0.5);

        self.min.copy(center).sub(&half_size);
        self.max.copy(center).add(&half_size);

        self
    }

    /// `Box3.copy()`.
    pub fn copy(&mut self, box_: &Self) -> &mut Self {
        self.min.copy(&box_.min);
        self.max.copy(&box_.max);

        self
    }

    /// `Box3.makeEmpty()`.
    pub fn make_empty(&mut self) -> &mut Self {
        self.min.x = f64::INFINITY;
        self.min.y = f64::INFINITY;
        self.min.z = f64::INFINITY;
        self.max.x = f64::NEG_INFINITY;
        self.max.y = f64::NEG_INFINITY;
        self.max.z = f64::NEG_INFINITY;

        self
    }

    /// `Box3.isEmpty()`.
    pub fn is_empty(&self) -> bool {
        // this is a more robust check for empty than ( volume <= 0 ) because volume can get positive with two negative axes

        (self.max.x < self.min.x) || (self.max.y < self.min.y) || (self.max.z < self.min.z)
    }

    /// `Box3.getCenter()`.
    pub fn get_center(&self) -> Vector3 {
        let mut target = Vector3::default();
        if self.is_empty() {
            *target.set(0.0, 0.0, 0.0)
        } else {
            *target
                .add_vectors(&self.min, &self.max)
                .multiply_scalar(0.5)
        }
    }

    /// `Box3.getSize()`.
    pub fn get_size(&self) -> Vector3 {
        let mut target = Vector3::default();
        if self.is_empty() {
            *target.set(0.0, 0.0, 0.0)
        } else {
            *target.sub_vectors(&self.max, &self.min)
        }
    }

    /// `Box3.expandByPoint()`.
    pub fn expand_by_point(&mut self, point: &Vector3) -> &mut Self {
        self.min.min(point);
        self.max.max(point);

        self
    }

    /// `Box3.expandByVector()`.
    pub fn expand_by_vector(&mut self, vector: &Vector3) -> &mut Self {
        self.min.sub(vector);
        self.max.add(vector);

        self
    }

    /// `Box3.expandByScalar()`.
    pub fn expand_by_scalar(&mut self, scalar: f64) -> &mut Self {
        self.min.add_scalar(-scalar);
        self.max.add_scalar(scalar);

        self
    }

    /// `Box3.containsPoint()`.
    pub fn contains_point(&self, point: &Vector3) -> bool {
        point.x >= self.min.x
            && point.x <= self.max.x
            && point.y >= self.min.y
            && point.y <= self.max.y
            && point.z >= self.min.z
            && point.z <= self.max.z
    }

    /// `Box3.containsBox()`.
    pub fn contains_box(&self, box_: &Self) -> bool {
        self.min.x <= box_.min.x
            && box_.max.x <= self.max.x
            && self.min.y <= box_.min.y
            && box_.max.y <= self.max.y
            && self.min.z <= box_.min.z
            && box_.max.z <= self.max.z
    }

    /// `Box3.getParameter()`.
    pub fn get_parameter(&self, point: &Vector3) -> Vector3 {
        // This can potentially have a divide by zero if the box
        // has a size dimension of 0.

        let mut target = Vector3::default();
        *target.set(
            (point.x - self.min.x) / (self.max.x - self.min.x),
            (point.y - self.min.y) / (self.max.y - self.min.y),
            (point.z - self.min.z) / (self.max.z - self.min.z),
        )
    }

    /// `Box3.intersectsBox()`.
    pub fn intersects_box(&self, box_: &Self) -> bool {
        // using 6 splitting planes to rule out intersections.
        box_.max.x >= self.min.x
            && box_.min.x <= self.max.x
            && box_.max.y >= self.min.y
            && box_.min.y <= self.max.y
            && box_.max.z >= self.min.z
            && box_.min.z <= self.max.z
    }

    /// `Box3.intersectsSphere()`.
    pub fn intersects_sphere(&self, sphere: &Sphere) -> bool {
        // Find the point on the AABB closest to the sphere center.
        let vector = self.clamp_point(&sphere.center);

        // If that point is inside the sphere, the AABB and sphere intersect.
        vector.distance_to_squared(&sphere.center) <= (sphere.radius * sphere.radius)
    }

    /// `Box3.intersectsPlane()`.
    pub fn intersects_plane(&self, plane: &Plane) -> bool {
        // We compute the minimum and maximum dot product values. If those values
        // are on the same side (back or front) of the plane, then there is no intersection.

        let mut min;
        let mut max;

        if plane.normal.x > 0.0 {
            min = plane.normal.x * self.min.x;
            max = plane.normal.x * self.max.x;
        } else {
            min = plane.normal.x * self.max.x;
            max = plane.normal.x * self.min.x;
        }

        if plane.normal.y > 0.0 {
            min += plane.normal.y * self.min.y;
            max += plane.normal.y * self.max.y;
        } else {
            min += plane.normal.y * self.max.y;
            max += plane.normal.y * self.min.y;
        }

        if plane.normal.z > 0.0 {
            min += plane.normal.z * self.min.z;
            max += plane.normal.z * self.max.z;
        } else {
            min += plane.normal.z * self.max.z;
            max += plane.normal.z * self.min.z;
        }

        (min <= -plane.constant) && (max >= -plane.constant)
    }

    /// `Box3.intersectsTriangle()`.
    pub fn intersects_triangle(&self, triangle: &Triangle) -> bool {
        if self.is_empty() {
            return false;
        }

        // compute box center and extents
        let center = self.get_center();
        let mut extents = Vector3::default();
        extents.sub_vectors(&self.max, &center);

        // translate triangle to aabb origin
        let mut v0 = Vector3::default();
        let mut v1 = Vector3::default();
        let mut v2 = Vector3::default();
        v0.sub_vectors(&triangle.a, &center);
        v1.sub_vectors(&triangle.b, &center);
        v2.sub_vectors(&triangle.c, &center);

        // compute edge vectors for triangle
        let mut f0 = Vector3::default();
        let mut f1 = Vector3::default();
        let mut f2 = Vector3::default();
        f0.sub_vectors(&v1, &v0);
        f1.sub_vectors(&v2, &v1);
        f2.sub_vectors(&v0, &v2);

        // test against axes that are given by cross product combinations of the edges of the triangle and the edges of the aabb
        // make an axis testing of each of the 3 sides of the aabb against each of the 3 sides of the triangle = 9 axis of separation
        // axis_ij = u_i x f_j (u0, u1, u2 = face normals of aabb = x,y,z axes vectors since aabb is axis aligned)
        let axes = [
            0.0, -f0.z, f0.y, 0.0, -f1.z, f1.y, 0.0, -f2.z, f2.y, f0.z, 0.0, -f0.x, f1.z, 0.0,
            -f1.x, f2.z, 0.0, -f2.x, -f0.y, f0.x, 0.0, -f1.y, f1.x, 0.0, -f2.y, f2.x, 0.0,
        ];
        if !sat_for_axes(&axes, &v0, &v1, &v2, &extents) {
            return false;
        }

        // test 3 face normals from the aabb
        let axes = [1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0];
        if !sat_for_axes(&axes, &v0, &v1, &v2, &extents) {
            return false;
        }

        // finally testing the face normal of the triangle
        // use already existing triangle edge vectors here
        let mut triangle_normal = Vector3::default();
        triangle_normal.cross_vectors(&f0, &f1);
        let axes = [triangle_normal.x, triangle_normal.y, triangle_normal.z];

        sat_for_axes(&axes, &v0, &v1, &v2, &extents)
    }

    /// `Box3.clampPoint()`.
    pub fn clamp_point(&self, point: &Vector3) -> Vector3 {
        let mut target = Vector3::default();
        *target.copy(point).clamp(&self.min, &self.max)
    }

    /// `Box3.distanceToPoint()`.
    pub fn distance_to_point(&self, point: &Vector3) -> f64 {
        self.clamp_point(point).distance_to(point)
    }

    /// `Box3.getBoundingSphere()`.
    pub fn get_bounding_sphere(&self) -> Sphere {
        let mut target = Sphere::default();

        if self.is_empty() {
            target.make_empty();
        } else {
            target.center = self.get_center();

            target.radius = self.get_size().length() * 0.5;
        }

        target
    }

    /// `Box3.intersect()`.
    pub fn intersect(&mut self, box_: &Self) -> &mut Self {
        self.min.max(&box_.min);
        self.max.min(&box_.max);

        // ensure that if there is no overlap, the result is fully empty, not slightly empty with non-inf/+inf values that will cause subsequence intersects to erroneously return valid values.
        if self.is_empty() {
            self.make_empty();
        }

        self
    }

    /// `Box3.union()`.
    pub fn union(&mut self, box_: &Self) -> &mut Self {
        self.min.min(&box_.min);
        self.max.max(&box_.max);

        self
    }

    /// `Box3.applyMatrix4()`.
    pub fn apply_matrix4(&mut self, matrix: &Matrix4) -> &mut Self {
        // transform of empty box is an empty box.
        if self.is_empty() {
            return self;
        }

        let mut points = [Vector3::default(); 8];

        // NOTE: I am using a binary pattern to specify all 2^3 combinations below
        points[0]
            .set(self.min.x, self.min.y, self.min.z)
            .apply_matrix4(matrix); // 000
        points[1]
            .set(self.min.x, self.min.y, self.max.z)
            .apply_matrix4(matrix); // 001
        points[2]
            .set(self.min.x, self.max.y, self.min.z)
            .apply_matrix4(matrix); // 010
        points[3]
            .set(self.min.x, self.max.y, self.max.z)
            .apply_matrix4(matrix); // 011
        points[4]
            .set(self.max.x, self.min.y, self.min.z)
            .apply_matrix4(matrix); // 100
        points[5]
            .set(self.max.x, self.min.y, self.max.z)
            .apply_matrix4(matrix); // 101
        points[6]
            .set(self.max.x, self.max.y, self.min.z)
            .apply_matrix4(matrix); // 110
        points[7]
            .set(self.max.x, self.max.y, self.max.z)
            .apply_matrix4(matrix); // 111

        self.set_from_points(&points);

        self
    }

    /// `Box3.translate()`.
    pub fn translate(&mut self, offset: &Vector3) -> &mut Self {
        self.min.add(offset);
        self.max.add(offset);

        self
    }

    /// `Box3.equals()`.
    pub fn equals(&self, box_: &Self) -> bool {
        box_.min.equals(&self.min) && box_.max.equals(&self.max)
    }
}

/// `Box3.js` module-private `satForAxes()`.
fn sat_for_axes(axes: &[f64], v0: &Vector3, v1: &Vector3, v2: &Vector3, extents: &Vector3) -> bool {
    let mut test_axis = Vector3::default();

    let j = axes.len() - 3;
    let mut i = 0;
    while i <= j {
        test_axis.from_array(axes, i);
        // project the aabb onto the separating axis
        let r = extents.x * test_axis.x.abs()
            + extents.y * test_axis.y.abs()
            + extents.z * test_axis.z.abs();
        // project all 3 vertices of the triangle onto the separating axis
        let p0 = v0.dot(&test_axis);
        let p1 = v1.dot(&test_axis);
        let p2 = v2.dot(&test_axis);
        // actual test, basically see if either of the most extreme of the triangle points intersects r
        if js_max(-js_max(p0, js_max(p1, p2)), js_min(p0, js_min(p1, p2))) > r {
            // points of the projected triangle are outside the projected half-length of the aabb
            // the axis is separating and we can exit
            return false;
        }

        i += 3;
    }

    true
}
