//! `Renderer.copyFramebufferToTexture()` for the viewport nodes — the
//! framebuffer copy behind `viewportSharedTexture()`, `viewportTexture()` and
//! `viewportDepthTexture()` (`src/nodes/display/viewport_texture.rs`).
//!
//! Three copies from inside `renderObject()`: the node's `updateBefore()` runs
//! just before the draw of the first object that reads it, and
//! `WebGPUBackend.copyFramebufferToTexture()` ends the open pass, copies, and
//! begins a new one that loads what is there. The port builds every draw of a
//! pass before it records any of them, so a copy is a *request* made while the
//! draws are being built — "copy before draw `n`" — and [`Renderer::draw`]
//! records the pass in segments with the copies between them. The copy lands
//! where three's does: after everything ahead of the reading object in the
//! opaque-then-transparent list, before the object itself.
//!
//! A pass that no material reads from is recorded exactly as before, in one
//! segment: nothing here runs unless a viewport node's `updateBefore()` does.
//!
//! The destination is (re)allocated by the request, not by the copy, because
//! the reading draw's bind group is made right after the request returns and
//! has to name the texture the copy will fill. The request also keeps the
//! `wgpu::Texture` it allocated, so a later resize of a shared texture — a
//! nested render into a target of another size — cannot redirect an earlier
//! copy into a texture the earlier draws do not read.

use super::{PassTarget, Renderer};
use std::collections::HashMap;

use crate::textures::{DepthTexture, Texture, TextureOwner, TextureType};

/// What [`Renderer::draw`] knows about the pass it is building, for the copy
/// requests made while it builds.
pub(super) struct ScreenReads {
    width: u32,
    height: u32,
    color_format: wgpu::TextureFormat,
    depth_format: Option<wgpu::TextureFormat>,
    /// A colour copy is possible: the pass has a single-sampled colour
    /// texture to copy from (every pass but a cube shadow face).
    color: bool,
    /// A depth copy is possible: the pass has a depth texture and it is
    /// single-sampled. WebGPU copies between textures of one sample count
    /// only, and the destination is bound as `texture_depth_2d`, so under MSAA
    /// the copy is skipped and the reading draw sees the depth texture's
    /// previous contents (zeros, the first time).
    depth: bool,
    /// The index the draw being built will have in the pass.
    pub(super) draw_index: usize,
    /// The requests, in the order they were made.
    pub(super) copies: Vec<(usize, FramebufferCopy)>,
}

/// The `wgpu::Texture` this renderer allocated for each copy destination,
/// by texture id: the port's half of three's per-renderer `backend.get(
/// texture )`.
///
/// A texture handle carries one GPU texture, but the shared textures behind
/// `viewportSharedTexture()` and `viewportDepthTexture()` are thread-locals
/// that outlive any one renderer. The texture a handle carries may have been
/// made on another renderer's device, so it is reused only when it is the
/// one this renderer made. Otherwise the request makes a new one. Two live
/// renderers that read the same shared texture each re-point the handle at
/// their own before they bind it.
#[derive(Default)]
pub(super) struct Destinations(HashMap<(bool, usize), (TextureOwner, wgpu::Texture)>);

impl Destinations {
    /// Whether `gpu` is the texture this renderer made for `key`.
    fn owns(&self, key: (bool, usize), gpu: &wgpu::Texture) -> bool {
        self.0.get(&key).is_some_and(|(_, made)| made == gpu)
    }

    /// Record `gpu` as this renderer's texture for `key`, and forget the
    /// textures of handles that have since been dropped.
    fn insert(&mut self, key: (bool, usize), owner: TextureOwner, gpu: wgpu::Texture) {
        self.0.retain(|_, (owner, _)| owner.strong_count() > 0);
        self.0.insert(key, (owner, gpu));
    }
}

/// One copy between two segments of a pass.
pub(super) enum FramebufferCopy {
    /// The transmission pass's `viewportOpaqueMipTexture()`.
    OpaqueFrame,
    /// The colour attachment into this texture.
    Color(wgpu::Texture),
    /// The depth attachment into this texture.
    Depth(wgpu::Texture),
}

impl ScreenReads {
    pub(super) fn new(target: &PassTarget) -> Self {
        Self {
            width: target.width,
            height: target.height,
            color_format: target.color_format,
            depth_format: target.depth_format,
            color: target.color_texture.is_some(),
            depth: target.depth_texture.is_some() && target.sample_count == 1,
            draw_index: 0,
            copies: Vec::new(),
        }
    }
}

impl Renderer {
    /// `renderer.copyFramebufferToTexture( framebufferTexture )`: size
    /// `texture` to the pass being built and copy the colour attachment into
    /// it before the draw being built.
    ///
    /// Outside a pass — nothing to copy from — the texture still gets a GPU
    /// texture, 1×1, so that the draw that binds it has something to bind.
    pub(crate) fn copy_framebuffer_to_texture(&mut self, texture: &Texture) {
        let (width, height, format, copy) = match &self.screen_reads {
            Some(reads) => (reads.width, reads.height, reads.color_format, reads.color),
            None => (1, 1, wgpu::TextureFormat::Rgba8Unorm, false),
        };

        let current = texture
            .has_gpu()
            .then(|| texture.with_gpu(|gpu| gpu.clone()))
            .filter(|gpu| gpu.width() == width && gpu.height() == height && gpu.format() == format)
            .filter(|gpu| self.screen_read_textures.owns((false, texture.id()), gpu));
        let gpu = match current {
            Some(gpu) => gpu,
            None => {
                let gpu = self.device.create_texture(&wgpu::TextureDescriptor {
                    label: Some("three-rs FramebufferTexture"),
                    size: wgpu::Extent3d {
                        width,
                        height,
                        depth_or_array_layers: 1,
                    },
                    // `ViewportTextureNode.generateMipmaps` is false for every
                    // node this port has, so one level, whatever `minFilter`.
                    mip_level_count: 1,
                    sample_count: 1,
                    dimension: wgpu::TextureDimension::D2,
                    format,
                    usage: wgpu::TextureUsages::COPY_DST | wgpu::TextureUsages::TEXTURE_BINDING,
                    view_formats: &[],
                });
                texture.set_size(width, height);
                texture.set_format(format);
                texture.set_gpu(gpu.clone());
                self.screen_read_textures.insert(
                    (false, texture.id()),
                    texture.owner(),
                    gpu.clone(),
                );
                gpu
            }
        };

        if let Some(reads) = self.screen_reads.as_mut().filter(|_| copy) {
            reads
                .copies
                .push((reads.draw_index, FramebufferCopy::Color(gpu)));
        }
    }

    /// `renderer.copyFramebufferToTexture( depthTexture )` — the depth
    /// attachment, for `viewportDepthTexture()`. As
    /// [`copy_framebuffer_to_texture`](Self::copy_framebuffer_to_texture).
    pub(crate) fn copy_framebuffer_to_depth_texture(&mut self, depth: &DepthTexture) {
        let (width, height, format, copy) = match &self.screen_reads {
            Some(reads) => (
                reads.width,
                reads.height,
                reads
                    .depth_format
                    .unwrap_or(wgpu::TextureFormat::Depth24Plus),
                reads.depth,
            ),
            None => (1, 1, wgpu::TextureFormat::Depth24Plus, false),
        };

        let current = depth
            .inner()
            .borrow()
            .gpu
            .clone()
            .filter(|gpu| gpu.width() == width && gpu.height() == height && gpu.format() == format)
            .filter(|gpu| self.screen_read_textures.owns((true, depth.id()), gpu));
        let gpu = match current {
            Some(gpu) => gpu,
            None => {
                let gpu = self.device.create_texture(&wgpu::TextureDescriptor {
                    label: Some("three-rs viewport DepthTexture"),
                    size: wgpu::Extent3d {
                        width,
                        height,
                        depth_or_array_layers: 1,
                    },
                    mip_level_count: 1,
                    sample_count: 1,
                    dimension: wgpu::TextureDimension::D2,
                    format,
                    usage: wgpu::TextureUsages::COPY_DST | wgpu::TextureUsages::TEXTURE_BINDING,
                    view_formats: &[],
                });
                // The type follows the source, as three's `updateTexture(
                // framebufferTexture, { renderTarget } )` does.
                let texture_type = match format {
                    wgpu::TextureFormat::Depth32Float => TextureType::Float,
                    _ => TextureType::UnsignedInt,
                };
                depth
                    .set_type(texture_type)
                    .expect("three-rs: a depth format maps to a depth type");
                let mut inner = depth.inner().borrow_mut();
                inner.width = width;
                inner.height = height;
                inner.gpu = Some(gpu.clone());
                drop(inner);
                self.screen_read_textures
                    .insert((true, depth.id()), depth.owner(), gpu.clone());
                gpu
            }
        };

        if let Some(reads) = self.screen_reads.as_mut().filter(|_| copy) {
            reads
                .copies
                .push((reads.draw_index, FramebufferCopy::Depth(gpu)));
        }
    }

    /// Run the copies queued for one segment boundary, after the segment
    /// before it was submitted.
    pub(super) fn run_framebuffer_copies<'a>(
        &mut self,
        target: &PassTarget,
        copies: impl Iterator<Item = &'a FramebufferCopy>,
    ) {
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("three-rs copyFramebufferToTexture"),
            });
        let mut opaque_frame = false;
        for copy in copies {
            let (source, destination) = match copy {
                FramebufferCopy::OpaqueFrame => {
                    opaque_frame = true;
                    continue;
                }
                FramebufferCopy::Color(destination) => (target.color_texture.as_ref(), destination),
                FramebufferCopy::Depth(destination) => (target.depth_texture.as_ref(), destination),
            };
            let Some(source) = source else { continue };
            // The destination was sized to this pass; a depth copy has to
            // cover the whole subresource, so a mismatch is skipped rather
            // than clipped.
            if source.width() != destination.width()
                || source.height() != destination.height()
                || source.format() != destination.format()
            {
                continue;
            }
            encoder.copy_texture_to_texture(
                source.as_image_copy(),
                destination.as_image_copy(),
                wgpu::Extent3d {
                    width: source.width(),
                    height: source.height(),
                    depth_or_array_layers: 1,
                },
            );
        }
        self.queue.submit(Some(encoder.finish()));
        if opaque_frame {
            let source = target.color_texture.clone().expect(
                "three-rs: a transmissive split only happens on a target with a copy source",
            );
            self.copy_framebuffer_to_opaque_frame(&source);
        }
    }
}
