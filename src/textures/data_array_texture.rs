//! Port of `three.js/src/textures/DataArrayTexture.js` — the morph-target data
//! texture (`Morph.js`' `getEntry()`), the one 2-D-array texture on the ladder.

use std::cell::{Ref, RefCell};
use std::rc::Rc;
use super::TextureId;

pub struct DataArrayTextureInner {
    /// `new Float32Array( width * height * 4 * depth )` — one RGBA texel per
    /// vertex datum, layer-major.
    pub data: Vec<f32>,
    pub width: u32,
    pub height: u32,
    /// `image.depth` — the array layer count, i.e. the morph-target count.
    pub depth: u32,
    pub gpu: Option<wgpu::Texture>,
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

    pub fn id(&self) -> usize {
        self.1.get()
    }

    pub fn borrow(&self) -> Ref<'_, DataArrayTextureInner> {
        self.0.borrow()
    }

    pub fn size(&self) -> (u32, u32, u32) {
        let inner = self.0.borrow();
        (inner.width, inner.height, inner.depth)
    }

    pub fn set_gpu(&self, gpu: wgpu::Texture) {
        self.0.borrow_mut().gpu = Some(gpu);
    }

    pub fn has_gpu(&self) -> bool {
        self.0.borrow().gpu.is_some()
    }

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
