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
use super::{Color, Matrix3};

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

/// The colour spaces `constants.js` names, as far as the port needs them.
///
/// three.js spells a colour space as a string constant, and one set of
/// constants is shared by `ColorManagement`, `Color`'s `set*`/`get*`
/// methods, `Texture.colorSpace` and `renderer.outputColorSpace`. This enum is
/// that set, and the one type all four take.
///
/// `ColorManagement` defines the two spaces three.js registers by default:
/// linear sRGB (the working space) and sRGB. Display P3 and the other spaces
/// in `addons/math/ColorSpaces.js` are not ported, which is why the enum is
/// `#[non_exhaustive]`. The KTX2 loader reports what a file declares in its
/// own [`Ktx2ColorSpace`](crate::loaders::Ktx2ColorSpace), which does have P3.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum ColorSpace {
    /// `NoColorSpace` (`''`): no colour space at all. `ColorManagement`
    /// converts nothing to or from it, and a texture tagged with it is sampled
    /// with no transfer function. It is `Texture.colorSpace`'s default.
    NoColorSpace,
    /// `SRGBColorSpace` (`'srgb'`): Rec. 709 primaries, sRGB transfer.
    Srgb,
    /// `LinearSRGBColorSpace` (`'srgb-linear'`): Rec. 709 primaries, linear
    /// transfer. `ColorManagement.workingColorSpace`.
    LinearSrgb,
}

impl ColorSpace {
    /// `spaces[ colorSpace ].transfer`, with `NoColorSpace` answering
    /// `LinearTransfer` as `ColorManagement.getTransfer()` does.
    pub(crate) fn transfer(self) -> Transfer {
        match self {
            ColorSpace::Srgb => Transfer::Srgb,
            ColorSpace::LinearSrgb | ColorSpace::NoColorSpace => Transfer::Linear,
        }
    }
}

/// The transfer functions `constants.js` names, i.e. `LinearTransfer` and
/// `SRGBTransfer`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Transfer {
    /// `LinearTransfer`.
    Linear,
    /// `SRGBTransfer`.
    Srgb,
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
            working_color_space: ColorSpace::LinearSrgb,
        }
    }
}

impl ColorManagement {
    /// `ColorManagement.convert()`. It is a no-op when either space is
    /// [`ColorSpace::NoColorSpace`], three.js' falsy `''`.
    pub fn convert(
        &self,
        color: &mut Color,
        source_color_space: ColorSpace,
        target_color_space: ColorSpace,
    ) {
        if !self.enabled
            || source_color_space == target_color_space
            || source_color_space == ColorSpace::NoColorSpace
            || target_color_space == ColorSpace::NoColorSpace
        {
            return;
        }

        if Self::space_transfer(source_color_space) == Transfer::Srgb {
            color.r = srgb_to_linear(color.r);
            color.g = srgb_to_linear(color.g);
            color.b = srgb_to_linear(color.b);
        }

        if Self::space_primaries(source_color_space) != Self::space_primaries(target_color_space) {
            color.apply_matrix3(&Self::space_to_xyz(source_color_space));
            color.apply_matrix3(&Self::space_from_xyz(target_color_space));
        }

        if Self::space_transfer(target_color_space) == Transfer::Srgb {
            color.r = linear_to_srgb(color.r);
            color.g = linear_to_srgb(color.g);
            color.b = linear_to_srgb(color.b);
        }
    }

    /// `ColorManagement.workingToColorSpace()`.
    pub fn working_to_color_space(&self, color: &mut Color, target_color_space: ColorSpace) {
        self.convert(color, self.working_color_space, target_color_space);
    }

    /// `ColorManagement.colorSpaceToWorking()`.
    pub fn color_space_to_working(&self, color: &mut Color, source_color_space: ColorSpace) {
        self.convert(color, source_color_space, self.working_color_space);
    }

    /// `ColorManagement.getPrimaries()`. Both defined spaces have Rec. 709
    /// primaries. `NoColorSpace` has none, and three.js' lookup would throw;
    /// here it answers Rec. 709 too.
    pub fn get_primaries(&self, color_space: ColorSpace) -> [f64; 6] {
        Self::space_primaries(color_space)
    }

    /// `ColorManagement.getTransfer()`: `NoColorSpace` answers
    /// `LinearTransfer`, as in three.js.
    pub fn get_transfer(&self, color_space: ColorSpace) -> Transfer {
        color_space.transfer()
    }

    /// `ColorManagement.getLuminanceCoefficients()`; `None` means the working
    /// colour space, as in three.js' default argument.
    pub fn get_luminance_coefficients(&self, color_space: Option<ColorSpace>) -> [f64; 3] {
        let _ = color_space.unwrap_or(self.working_color_space);
        REC709_LUMINANCE_COEFFICIENTS
    }

    /// `spaces[ colorSpace ].primaries`.
    fn space_primaries(_color_space: ColorSpace) -> [f64; 6] {
        REC709_PRIMARIES
    }

    /// `spaces[ colorSpace ].transfer`.
    fn space_transfer(color_space: ColorSpace) -> Transfer {
        color_space.transfer()
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
