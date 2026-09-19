//! Port of `three.js/src/textures/DepthTexture.js` (rung 1 subset).
//!
//! The handle is an `Rc` so that the JS pattern in the example — one
//! `DepthTexture` referenced both by the render target and by the quad
//! material's colour node — maps across directly.

use super::TextureId;
use crate::error::Error;
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
    /// Whether this type can back a render target's colour attachment.
    pub fn is_color(self) -> bool {
        matches!(self, TextureType::UnsignedByte | TextureType::HalfFloat)
    }

    /// Whether this type can back a depth attachment.
    pub fn is_depth(self) -> bool {
        matches!(self, TextureType::UnsignedInt | TextureType::Float)
    }

    /// `WebGPUTextureUtils.getFormat()` for a colour texture with
    /// `RGBAFormat` and `NoColorSpace`.
    ///
    /// `pub(crate)`: the type is checked where the caller supplies it
    /// (`RenderTarget::new_with_options`), which is what turns the mismatch
    /// into an [`Error`] rather than the panic below.
    pub(crate) fn color_gpu_format(self) -> wgpu::TextureFormat {
        match self {
            TextureType::UnsignedByte => wgpu::TextureFormat::Rgba8Unorm,
            TextureType::HalfFloat => wgpu::TextureFormat::Rgba16Float,
            other => panic!(
                "three-rs: the render target's texture type is a colour type \
                 (RenderTarget::new_with_options checks it), got {other:?}"
            ),
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
    /// `texture.isMultisampleRenderTargetTexture` — set when the owning render
    /// target's `samples` is raised above 1, which is what makes the node
    /// builder declare the binding `texture_depth_multisampled_2d` and the
    /// bind-group layout entry `multisampled: true`.
    pub multisample: bool,
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
                multisample: false,
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

    /// `texture.isMultisampleRenderTargetTexture`, which
    /// `RenderTarget::set_samples` keeps in step with the target's sample
    /// count. A multisampled depth attachment is never resolved — the fog
    /// composite reads sample 0 of it directly.
    pub fn set_multisample(&self, multisample: bool) {
        let mut inner = self.0.borrow_mut();
        if inner.multisample != multisample {
            inner.multisample = multisample;
            inner.gpu = None;
        }
    }

    /// Whether this depth texture is a multisampled render-target attachment.
    pub fn is_multisample(&self) -> bool {
        self.0.borrow().multisample
    }

    /// `depthTexture.type = type`. Errors rather than deferring the failure to
    /// the pass that would have used the texture: only `UnsignedIntType` and
    /// `FloatType` have a depth format.
    pub fn set_type(&self, texture_type: TextureType) -> Result<(), Error> {
        if !texture_type.is_depth() {
            return Err(Error::UnsupportedTextureType {
                what: "depth",
                texture_type,
            });
        }

        self.0.borrow_mut().texture_type = texture_type;

        Ok(())
    }

    pub fn texture_type(&self) -> TextureType {
        self.0.borrow().texture_type
    }

    /// `WebGPUTextureUtils.getFormat()` for `DepthFormat`.
    ///
    /// `pub(crate)`: [`Self::set_type`] is the only way in and it rejects a
    /// non-depth type, so the panic below is an invariant.
    pub(crate) fn gpu_format(&self) -> wgpu::TextureFormat {
        match self.texture_type() {
            TextureType::UnsignedInt => wgpu::TextureFormat::Depth24Plus,
            TextureType::Float => wgpu::TextureFormat::Depth32Float,
            other => panic!(
                "three-rs: the depth texture's type is a depth type \
                 (DepthTexture::set_type checks it), got {other:?}"
            ),
        }
    }

    pub fn id(&self) -> usize {
        self.1.get()
    }

    pub(crate) fn inner(&self) -> &RefCell<DepthTextureInner> {
        &self.0
    }
}
