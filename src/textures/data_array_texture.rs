//! Port of `three.js/src/textures/DataArrayTexture.js` — the morph-target data
//! texture (`Morph.js`' `getEntry()`), the one 2-D-array texture on the ladder.

use super::TextureId;
use std::cell::{Ref, RefCell};
use std::rc::Rc;

/// The state behind a [`DataArrayTexture`] handle.
pub struct DataArrayTextureInner {
    /// `new Float32Array( width * height * 4 * depth )` — one RGBA texel per
    /// vertex datum, layer-major.
    pub data: Vec<f32>,
    /// `image.width`.
    pub width: u32,
    /// `image.height`.
    pub height: u32,
    /// `image.depth` — the array layer count, i.e. the morph-target count.
    pub depth: u32,
    pub(crate) gpu: Option<wgpu::Texture>,
}

/// Cloning is a handle copy, as in JS.
#[derive(Clone)]
pub struct DataArrayTexture(Rc<RefCell<DataArrayTextureInner>>, TextureId);

/// The id is identity, not content: leaving it out keeps the `Debug` of a
/// binding description — what `examples/dump_wgsl.rs` prints beside the WGSL —
/// a function of the texture itself.
impl std::fmt::Debug for DataArrayTexture {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("DataArrayTexture").field(&self.0).finish()
    }
}

impl DataArrayTexture {
    /// `dataArrayTexture.dispose()` — drops this handle, and nothing more: a texture
    /// handle is an `Rc`, so the renderer frees the GPU side once every handle
    /// is gone, at the top of the next render (`docs/scene-graph.md`,
    /// "Identity and eviction"). Dropping the handle does the same; this
    /// exists so three.js code ports line for line.
    pub fn dispose(self) {}

    /// `new DataArrayTexture( data, width, height, depth )` with
    /// `texture.type = FloatType` — `rgba32float` on the GPU. `NearestFilter`
    /// and no mipmaps, and the shader only ever `textureLoad`s it, so there is
    /// no sampler binding at all.
    pub fn new(data: Vec<f32>, width: u32, height: u32, depth: u32) -> Self {
        Self(
            Rc::new(RefCell::new(DataArrayTextureInner {
                data,
                width,
                height,
                depth,
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

    /// Borrows the texture's data, width, height and depth.
    pub fn borrow(&self) -> Ref<'_, DataArrayTextureInner> {
        self.0.borrow()
    }

    /// `( image.width, image.height, image.depth )`.
    pub fn size(&self) -> (u32, u32, u32) {
        let inner = self.0.borrow();
        (inner.width, inner.height, inner.depth)
    }

    pub(crate) fn set_gpu(&self, gpu: wgpu::Texture) {
        self.0.borrow_mut().gpu = Some(gpu);
    }

    pub(crate) fn has_gpu(&self) -> bool {
        self.0.borrow().gpu.is_some()
    }

    /// Runs `f` with the uploaded GPU texture. Panics if the texture has not
    /// been uploaded yet.
    pub fn with_gpu<R>(&self, f: impl FnOnce(&wgpu::Texture) -> R) -> R {
        let inner = self.0.borrow();
        f(inner
            .gpu
            .as_ref()
            .expect("three-rs: data array texture not uploaded"))
    }
}

/// As `TextureInner`: the morph data is one `f32` per vertex datum and has no
/// place in a debug dump.
impl std::fmt::Debug for DataArrayTextureInner {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DataArrayTextureInner")
            .field("data", &super::texture::DataLen(self.data.len()))
            .field("width", &self.width)
            .field("height", &self.height)
            .field("depth", &self.depth)
            .field("gpu", &self.gpu)
            .finish()
    }
}
