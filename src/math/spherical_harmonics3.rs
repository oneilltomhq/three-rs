//! Port of `three.js/src/math/SphericalHarmonics3.js`.

use super::Vector3;

/// A third-order spherical harmonics (SH).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SphericalHarmonics3 {
    /// The (9) SH coefficients.
    pub coefficients: [Vector3; 9],
}

impl Default for SphericalHarmonics3 {
    /// `new SphericalHarmonics3()`: nine zeroed coefficients.
    fn default() -> Self {
        Self {
            coefficients: [Vector3::ZERO; 9],
        }
    }
}

impl SphericalHarmonics3 {
    /// `SphericalHarmonics3.isSphericalHarmonics3`.
    pub const IS_SPHERICAL_HARMONICS3: bool = true;

    /// `SphericalHarmonics3.set()`.
    pub fn set(&mut self, coefficients: &[Vector3; 9]) -> &mut Self {
        for i in 0..9 {
            let c = coefficients[i];
            self.coefficients[i].copy(&c);
        }

        self
    }

    /// `SphericalHarmonics3.zero()`.
    pub fn zero(&mut self) -> &mut Self {
        for i in 0..9 {
            self.coefficients[i].set(0.0, 0.0, 0.0);
        }

        self
    }

    /// `SphericalHarmonics3.getAt()`: the radiance in the direction of `normal`.
    pub fn get_at(&self, normal: &Vector3) -> Vector3 {
        // normal is assumed to be unit length

        let (x, y, z) = (normal.x, normal.y, normal.z);

        let coeff = &self.coefficients;

        let mut target = Vector3::ZERO;

        // band 0
        target.copy(&coeff[0]).multiply_scalar(0.282095);

        // band 1
        target.add_scaled_vector(&coeff[1], 0.488603 * y);
        target.add_scaled_vector(&coeff[2], 0.488603 * z);
        target.add_scaled_vector(&coeff[3], 0.488603 * x);

        // band 2
        target.add_scaled_vector(&coeff[4], 1.092548 * (x * y));
        target.add_scaled_vector(&coeff[5], 1.092548 * (y * z));
        target.add_scaled_vector(&coeff[6], 0.315392 * (3.0 * z * z - 1.0));
        target.add_scaled_vector(&coeff[7], 1.092548 * (x * z));
        target.add_scaled_vector(&coeff[8], 0.546274 * (x * x - y * y));

        target
    }

    /// `SphericalHarmonics3.getIrradianceAt()`: the irradiance (radiance
    /// convolved with cosine lobe) in the direction of `normal`.
    pub fn get_irradiance_at(&self, normal: &Vector3) -> Vector3 {
        // normal is assumed to be unit length

        let (x, y, z) = (normal.x, normal.y, normal.z);

        let coeff = &self.coefficients;

        let mut target = Vector3::ZERO;

        // band 0
        target.copy(&coeff[0]).multiply_scalar(0.886227); // π * 0.282095

        // band 1
        target.add_scaled_vector(&coeff[1], 2.0 * 0.511664 * y); // ( 2 * π / 3 ) * 0.488603
        target.add_scaled_vector(&coeff[2], 2.0 * 0.511664 * z);
        target.add_scaled_vector(&coeff[3], 2.0 * 0.511664 * x);

        // band 2
        target.add_scaled_vector(&coeff[4], 2.0 * 0.429043 * x * y); // ( π / 4 ) * 1.092548
        target.add_scaled_vector(&coeff[5], 2.0 * 0.429043 * y * z);
        target.add_scaled_vector(&coeff[6], 0.743125 * z * z - 0.247708); // ( π / 4 ) * 0.315392 * 3
        target.add_scaled_vector(&coeff[7], 2.0 * 0.429043 * x * z);
        target.add_scaled_vector(&coeff[8], 0.429043 * (x * x - y * y)); // ( π / 4 ) * 0.546274

        target
    }

    /// `SphericalHarmonics3.add()`.
    pub fn add(&mut self, sh: &Self) -> &mut Self {
        for i in 0..9 {
            let c = sh.coefficients[i];
            self.coefficients[i].add(&c);
        }

        self
    }

    /// `SphericalHarmonics3.addScaledSH()`.
    pub fn add_scaled_sh(&mut self, sh: &Self, s: f64) -> &mut Self {
        for i in 0..9 {
            let c = sh.coefficients[i];
            self.coefficients[i].add_scaled_vector(&c, s);
        }

        self
    }

    /// `SphericalHarmonics3.scale()`.
    pub fn scale(&mut self, s: f64) -> &mut Self {
        for i in 0..9 {
            self.coefficients[i].multiply_scalar(s);
        }

        self
    }

    /// `SphericalHarmonics3.lerp()`.
    pub fn lerp(&mut self, sh: &Self, alpha: f64) -> &mut Self {
        for i in 0..9 {
            let c = sh.coefficients[i];
            self.coefficients[i].lerp(&c, alpha);
        }

        self
    }

    /// `SphericalHarmonics3.equals()`.
    pub fn equals(&self, sh: &Self) -> bool {
        for i in 0..9 {
            if !self.coefficients[i].equals(&sh.coefficients[i]) {
                return false;
            }
        }

        true
    }

    /// `SphericalHarmonics3.copy()`.
    pub fn copy(&mut self, sh: &Self) -> &mut Self {
        self.set(&sh.coefficients)
    }

    /// `SphericalHarmonics3.fromArray()`.
    pub fn from_array(&mut self, array: &[f64], offset: usize) -> &mut Self {
        let coefficients = &mut self.coefficients;

        for i in 0..9 {
            coefficients[i].from_array(array, offset + (i * 3));
        }

        self
    }

    /// `SphericalHarmonics3.toArray()`, flat: 9 coefficients of 3 components.
    pub fn to_array(&self) -> [f64; 27] {
        let mut array = [0.0; 27];
        self.to_array_into(&mut array, 0);
        array
    }

    /// `SphericalHarmonics3.toArray( array, offset )`.
    pub fn to_array_into(&self, array: &mut [f64], offset: usize) {
        let coefficients = &self.coefficients;

        for i in 0..9 {
            let c = coefficients[i].to_array();
            let o = offset + (i * 3);
            array[o] = c[0];
            array[o + 1] = c[1];
            array[o + 2] = c[2];
        }
    }

    /// `SphericalHarmonics3.getBasisAt()`: the SH basis for `normal`.
    pub fn static_get_basis_at(normal: &Vector3) -> [f64; 9] {
        // normal is assumed to be unit length

        let (x, y, z) = (normal.x, normal.y, normal.z);

        let mut sh_basis = [0.0; 9];

        // band 0
        sh_basis[0] = 0.282095;

        // band 1
        sh_basis[1] = 0.488603 * y;
        sh_basis[2] = 0.488603 * z;
        sh_basis[3] = 0.488603 * x;

        // band 2
        sh_basis[4] = 1.092548 * x * y;
        sh_basis[5] = 1.092548 * y * z;
        sh_basis[6] = 0.315392 * (3.0 * z * z - 1.0);
        sh_basis[7] = 1.092548 * x * z;
        sh_basis[8] = 0.546274 * (x * x - y * y);

        sh_basis
    }
}
