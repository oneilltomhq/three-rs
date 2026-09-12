//! Small pieces of `three.js/src/math` that the crate's `src/math` does not
//! have yet and that the geometry generators need. Kept here so `src/math`
//! stays untouched; fold them in when the math module is next edited.

use crate::math::Matrix4;

/// `Matrix4.set()` — arguments are in row-major order, `elements` is stored
/// column-major.
#[allow(clippy::too_many_arguments)]
pub fn matrix4_set(
    m: &mut Matrix4,
    n11: f64, n12: f64, n13: f64, n14: f64,
    n21: f64, n22: f64, n23: f64, n24: f64,
    n31: f64, n32: f64, n33: f64, n34: f64,
    n41: f64, n42: f64, n43: f64, n44: f64,
) {
    let te = &mut m.elements;
    te[0] = n11; te[4] = n12; te[8] = n13; te[12] = n14;
    te[1] = n21; te[5] = n22; te[9] = n23; te[13] = n24;
    te[2] = n31; te[6] = n32; te[10] = n33; te[14] = n34;
    te[3] = n41; te[7] = n42; te[11] = n43; te[15] = n44;
}

/// `Matrix4.transpose()`.
pub fn matrix4_transpose(m: &mut Matrix4) {
    let te = &mut m.elements;
    te.swap(1, 4);
    te.swap(2, 8);
    te.swap(6, 9);
    te.swap(3, 12);
    te.swap(7, 13);
    te.swap(11, 14);
}

/// `three.js/src/math/Vector4.js`, reduced to what `TeapotGeometry` uses.
#[derive(Clone, Copy, Debug, Default)]
pub struct Vector4 {
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub w: f64,
}

impl Vector4 {
    pub const ZERO: Self = Self { x: 0.0, y: 0.0, z: 0.0, w: 0.0 };

    /// `Vector4.fromArray()`.
    pub fn from_array(a: &[f64; 4]) -> Self {
        Self { x: a[0], y: a[1], z: a[2], w: a[3] }
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

    /// `Vector4.dot()`.
    pub fn dot(&self, v: &Self) -> f64 {
        self.x * v.x + self.y * v.y + self.z * v.z + self.w * v.w
    }
}

/// `Vector3.lerp()`.
pub fn vector3_lerp(v: &mut crate::math::Vector3, target: &crate::math::Vector3, alpha: f64) {
    v.x += (target.x - v.x) * alpha;
    v.y += (target.y - v.y) * alpha;
    v.z += (target.z - v.z) * alpha;
}
