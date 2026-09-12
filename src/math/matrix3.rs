//! Port of `three.js/src/math/Matrix3.js` (rung 2 subset).
//!
//! Column-major like `Matrix4`: `elements[0..3]` is the first column.

use super::Matrix4;

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

    /// The nine elements narrowed to `f32`, laid out as three `vec4` columns —
    /// the std140/WGSL uniform layout of a `mat3x3<f32>`.
    pub fn to_padded_f32_array(&self) -> [f32; 12] {
        let e = &self.elements;
        [
            e[0] as f32, e[1] as f32, e[2] as f32, 0.0, //
            e[3] as f32, e[4] as f32, e[5] as f32, 0.0, //
            e[6] as f32, e[7] as f32, e[8] as f32, 0.0,
        ]
    }
}
