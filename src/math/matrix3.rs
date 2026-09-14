//! Port of `three.js/src/math/Matrix3.js` (rung 2 subset).
//!
//! Column-major like `Matrix4`: `elements[0..3]` is the first column.

use super::{Matrix4, Vector3};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Matrix3 {
    pub elements: [f64; 9],
}

impl Default for Matrix3 {
    fn default() -> Self {
        Self::identity()
    }
}

impl Matrix3 {
    pub const fn identity() -> Self {
        Self {
            elements: [1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0],
        }
    }

    /// `Matrix3.set( n11, n12, n13, n21, ... )` — row-order arguments into a
    /// column-major store, exactly as three.js writes them.
    #[allow(clippy::too_many_arguments)]
    pub fn set(
        &mut self,
        n11: f64,
        n12: f64,
        n13: f64,
        n21: f64,
        n22: f64,
        n23: f64,
        n31: f64,
        n32: f64,
        n33: f64,
    ) -> &mut Self {
        let te = &mut self.elements;
        te[0] = n11;
        te[1] = n21;
        te[2] = n31;
        te[3] = n12;
        te[4] = n22;
        te[5] = n32;
        te[6] = n13;
        te[7] = n23;
        te[8] = n33;
        self
    }

    /// `Matrix3.setFromMatrix4()`.
    pub fn set_from_matrix4(&mut self, m: &Matrix4) -> &mut Self {
        let me = &m.elements;
        self.set(
            me[0], me[4], me[8], //
            me[1], me[5], me[9], //
            me[2], me[6], me[10],
        )
    }

    /// `Matrix3.invert()`.
    pub fn invert(&mut self) -> &mut Self {
        let te = &mut self.elements;

        let (n11, n21, n31) = (te[0], te[1], te[2]);
        let (n12, n22, n32) = (te[3], te[4], te[5]);
        let (n13, n23, n33) = (te[6], te[7], te[8]);

        let t11 = n33 * n22 - n32 * n23;
        let t12 = n32 * n13 - n33 * n12;
        let t13 = n23 * n12 - n22 * n13;

        let det = n11 * t11 + n21 * t12 + n31 * t13;

        if det == 0.0 {
            *te = [0.0; 9];
            return self;
        }

        let det_inv = 1.0 / det;

        te[0] = t11 * det_inv;
        te[1] = (n31 * n23 - n33 * n21) * det_inv;
        te[2] = (n32 * n21 - n31 * n22) * det_inv;

        te[3] = t12 * det_inv;
        te[4] = (n33 * n11 - n31 * n13) * det_inv;
        te[5] = (n31 * n12 - n32 * n11) * det_inv;

        te[6] = t13 * det_inv;
        te[7] = (n21 * n13 - n23 * n11) * det_inv;
        te[8] = (n22 * n11 - n21 * n12) * det_inv;

        self
    }

    /// `Matrix3.transpose()`.
    pub fn transpose(&mut self) -> &mut Self {
        let m = &mut self.elements;
        m.swap(1, 3);
        m.swap(2, 6);
        m.swap(5, 7);
        self
    }

    /// `Matrix3.getNormalMatrix( matrix4 )`.
    pub fn get_normal_matrix(&mut self, m: &Matrix4) -> &mut Self {
        self.set_from_matrix4(m).invert().transpose()
    }

    /// `Matrix3.identity()`.
    pub fn set_identity(&mut self) -> &mut Self {
        self.set(1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0)
    }

    /// `Matrix3.copy()`.
    pub fn copy(&mut self, m: &Self) -> &mut Self {
        self.elements = m.elements;
        self
    }

    /// `Matrix3.extractBasis()`.
    pub fn extract_basis(&self, x_axis: &mut Vector3, y_axis: &mut Vector3, z_axis: &mut Vector3) {
        x_axis.set_from_matrix3_column(self, 0);
        y_axis.set_from_matrix3_column(self, 1);
        z_axis.set_from_matrix3_column(self, 2);
    }

    /// `Matrix3.multiply()`.
    pub fn multiply(&mut self, m: &Self) -> &mut Self {
        let a = *self;
        self.multiply_matrices(&a, m)
    }

    /// `Matrix3.premultiply()`.
    pub fn premultiply(&mut self, m: &Self) -> &mut Self {
        let b = *self;
        self.multiply_matrices(m, &b)
    }

    /// `Matrix3.multiplyMatrices()`.
    pub fn multiply_matrices(&mut self, a: &Self, b: &Self) -> &mut Self {
        let ae = &a.elements;
        let be = &b.elements;

        let (a11, a12, a13) = (ae[0], ae[3], ae[6]);
        let (a21, a22, a23) = (ae[1], ae[4], ae[7]);
        let (a31, a32, a33) = (ae[2], ae[5], ae[8]);

        let (b11, b12, b13) = (be[0], be[3], be[6]);
        let (b21, b22, b23) = (be[1], be[4], be[7]);
        let (b31, b32, b33) = (be[2], be[5], be[8]);

        let te = &mut self.elements;

        te[0] = a11 * b11 + a12 * b21 + a13 * b31;
        te[3] = a11 * b12 + a12 * b22 + a13 * b32;
        te[6] = a11 * b13 + a12 * b23 + a13 * b33;

        te[1] = a21 * b11 + a22 * b21 + a23 * b31;
        te[4] = a21 * b12 + a22 * b22 + a23 * b32;
        te[7] = a21 * b13 + a22 * b23 + a23 * b33;

        te[2] = a31 * b11 + a32 * b21 + a33 * b31;
        te[5] = a31 * b12 + a32 * b22 + a33 * b32;
        te[8] = a31 * b13 + a32 * b23 + a33 * b33;

        self
    }

    /// `Matrix3.multiplyScalar()`.
    pub fn multiply_scalar(&mut self, s: f64) -> &mut Self {
        for e in self.elements.iter_mut() {
            *e *= s;
        }
        self
    }

    /// `Matrix3.determinant()`.
    pub fn determinant(&self) -> f64 {
        let te = &self.elements;
        let (a, b, c) = (te[0], te[1], te[2]);
        let (d, e, f) = (te[3], te[4], te[5]);
        let (g, h, i) = (te[6], te[7], te[8]);

        a * e * i - a * f * h - b * d * i + b * f * g + c * d * h - c * e * g
    }

    /// `Matrix3.transposeIntoArray()`.
    pub fn transpose_into_array(&self) -> [f64; 9] {
        let m = &self.elements;
        [m[0], m[3], m[6], m[1], m[4], m[7], m[2], m[5], m[8]]
    }

    /// `Matrix3.setUvTransform()`.
    #[allow(clippy::too_many_arguments)]
    pub fn set_uv_transform(
        &mut self,
        tx: f64,
        ty: f64,
        sx: f64,
        sy: f64,
        rotation: f64,
        cx: f64,
        cy: f64,
    ) -> &mut Self {
        let c = rotation.cos();
        let s = rotation.sin();

        self.set(
            sx * c,
            sx * s,
            -sx * (c * cx + s * cy) + cx + tx,
            -sy * s,
            sy * c,
            -sy * (-s * cx + c * cy) + cy + ty,
            0.0,
            0.0,
            1.0,
        )
    }

    /// `Matrix3.makeTranslation()`.
    pub fn make_translation(&mut self, x: f64, y: f64) -> &mut Self {
        self.set(1.0, 0.0, x, 0.0, 1.0, y, 0.0, 0.0, 1.0)
    }

    /// `Matrix3.makeRotation()` — counter-clockwise.
    pub fn make_rotation(&mut self, theta: f64) -> &mut Self {
        let c = theta.cos();
        let s = theta.sin();

        self.set(c, -s, 0.0, s, c, 0.0, 0.0, 0.0, 1.0)
    }

    /// `Matrix3.makeScale()`.
    pub fn make_scale(&mut self, x: f64, y: f64) -> &mut Self {
        self.set(x, 0.0, 0.0, 0.0, y, 0.0, 0.0, 0.0, 1.0)
    }

    /// `Matrix3.equals()`.
    pub fn equals(&self, m: &Self) -> bool {
        self.elements == m.elements
    }

    /// `Matrix3.fromArray()`.
    pub fn from_array(&mut self, array: &[f64], offset: usize) -> &mut Self {
        self.elements.copy_from_slice(&array[offset..offset + 9]);
        self
    }

    /// `Matrix3.toArray()`.
    pub fn to_array(&self) -> [f64; 9] {
        self.elements
    }

    /// The nine elements narrowed to `f32`, laid out as three `vec4` columns —
    /// the std140/WGSL uniform layout of a `mat3x3<f32>`.
    pub fn to_padded_f32_array(&self) -> [f32; 12] {
        let e = &self.elements;
        [
            e[0] as f32,
            e[1] as f32,
            e[2] as f32,
            0.0, //
            e[3] as f32,
            e[4] as f32,
            e[5] as f32,
            0.0, //
            e[6] as f32,
            e[7] as f32,
            e[8] as f32,
            0.0,
        ]
    }
}
