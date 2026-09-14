//! Port of `three.js/src/math/ColorManagement.js`.
//!
//! Only the singleton surface lives here: the transfer functions themselves
//! (`SRGBToLinear` / `LinearToSRGB`) are already ported in
//! [`crate::math::srgb_to_linear`] / [`crate::math::linear_to_srgb`], and
//! [`Color`] already routes its own `setRGB`/`getRGB` through them, so this
//! module reuses those rather than duplicating the curves.
//!
//! three.js' `ColorManagement` is a mutable module-level singleton. Rust has no
//! mutable global without synchronisation, so it is a plain value here:
//! `ColorManagement::default()` is three.js' singleton state (`enabled: true`,
//! `workingColorSpace: LinearSRGBColorSpace`), and the caller owns it.

use super::color::{linear_to_srgb, srgb_to_linear};
use super::{Color, ColorSpace, Matrix3};

/// `LINEAR_REC709_TO_XYZ`.
pub fn linear_rec709_to_xyz() -> Matrix3 {
    *Matrix3::identity().set(
        0.4123908, 0.3575843, 0.1804808, //
        0.2126390, 0.7151687, 0.0721923, //
        0.0193308, 0.1191948, 0.9505322,
    )
}

/// `XYZ_TO_LINEAR_REC709`.
pub fn xyz_to_linear_rec709() -> Matrix3 {
    *Matrix3::identity().set(
        3.2409699, -1.5373832, -0.4986108, //
        -0.9692436, 1.8759675, 0.0415551, //
        0.0556301, -0.2039770, 1.0569715,
    )
}

/// `REC709_PRIMARIES`: chromaticity coordinates `[ rx ry gx gy bx by ]`.
pub const REC709_PRIMARIES: [f64; 6] = [0.640, 0.330, 0.300, 0.600, 0.150, 0.060];

/// `REC709_LUMINANCE_COEFFICIENTS`.
pub const REC709_LUMINANCE_COEFFICIENTS: [f64; 3] = [0.2126, 0.7152, 0.0722];

/// `D65`: the reference white `[ x y ]`.
pub const D65: [f64; 2] = [0.3127, 0.3290];

/// The transfer functions `constants.js` names, i.e. `LinearTransfer` and
/// `SRGBTransfer`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[allow(clippy::upper_case_acronyms)] // mirrors three.js's `SRGBTransfer`; public API, not renaming
pub enum Transfer {
    /// `LinearTransfer`.
    Linear,
    /// `SRGBTransfer`.
    SRGB,
}

/// `ColorManagement`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ColorManagement {
    /// `ColorManagement.enabled`.
    pub enabled: bool,
    /// `ColorManagement.workingColorSpace`.
    pub working_color_space: ColorSpace,
}

impl Default for ColorManagement {
    fn default() -> Self {
        Self {
            enabled: true,
            working_color_space: ColorSpace::LinearSRGB,
        }
    }
}

impl ColorManagement {
    /// `ColorManagement.convert()`. `None` stands in for three.js' falsy /
    /// `NoColorSpace` argument, for which `convert()` is a no-op.
    pub fn convert(
        &self,
        color: &mut Color,
        source_color_space: Option<ColorSpace>,
        target_color_space: Option<ColorSpace>,
    ) {
        let (Some(source_color_space), Some(target_color_space)) =
            (source_color_space, target_color_space)
        else {
            return;
        };

        if !self.enabled || source_color_space == target_color_space {
            return;
        }

        if Self::space_transfer(source_color_space) == Transfer::SRGB {
            color.r = srgb_to_linear(color.r);
            color.g = srgb_to_linear(color.g);
            color.b = srgb_to_linear(color.b);
        }

        if Self::space_primaries(source_color_space) != Self::space_primaries(target_color_space) {
            color.apply_matrix3(&Self::space_to_xyz(source_color_space));
            color.apply_matrix3(&Self::space_from_xyz(target_color_space));
        }

        if Self::space_transfer(target_color_space) == Transfer::SRGB {
            color.r = linear_to_srgb(color.r);
            color.g = linear_to_srgb(color.g);
            color.b = linear_to_srgb(color.b);
        }
    }

    /// `ColorManagement.workingToColorSpace()`.
    pub fn working_to_color_space(
        &self,
        color: &mut Color,
        target_color_space: Option<ColorSpace>,
    ) {
        self.convert(color, Some(self.working_color_space), target_color_space);
    }

    /// `ColorManagement.colorSpaceToWorking()`.
    pub fn color_space_to_working(
        &self,
        color: &mut Color,
        source_color_space: Option<ColorSpace>,
    ) {
        self.convert(color, source_color_space, Some(self.working_color_space));
    }

    /// `ColorManagement.fromWorkingColorSpace()` — deprecated in r177, renamed
    /// to `workingToColorSpace()`.
    pub fn from_working_color_space(
        &self,
        color: &mut Color,
        target_color_space: Option<ColorSpace>,
    ) {
        self.working_to_color_space(color, target_color_space);
    }

    /// `ColorManagement.toWorkingColorSpace()` — deprecated in r177, renamed
    /// to `colorSpaceToWorking()`.
    pub fn to_working_color_space(
        &self,
        color: &mut Color,
        source_color_space: Option<ColorSpace>,
    ) {
        self.color_space_to_working(color, source_color_space);
    }

    /// `ColorManagement.getPrimaries()`. `None` (`NoColorSpace`) has no
    /// primaries, as in three.js where the lookup would throw.
    pub fn get_primaries(&self, color_space: ColorSpace) -> [f64; 6] {
        Self::space_primaries(color_space)
    }

    /// `ColorManagement.getTransfer()`: `None` is `NoColorSpace`, which three.js
    /// answers with `LinearTransfer`.
    pub fn get_transfer(&self, color_space: Option<ColorSpace>) -> Transfer {
        match color_space {
            None => Transfer::Linear,
            Some(color_space) => Self::space_transfer(color_space),
        }
    }

    /// `ColorManagement.getLuminanceCoefficients()`; `None` means the working
    /// colour space, as in three.js' default argument.
    pub fn get_luminance_coefficients(&self, color_space: Option<ColorSpace>) -> [f64; 3] {
        let _ = color_space.unwrap_or(self.working_color_space);
        REC709_LUMINANCE_COEFFICIENTS
    }

    /// `ColorManagement._getMatrix()`.
    pub fn get_matrix(
        &self,
        source_color_space: ColorSpace,
        target_color_space: ColorSpace,
    ) -> Matrix3 {
        let mut target_matrix = Matrix3::identity();
        target_matrix
            .copy(&Self::space_to_xyz(source_color_space))
            .multiply(&Self::space_from_xyz(target_color_space));
        target_matrix
    }

    /// `ColorManagement._getDrawingBufferColorSpace()`.
    pub fn get_drawing_buffer_color_space(&self, _color_space: ColorSpace) -> ColorSpace {
        // `outputColorSpaceConfig.drawingBufferColorSpace` is `SRGBColorSpace`
        // for both spaces three.js defines.
        ColorSpace::SRGB
    }

    /// `ColorManagement._getUnpackColorSpace()`; only `LinearSRGBColorSpace`
    /// carries a `workingColorSpaceConfig` in three.js.
    pub fn get_unpack_color_space(&self, color_space: Option<ColorSpace>) -> Option<ColorSpace> {
        match color_space.unwrap_or(self.working_color_space) {
            ColorSpace::LinearSRGB => Some(ColorSpace::SRGB),
            ColorSpace::SRGB => None,
        }
    }

    /// `spaces[ colorSpace ].primaries`.
    fn space_primaries(_color_space: ColorSpace) -> [f64; 6] {
        REC709_PRIMARIES
    }

    /// `spaces[ colorSpace ].transfer`.
    fn space_transfer(color_space: ColorSpace) -> Transfer {
        match color_space {
            ColorSpace::LinearSRGB => Transfer::Linear,
            ColorSpace::SRGB => Transfer::SRGB,
        }
    }

    /// `spaces[ colorSpace ].whitePoint`.
    pub fn get_white_point(&self, _color_space: ColorSpace) -> [f64; 2] {
        D65
    }

    /// `spaces[ colorSpace ].toXYZ`.
    fn space_to_xyz(_color_space: ColorSpace) -> Matrix3 {
        linear_rec709_to_xyz()
    }

    /// `spaces[ colorSpace ].fromXYZ`.
    fn space_from_xyz(_color_space: ColorSpace) -> Matrix3 {
        xyz_to_linear_rec709()
    }
}
