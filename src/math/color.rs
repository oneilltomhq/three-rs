//! Port of `three.js/src/math/Color.js` (rung 1 subset).
//!
//! three.js' colour management is on by default: a hex literal is interpreted
//! as sRGB and stored in the working colour space (linear-sRGB), which is what
//! the renderer then uses as a clear value.

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Color {
    pub r: f64,
    pub g: f64,
    pub b: f64,
}

impl Color {
    pub const fn new(r: f64, g: f64, b: f64) -> Self {
        Self { r, g, b }
    }

    /// `new Color( hex )` — `setHex( hex, SRGBColorSpace )`.
    pub fn from_hex(hex: u32) -> Self {
        let r = ((hex >> 16) & 255) as f64 / 255.0;
        let g = ((hex >> 8) & 255) as f64 / 255.0;
        let b = (hex & 255) as f64 / 255.0;

        Self {
            r: srgb_to_linear(r),
            g: srgb_to_linear(g),
            b: srgb_to_linear(b),
        }
    }
}

/// `ColorManagement.SRGBToLinear()`.
pub fn srgb_to_linear(c: f64) -> f64 {
    if c < 0.04045 {
        c * 0.0773993808
    } else {
        (c * 0.9478672986 + 0.0521327014).powf(2.4)
    }
}
