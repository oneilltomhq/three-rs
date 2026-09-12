//! Port of `three.js/src/math/Matrix4.js` (rung 1 subset).
//!
//! Column-major, same element order as three.js (`elements[0..4]` is the first
//! column), so the `f32` narrowing at upload time is a straight copy and
//! matches what WebGPURenderer writes into its uniform buffers.

use super::{Quaternion, Vector3};

/// three.js `WebGPUCoordinateSystem`: clip space depth is 0..1.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CoordinateSystem {
    WebGPU,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Matrix4 {
    pub elements: [f64; 16],
}

impl Default for Matrix4 {
    fn default() -> Self {
        Self::identity()
    }
}

impl Matrix4 {
    pub const fn identity() -> Self {
        Self {
            elements: [
                1.0, 0.0, 0.0, 0.0, //
                0.0, 1.0, 0.0, 0.0, //
                0.0, 0.0, 1.0, 0.0, //
                0.0, 0.0, 0.0, 1.0,
            ],
        }
    }

    /// The 16 elements narrowed to `f32`, ready for a uniform buffer.
    pub fn to_f32_array(&self) -> [f32; 16] {
        let mut out = [0.0f32; 16];
        for i in 0..16 {
            out[i] = self.elements[i] as f32;
        }
        out
    }

    /// `Matrix4.compose()`.
    pub fn compose(&mut self, position: &Vector3, quaternion: &Quaternion, scale: &Vector3) -> &mut Self {
        let te = &mut self.elements;

        let (x, y, z, w) = (quaternion.x, quaternion.y, quaternion.z, quaternion.w);
        let (x2, y2, z2) = (x + x, y + y, z + z);
        let (xx, xy, xz) = (x * x2, x * y2, x * z2);
        let (yy, yz, zz) = (y * y2, y * z2, z * z2);
        let (wx, wy, wz) = (w * x2, w * y2, w * z2);

        let (sx, sy, sz) = (scale.x, scale.y, scale.z);

        te[0] = (1.0 - (yy + zz)) * sx;
        te[1] = (xy + wz) * sx;
        te[2] = (xz - wy) * sx;
        te[3] = 0.0;

        te[4] = (xy - wz) * sy;
        te[5] = (1.0 - (xx + zz)) * sy;
        te[6] = (yz + wx) * sy;
        te[7] = 0.0;

        te[8] = (xz + wy) * sz;
        te[9] = (yz - wx) * sz;
        te[10] = (1.0 - (xx + yy)) * sz;
        te[11] = 0.0;

        te[12] = position.x;
        te[13] = position.y;
        te[14] = position.z;
        te[15] = 1.0;

        self
    }

    /// `Matrix4.lookAt()`: writes only the upper 3x3 rotation.
    pub fn look_at(&mut self, eye: &Vector3, target: &Vector3, up: &Vector3) -> &mut Self {
        let mut z = Vector3::ZERO;
        let mut x = Vector3::ZERO;
        let mut y = Vector3::ZERO;

        z.sub_vectors(eye, target);

        if z.length_sq() == 0.0 {
            z.z = 1.0;
        }

        z.normalize();
        x.cross_vectors(up, &z);

        if x.length_sq() == 0.0 {
            // up and z are parallel
            if up.z.abs() == 1.0 {
                z.x += 0.0001;
            } else {
                z.z += 0.0001;
            }

            z.normalize();
            x.cross_vectors(up, &z);
        }

        x.normalize();
        y.cross_vectors(&z, &x);

        let te = &mut self.elements;
        te[0] = x.x;
        te[4] = y.x;
        te[8] = z.x;
        te[1] = x.y;
        te[5] = y.y;
        te[9] = z.y;
        te[2] = x.z;
        te[6] = y.z;
        te[10] = z.z;

        self
    }

    /// `Matrix4.multiplyMatrices()`.
    pub fn multiply_matrices(&mut self, a: &Self, b: &Self) -> &mut Self {
        let ae = &a.elements;
        let be = &b.elements;

        let (a11, a12, a13, a14) = (ae[0], ae[4], ae[8], ae[12]);
        let (a21, a22, a23, a24) = (ae[1], ae[5], ae[9], ae[13]);
        let (a31, a32, a33, a34) = (ae[2], ae[6], ae[10], ae[14]);
        let (a41, a42, a43, a44) = (ae[3], ae[7], ae[11], ae[15]);

        let (b11, b12, b13, b14) = (be[0], be[4], be[8], be[12]);
        let (b21, b22, b23, b24) = (be[1], be[5], be[9], be[13]);
        let (b31, b32, b33, b34) = (be[2], be[6], be[10], be[14]);
        let (b41, b42, b43, b44) = (be[3], be[7], be[11], be[15]);

        let te = &mut self.elements;

        te[0] = a11 * b11 + a12 * b21 + a13 * b31 + a14 * b41;
        te[4] = a11 * b12 + a12 * b22 + a13 * b32 + a14 * b42;
        te[8] = a11 * b13 + a12 * b23 + a13 * b33 + a14 * b43;
        te[12] = a11 * b14 + a12 * b24 + a13 * b34 + a14 * b44;

        te[1] = a21 * b11 + a22 * b21 + a23 * b31 + a24 * b41;
        te[5] = a21 * b12 + a22 * b22 + a23 * b32 + a24 * b42;
        te[9] = a21 * b13 + a22 * b23 + a23 * b33 + a24 * b43;
        te[13] = a21 * b14 + a22 * b24 + a23 * b34 + a24 * b44;

        te[2] = a31 * b11 + a32 * b21 + a33 * b31 + a34 * b41;
        te[6] = a31 * b12 + a32 * b22 + a33 * b32 + a34 * b42;
        te[10] = a31 * b13 + a32 * b23 + a33 * b33 + a34 * b43;
        te[14] = a31 * b14 + a32 * b24 + a33 * b34 + a34 * b44;

        te[3] = a41 * b11 + a42 * b21 + a43 * b31 + a44 * b41;
        te[7] = a41 * b12 + a42 * b22 + a43 * b32 + a44 * b42;
        te[11] = a41 * b13 + a42 * b23 + a43 * b33 + a44 * b43;
        te[15] = a41 * b14 + a42 * b24 + a43 * b34 + a44 * b44;

        self
    }

    /// `Matrix4.invert()`.
    pub fn invert(&mut self) -> &mut Self {
        let te = &mut self.elements;

        let (n11, n21, n31, n41) = (te[0], te[1], te[2], te[3]);
        let (n12, n22, n32, n42) = (te[4], te[5], te[6], te[7]);
        let (n13, n23, n33, n43) = (te[8], te[9], te[10], te[11]);
        let (n14, n24, n34, n44) = (te[12], te[13], te[14], te[15]);

        let t1 = n11 * n22 - n21 * n12;
        let t2 = n11 * n32 - n31 * n12;
        let t3 = n11 * n42 - n41 * n12;
        let t4 = n21 * n32 - n31 * n22;
        let t5 = n21 * n42 - n41 * n22;
        let t6 = n31 * n42 - n41 * n32;
        let t7 = n13 * n24 - n23 * n14;
        let t8 = n13 * n34 - n33 * n14;
        let t9 = n13 * n44 - n43 * n14;
        let t10 = n23 * n34 - n33 * n24;
        let t11 = n23 * n44 - n43 * n24;
        let t12 = n33 * n44 - n43 * n34;

        let det = t1 * t12 - t2 * t11 + t3 * t10 + t4 * t9 - t5 * t8 + t6 * t7;

        if det == 0.0 {
            *te = [0.0; 16];
            return self;
        }

        let det_inv = 1.0 / det;

        te[0] = (n22 * t12 - n32 * t11 + n42 * t10) * det_inv;
        te[1] = (n31 * t11 - n21 * t12 - n41 * t10) * det_inv;
        te[2] = (n24 * t6 - n34 * t5 + n44 * t4) * det_inv;
        te[3] = (n33 * t5 - n23 * t6 - n43 * t4) * det_inv;

        te[4] = (n32 * t9 - n12 * t12 - n42 * t8) * det_inv;
        te[5] = (n11 * t12 - n31 * t9 + n41 * t8) * det_inv;
        te[6] = (n34 * t3 - n14 * t6 - n44 * t2) * det_inv;
        te[7] = (n13 * t6 - n33 * t3 + n43 * t2) * det_inv;

        te[8] = (n12 * t11 - n22 * t9 + n42 * t7) * det_inv;
        te[9] = (n21 * t9 - n11 * t11 - n41 * t7) * det_inv;
        te[10] = (n14 * t5 - n24 * t3 + n44 * t1) * det_inv;
        te[11] = (n23 * t3 - n13 * t5 - n43 * t1) * det_inv;

        te[12] = (n22 * t8 - n12 * t10 - n32 * t7) * det_inv;
        te[13] = (n11 * t10 - n21 * t8 + n31 * t7) * det_inv;
        te[14] = (n24 * t2 - n14 * t4 - n34 * t1) * det_inv;
        te[15] = (n13 * t4 - n23 * t2 + n33 * t1) * det_inv;

        self
    }

    /// `Matrix4.makePerspective()`, `reversedDepth = false`.
    pub fn make_perspective(
        &mut self,
        left: f64,
        right: f64,
        top: f64,
        bottom: f64,
        near: f64,
        far: f64,
        coordinate_system: CoordinateSystem,
    ) -> &mut Self {
        let x = 2.0 * near / (right - left);
        let y = 2.0 * near / (top - bottom);

        let a = (right + left) / (right - left);
        let b = (top + bottom) / (top - bottom);

        let (c, d) = match coordinate_system {
            CoordinateSystem::WebGPU => (-far / (far - near), (-far * near) / (far - near)),
        };

        let te = &mut self.elements;
        te[0] = x;
        te[4] = 0.0;
        te[8] = a;
        te[12] = 0.0;
        te[1] = 0.0;
        te[5] = y;
        te[9] = b;
        te[13] = 0.0;
        te[2] = 0.0;
        te[6] = 0.0;
        te[10] = c;
        te[14] = d;
        te[3] = 0.0;
        te[7] = 0.0;
        te[11] = -1.0;
        te[15] = 0.0;

        self
    }
}
