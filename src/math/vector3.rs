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

    pub fn dot(&self, v: &Self) -> f64 {
        self.x * v.x + self.y * v.y + self.z * v.z
    }
}
