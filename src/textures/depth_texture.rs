//! Port of `three.js/src/textures/DepthTexture.js` (rung 1 subset).
//!
//! The handle is an `Rc` so that the JS pattern in the example — one
//! `DepthTexture` referenced both by the render target and by the quad
//! material's colour node — maps across directly.

use super::TextureId;
use std::cell::RefCell;
use std::rc::Rc;

/// `three.js/src/constants.js` texture types, as far as the port needs them.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TextureType {
    UnsignedByte,
    HalfFloat,
    UnsignedInt,
    Float,
}

impl TextureType {
    /// `WebGPUTextureUtils.getFormat()` for a colour texture with
    /// `RGBAFormat` and `NoColorSpace`.
    pub fn color_gpu_format(self) -> wgpu::TextureFormat {
        match self {
            TextureType::UnsignedByte => wgpu::TextureFormat::Rgba8Unorm,
            TextureType::HalfFloat => wgpu::TextureFormat::Rgba16Float,
            other => panic!("three-rs: {other:?} is not a colour texture type here"),
        }
    }
}

/// `three.js/src/constants.js` texture filters.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TextureFilter {
    Nearest,
    Linear,
}

#[derive(Debug)]
pub struct DepthTextureInner {
    pub texture_type: TextureType,
    pub mag_filter: TextureFilter,
    pub min_filter: TextureFilter,
    pub width: u32,
    pub height: u32,
    /// The GPU texture, created by the renderer when the owning render target
    /// is first used.
    pub gpu: Option<wgpu::Texture>,
}

#[derive(Clone)]
pub struct DepthTexture(Rc<RefCell<DepthTextureInner>>, TextureId);

/// The id is identity, not content: leaving it out keeps the `Debug` of a
/// binding description — what `examples/dump_wgsl.rs` prints beside the WGSL —
/// a function of the texture itself.
impl std::fmt::Debug for DepthTexture {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("DepthTexture").field(&self.0).finish()
    }
}

impl Default for DepthTexture {
    fn default() -> Self {
        Self::new()
    }
}

impl DepthTexture {
    /// `new DepthTexture()`: `UnsignedIntType`, `NearestFilter`, no size yet.
    pub fn new() -> Self {
        Self(
            Rc::new(RefCell::new(DepthTextureInner {
                texture_type: TextureType::UnsignedInt,
                mag_filter: TextureFilter::Nearest,
                min_filter: TextureFilter::Nearest,
                width: 0,
                height: 0,
                gpu: None,
            })),
            TextureId::next(),
        )
    }

    /// `depthTexture.minFilter = depthTexture.magFilter = LinearFilter` —
    /// what `ShadowNode.setupRenderTarget()` sets for PCF, so that the
    /// comparison sampler gives four bilinear-weighted comparisons per tap.
    pub fn set_filters(&self, min_filter: TextureFilter, mag_filter: TextureFilter) {
        let mut inner = self.0.borrow_mut();
        inner.min_filter = min_filter;
        inner.mag_filter = mag_filter;
    }

    pub fn set_type(&self, texture_type: TextureType) {
        self.0.borrow_mut().texture_type = texture_type;
    }

    pub fn texture_type(&self) -> TextureType {
        self.0.borrow().texture_type
    }

    /// `WebGPUTextureUtils.getFormat()` for `DepthFormat`.
    pub fn gpu_format(&self) -> wgpu::TextureFormat {
        match self.texture_type() {
            TextureType::UnsignedInt => wgpu::TextureFormat::Depth24Plus,
            TextureType::Float => wgpu::TextureFormat::Depth32Float,
            other => panic!("three-rs: {other:?} is not a depth texture type"),
        }
    }

    pub fn id(&self) -> usize {
        self.1.get()
    }

    pub(crate) fn inner(&self) -> &RefCell<DepthTextureInner> {
        &self.0
    }
}
