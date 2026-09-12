//! Port of `three.js/src/textures/DepthTexture.js` (rung 1 subset).
//!
//! The handle is an `Rc` so that the JS pattern in the example — one
//! `DepthTexture` referenced both by the render target and by the quad
//! material's colour node — maps across directly.

use std::cell::RefCell;
use std::rc::Rc;

/// `three.js/src/constants.js` texture types, as far as rung 1 needs them.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TextureType {
    UnsignedInt,
    Float,
}

/// `three.js/src/constants.js` texture filters.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
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

#[derive(Clone, Debug)]
pub struct DepthTexture(Rc<RefCell<DepthTextureInner>>);

impl Default for DepthTexture {
    fn default() -> Self {
        Self::new()
    }
}

impl DepthTexture {
    /// `new DepthTexture()`: `UnsignedIntType`, `NearestFilter`, no size yet.
    pub fn new() -> Self {
        Self(Rc::new(RefCell::new(DepthTextureInner {
            texture_type: TextureType::UnsignedInt,
            mag_filter: TextureFilter::Nearest,
            min_filter: TextureFilter::Nearest,
            width: 0,
            height: 0,
            gpu: None,
        })))
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
        }
    }

    pub fn id(&self) -> usize {
        Rc::as_ptr(&self.0) as usize
    }

    pub(crate) fn inner(&self) -> &RefCell<DepthTextureInner> {
        &self.0
    }
}
