//! Port of `three.js/src/math/Quaternion.js`.

use super::math_utils::{clamp, js_min};
use super::{Euler, EulerOrder, Matrix4, Vector3};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Quaternion {
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub w: f64,
}

impl Default for Quaternion {
    fn default() -> Self {
        Self::new(0.0, 0.0, 0.0, 1.0)
    }
}

impl Quaternion {
    pub const fn new(x: f64, y: f64, z: f64, w: f64) -> Self {
        Self { x, y, z, w }
    }

    /// `Quaternion.slerpFlat()` — interpolates in place inside flat arrays, the
    /// form `AnimationMixer` uses.
    #[allow(clippy::too_many_arguments)]
    pub fn slerp_flat(
        dst: &mut [f64],
        dst_offset: usize,
        src0: &[f64],
        src_offset0: usize,
        src1: &[f64],
        src_offset1: usize,
        t: f64,
    ) {
        // fuzz-free, array-based Quaternion SLERP operation
        let mut t = t;

        let mut x0 = src0[src_offset0];
        let mut y0 = src0[src_offset0 + 1];
        let mut z0 = src0[src_offset0 + 2];
        let mut w0 = src0[src_offset0 + 3];

        let mut x1 = src1[src_offset1];
        let mut y1 = src1[src_offset1 + 1];
        let mut z1 = src1[src_offset1 + 2];
        let mut w1 = src1[src_offset1 + 3];

        if w0 != w1 || x0 != x1 || y0 != y1 || z0 != z1 {
            let mut dot = x0 * x1 + y0 * y1 + z0 * z1 + w0 * w1;

            if dot < 0.0 {
                x1 = -x1;
                y1 = -y1;
                z1 = -z1;
                w1 = -w1;
                dot = -dot;
            }

            let mut s = 1.0 - t;

            if dot < 0.9995 {
                // slerp
                let theta = dot.acos();
                let sin = theta.sin();

                s = (s * theta).sin() / sin;
                t = (t * theta).sin() / sin;

                x0 = x0 * s + x1 * t;
                y0 = y0 * s + y1 * t;
                z0 = z0 * s + z1 * t;
                w0 = w0 * s + w1 * t;
            } else {
                // for small angles, lerp then normalize
                x0 = x0 * s + x1 * t;
                y0 = y0 * s + y1 * t;
                z0 = z0 * s + z1 * t;
                w0 = w0 * s + w1 * t;

                let f = 1.0 / (x0 * x0 + y0 * y0 + z0 * z0 + w0 * w0).sqrt();

                x0 *= f;
                y0 *= f;
                z0 *= f;
                w0 *= f;
            }
        }

        dst[dst_offset] = x0;
        dst[dst_offset + 1] = y0;
        dst[dst_offset + 2] = z0;
        dst[dst_offset + 3] = w0;
    }

    /// `Quaternion.multiplyQuaternionsFlat()`.
    #[allow(clippy::too_many_arguments)]
    pub fn multiply_quaternions_flat(
        dst: &mut [f64],
        dst_offset: usize,
        src0: &[f64],
        src_offset0: usize,
        src1: &[f64],
        src_offset1: usize,
    ) {
        let x0 = src0[src_offset0];
        let y0 = src0[src_offset0 + 1];
        let z0 = src0[src_offset0 + 2];
        let w0 = src0[src_offset0 + 3];

        let x1 = src1[src_offset1];
        let y1 = src1[src_offset1 + 1];
        let z1 = src1[src_offset1 + 2];
        let w1 = src1[src_offset1 + 3];

        dst[dst_offset] = x0 * w1 + w0 * x1 + y0 * z1 - z0 * y1;
        dst[dst_offset + 1] = y0 * w1 + w0 * y1 + z0 * x1 - x0 * z1;
        dst[dst_offset + 2] = z0 * w1 + w0 * z1 + x0 * y1 - y0 * x1;
        dst[dst_offset + 3] = w0 * w1 - x0 * x1 - y0 * y1 - z0 * z1;
    }

    pub fn set(&mut self, x: f64, y: f64, z: f64, w: f64) -> &mut Self {
        self.x = x;
        self.y = y;
        self.z = z;
        self.w = w;
        self
    }

    /// `Quaternion.copy()`.
    pub fn copy(&mut self, q: &Self) -> &mut Self {
        *self = *q;
        self
    }

    /// `Quaternion.setFromEuler()`.
    pub fn set_from_euler(&mut self, euler: &Euler) -> &mut Self {
        let (x, y, z) = (euler.x, euler.y, euler.z);

        // http://www.mathworks.com/matlabcentral/fileexchange/
        //   20696-function-to-convert-between-dcm-euler-angles-quaternions-and-euler-vectors/
        //   content/SpinCalc.m

        let c1 = (x / 2.0).cos();
        let c2 = (y / 2.0).cos();
        let c3 = (z / 2.0).cos();

        let s1 = (x / 2.0).sin();
        let s2 = (y / 2.0).sin();
        let s3 = (z / 2.0).sin();

        match euler.order {
            EulerOrder::XYZ => {
                self.x = s1 * c2 * c3 + c1 * s2 * s3;
                self.y = c1 * s2 * c3 - s1 * c2 * s3;
                self.z = c1 * c2 * s3 + s1 * s2 * c3;
                self.w = c1 * c2 * c3 - s1 * s2 * s3;
            }
            EulerOrder::YXZ => {
                self.x = s1 * c2 * c3 + c1 * s2 * s3;
                self.y = c1 * s2 * c3 - s1 * c2 * s3;
                self.z = c1 * c2 * s3 - s1 * s2 * c3;
                self.w = c1 * c2 * c3 + s1 * s2 * s3;
            }
            EulerOrder::ZXY => {
                self.x = s1 * c2 * c3 - c1 * s2 * s3;
                self.y = c1 * s2 * c3 + s1 * c2 * s3;
                self.z = c1 * c2 * s3 + s1 * s2 * c3;
                self.w = c1 * c2 * c3 - s1 * s2 * s3;
            }
            EulerOrder::ZYX => {
                self.x = s1 * c2 * c3 - c1 * s2 * s3;
                self.y = c1 * s2 * c3 + s1 * c2 * s3;
                self.z = c1 * c2 * s3 - s1 * s2 * c3;
                self.w = c1 * c2 * c3 + s1 * s2 * s3;
            }
            EulerOrder::YZX => {
                self.x = s1 * c2 * c3 + c1 * s2 * s3;
                self.y = c1 * s2 * c3 + s1 * c2 * s3;
                self.z = c1 * c2 * s3 - s1 * s2 * c3;
                self.w = c1 * c2 * c3 - s1 * s2 * s3;
            }
            EulerOrder::XZY => {
                self.x = s1 * c2 * c3 - c1 * s2 * s3;
                self.y = c1 * s2 * c3 - s1 * c2 * s3;
                self.z = c1 * c2 * s3 + s1 * s2 * c3;
                self.w = c1 * c2 * c3 + s1 * s2 * s3;
            }
        }

        self
    }

    /// `Quaternion.setFromAxisAngle()`. `axis` is assumed to be normalized.
    pub fn set_from_axis_angle(&mut self, axis: &Vector3, angle: f64) -> &mut Self {
        let half_angle = angle / 2.0;
        let s = half_angle.sin();

        self.x = axis.x * s;
        self.y = axis.y * s;
        self.z = axis.z * s;
        self.w = half_angle.cos();

        self
    }

    /// `Quaternion.setFromRotationMatrix()`. Assumes the upper 3x3 of `m` is a
    /// pure rotation.
    pub fn set_from_rotation_matrix(&mut self, m: &Matrix4) -> &mut Self {
        let te = &m.elements;
        let (m11, m12, m13) = (te[0], te[4], te[8]);
        let (m21, m22, m23) = (te[1], te[5], te[9]);
        let (m31, m32, m33) = (te[2], te[6], te[10]);

        let trace = m11 + m22 + m33;

        if trace > 0.0 {
            let s = 0.5 / (trace + 1.0).sqrt();
            self.w = 0.25 / s;
            self.x = (m32 - m23) * s;
            self.y = (m13 - m31) * s;
            self.z = (m21 - m12) * s;
        } else if m11 > m22 && m11 > m33 {
            let s = 2.0 * (1.0 + m11 - m22 - m33).sqrt();
            self.w = (m32 - m23) / s;
            self.x = 0.25 * s;
            self.y = (m12 + m21) / s;
            self.z = (m13 + m31) / s;
        } else if m22 > m33 {
            let s = 2.0 * (1.0 + m22 - m11 - m33).sqrt();
            self.w = (m13 - m31) / s;
            self.x = (m12 + m21) / s;
            self.y = 0.25 * s;
            self.z = (m23 + m32) / s;
        } else {
            let s = 2.0 * (1.0 + m33 - m11 - m22).sqrt();
            self.w = (m21 - m12) / s;
            self.x = (m13 + m31) / s;
            self.y = (m23 + m32) / s;
            self.z = 0.25 * s;
        }

        self
    }

    /// `Quaternion.setFromUnitVectors()`. Both vectors are assumed normalized.
    pub fn set_from_unit_vectors(&mut self, v_from: &Vector3, v_to: &Vector3) -> &mut Self {
        let mut r = v_from.dot(v_to) + 1.0;

        if r < 1e-8 {
            // vFrom and vTo point in opposite directions
            r = 0.0;

            if v_from.x.abs() > v_from.z.abs() {
                self.x = -v_from.y;
                self.y = v_from.x;
                self.z = 0.0;
                self.w = r;
            } else {
                self.x = 0.0;
                self.y = -v_from.z;
                self.z = v_from.y;
                self.w = r;
            }
        } else {
            // crossVectors( vFrom, vTo )
            self.x = v_from.y * v_to.z - v_from.z * v_to.y;
            self.y = v_from.z * v_to.x - v_from.x * v_to.z;
            self.z = v_from.x * v_to.y - v_from.y * v_to.x;
            self.w = r;
        }

        self.normalize()
    }

    /// `Quaternion.angleTo()`.
    pub fn angle_to(&self, q: &Self) -> f64 {
        2.0 * clamp(self.dot(q), -1.0, 1.0).abs().acos()
    }

    /// `Quaternion.rotateTowards()`.
    pub fn rotate_towards(&mut self, q: &Self, step: f64) -> &mut Self {
        let angle = self.angle_to(q);

        if angle == 0.0 {
            return self;
        }

        let t = js_min(1.0, step / angle);

        self.slerp(q, t);

        self
    }

    /// `Quaternion.identity()`.
    pub fn identity(&mut self) -> &mut Self {
        self.set(0.0, 0.0, 0.0, 1.0)
    }

    /// `Quaternion.invert()` — for a unit quaternion this is the conjugate.
    pub fn invert(&mut self) -> &mut Self {
        self.conjugate()
    }

    /// `Quaternion.conjugate()`.
    pub fn conjugate(&mut self) -> &mut Self {
        self.x *= -1.0;
        self.y *= -1.0;
        self.z *= -1.0;
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

    /// `Quaternion.normalize()` — a zero-length quaternion becomes the identity.
    pub fn normalize(&mut self) -> &mut Self {
        let mut l = self.length();

        if l == 0.0 {
            self.x = 0.0;
            self.y = 0.0;
            self.z = 0.0;
            self.w = 1.0;
        } else {
            l = 1.0 / l;

            self.x *= l;
            self.y *= l;
            self.z *= l;
            self.w *= l;
        }

        self
    }

    /// `Quaternion.multiply()`.
    pub fn multiply(&mut self, q: &Self) -> &mut Self {
        let a = *self;
        self.multiply_quaternions(&a, q)
    }

    /// `Quaternion.premultiply()`.
    pub fn premultiply(&mut self, q: &Self) -> &mut Self {
        let b = *self;
        self.multiply_quaternions(q, &b)
    }

    /// `Quaternion.multiplyQuaternions()`.
    pub fn multiply_quaternions(&mut self, a: &Self, b: &Self) -> &mut Self {
        let (qax, qay, qaz, qaw) = (a.x, a.y, a.z, a.w);
        let (qbx, qby, qbz, qbw) = (b.x, b.y, b.z, b.w);

        self.x = qax * qbw + qaw * qbx + qay * qbz - qaz * qby;
        self.y = qay * qbw + qaw * qby + qaz * qbx - qax * qbz;
        self.z = qaz * qbw + qaw * qbz + qax * qby - qay * qbx;
        self.w = qaw * qbw - qax * qbx - qay * qby - qaz * qbz;

        self
    }

    /// `Quaternion.slerp()`.
    pub fn slerp(&mut self, qb: &Self, t: f64) -> &mut Self {
        let mut t = t;

        let (mut x, mut y, mut z, mut w) = (qb.x, qb.y, qb.z, qb.w);

        let mut dot = self.dot(qb);

        if dot < 0.0 {
            x = -x;
            y = -y;
            z = -z;
            w = -w;
            dot = -dot;
        }

        let mut s = 1.0 - t;

        if dot < 0.9995 {
            // slerp
            let theta = dot.acos();
            let sin = theta.sin();

            s = (s * theta).sin() / sin;
            t = (t * theta).sin() / sin;

            self.x = self.x * s + x * t;
            self.y = self.y * s + y * t;
            self.z = self.z * s + z * t;
            self.w = self.w * s + w * t;
        } else {
            // for small angles, lerp then normalize
            self.x = self.x * s + x * t;
            self.y = self.y * s + y * t;
            self.z = self.z * s + z * t;
            self.w = self.w * s + w * t;

            self.normalize();
        }

        self
    }

    /// `Quaternion.slerpQuaternions()`.
    pub fn slerp_quaternions(&mut self, qa: &Self, qb: &Self, t: f64) -> &mut Self {
        self.copy(qa).slerp(qb, t)
    }

    /// `Quaternion.equals()`.
    pub fn equals(&self, q: &Self) -> bool {
        q.x == self.x && q.y == self.y && q.z == self.z && q.w == self.w
    }

    /// `Quaternion.fromArray()`.
    pub fn from_array(&mut self, array: &[f64], offset: usize) -> &mut Self {
        self.x = array[offset];
        self.y = array[offset + 1];
        self.z = array[offset + 2];
        self.w = array[offset + 3];
        self
    }

    /// `Quaternion.toArray()`.
    pub fn to_array(&self) -> [f64; 4] {
        [self.x, self.y, self.z, self.w]
    }
}
