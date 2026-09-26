//! Ports of `three.js/src/textures/CompressedTexture.js` and
//! `CompressedArrayTexture.js`.
//!
//! Upstream both are `Texture` subclasses whose `image` is `{ width, height
//! [, depth] }` and whose pixels live in `mipmaps`. Here they are constructors
//! of the one [`Texture`] handle, the way `DataTexture`'s float variants are
//! (`Texture::data_rgba16float`): a compressed texture has to fit every slot a
//! `Texture` fits — a glTF `KHR_texture_basisu` base colour map is a
//! `MeshStandardMaterial.map` like any PNG — so it cannot be a separate type.
//! What makes it compressed is that [`TextureInner::mipmaps`] is filled and
//! `format` is a block-compressed `wgpu::TextureFormat`; what makes it an
//! array is [`TextureInner::depth`].
//!
//! [`TextureInner::mipmaps`]: super::TextureInner::mipmaps
//! [`TextureInner::depth`]: super::TextureInner::depth

use super::texture::{Mipmap, Texture};
use super::{MinFilter, TextureFilter};

impl Texture {
    /// `new CompressedTexture( mipmaps, width, height, format, type )`.
    ///
    /// The constructor's own differences from `Texture` (`CompressedTexture.js`):
    /// `flipY = false` — "can't flip compressed textures" — and
    /// `generateMipmaps = false` — "can't generate mipmaps for compressed
    /// textures". The filters stay at the `Texture` defaults; `KTX2Loader`
    /// overrides them.
    ///
    /// `format` is the GPU format the renderer creates the texture with —
    /// the port's stand-in for three's `( format, type, colorSpace )` triple,
    /// which `WebGPUTextureUtils.getFormat()` turns into one — and every
    /// mip's `data` must be laid out in it.
    pub fn compressed(
        mipmaps: Vec<Mipmap>,
        width: u32,
        height: u32,
        format: wgpu::TextureFormat,
    ) -> Self {
        assert!(
            !mipmaps.is_empty(),
            "three-rs: a CompressedTexture needs at least one mip level"
        );
        let texture = Texture::new(width, height, None);
        {
            let mut inner = texture.borrow_mut_inner();
            inner.flip_y = false;
            inner.generate_mipmaps = false;
            inner.format = format;
            inner.mipmaps = mipmaps;
        }
        texture
    }

    /// `new CompressedArrayTexture( mipmaps, width, height, depth, format, type )`
    /// — `image.depth` layers, each mip's `data` holding every layer of the
    /// level in order. `wrapR` is `ClampToEdgeWrapping` upstream and has no
    /// sampler field to go to in a 2-D-array view, so it is not stored.
    pub fn compressed_array(
        mipmaps: Vec<Mipmap>,
        width: u32,
        height: u32,
        depth: u32,
        format: wgpu::TextureFormat,
    ) -> Self {
        assert!(
            depth >= 1,
            "three-rs: a CompressedArrayTexture has at least one layer"
        );
        let texture = Texture::compressed(mipmaps, width, height, format);
        texture.borrow_mut_inner().depth = depth;
        texture
    }

    /// `texture.minFilter = min; texture.magFilter = mag`.
    pub fn set_filters(&self, min_filter: MinFilter, mag_filter: TextureFilter) {
        let mut inner = self.borrow_mut_inner();
        inner.min_filter = min_filter;
        inner.mag_filter = mag_filter;
    }

    /// `texture.premultiplyAlpha = value`.
    pub fn set_premultiply_alpha(&self, premultiply_alpha: bool) {
        self.borrow_mut_inner().premultiply_alpha = premultiply_alpha;
    }

    /// `texture.isCompressedArrayTexture` — sampled with a layer index
    /// through a `texture_2d_array<f32>` binding.
    pub fn is_array(&self) -> bool {
        self.borrow().depth > 0
    }

    /// `texture.mipmaps.length > 0` — the texture uploads the levels it was
    /// given rather than `data`.
    pub fn has_mipmaps(&self) -> bool {
        !self.borrow().mipmaps.is_empty()
    }
}
