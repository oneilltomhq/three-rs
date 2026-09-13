//! Port of `three.js/src/textures/CubeDepthTexture.js` — the six-face depth
//! texture a `PointShadowNode` renders into and then samples with
//! `textureSampleCompare` through a comparison sampler.
//!
//! Like [`DepthTexture`](super::DepthTexture) the handle is an `Rc`: the node
//! graph names the texture while the renderer owns its GPU side, exactly as
//! `light.shadow.map` is shared between the shadow node and the renderer.

use std::cell::RefCell;
use std::rc::Rc;

use super::TextureFilter;
use super::TextureId;

#[derive(Debug)]
pub struct CubeDepthTextureInner {
    /// `shadow.mapSize.width` — a cube face is square.
    pub size: u32,
    /// `depthTexture.minFilter` / `.magFilter`. `PCFShadowMap` with
    /// `textureSampleCompare` available means `LinearFilter`.
    pub mag_filter: TextureFilter,
    pub min_filter: TextureFilter,
    /// The GPU texture: a `depth24plus` 2D texture with six array layers,
    /// created by the renderer the first time the shadow is rendered.
    pub gpu: Option<wgpu::Texture>,
}

#[derive(Clone)]
pub struct CubeDepthTexture(Rc<RefCell<CubeDepthTextureInner>>, TextureId);

/// The id is identity, not content: leaving it out keeps the `Debug` of a
/// binding description — what `examples/dump_wgsl.rs` prints beside the WGSL —
/// a function of the texture itself.
impl std::fmt::Debug for CubeDepthTexture {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("CubeDepthTexture").field(&self.0).finish()
    }
}

impl CubeDepthTexture {
    /// `new CubeDepthTexture( size )`.
    pub fn new(size: u32) -> Self {
        Self(
            Rc::new(RefCell::new(CubeDepthTextureInner {
                size,
                mag_filter: TextureFilter::Linear,
                min_filter: TextureFilter::Linear,
                gpu: None,
            })),
            TextureId::next(),
        )
    }

    pub fn size(&self) -> u32 {
        self.0.borrow().size
    }

    /// `WebGPUTextureUtils.getFormat()` for a `DepthTexture` of
    /// `UnsignedIntType`, which is what the dumped shadow pipeline uses.
    pub fn gpu_format(&self) -> wgpu::TextureFormat {
        wgpu::TextureFormat::Depth24Plus
    }

    pub fn id(&self) -> usize {
        self.1.get()
    }

    pub(crate) fn inner(&self) -> &RefCell<CubeDepthTextureInner> {
        &self.0
    }
}
