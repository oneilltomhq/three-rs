//! Port of `three.js/src/textures/CubeTexture.js` + the `Texture.js` fields the
//! WebGPU backend reads (rung 3 subset).
//!
//! Handle semantics match JS object identity: the example hands the same
//! `CubeTexture` to `scene.background` and to `material.envMap`, and the
//! renderer must upload it once.

use std::cell::Ref;

use super::texture::MinFilter;
use super::{TextureFilter, TextureId, TextureType};
use std::cell::RefCell;
use std::rc::Rc;

/// `three.js/src/constants.js` colour spaces, as far as the port needs them.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
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

/// One image of `CubeTexture.images`, top-down — what `ImageLoader` hands the
/// backend after the browser has decoded the PNG, or the `DataTexture`
/// `HDRCubeTextureLoader` builds per face.
///
/// `data` is in the texture's own format, not always RGBA8: an
/// `HDRCubeTextureLoader` face is four binary16 channels per texel, eight
/// bytes. The owning [`CubeTexture`]'s `texture_type` says which, and the
/// upload takes its stride from the format rather than assuming four bytes.
#[derive(Clone)]
pub struct Image {
    pub width: u32,
    pub height: u32,
    pub data: Vec<u8>,
}

impl Image {
    /// An RGBA8 face, the `UnsignedByteType` default.
    pub fn rgba8(width: u32, height: u32, data: Vec<u8>) -> Self {
        assert_eq!(
            data.len() as u32,
            width * height * 4,
            "three-rs: an RGBA8 cube face holds four bytes per texel"
        );
        Self {
            width,
            height,
            data,
        }
    }

    /// An `rgba16float` face from one binary16 bit pattern per channel — what
    /// [`HdrData::HalfFloat`](crate::loaders::HdrData::HalfFloat) holds.
    pub fn rgba16float(width: u32, height: u32, data: &[u16]) -> Self {
        assert_eq!(
            data.len() as u32,
            width * height * 4,
            "three-rs: an rgba16float cube face holds four halves per texel"
        );
        Self {
            width,
            height,
            data: bytemuck::cast_slice(data).to_vec(),
        }
    }
}

#[derive(Debug)]
pub struct CubeTextureInner {
    /// `CubeTexture.images`, in the order px, nx, py, ny, pz, nz.
    pub images: Vec<Image>,
    pub mapping: Mapping,
    pub color_space: ColorSpace,
    /// `Texture.type` — `UnsignedByteType` for a PNG cube, `HalfFloatType` for
    /// the one `HDRCubeTextureLoader` builds.
    pub texture_type: TextureType,
    /// `CubeTexture` overwrites `Texture.flipY` with `false`.
    pub flip_y: bool,
    /// `Texture.generateMipmaps`, `true` by default.
    pub generate_mipmaps: bool,
    /// `Texture.anisotropy`.
    pub anisotropy: u16,
    pub mag_filter: TextureFilter,
    pub min_filter: MinFilter,
    pub gpu: Option<wgpu::Texture>,
}

/// Cloning is a handle copy.
#[derive(Clone)]
pub struct CubeTexture(Rc<RefCell<CubeTextureInner>>, TextureId);

/// The id is identity, not content: leaving it out keeps the `Debug` of a
/// binding description — what `examples/dump_wgsl.rs` prints beside the WGSL —
/// a function of the texture itself.
impl std::fmt::Debug for CubeTexture {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("CubeTexture").field(&self.0).finish()
    }
}

impl CubeTexture {
    /// `new CubeTexture()`: `CubeReflectionMapping`, `ClampToEdgeWrapping`,
    /// `LinearFilter` / `LinearMipmapLinearFilter`, `RGBAFormat`,
    /// `UnsignedByteType`, `NoColorSpace`, `flipY = false`.
    pub fn new(images: Vec<Image>) -> Self {
        Self(
            Rc::new(RefCell::new(CubeTextureInner {
                images,
                mapping: Mapping::CubeReflection,
                color_space: ColorSpace::NoColorSpace,
                texture_type: TextureType::UnsignedByte,
                flip_y: false,
                generate_mipmaps: true,
                anisotropy: 1,
                mag_filter: TextureFilter::Linear,
                min_filter: MinFilter::LinearMipmapLinear,
                gpu: None,
            })),
            TextureId::next(),
        )
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

    /// `texture.type = HalfFloatType`, with the filters and mip policy
    /// `HDRCubeTextureLoader` sets alongside it left to the caller.
    pub fn set_texture_type(&self, texture_type: TextureType) -> Result<(), crate::error::Error> {
        if !texture_type.is_color() {
            return Err(crate::error::Error::UnsupportedTextureType {
                what: "colour",
                texture_type,
            });
        }
        self.0.borrow_mut().texture_type = texture_type;
        Ok(())
    }

    pub fn texture_type(&self) -> TextureType {
        self.0.borrow().texture_type
    }

    pub fn set_generate_mipmaps(&self, generate_mipmaps: bool) {
        self.0.borrow_mut().generate_mipmaps = generate_mipmaps;
    }

    /// `texture.minFilter` / `texture.magFilter`.
    pub fn set_filters(&self, min_filter: MinFilter, mag_filter: TextureFilter) {
        let mut inner = self.0.borrow_mut();
        inner.min_filter = min_filter;
        inner.mag_filter = mag_filter;
    }

    /// `WebGPUTextureUtils.getFormat()` for `RGBAFormat` plus the texture's
    /// type: the sRGB transfer function is applied by the GPU on sample, which
    /// is why `WGSLNodeBuilder.needsToWorkingColorSpace()` stays `false` and no
    /// colour-space node appears in the generated WGSL.
    ///
    /// `HalfFloatType` has no sRGB variant and never wants one — the HDR faces
    /// are `LinearSRGBColorSpace`, i.e. the working space itself.
    pub fn gpu_format(&self) -> wgpu::TextureFormat {
        match self.texture_type() {
            TextureType::HalfFloat => wgpu::TextureFormat::Rgba16Float,
            _ => match self.color_space() {
                ColorSpace::SRGB => wgpu::TextureFormat::Rgba8UnormSrgb,
                ColorSpace::NoColorSpace => wgpu::TextureFormat::Rgba8Unorm,
            },
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
        self.1.get()
    }

    /// The faces and the flags, as `Texture::borrow()` gives them — enough to
    /// assert the upload's stride, face order and row order without a GPU.
    pub fn borrow(&self) -> Ref<'_, CubeTextureInner> {
        self.0.borrow()
    }

    pub(crate) fn inner(&self) -> &RefCell<CubeTextureInner> {
        &self.0
    }
}

/// As `TextureInner`: six decoded faces are six megabyte buffers.
impl std::fmt::Debug for Image {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Image")
            .field("width", &self.width)
            .field("height", &self.height)
            .field("data", &super::texture::DataLen(self.data.len()))
            .finish()
    }
}
