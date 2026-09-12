//! Port of `three.js/src/textures/Texture.js` (the 2D path, rung-4 subset).
//!
//! One handle covers both roles the ladder needs: an image loaded from disk and
//! the colour attachment of a `RenderTarget` (`RenderTarget.texture`). Three
//! makes no distinction either — `renderTarget.texture` is a plain `Texture`
//! whose image is filled in by the backend.

use std::cell::{Ref, RefCell};
use std::rc::Rc;

use super::{ColorSpace, TextureFilter};

/// `three.js/src/constants.js` wrapping modes (only the default so far).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Wrapping {
    ClampToEdge,
}

/// `Texture.minFilter` — the mip-aware half of the filter pair.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MinFilter {
    Nearest,
    Linear,
    /// `LinearMipmapLinearFilter`, the `Texture` default.
    LinearMipmapLinear,
}

impl MinFilter {
    pub fn min(self) -> TextureFilter {
        match self {
            MinFilter::Nearest => TextureFilter::Nearest,
            _ => TextureFilter::Linear,
        }
    }

    /// `WebGPUTextureUtils` maps `*MipmapLinear` to `mipmapFilter: 'linear'`
    /// and everything else to `'nearest'`.
    pub fn mipmap(self) -> TextureFilter {
        match self {
            MinFilter::LinearMipmapLinear => TextureFilter::Linear,
            _ => TextureFilter::Nearest,
        }
    }
}

#[derive(Debug)]
pub struct TextureInner {
    pub width: u32,
    pub height: u32,
    /// RGBA8 rows, top-down as decoded. `None` for a render-target texture.
    pub data: Option<Vec<u8>>,
    pub color_space: ColorSpace,
    pub flip_y: bool,
    pub generate_mipmaps: bool,
    pub wrap_s: Wrapping,
    pub wrap_t: Wrapping,
    pub mag_filter: TextureFilter,
    pub min_filter: MinFilter,
    pub anisotropy: u16,
    /// `false` for `renderTarget.texture` — the renderer owns the GPU texture.
    pub own_gpu: bool,
    pub gpu: Option<wgpu::Texture>,
    /// The format to use if the renderer has to create it (render targets).
    pub format: wgpu::TextureFormat,
}

/// Cloning is a handle copy, as in JS.
#[derive(Clone, Debug)]
pub struct Texture(Rc<RefCell<TextureInner>>);

impl Texture {
    /// `new Texture()` defaults: `ClampToEdgeWrapping`, `LinearFilter`,
    /// `LinearMipmapLinearFilter`, `RGBAFormat`, `UnsignedByteType`,
    /// `NoColorSpace`, `flipY = true`, `generateMipmaps = true`, anisotropy 1.
    pub fn new(width: u32, height: u32, data: Option<Vec<u8>>) -> Self {
        Self(Rc::new(RefCell::new(TextureInner {
            width,
            height,
            data,
            color_space: ColorSpace::NoColorSpace,
            flip_y: true,
            generate_mipmaps: true,
            wrap_s: Wrapping::ClampToEdge,
            wrap_t: Wrapping::ClampToEdge,
            mag_filter: TextureFilter::Linear,
            min_filter: MinFilter::LinearMipmapLinear,
            anisotropy: 1,
            own_gpu: true,
            gpu: None,
            format: wgpu::TextureFormat::Rgba8Unorm,
        })))
    }

    /// The texture behind a render target's colour attachment: no image data,
    /// no mipmaps, `flipY` irrelevant, and `minFilter = LinearFilter` (which is
    /// what `RenderTarget`'s options set).
    pub fn render_target(width: u32, height: u32, format: wgpu::TextureFormat) -> Self {
        let texture = Self::new(width, height, None);
        {
            let mut inner = texture.0.borrow_mut();
            inner.generate_mipmaps = false;
            inner.min_filter = MinFilter::Linear;
            inner.own_gpu = false;
            inner.format = format;
        }
        texture
    }

    pub fn id(&self) -> usize {
        Rc::as_ptr(&self.0) as *const u8 as usize
    }

    pub fn borrow(&self) -> Ref<'_, TextureInner> {
        self.0.borrow()
    }

    pub fn size(&self) -> (u32, u32) {
        let inner = self.0.borrow();
        (inner.width, inner.height)
    }

    pub fn set_size(&self, width: u32, height: u32) {
        let mut inner = self.0.borrow_mut();
        inner.width = width;
        inner.height = height;
    }

    pub fn set_gpu(&self, gpu: wgpu::Texture) {
        self.0.borrow_mut().gpu = Some(gpu);
    }

    pub fn has_gpu(&self) -> bool {
        self.0.borrow().gpu.is_some()
    }

    pub fn with_gpu<R>(&self, f: impl FnOnce(&wgpu::Texture) -> R) -> R {
        let inner = self.0.borrow();
        f(inner.gpu.as_ref().expect("three-rs: texture not uploaded"))
    }

    pub fn format(&self) -> wgpu::TextureFormat {
        self.0.borrow().format
    }

    pub fn set_format(&self, format: wgpu::TextureFormat) {
        self.0.borrow_mut().format = format;
    }

    /// `Texture.mipmapCount` — `floor( log2( max( w, h ) ) ) + 1`.
    pub fn mip_level_count(&self) -> u32 {
        let inner = self.0.borrow();
        if !inner.generate_mipmaps {
            return 1;
        }
        let max = inner.width.max(inner.height) as f64;
        (max.log2().floor() as u32) + 1
    }
}
