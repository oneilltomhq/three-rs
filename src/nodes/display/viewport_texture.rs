//! Ports of `three.js/src/nodes/display/ViewportTextureNode.js`,
//! `ViewportSharedTextureNode.js` and `ViewportDepthTextureNode.js`, plus
//! `viewportLinearDepth` (`ViewportDepthNode.js`) and `viewportSafeUV`
//! (`utils/ViewportUtils.js`) — reading back what the pass has drawn so far.
//!
//! Each node is a texture read whose texture its own `updateBefore()` fills:
//! `renderer.copyFramebufferToTexture()` copies the attachment of the pass
//! being drawn into the node's texture, right before the first draw that
//! reads it (`NodeUpdateType.RENDER`, so once a render per node). The
//! renderer's half — ending the pass, copying, and going on in a pass that
//! loads — is `src/renderer/screen_reads.rs`.
//!
//! Three's three textures, and how they bind:
//!
//! * [`viewport_shared_texture`] — one `FramebufferTexture` shared by every
//!   such node, `NearestFilter` both ways, so unfilterable: no sampler, and
//!   the tap is a `textureLoad`. Each node still copies on its own guard, so
//!   `webgpu_backdrop`'s eight spheres copy eight times, each seeing the
//!   spheres drawn before it.
//! * [`viewport_texture`] — a `FramebufferTexture` of the node's own with
//!   `minFilter = LinearMipmapLinearFilter`: a sampler and `textureSample`.
//!   `generateMipmaps` stays false, so the copy has one level.
//! * [`viewport_depth_texture`] — a `DepthTexture` of the node's own, bound
//!   `texture_depth_2d` and read with `textureLoad`.
//!
//! **Divergence**: three keeps one texture per render target the node is
//! drawn into (`getTextureForReference`); the port keeps one per node and
//! resizes it to whichever pass copies into it. A node read in two passes of
//! different sizes in one frame reallocates its texture twice a frame rather
//! than holding two.

use std::cell::RefCell;

use crate::nodes::node::{CustomNode, SampleMode, TextureSource, Type};
use crate::nodes::tsl::{custom, linear_depth, linear_depth_of, screen_uv, texture_node};
use crate::nodes::{NodeBuilder, NodeRef, NodeUpdateType};
use crate::renderer::Renderer;
use crate::textures::{DepthTexture, MinFilter, Texture, TextureFilter};

thread_local! {
    /// `_sharedFramebuffer` — `ViewportSharedTextureNode`'s module-level
    /// `FramebufferTexture`, made by the first node that asks.
    static SHARED_FRAMEBUFFER: RefCell<Option<Texture>> = const { RefCell::new(None) };
}

/// `new FramebufferTexture()`: no image, no mipmaps, `NearestFilter` both
/// ways, and the renderer owns the GPU texture.
fn framebuffer_texture() -> Texture {
    let texture = Texture::render_target(1, 1, wgpu::TextureFormat::Rgba8Unorm);
    texture.set_min_filter(MinFilter::Nearest);
    texture.set_mag_filter(TextureFilter::Nearest);
    texture
}

/// What a viewport node reads.
enum Framebuffer {
    /// The shared colour texture, or a node's own.
    Color(Texture),
    /// A node's own depth texture.
    Depth(DepthTexture),
}

/// `ViewportTextureNode` and its two subclasses: a texture tap at `uv`, and
/// the `updateBefore()` that fills the texture.
struct ViewportTextureNode {
    name: &'static str,
    framebuffer: Framebuffer,
    uv: NodeRef,
}

impl CustomNode for ViewportTextureNode {
    fn type_name(&self) -> &'static str {
        self.name
    }

    fn node_type(&self) -> Type {
        match self.framebuffer {
            Framebuffer::Color(_) => Type::Vec4,
            Framebuffer::Depth(_) => Type::F32,
        }
    }

    /// `TextureNode.setup()`: the uv the node was given is not the default
    /// `uv()`, so `updateMatrix` is off and no uv-matrix uniform appears.
    fn setup(&self, _builder: &NodeBuilder) -> NodeRef {
        match &self.framebuffer {
            Framebuffer::Color(texture) => {
                let mode = if texture.is_unfilterable() {
                    SampleMode::Load
                } else {
                    SampleMode::Sample
                };
                texture_node(
                    TextureSource::Texture2D(texture.clone()),
                    self.uv.clone(),
                    mode,
                    Type::Vec4,
                )
            }
            Framebuffer::Depth(depth) => texture_node(
                TextureSource::Depth(depth.clone()),
                self.uv.clone(),
                SampleMode::Load,
                Type::F32,
            ),
        }
    }

    /// `this.updateBeforeType = NodeUpdateType.RENDER`.
    fn update_before_type(&self) -> NodeUpdateType {
        NodeUpdateType::Render
    }

    /// `ViewportTextureNode.updateBefore()`: size the texture to the target
    /// and `renderer.copyFramebufferToTexture( framebufferTexture )`.
    fn update_before(&self, renderer: &mut Renderer) -> bool {
        match &self.framebuffer {
            Framebuffer::Color(texture) => renderer.copy_framebuffer_to_texture(texture),
            Framebuffer::Depth(depth) => renderer.copy_framebuffer_to_depth_texture(depth),
        }
        true
    }
}

/// `viewportSharedTexture()` — the pass so far, at `screenUV`, through the
/// one texture every shared node copies into.
pub fn viewport_shared_texture() -> NodeRef {
    viewport_shared_texture_at(screen_uv())
}

/// `viewportSharedTexture( uv )`.
pub fn viewport_shared_texture_at(uv: impl Into<NodeRef>) -> NodeRef {
    let texture = SHARED_FRAMEBUFFER.with(|shared| {
        shared
            .borrow_mut()
            .get_or_insert_with(framebuffer_texture)
            .clone()
    });
    custom(ViewportTextureNode {
        name: "ViewportSharedTextureNode",
        framebuffer: Framebuffer::Color(texture),
        uv: uv.into(),
    })
}

/// `viewportTexture()` — the pass so far, at `screenUV`, through a texture of
/// this node's own, sampled bilinearly.
pub fn viewport_texture() -> NodeRef {
    viewport_texture_at(screen_uv())
}

/// `viewportTexture( uv )`.
pub fn viewport_texture_at(uv: impl Into<NodeRef>) -> NodeRef {
    // `defaultFramebuffer.minFilter = LinearMipmapLinearFilter`; the mag
    // filter stays `NearestFilter`, so the texture is filterable and binds a
    // sampler.
    let texture = framebuffer_texture();
    texture.set_min_filter(MinFilter::LinearMipmapLinear);
    custom(ViewportTextureNode {
        name: "ViewportTextureNode",
        framebuffer: Framebuffer::Color(texture),
        uv: uv.into(),
    })
}

/// `viewportDepthTexture()` — the depth buffer so far, at `screenUV`: the raw
/// `[0,1]` depth-buffer value.
pub fn viewport_depth_texture() -> NodeRef {
    viewport_depth_texture_at(screen_uv())
}

/// `viewportDepthTexture( uv )`.
pub fn viewport_depth_texture_at(uv: impl Into<NodeRef>) -> NodeRef {
    custom(ViewportTextureNode {
        name: "ViewportDepthTextureNode",
        framebuffer: Framebuffer::Depth(DepthTexture::new()),
        uv: uv.into(),
    })
}

/// `viewportLinearDepth` — `linearDepth( viewportDepthTexture() )`: the depth
/// buffer behind the fragment, linear in `[0,1]` between the clip planes.
pub fn viewport_linear_depth() -> NodeRef {
    linear_depth_of(viewport_depth_texture())
}

/// `viewportSafeUV( uv )` — `uv`, unless what the depth buffer holds there is
/// in front of this fragment, in which case `screenUV`: a refraction offset
/// never pulls in an object that stands between the camera and the surface.
pub fn viewport_safe_uv(uv: impl Into<NodeRef>) -> NodeRef {
    let uv = uv.into();
    let depth = linear_depth();
    let depth_diff = linear_depth_of(viewport_depth_texture_at(uv.clone())).sub(depth);
    depth_diff.less_than(0.0).select(screen_uv(), uv)
}
