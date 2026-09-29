//! Port of `three.js/src/math/Vector2.js`.

use super::math_utils::{clamp, js_max, js_min};
use super::vector3::js_round;
use super::Matrix3;

/// three.js' `Vector2`: an ordered pair of numbers, used for 2D points,
/// directions, and any other pair of related values.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Vector2 {
    /// The x component.
    pub x: f64,
    /// The y component.
    pub y: f64,
}

impl Vector2 {
    /// Constructs a new vector with the given components.
    pub const fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }

    /// The zero vector.
    pub const ZERO: Self = Self::new(0.0, 0.0);

    /// `Vector2.width` — the alias `Vector2` exposes for `x`.
    pub fn width(&self) -> f64 {
        self.x
    }

    /// `Vector2.height` — the alias `Vector2` exposes for `y`.
    pub fn height(&self) -> f64 {
        self.y
    }

    /// Sets the vector's components.
    pub fn set(&mut self, x: f64, y: f64) -> &mut Self {
        self.x = x;
        self.y = y;
        self
    }

    /// `Vector2.setScalar()`: sets both components to `scalar`.
    pub fn set_scalar(&mut self, scalar: f64) -> &mut Self {
        self.set(scalar, scalar)
    }

    /// `Vector2.setX()`.
    pub fn set_x(&mut self, x: f64) -> &mut Self {
        self.x = x;
        self
    }

    /// `Vector2.setY()`.
    pub fn set_y(&mut self, y: f64) -> &mut Self {
        self.y = y;
        self
    }

    /// `Vector2.setComponent()`: sets `x` (index 0) or `y` (index 1). Panics
    /// where three.js throws.
    pub fn set_component(&mut self, index: usize, value: f64) -> &mut Self {
        match index {
            0 => self.x = value,
            1 => self.y = value,
            _ => panic!("THREE.Vector2: index is out of range: {index}"),
        }
        self
    }

    /// `Vector2.getComponent()`: `x` (index 0) or `y` (index 1). Panics where
    /// three.js throws.
    pub fn get_component(&self, index: usize) -> f64 {
        match index {
            0 => self.x,
            1 => self.y,
            _ => panic!("THREE.Vector2: index is out of range: {index}"),
        }
    }

    /// Copies the components of `v` into this vector.
    pub fn copy(&mut self, v: &Self) -> &mut Self {
        self.x = v.x;
        self.y = v.y;
        self
    }

    /// Adds `v` to this vector.
    pub fn add(&mut self, v: &Self) -> &mut Self {
        self.x += v.x;
        self.y += v.y;
        self
    }

    /// `Vector2.addScalar()`: adds `s` to both components.
    pub fn add_scalar(&mut self, s: f64) -> &mut Self {
        self.x += s;
        self.y += s;
        self
    }

    /// `Vector2.addVectors()`: sets this vector to `a + b`.
    pub fn add_vectors(&mut self, a: &Self, b: &Self) -> &mut Self {
        self.x = a.x + b.x;
        self.y = a.y + b.y;
        self
    }

    /// `Vector2.addScaledVector()`: adds `v * s` to this vector.
    pub fn add_scaled_vector(&mut self, v: &Self, s: f64) -> &mut Self {
        self.x += v.x * s;
        self.y += v.y * s;
        self
    }

    /// Subtracts `v` from this vector.
    pub fn sub(&mut self, v: &Self) -> &mut Self {
        self.x -= v.x;
        self.y -= v.y;
        self
    }

    /// `Vector2.subScalar()`: subtracts `s` from both components.
    pub fn sub_scalar(&mut self, s: f64) -> &mut Self {
        self.x -= s;
        self.y -= s;
        self
    }

    /// `Vector2.subVectors()`: sets this vector to `a - b`.
    pub fn sub_vectors(&mut self, a: &Self, b: &Self) -> &mut Self {
        self.x = a.x - b.x;
        self.y = a.y - b.y;
        self
    }

    /// Multiplies this vector by `v`, component-wise.
    pub fn multiply(&mut self, v: &Self) -> &mut Self {
        self.x *= v.x;
        self.y *= v.y;
        self
    }

    /// `Vector2.multiplyScalar()`.
    pub fn multiply_scalar(&mut self, scalar: f64) -> &mut Self {
        self.x *= scalar;
        self.y *= scalar;
        self
    }

    /// Divides this vector by `v`, component-wise.
    pub fn divide(&mut self, v: &Self) -> &mut Self {
        self.x /= v.x;
        self.y /= v.y;
        self
    }

    /// `Vector2.divideScalar()`.
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

    /// Replaces each component with the smaller of itself and the matching
    /// component of `v`.
    pub fn min(&mut self, v: &Self) -> &mut Self {
        self.x = js_min(self.x, v.x);
        self.y = js_min(self.y, v.y);
        self
    }

    /// Replaces each component with the larger of itself and the matching
    /// component of `v`.
    pub fn max(&mut self, v: &Self) -> &mut Self {
        self.x = js_max(self.x, v.x);
        self.y = js_max(self.y, v.y);
        self
    }

    /// Clamps each component between the matching components of `min` and
    /// `max`.
    pub fn clamp(&mut self, min: &Self, max: &Self) -> &mut Self {
        self.x = clamp(self.x, min.x, max.x);
        self.y = clamp(self.y, min.y, max.y);
        self
    }

    /// `Vector2.clampScalar()`: clamps each component between `min_val` and
    /// `max_val`.
    pub fn clamp_scalar(&mut self, min_val: f64, max_val: f64) -> &mut Self {
        self.x = clamp(self.x, min_val, max_val);
        self.y = clamp(self.y, min_val, max_val);
        self
    }

    /// `Vector2.clampLength()`: clamps this vector's length between `min` and
    /// `max`, preserving its direction.
    pub fn clamp_length(&mut self, min: f64, max: f64) -> &mut Self {
        let length = self.length();
        self.divide_scalar(if length == 0.0 { 1.0 } else { length })
            .multiply_scalar(clamp(length, min, max))
    }

    /// Rounds each component down to the nearest integer.
    pub fn floor(&mut self) -> &mut Self {
        self.x = self.x.floor();
        self.y = self.y.floor();
        self
    }

    /// Rounds each component up to the nearest integer.
    pub fn ceil(&mut self) -> &mut Self {
        self.x = self.x.ceil();
        self.y = self.y.ceil();
        self
    }

    /// `Vector2.round()` — `Math.round`, which rounds half *up* (toward
    /// +Infinity), not half away from zero the way Rust's `f64::round` does.
    pub fn round(&mut self) -> &mut Self {
        self.x = js_round(self.x);
        self.y = js_round(self.y);
        self
    }

    /// `Vector2.roundToZero()` — `Math.trunc`.
    pub fn round_to_zero(&mut self) -> &mut Self {
        self.x = self.x.trunc();
        self.y = self.y.trunc();
        self
    }

    /// Negates both components.
    pub fn negate(&mut self) -> &mut Self {
        self.x = -self.x;
        self.y = -self.y;
        self
    }

    /// The dot product of this vector and `v`.
    pub fn dot(&self, v: &Self) -> f64 {
        self.x * v.x + self.y * v.y
    }

    /// `Vector2.cross()` — the z component of the 3D cross product.
    pub fn cross(&self, v: &Self) -> f64 {
        self.x * v.y - self.y * v.x
    }

    /// `Vector2.lengthSq()`: the squared Euclidean length, cheaper than
    /// [`Vector2::length`] when only comparing lengths.
    pub fn length_sq(&self) -> f64 {
        self.x * self.x + self.y * self.y
    }

    /// The Euclidean length (distance from `(0, 0)` to `(x, y)`).
    pub fn length(&self) -> f64 {
        (self.x * self.x + self.y * self.y).sqrt()
    }

    /// `Vector2.manhattanLength()`.
    pub fn manhattan_length(&self) -> f64 {
        self.x.abs() + self.y.abs()
    }

    /// Scales this vector to unit length, or to `(0, 0)` if its length is 0.
    pub fn normalize(&mut self) -> &mut Self {
        let l = self.length();
        self.divide_scalar(if l == 0.0 { 1.0 } else { l })
    }

    /// `Vector2.angle()` — in the range `[0, 2pi)`, measured counter-clockwise
    /// from the positive x axis.
    pub fn angle(&self) -> f64 {
        (-self.y).atan2(-self.x) + std::f64::consts::PI
    }

    /// `Vector2.angleTo()`: the angle in radians between this vector and `v`.
    pub fn angle_to(&self, v: &Self) -> f64 {
        let denominator = (self.length_sq() * v.length_sq()).sqrt();

        if denominator == 0.0 {
            return std::f64::consts::PI / 2.0;
        }

        let theta = self.dot(v) / denominator;

        clamp(theta, -1.0, 1.0).acos()
    }

    /// The distance from this vector to `v`.
    pub fn distance_to(&self, v: &Self) -> f64 {
        self.distance_to_squared(v).sqrt()
    }

    /// `Vector2.distanceToSquared()`: cheaper than [`Vector2::distance_to`]
    /// when only comparing distances.
    pub fn distance_to_squared(&self, v: &Self) -> f64 {
        let (dx, dy) = (self.x - v.x, self.y - v.y);
        dx * dx + dy * dy
    }

    /// `Vector2.manhattanDistanceTo()`.
    pub fn manhattan_distance_to(&self, v: &Self) -> f64 {
        (self.x - v.x).abs() + (self.y - v.y).abs()
    }

    /// `Vector2.setLength()`: rescales this vector to the given length,
    /// preserving its direction.
    pub fn set_length(&mut self, length: f64) -> &mut Self {
        self.normalize().multiply_scalar(length)
    }

    /// Linearly interpolates from this vector toward `v`, where `alpha` is
    /// the distance along the line (`0` stays at this vector, `1` reaches
    /// `v`).
    pub fn lerp(&mut self, v: &Self, alpha: f64) -> &mut Self {
        self.x += (v.x - self.x) * alpha;
        self.y += (v.y - self.y) * alpha;
        self
    }

    /// `Vector2.lerpVectors()`: sets this vector to the linear interpolation
    /// from `v1` to `v2` at `alpha`.
    pub fn lerp_vectors(&mut self, v1: &Self, v2: &Self, alpha: f64) -> &mut Self {
        self.x = v1.x + (v2.x - v1.x) * alpha;
        self.y = v1.y + (v2.y - v1.y) * alpha;
        self
    }

    /// Whether this vector's components equal `v`'s.
    pub fn equals(&self, v: &Self) -> bool {
        v.x == self.x && v.y == self.y
    }

    /// `Vector2.fromArray()`: reads `x` from `array[offset]` and `y` from
    /// `array[offset + 1]`.
    pub fn from_array(&mut self, array: &[f64], offset: usize) -> &mut Self {
        self.x = array[offset];
        self.y = array[offset + 1];
        self
    }

    /// `Vector2.toArray()`: `[x, y]`.
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
}
