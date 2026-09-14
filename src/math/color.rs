//! Port of `three.js/src/math/Color.js`.
//!
//! three.js' colour management is on by default: a hex literal is interpreted
//! as sRGB and stored in the working colour space (linear-sRGB), which is what
//! the renderer then uses as a clear value. The working space here is always
//! `LinearSRGBColorSpace` and the only other space is `SRGBColorSpace`, so the
//! `ColorManagement.convert()` machinery collapses to the two transfer
//! functions below (the primaries never differ).

use super::math_utils::{clamp, euclidean_modulo, js_max, js_min, lerp};
use super::vector3::js_round;
use super::{Matrix3, Vector3};

/// `ColorManagement`'s two named spaces, as far as `Color` is concerned.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ColorSpace {
    /// `LinearSRGBColorSpace` — `ColorManagement.workingColorSpace`.
    LinearSRGB,
    /// `SRGBColorSpace`.
    SRGB,
}

/// The `{ h, s, l }` object `Color.getHSL()` fills in.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Hsl {
    pub h: f64,
    pub s: f64,
    pub l: f64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Color {
    pub r: f64,
    pub g: f64,
    pub b: f64,
}

impl Default for Color {
    /// `new Color()` — three.js' constructor initialises to white before
    /// applying its arguments, so a no-argument `Color` is white.
    fn default() -> Self {
        Self::new(1.0, 1.0, 1.0)
    }
}

impl Color {
    /// `new Color( r, g, b )` / `setRGB( r, g, b )` with the working colour
    /// space, i.e. the components are stored as given.
    pub const fn new(r: f64, g: f64, b: f64) -> Self {
        Self { r, g, b }
    }

    /// `new Color( hex )` — `setHex( hex, SRGBColorSpace )`.
    pub fn from_hex(hex: u32) -> Self {
        let mut c = Self::default();
        c.set_hex(hex, ColorSpace::SRGB);
        c
    }

    /// `Color.setScalar()`.
    pub fn set_scalar(&mut self, scalar: f64) -> &mut Self {
        self.r = scalar;
        self.g = scalar;
        self.b = scalar;
        self
    }

    /// `Color.setHex()`.
    pub fn set_hex(&mut self, hex: u32, color_space: ColorSpace) -> &mut Self {
        self.r = ((hex >> 16) & 255) as f64 / 255.0;
        self.g = ((hex >> 8) & 255) as f64 / 255.0;
        self.b = (hex & 255) as f64 / 255.0;

        self.color_space_to_working(color_space)
    }

    /// `Color.setRGB()`.
    pub fn set_rgb(&mut self, r: f64, g: f64, b: f64, color_space: ColorSpace) -> &mut Self {
        self.r = r;
        self.g = g;
        self.b = b;

        self.color_space_to_working(color_space)
    }

    /// `Color.setHSL()`. `h`, `s` and `l` are in 0..1.
    pub fn set_hsl(&mut self, h: f64, s: f64, l: f64, color_space: ColorSpace) -> &mut Self {
        let h = euclidean_modulo(h, 1.0);
        let s = clamp(s, 0.0, 1.0);
        let l = clamp(l, 0.0, 1.0);

        if s == 0.0 {
            self.r = l;
            self.g = l;
            self.b = l;
        } else {
            let p = if l <= 0.5 {
                l * (1.0 + s)
            } else {
                l + s - (l * s)
            };
            let q = (2.0 * l) - p;

            self.r = hue2rgb(q, p, h + 1.0 / 3.0);
            self.g = hue2rgb(q, p, h);
            self.b = hue2rgb(q, p, h - 1.0 / 3.0);
        }

        self.color_space_to_working(color_space)
    }

    /// `Color.copy()`.
    pub fn copy(&mut self, color: &Self) -> &mut Self {
        *self = *color;
        self
    }

    /// `Color.copySRGBToLinear()`.
    pub fn copy_srgb_to_linear(&mut self, color: &Self) -> &mut Self {
        self.r = srgb_to_linear(color.r);
        self.g = srgb_to_linear(color.g);
        self.b = srgb_to_linear(color.b);
        self
    }

    /// `Color.copyLinearToSRGB()`.
    pub fn copy_linear_to_srgb(&mut self, color: &Self) -> &mut Self {
        self.r = linear_to_srgb(color.r);
        self.g = linear_to_srgb(color.g);
        self.b = linear_to_srgb(color.b);
        self
    }

    /// `Color.convertSRGBToLinear()`.
    pub fn convert_srgb_to_linear(&mut self) -> &mut Self {
        let c = *self;
        self.copy_srgb_to_linear(&c)
    }

    /// `Color.convertLinearToSRGB()`.
    pub fn convert_linear_to_srgb(&mut self) -> &mut Self {
        let c = *self;
        self.copy_linear_to_srgb(&c)
    }

    /// `Color.getHex()`.
    pub fn get_hex(&self, color_space: ColorSpace) -> u32 {
        let mut c = *self;
        c.working_to_color_space(color_space);

        (js_round(clamp(c.r * 255.0, 0.0, 255.0)) as u32) * 65536
            + (js_round(clamp(c.g * 255.0, 0.0, 255.0)) as u32) * 256
            + (js_round(clamp(c.b * 255.0, 0.0, 255.0)) as u32)
    }

    /// `Color.getHexString()`.
    pub fn get_hex_string(&self, color_space: ColorSpace) -> String {
        format!("{:06x}", self.get_hex(color_space))
    }

    /// `Color.getHSL()`.
    pub fn get_hsl(&self, color_space: ColorSpace) -> Hsl {
        let mut c = *self;
        c.working_to_color_space(color_space);

        let (r, g, b) = (c.r, c.g, c.b);

        let max = js_max(js_max(r, g), b);
        let min = js_min(js_min(r, g), b);

        let hue;
        let saturation;
        let lightness = (min + max) / 2.0;

        if min == max {
            hue = 0.0;
            saturation = 0.0;
        } else {
            let delta = max - min;

            saturation = if lightness <= 0.5 {
                delta / (max + min)
            } else {
                delta / (2.0 - max - min)
            };

            let h = if max == r {
                (g - b) / delta + if g < b { 6.0 } else { 0.0 }
            } else if max == g {
                (b - r) / delta + 2.0
            } else {
                (r - g) / delta + 4.0
            };

            hue = h / 6.0;
        }

        Hsl {
            h: hue,
            s: saturation,
            l: lightness,
        }
    }

    /// `Color.getRGB()`.
    pub fn get_rgb(&self, color_space: ColorSpace) -> Self {
        let mut c = *self;
        c.working_to_color_space(color_space);
        c
    }

    /// `Color.getStyle()`, `SRGBColorSpace`.
    pub fn get_style(&self) -> String {
        let c = self.get_rgb(ColorSpace::SRGB);

        format!(
            "rgb({},{},{})",
            js_round(c.r * 255.0),
            js_round(c.g * 255.0),
            js_round(c.b * 255.0)
        )
    }

    /// `Color.offsetHSL()`.
    pub fn offset_hsl(&mut self, h: f64, s: f64, l: f64) -> &mut Self {
        let hsl = self.get_hsl(ColorSpace::LinearSRGB);
        self.set_hsl(hsl.h + h, hsl.s + s, hsl.l + l, ColorSpace::LinearSRGB)
    }

    /// `Color.add()`.
    pub fn add(&mut self, color: &Self) -> &mut Self {
        self.r += color.r;
        self.g += color.g;
        self.b += color.b;
        self
    }

    /// `Color.addColors()`.
    pub fn add_colors(&mut self, color1: &Self, color2: &Self) -> &mut Self {
        self.r = color1.r + color2.r;
        self.g = color1.g + color2.g;
        self.b = color1.b + color2.b;
        self
    }

    /// `Color.addScalar()`.
    pub fn add_scalar(&mut self, s: f64) -> &mut Self {
        self.r += s;
        self.g += s;
        self.b += s;
        self
    }

    /// `Color.sub()` — clamps at zero.
    pub fn sub(&mut self, color: &Self) -> &mut Self {
        self.r = js_max(0.0, self.r - color.r);
        self.g = js_max(0.0, self.g - color.g);
        self.b = js_max(0.0, self.b - color.b);
        self
    }

    /// `Color.multiply()`.
    pub fn multiply(&mut self, color: &Self) -> &mut Self {
        self.r *= color.r;
        self.g *= color.g;
        self.b *= color.b;
        self
    }

    /// `Color.multiplyScalar()`.
    pub fn multiply_scalar(&mut self, s: f64) -> &mut Self {
        self.r *= s;
        self.g *= s;
        self.b *= s;
        self
    }

    /// `Color.lerp()`.
    pub fn lerp(&mut self, color: &Self, alpha: f64) -> &mut Self {
        self.r += (color.r - self.r) * alpha;
        self.g += (color.g - self.g) * alpha;
        self.b += (color.b - self.b) * alpha;
        self
    }

    /// `Color.lerpColors()`.
    pub fn lerp_colors(&mut self, color1: &Self, color2: &Self, alpha: f64) -> &mut Self {
        self.r = color1.r + (color2.r - color1.r) * alpha;
        self.g = color1.g + (color2.g - color1.g) * alpha;
        self.b = color1.b + (color2.b - color1.b) * alpha;
        self
    }

    /// `Color.lerpHSL()`.
    pub fn lerp_hsl(&mut self, color: &Self, alpha: f64) -> &mut Self {
        let a = self.get_hsl(ColorSpace::LinearSRGB);
        let b = color.get_hsl(ColorSpace::LinearSRGB);

        let h = lerp(a.h, b.h, alpha);
        let s = lerp(a.s, b.s, alpha);
        let l = lerp(a.l, b.l, alpha);

        self.set_hsl(h, s, l, ColorSpace::LinearSRGB)
    }

    /// `Color.setFromVector3()`.
    pub fn set_from_vector3(&mut self, v: &Vector3) -> &mut Self {
        self.r = v.x;
        self.g = v.y;
        self.b = v.z;
        self
    }

    /// `Color.applyMatrix3()`.
    pub fn apply_matrix3(&mut self, m: &Matrix3) -> &mut Self {
        let (r, g, b) = (self.r, self.g, self.b);
        let e = &m.elements;

        self.r = e[0] * r + e[3] * g + e[6] * b;
        self.g = e[1] * r + e[4] * g + e[7] * b;
        self.b = e[2] * r + e[5] * g + e[8] * b;

        self
    }

    /// `Color.equals()`.
    pub fn equals(&self, c: &Self) -> bool {
        c.r == self.r && c.g == self.g && c.b == self.b
    }

    /// `Color.fromArray()`.
    pub fn from_array(&mut self, array: &[f64], offset: usize) -> &mut Self {
        self.r = array[offset];
        self.g = array[offset + 1];
        self.b = array[offset + 2];
        self
    }

    /// `Color.toArray()`.
    pub fn to_array(&self) -> [f64; 3] {
        [self.r, self.g, self.b]
    }

    /// `ColorManagement.colorSpaceToWorking()`: the working space is
    /// linear-sRGB, so only an sRGB source needs decoding.
    fn color_space_to_working(&mut self, source: ColorSpace) -> &mut Self {
        if source == ColorSpace::SRGB {
            self.r = srgb_to_linear(self.r);
            self.g = srgb_to_linear(self.g);
            self.b = srgb_to_linear(self.b);
        }
        self
    }

    /// `ColorManagement.workingToColorSpace()`.
    fn working_to_color_space(&mut self, target: ColorSpace) -> &mut Self {
        if target == ColorSpace::SRGB {
            self.r = linear_to_srgb(self.r);
            self.g = linear_to_srgb(self.g);
            self.b = linear_to_srgb(self.b);
        }
        self
    }
}

/// `Color.js`' `hue2rgb()`.
fn hue2rgb(p: f64, q: f64, t: f64) -> f64 {
    let mut t = t;

    if t < 0.0 {
        t += 1.0;
    }
    if t > 1.0 {
        t -= 1.0;
    }

    if t < 1.0 / 6.0 {
        return p + (q - p) * 6.0 * t;
    }
    if t < 1.0 / 2.0 {
        return q;
    }
    if t < 2.0 / 3.0 {
        return p + (q - p) * 6.0 * (2.0 / 3.0 - t);
    }

    p
}

/// `ColorManagement.SRGBToLinear()`.
pub fn srgb_to_linear(c: f64) -> f64 {
    if c < 0.04045 {
        c * 0.0773993808
    } else {
        (c * 0.9478672986 + 0.0521327014).powf(2.4)
    }
}

/// `ColorManagement.LinearToSRGB()`.
pub fn linear_to_srgb(c: f64) -> f64 {
    if c < 0.0031308 {
        c * 12.92
    } else {
        1.055 * c.powf(0.41666) - 0.055
    }
}
