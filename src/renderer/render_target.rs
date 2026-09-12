//! Port of `three.js/src/core/RenderTarget.js` (the subset rungs 1–2 need).

use std::cell::RefCell;
use std::rc::Rc;

use crate::textures::{DepthTexture, Texture, TextureFilter, TextureType};

/// `new RenderTarget( width, height, options )` — the options the port reads.
#[derive(Clone, Copy, Debug)]
pub struct RenderTargetOptions {
    pub texture_type: TextureType,
    /// `RenderTarget`'s default `samples` is 0; the renderer's `antialias`
    /// option does **not** propagate to user render targets, only to the
    /// internal framebuffer target.
    pub samples: u32,
    pub depth_buffer: bool,
    pub min_filter: TextureFilter,
    pub mag_filter: TextureFilter,
}

impl Default for RenderTargetOptions {
    fn default() -> Self {
        Self {
            texture_type: TextureType::UnsignedByte,
            samples: 0,
            depth_buffer: true,
            min_filter: TextureFilter::Linear,
            mag_filter: TextureFilter::Linear,
        }
    }
}

#[derive(Debug)]
pub struct RenderTargetInner {
    pub width: u32,
    pub height: u32,
    pub samples: u32,
    pub texture_type: TextureType,
    pub depth_buffer: bool,
    pub min_filter: TextureFilter,
    pub mag_filter: TextureFilter,
    /// A `DepthTexture` the application attached, which it can then sample.
    pub depth_texture: Option<DepthTexture>,
    /// `renderTarget.texture` — a real `Texture` so `texture( rt.texture )`
    /// works; the renderer owns its GPU object (`own_gpu` is false).
    pub texture: Texture,
    /// The MSAA colour texture `samples > 1` asks for; the single-sample
    /// `color` texture is then its resolve target.
    pub msaa: Option<wgpu::Texture>,
    /// The depth buffer auto-allocated when `depth_buffer` is set and no
    /// `DepthTexture` was attached.
    pub depth: Option<wgpu::Texture>,
}

/// Cloning is a handle copy, matching JS object identity.
#[derive(Clone, Debug)]
pub struct RenderTarget(Rc<RefCell<RenderTargetInner>>);

impl RenderTarget {
    pub fn new(width: u32, height: u32) -> Self {
        Self::new_with_options(width, height, RenderTargetOptions::default())
    }

    pub fn new_with_options(width: u32, height: u32, options: RenderTargetOptions) -> Self {
        Self(Rc::new(RefCell::new(RenderTargetInner {
            width,
            height,
            samples: options.samples,
            texture_type: options.texture_type,
            depth_buffer: options.depth_buffer,
            min_filter: options.min_filter,
            mag_filter: options.mag_filter,
            depth_texture: None,
            texture: Texture::render_target(width, height, options.texture_type.color_gpu_format()),
            msaa: None,
            depth: None,
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

    /// `RenderTarget.setSize()` — drops the GPU textures so they are recreated.
    pub fn set_size(&self, width: u32, height: u32) {
        let mut inner = self.0.borrow_mut();
        if inner.width != width || inner.height != height {
            inner.width = width;
            inner.height = height;
            inner.texture.set_size(width, height);
            inner.texture.clear_gpu();
            inner.msaa = None;
            inner.depth = None;
        }
    }

    /// `renderTarget.texture`.
    pub fn texture(&self) -> Texture {
        self.0.borrow().texture.clone()
    }

    pub fn samples(&self) -> u32 {
        self.0.borrow().samples
    }

    pub fn color_format(&self) -> wgpu::TextureFormat {
        self.0.borrow().texture_type.color_gpu_format()
    }

    pub(crate) fn inner(&self) -> &RefCell<RenderTargetInner> {
        &self.0
    }
}
