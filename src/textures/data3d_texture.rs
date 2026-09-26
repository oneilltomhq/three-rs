//! Ports of `three.js/src/textures/Data3DTexture.js` and
//! `src/renderers/common/Storage3DTexture.js` — the two volume textures.
//!
//! One Rust type covers both, the way `Texture` covers `StorageTexture`: they
//! differ in their constructor defaults and in whether the GPU texture is
//! created with `STORAGE_BINDING`, and nothing else the port reads.
//!
//! - `Data3DTexture` holds CPU bytes, is `NearestFilter` on both filters,
//!   `ClampToEdgeWrapping` on all three axes, `generateMipmaps = false`,
//!   `flipY = false` and `unpackAlignment = 1`. `webgpu_volume_perlin` sets
//!   `format = RedFormat` and both filters to `LinearFilter` on it.
//! - `Storage3DTexture` holds no bytes — a compute kernel writes it — and is
//!   `LinearFilter` on both filters with `mipmapsAutoUpdate = false`.
//!
//! Sampled, either is a `texture_3d<f32>` read through `texture3D()`
//! ([`crate::nodes::tsl::texture_3d`]); stored to, a
//! `texture_storage_3d<format, access>` ([`crate::nodes::tsl::texture_store`]).

use super::texture::DataLen;
use super::{MinFilter, TextureFilter, TextureId, Wrapping};
use std::cell::{Ref, RefCell};
use std::rc::Rc;

pub struct Data3DTextureInner {
    pub width: u32,
    pub height: u32,
    pub depth: u32,
    /// `image.data`, tightly packed texel rows, slice after slice — the
    /// `Uint8Array` of `webgpu_volume_perlin` for an `r8unorm` texture. `None`
    /// for a storage texture, which the GPU fills.
    pub data: Option<Vec<u8>>,
    /// `texture.format` + `texture.type`, spelled as the GPU format
    /// `WebGPUTextureUtils.getFormat()` would pick.
    pub format: wgpu::TextureFormat,
    pub mag_filter: TextureFilter,
    pub min_filter: MinFilter,
    pub wrap_s: Wrapping,
    pub wrap_t: Wrapping,
    pub wrap_r: Wrapping,
    /// `texture.isStorageTexture` — created with `STORAGE_BINDING` and bound
    /// as `texture_storage_3d` by `storageTexture()` / `textureStore()`.
    pub is_storage: bool,
    /// `texture.version`, bumped by `needsUpdate = true`.
    pub version: u32,
    uploaded: u32,
    pub gpu: Option<wgpu::Texture>,
}

/// Cloning is a handle copy, as in JS.
#[derive(Clone)]
pub struct Data3DTexture(Rc<RefCell<Data3DTextureInner>>, TextureId);

impl std::fmt::Debug for Data3DTexture {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("Data3DTexture").field(&self.0).finish()
    }
}

impl Data3DTexture {
    /// `new Data3DTexture( data, width, height, depth )` with
    /// `texture.format` already applied as `format`: `RedFormat` +
    /// `UnsignedByteType` is [`wgpu::TextureFormat::R8Unorm`], the default
    /// `RGBAFormat` is [`wgpu::TextureFormat::Rgba8Unorm`].
    ///
    /// Panics if `data` is not exactly `width * height * depth` texels of
    /// `format` — a short volume would shear every slice after the first.
    pub fn new(
        data: Vec<u8>,
        width: u32,
        height: u32,
        depth: u32,
        format: wgpu::TextureFormat,
    ) -> Self {
        let texel = format
            .block_copy_size(None)
            .expect("three-rs: the texture format has no single block size");
        assert_eq!(
            data.len(),
            (width * height * depth * texel) as usize,
            "three-rs: a {width}x{height}x{depth} {format:?} Data3DTexture holds {} bytes",
            width * height * depth * texel
        );
        Self::with(Some(data), width, height, depth, format, false)
    }

    /// `new Storage3DTexture( width, height, depth )`: `rgba8unorm` until
    /// [`set_format`](Self::set_format) says otherwise (`texture.type =
    /// HalfFloatType` is `rgba16float`), `LinearFilter` on both filters.
    ///
    /// `mipmapsAutoUpdate = false` and every read three makes of one goes
    /// through `texture3D( t, null, 0 )`, i.e. level 0, so the port allocates
    /// the one level that is ever written or read rather than the full chain
    /// three allocates and leaves empty.
    pub fn storage(width: u32, height: u32, depth: u32) -> Self {
        let texture = Self::with(
            None,
            width,
            height,
            depth,
            wgpu::TextureFormat::Rgba8Unorm,
            true,
        );
        {
            let mut inner = texture.0.borrow_mut();
            inner.mag_filter = TextureFilter::Linear;
            inner.min_filter = MinFilter::Linear;
        }
        texture
    }

    fn with(
        data: Option<Vec<u8>>,
        width: u32,
        height: u32,
        depth: u32,
        format: wgpu::TextureFormat,
        is_storage: bool,
    ) -> Self {
        Self(
            Rc::new(RefCell::new(Data3DTextureInner {
                width,
                height,
                depth,
                data,
                format,
                mag_filter: TextureFilter::Nearest,
                min_filter: MinFilter::Nearest,
                wrap_s: Wrapping::ClampToEdge,
                wrap_t: Wrapping::ClampToEdge,
                wrap_r: Wrapping::ClampToEdge,
                is_storage,
                version: 1,
                uploaded: 0,
                gpu: None,
            })),
            TextureId::next(),
        )
    }

    pub fn id(&self) -> usize {
        self.1.get()
    }

    pub fn borrow(&self) -> Ref<'_, Data3DTextureInner> {
        self.0.borrow()
    }

    pub fn size(&self) -> (u32, u32, u32) {
        let inner = self.0.borrow();
        (inner.width, inner.height, inner.depth)
    }

    pub fn format(&self) -> wgpu::TextureFormat {
        self.0.borrow().format
    }

    /// `texture.format` / `texture.type` before the first upload.
    pub fn set_format(&self, format: wgpu::TextureFormat) {
        let mut inner = self.0.borrow_mut();
        assert!(
            inner.gpu.is_none(),
            "three-rs: the format of a Data3DTexture is fixed once it is on the GPU"
        );
        inner.format = format;
    }

    pub fn is_storage(&self) -> bool {
        self.0.borrow().is_storage
    }

    pub fn set_min_filter(&self, min_filter: MinFilter) {
        self.0.borrow_mut().min_filter = min_filter;
    }

    pub fn set_mag_filter(&self, mag_filter: TextureFilter) {
        self.0.borrow_mut().mag_filter = mag_filter;
    }

    pub fn set_wrapping(&self, wrap_s: Wrapping, wrap_t: Wrapping, wrap_r: Wrapping) {
        let mut inner = self.0.borrow_mut();
        inner.wrap_s = wrap_s;
        inner.wrap_t = wrap_t;
        inner.wrap_r = wrap_r;
    }

    /// `WGSLNodeBuilder.isUnfilterable()` for this texture: `NearestFilter` on
    /// both filters (the `Data3DTexture` default), or a format the GPU cannot
    /// filter.
    pub fn is_unfilterable(&self) -> bool {
        let inner = self.0.borrow();
        let nearest =
            inner.mag_filter == TextureFilter::Nearest && inner.min_filter == MinFilter::Nearest;
        nearest
            || !matches!(
                inner.format.sample_type(None, None),
                Some(wgpu::TextureSampleType::Float { filterable: true })
            )
    }

    /// `texture.needsUpdate = true`.
    pub fn set_needs_update(&self) {
        self.0.borrow_mut().version += 1;
    }

    /// `texture.image.data.set( data ); texture.needsUpdate = true`.
    pub fn set_data(&self, data: Vec<u8>) {
        let mut inner = self.0.borrow_mut();
        assert!(
            !inner.is_storage,
            "three-rs: a Storage3DTexture is written by the GPU, not from the CPU"
        );
        let expected = inner.data.as_ref().map_or(0, Vec::len);
        assert_eq!(
            data.len(),
            expected,
            "three-rs: set_data got {} bytes for a Data3DTexture that holds {expected}",
            data.len()
        );
        inner.data = Some(data);
        inner.version += 1;
    }

    pub fn version(&self) -> u32 {
        self.0.borrow().version
    }

    pub fn set_gpu(&self, gpu: wgpu::Texture) {
        self.0.borrow_mut().gpu = Some(gpu);
    }

    pub fn has_gpu(&self) -> bool {
        self.0.borrow().gpu.is_some()
    }

    /// True when the GPU texture is missing, or — for a data texture — the
    /// bytes have moved on since the last upload. A storage texture is never
    /// uploaded to, so once it exists it is current.
    pub fn needs_upload(&self) -> bool {
        let inner = self.0.borrow();
        inner.gpu.is_none() || (!inner.is_storage && inner.uploaded != inner.version)
    }

    pub fn mark_uploaded(&self) {
        let mut inner = self.0.borrow_mut();
        inner.uploaded = inner.version;
    }

    pub fn with_gpu<R>(&self, f: impl FnOnce(&wgpu::Texture) -> R) -> R {
        let inner = self.0.borrow();
        f(inner
            .gpu
            .as_ref()
            .expect("three-rs: 3D texture not uploaded"))
    }
}

impl std::fmt::Debug for Data3DTextureInner {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Data3DTextureInner")
            .field("width", &self.width)
            .field("height", &self.height)
            .field("depth", &self.depth)
            .field("data", &self.data.as_ref().map(|data| DataLen(data.len())))
            .field("format", &self.format)
            .field("mag_filter", &self.mag_filter)
            .field("min_filter", &self.min_filter)
            .field("wrap_s", &self.wrap_s)
            .field("wrap_t", &self.wrap_t)
            .field("wrap_r", &self.wrap_r)
            .field("is_storage", &self.is_storage)
            .field("gpu", &self.gpu)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `Data3DTexture`'s constructor defaults (`Data3DTexture.js`).
    #[test]
    fn a_data_3d_texture_is_nearest_and_clamped() {
        let texture = Data3DTexture::new(vec![0; 8], 2, 2, 2, wgpu::TextureFormat::R8Unorm);
        let inner = texture.borrow();
        assert_eq!(inner.mag_filter, TextureFilter::Nearest);
        assert_eq!(inner.min_filter, MinFilter::Nearest);
        assert_eq!(inner.wrap_r, Wrapping::ClampToEdge);
        assert!(!inner.is_storage);
        drop(inner);
        assert!(texture.is_unfilterable());
        assert!(texture.needs_upload());
    }

    /// `webgpu_volume_perlin` turns both filters to `LinearFilter`, which is
    /// what makes the volume `textureSampleLevel`-able.
    #[test]
    fn linear_filters_make_it_filterable() {
        let texture = Data3DTexture::new(vec![0; 8], 2, 2, 2, wgpu::TextureFormat::R8Unorm);
        texture.set_min_filter(MinFilter::Linear);
        texture.set_mag_filter(TextureFilter::Linear);
        assert!(!texture.is_unfilterable());
    }

    #[test]
    #[should_panic(expected = "holds 32 bytes")]
    fn the_byte_count_follows_the_format() {
        Data3DTexture::new(vec![0; 8], 2, 2, 2, wgpu::TextureFormat::Rgba8Unorm);
    }

    /// `Storage3DTexture`'s constructor defaults (`Storage3DTexture.js`).
    #[test]
    fn a_storage_3d_texture_is_linear_and_empty() {
        let texture = Data3DTexture::storage(4, 4, 4);
        assert!(texture.is_storage());
        assert!(!texture.is_unfilterable());
        assert!(texture.borrow().data.is_none());
        assert_eq!(texture.format(), wgpu::TextureFormat::Rgba8Unorm);
    }

    #[test]
    fn set_data_bumps_the_version() {
        let texture = Data3DTexture::new(vec![0; 8], 2, 2, 2, wgpu::TextureFormat::R8Unorm);
        let before = texture.version();
        texture.set_data(vec![1; 8]);
        assert_eq!(texture.version(), before + 1);
    }
}
