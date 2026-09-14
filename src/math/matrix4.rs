//! Port of `three.js/src/math/Matrix4.js` (rung 1 subset).
//!
//! Column-major, same element order as three.js (`elements[0..4]` is the first
//! column), so the `f32` narrowing at upload time is a straight copy and
//! matches what WebGPURenderer writes into its uniform buffers.

use super::{Quaternion, Vector3};

/// three.js' `WebGLCoordinateSystem` / `WebGPUCoordinateSystem`: clip space
/// depth is -1..1 for the former and 0..1 for the latter. `Matrix4`'s
/// projection builders default to `WebGL`; `WebGPURenderer` always passes
/// `WebGPU`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum CoordinateSystem {
    #[default]
    WebGL,
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
        for (o, e) in out.iter_mut().zip(self.elements.iter()) {
            *o = *e as f32;
        }
        out
    }

    /// `Matrix4.makeScale()`.
    pub fn make_scale(&mut self, x: f64, y: f64, z: f64) -> &mut Self {
        self.elements = [
            x, 0.0, 0.0, 0.0, //
            0.0, y, 0.0, 0.0, //
            0.0, 0.0, z, 0.0, //
            0.0, 0.0, 0.0, 1.0,
        ];
        self
    }

    /// `Matrix4.compose()`.
    pub fn compose(
        &mut self,
        position: &Vector3,
        quaternion: &Quaternion,
        scale: &Vector3,
    ) -> &mut Self {
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
    #[allow(clippy::too_many_arguments)] // mirrors three.js's `makePerspective(left, right, top, bottom, near, far, coordinateSystem)`; public API, batched separately (#9/#38/#39)
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
            CoordinateSystem::WebGL => (
                -(far + near) / (far - near),
                (-2.0 * far * near) / (far - near),
            ),
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

    /// `Matrix4.set( n11, n12, ... )` — row-order arguments into a
    /// column-major store.
    #[allow(clippy::too_many_arguments)]
    pub fn set(
        &mut self,
        n11: f64,
        n12: f64,
        n13: f64,
        n14: f64,
        n21: f64,
        n22: f64,
        n23: f64,
        n24: f64,
        n31: f64,
        n32: f64,
        n33: f64,
        n34: f64,
        n41: f64,
        n42: f64,
        n43: f64,
        n44: f64,
    ) -> &mut Self {
        self.elements = [
            n11, n21, n31, n41, //
            n12, n22, n32, n42, //
            n13, n23, n33, n43, //
            n14, n24, n34, n44,
        ];
        self
    }

    /// `new Matrix4( n11, ... )`.
    #[allow(clippy::too_many_arguments)]
    pub fn from_rows(
        n11: f64,
        n12: f64,
        n13: f64,
        n14: f64,
        n21: f64,
        n22: f64,
        n23: f64,
        n24: f64,
        n31: f64,
        n32: f64,
        n33: f64,
        n34: f64,
        n41: f64,
        n42: f64,
        n43: f64,
        n44: f64,
    ) -> Self {
        let mut m = Self::identity();
        m.set(
            n11, n12, n13, n14, n21, n22, n23, n24, n31, n32, n33, n34, n41, n42, n43, n44,
        );
        m
    }

    /// `Matrix4.identity()`.
    pub fn set_identity(&mut self) -> &mut Self {
        self.elements = Self::identity().elements;
        self
    }

    /// `Matrix4.copy()`.
    pub fn copy(&mut self, m: &Self) -> &mut Self {
        self.elements = m.elements;
        self
    }

    /// `Matrix4.copyPosition()`.
    pub fn copy_position(&mut self, m: &Self) -> &mut Self {
        let (te, me) = (&mut self.elements, &m.elements);
        te[12] = me[12];
        te[13] = me[13];
        te[14] = me[14];
        self
    }

    /// `Matrix4.setFromMatrix3()`.
    pub fn set_from_matrix3(&mut self, m: &crate::math::Matrix3) -> &mut Self {
        let me = m.elements;
        self.set(
            me[0], me[3], me[6], 0.0, //
            me[1], me[4], me[7], 0.0, //
            me[2], me[5], me[8], 0.0, //
            0.0, 0.0, 0.0, 1.0,
        )
    }

    /// `Matrix4.extractBasis()`.
    pub fn extract_basis(&self, x_axis: &mut Vector3, y_axis: &mut Vector3, z_axis: &mut Vector3) {
        if self.determinant_affine() == 0.0 {
            x_axis.set(1.0, 0.0, 0.0);
            y_axis.set(0.0, 1.0, 0.0);
            z_axis.set(0.0, 0.0, 1.0);
            return;
        }

        x_axis.set_from_matrix_column(self, 0);
        y_axis.set_from_matrix_column(self, 1);
        z_axis.set_from_matrix_column(self, 2);
    }

    /// `Matrix4.makeBasis()`.
    pub fn make_basis(
        &mut self,
        x_axis: &Vector3,
        y_axis: &Vector3,
        z_axis: &Vector3,
    ) -> &mut Self {
        self.set(
            x_axis.x, y_axis.x, z_axis.x, 0.0, //
            x_axis.y, y_axis.y, z_axis.y, 0.0, //
            x_axis.z, y_axis.z, z_axis.z, 0.0, //
            0.0, 0.0, 0.0, 1.0,
        )
    }

    /// `Matrix4.extractRotation()`: the upper 3x3 of `m` with its scale divided
    /// out. A singular matrix yields the identity.
    pub fn extract_rotation(&mut self, m: &Self) -> &mut Self {
        if m.determinant_affine() == 0.0 {
            return self.set_identity();
        }

        let mut v1 = Vector3::ZERO;

        let scale_x = 1.0 / v1.set_from_matrix_column(m, 0).length();
        let scale_y = 1.0 / v1.set_from_matrix_column(m, 1).length();
        let scale_z = 1.0 / v1.set_from_matrix_column(m, 2).length();

        let me = m.elements;
        let te = &mut self.elements;

        te[0] = me[0] * scale_x;
        te[1] = me[1] * scale_x;
        te[2] = me[2] * scale_x;
        te[3] = 0.0;

        te[4] = me[4] * scale_y;
        te[5] = me[5] * scale_y;
        te[6] = me[6] * scale_y;
        te[7] = 0.0;

        te[8] = me[8] * scale_z;
        te[9] = me[9] * scale_z;
        te[10] = me[10] * scale_z;
        te[11] = 0.0;

        te[12] = 0.0;
        te[13] = 0.0;
        te[14] = 0.0;
        te[15] = 1.0;

        self
    }

    /// `Matrix4.makeRotationFromEuler()`.
    pub fn make_rotation_from_euler(&mut self, euler: &crate::math::Euler) -> &mut Self {
        use crate::math::EulerOrder;

        let (x, y, z) = (euler.x, euler.y, euler.z);
        let (a, b) = (x.cos(), x.sin());
        let (c, d) = (y.cos(), y.sin());
        let (e, f) = (z.cos(), z.sin());

        let te = &mut self.elements;

        match euler.order {
            EulerOrder::XYZ => {
                let (ae, af, be, bf) = (a * e, a * f, b * e, b * f);

                te[0] = c * e;
                te[4] = -c * f;
                te[8] = d;

                te[1] = af + be * d;
                te[5] = ae - bf * d;
                te[9] = -b * c;

                te[2] = bf - ae * d;
                te[6] = be + af * d;
                te[10] = a * c;
            }
            EulerOrder::YXZ => {
                let (ce, cf, de, df) = (c * e, c * f, d * e, d * f);

                te[0] = ce + df * b;
                te[4] = de * b - cf;
                te[8] = a * d;

                te[1] = a * f;
                te[5] = a * e;
                te[9] = -b;

                te[2] = cf * b - de;
                te[6] = df + ce * b;
                te[10] = a * c;
            }
            EulerOrder::ZXY => {
                let (ce, cf, de, df) = (c * e, c * f, d * e, d * f);

                te[0] = ce - df * b;
                te[4] = -a * f;
                te[8] = de + cf * b;

                te[1] = cf + de * b;
                te[5] = a * e;
                te[9] = df - ce * b;

                te[2] = -a * d;
                te[6] = b;
                te[10] = a * c;
            }
            EulerOrder::ZYX => {
                let (ae, af, be, bf) = (a * e, a * f, b * e, b * f);

                te[0] = c * e;
                te[4] = be * d - af;
                te[8] = ae * d + bf;

                te[1] = c * f;
                te[5] = bf * d + ae;
                te[9] = af * d - be;

                te[2] = -d;
                te[6] = b * c;
                te[10] = a * c;
            }
            EulerOrder::YZX => {
                let (ac, ad, bc, bd) = (a * c, a * d, b * c, b * d);

                te[0] = c * e;
                te[4] = bd - ac * f;
                te[8] = bc * f + ad;

                te[1] = f;
                te[5] = a * e;
                te[9] = -b * e;

                te[2] = -d * e;
                te[6] = ad * f + bc;
                te[10] = ac - bd * f;
            }
            EulerOrder::XZY => {
                let (ac, ad, bc, bd) = (a * c, a * d, b * c, b * d);

                te[0] = c * e;
                te[4] = -f;
                te[8] = d * e;

                te[1] = ac * f + bd;
                te[5] = a * e;
                te[9] = ad * f - bc;

                te[2] = bc * f - ad;
                te[6] = b * e;
                te[10] = bd * f + ac;
            }
        }

        // bottom row
        te[3] = 0.0;
        te[7] = 0.0;
        te[11] = 0.0;

        // last column
        te[12] = 0.0;
        te[13] = 0.0;
        te[14] = 0.0;
        te[15] = 1.0;

        self
    }

    /// `Matrix4.makeRotationFromQuaternion()`.
    pub fn make_rotation_from_quaternion(&mut self, q: &Quaternion) -> &mut Self {
        self.compose(&Vector3::ZERO, q, &Vector3::new(1.0, 1.0, 1.0))
    }

    /// `Matrix4.multiply()`.
    pub fn multiply(&mut self, m: &Self) -> &mut Self {
        let a = *self;
        self.multiply_matrices(&a, m)
    }

    /// `Matrix4.premultiply()`.
    pub fn premultiply(&mut self, m: &Self) -> &mut Self {
        let b = *self;
        self.multiply_matrices(m, &b)
    }

    /// `Matrix4.multiplyScalar()`.
    pub fn multiply_scalar(&mut self, s: f64) -> &mut Self {
        for e in self.elements.iter_mut() {
            *e *= s;
        }
        self
    }

    /// `Matrix4.determinant()`.
    pub fn determinant(&self) -> f64 {
        let te = &self.elements;

        let (n11, n12, n13, n14) = (te[0], te[4], te[8], te[12]);
        let (n21, n22, n23, n24) = (te[1], te[5], te[9], te[13]);
        let (n31, n32, n33, n34) = (te[2], te[6], te[10], te[14]);
        let (n41, n42, n43, n44) = (te[3], te[7], te[11], te[15]);

        let t11 = n23 * n34 - n24 * n33;
        let t12 = n22 * n34 - n24 * n32;
        let t13 = n22 * n33 - n23 * n32;
        let t21 = n21 * n34 - n24 * n31;
        let t22 = n21 * n33 - n23 * n31;
        let t23 = n21 * n32 - n22 * n31;

        n11 * (n42 * t11 - n43 * t12 + n44 * t13) - n12 * (n41 * t11 - n43 * t21 + n44 * t22)
            + n13 * (n41 * t12 - n42 * t21 + n44 * t23)
            - n14 * (n41 * t13 - n42 * t22 + n43 * t23)
    }

    /// `Matrix4.determinantAffine()` — the determinant of the upper 3x3.
    pub fn determinant_affine(&self) -> f64 {
        let te = &self.elements;

        let (n11, n12, n13) = (te[0], te[4], te[8]);
        let (n21, n22, n23) = (te[1], te[5], te[9]);
        let (n31, n32, n33) = (te[2], te[6], te[10]);

        n11 * (n22 * n33 - n23 * n32) - n12 * (n21 * n33 - n23 * n31)
            + n13 * (n21 * n32 - n22 * n31)
    }

    /// `Matrix4.transpose()`.
    pub fn transpose(&mut self) -> &mut Self {
        let te = &mut self.elements;
        te.swap(1, 4);
        te.swap(2, 8);
        te.swap(6, 9);
        te.swap(3, 12);
        te.swap(7, 13);
        te.swap(11, 14);
        self
    }

    /// `Matrix4.setPosition()`.
    pub fn set_position(&mut self, x: f64, y: f64, z: f64) -> &mut Self {
        let te = &mut self.elements;
        te[12] = x;
        te[13] = y;
        te[14] = z;
        self
    }

    /// `Matrix4.scale( v )`.
    pub fn scale(&mut self, v: &Vector3) -> &mut Self {
        let (x, y, z) = (v.x, v.y, v.z);
        let te = &mut self.elements;

        te[0] *= x;
        te[4] *= y;
        te[8] *= z;
        te[1] *= x;
        te[5] *= y;
        te[9] *= z;
        te[2] *= x;
        te[6] *= y;
        te[10] *= z;
        te[3] *= x;
        te[7] *= y;
        te[11] *= z;

        self
    }

    /// `Matrix4.getMaxScaleOnAxis()`.
    pub fn get_max_scale_on_axis(&self) -> f64 {
        let te = &self.elements;

        let scale_x_sq = te[0] * te[0] + te[1] * te[1] + te[2] * te[2];
        let scale_y_sq = te[4] * te[4] + te[5] * te[5] + te[6] * te[6];
        let scale_z_sq = te[8] * te[8] + te[9] * te[9] + te[10] * te[10];

        let max = crate::math::math_utils::js_max(
            crate::math::math_utils::js_max(scale_x_sq, scale_y_sq),
            scale_z_sq,
        );

        max.sqrt()
    }

    /// `Matrix4.makeTranslation()`.
    pub fn make_translation(&mut self, x: f64, y: f64, z: f64) -> &mut Self {
        self.set(
            1.0, 0.0, 0.0, x, //
            0.0, 1.0, 0.0, y, //
            0.0, 0.0, 1.0, z, //
            0.0, 0.0, 0.0, 1.0,
        )
    }

    /// `Matrix4.makeRotationX()`.
    pub fn make_rotation_x(&mut self, theta: f64) -> &mut Self {
        let (c, s) = (theta.cos(), theta.sin());

        self.set(
            1.0, 0.0, 0.0, 0.0, //
            0.0, c, -s, 0.0, //
            0.0, s, c, 0.0, //
            0.0, 0.0, 0.0, 1.0,
        )
    }

    /// `Matrix4.makeRotationY()`.
    pub fn make_rotation_y(&mut self, theta: f64) -> &mut Self {
        let (c, s) = (theta.cos(), theta.sin());

        self.set(
            c, 0.0, s, 0.0, //
            0.0, 1.0, 0.0, 0.0, //
            -s, 0.0, c, 0.0, //
            0.0, 0.0, 0.0, 1.0,
        )
    }

    /// `Matrix4.makeRotationZ()`.
    pub fn make_rotation_z(&mut self, theta: f64) -> &mut Self {
        let (c, s) = (theta.cos(), theta.sin());

        self.set(
            c, -s, 0.0, 0.0, //
            s, c, 0.0, 0.0, //
            0.0, 0.0, 1.0, 0.0, //
            0.0, 0.0, 0.0, 1.0,
        )
    }

    /// `Matrix4.makeRotationAxis()`. `axis` is assumed to be normalized.
    pub fn make_rotation_axis(&mut self, axis: &Vector3, angle: f64) -> &mut Self {
        let c = angle.cos();
        let s = angle.sin();
        let t = 1.0 - c;
        let (x, y, z) = (axis.x, axis.y, axis.z);
        let (tx, ty) = (t * x, t * y);

        self.set(
            tx * x + c,
            tx * y - s * z,
            tx * z + s * y,
            0.0, //
            tx * y + s * z,
            ty * y + c,
            ty * z - s * x,
            0.0, //
            tx * z - s * y,
            ty * z + s * x,
            t * z * z + c,
            0.0, //
            0.0,
            0.0,
            0.0,
            1.0,
        )
    }

    /// `Matrix4.makeShear()`.
    pub fn make_shear(
        &mut self,
        xy: f64,
        xz: f64,
        yx: f64,
        yz: f64,
        zx: f64,
        zy: f64,
    ) -> &mut Self {
        self.set(
            1.0, yx, zx, 0.0, //
            xy, 1.0, zy, 0.0, //
            xz, yz, 1.0, 0.0, //
            0.0, 0.0, 0.0, 1.0,
        )
    }

    /// `Matrix4.decompose()`.
    pub fn decompose(
        &self,
        position: &mut Vector3,
        quaternion: &mut Quaternion,
        scale: &mut Vector3,
    ) {
        let te = &self.elements;

        position.x = te[12];
        position.y = te[13];
        position.z = te[14];

        let det = self.determinant_affine();

        if det == 0.0 {
            scale.set(1.0, 1.0, 1.0);
            quaternion.identity();
            return;
        }

        let mut v1 = Vector3::ZERO;

        let mut sx = v1.set(te[0], te[1], te[2]).length();
        let sy = v1.set(te[4], te[5], te[6]).length();
        let sz = v1.set(te[8], te[9], te[10]).length();

        // if determinant is negative, we need to invert one scale
        if det < 0.0 {
            sx = -sx;
        }

        // scale the rotation part
        let mut m1 = *self;

        let inv_sx = 1.0 / sx;
        let inv_sy = 1.0 / sy;
        let inv_sz = 1.0 / sz;

        m1.elements[0] *= inv_sx;
        m1.elements[1] *= inv_sx;
        m1.elements[2] *= inv_sx;

        m1.elements[4] *= inv_sy;
        m1.elements[5] *= inv_sy;
        m1.elements[6] *= inv_sy;

        m1.elements[8] *= inv_sz;
        m1.elements[9] *= inv_sz;
        m1.elements[10] *= inv_sz;

        quaternion.set_from_rotation_matrix(&m1);

        scale.x = sx;
        scale.y = sy;
        scale.z = sz;
    }

    /// `Matrix4.makeOrthographic()`, `reversedDepth = false`.
    #[allow(clippy::too_many_arguments)]
    pub fn make_orthographic(
        &mut self,
        left: f64,
        right: f64,
        top: f64,
        bottom: f64,
        near: f64,
        far: f64,
        coordinate_system: CoordinateSystem,
    ) -> &mut Self {
        let x = 2.0 / (right - left);
        let y = 2.0 / (top - bottom);

        let a = -(right + left) / (right - left);
        let b = -(top + bottom) / (top - bottom);

        let (c, d) = match coordinate_system {
            CoordinateSystem::WebGL => (-2.0 / (far - near), -(far + near) / (far - near)),
            CoordinateSystem::WebGPU => (-1.0 / (far - near), -near / (far - near)),
        };

        let te = &mut self.elements;
        te[0] = x;
        te[4] = 0.0;
        te[8] = 0.0;
        te[12] = a;
        te[1] = 0.0;
        te[5] = y;
        te[9] = 0.0;
        te[13] = b;
        te[2] = 0.0;
        te[6] = 0.0;
        te[10] = c;
        te[14] = d;
        te[3] = 0.0;
        te[7] = 0.0;
        te[11] = 0.0;
        te[15] = 1.0;

        self
    }

    /// `Matrix4.equals()`.
    pub fn equals(&self, m: &Self) -> bool {
        self.elements == m.elements
    }

    /// `Matrix4.fromArray()`.
    pub fn from_array(&mut self, array: &[f64], offset: usize) -> &mut Self {
        self.elements.copy_from_slice(&array[offset..offset + 16]);
        self
    }

    /// `Matrix4.toArray()`.
    pub fn to_array(&self) -> [f64; 16] {
        self.elements
    }
}
