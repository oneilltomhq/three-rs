//! Port of `three.js/src/textures/CubeTexture.js` + the `Texture.js` fields the
//! WebGPU backend reads (rung 3 subset).
//!
//! Handle semantics match JS object identity: the example hands the same
//! `CubeTexture` to `scene.background` and to `material.envMap`, and the
//! renderer must upload it once.

use std::cell::RefCell;
use std::rc::Rc;

/// `three.js/src/constants.js` colour spaces, as far as the port needs them.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ColorSpace {
    NoColorSpace,
    SRGB,
}

/// `three.js/src/constants.js` texture mappings.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mapping {
    CubeReflection,
    CubeRefraction,
}

/// One decoded image of `CubeTexture.images`, RGBA8 top-down — what
/// `ImageLoader` hands the backend after the browser has decoded the PNG.
#[derive(Clone, Debug)]
pub struct Image {
    pub width: u32,
    pub height: u32,
    pub data: Vec<u8>,
}

#[derive(Debug)]
pub struct CubeTextureInner {
    /// `CubeTexture.images`, in the order px, nx, py, ny, pz, nz.
    pub images: Vec<Image>,
    pub mapping: Mapping,
    pub color_space: ColorSpace,
    /// `CubeTexture` overwrites `Texture.flipY` with `false`.
    pub flip_y: bool,
    /// `Texture.generateMipmaps`, `true` by default.
    pub generate_mipmaps: bool,
    /// `Texture.anisotropy`.
    pub anisotropy: u16,
    pub gpu: Option<wgpu::Texture>,
}

/// Cloning is a handle copy.
#[derive(Clone, Debug)]
pub struct CubeTexture(Rc<RefCell<CubeTextureInner>>);

impl CubeTexture {
    /// `new CubeTexture()`: `CubeReflectionMapping`, `ClampToEdgeWrapping`,
    /// `LinearFilter` / `LinearMipmapLinearFilter`, `RGBAFormat`,
    /// `UnsignedByteType`, `NoColorSpace`, `flipY = false`.
    pub fn new(images: Vec<Image>) -> Self {
        Self(Rc::new(RefCell::new(CubeTextureInner {
            images,
            mapping: Mapping::CubeReflection,
            color_space: ColorSpace::NoColorSpace,
            flip_y: false,
            generate_mipmaps: true,
            anisotropy: 1,
            gpu: None,
        })))
    }

    pub fn set_color_space(&self, color_space: ColorSpace) {
        self.0.borrow_mut().color_space = color_space;
    }

    pub fn color_space(&self) -> ColorSpace {
        self.0.borrow().color_space
    }

    pub fn mapping(&self) -> Mapping {
        self.0.borrow().mapping
    }

    /// `WebGPUTextureUtils.getFormat()` for `RGBAFormat` + `UnsignedByteType`:
    /// the sRGB transfer function is applied by the GPU on sample, which is why
    /// `WGSLNodeBuilder.needsToWorkingColorSpace()` stays `false` and no
    /// colour-space node appears in the generated WGSL.
    pub fn gpu_format(&self) -> wgpu::TextureFormat {
        match self.color_space() {
            ColorSpace::SRGB => wgpu::TextureFormat::Rgba8UnormSrgb,
            ColorSpace::NoColorSpace => wgpu::TextureFormat::Rgba8Unorm,
        }
    }

    /// `Textures.getMipLevels()`: `floor( log2( max( width, height ) ) ) + 1`.
    pub fn mip_level_count(&self) -> u32 {
        let inner = self.0.borrow();
        if !inner.generate_mipmaps {
            return 1;
        }
        let size = inner.images[0].width.max(inner.images[0].height) as f64;
        size.log2().floor() as u32 + 1
    }

    pub fn size(&self) -> (u32, u32) {
        let inner = self.0.borrow();
        (inner.images[0].width, inner.images[0].height)
    }

    pub fn id(&self) -> usize {
        Rc::as_ptr(&self.0) as usize
    }

    pub(crate) fn inner(&self) -> &RefCell<CubeTextureInner> {
        &self.0
    }
}
