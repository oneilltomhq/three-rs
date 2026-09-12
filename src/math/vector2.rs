//! Port of `three.js/src/math/Vector2.js`.

use super::math_utils::{clamp, js_max, js_min};
use super::vector3::js_round;
use super::Matrix3;

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Vector2 {
    pub x: f64,
    pub y: f64,
}

impl Vector2 {
    pub const fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }

    pub const ZERO: Self = Self::new(0.0, 0.0);

    /// `Vector2.width` — the alias `Vector2` exposes for `x`.
    pub fn width(&self) -> f64 {
        self.x
    }

    /// `Vector2.height` — the alias `Vector2` exposes for `y`.
    pub fn height(&self) -> f64 {
        self.y
    }

    pub fn set(&mut self, x: f64, y: f64) -> &mut Self {
        self.x = x;
        self.y = y;
        self
    }

    pub fn set_scalar(&mut self, scalar: f64) -> &mut Self {
        self.set(scalar, scalar)
    }

    pub fn set_x(&mut self, x: f64) -> &mut Self {
        self.x = x;
        self
    }

    pub fn set_y(&mut self, y: f64) -> &mut Self {
        self.y = y;
        self
    }

    pub fn set_component(&mut self, index: usize, value: f64) -> &mut Self {
        match index {
            0 => self.x = value,
            1 => self.y = value,
            _ => panic!("THREE.Vector2: index is out of range: {index}"),
        }
        self
    }

    pub fn get_component(&self, index: usize) -> f64 {
        match index {
            0 => self.x,
            1 => self.y,
            _ => panic!("THREE.Vector2: index is out of range: {index}"),
        }
    }

    pub fn copy(&mut self, v: &Self) -> &mut Self {
        self.x = v.x;
        self.y = v.y;
        self
    }

    pub fn add(&mut self, v: &Self) -> &mut Self {
        self.x += v.x;
        self.y += v.y;
        self
    }

    pub fn add_scalar(&mut self, s: f64) -> &mut Self {
        self.x += s;
        self.y += s;
        self
    }

    pub fn add_vectors(&mut self, a: &Self, b: &Self) -> &mut Self {
        self.x = a.x + b.x;
        self.y = a.y + b.y;
        self
    }

    pub fn add_scaled_vector(&mut self, v: &Self, s: f64) -> &mut Self {
        self.x += v.x * s;
        self.y += v.y * s;
        self
    }

    pub fn sub(&mut self, v: &Self) -> &mut Self {
        self.x -= v.x;
        self.y -= v.y;
        self
    }

    pub fn sub_scalar(&mut self, s: f64) -> &mut Self {
        self.x -= s;
        self.y -= s;
        self
    }

    pub fn sub_vectors(&mut self, a: &Self, b: &Self) -> &mut Self {
        self.x = a.x - b.x;
        self.y = a.y - b.y;
        self
    }

    pub fn multiply(&mut self, v: &Self) -> &mut Self {
        self.x *= v.x;
        self.y *= v.y;
        self
    }

    pub fn multiply_scalar(&mut self, scalar: f64) -> &mut Self {
        self.x *= scalar;
        self.y *= scalar;
        self
    }

    pub fn divide(&mut self, v: &Self) -> &mut Self {
        self.x /= v.x;
        self.y /= v.y;
        self
    }

    pub fn divide_scalar(&mut self, scalar: f64) -> &mut Self {
        self.multiply_scalar(1.0 / scalar)
    }

    /// `Vector2.applyMatrix3()`.
    pub fn apply_matrix3(&mut self, m: &Matrix3) -> &mut Self {
        let (x, y) = (self.x, self.y);
        let e = &m.elements;

        self.x = e[0] * x + e[3] * y + e[6];
        self.y = e[1] * x + e[4] * y + e[7];

        self
    }

    pub fn min(&mut self, v: &Self) -> &mut Self {
        self.x = js_min(self.x, v.x);
        self.y = js_min(self.y, v.y);
        self
    }

    pub fn max(&mut self, v: &Self) -> &mut Self {
        self.x = js_max(self.x, v.x);
        self.y = js_max(self.y, v.y);
        self
    }

    pub fn clamp(&mut self, min: &Self, max: &Self) -> &mut Self {
        self.x = clamp(self.x, min.x, max.x);
        self.y = clamp(self.y, min.y, max.y);
        self
    }

    pub fn clamp_scalar(&mut self, min_val: f64, max_val: f64) -> &mut Self {
        self.x = clamp(self.x, min_val, max_val);
        self.y = clamp(self.y, min_val, max_val);
        self
    }

    pub fn clamp_length(&mut self, min: f64, max: f64) -> &mut Self {
        let length = self.length();
        self.divide_scalar(if length == 0.0 { 1.0 } else { length })
            .multiply_scalar(clamp(length, min, max))
    }

    pub fn floor(&mut self) -> &mut Self {
        self.x = self.x.floor();
        self.y = self.y.floor();
        self
    }

    pub fn ceil(&mut self) -> &mut Self {
        self.x = self.x.ceil();
        self.y = self.y.ceil();
        self
    }

    pub fn round(&mut self) -> &mut Self {
        self.x = js_round(self.x);
        self.y = js_round(self.y);
        self
    }

    pub fn round_to_zero(&mut self) -> &mut Self {
        self.x = self.x.trunc();
        self.y = self.y.trunc();
        self
    }

    pub fn negate(&mut self) -> &mut Self {
        self.x = -self.x;
        self.y = -self.y;
        self
    }

    pub fn dot(&self, v: &Self) -> f64 {
        self.x * v.x + self.y * v.y
    }

    /// `Vector2.cross()` — the z component of the 3D cross product.
    pub fn cross(&self, v: &Self) -> f64 {
        self.x * v.y - self.y * v.x
    }

    pub fn length_sq(&self) -> f64 {
        self.x * self.x + self.y * self.y
    }

    pub fn length(&self) -> f64 {
        (self.x * self.x + self.y * self.y).sqrt()
    }

    pub fn manhattan_length(&self) -> f64 {
        self.x.abs() + self.y.abs()
    }

    pub fn normalize(&mut self) -> &mut Self {
        let l = self.length();
        self.divide_scalar(if l == 0.0 { 1.0 } else { l })
    }

    /// `Vector2.angle()` — in the range `[0, 2pi)`, measured counter-clockwise
    /// from the positive x axis.
    pub fn angle(&self) -> f64 {
        (-self.y).atan2(-self.x) + std::f64::consts::PI
    }

    pub fn angle_to(&self, v: &Self) -> f64 {
        let denominator = (self.length_sq() * v.length_sq()).sqrt();

        if denominator == 0.0 {
            return std::f64::consts::PI / 2.0;
        }

        let theta = self.dot(v) / denominator;

        clamp(theta, -1.0, 1.0).acos()
    }

    pub fn distance_to(&self, v: &Self) -> f64 {
        self.distance_to_squared(v).sqrt()
    }

    pub fn distance_to_squared(&self, v: &Self) -> f64 {
        let (dx, dy) = (self.x - v.x, self.y - v.y);
        dx * dx + dy * dy
    }

    pub fn manhattan_distance_to(&self, v: &Self) -> f64 {
        (self.x - v.x).abs() + (self.y - v.y).abs()
    }

    pub fn set_length(&mut self, length: f64) -> &mut Self {
        self.normalize().multiply_scalar(length)
    }

    pub fn lerp(&mut self, v: &Self, alpha: f64) -> &mut Self {
        self.x += (v.x - self.x) * alpha;
        self.y += (v.y - self.y) * alpha;
        self
    }

    pub fn lerp_vectors(&mut self, v1: &Self, v2: &Self, alpha: f64) -> &mut Self {
        self.x = v1.x + (v2.x - v1.x) * alpha;
        self.y = v1.y + (v2.y - v1.y) * alpha;
        self
    }

    pub fn equals(&self, v: &Self) -> bool {
        v.x == self.x && v.y == self.y
    }

    pub fn from_array(&mut self, array: &[f64], offset: usize) -> &mut Self {
        self.x = array[offset];
        self.y = array[offset + 1];
        self
    }

    pub fn to_array(&self) -> [f64; 2] {
        [self.x, self.y]
    }

    /// `Vector2.rotateAround()`.
    pub fn rotate_around(&mut self, center: &Self, angle: f64) -> &mut Self {
        let (c, s) = (angle.cos(), angle.sin());

        let x = self.x - center.x;
        let y = self.y - center.y;

        self.x = x * c - y * s + center.x;
        self.y = x * s + y * c + center.y;

        self
    }

    /// The two components narrowed to `f32`, for a `vec2<f32>` uniform.
    pub fn to_f32_array(&self) -> [f32; 2] {
        [self.x as f32, self.y as f32]
    }
}
