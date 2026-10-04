//! Port of `three.js/src/textures/DataTexture.js` — the flat 2-D data texture
//! `BatchedMesh` keeps its per-instance matrices, colours and the indirect
//! (draw-ordinal → instance-id) table in.
//!
//! Three's `DataTexture` is `NearestFilter` with no mipmaps and is only ever
//! read with `textureLoad`, so — like `DataArrayTexture` — it has no sampler
//! binding at all.

use super::TextureId;
use std::cell::{Ref, RefCell};
use std::rc::Rc;

/// `texture.type` / `texture.format`, the only two combinations the ladder
/// needs: `FloatType` + `RGBAFormat` (`rgba32float`) and `UnsignedIntType` +
/// `RedIntegerFormat` (`r32uint`).
#[derive(Clone, Debug)]
#[non_exhaustive]
pub enum DataTextureData {
    /// `Float32Array`, four channels per texel — `rgba32float`.
    F32(Vec<f32>),
    /// `Uint32Array`, one channel per texel — `r32uint`.
    U32(Vec<u32>),
}

/// The state behind a [`DataTexture`] handle.
pub struct DataTextureInner {
    /// `texture.image.data`.
    pub data: DataTextureData,
    /// `texture.image.width`.
    pub width: u32,
    /// `texture.image.height`.
    pub height: u32,
    /// `texture.version`, bumped by `needsUpdate = true`. The renderer
    /// re-uploads when this runs ahead of `uploaded`.
    pub version: u32,
    uploaded: u32,
    pub(crate) gpu: Option<wgpu::Texture>,
}

/// Cloning is a handle copy, as in JS.
#[derive(Clone)]
pub struct DataTexture(Rc<RefCell<DataTextureInner>>, TextureId);

impl std::fmt::Debug for DataTexture {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("DataTexture").field(&self.0).finish()
    }
}

impl DataTexture {
    /// `dataTexture.dispose()` — drops this handle, and nothing more: a texture
    /// handle is an `Rc`, so the renderer frees the GPU side once every handle
    /// is gone, at the top of the next render (`docs/scene-graph.md`,
    /// "Identity and eviction"). Dropping the handle does the same; this
    /// exists so three.js code ports line for line.
    pub fn dispose(self) {}

    /// `new DataTexture( new Float32Array( w * h * 4 ), w, h, RGBAFormat, FloatType )`.
    pub fn new_f32(data: Vec<f32>, width: u32, height: u32) -> Self {
        Self::new(DataTextureData::F32(data), width, height)
    }

    /// `new DataTexture( new Uint32Array( w * h ), w, h, RedIntegerFormat, UnsignedIntType )`.
    pub fn new_u32(data: Vec<u32>, width: u32, height: u32) -> Self {
        Self::new(DataTextureData::U32(data), width, height)
    }

    fn new(data: DataTextureData, width: u32, height: u32) -> Self {
        Self(
            Rc::new(RefCell::new(DataTextureInner {
                data,
                width,
                height,
                version: 1,
                uploaded: 0,
                gpu: None,
            })),
            TextureId::next(),
        )
    }

    /// `texture.id` — unique per texture, stable for its lifetime.
    pub fn id(&self) -> usize {
        self.1.get()
    }

    /// The handle's liveness, without the handle; see [`TextureOwner`].
    ///
    /// [`TextureOwner`]: super::TextureOwner
    pub(crate) fn owner(&self) -> super::TextureOwner {
        Rc::downgrade(&self.0) as super::TextureOwner
    }

    /// Borrows the texture's data, width and height.
    pub fn borrow(&self) -> Ref<'_, DataTextureInner> {
        self.0.borrow()
    }

    /// `( texture.image.width, texture.image.height )`.
    pub fn size(&self) -> (u32, u32) {
        let inner = self.0.borrow();
        (inner.width, inner.height)
    }

    /// `texture.image.data` is an integer array — the binding is
    /// `texture_2d<u32>` rather than `texture_2d<f32>`.
    pub(crate) fn is_uint(&self) -> bool {
        matches!(self.0.borrow().data, DataTextureData::U32(_))
    }

    /// `texture.needsUpdate = true`.
    pub fn set_needs_update(&self) {
        self.0.borrow_mut().version += 1;
    }

    /// Edit the float payload in place and bump the version.
    pub fn with_f32_mut<R>(&self, f: impl FnOnce(&mut [f32]) -> R) -> R {
        let mut inner = self.0.borrow_mut();
        inner.version += 1;
        match &mut inner.data {
            DataTextureData::F32(v) => f(v),
            DataTextureData::U32(_) => panic!("three-rs: data texture is not a float texture"),
        }
    }

    /// Edit the uint payload in place and bump the version.
    pub fn with_u32_mut<R>(&self, f: impl FnOnce(&mut [u32]) -> R) -> R {
        let mut inner = self.0.borrow_mut();
        inner.version += 1;
        match &mut inner.data {
            DataTextureData::U32(v) => f(v),
            DataTextureData::F32(_) => panic!("three-rs: data texture is not a uint texture"),
        }
    }

    /// Read the float payload — `getMatrixAt` and the unit tests.
    pub fn with_f32<R>(&self, f: impl FnOnce(&[f32]) -> R) -> R {
        let inner = self.0.borrow();
        match &inner.data {
            DataTextureData::F32(v) => f(v),
            DataTextureData::U32(_) => panic!("three-rs: data texture is not a float texture"),
        }
    }

    /// Read the uint payload.
    pub fn with_u32<R>(&self, f: impl FnOnce(&[u32]) -> R) -> R {
        let inner = self.0.borrow();
        match &inner.data {
            DataTextureData::U32(v) => f(v),
            DataTextureData::F32(_) => panic!("three-rs: data texture is not a uint texture"),
        }
    }

    pub(crate) fn set_gpu(&self, gpu: wgpu::Texture) {
        self.0.borrow_mut().gpu = Some(gpu);
    }

    pub(crate) fn has_gpu(&self) -> bool {
        self.0.borrow().gpu.is_some()
    }

    /// True when the CPU-side payload has moved on since the last upload.
    pub(crate) fn needs_upload(&self) -> bool {
        let inner = self.0.borrow();
        inner.gpu.is_none() || inner.uploaded != inner.version
    }

    pub(crate) fn mark_uploaded(&self) {
        let mut inner = self.0.borrow_mut();
        inner.uploaded = inner.version;
    }

    /// Runs `f` with the uploaded GPU texture. Panics if the texture has not
    /// been uploaded yet.
    pub fn with_gpu<R>(&self, f: impl FnOnce(&wgpu::Texture) -> R) -> R {
        let inner = self.0.borrow();
        f(inner
            .gpu
            .as_ref()
            .expect("three-rs: data texture not uploaded"))
    }
}

impl std::fmt::Debug for DataTextureInner {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let len = match &self.data {
            DataTextureData::F32(v) => v.len(),
            DataTextureData::U32(v) => v.len(),
        };
        f.debug_struct("DataTextureInner")
            .field("data", &super::texture::DataLen(len))
            .field("width", &self.width)
            .field("height", &self.height)
            .field("gpu", &self.gpu)
            .finish()
    }
}
