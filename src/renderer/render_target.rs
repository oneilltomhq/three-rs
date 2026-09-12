//! Port of `three.js/src/core/RenderTarget.js` (rung 1 subset).

use std::cell::RefCell;
use std::rc::Rc;

use crate::textures::DepthTexture;

#[derive(Debug)]
pub struct RenderTargetInner {
    pub width: u32,
    pub height: u32,
    /// `RenderTarget`'s default `samples` is 0 — the renderer's `antialias`
    /// option does **not** propagate to render targets.
    pub samples: u32,
    pub depth_texture: Option<DepthTexture>,
    pub color: Option<wgpu::Texture>,
}

/// `new RenderTarget( width, height )`. Cloning is a handle copy, matching JS
/// object identity.
#[derive(Clone, Debug)]
pub struct RenderTarget(Rc<RefCell<RenderTargetInner>>);

impl RenderTarget {
    pub fn new(width: u32, height: u32) -> Self {
        Self(Rc::new(RefCell::new(RenderTargetInner {
            width,
            height,
            samples: 0,
            depth_texture: None,
            color: None,
        })))
    }

    pub fn set_depth_texture(&self, depth_texture: DepthTexture) {
        self.0.borrow_mut().depth_texture = Some(depth_texture);
    }

    pub fn depth_texture(&self) -> Option<DepthTexture> {
        self.0.borrow().depth_texture.clone()
    }

    pub fn size(&self) -> (u32, u32) {
        let inner = self.0.borrow();
        (inner.width, inner.height)
    }

    pub fn samples(&self) -> u32 {
        self.0.borrow().samples
    }

    /// `RenderTarget.texture` has `RGBAFormat` + `UnsignedByteType` and
    /// `NoColorSpace`, i.e. a plain `rgba8unorm` GPU texture.
    pub const COLOR_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;

    pub(crate) fn inner(&self) -> &RefCell<RenderTargetInner> {
        &self.0
    }
}
