//! Port of `three.js/src/textures/Texture.js` (the 2D path, rung-4 subset).
//!
//! One handle covers both roles the ladder needs: an image loaded from disk and
//! the colour attachment of a `RenderTarget` (`RenderTarget.texture`). Three
//! makes no distinction either — `renderTarget.texture` is a plain `Texture`
//! whose image is filled in by the backend.

use std::cell::{Ref, RefCell};
use std::rc::Rc;

use super::{ColorSpace, TextureFilter};
use crate::math::{Matrix3, Vector2};

/// `three.js/src/constants.js` wrapping modes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Wrapping {
    /// `ClampToEdgeWrapping` — the `Texture` default.
    ClampToEdge,
    /// `RepeatWrapping`.
    Repeat,
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
    /// `Texture.offset` / `.repeat` / `.center` / `.rotation` —
    /// `updateMatrix()`'s inputs.
    pub offset: Vector2,
    pub repeat: Vector2,
    pub center: Vector2,
    pub rotation: f64,
    /// `Texture.matrix`, kept in step with the four above.
    pub matrix: Matrix3,
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
            offset: Vector2::new(0.0, 0.0),
            repeat: Vector2::new(1.0, 1.0),
            center: Vector2::new(0.0, 0.0),
            rotation: 0.0,
            matrix: Matrix3::identity(),
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

    /// `texture.colorSpace = SRGBColorSpace`. As with `CubeTexture`, the
    /// transfer function is applied by the GPU on sample — the format becomes
    /// `rgba8unorm-srgb` and `WGSLNodeBuilder.needsToWorkingColorSpace()` stays
    /// false, so no colour-space node appears in the generated WGSL.
    pub fn set_color_space(&self, color_space: ColorSpace) {
        let mut inner = self.0.borrow_mut();
        inner.color_space = color_space;
        inner.format = match color_space {
            ColorSpace::SRGB => wgpu::TextureFormat::Rgba8UnormSrgb,
            ColorSpace::NoColorSpace => wgpu::TextureFormat::Rgba8Unorm,
        };
    }

    pub fn color_space(&self) -> ColorSpace {
        self.0.borrow().color_space
    }

    /// `texture.flipY` — an upload-time concern only. `Texture.updateMatrix()`
    /// goes through `Matrix3.setUvTransform( offset, repeat, rotation, center )`,
    /// which has no `flipY` term, so the texture matrix is unaffected.
    pub fn set_flip_y(&self, flip_y: bool) {
        self.0.borrow_mut().flip_y = flip_y;
    }

    pub fn set_generate_mipmaps(&self, generate_mipmaps: bool) {
        self.0.borrow_mut().generate_mipmaps = generate_mipmaps;
    }

    pub fn set_min_filter(&self, min_filter: MinFilter) {
        self.0.borrow_mut().min_filter = min_filter;
    }

    pub fn set_mag_filter(&self, mag_filter: TextureFilter) {
        self.0.borrow_mut().mag_filter = mag_filter;
    }

    pub fn borrow(&self) -> Ref<'_, TextureInner> {
        self.0.borrow()
    }

    /// `texture.wrapS = texture.wrapT = wrapping`.
    pub fn set_wrapping(&self, wrap_s: Wrapping, wrap_t: Wrapping) {
        let mut inner = self.0.borrow_mut();
        inner.wrap_s = wrap_s;
        inner.wrap_t = wrap_t;
    }

    /// `texture.anisotropy = n`.
    pub fn set_anisotropy(&self, anisotropy: u16) {
        self.0.borrow_mut().anisotropy = anisotropy;
    }

    /// `texture.repeat.set( x, y )` — and `updateMatrix()`, which three.js runs
    /// for us every frame because `matrixAutoUpdate` is on by default.
    pub fn set_repeat(&self, x: f64, y: f64) {
        self.0.borrow_mut().repeat = Vector2::new(x, y);
        self.update_matrix();
    }

    /// `texture.offset.set( x, y )`.
    pub fn set_offset(&self, x: f64, y: f64) {
        self.0.borrow_mut().offset = Vector2::new(x, y);
        self.update_matrix();
    }

    /// `texture.center.set( x, y )`.
    pub fn set_center(&self, x: f64, y: f64) {
        self.0.borrow_mut().center = Vector2::new(x, y);
        self.update_matrix();
    }

    /// `texture.rotation = theta`.
    pub fn set_rotation(&self, rotation: f64) {
        self.0.borrow_mut().rotation = rotation;
        self.update_matrix();
    }

    /// `Texture.updateMatrix()`.
    pub fn update_matrix(&self) {
        let mut inner = self.0.borrow_mut();
        let (offset, repeat, center, rotation) =
            (inner.offset, inner.repeat, inner.center, inner.rotation);
        inner.matrix.set_uv_transform(
            offset.x, offset.y, repeat.x, repeat.y, rotation, center.x, center.y,
        );
    }

    /// `Texture.matrix` — what `TextureNode.setupUV()` multiplies the UV by.
    pub fn matrix(&self) -> Matrix3 {
        self.0.borrow().matrix
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

    pub fn clear_gpu(&self) {
        self.0.borrow_mut().gpu = None;
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
