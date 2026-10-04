//! Port of `three.js/src/core/RenderTarget.js` (the subset rungs 1–2 need).

use std::cell::RefCell;
use std::rc::Rc;

use crate::error::Error;
use crate::math::Vector4;
use crate::textures::{DepthTexture, MinFilter, Texture, TextureFilter, TextureType};

/// `renderTarget.texture` and each `getTexture( name )` clone of it: a colour
/// attachment carrying the target's filter pair, which is what
/// `pass( scene, camera, { minFilter, magFilter } )` sets and what decides
/// whether the shader that samples it gets a sampler at all
/// ([`Texture::is_unfilterable`]).
fn color_attachment(
    width: u32,
    height: u32,
    format: wgpu::TextureFormat,
    min_filter: TextureFilter,
    mag_filter: TextureFilter,
) -> Texture {
    let texture = Texture::render_target(width, height, format);
    texture.set_min_filter(match min_filter {
        TextureFilter::Nearest => MinFilter::Nearest,
        TextureFilter::Linear => MinFilter::Linear,
    });
    texture.set_mag_filter(mag_filter);
    texture
}

/// The name three.js gives `renderTarget.textures[ 0 ]` through
/// `PassNode`: `getTextureNode()`'s default argument, and the key the scene
/// pass's own colour lands on in `mrt( { output, … } )`.
pub const OUTPUT_ATTACHMENT: &str = "output";

/// `new RenderTarget( width, height, options )` — the options the port reads.
#[derive(Clone, Copy, Debug)]
#[non_exhaustive]
pub struct RenderTargetOptions {
    /// `options.type` — the colour attachment's pixel format.
    pub texture_type: TextureType,
    /// `RenderTarget`'s default `samples` is 0; the renderer's `antialias`
    /// option does **not** propagate to user render targets, only to the
    /// internal framebuffer target.
    pub samples: u32,
    /// `options.depthBuffer` — whether the target gets a depth attachment.
    pub depth_buffer: bool,
    /// `options.minFilter` — the colour attachment's minification filter.
    pub min_filter: TextureFilter,
    /// `options.magFilter` — the colour attachment's magnification filter.
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
pub(crate) struct RenderTargetInner {
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
    /// works; the renderer owns its GPU object (`own_gpu` is false). This is
    /// `renderTarget.textures[ 0 ]`, whose `name` three.js leaves empty and
    /// `PassNode` treats as [`OUTPUT_ATTACHMENT`].
    pub texture: Texture,
    /// `renderTarget.textures` past the first: the extra MRT colour
    /// attachments, in the order `PassNode.getTexture( name )` pushed them,
    /// which is the order their `@location`s are assigned in.
    ///
    /// Three clones `renderTarget.texture` for each, so every attachment shares
    /// the target's size, type and filters; only the name differs.
    pub extra_textures: Vec<(String, Texture)>,
    /// `PassNode._previousTextures` — a second texture per output name, sized
    /// and allocated with the target but **never** a colour attachment. See
    /// [`RenderTarget::add_previous_texture`].
    pub previous_textures: Vec<(String, Texture)>,
    /// The MSAA colour texture `samples > 1` asks for; the single-sample
    /// `color` texture is then its resolve target.
    pub msaa: Option<wgpu::Texture>,
    /// The same, one per entry of `extra_textures`: a multisampled target
    /// whose resolve is that attachment. `webgpu_mrt` is `antialias: true`
    /// with four attachments, and WebGPU requires every colour attachment of a
    /// pass to have the same sample count.
    pub msaa_extra: Vec<wgpu::Texture>,
    /// The depth buffer auto-allocated when `depth_buffer` is set and no
    /// `DepthTexture` was attached.
    pub depth: Option<wgpu::Texture>,
    /// `renderTarget.viewport` — `new Vector4( 0, 0, width, height )`, in the
    /// target's own pixels (a render target has no pixel ratio). A pass into
    /// this target restricts itself to this rectangle; see
    /// [`Renderer::set_viewport`](super::Renderer::set_viewport) for the origin
    /// convention, which is the same one.
    pub viewport: Vector4,
    /// `renderTarget.scissor`, applied only when `scissor_test` is set.
    pub scissor: Vector4,
    /// `renderTarget.scissorTest`.
    pub scissor_test: bool,
    /// `_textures.get( renderTarget ).depthInitialized` — whether anything has
    /// yet rendered into this target's depth attachment. See
    /// `Renderer::render`'s manual first clear.
    pub depth_initialized: bool,
    /// Whether the colour texture is also a copy destination
    /// (`COPY_DST`) — see [`RenderTarget::set_copy_destination`].
    pub copy_destination: bool,
}

/// Cloning is a handle copy, matching JS object identity.
#[derive(Clone, Debug)]
pub struct RenderTarget(Rc<RefCell<RenderTargetInner>>);

impl RenderTarget {
    /// `new RenderTarget( width, height )` — the default options' texture type
    /// is `UnsignedByteType`, so this one cannot fail.
    pub fn new(width: u32, height: u32) -> Self {
        Self::new_with_options(width, height, RenderTargetOptions::default())
            .expect("three-rs: the default render target options are a colour type")
    }

    /// Errors when `options.texture_type` has no colour format — three.js
    /// would have produced a target that fails at the first pass instead.
    pub fn new_with_options(
        width: u32,
        height: u32,
        options: RenderTargetOptions,
    ) -> Result<Self, Error> {
        if !options.texture_type.is_color() {
            return Err(Error::UnsupportedTextureType {
                what: "colour",
                texture_type: options.texture_type,
            });
        }

        Ok(Self(Rc::new(RefCell::new(RenderTargetInner {
            width,
            height,
            samples: options.samples,
            texture_type: options.texture_type,
            depth_buffer: options.depth_buffer,
            min_filter: options.min_filter,
            mag_filter: options.mag_filter,
            depth_texture: None,
            texture: color_attachment(
                width,
                height,
                options.texture_type.color_gpu_format(),
                options.min_filter,
                options.mag_filter,
            ),
            extra_textures: Vec::new(),
            previous_textures: Vec::new(),
            msaa: None,
            msaa_extra: Vec::new(),
            depth: None,
            viewport: Vector4::new(0.0, 0.0, width as f64, height as f64),
            scissor: Vector4::new(0.0, 0.0, width as f64, height as f64),
            scissor_test: false,
            depth_initialized: false,
            copy_destination: false,
        }))))
    }

    /// `renderTarget.clone()` — a second target with the same descriptor and
    /// its **own** textures, which is what `SSAAPassNode.setup()` builds its
    /// `_sampleRenderTarget` from.
    ///
    /// Not `Clone::clone`: that is the handle copy, and it has to stay one,
    /// because a `RenderTarget` is a JS object identity everywhere else in the
    /// port. `RenderTarget.copy()` copies the depth texture *reference*; this
    /// gives the copy a fresh `DepthTexture` instead, because `RenderTarget`'s
    /// constructor is what attached the original's and three.js' own
    /// `_sampleRenderTarget` ends up with a depth texture of its own (the
    /// dump's textures 5 and 28, one per target).
    pub fn clone_target(&self) -> Self {
        let inner = self.0.borrow();
        let clone = Self::new_with_options(
            inner.width,
            inner.height,
            RenderTargetOptions {
                texture_type: inner.texture_type,
                samples: inner.samples,
                depth_buffer: inner.depth_buffer,
                min_filter: inner.min_filter,
                mag_filter: inner.mag_filter,
            },
        )
        .expect("three-rs: the source target's texture type is already a colour type");
        clone.0.borrow().texture.set_format(inner.texture.format());
        if inner.depth_texture.is_some() {
            clone.set_depth_texture(DepthTexture::new());
        }
        for (name, _) in &inner.extra_textures {
            clone.add_texture(name);
        }
        clone
    }

    /// `PassNode.getTexture( name )`'s second half: `renderTarget.texture`
    /// cloned, named, and pushed onto `renderTarget.textures`. Idempotent, as
    /// three.js's own `_textures[ name ]` memo makes it.
    ///
    /// The clone is a *fresh* texture, not a handle copy: each attachment needs
    /// its own GPU object, and its own identity so that `texture( … )` on it
    /// binds the right one.
    #[doc(hidden)]
    pub fn add_texture(&self, name: &str) -> Texture {
        if name == OUTPUT_ATTACHMENT {
            return self.texture();
        }
        let mut inner = self.0.borrow_mut();
        if let Some((_, texture)) = inner.extra_textures.iter().find(|(n, _)| n == name) {
            return texture.clone();
        }
        let texture = color_attachment(
            inner.width,
            inner.height,
            inner.texture.format(),
            inner.min_filter,
            inner.mag_filter,
        );
        inner
            .extra_textures
            .push((name.to_string(), texture.clone()));
        texture
    }

    /// `PassNode.getPreviousTexture( name )`: a second texture behind one
    /// output name, holding the frame before this one. It shares the target's
    /// descriptor and is allocated and resized with it, but it is never a
    /// colour attachment — nothing is ever rendered into it directly.
    ///
    /// **How the toggle is modelled.** `PassNode.toggleTexture()` swaps the
    /// two `Texture` objects — the one in `renderTarget.textures` and the one
    /// held aside — and then calls `updateTexture()` on the two `TextureNode`s
    /// so that "current" and "previous" follow them. A `NodeRef` in this port
    /// is immutable, so the port alternates the other end:
    /// `getTextureNode()`'s node always samples the attachment's texture,
    /// `getPreviousTextureNode()`'s always samples this one, and
    /// [`Self::toggle_texture`] exchanges the **GPU textures** behind the two
    /// handles. The GPU sees the same two allocations alternating in the same
    /// order; what differs is that the port's bind groups keep their texture
    /// ids across the swap, where three.js' change every frame.
    ///
    /// Idempotent, as `_previousTextures[ name ]` makes it.
    pub(crate) fn add_previous_texture(&self, name: &str) -> Texture {
        let mut inner = self.0.borrow_mut();
        if let Some((_, texture)) = inner.previous_textures.iter().find(|(n, _)| n == name) {
            return texture.clone();
        }
        let texture = color_attachment(
            inner.width,
            inner.height,
            inner.texture.format(),
            inner.min_filter,
            inner.mag_filter,
        );
        inner
            .previous_textures
            .push((name.to_string(), texture.clone()));
        texture
    }

    /// `PassNode.toggleTexture( name )` — see [`Self::add_previous_texture`].
    /// A no-op for a name that has no previous texture, exactly as three's is.
    pub(crate) fn toggle_texture(&self, name: &str) {
        let inner = self.0.borrow();
        let Some((_, previous)) = inner.previous_textures.iter().find(|(n, _)| n == name) else {
            return;
        };
        let current = if name == OUTPUT_ATTACHMENT {
            inner.texture.clone()
        } else {
            match inner.extra_textures.iter().find(|(n, _)| n == name) {
                Some((_, texture)) => texture.clone(),
                None => return,
            }
        };
        current.swap_gpu(previous);
    }

    /// `renderTarget.textures.map( texture => texture.name )` — what
    /// `MRTNode.setup()` resolves its output names against, so the position of
    /// a name here is the `@location` its value is written to.
    #[doc(hidden)]
    pub fn attachment_names(&self) -> Vec<String> {
        let inner = self.0.borrow();
        let mut names = vec![OUTPUT_ATTACHMENT.to_string()];
        names.extend(inner.extra_textures.iter().map(|(n, _)| n.clone()));
        names
    }

    /// `new RenderTarget( width, height, { count } )`: `count - 1` clones of
    /// `renderTarget.texture` pushed onto `renderTarget.textures`, each with
    /// the target's size, type and filters and an empty `name` — the page
    /// names them afterwards ([`Self::set_texture_name`]), and `MRTNode`
    /// writes only the ones whose name its outputs carry.
    ///
    /// A setter rather than a [`RenderTargetOptions`] field, for the same
    /// reason as [`Self::set_rg_format`]. It only ever grows the list, and it
    /// has to run before the target is first drawn into, as three's
    /// constructor option does by construction.
    pub fn set_count(&self, count: usize) {
        let mut inner = self.0.borrow_mut();
        while inner.extra_textures.len() + 1 < count {
            let texture = color_attachment(
                inner.width,
                inner.height,
                inner.texture.format(),
                inner.min_filter,
                inner.mag_filter,
            );
            inner.extra_textures.push((String::new(), texture));
        }
    }

    /// `renderTarget.textures[ index ].name = name` — what the attachment
    /// answers to in `mrt( { name: … } )` and in [`Self::add_texture`].
    ///
    /// Attachment 0 is always [`OUTPUT_ATTACHMENT`] in this port (the name
    /// lives on the target, not the texture), so naming it anything else
    /// panics rather than silently leaving `mrt( { output } )` unwritten.
    pub fn set_texture_name(&self, index: usize, name: &str) {
        if index == 0 {
            assert_eq!(
                name, OUTPUT_ATTACHMENT,
                "three-rs: renderTarget.textures[ 0 ] is always named `{OUTPUT_ATTACHMENT}` here"
            );
            return;
        }
        let mut inner = self.0.borrow_mut();
        let entry = inner
            .extra_textures
            .get_mut(index - 1)
            .expect("three-rs: set_texture_name past renderTarget.textures' count");
        entry.0 = name.to_string();
    }

    /// `renderTarget.textures` — attachment 0 first.
    pub fn textures(&self) -> Vec<Texture> {
        let inner = self.0.borrow();
        let mut textures = vec![inner.texture.clone()];
        textures.extend(inner.extra_textures.iter().map(|(_, t)| t.clone()));
        textures
    }

    /// `renderTarget.depthTexture = texture`.
    pub fn set_depth_texture(&self, depth_texture: DepthTexture) {
        let mut inner = self.0.borrow_mut();
        depth_texture.set_multisample(inner.samples > 1);
        inner.depth_texture = Some(depth_texture);
    }

    /// Let the colour texture be the destination of a texture copy —
    /// `renderer.copyTextureToTexture( …, renderTarget.texture )`, which
    /// `TRAANode` updates its history with. WebGPU fixes a texture's usage at
    /// creation, and three's backend gives every render target `COPY_DST`;
    /// the port asks only where a copy lands, so no other target's texture
    /// changes. Drops the GPU texture if it was created without the flag.
    pub(crate) fn set_copy_destination(&self) {
        let mut inner = self.0.borrow_mut();
        if !inner.copy_destination {
            inner.copy_destination = true;
            inner.texture.clear_gpu();
        }
    }

    /// `renderTarget.depthTexture`.
    pub fn depth_texture(&self) -> Option<DepthTexture> {
        self.0.borrow().depth_texture.clone()
    }

    /// `options.format = RGFormat` — a two-channel colour attachment, which
    /// is what `ShadowNode` asks for its VSM blur targets
    /// (`{ format: RGFormat, type: HalfFloatType }`). The fragment programs
    /// drawn into it then write a `vec2` (`NodeBuilder.getOutputType()`).
    ///
    /// A setter rather than a [`RenderTargetOptions`] field, so that adding
    /// the one format the ladder needs does not break every literal
    /// construction of the options.
    pub fn set_rg_format(&self) {
        let inner = self.0.borrow();
        let format = match inner.texture_type {
            TextureType::HalfFloat => wgpu::TextureFormat::Rg16Float,
            _ => wgpu::TextureFormat::Rg8Unorm,
        };
        inner.texture.set_format(format);
        inner.texture.clear_gpu();
    }

    /// `options.format = RedFormat` — a one-channel colour attachment:
    /// `DepthOfFieldNode`'s CoC targets (`{ format: RedFormat, type:
    /// HalfFloatType }`). The fragment programs drawn into it write an `f32`
    /// (`NodeBuilder.getOutputType()`), and a texture node sampling it is a
    /// `float` node, so every tap reads `.x`.
    ///
    /// Call it before [`set_count`](Self::set_count): the extra attachments
    /// take the first one's format when they are made.
    ///
    /// The match is spelled out rather than defaulting to `R8Unorm`, so a
    /// type with no red format here can never be quantised to eight bits:
    /// `FloatType` / `UnsignedIntType` colour targets are refused by
    /// [`RenderTarget::new_with_options`], as they are for `RGBAFormat`, so
    /// they cannot reach this setter.
    pub fn set_red_format(&self) {
        let inner = self.0.borrow();
        let format = match inner.texture_type {
            TextureType::UnsignedByte => wgpu::TextureFormat::R8Unorm,
            TextureType::HalfFloat => wgpu::TextureFormat::R16Float,
            other => panic!(
                "three-rs: a RedFormat render target needs an UnsignedByteType \
                 or HalfFloatType colour attachment, got {other:?}"
            ),
        };
        inner.texture.set_format(format);
        inner.texture.clear_gpu();
    }

    /// `(renderTarget.width, renderTarget.height)`.
    pub fn size(&self) -> (u32, u32) {
        let inner = self.0.borrow();
        (inner.width, inner.height)
    }

    /// `RenderTarget.setSize()` — drops the GPU textures so they are recreated.
    pub fn set_size(&self, width: u32, height: u32) {
        self.set_size_keeping_depth(width, height, false)
    }

    /// `setSize()` for a target whose `depthTexture` belongs to *another*
    /// target — `pass( scene, camera, { depthTexture } )`. Dropping the shared
    /// texture's GPU handle here would throw away the depth the owning pass
    /// has already rendered, which is the whole point of sharing it.
    pub(crate) fn set_size_keeping_depth(&self, width: u32, height: u32, shared_depth: bool) {
        let mut inner = self.0.borrow_mut();
        if inner.width != width || inner.height != height {
            inner.width = width;
            inner.height = height;
            inner.texture.set_size(width, height);
            inner.texture.clear_gpu();
            // `RenderTarget.setSize()`: the viewport and the scissor go back to
            // the whole target.
            inner.viewport = Vector4::new(0.0, 0.0, width as f64, height as f64);
            inner.scissor = Vector4::new(0.0, 0.0, width as f64, height as f64);
            inner.msaa = None;
            inner.msaa_extra.clear();
            inner.depth = None;
            if !shared_depth {
                if let Some(depth_texture) = &inner.depth_texture {
                    depth_texture.inner().borrow_mut().gpu = None;
                }
            }
            for (_, texture) in inner.extra_textures.iter().chain(&inner.previous_textures) {
                texture.set_size(width, height);
                texture.clear_gpu();
            }
        }
    }

    /// `_textures.get( renderTarget ).depthInitialized`.
    pub(crate) fn depth_initialized(&self) -> bool {
        self.0.borrow().depth_initialized
    }

    pub(crate) fn set_depth_initialized(&self, initialized: bool) {
        self.0.borrow_mut().depth_initialized = initialized;
    }

    /// `renderTarget.depthBuffer`.
    pub fn depth_buffer(&self) -> bool {
        self.0.borrow().depth_buffer
    }

    /// `renderTarget.texture`.
    pub fn texture(&self) -> Texture {
        self.0.borrow().texture.clone()
    }

    /// `renderTarget.samples`.
    pub fn samples(&self) -> u32 {
        self.0.borrow().samples
    }

    /// `renderTarget.samples = renderer.samples`, which is what
    /// `PassNode.setup()` does before the first nested render.
    pub fn set_samples(&self, samples: u32) {
        let mut inner = self.0.borrow_mut();
        if inner.samples == samples {
            return;
        }
        inner.samples = samples;
        inner.msaa = None;
        inner.msaa_extra.clear();
        inner.depth = None;
        // An attached `DepthTexture` is allocated with the target's sample
        // count, so the flag the node builder reads has to move with it —
        // `pass( scene, camera )` under `antialias: true` binds
        // `texture_depth_multisampled_2d`.
        if let Some(depth_texture) = &inner.depth_texture {
            depth_texture.set_multisample(samples > 1);
        }
    }

    /// `renderTarget.viewport`.
    pub fn viewport(&self) -> Vector4 {
        self.0.borrow().viewport
    }

    /// `renderTarget.viewport.set( x, y, width, height )` — the rectangle of
    /// *this* target a render into it is confined to, and what
    /// `PMREMGenerator._setViewport()` sets before each tile of the cubeUV
    /// atlas is rendered, so that 21 passes share two textures. Origin
    /// top-left, in the target's own pixels; see
    /// [`Renderer::set_viewport`](super::Renderer::set_viewport).
    ///
    /// `( 0, 0, width, height )` — the default, and what [`Self::set_size`]
    /// restores — is the whole target, which is the rectangle wgpu would use
    /// anyway.
    pub fn set_viewport(&self, x: f64, y: f64, width: f64, height: f64) {
        self.0.borrow_mut().viewport = Vector4::new(x, y, width, height);
    }

    /// `renderTarget.scissor`.
    pub fn scissor(&self) -> Vector4 {
        self.0.borrow().scissor
    }

    /// `renderTarget.scissor.set( x, y, width, height )`.
    pub fn set_scissor(&self, x: f64, y: f64, width: f64, height: f64) {
        self.0.borrow_mut().scissor = Vector4::new(x, y, width, height);
    }

    /// `renderTarget.scissorTest`.
    pub fn scissor_test(&self) -> bool {
        self.0.borrow().scissor_test
    }

    /// `renderTarget.scissorTest = value`.
    pub fn set_scissor_test(&self, scissor_test: bool) {
        self.0.borrow_mut().scissor_test = scissor_test;
    }

    pub(crate) fn inner(&self) -> &RefCell<RenderTargetInner> {
        &self.0
    }
}
