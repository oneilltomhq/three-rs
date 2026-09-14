//! Port of `three.js/src/math/Matrix2.js`.
//!
//! A note on row-major and column-major ordering: [`Matrix2::new`] and
//! [`Matrix2::set`] take their arguments in row-major order, while
//! [`Matrix2::elements`] stores them in column-major order. So
//! `Matrix2::new( 11.0, 12.0, 21.0, 22.0 )` gives
//! `elements == [ 11.0, 21.0, 12.0, 22.0 ]`.

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Matrix2 {
    /// A column-major list of matrix values.
    pub elements: [f64; 4],
}

impl Default for Matrix2 {
    /// `new Matrix2()` with no arguments: the identity matrix.
    fn default() -> Self {
        Self {
            elements: [
                1.0, 0.0, //
                0.0, 1.0,
            ],
        }
    }
}

impl Matrix2 {
    /// `new Matrix2( n11, n12, n21, n22 )` — arguments in row-major order.
    pub fn new(n11: f64, n12: f64, n21: f64, n22: f64) -> Self {
        let mut m = Self::default();
        m.set(n11, n12, n21, n22);
        m
    }

    /// `Matrix2.identity()`.
    pub fn identity(&mut self) -> &mut Self {
        self.set(
            1.0, 0.0, //
            0.0, 1.0,
        );

        self
    }

    /// `Matrix2.fromArray()` — the array holds the elements in column-major order.
    pub fn from_array(&mut self, array: &[f64], offset: usize) -> &mut Self {
        self.elements.copy_from_slice(&array[offset..offset + 4]);

        self
    }

    /// `Matrix2.set()` — arguments in row-major order.
    pub fn set(&mut self, n11: f64, n12: f64, n21: f64, n22: f64) -> &mut Self {
        let te = &mut self.elements;

        te[0] = n11;
        te[2] = n12;
        te[1] = n21;
        te[3] = n22;

        self
    }
}
