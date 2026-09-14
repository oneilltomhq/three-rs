//! Port of `three.js/src/math/Vector3.js`.
//!
//! All arithmetic is `f64`, matching JavaScript number semantics: three.js
//! computes every transform in doubles and only narrows to `f32` when the
//! value reaches a buffer or a uniform.

use super::math_utils::{clamp, js_max, js_min};
use super::{Color, Euler, Matrix3, Matrix4, Quaternion};

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Vector3 {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

impl Vector3 {
    pub const fn new(x: f64, y: f64, z: f64) -> Self {
        Self { x, y, z }
    }

    pub const ZERO: Self = Self::new(0.0, 0.0, 0.0);

    pub fn set(&mut self, x: f64, y: f64, z: f64) -> &mut Self {
        self.x = x;
        self.y = y;
        self.z = z;
        self
    }

    /// `Vector3.setScalar()`.
    pub fn set_scalar(&mut self, scalar: f64) -> &mut Self {
        self.set(scalar, scalar, scalar)
    }

    pub fn set_x(&mut self, x: f64) -> &mut Self {
        self.x = x;
        self
    }

    pub fn set_y(&mut self, y: f64) -> &mut Self {
        self.y = y;
        self
    }

    pub fn set_z(&mut self, z: f64) -> &mut Self {
        self.z = z;
        self
    }

    /// `Vector3.setComponent()`; panics where three.js throws.
    pub fn set_component(&mut self, index: usize, value: f64) -> &mut Self {
        match index {
            0 => self.x = value,
            1 => self.y = value,
            2 => self.z = value,
            _ => panic!("THREE.Vector3: index is out of range: {index}"),
        }
        self
    }

    /// `Vector3.getComponent()`.
    pub fn get_component(&self, index: usize) -> f64 {
        match index {
            0 => self.x,
            1 => self.y,
            2 => self.z,
            _ => panic!("THREE.Vector3: index is out of range: {index}"),
        }
    }

    /// `Vector3.copy()`.
    pub fn copy(&mut self, v: &Self) -> &mut Self {
        self.x = v.x;
        self.y = v.y;
        self.z = v.z;
        self
    }

    pub fn add(&mut self, v: &Self) -> &mut Self {
        self.x += v.x;
        self.y += v.y;
        self.z += v.z;
        self
    }

    /// `Vector3.addScalar()`.
    pub fn add_scalar(&mut self, s: f64) -> &mut Self {
        self.x += s;
        self.y += s;
        self.z += s;
        self
    }

    pub fn add_vectors(&mut self, a: &Self, b: &Self) -> &mut Self {
        self.x = a.x + b.x;
        self.y = a.y + b.y;
        self.z = a.z + b.z;
        self
    }

    /// `Vector3.addScaledVector()`.
    pub fn add_scaled_vector(&mut self, v: &Self, s: f64) -> &mut Self {
        self.x += v.x * s;
        self.y += v.y * s;
        self.z += v.z * s;
        self
    }

    /// `Vector3.sub()`.
    pub fn sub(&mut self, v: &Self) -> &mut Self {
        self.x -= v.x;
        self.y -= v.y;
        self.z -= v.z;
        self
    }

    /// `Vector3.subScalar()`.
    pub fn sub_scalar(&mut self, s: f64) -> &mut Self {
        self.x -= s;
        self.y -= s;
        self.z -= s;
        self
    }

    pub fn sub_vectors(&mut self, a: &Self, b: &Self) -> &mut Self {
        self.x = a.x - b.x;
        self.y = a.y - b.y;
        self.z = a.z - b.z;
        self
    }

    /// `Vector3.multiply()`.
    pub fn multiply(&mut self, v: &Self) -> &mut Self {
        self.x *= v.x;
        self.y *= v.y;
        self.z *= v.z;
        self
    }

    pub fn multiply_scalar(&mut self, s: f64) -> &mut Self {
        self.x *= s;
        self.y *= s;
        self.z *= s;
        self
    }

    /// `Vector3.multiplyVectors()`.
    pub fn multiply_vectors(&mut self, a: &Self, b: &Self) -> &mut Self {
        self.x = a.x * b.x;
        self.y = a.y * b.y;
        self.z = a.z * b.z;
        self
    }

    /// `Vector3.applyEuler()`.
    pub fn apply_euler(&mut self, euler: &Euler) -> &mut Self {
        let mut q = Quaternion::default();
        q.set_from_euler(euler);
        self.apply_quaternion(&q)
    }

    /// `Vector3.applyAxisAngle()`.
    pub fn apply_axis_angle(&mut self, axis: &Self, angle: f64) -> &mut Self {
        let mut q = Quaternion::default();
        q.set_from_axis_angle(axis, angle);
        self.apply_quaternion(&q)
    }

    /// `Vector3.applyMatrix3()`.
    pub fn apply_matrix3(&mut self, m: &Matrix3) -> &mut Self {
        let (x, y, z) = (self.x, self.y, self.z);
        let e = &m.elements;

        self.x = e[0] * x + e[3] * y + e[6] * z;
        self.y = e[1] * x + e[4] * y + e[7] * z;
        self.z = e[2] * x + e[5] * y + e[8] * z;

        self
    }

    /// `Vector3.applyNormalMatrix()`.
    pub fn apply_normal_matrix(&mut self, m: &Matrix3) -> &mut Self {
        self.apply_matrix3(m).normalize()
    }

    /// `Vector3.applyMatrix4()`.
    pub fn apply_matrix4(&mut self, m: &Matrix4) -> &mut Self {
        let (x, y, z) = (self.x, self.y, self.z);
        let e = &m.elements;

        let w = 1.0 / (e[3] * x + e[7] * y + e[11] * z + e[15]);

        self.x = (e[0] * x + e[4] * y + e[8] * z + e[12]) * w;
        self.y = (e[1] * x + e[5] * y + e[9] * z + e[13]) * w;
        self.z = (e[2] * x + e[6] * y + e[10] * z + e[14]) * w;

        self
    }

    /// `Vector3.applyQuaternion()`.
    pub fn apply_quaternion(&mut self, q: &Quaternion) -> &mut Self {
        let (vx, vy, vz) = (self.x, self.y, self.z);
        let (qx, qy, qz, qw) = (q.x, q.y, q.z, q.w);

        // t = 2 * cross( q.xyz, v )
        let tx = 2.0 * (qy * vz - qz * vy);
        let ty = 2.0 * (qz * vx - qx * vz);
        let tz = 2.0 * (qx * vy - qy * vx);

        // v + q.w * t + cross( q.xyz, t )
        self.x = vx + qw * tx + qy * tz - qz * ty;
        self.y = vy + qw * ty + qz * tx - qx * tz;
        self.z = vz + qw * tz + qx * ty - qy * tx;

        self
    }

    /// `Vector3.project()`: world space to normalised device coordinates.
    pub fn project(&mut self, camera: &impl crate::cameras::RenderCamera) -> &mut Self {
        self.apply_matrix4(&camera.matrix_world_inverse())
            .apply_matrix4(&camera.projection_matrix())
    }

    /// `Vector3.unproject()`: normalised device coordinates back to world space.
    pub fn unproject(&mut self, camera: &impl crate::cameras::RenderCamera) -> &mut Self {
        self.apply_matrix4(&camera.projection_matrix_inverse())
            .apply_matrix4(&camera.matrix_world())
    }

    /// `Vector3.transformDirection()`.
    pub fn transform_direction(&mut self, m: &Matrix4) -> &mut Self {
        let (x, y, z) = (self.x, self.y, self.z);
        let e = &m.elements;

        self.x = e[0] * x + e[4] * y + e[8] * z;
        self.y = e[1] * x + e[5] * y + e[9] * z;
        self.z = e[2] * x + e[6] * y + e[10] * z;

        self.normalize()
    }

    /// `Vector3.divide()`.
    pub fn divide(&mut self, v: &Self) -> &mut Self {
        self.x /= v.x;
        self.y /= v.y;
        self.z /= v.z;
        self
    }

    pub fn divide_scalar(&mut self, s: f64) -> &mut Self {
        self.multiply_scalar(1.0 / s)
    }

    /// `Vector3.min()`.
    pub fn min(&mut self, v: &Self) -> &mut Self {
        self.x = js_min(self.x, v.x);
        self.y = js_min(self.y, v.y);
        self.z = js_min(self.z, v.z);
        self
    }

    /// `Vector3.max()`.
    pub fn max(&mut self, v: &Self) -> &mut Self {
        self.x = js_max(self.x, v.x);
        self.y = js_max(self.y, v.y);
        self.z = js_max(self.z, v.z);
        self
    }

    /// `Vector3.clamp()`.
    pub fn clamp(&mut self, min: &Self, max: &Self) -> &mut Self {
        self.x = clamp(self.x, min.x, max.x);
        self.y = clamp(self.y, min.y, max.y);
        self.z = clamp(self.z, min.z, max.z);
        self
    }

    /// `Vector3.clampScalar()`.
    pub fn clamp_scalar(&mut self, min_val: f64, max_val: f64) -> &mut Self {
        self.x = clamp(self.x, min_val, max_val);
        self.y = clamp(self.y, min_val, max_val);
        self.z = clamp(self.z, min_val, max_val);
        self
    }

    /// `Vector3.clampLength()`.
    pub fn clamp_length(&mut self, min: f64, max: f64) -> &mut Self {
        let length = self.length();
        self.divide_scalar(if length == 0.0 { 1.0 } else { length })
            .multiply_scalar(clamp(length, min, max))
    }

    /// `Vector3.floor()`.
    pub fn floor(&mut self) -> &mut Self {
        self.x = self.x.floor();
        self.y = self.y.floor();
        self.z = self.z.floor();
        self
    }

    /// `Vector3.ceil()`.
    pub fn ceil(&mut self) -> &mut Self {
        self.x = self.x.ceil();
        self.y = self.y.ceil();
        self.z = self.z.ceil();
        self
    }

    /// `Vector3.round()` — `Math.round`, which rounds half *up* (toward
    /// +Infinity), not half away from zero the way Rust's `f64::round` does.
    pub fn round(&mut self) -> &mut Self {
        self.x = js_round(self.x);
        self.y = js_round(self.y);
        self.z = js_round(self.z);
        self
    }

    /// `Vector3.roundToZero()` — `Math.trunc`.
    pub fn round_to_zero(&mut self) -> &mut Self {
        self.x = self.x.trunc();
        self.y = self.y.trunc();
        self.z = self.z.trunc();
        self
    }

    /// `Vector3.negate()`.
    pub fn negate(&mut self) -> &mut Self {
        self.x = -self.x;
        self.y = -self.y;
        self.z = -self.z;
        self
    }

    pub fn dot(&self, v: &Self) -> f64 {
        self.x * v.x + self.y * v.y + self.z * v.z
    }

    pub fn length_sq(&self) -> f64 {
        self.x * self.x + self.y * self.y + self.z * self.z
    }

    pub fn length(&self) -> f64 {
        (self.x * self.x + self.y * self.y + self.z * self.z).sqrt()
    }

    /// `Vector3.manhattanLength()`.
    pub fn manhattan_length(&self) -> f64 {
        self.x.abs() + self.y.abs() + self.z.abs()
    }

    /// `Vector3.normalize()`: divides by the length, or by 1 when the length is 0.
    pub fn normalize(&mut self) -> &mut Self {
        let l = self.length();
        self.divide_scalar(if l == 0.0 { 1.0 } else { l })
    }

    pub fn normalized(mut self) -> Self {
        self.normalize();
        self
    }

    /// `Vector3.setLength()`.
    pub fn set_length(&mut self, length: f64) -> &mut Self {
        self.normalize().multiply_scalar(length)
    }

    /// `Vector3.lerp()`.
    pub fn lerp(&mut self, v: &Self, alpha: f64) -> &mut Self {
        self.x += (v.x - self.x) * alpha;
        self.y += (v.y - self.y) * alpha;
        self.z += (v.z - self.z) * alpha;
        self
    }

    /// `Vector3.lerpVectors()`.
    pub fn lerp_vectors(&mut self, v1: &Self, v2: &Self, alpha: f64) -> &mut Self {
        self.x = v1.x + (v2.x - v1.x) * alpha;
        self.y = v1.y + (v2.y - v1.y) * alpha;
        self.z = v1.z + (v2.z - v1.z) * alpha;
        self
    }

    pub fn cross(&mut self, v: &Self) -> &mut Self {
        let a = *self;
        self.cross_vectors(&a, v)
    }

    pub fn cross_vectors(&mut self, a: &Self, b: &Self) -> &mut Self {
        let (ax, ay, az) = (a.x, a.y, a.z);
        let (bx, by, bz) = (b.x, b.y, b.z);
        self.x = ay * bz - az * by;
        self.y = az * bx - ax * bz;
        self.z = ax * by - ay * bx;
        self
    }

    /// `Vector3.projectOnVector()`.
    pub fn project_on_vector(&mut self, v: &Self) -> &mut Self {
        let denominator = v.length_sq();

        if denominator == 0.0 {
            return self.set(0.0, 0.0, 0.0);
        }

        let scalar = v.dot(self) / denominator;
        self.copy(v).multiply_scalar(scalar)
    }

    /// `Vector3.projectOnPlane()`.
    pub fn project_on_plane(&mut self, plane_normal: &Self) -> &mut Self {
        let mut v = *self;
        v.project_on_vector(plane_normal);
        self.sub(&v)
    }

    /// `Vector3.reflect()`.
    pub fn reflect(&mut self, normal: &Self) -> &mut Self {
        let mut v = *normal;
        v.multiply_scalar(2.0 * self.dot(normal));
        self.sub(&v)
    }

    /// `Vector3.angleTo()`.
    pub fn angle_to(&self, v: &Self) -> f64 {
        let denominator = (self.length_sq() * v.length_sq()).sqrt();

        if denominator == 0.0 {
            return std::f64::consts::PI / 2.0;
        }

        let theta = self.dot(v) / denominator;

        clamp(theta, -1.0, 1.0).acos()
    }

    /// `Vector3.distanceTo()`.
    pub fn distance_to(&self, v: &Self) -> f64 {
        self.distance_to_squared(v).sqrt()
    }

    /// `Vector3.distanceToSquared()`.
    pub fn distance_to_squared(&self, v: &Self) -> f64 {
        let (dx, dy, dz) = (self.x - v.x, self.y - v.y, self.z - v.z);
        dx * dx + dy * dy + dz * dz
    }

    /// `Vector3.manhattanDistanceTo()`.
    pub fn manhattan_distance_to(&self, v: &Self) -> f64 {
        (self.x - v.x).abs() + (self.y - v.y).abs() + (self.z - v.z).abs()
    }

    /// `Vector3.setFromSphericalCoords()`.
    pub fn set_from_spherical_coords(&mut self, radius: f64, phi: f64, theta: f64) -> &mut Self {
        let sin_phi_radius = phi.sin() * radius;

        self.x = sin_phi_radius * theta.sin();
        self.y = phi.cos() * radius;
        self.z = sin_phi_radius * theta.cos();

        self
    }

    /// `Vector3.setFromCylindricalCoords()`.
    pub fn set_from_cylindrical_coords(&mut self, radius: f64, theta: f64, y: f64) -> &mut Self {
        self.x = radius * theta.sin();
        self.y = y;
        self.z = radius * theta.cos();
        self
    }

    /// `Vector3.setFromMatrixPosition()`.
    pub fn set_from_matrix_position(&mut self, m: &Matrix4) -> &mut Self {
        let e = &m.elements;
        self.x = e[12];
        self.y = e[13];
        self.z = e[14];
        self
    }

    /// `Vector3.setFromMatrixScale()`.
    pub fn set_from_matrix_scale(&mut self, m: &Matrix4) -> &mut Self {
        let sx = self.set_from_matrix_column(m, 0).length();
        let sy = self.set_from_matrix_column(m, 1).length();
        let sz = self.set_from_matrix_column(m, 2).length();

        self.x = sx;
        self.y = sy;
        self.z = sz;

        self
    }

    /// `Vector3.setFromMatrixColumn()`.
    pub fn set_from_matrix_column(&mut self, m: &Matrix4, index: usize) -> &mut Self {
        self.from_array(&m.elements, index * 4)
    }

    /// `Vector3.setFromMatrix3Column()`.
    pub fn set_from_matrix3_column(&mut self, m: &Matrix3, index: usize) -> &mut Self {
        self.from_array(&m.elements, index * 3)
    }

    /// `Vector3.setFromEuler()`.
    pub fn set_from_euler(&mut self, e: &Euler) -> &mut Self {
        self.x = e.x;
        self.y = e.y;
        self.z = e.z;
        self
    }

    /// `Vector3.setFromColor()`.
    pub fn set_from_color(&mut self, c: &Color) -> &mut Self {
        self.x = c.r;
        self.y = c.g;
        self.z = c.b;
        self
    }

    /// `Vector3.equals()`.
    pub fn equals(&self, v: &Self) -> bool {
        v.x == self.x && v.y == self.y && v.z == self.z
    }

    /// `Vector3.fromArray()`.
    pub fn from_array(&mut self, array: &[f64], offset: usize) -> &mut Self {
        self.x = array[offset];
        self.y = array[offset + 1];
        self.z = array[offset + 2];
        self
    }

    /// `Vector3.toArray()`.
    pub fn to_array(&self) -> [f64; 3] {
        [self.x, self.y, self.z]
    }
}

/// `Math.round`: ties go toward +Infinity, so `-0.5` rounds to `-0`, not `-1`.
pub(crate) fn js_round(x: f64) -> f64 {
    (x + 0.5).floor()
}
