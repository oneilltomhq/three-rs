//! Port of `three.js/src/textures/Texture.js` (the 2D path, rung-4 subset).
//!
//! One handle covers both roles the ladder needs: an image loaded from disk and
//! the colour attachment of a `RenderTarget` (`RenderTarget.texture`). Three
//! makes no distinction either — `renderTarget.texture` is a plain `Texture`
//! whose image is filled in by the backend.

use std::cell::{Ref, RefCell};
use std::rc::Rc;

use super::TextureId;
use super::{ColorSpace, TextureFilter};
use crate::math::{Matrix3, Vector2};

/// `three.js/src/constants.js` wrapping modes.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Wrapping {
    /// `ClampToEdgeWrapping` — the `Texture` default.
    ClampToEdge,
    /// `RepeatWrapping`.
    Repeat,
}

/// `Texture.minFilter` — the mip-aware half of the filter pair.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum MinFilter {
    Nearest,
    Linear,
    /// `LinearMipmapLinearFilter`, the `Texture` default.
    LinearMipmapLinear,
}

impl MinFilter {
    pub fn min(self) -> TextureFilter {
        match self {
            MinFilter::Nearest => TextureFilter::Nearest,
            _ => TextureFilter::Linear,
        }
    }

    /// `WebGPUTextureUtils` maps `*MipmapLinear` to `mipmapFilter: 'linear'`
    /// and everything else to `'nearest'`.
    pub fn mipmap(self) -> TextureFilter {
        match self {
            MinFilter::LinearMipmapLinear => TextureFilter::Linear,
            _ => TextureFilter::Nearest,
        }
    }
}

pub struct TextureInner {
    pub width: u32,
    pub height: u32,
    /// RGBA8 rows, top-down as decoded. `None` for a render-target texture.
    pub data: Option<Vec<u8>>,
    pub color_space: ColorSpace,
    pub flip_y: bool,
    pub generate_mipmaps: bool,
    pub wrap_s: Wrapping,
    pub wrap_t: Wrapping,
    pub mag_filter: TextureFilter,
    pub min_filter: MinFilter,
    pub anisotropy: u16,
    /// `Texture.offset` / `.repeat` / `.center` / `.rotation` —
    /// `updateMatrix()`'s inputs.
    pub offset: Vector2,
    pub repeat: Vector2,
    pub center: Vector2,
    pub rotation: f64,
    /// `Texture.matrix`, kept in step with the four above.
    pub matrix: Matrix3,
    /// `false` for `renderTarget.texture` — the renderer owns the GPU texture.
    pub own_gpu: bool,
    pub gpu: Option<wgpu::Texture>,
    /// The format to use if the renderer has to create it (render targets).
    pub format: wgpu::TextureFormat,
    /// `Texture.version` — "starts at 0 and counts how many times
    /// `needsUpdate` is set to true". Part of the renderer's texture cache
    /// key, exactly as `Material.version` is part of the program cache key.
    pub version: u32,
}

/// Cloning is a handle copy, as in JS.
#[derive(Clone)]
pub struct Texture(Rc<RefCell<TextureInner>>, TextureId);

/// The id is identity, not content: leaving it out keeps the `Debug` of a
/// binding description — what `examples/dump_wgsl.rs` prints beside the WGSL —
/// a function of the texture itself.
impl std::fmt::Debug for Texture {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("Texture").field(&self.0).finish()
    }
}

impl Texture {
    /// `new Texture()` defaults: `ClampToEdgeWrapping`, `LinearFilter`,
    /// `LinearMipmapLinearFilter`, `RGBAFormat`, `UnsignedByteType`,
    /// `NoColorSpace`, `flipY = true`, `generateMipmaps = true`, anisotropy 1.
    pub fn new(width: u32, height: u32, data: Option<Vec<u8>>) -> Self {
        Self(
            Rc::new(RefCell::new(TextureInner {
                width,
                height,
                data,
                color_space: ColorSpace::NoColorSpace,
                flip_y: true,
                generate_mipmaps: true,
                wrap_s: Wrapping::ClampToEdge,
                wrap_t: Wrapping::ClampToEdge,
                mag_filter: TextureFilter::Linear,
                min_filter: MinFilter::LinearMipmapLinear,
                anisotropy: 1,
                offset: Vector2::new(0.0, 0.0),
                repeat: Vector2::new(1.0, 1.0),
                center: Vector2::new(0.0, 0.0),
                rotation: 0.0,
                matrix: Matrix3::identity(),
                own_gpu: true,
                gpu: None,
                format: wgpu::TextureFormat::Rgba8Unorm,
                version: 0,
            })),
            TextureId::next(),
        )
    }

    /// The texture behind a render target's colour attachment: no image data,
    /// no mipmaps, `flipY` irrelevant, and `minFilter = LinearFilter` (which is
    /// what `RenderTarget`'s options set).
    pub fn render_target(width: u32, height: u32, format: wgpu::TextureFormat) -> Self {
        let texture = Self::new(width, height, None);
        {
            let mut inner = texture.0.borrow_mut();
            inner.generate_mipmaps = false;
            inner.min_filter = MinFilter::Linear;
            inner.own_gpu = false;
            inner.format = format;
        }
        texture
    }

    /// `new ExternalTexture( sourceTexture )` — a `wgpu::Texture` the caller
    /// created and goes on owning, wrapped so a material can sample it.
    ///
    /// This is how a host that already has GPU textures hands them to the
    /// scene: a Wayland compositor imports a client's dmabuf as a
    /// `wgpu::Texture` and wraps it here, and the renderer samples the client's
    /// pixels with no copy and no upload. Pair it with
    /// [`Renderer::with_device`](crate::renderer::Renderer::with_device), since
    /// the texture only works on the device it was created on.
    ///
    /// Size and format are taken from the wgpu texture, so they cannot drift
    /// from it. `own_gpu = false`: the renderer never re-creates, re-uploads or
    /// destroys it, and never generates mipmaps into it — the handle is a
    /// refcount, so the underlying texture lives as long as either side holds
    /// one. Filters are `LinearFilter` on both, wrapping `ClampToEdge`, and
    /// there is no mip chain to filter between.
    ///
    /// `color_space` must agree with the wgpu format's transfer function —
    /// an `*Srgb` format is [`ColorSpace::SRGB`] and everything else is
    /// [`ColorSpace::NoColorSpace`] — because the GPU applies the transfer on
    /// sample and a mismatch is a silently wrong-looking frame rather than an
    /// error. It is asserted here.
    pub fn external(gpu: wgpu::Texture, color_space: ColorSpace) -> Self {
        assert_eq!(
            gpu.dimension(),
            wgpu::TextureDimension::D2,
            "three-rs: an external Texture is a 2D texture"
        );
        assert_eq!(
            gpu.depth_or_array_layers(),
            1,
            "three-rs: an external Texture is a single layer"
        );
        assert_eq!(
            gpu.sample_count(),
            1,
            "three-rs: an external Texture is not multisampled"
        );
        assert!(
            gpu.usage().contains(wgpu::TextureUsages::TEXTURE_BINDING),
            "three-rs: an external Texture must be created with TEXTURE_BINDING"
        );
        let format = gpu.format();
        assert_eq!(
            format.is_srgb(),
            color_space == ColorSpace::SRGB,
            "three-rs: {format:?} and {color_space:?} disagree about the transfer function"
        );

        let texture = Self::new(gpu.width(), gpu.height(), None);
        {
            let mut inner = texture.0.borrow_mut();
            inner.color_space = color_space;
            inner.flip_y = false;
            inner.generate_mipmaps = false;
            inner.mag_filter = TextureFilter::Linear;
            inner.min_filter = MinFilter::Linear;
            inner.own_gpu = false;
            inner.format = format;
            inner.gpu = Some(gpu);
        }
        texture
    }

    /// `new DataTexture( new Float32Array( data ), width, height, RedFormat,
    /// FloatType )` — the SDF atlas texture lib3's `VectorFontAtlas` hands to
    /// `texture()`.
    ///
    /// `DataTexture`'s constructor differs from `Texture`'s in three places
    /// (`DataTexture.js`): `flipY = false`, `generateMipmaps = false` and
    /// `minFilter = magFilter = NearestFilter`; lib3 then sets both filters back
    /// to `LinearFilter`, which is what this builds. The colour space stays
    /// `NoColorSpace` — the field is distance, not colour, and tagging it sRGB
    /// would put a transfer function on the SDF (plan §5.3).
    pub fn data_r32float(width: u32, height: u32, data: &[f32]) -> Self {
        assert_eq!(
            data.len() as u32,
            width * height,
            "three-rs: a RedFormat DataTexture holds one float per texel"
        );
        let texture = Self::new(width, height, Some(bytemuck::cast_slice(data).to_vec()));
        {
            let mut inner = texture.0.borrow_mut();
            inner.flip_y = false;
            inner.generate_mipmaps = false;
            inner.mag_filter = TextureFilter::Linear;
            inner.min_filter = MinFilter::Linear;
            inner.format = wgpu::TextureFormat::R32Float;
        }
        texture
    }

    /// `new DataTexture( new Uint16Array( data ), width, height, RGBAFormat,
    /// HalfFloatType )` — what `DataTextureLoader` builds from
    /// [`HdrLoader::parse`](crate::loaders::HdrLoader::parse)'s half-float
    /// result.
    ///
    /// `DataTexture`'s constructor differs from `Texture`'s in three places
    /// (`DataTexture.js`): `flipY = false`, `generateMipmaps = false` and
    /// `minFilter = magFilter = NearestFilter`. `HDRLoader`'s `texData` then
    /// sets both filters back to `LinearFilter` and `flipY` to true, which
    /// [`HdrLoader::load`](crate::loaders::HdrLoader::load) does; the cube
    /// loader leaves `flipY` at the `DataTexture` default. The colour space
    /// stays `NoColorSpace` — `LinearSRGBColorSpace` is the working space and
    /// carries no transfer function, so `rgba16float` is the right format and
    /// nothing is applied on sample.
    ///
    /// `data` is one binary16 bit pattern per channel, four per texel, which is
    /// the `Uint16Array` upstream hands `write_texture` unchanged.
    pub fn data_rgba16float(width: u32, height: u32, data: &[u16]) -> Self {
        assert_eq!(
            data.len() as u32,
            width * height * 4,
            "three-rs: an RGBA HalfFloatType DataTexture holds four halves per texel"
        );
        Self::data_float(
            width,
            height,
            wgpu::TextureFormat::Rgba16Float,
            bytemuck::cast_slice(data),
        )
    }

    /// `new DataTexture( new Float32Array( data ), width, height, RGBAFormat,
    /// FloatType )` — the `FloatType` half of the same loader.
    pub fn data_rgba32float(width: u32, height: u32, data: &[f32]) -> Self {
        assert_eq!(
            data.len() as u32,
            width * height * 4,
            "three-rs: an RGBA FloatType DataTexture holds four floats per texel"
        );
        Self::data_float(
            width,
            height,
            wgpu::TextureFormat::Rgba32Float,
            bytemuck::cast_slice(data),
        )
    }

    /// The `DataTexture` defaults both float constructors share.
    fn data_float(width: u32, height: u32, format: wgpu::TextureFormat, bytes: &[u8]) -> Self {
        let texture = Self::new(width, height, Some(bytes.to_vec()));
        {
            let mut inner = texture.0.borrow_mut();
            inner.flip_y = false;
            inner.generate_mipmaps = false;
            inner.mag_filter = TextureFilter::Linear;
            inner.min_filter = MinFilter::Linear;
            inner.format = format;
        }
        texture
    }

    pub fn id(&self) -> usize {
        self.1.get()
    }

    /// `texture.colorSpace = SRGBColorSpace`. As with `CubeTexture`, the
    /// transfer function is applied by the GPU on sample — the format becomes
    /// `rgba8unorm-srgb` and `WGSLNodeBuilder.needsToWorkingColorSpace()` stays
    /// false, so no colour-space node appears in the generated WGSL.
    pub fn set_color_space(&self, color_space: ColorSpace) {
        let mut inner = self.0.borrow_mut();
        inner.color_space = color_space;
        inner.format = match color_space {
            ColorSpace::SRGB => wgpu::TextureFormat::Rgba8UnormSrgb,
            ColorSpace::NoColorSpace => wgpu::TextureFormat::Rgba8Unorm,
        };
    }

    pub fn color_space(&self) -> ColorSpace {
        self.0.borrow().color_space
    }

    /// `texture.flipY` — an upload-time concern only. `Texture.updateMatrix()`
    /// goes through `Matrix3.setUvTransform( offset, repeat, rotation, center )`,
    /// which has no `flipY` term, so the texture matrix is unaffected.
    pub fn set_flip_y(&self, flip_y: bool) {
        self.0.borrow_mut().flip_y = flip_y;
    }

    pub fn set_generate_mipmaps(&self, generate_mipmaps: bool) {
        self.0.borrow_mut().generate_mipmaps = generate_mipmaps;
    }

    pub fn set_min_filter(&self, min_filter: MinFilter) {
        self.0.borrow_mut().min_filter = min_filter;
    }

    pub fn set_mag_filter(&self, mag_filter: TextureFilter) {
        self.0.borrow_mut().mag_filter = mag_filter;
    }

    pub fn borrow(&self) -> Ref<'_, TextureInner> {
        self.0.borrow()
    }

    /// `texture.wrapS = texture.wrapT = wrapping`.
    pub fn set_wrapping(&self, wrap_s: Wrapping, wrap_t: Wrapping) {
        let mut inner = self.0.borrow_mut();
        inner.wrap_s = wrap_s;
        inner.wrap_t = wrap_t;
    }

    /// `texture.anisotropy = n`.
    pub fn set_anisotropy(&self, anisotropy: u16) {
        self.0.borrow_mut().anisotropy = anisotropy;
    }

    /// `texture.repeat.set( x, y )` — and `updateMatrix()`, which three.js runs
    /// for us every frame because `matrixAutoUpdate` is on by default.
    pub fn set_repeat(&self, x: f64, y: f64) {
        self.0.borrow_mut().repeat = Vector2::new(x, y);
        self.update_matrix();
    }

    /// `texture.offset.set( x, y )`.
    pub fn set_offset(&self, x: f64, y: f64) {
        self.0.borrow_mut().offset = Vector2::new(x, y);
        self.update_matrix();
    }

    /// `texture.center.set( x, y )`.
    pub fn set_center(&self, x: f64, y: f64) {
        self.0.borrow_mut().center = Vector2::new(x, y);
        self.update_matrix();
    }

    /// `texture.rotation = theta`.
    pub fn set_rotation(&self, rotation: f64) {
        self.0.borrow_mut().rotation = rotation;
        self.update_matrix();
    }

    /// `Texture.updateMatrix()`.
    pub fn update_matrix(&self) {
        let mut inner = self.0.borrow_mut();
        let (offset, repeat, center, rotation) =
            (inner.offset, inner.repeat, inner.center, inner.rotation);
        inner.matrix.set_uv_transform(
            offset.x, offset.y, repeat.x, repeat.y, rotation, center.x, center.y,
        );
    }

    /// `Texture.matrix` — what `TextureNode.setupUV()` multiplies the UV by.
    pub fn matrix(&self) -> Matrix3 {
        self.0.borrow().matrix
    }

    pub fn size(&self) -> (u32, u32) {
        let inner = self.0.borrow();
        (inner.width, inner.height)
    }

    pub fn set_size(&self, width: u32, height: u32) {
        let mut inner = self.0.borrow_mut();
        inner.width = width;
        inner.height = height;
    }

    /// The byte length one full mip-0 image of this texture takes: `width *
    /// height * the format's texel size`, which is what [`set_data`] asserts
    /// and what the renderer uploads.
    ///
    /// [`set_data`]: Self::set_data
    pub fn data_len(&self) -> usize {
        let inner = self.0.borrow();
        let bytes_per_texel = inner
            .format
            .block_copy_size(None)
            .expect("three-rs: the texture format has no single block size")
            as usize;
        inner.width as usize * inner.height as usize * bytes_per_texel
    }

    /// `texture.image.data.set( data ); texture.needsUpdate = true` — new
    /// pixels for a texture that has already been uploaded, at the same size
    /// and format.
    ///
    /// This is the per-frame path for a texture whose contents change but whose
    /// allocation should not: a screencast frame, an shm client buffer, a video
    /// texture. The renderer writes the bytes into the GPU texture it already
    /// has (and regenerates the mip chain if `generate_mipmaps`) instead of
    /// destroying and re-creating it, so the texture, its view and the bind
    /// groups built from it survive.
    ///
    /// Unlike `Material::set_needs_update`, no separate call is needed: setting
    /// the data *is* the change, so this bumps the version itself. Panics if
    /// `data` is not exactly [`data_len`](Self::data_len) bytes — a short
    /// buffer would otherwise shear or tear the image rather than fail.
    ///
    /// Calling it on a [`Texture::external`] or a render target's texture is a
    /// mistake: the renderer does not own those and will not upload into them.
    pub fn set_data(&self, data: Vec<u8>) {
        let expected = self.data_len();
        assert_eq!(
            data.len(),
            expected,
            "three-rs: set_data got {} bytes for a {:?} texture that holds {expected}",
            data.len(),
            self.size(),
        );
        assert!(
            self.0.borrow().own_gpu,
            "three-rs: set_data on a texture the renderer does not own \
             (an external texture, or a render target's) — write to the \
             wgpu::Texture directly"
        );
        let mut inner = self.0.borrow_mut();
        inner.data = Some(data);
        inner.version += 1;
    }

    /// `texture.needsUpdate = true`: bump the version so the next frame
    /// re-uploads the image into the GPU texture the renderer already has.
    ///
    /// [`set_data`](Self::set_data) does this for you; this is for forcing a
    /// re-upload of bytes that were not replaced through it.
    pub fn set_needs_update(&self) {
        self.0.borrow_mut().version += 1;
    }

    /// `Texture.version` — how many times the image has been marked changed.
    /// The renderer's texture cache is keyed on `( id, version )`.
    pub fn version(&self) -> u32 {
        self.0.borrow().version
    }

    pub fn set_gpu(&self, gpu: wgpu::Texture) {
        self.0.borrow_mut().gpu = Some(gpu);
    }

    pub fn clear_gpu(&self) {
        self.0.borrow_mut().gpu = None;
    }

    pub fn has_gpu(&self) -> bool {
        self.0.borrow().gpu.is_some()
    }

    pub fn with_gpu<R>(&self, f: impl FnOnce(&wgpu::Texture) -> R) -> R {
        let inner = self.0.borrow();
        f(inner.gpu.as_ref().expect("three-rs: texture not uploaded"))
    }

    pub fn format(&self) -> wgpu::TextureFormat {
        self.0.borrow().format
    }

    pub fn set_format(&self, format: wgpu::TextureFormat) {
        self.0.borrow_mut().format = format;
    }

    /// `Texture.mipmapCount` — `floor( log2( max( w, h ) ) ) + 1`.
    pub fn mip_level_count(&self) -> u32 {
        let inner = self.0.borrow();
        if !inner.generate_mipmaps {
            return 1;
        }
        let max = inner.width.max(inner.height) as f64;
        (max.log2().floor() as u32) + 1
    }
}

/// The derived `Debug` printed every byte of `data` — megabytes for an SDF
/// atlas, and the program cache key used to be a `format!( "{:?}" )` of a
/// binding description that reaches here. The length stands in for the pixels.
impl std::fmt::Debug for TextureInner {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TextureInner")
            .field("width", &self.width)
            .field("height", &self.height)
            .field("data", &self.data.as_ref().map(|data| DataLen(data.len())))
            .field("color_space", &self.color_space)
            .field("flip_y", &self.flip_y)
            .field("generate_mipmaps", &self.generate_mipmaps)
            .field("wrap_s", &self.wrap_s)
            .field("wrap_t", &self.wrap_t)
            .field("mag_filter", &self.mag_filter)
            .field("min_filter", &self.min_filter)
            .field("anisotropy", &self.anisotropy)
            .field("offset", &self.offset)
            .field("repeat", &self.repeat)
            .field("center", &self.center)
            .field("rotation", &self.rotation)
            .field("matrix", &self.matrix)
            .field("own_gpu", &self.own_gpu)
            .field("gpu", &self.gpu)
            .field("format", &self.format)
            .finish()
    }
}

/// `data: 1048576 bytes` in place of a million numbers.
pub(crate) struct DataLen(pub usize);

impl std::fmt::Debug for DataLen {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} bytes", self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rgba(width: u32, height: u32) -> Texture {
        Texture::new(
            width,
            height,
            Some(vec![0u8; (width * height * 4) as usize]),
        )
    }

    #[test]
    fn a_new_texture_is_at_version_zero() {
        assert_eq!(rgba(2, 2).version(), 0);
    }

    #[test]
    fn set_data_replaces_the_image_and_bumps_the_version() {
        let texture = rgba(2, 2);
        texture.set_data(vec![7u8; 16]);

        assert_eq!(texture.version(), 1);
        assert_eq!(texture.borrow().data.as_deref(), Some(&[7u8; 16][..]));

        texture.set_data(vec![9u8; 16]);
        assert_eq!(texture.version(), 2);
    }

    #[test]
    fn needs_update_bumps_the_version_on_its_own() {
        let texture = rgba(2, 2);
        texture.set_needs_update();
        texture.set_needs_update();
        assert_eq!(texture.version(), 2);
    }

    /// The byte length follows the format, not a hardcoded RGBA8: the SDF
    /// atlas is `r32float`, one float per texel.
    #[test]
    fn data_len_follows_the_format() {
        assert_eq!(rgba(3, 5).data_len(), 3 * 5 * 4);

        let atlas = Texture::data_r32float(3, 5, &[0.0; 15]);
        assert_eq!(atlas.data_len(), 3 * 5 * 4);
    }

    #[test]
    #[should_panic(expected = "set_data got")]
    fn set_data_rejects_the_wrong_byte_count() {
        rgba(2, 2).set_data(vec![0u8; 15]);
    }

    /// An external texture's GPU handle is the caller's; the renderer must not
    /// be able to overwrite it through the CPU path.
    #[test]
    #[should_panic(expected = "does not own")]
    fn set_data_refuses_a_texture_the_renderer_does_not_own() {
        let target = Texture::render_target(2, 2, wgpu::TextureFormat::Rgba8Unorm);
        target.set_data(vec![0u8; 16]);
    }
}
