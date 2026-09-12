//! Port of `three.js/src/math/Vector4.js`.

use super::math_utils::{clamp, js_max, js_min};
use super::vector3::js_round;
use super::{Matrix4, Quaternion};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Vector4 {
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub w: f64,
}

impl Default for Vector4 {
    /// `new Vector4()` — `w` defaults to 1, not 0.
    fn default() -> Self {
        Self::new(0.0, 0.0, 0.0, 1.0)
    }
}

impl Vector4 {
    pub const fn new(x: f64, y: f64, z: f64, w: f64) -> Self {
        Self { x, y, z, w }
    }

    /// `Vector4.width` — the alias `Vector4` exposes for `z`.
    pub fn width(&self) -> f64 {
        self.z
    }

    /// `Vector4.height` — the alias `Vector4` exposes for `w`.
    pub fn height(&self) -> f64 {
        self.w
    }

    pub fn set(&mut self, x: f64, y: f64, z: f64, w: f64) -> &mut Self {
        self.x = x;
        self.y = y;
        self.z = z;
        self.w = w;
        self
    }

    pub fn set_scalar(&mut self, scalar: f64) -> &mut Self {
        self.set(scalar, scalar, scalar, scalar)
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

    pub fn set_w(&mut self, w: f64) -> &mut Self {
        self.w = w;
        self
    }

    pub fn set_component(&mut self, index: usize, value: f64) -> &mut Self {
        match index {
            0 => self.x = value,
            1 => self.y = value,
            2 => self.z = value,
            3 => self.w = value,
            _ => panic!("THREE.Vector4: index is out of range: {index}"),
        }
        self
    }

    pub fn get_component(&self, index: usize) -> f64 {
        match index {
            0 => self.x,
            1 => self.y,
            2 => self.z,
            3 => self.w,
            _ => panic!("THREE.Vector4: index is out of range: {index}"),
        }
    }

    pub fn copy(&mut self, v: &Self) -> &mut Self {
        self.x = v.x;
        self.y = v.y;
        self.z = v.z;
        self.w = v.w;
        self
    }

    pub fn add(&mut self, v: &Self) -> &mut Self {
        self.x += v.x;
        self.y += v.y;
        self.z += v.z;
        self.w += v.w;
        self
    }

    pub fn add_scalar(&mut self, s: f64) -> &mut Self {
        self.x += s;
        self.y += s;
        self.z += s;
        self.w += s;
        self
    }

    pub fn add_vectors(&mut self, a: &Self, b: &Self) -> &mut Self {
        self.x = a.x + b.x;
        self.y = a.y + b.y;
        self.z = a.z + b.z;
        self.w = a.w + b.w;
        self
    }

    pub fn add_scaled_vector(&mut self, v: &Self, s: f64) -> &mut Self {
        self.x += v.x * s;
        self.y += v.y * s;
        self.z += v.z * s;
        self.w += v.w * s;
        self
    }

    pub fn sub(&mut self, v: &Self) -> &mut Self {
        self.x -= v.x;
        self.y -= v.y;
        self.z -= v.z;
        self.w -= v.w;
        self
    }

    pub fn sub_scalar(&mut self, s: f64) -> &mut Self {
        self.x -= s;
        self.y -= s;
        self.z -= s;
        self.w -= s;
        self
    }

    pub fn sub_vectors(&mut self, a: &Self, b: &Self) -> &mut Self {
        self.x = a.x - b.x;
        self.y = a.y - b.y;
        self.z = a.z - b.z;
        self.w = a.w - b.w;
        self
    }

    pub fn multiply(&mut self, v: &Self) -> &mut Self {
        self.x *= v.x;
        self.y *= v.y;
        self.z *= v.z;
        self.w *= v.w;
        self
    }

    pub fn multiply_scalar(&mut self, scalar: f64) -> &mut Self {
        self.x *= scalar;
        self.y *= scalar;
        self.z *= scalar;
        self.w *= scalar;
        self
    }

    /// `Vector4.applyMatrix4()`.
    pub fn apply_matrix4(&mut self, m: &Matrix4) -> &mut Self {
        let (x, y, z, w) = (self.x, self.y, self.z, self.w);
        let e = &m.elements;

        self.x = e[0] * x + e[4] * y + e[8] * z + e[12] * w;
        self.y = e[1] * x + e[5] * y + e[9] * z + e[13] * w;
        self.z = e[2] * x + e[6] * y + e[10] * z + e[14] * w;
        self.w = e[3] * x + e[7] * y + e[11] * z + e[15] * w;

        self
    }

    pub fn divide(&mut self, v: &Self) -> &mut Self {
        self.x /= v.x;
        self.y /= v.y;
        self.z /= v.z;
        self.w /= v.w;
        self
    }

    pub fn divide_scalar(&mut self, scalar: f64) -> &mut Self {
        self.multiply_scalar(1.0 / scalar)
    }

    /// `Vector4.setAxisAngleFromQuaternion()`: `xyz` is the axis, `w` the angle.
    pub fn set_axis_angle_from_quaternion(&mut self, q: &Quaternion) -> &mut Self {
        // http://www.euclideanspace.com/maths/geometry/rotations/conversions/quaternionToAngle/index.htm
        // q is assumed to be normalized
        self.w = 2.0 * q.w.acos();

        let s = (1.0 - q.w * q.w).sqrt();

        if s < 0.0001 {
            self.x = 1.0;
            self.y = 0.0;
            self.z = 0.0;
        } else {
            self.x = q.x / s;
            self.y = q.y / s;
            self.z = q.z / s;
        }

        self
    }

    /// `Vector4.setAxisAngleFromRotationMatrix()`.
    pub fn set_axis_angle_from_rotation_matrix(&mut self, m: &Matrix4) -> &mut Self {
        // http://www.euclideanspace.com/maths/geometry/rotations/conversions/matrixToAngle/index.htm
        // assumes the upper 3x3 of m is a pure rotation matrix (i.e, unscaled)

        let epsilon = 0.01; // margin to allow for rounding errors
        let epsilon2 = 0.1; // margin to distinguish between 0 and 180 degrees

        let te = &m.elements;
        let (m11, m12, m13) = (te[0], te[4], te[8]);
        let (m21, m22, m23) = (te[1], te[5], te[9]);
        let (m31, m32, m33) = (te[2], te[6], te[10]);

        if (m12 - m21).abs() < epsilon && (m13 - m31).abs() < epsilon && (m23 - m32).abs() < epsilon
        {
            // singularity found; first check for identity matrix which must have +1 for all terms in leading diagonal
            if (m12 + m21).abs() < epsilon2
                && (m13 + m31).abs() < epsilon2
                && (m23 + m32).abs() < epsilon2
                && (m11 + m22 + m33 - 3.0).abs() < epsilon2
            {
                // this singularity is identity matrix so angle = 0
                self.set(1.0, 0.0, 0.0, 0.0);
                return self; // zero angle, arbitrary axis
            }

            // otherwise this singularity is angle = 180
            let angle = std::f64::consts::PI;

            let xx = (m11 + 1.0) / 2.0;
            let yy = (m22 + 1.0) / 2.0;
            let zz = (m33 + 1.0) / 2.0;
            let xy = (m12 + m21) / 4.0;
            let xz = (m13 + m31) / 4.0;
            let yz = (m23 + m32) / 4.0;

            let (x, y, z);

            if xx > yy && xx > zz {
                if xx < epsilon {
                    x = 0.0;
                    y = 0.707106781;
                    z = 0.707106781;
                } else {
                    x = xx.sqrt();
                    y = xy / x;
                    z = xz / x;
                }
            } else if yy > zz {
                if yy < epsilon {
                    x = 0.707106781;
                    y = 0.0;
                    z = 0.707106781;
                } else {
                    y = yy.sqrt();
                    x = xy / y;
                    z = yz / y;
                }
            } else if zz < epsilon {
                x = 0.707106781;
                y = 0.707106781;
                z = 0.0;
            } else {
                z = zz.sqrt();
                x = xz / z;
                y = yz / z;
            }

            self.set(x, y, z, angle);
            return self; // return 180 deg rotation
        }

        // as we have reached here there are no singularities so we can handle normally
        let mut s = ((m32 - m23) * (m32 - m23)
            + (m13 - m31) * (m13 - m31)
            + (m21 - m12) * (m21 - m12))
            .sqrt(); // used to normalize

        if s.abs() < 0.001 {
            s = 1.0;
        }

        self.x = (m32 - m23) / s;
        self.y = (m13 - m31) / s;
        self.z = (m21 - m12) / s;
        self.w = ((m11 + m22 + m33 - 1.0) / 2.0).acos();

        self
    }

    /// `Vector4.setFromMatrixPosition()`.
    pub fn set_from_matrix_position(&mut self, m: &Matrix4) -> &mut Self {
        let e = &m.elements;
        self.x = e[12];
        self.y = e[13];
        self.z = e[14];
        self.w = e[15];
        self
    }

    pub fn min(&mut self, v: &Self) -> &mut Self {
        self.x = js_min(self.x, v.x);
        self.y = js_min(self.y, v.y);
        self.z = js_min(self.z, v.z);
        self.w = js_min(self.w, v.w);
        self
    }

    pub fn max(&mut self, v: &Self) -> &mut Self {
        self.x = js_max(self.x, v.x);
        self.y = js_max(self.y, v.y);
        self.z = js_max(self.z, v.z);
        self.w = js_max(self.w, v.w);
        self
    }

    pub fn clamp(&mut self, min: &Self, max: &Self) -> &mut Self {
        self.x = clamp(self.x, min.x, max.x);
        self.y = clamp(self.y, min.y, max.y);
        self.z = clamp(self.z, min.z, max.z);
        self.w = clamp(self.w, min.w, max.w);
        self
    }

    pub fn clamp_scalar(&mut self, min_val: f64, max_val: f64) -> &mut Self {
        self.x = clamp(self.x, min_val, max_val);
        self.y = clamp(self.y, min_val, max_val);
        self.z = clamp(self.z, min_val, max_val);
        self.w = clamp(self.w, min_val, max_val);
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
        self.z = self.z.floor();
        self.w = self.w.floor();
        self
    }

    pub fn ceil(&mut self) -> &mut Self {
        self.x = self.x.ceil();
        self.y = self.y.ceil();
        self.z = self.z.ceil();
        self.w = self.w.ceil();
        self
    }

    pub fn round(&mut self) -> &mut Self {
        self.x = js_round(self.x);
        self.y = js_round(self.y);
        self.z = js_round(self.z);
        self.w = js_round(self.w);
        self
    }

    pub fn round_to_zero(&mut self) -> &mut Self {
        self.x = self.x.trunc();
        self.y = self.y.trunc();
        self.z = self.z.trunc();
        self.w = self.w.trunc();
        self
    }

    pub fn negate(&mut self) -> &mut Self {
        self.x = -self.x;
        self.y = -self.y;
        self.z = -self.z;
        self.w = -self.w;
        self
    }

    pub fn dot(&self, v: &Self) -> f64 {
        self.x * v.x + self.y * v.y + self.z * v.z + self.w * v.w
    }

    pub fn length_sq(&self) -> f64 {
        self.x * self.x + self.y * self.y + self.z * self.z + self.w * self.w
    }

    pub fn length(&self) -> f64 {
        (self.x * self.x + self.y * self.y + self.z * self.z + self.w * self.w).sqrt()
    }

    pub fn manhattan_length(&self) -> f64 {
        self.x.abs() + self.y.abs() + self.z.abs() + self.w.abs()
    }

    pub fn normalize(&mut self) -> &mut Self {
        let l = self.length();
        self.divide_scalar(if l == 0.0 { 1.0 } else { l })
    }

    pub fn set_length(&mut self, length: f64) -> &mut Self {
        self.normalize().multiply_scalar(length)
    }

    pub fn lerp(&mut self, v: &Self, alpha: f64) -> &mut Self {
        self.x += (v.x - self.x) * alpha;
        self.y += (v.y - self.y) * alpha;
        self.z += (v.z - self.z) * alpha;
        self.w += (v.w - self.w) * alpha;
        self
    }

    pub fn lerp_vectors(&mut self, v1: &Self, v2: &Self, alpha: f64) -> &mut Self {
        self.x = v1.x + (v2.x - v1.x) * alpha;
        self.y = v1.y + (v2.y - v1.y) * alpha;
        self.z = v1.z + (v2.z - v1.z) * alpha;
        self.w = v1.w + (v2.w - v1.w) * alpha;
        self
    }

    pub fn equals(&self, v: &Self) -> bool {
        v.x == self.x && v.y == self.y && v.z == self.z && v.w == self.w
    }

    pub fn from_array(&mut self, array: &[f64], offset: usize) -> &mut Self {
        self.x = array[offset];
        self.y = array[offset + 1];
        self.z = array[offset + 2];
        self.w = array[offset + 3];
        self
    }

    pub fn to_array(&self) -> [f64; 4] {
        [self.x, self.y, self.z, self.w]
    }

    /// The four components narrowed to `f32`, for a `vec4<f32>` uniform.
    pub fn to_f32_array(&self) -> [f32; 4] {
        [self.x as f32, self.y as f32, self.z as f32, self.w as f32]
    }
}
