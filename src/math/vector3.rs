//! Port of `three.js/src/math/Vector3.js` (only what rung 1 needs).
//!
//! All arithmetic is `f64`, matching JavaScript number semantics: three.js
//! computes every transform in doubles and only narrows to `f32` when the
//! value reaches a buffer or a uniform.

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

    pub fn sub_vectors(&mut self, a: &Self, b: &Self) -> &mut Self {
        self.x = a.x - b.x;
        self.y = a.y - b.y;
        self.z = a.z - b.z;
        self
    }

    pub fn add_vectors(&mut self, a: &Self, b: &Self) -> &mut Self {
        self.x = a.x + b.x;
        self.y = a.y + b.y;
        self.z = a.z + b.z;
        self
    }

    pub fn cross_vectors(&mut self, a: &Self, b: &Self) -> &mut Self {
        let (ax, ay, az) = (a.x, a.y, a.z);
        let (bx, by, bz) = (b.x, b.y, b.z);
        self.x = ay * bz - az * by;
        self.y = az * bx - ax * bz;
        self.z = ax * by - ay * bx;
        self
    }

    pub fn length(&self) -> f64 {
        (self.x * self.x + self.y * self.y + self.z * self.z).sqrt()
    }

    pub fn length_sq(&self) -> f64 {
        self.x * self.x + self.y * self.y + self.z * self.z
    }

    pub fn multiply_scalar(&mut self, s: f64) -> &mut Self {
        self.x *= s;
        self.y *= s;
        self.z *= s;
        self
    }

    pub fn divide_scalar(&mut self, s: f64) -> &mut Self {
        self.multiply_scalar(1.0 / s)
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

    /// `Vector3.applyMatrix3()`.
    pub fn apply_matrix3(&mut self, m: &crate::math::Matrix3) -> &mut Self {
        let (x, y, z) = (self.x, self.y, self.z);
        let e = &m.elements;

        self.x = e[0] * x + e[3] * y + e[6] * z;
        self.y = e[1] * x + e[4] * y + e[7] * z;
        self.z = e[2] * x + e[5] * y + e[8] * z;

        self
    }

    /// `Vector3.applyNormalMatrix()`.
    pub fn apply_normal_matrix(&mut self, m: &crate::math::Matrix3) -> &mut Self {
        self.apply_matrix3(m).normalize()
    }

    /// `Vector3.applyMatrix4()`.
    pub fn apply_matrix4(&mut self, m: &crate::math::Matrix4) -> &mut Self {
        let (x, y, z) = (self.x, self.y, self.z);
        let e = &m.elements;

        let w = 1.0 / (e[3] * x + e[7] * y + e[11] * z + e[15]);

        self.x = (e[0] * x + e[4] * y + e[8] * z + e[12]) * w;
        self.y = (e[1] * x + e[5] * y + e[9] * z + e[13]) * w;
        self.z = (e[2] * x + e[6] * y + e[10] * z + e[14]) * w;

        self
    }

    pub fn add(&mut self, v: &Self) -> &mut Self {
        self.x += v.x;
        self.y += v.y;
        self.z += v.z;
        self
    }

    pub fn cross(&mut self, v: &Self) -> &mut Self {
        let a = *self;
        self.cross_vectors(&a, v)
    }

    pub fn dot(&self, v: &Self) -> f64 {
        self.x * v.x + self.y * v.y + self.z * v.z
    }
}
