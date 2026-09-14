//! Port of `three.js/src/math/Box2.js`.

use super::Vector2;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Box2 {
    pub min: Vector2,
    pub max: Vector2,
}

impl Default for Box2 {
    /// `new Box2()` — min `(+Infinity, +Infinity)`, max `(-Infinity, -Infinity)`.
    fn default() -> Self {
        Self {
            min: Vector2::new(f64::INFINITY, f64::INFINITY),
            max: Vector2::new(f64::NEG_INFINITY, f64::NEG_INFINITY),
        }
    }
}

impl Box2 {
    pub const IS_BOX2: bool = true;

    pub const fn new(min: Vector2, max: Vector2) -> Self {
        Self { min, max }
    }

    /// `Box2.set()`.
    pub fn set(&mut self, min: &Vector2, max: &Vector2) -> &mut Self {
        self.min.copy(min);
        self.max.copy(max);

        self
    }

    /// `Box2.setFromPoints()`.
    pub fn set_from_points(&mut self, points: &[Vector2]) -> &mut Self {
        self.make_empty();

        for point in points {
            self.expand_by_point(point);
        }

        self
    }

    /// `Box2.setFromCenterAndSize()`.
    pub fn set_from_center_and_size(&mut self, center: &Vector2, size: &Vector2) -> &mut Self {
        let mut vector = Vector2::default();
        let half_size = *vector.copy(size).multiply_scalar(0.5);
        self.min.copy(center).sub(&half_size);
        self.max.copy(center).add(&half_size);

        self
    }

    /// `Box2.copy()`.
    pub fn copy(&mut self, box_: &Self) -> &mut Self {
        self.min.copy(&box_.min);
        self.max.copy(&box_.max);

        self
    }

    /// `Box2.makeEmpty()`.
    pub fn make_empty(&mut self) -> &mut Self {
        self.min.x = f64::INFINITY;
        self.min.y = f64::INFINITY;
        self.max.x = f64::NEG_INFINITY;
        self.max.y = f64::NEG_INFINITY;

        self
    }

    /// `Box2.isEmpty()`.
    pub fn is_empty(&self) -> bool {
        // this is a more robust check for empty than ( volume <= 0 ) because volume can get positive with two negative axes

        (self.max.x < self.min.x) || (self.max.y < self.min.y)
    }

    /// `Box2.getCenter()`.
    pub fn get_center(&self) -> Vector2 {
        let mut target = Vector2::default();
        if self.is_empty() {
            *target.set(0.0, 0.0)
        } else {
            *target
                .add_vectors(&self.min, &self.max)
                .multiply_scalar(0.5)
        }
    }

    /// `Box2.getSize()`.
    pub fn get_size(&self) -> Vector2 {
        let mut target = Vector2::default();
        if self.is_empty() {
            *target.set(0.0, 0.0)
        } else {
            *target.sub_vectors(&self.max, &self.min)
        }
    }

    /// `Box2.expandByPoint()`.
    pub fn expand_by_point(&mut self, point: &Vector2) -> &mut Self {
        self.min.min(point);
        self.max.max(point);

        self
    }

    /// `Box2.expandByVector()`.
    pub fn expand_by_vector(&mut self, vector: &Vector2) -> &mut Self {
        self.min.sub(vector);
        self.max.add(vector);

        self
    }

    /// `Box2.expandByScalar()`.
    pub fn expand_by_scalar(&mut self, scalar: f64) -> &mut Self {
        self.min.add_scalar(-scalar);
        self.max.add_scalar(scalar);

        self
    }

    /// `Box2.containsPoint()`.
    pub fn contains_point(&self, point: &Vector2) -> bool {
        point.x >= self.min.x
            && point.x <= self.max.x
            && point.y >= self.min.y
            && point.y <= self.max.y
    }

    /// `Box2.containsBox()`.
    pub fn contains_box(&self, box_: &Self) -> bool {
        self.min.x <= box_.min.x
            && box_.max.x <= self.max.x
            && self.min.y <= box_.min.y
            && box_.max.y <= self.max.y
    }

    /// `Box2.getParameter()`.
    pub fn get_parameter(&self, point: &Vector2) -> Vector2 {
        // This can potentially have a divide by zero if the box
        // has a size dimension of 0.

        let mut target = Vector2::default();
        *target.set(
            (point.x - self.min.x) / (self.max.x - self.min.x),
            (point.y - self.min.y) / (self.max.y - self.min.y),
        )
    }

    /// `Box2.intersectsBox()`.
    pub fn intersects_box(&self, box_: &Self) -> bool {
        // using 4 splitting planes to rule out intersections

        box_.max.x >= self.min.x
            && box_.min.x <= self.max.x
            && box_.max.y >= self.min.y
            && box_.min.y <= self.max.y
    }

    /// `Box2.clampPoint()`.
    pub fn clamp_point(&self, point: &Vector2) -> Vector2 {
        let mut target = Vector2::default();
        *target.copy(point).clamp(&self.min, &self.max)
    }

    /// `Box2.distanceToPoint()`.
    pub fn distance_to_point(&self, point: &Vector2) -> f64 {
        self.clamp_point(point).distance_to(point)
    }

    /// `Box2.intersect()`.
    pub fn intersect(&mut self, box_: &Self) -> &mut Self {
        self.min.max(&box_.min);
        self.max.min(&box_.max);

        if self.is_empty() {
            self.make_empty();
        }

        self
    }

    /// `Box2.union()`.
    pub fn union(&mut self, box_: &Self) -> &mut Self {
        self.min.min(&box_.min);
        self.max.max(&box_.max);

        self
    }

    /// `Box2.translate()`.
    pub fn translate(&mut self, offset: &Vector2) -> &mut Self {
        self.min.add(offset);
        self.max.add(offset);

        self
    }

    /// `Box2.equals()`.
    pub fn equals(&self, box_: &Self) -> bool {
        box_.min.equals(&self.min) && box_.max.equals(&self.max)
    }
}
