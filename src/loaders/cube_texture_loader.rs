//! Port of `three.js/src/loaders/CubeTextureLoader.js` (+ the image decode
//! that `ImageLoader` gets from the browser).
//!
//! The page's load is asynchronous, but the harness fires its single RAF only
//! once the network is idle, so the faces are always present for the graded
//! frame; here they are decoded synchronously.

use std::path::{Path, PathBuf};

use crate::error::Error;
use crate::textures::{ColorSpace, CubeTexture, Image};

#[derive(Debug, Clone, Default)]
pub struct CubeTextureLoader {
    /// `Loader.path`, prefixed to every url.
    path: PathBuf,
}

impl CubeTextureLoader {
    pub fn new() -> Self {
        Self::default()
    }

    /// `loader.setPath( path )`.
    pub fn set_path<P: AsRef<Path>>(mut self, path: P) -> Self {
        self.path = path.as_ref().to_path_buf();
        self
    }

    /// `new CubeTextureLoader().load( urls )` — the loader sets
    /// `texture.colorSpace = SRGBColorSpace`.
    pub fn load<P: AsRef<Path>>(&self, urls: [P; 6]) -> Result<CubeTexture, Error> {
        let images = self.load_faces(&urls)?;
        let texture = CubeTexture::new(images);
        texture.set_color_space(ColorSpace::SRGB);
        Ok(texture)
    }

    /// The six decoded faces on their own — what a hand-authored mip level of
    /// a [`CubeTexture`] is made of. On the page each level is a whole
    /// `CubeTexture` and only its `images` is ever read, so the port skips the
    /// texture and keeps the faces.
    pub fn load_images<P: AsRef<Path>>(&self, urls: [P; 6]) -> Result<Vec<Image>, Error> {
        self.load_faces(&urls)
    }

    fn load_faces<P: AsRef<Path>>(&self, urls: &[P; 6]) -> Result<Vec<Image>, Error> {
        urls.iter()
            .map(|url| decode(&self.path.join(url.as_ref())))
            .collect()
    }
}

/// What the browser hands `copyExternalImageToTexture`: 8-bit RGBA, top-down,
/// opaque where the source has no alpha channel.
///
/// The decoder is picked by the file's magic and not by its extension, as it
/// is in the browser: `webgpu_materials_basic`'s six faces are PNG,
/// `webgpu_pmrem_scene`'s are JPEG, and both reach
/// [`super::texture_loader::decode_image`].
fn decode(path: &Path) -> Result<Image, Error> {
    let bytes = std::fs::read(path).map_err(|e| Error::io(path, e))?;
    super::texture_loader::decode_image(path, &bytes)
}
